import { appEnv, tokenResponse } from './github-app-fixture.mjs';
import test from 'node:test';
import assert from 'node:assert/strict';
import worker, { verifyIssue, verifyRevision } from './requests.mjs';

const origin = 'https://feature.sim-gui.com';
const env = { ...appEnv, REQUEST_SIGNING_SECRET: 'test-only-secret', FEATURE_REQUESTS_ENABLED: 'true',
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
    if (String(url).endsWith('/access_tokens')) return tokenResponse();
    assert.equal(init.redirect, 'manual');
    const path = new URL(url).pathname;
    if (init.method === 'POST') {
      writes++;
      if (path.endsWith('/comments')) {
        const comment = { ...JSON.parse(init.body), id: 100 + comments.length, user: { login: 'sim-gui-requests[bot]' },
          created_at: new Date(now).toISOString(), issue_url: 'https://api.github.com/repos/hlyall01/hackmaster-sim/issues/7' };
        comments.push(comment); issues[0].comments = comments.length;
        return Response.json(comment, { status: 201 });
      }
      const issue = { ...JSON.parse(init.body), number: 7, user: { login: 'sim-gui-requests[bot]' },
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
async function token(ip, scope = 'request') {
  const response = await worker.fetch(request(`/api/request-challenge?scope=${scope}`, {}, ip), env);
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
  assert.ok(await verifyIssue(issues[0], env.REQUEST_SIGNING_SECRET));
  assert.ok(!issues[0].body.includes('192.0.2.1'));
  assert.equal(await verifyIssue({ ...issues[0], title: 'tampered' }, env.REQUEST_SIGNING_SECRET), null);
  assert.equal(await verifyIssue({ ...issues[0], body: issues[0].body.replace('statistics', 'credentials') }, env.REQUEST_SIGNING_SECRET), null);
  assert.equal(await verifyIssue({ ...issues[0], user: { login: 'attacker' } }, env.REQUEST_SIGNING_SECRET), null);
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
  assert.equal((await submit(challenge, {}, { headers: { Origin: 'https://sim-gui.com' } })).status, 403);
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

test('old ticket and API links redirect to the feature host without touching GitHub', async () => {
  globalThis.fetch = async () => { throw new Error('Redirects must not call GitHub'); };
  for (const path of ['/6', '/6/?from=old-link', '/api/tickets/6', '/api/request-challenge']) {
    for (const method of ['GET', 'HEAD']) {
      const response = await worker.fetch(new Request('https://sim-gui.com' + path, { method }), env);
      assert.equal(response.status, 308);
      assert.equal(response.headers.get('Location'), origin + path);
    }
  }
  assert.equal(await (await worker.fetch(new Request('https://sim-gui.com/'), env)).text(), '/');
});

test('legacy host rejects submissions even with a valid new-host origin', async () => {
  const challenge = await token(); now += 4000;
  const response = await worker.fetch(new Request('https://sim-gui.com/api/feature-requests', {
    method: 'POST', headers: { Origin: origin, 'Content-Type': 'application/json', 'CF-Connecting-IP': '192.0.2.1' },
    body: JSON.stringify({ title: 'Compare saved fighters', description: 'Show two saved fighter sheets together so I can compare their statistics.', website: '', challenge }),
  }), env);
  assert.equal(response.status, 403);
  assert.equal(writes, 0);
});

function bot(state) {
  comments.push({ id: 100 + comments.length, user: { login: 'github-actions[bot]' }, body: `<!-- sim-status:${JSON.stringify(state)} -->` });
  issues[0].comments = comments.length;
}
async function readyTicket() {
  const challenge = await token(); now += 4000; await submit(challenge);
  bot({ status: 'ready', preview: 'https://first.hackmaster-sim-previews.pages.dev',
    pr: 'https://github.com/hlyall01/hackmaster-sim/pull/8', sha: 'a'.repeat(40) });
  now += 600_000;
}
async function revision(challenge, fields = {}, ip) {
  return worker.fetch(request('/api/tickets/7/revisions', { method: 'POST', body: JSON.stringify({
    challenge, revision: 0, description: 'Keep the feature and add a reset button.', website: '', ...fields,
  }) }, ip), env);
}
async function status() { return (await worker.fetch(request('/api/tickets/7'), env)).json(); }

test('signed follow-ups keep the old preview, deduplicate retries and refresh when ready', async () => {
  await readyTicket();
  const challenge = await token(undefined, 'revision:7'); now += 4000;
  assert.equal((await revision(challenge)).status, 201);
  const comment = comments.at(-1);
  const proof = await verifyRevision(comment, '7', env.REQUEST_SIGNING_SECRET);
  assert.equal(proof.metadata.sha, 'a'.repeat(40));
  assert.equal(await verifyRevision(comment, '8', env.REQUEST_SIGNING_SECRET), null);
  assert.equal(await verifyRevision({ ...comment, body: comment.body.replace('reset', 'delete') }, '7', env.REQUEST_SIGNING_SECRET), null);
  assert.equal(await verifyRevision({ ...comment, user: { login: 'attacker' } }, '7', env.REQUEST_SIGNING_SECRET), null);
  assert.equal((await revision(challenge)).status, 200);
  assert.equal(writes, 2);
  let ticket = await status();
  assert.equal(ticket.status, 'queued'); assert.equal(ticket.canRevise, false);
  assert.equal(ticket.preview, 'https://first.hackmaster-sim-previews.pages.dev');
  assert.equal(ticket.history.length, 1);
  bot({ status: 'ready', revision: 0, preview: 'https://stale.hackmaster-sim-previews.pages.dev' });
  assert.equal((await status()).status, 'queued', 'An old run cannot finish a newer revision');
  bot({ status: 'coding', revision: comment.id });
  assert.equal((await status()).preview, ticket.preview);
  bot({ status: 'ready', revision: comment.id, preview: 'https://second.hackmaster-sim-previews.pages.dev', sha: 'b'.repeat(40) });
  ticket = await status();
  assert.equal(ticket.canRevise, true); assert.equal(ticket.revision, comment.id);
  assert.equal(ticket.pr, 'https://github.com/hlyall01/hackmaster-sim/pull/8');
  assert.equal(ticket.preview, 'https://second.hackmaster-sim-previews.pages.dev');
});

test('revisions reject wrong scope, stale state, busy tickets, closed tickets and spam', async () => {
  await readyTicket();
  const wrong = await token(); const challenge = await token(undefined, 'revision:7'); now += 4000;
  assert.equal((await revision(wrong)).status, 400);
  assert.equal((await revision(challenge, { revision: 99 })).status, 409);
  assert.equal((await revision(challenge, { website: 'spam' })).status, 400);
  issues[0].state = 'closed'; assert.equal((await revision(challenge)).status, 409);
  issues[0].state = 'open'; bot({ status: 'coding' });
  assert.equal((await revision(challenge)).status, 409);
  bot({ status: 'ready' });
  assert.equal((await revision(challenge)).status, 201);
  const id = comments.at(-1).id;
  bot({ status: 'ready', revision: id });
  const next = await token(undefined, 'revision:7'); now += 4000;
  assert.equal((await revision(next, { revision: id, description: 'Another revision on the same feature.' })).status, 429);
  const fresh = await token(); now += 4000;
  assert.equal((await submit(fresh, { title: 'Different feature request' })).status, 429, 'Revision cooldown also blocks new issues');
});

test('another visitor may revise, but unsigned comments cannot queue agents', async () => {
  await readyTicket();
  comments.push({ user: { login: 'attacker' }, body: 'Please do something' });
  assert.equal((await status()).status, 'ready');
  const challenge = await token('192.0.2.2', 'revision:7'); now += 4000;
  assert.equal((await revision(challenge, {}, '192.0.2.2')).status, 201);
});

test('screening blocks follow-ups until decided; rejected feedback can be corrected without losing the preview', async () => {
  await readyTicket();
  bot({ status: 'screening', message: 'Checking relevance.' });
  assert.equal((await status()).canRevise, false);
  bot({ status: 'rejected', message: 'Not a feature for sim-gui.' });
  const ticket = await status();
  assert.equal(ticket.status, 'rejected'); assert.equal(ticket.canRevise, true);
  assert.equal(ticket.preview, 'https://first.hackmaster-sim-previews.pages.dev');
  const challenge = await token(undefined, 'revision:7'); now += 4000;
  assert.equal((await revision(challenge, { description: 'Instead, let me compare my saved fighters.' })).status, 201);
});
