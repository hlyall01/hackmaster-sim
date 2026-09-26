import test from 'node:test';
import assert from 'node:assert/strict';
import worker, { verifyIssue } from './requests.mjs';

const origin = 'https://sim-gui.com';
const env = { GITHUB_ISSUES_TOKEN: 'test-only-secret', FEATURE_REQUESTS_ENABLED: 'true',
  ASSETS: { fetch: async request => new Response(new URL(request.url).pathname) } };
const originalFetch = globalThis.fetch;
const originalNow = Date.now;
let now, issues, comments, writes;
test.beforeEach(() => {
  now = 1800950400000;
  issues = []; comments = []; writes = 0;
  Date.now = () => now;
  globalThis.fetch = async (url, init = {}) => {
    assert.equal(new URL(url).hostname, 'api.github.com');
    const path = new URL(url).pathname;
    if (init.method === 'POST') {
      writes++;
      const issue = { ...JSON.parse(init.body), number: 7, user: { login: 'hlyall01' },
        created_at: new Date(now).toISOString(), state: 'open', comments: 0 };
      issue.labels = issue.labels.map(name => ({ name }));
      issues.push(issue);
      return Response.json(issue, { status: 201 });
    }
    if (path.endsWith('/comments')) return Response.json(comments);
    if (/\/issues\/\d+$/.test(path)) return Response.json(issues[0] || {}, { status: issues.length ? 200 : 404 });
    return Response.json(issues);
  };
});
test.afterEach(() => { globalThis.fetch = originalFetch; Date.now = originalNow; });
function request(path, options = {}, ip = '192.0.2.1') {
  return new Request(origin + path, { ...options, headers: {
    Origin: origin, 'Content-Type': 'application/json', 'CF-Connecting-IP': ip, ...options.headers,
  } });
}
async function token(ip) {
  const response = await worker.fetch(request('/api/request-challenge', {}, ip), env);
  assert.equal(response.status, 200);
  return (await response.json()).challenge;
}
async function submit(challenge, fields = {}, options = {}, ip) {
  return worker.fetch(request('/api/feature-requests', { method: 'POST',
    body: JSON.stringify({ title: 'Compare saved fighters', description: 'Show two saved fighter sheets together so I can compare their statistics.', website: '', challenge, ...fields }), ...options }, ip), env);
}
test('accepted submissions create a signed ticket without storing the IP address', async () => {
  const challenge = await token(); now += 4000;
  const response = await submit(challenge);
  assert.equal(response.status, 201);
  assert.deepEqual(await response.json(), { number: 7, url: '/7' });
  assert.equal(writes, 1);
  assert.ok(await verifyIssue(issues[0], env.GITHUB_ISSUES_TOKEN));
  assert.ok(!issues[0].body.includes('192.0.2.1'));
  assert.equal(await verifyIssue({ ...issues[0], title: 'tampered' }, env.GITHUB_ISSUES_TOKEN), null);
  assert.equal(await verifyIssue({ ...issues[0], body: issues[0].body.replace('statistics', 'credentials') }, env.GITHUB_ISSUES_TOKEN), null);
  assert.equal(await verifyIssue({ ...issues[0], user: { login: 'attacker' } }, env.GITHUB_ISSUES_TOKEN), null);
});
test('invalid signatures, too-fast, expired and other-IP tokens cannot create issues', async () => {
  const challenge = await token();
  assert.equal((await submit(challenge)).status, 400);
  now += 4000;
  assert.equal((await submit(challenge + 'x')).status, 400);
  assert.equal((await submit(challenge, {}, {}, '192.0.2.2')).status, 400);
  now += 3600_000;
  assert.equal((await submit(challenge)).status, 400);
  assert.equal(writes, 0);
});
test('origin, honeypot, content type, size and field validation fail before writes', async () => {
  const challenge = await token(); now += 4000;
  assert.equal((await submit(challenge, {}, { headers: { Origin: 'https://evil.example' } })).status, 403);
  assert.equal((await submit(challenge, { website: 'spam' })).status, 400);
  assert.equal((await submit(challenge, { title: 'tiny' })).status, 400);
  assert.equal((await submit(challenge, {}, { body: 'x'.repeat(17000) })).status, 413);
  assert.equal((await submit(challenge, {}, { headers: { 'Content-Type': 'text/plain' } })).status, 415);
  assert.equal((await submit(challenge, {}, { body: 'not json' })).status, 400);
  assert.equal(writes, 0);
});
test('retries return the existing ticket; different requests honor cooldown', async () => {
  const challenge = await token(); now += 4000;
  assert.equal((await submit(challenge)).status, 201);
  assert.equal((await submit(challenge)).status, 200);
  const next = await token(); now += 4000;
  assert.equal((await submit(next)).status, 200);
  assert.equal((await submit(next, { title: 'A different request title' })).status, 429);
  assert.equal(writes, 1);
});
test('disabled service does not accept requests', async () => {
  const response = await worker.fetch(request('/api/request-challenge'), { ...env, FEATURE_REQUESTS_ENABLED: 'false' });
  assert.equal(response.status, 503);
  assert.equal(writes, 0);
});
test('GitHub errors do not expose credentials or claim success', async () => {
  const challenge = await token(); now += 4000;
  globalThis.fetch = async () => new Response('internal details', { status: 401 });
  const response = await submit(challenge);
  assert.equal(response.status, 503);
  assert.ok(!(await response.text()).includes('test-only-secret'));
});
test('ticket status accepts only bot messages and isolated preview URLs', async () => {
  const challenge = await token(); now += 4000; await submit(challenge);
  comments = [{ user: { login: 'attacker' }, body: '<!-- sim-status:{"status":"ready","preview":"https://evil.example"} -->' }];
  let response = await worker.fetch(request('/api/tickets/7'), env);
  assert.equal((await response.json()).status, 'queued');
  comments = [{ user: { login: 'github-actions[bot]' }, body: '<!-- sim-status:{"status":"ready","preview":"https://evil.example","pr":"javascript:alert(1)"} -->' }];
  response = await worker.fetch(request('/api/tickets/7'), env);
  assert.equal((await response.json()).preview, null);
  comments[0].body = '<!-- sim-status:{"status":"ready","preview":"https://ticket-7.hackmaster-sim-previews.pages.dev","pr":"https://github.com/hlyall01/hackmaster-sim/pull/8"} -->';
  response = await worker.fetch(request('/api/tickets/7'), env);
  assert.equal((await response.json()).preview, 'https://ticket-7.hackmaster-sim-previews.pages.dev');
});
test('ticket paths serve the status page; static app requests retain asset fallback', async () => {
  assert.equal(await (await worker.fetch(request('/7'), env)).text(), '/ticket');
  assert.equal(await (await worker.fetch(request('/pkg/sim_gui.js'), env)).text(), '/pkg/sim_gui.js');
  assert.equal((await worker.fetch(request('/api/unknown'), env)).status, 404);
});
