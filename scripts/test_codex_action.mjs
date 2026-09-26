// Run against the untouched, pinned checkout: node --test scripts/test_codex_action.mjs
// These tests use a fake CLI: no API calls, credentials, sudo or agent spend.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { test } from 'node:test';
import { patchBundle } from './patch_codex_action.mjs';

const root = path.resolve('target/codex-action-tests');
mkdirSync(root, { recursive: true });
const original = readFileSync('target/codex-action/dist/main.js', 'utf8');
const patched = patchBundle(original);

function execute(bundle, { hold = true, large = false, exitCode = 0, missingOutput = false } = {}) {
  const dir = mkdtempSync(path.join(root, 'run-'));
  writeFileSync(path.join(dir, 'action.cjs'), bundle);
  writeFileSync(path.join(dir, 'codex'), `#!${process.execPath}\n` + `
const { spawn } = require('node:child_process');
const { writeFileSync } = require('node:fs');
const args = process.argv.slice(2);
if (!${missingOutput}) writeFileSync(args[args.indexOf('--output-last-message') + 1], 'completed result');
if (${large}) {
  process.stdout.write('stdout-start:' + 'o'.repeat(1024 * 1024) + ':stdout-end\\n');
  process.stderr.write('stderr-start:' + 'e'.repeat(1024 * 1024) + ':stderr-end\\n');
}
console.log('CLI finished');
if (${hold}) {
  const descendant = spawn(process.execPath, ['-e', 'setTimeout(() => {}, 15000)'], { stdio: 'inherit' });
  writeFileSync(${JSON.stringify(path.join(dir, 'descendant.pid'))}, String(descendant.pid));
  descendant.unref();
}
process.exitCode = ${exitCode};
`, { mode: 0o755 });
  const resultPath = path.join(dir, 'result.txt');
  const started = Date.now();
  let result;
  try {
    result = spawnSync(process.execPath, [path.join(dir, 'action.cjs'), 'run-codex-exec',
      '--prompt', 'Fake command only', '--prompt-file', '', '--output-file', resultPath,
      '--codex-home', '', '--cd', dir, '--extra-args', '', '--output-schema', '',
      '--output-schema-file', '', '--sandbox', '', '--permission-profile', ':workspace',
      '--model', '', '--effort', '', '--safety-strategy', 'unsafe', '--codex-user', ''], {
      env: { PATH: `${dir}${path.delimiter}${path.dirname(process.execPath)}`, HOME: dir },
      encoding: 'utf8', timeout: 2000, maxBuffer: 5 * 1024 * 1024,
    });
  } finally {
    try { process.kill(Number(readFileSync(path.join(dir, 'descendant.pid'), 'utf8'))); }
    catch (error) { if (!['ENOENT', 'ESRCH'].includes(error.code)) throw error; }
  }
  return { ...result, elapsed: Date.now() - started, resultPath };
}

test('upstream keeps the runner waiting after final output when a descendant retains stdio', () => {
  const result = execute(original);
  assert.equal(result.error?.code, 'ETIMEDOUT');
  assert.equal(readFileSync(result.resultPath, 'utf8'), 'completed result');
});

test('patched production bundle returns promptly, forwards logs and publishes final output', () => {
  const result = execute(patched, { large: true });
  assert.equal(result.error, undefined);
  assert.equal(result.status, 0, result.stderr);
  assert.ok(result.elapsed < 2000);
  assert.ok(result.stdout.includes('stdout-start:' + 'o'.repeat(1024 * 1024) + ':stdout-end'));
  assert.ok(result.stderr.includes('stderr-start:' + 'e'.repeat(1024 * 1024) + ':stderr-end'));
  assert.match(result.stdout, /final-message::completed result/);
});

test('nonzero CLI exit still fails even when a result file exists', () => {
  const result = execute(patched, { exitCode: 7 });
  assert.equal(result.error, undefined);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /exited with code 7/);
  assert.doesNotMatch(result.stdout, /final-message::/);
});

test('missing result fails and changed upstream bundles fail closed', () => {
  const result = execute(patched, { hold: false, missingOutput: true });
  assert.equal(result.error, undefined);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /ENOENT/);
  assert.throws(() => patchBundle(original + '\n'), /differs from reviewed/);
});
