// Temporary fix for openai/codex-action#169 / #151. Keep upstream's sandbox,
// credential handling and privilege isolation; change only subprocess logging.
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const ACTION_SHA = '86365089eb2b84e0a8fb0717b304f8bdcb13b20e';
const BUNDLE_SHA256 = 'c0e530e7883cc18e28f854d171f58d83e2387f7decf29f1a8ec3aa682f6601be';

// Self-contained because this function is inserted into the pinned action bundle.
export async function runCodexChild(spawn, program, args, env, input, finalize) {
  await new Promise((resolve, reject) => {
    const child = spawn(program, args, { env, stdio: ['pipe', 'pipe', 'pipe'] });
    // Descendants get private pipes, never the runner's log transport descriptors.
    child.stdout.pipe(process.stdout, { end: false });
    child.stderr.pipe(process.stderr, { end: false });
    const closeStreams = () => {
      child.stdout.unpipe(process.stdout);
      child.stderr.unpipe(process.stderr);
      child.stdout.destroy();
      child.stderr.destroy();
      child.stdin.destroy();
    };
    child.once('error', error => { closeStreams(); reject(error); });
    child.stdin.on('error', error => {
      // A child that exits before consuming stdin still has an exit status below.
      if (error.code !== 'EPIPE') { closeStreams(); reject(error); }
    });
    child.once('exit', async (code, signal) => {
      // Drain buffered output, but never wait for a surviving descendant to close
      // stdout/stderr. The hard bound also covers continuous descendant output.
      await new Promise(drained => {
        const streams = [child.stdout, child.stderr];
        let quiet;
        const finish = () => {
          clearTimeout(quiet); clearTimeout(deadline);
          for (const stream of streams) stream.off('data', schedule);
          drained();
        };
        const schedule = () => {
          clearTimeout(quiet);
          quiet = setTimeout(() => {
            if (streams.every(stream => stream.destroyed ||
                (stream.readableLength === 0 && stream.readableFlowing !== false))) finish();
            else schedule();
          }, 50);
        };
        const deadline = setTimeout(finish, 1000);
        for (const stream of streams) stream.on('data', schedule);
        schedule();
      });
      closeStreams();
      if (code !== 0) {
        reject(new Error(`${program} exited with ${signal ? `signal ${signal}` : `code ${code}`}`));
        return;
      }
      try { await finalize(); resolve(); } catch (error) { reject(error); }
    });
    child.stdin.end(input);
  });
}

export function patchBundle(original) {
  if (createHash('sha256').update(original).digest('hex') !== BUNDLE_SHA256)
    throw new Error(`Codex action bundle differs from reviewed ${ACTION_SHA}; review the workaround before upgrading.`);
  const start = original.indexOf('    await new Promise((resolve, reject) => {', original.indexOf('async function runCodexExec('));
  const end = original.indexOf('\n  } finally {\n    await cleanupOutputSchema', start);
  if (start < 0 || end < 0) throw new Error('Could not locate pinned Codex subprocess block');
  return original.slice(0, start) + `    await (${runCodexChild.toString()})(\n` +
    '      import_child_process2.spawn, program2, command, env, input,\n' +
    '      () => finalizeExecution(outputFile, runAsUser)\n    );' + original.slice(end);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const bundle = path.join(process.argv[2] || 'target/codex-action', 'dist/main.js');
  writeFileSync(bundle, patchBundle(readFileSync(bundle, 'utf8')));
  console.log(`Applied bounded subprocess logging fix to Codex action ${ACTION_SHA}`);
}
