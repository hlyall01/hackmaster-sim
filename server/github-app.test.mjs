import test from 'node:test';
import assert from 'node:assert/strict';
import { verify as verifySignature } from 'node:crypto';
import worker, { githubAppToken, sign, verifyIssue, verifyRevision } from './requests.mjs';
import { appEnv, testKeys, tokenResponse } from './github-app-fixture.mjs';

const originalFetch = globalThis.fetch;
test.afterEach(() => { globalThis.fetch = originalFetch; });

test('app JWT is signed correctly and installation token is restricted to this repository and issues', async () => {
  globalThis.fetch = async (url, options) => {
    assert.equal(url, 'https://api.github.com/app/installations/456/access_tokens');
    assert.equal(options.redirect, 'manual');
    assert.deepEqual(JSON.parse(options.body), { repositories: ['hackmaster-sim'], permissions: { issues: 'write' } });
    const parts = options.headers.Authorization.slice(7).split('.');
    const claims = JSON.parse(Buffer.from(parts[1], 'base64url'));
    assert.equal(claims.iss, '123');
    assert.ok(claims.iat <= Date.now() / 1000 - 59);
    assert.ok(claims.exp > Date.now() / 1000 + 530 && claims.exp < Date.now() / 1000 + 541);
    assert.ok(verifySignature('RSA-SHA256', Buffer.from(parts.slice(0, 2).join('.')), testKeys.publicKey, Buffer.from(parts[2], 'base64url')));
    return tokenResponse();
  };
  assert.equal(await githubAppToken(appEnv), 'test-installation-token');
});

test('app authentication fails closed on redirects, API failures, expired tokens and missing keys', async () => {
  for (const response of [new Response('', { status: 302 }), new Response('', { status: 403 }),
    Response.json({ token: 'expired', expires_at: new Date(0).toISOString() }), Response.json({})]) {
    globalThis.fetch = async () => response;
    await assert.rejects(githubAppToken(appEnv));
  }
  await assert.rejects(githubAppToken({ ...appEnv, GITHUB_APP_PRIVATE_KEY: '' }));
});

test('one request shares token acquisition; later requests mint a fresh token independently of the signing secret', async () => {
  let exchanges = 0, reads = 0;
  const signingSecret = 'stable-signing-key';
  const metadata = Buffer.from('{}').toString('base64url');
  const title = '[Request] Historical ticket', description = 'A ticket created before migration.';
  const issue = { title, body: `${description}\n<!-- sim-request:v1:${metadata}.${await sign(signingSecret, `issue:${title}\n${description}\n${metadata}`)} -->`,
    user: { login: 'hlyall01' }, labels: [{ name: 'site-request' }], state: 'open', comments: 150 };
  globalThis.fetch = async (url, options) => {
    if (url.endsWith('/access_tokens')) { exchanges++; return tokenResponse(); }
    reads++;
    assert.equal(options.headers.Authorization, 'Bearer test-installation-token');
    return Response.json(url.includes('/comments') ? [] : issue);
  };
  const env = { ...appEnv, REQUEST_SIGNING_SECRET: signingSecret };
  for (let i = 0; i < 2; i++) {
    const response = await worker.fetch(new Request('https://feature.sim-gui.com/api/tickets/7'), env);
    assert.equal(response.status, 200);
  }
  assert.equal(exchanges, 2);
  assert.equal(reads, 6);
  assert.equal(env.appToken, undefined);
});

test('both historical and app authors require valid signatures; unrelated bots are rejected', async () => {
  const secret = 'stable-secret', metadata = Buffer.from('{}').toString('base64url');
  const issue = { title: 'Feature request', body: `Description\n<!-- sim-request:v1:${metadata}.${await sign(secret, `issue:Feature request\nDescription\n${metadata}`)} -->` };
  const revision = { issue_url: 'https://api.github.com/repos/hlyall01/hackmaster-sim/issues/7',
    body: `Feedback\n<!-- sim-revision:v1:${metadata}.${await sign(secret, `revision:7\nFeedback\n${metadata}`)} -->` };
  for (const login of ['hlyall01', 'sim-gui-requests[bot]']) {
    assert.ok(await verifyIssue({ ...issue, user: { login } }, secret));
    assert.ok(await verifyRevision({ ...revision, user: { login } }, 7, secret));
    assert.equal(await verifyIssue({ ...issue, user: { login } }, 'different-key'), null);
  }
  assert.equal(await verifyIssue({ ...issue, user: { login: 'attacker[bot]' } }, secret), null);
  assert.equal(await verifyRevision({ ...revision, user: { login: 'attacker[bot]' } }, 7, secret), null);
});
