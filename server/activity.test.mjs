import { appEnv, tokenResponse } from './github-app-fixture.mjs';
import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import worker, { sign, activityIdentity, readActivity, ticketState } from './requests.mjs';
import { projectEvent, SessionTail } from '../scripts/report_activity.mjs';

const origin = 'https://feature.sim-gui.com';
const env = { ...appEnv, REQUEST_SIGNING_SECRET: 'test-activity-secret', FEATURE_REQUESTS_ENABLED: 'true' };
const encode = value => Buffer.from(JSON.stringify(value)).toString('base64url');
const keys = await crypto.subtle.generateKey({ name: 'RSASSA-PKCS1-v1_5', modulusLength: 2048, publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' }, true, ['sign', 'verify']);
const jwk = { ...await crypto.subtle.exportKey('jwk', keys.publicKey), kid: 'test-key', use: 'sig' };
const originalFetch = globalThis.fetch, originalNow = Date.now;
let now, comments, issue, writes;
const state = () => ({ status: 'coding', revision: 0, run: '12345', attempt: '1' });
const bot = data => ({ user: { login: 'github-actions[bot]' }, body: `<!-- sim-status:${JSON.stringify(data)} -->` });
test.beforeEach(async () => {
  now = 1800950400000; Date.now = () => now; writes = [];
  comments = [bot(state())];
  const title = '[Request] Compare fighters', description = 'Compare two fighters together.', metadata = encode({});
  issue = { title, body: `${description}\n<!-- sim-request:v1:${metadata}.${await sign(env.REQUEST_SIGNING_SECRET, `issue:${title}\n${description}\n${metadata}`)} -->`,
    state: 'open', number: 7, user: { login: 'sim-gui-requests[bot]' }, labels: [{ name: 'site-request' }], comments: 1 };
  globalThis.fetch = async (url, options = {}) => {
    if (String(url).includes('token.actions.githubusercontent.com/')) return Response.json({ keys: [jwk] });
    assert.equal(new URL(url).hostname, 'api.github.com');
    if (String(url).endsWith('/access_tokens')) return tokenResponse();
    assert.equal(options.redirect, 'manual');
    const pathname = new URL(url).pathname;
    if (['POST', 'PATCH'].includes(options.method)) {
      writes.push(options.method);
      let comment = options.method === 'PATCH' ? comments.find(c => c.id === Number(pathname.split('/').at(-1))) : null;
      if (!comment) { comment = { id: comments.length + 100, user: { login: 'sim-gui-requests[bot]' } }; comments.push(comment); }
      Object.assign(comment, JSON.parse(options.body)); issue.comments = comments.length;
      return Response.json(comment);
    }
    if (pathname.endsWith('/comments')) return Response.json(comments);
    return Response.json(issue);
  };
});
test.afterEach(() => { globalThis.fetch = originalFetch; Date.now = originalNow; });
async function token(overrides = {}, header = {}) {
  const claims = { iss: 'https://token.actions.githubusercontent.com', aud: `${origin}/activity`, iat: now / 1000, nbf: now / 1000 - 1, exp: now / 1000 + 300,
    repository: 'hlyall01/hackmaster-sim', repository_id: '1121535444', repository_owner_id: '11281654', ref: 'refs/heads/main',
    workflow_ref: 'hlyall01/hackmaster-sim/.github/workflows/feature-request.yml@refs/heads/main', event_name: 'issues', run_id: '12345', run_attempt: '1', ...overrides };
  const message = `${encode({ alg: 'RS256', typ: 'JWT', kid: 'test-key', ...header })}.${encode(claims)}`;
  const signature = await crypto.subtle.sign('RSASSA-PKCS1-v1_5', keys.privateKey, Buffer.from(message));
  return `${message}.${Buffer.from(signature).toString('base64url')}`;
}
function event(id = 1, text = 'Checking the fighter editor.') { return { id, text, kind: 'message', at: new Date(now).toISOString() }; }
async function post(body = { revision: 0, events: [event()] }, identity) {
  return worker.fetch(new Request(`${origin}/api/tickets/7/activity`, { method: 'POST', headers: { Authorization: `Bearer ${identity || await token()}`, 'Content-Type': 'application/json' }, body: JSON.stringify(body) }), env);
}
test('activity identity validates signature, time, repository, workflow and audience', async () => {
  assert.deepEqual(await activityIdentity(await token()), { run: '12345', attempt: '1' });
  for (const overrides of [{ exp: now / 1000 }, { nbf: now / 1000 + 100 }, { aud: 'other' }, { repository_id: '999' }, { repository_owner_id: '999' }, { ref: 'refs/pull/2/merge' }, { workflow_ref: 'other' }, { event_name: 'pull_request' }])
    await assert.rejects(activityIdentity(await token(overrides)), /Invalid activity identity/);
  await assert.rejects(activityIdentity(await token({}, { alg: 'none' })), /Invalid activity identity/);
  const valid = await token();
  await assert.rejects(activityIdentity(valid.slice(0, -20) + 'invalidsignature'), /Invalid activity identity/);
});
test('reporter updates one signed comment, deduplicates and retains activity after completion', async () => {
  assert.equal((await post()).status, 200);
  now += 15000;
  assert.equal((await post({ revision: 0, events: [event(1), event(2, 'Making the change.')] })).status, 200);
  assert.equal((await post()).status, 200);
  assert.deepEqual(writes, ['POST', 'PATCH']);
  comments.push(bot({ ...state(), status: 'ready' }));
  const response = await worker.fetch(new Request(`${origin}/api/tickets/7`), env);
  const data = await response.json();
  assert.equal(data.status, 'ready'); assert.equal(data.activity.events.length, 2);
  assert.match(data.run, /runs\/12345\/attempts\/1$/);
  assert.equal((await post({ revision: 0, events: [event(3)] })).status, 409);
});
test('wrong run, attempt, revision, closed issue, flooding and oversized data cannot write', async () => {
  assert.equal((await post(undefined, await token({ run_id: '999' }))).status, 409);
  assert.equal((await post(undefined, await token({ run_attempt: '2' }))).status, 409);
  assert.equal((await post({ revision: 100, events: [event()] })).status, 409);
  issue.state = 'closed'; assert.equal((await post()).status, 409); issue.state = 'open';
  assert.equal((await post({ revision: 0, events: Array.from({ length: 41 }, (_, i) => event(i + 1)) })).status, 400);
  assert.equal((await post({ revision: 0, events: [event(1, 'x'.repeat(33000))] })).status, 413);
  assert.equal((await post({ revision: 0, events: [event(2), event(1)] })).status, 400);
  assert.equal((await post()).status, 200);
  assert.equal((await post({ revision: 0, events: [event(2)] })).status, 429);
  assert.deepEqual(writes, ['POST']);
});
test('signed activity cannot be forged, copied across tickets or overwrite authoritative status', async () => {
  assert.equal((await post()).status, 200);
  const comment = comments.at(-1);
  assert.equal(await readActivity([comment], 8, state(), env.REQUEST_SIGNING_SECRET), null);
  assert.equal(await readActivity([{ ...comment, body: comment.body.replace('sim-activity:v1:', 'sim-activity:v1:x') }], 7, state(), env.REQUEST_SIGNING_SECRET), null);
  assert.equal(await readActivity([comment], 7, { ...state(), revision: 100 }, env.REQUEST_SIGNING_SECRET), null);
  assert.equal((await ticketState(comments, 7, env.REQUEST_SIGNING_SECRET)).status, 'coding');
});
const record = payload => ({ timestamp: '2026-09-26T12:00:00Z', type: 'response_item', payload });
test('only public assistant phases and generic tools are projected; secrets and internals are excluded', () => {
  for (const payload of [{ type: 'reasoning', summary: ['private'] }, { type: 'function_call_output', output: 'password' }, { type: 'message', role: 'user', phase: 'commentary', content: [{ type: 'input_text', text: 'private' }] }, { type: 'message', role: 'assistant', phase: 'analysis', content: [{ type: 'output_text', text: 'private' }] }]) assert.equal(projectEvent(record(payload)), null);
  assert.equal(projectEvent(record({ type: 'function_call', name: 'exec_command', arguments: 'secret command' })).text, 'Running a command');
  assert.equal(projectEvent(record({ type: 'custom_tool_call', name: 'apply_patch', input: 'private diff' })).text, 'Editing files');
  const projected = projectEvent(record({ type: 'message', role: 'assistant', phase: 'commentary', content: [{ type: 'output_text', text: 'Checking sk-abcdefghijklmnop <!-- sim-status:{} -->' }] }));
  assert.equal(projected.text, 'Checking [redacted]');
  assert.equal(projectEvent(record({ type: 'message', role: 'assistant', phase: 'final_answer', content: [{ type: 'output_text', text: JSON.stringify({ status: 'implemented', summary: 'Finished testing.' }) }] })).text, 'Finished testing.');
});
test('session tail reads partial UTF-8 once, skips oversized records and ignores non-session files', () => {
  fs.mkdirSync('target/activity-tests', { recursive: true });
  const directory = fs.mkdtempSync('target/activity-tests/tail-');
  const name = path.join(directory, 'rollout.jsonl');
  const tail = new SessionTail();
  const line = Buffer.from(JSON.stringify(record({ type: 'message', role: 'assistant', phase: 'commentary', content: [{ type: 'output_text', text: 'Checking café ☕' }] })) + '\n');
  fs.writeFileSync(path.join(directory, 'auth.json'), line);
  const split = line.indexOf(Buffer.from('é')) + 1;
  fs.writeFileSync(name, line.subarray(0, split)); assert.deepEqual(tail.read(directory), []);
  fs.appendFileSync(name, line.subarray(split)); assert.equal(tail.read(directory)[0].text, 'Checking café ☕');
  assert.deepEqual(tail.read(directory), []);
  fs.appendFileSync(name, 'x'.repeat(300000)); assert.deepEqual(tail.read(directory), []);
  fs.appendFileSync(name, '\n'); fs.appendFileSync(name, line);
  assert.equal(tail.read(directory)[0].text, 'Checking café ☕');
});
