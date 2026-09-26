// Trusted, best-effort telemetry. Read session records only, never auth/config files.
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { StringDecoder } from 'node:string_decoder';

const origin = 'https://feature.sim-gui.com';
export function projectEvent(record) {
  const item = record?.payload;
  if (record?.type !== 'response_item' || !item) return null;
  let kind, text;
  if (item.type === 'message' && item.role === 'assistant' && ['commentary', 'final_answer'].includes(item.phase)) {
    text = (item.content || []).filter(part => part.type === 'output_text').map(part => part.text).join('\n');
    kind = item.phase === 'final_answer' ? 'summary' : 'message';
    if (kind === 'summary') { try { text = JSON.parse(text).summary || text; } catch { /* Plain text final answer. */ } }
  } else if (['function_call', 'custom_tool_call', 'local_shell_call', 'web_search_call'].includes(item.type)) {
    kind = 'tool';
    const name = item.name || item.type;
    if (/apply_patch/.test(name)) text = 'Editing files';
    else if (/exec_command|shell|terminal/.test(name)) text = 'Running a command';
    else if (/update_plan/.test(name)) text = 'Updating the work plan';
    else if (/search|browse/.test(name)) text = 'Looking up information';
    else text = 'Using a tool';
  } else return null; // Never relay reasoning, prompts, command arguments or tool output.
  if (typeof text !== 'string' || !text.trim()) return null;
  text = text.replace(/<!--[\s\S]*?-->/g, '').replace(/[\u0000-\u001f\u007f]/g, ' ')
    .replace(/\b(?:sk-[\w-]{12,}|gh[pousr]_[\w]{12,}|github_pat_[\w]{12,}|eyJ[\w-]+\.[\w-]+\.[\w-]+)\b/g, '[redacted]').trim().slice(0, 1200);
  return text ? { kind, text, at: Number.isFinite(Date.parse(record.timestamp)) ? new Date(record.timestamp).toISOString() : new Date().toISOString() } : null;
}

export class SessionTail {
  constructor() { this.files = new Map(); }
  read(root) {
    const records = [];
    const visit = (directory, depth = 0) => {
      if (depth > 4 || !fs.existsSync(directory) || fs.lstatSync(directory).isSymbolicLink()) return;
      for (const entry of fs.readdirSync(directory, { withFileTypes: true }).slice(0, 100)) {
        const name = path.join(directory, entry.name);
        if (entry.isDirectory()) { visit(name, depth + 1); continue; }
        if (!entry.isFile() || !entry.name.endsWith('.jsonl')) continue;
        let state = this.files.get(name);
        if (!state) { state = { offset: 0, pending: '', decoder: new StringDecoder('utf8'), dropping: false }; this.files.set(name, state); }
        const fd = fs.openSync(name, fs.constants.O_RDONLY | fs.constants.O_NOFOLLOW);
        try {
          const buffer = Buffer.alloc(256 * 1024);
          // Bound each pass even if a tool produces a very large log.
          for (let read = 0; read < 16; read++) {
            const size = fs.readSync(fd, buffer, 0, buffer.length, state.offset);
            if (!size) break;
            state.offset += size;
            const parts = (state.pending + state.decoder.write(buffer.subarray(0, size))).split('\n');
            state.pending = parts.pop();
            for (const line of parts) {
              if (state.dropping) { state.dropping = false; continue; }
              if (line.length > 128000) continue;
              try { const event = projectEvent(JSON.parse(line)); if (event) records.push(event); } catch { /* Partial/unknown records aren't public. */ }
            }
            if (state.pending.length > 128000) { state.pending = ''; state.dropping = true; }
          }
        } finally { fs.closeSync(fd); }
      }
    };
    visit(root);
    return records;
  }
}

async function main() {
  const env = process.env;
  if (!/^[1-9][0-9]{0,8}$/.test(env.ISSUE_NUMBER || '') || !env.CODEX_HOME || !env.ACTIVITY_STOP_FILE) return;
  const tail = new SessionTail();
  const events = [];
  let serial = 0, sent = 0, nextSend = 0, stopping = 0;
  const deadline = Date.now() + 26 * 60_000;
  while (Date.now() < deadline) {
    try {
      for (const event of tail.read(path.join(env.CODEX_HOME, 'sessions'))) {
        // Suppress repeated generic tool labels, but preserve real commentary.
        if (event.kind === 'tool' && events.at(-1)?.text === event.text) continue;
        events.push({ ...event, id: ++serial });
      }
      while (events.length > 40 || Buffer.byteLength(JSON.stringify({ events, revision: Number(env.REVISION_ID || 0) })) > 30000) events.shift();
      if (serial > sent && Date.now() >= nextSend && events.length) {
        nextSend = Date.now() + 15000;
        const tokenURL = new URL(env.ACTIONS_ID_TOKEN_REQUEST_URL);
        tokenURL.searchParams.set('audience', `${origin}/activity`);
        const identity = await fetch(tokenURL, { headers: { Authorization: `Bearer ${env.ACTIONS_ID_TOKEN_REQUEST_TOKEN}` }, signal: AbortSignal.timeout(10000), redirect: 'error' });
        if (!identity.ok) throw new Error('identity');
        const { value } = await identity.json();
        const response = await fetch(`${origin}/api/tickets/${env.ISSUE_NUMBER}/activity`, {
          method: 'POST', headers: { Authorization: `Bearer ${value}`, 'Content-Type': 'application/json' },
          body: JSON.stringify({ events, revision: Number(env.REVISION_ID || 0) }), signal: AbortSignal.timeout(15000), redirect: 'error',
        });
        await response.body?.cancel();
        if (response.ok) sent = serial;
        else if (response.status === 409) break;
        else console.warn(`Activity update unavailable (${response.status}); will retry.`);
      }
    } catch { console.warn('Activity update unavailable; will retry.'); }
    if (fs.existsSync(env.ACTIVITY_STOP_FILE)) {
      stopping ||= Date.now();
      if (sent === serial || Date.now() - stopping > 35000) break;
    }
    await new Promise(resolve => setTimeout(resolve, 2000));
  }
  fs.writeFileSync(`${env.ACTIVITY_STOP_FILE}.done`, 'done');
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) await main();
