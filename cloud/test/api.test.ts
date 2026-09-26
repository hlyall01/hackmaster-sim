import { test } from 'node:test';
import assert from 'node:assert/strict';
import { DatabaseSync } from 'node:sqlite';
import { readFileSync } from 'node:fs';
import { generateKeyPair, exportJWK, SignJWT } from 'jose';
import worker, { characterApi } from '../src/worker.ts';

// Exercise actual migration SQL, constraints and triggers against SQLite, not a query-string mock.
function fixture() {
  const sql = new DatabaseSync(':memory:');
  sql.exec('PRAGMA foreign_keys=ON');
  sql.exec(readFileSync(new URL('../migrations/0001_characters.sql', import.meta.url), 'utf8'));
  const db = { prepare(query: string) {
    let args: (string | number | null)[] = [];
    return {
      bind(...values: (string | number | null)[]) { args = values; return this; },
      async run() { return { meta: sql.prepare(query).run(...args) }; },
      async first() { return sql.prepare(query).get(...args) ?? null; },
      async all() { return { results: sql.prepare(query).all(...args) }; },
    };
  } };
  const env = { DB: db, APP_ORIGIN: 'https://characters-test.sim-gui.com', ENVIRONMENT: 'character-test',
    ACCESS_ISSUER: 'https://test.cloudflareaccess.com', ACCESS_AUD: 'test-audience', BOOTSTRAP_ADMIN_EMAIL: 'admin@example.com' };
  const identities = { admin: { id: 'admin', email: 'admin@example.com' }, player: { id: 'player', email: 'player@example.com' }, stranger: { id: 'stranger', email: 'stranger@example.com' } };
  const request = (path: string, method = 'GET', input?: unknown, origin = env.APP_ORIGIN) => new Request(`${env.APP_ORIGIN}/api/${path}`, {
    method, headers: { 'Origin': origin, 'Content-Type': 'application/json' }, body: input === undefined ? undefined : JSON.stringify(input),
  });
  const call = async (who: keyof typeof identities, path: string, method = 'GET', input?: unknown, origin?: string) => {
    try { const response = await characterApi(request(path, method, input, origin), env, identities[who]); return { status: response.status, data: await response.json() }; }
    catch (e) { return { status: e.status, data: { error: e.message } }; }
  };
  return { sql, env, call, request };
}
const document = (name = 'Arthur') => ({ schema_version: 1, player: {
  name, weapon_id: 'Longsword', armor_id: 'None', shield_id: 'None', offhand_weapon_id: null, npc_preset: null,
  level: 12, environment: { darkness: true }, magic: { equipment: ['preserved'] }, miscellaneous: { custom: 19 },
} });
const saveBody = (version: number, doc = document()) => ({ version, document: doc, mutation_id: crypto.randomUUID() });

test('new players have no characters, cannot create or assign or list other users', async () => {
  const { call } = fixture();
  assert.equal((await call('player', 'session')).data.user.admin, false);
  assert.deepEqual((await call('player', 'characters')).data, []);
  assert.equal((await call('player', 'users')).status, 403);
  assert.equal((await call('player', 'characters', 'POST', saveBody(1))).status, 403);
});

test('assignment, complete saves, conflicts, revision recovery and revoked access', async () => {
  const { call } = fixture();
  await call('player', 'session');
  const created = await call('admin', 'characters', 'POST', saveBody(1));
  assert.equal(created.status, 201);
  const id = created.data.id;
  assert.equal((await call('player', `characters/${id}`)).status, 404);
  assert.equal((await call('admin', `characters/${id}/assignments`, 'PUT', { user_id: 'player' })).status, 200);
  assert.equal((await call('player', 'characters')).data.length, 1);
  const revised = document('Arthur revised');
  assert.equal((await call('player', `characters/${id}`, 'PUT', saveBody(1, revised))).status, 200);
  assert.deepEqual((await call('player', `characters/${id}`)).data.document, revised);
  assert.equal((await call('player', `characters/${id}`, 'PUT', saveBody(1))).status, 409);
  assert.equal((await call('stranger', `characters/${id}/export`)).status, 404);
  assert.equal((await call('player', `characters/${id}/assignments`, 'PUT', { user_id: 'stranger' })).status, 403);
  assert.equal((await call('player', `characters/${id}/restore`, 'POST', { ...saveBody(2), restore_version: 1 })).status, 403);
  assert.equal((await call('admin', `characters/${id}/revisions`)).data.length, 2);
  const restored = await call('admin', `characters/${id}/restore`, 'POST', { ...saveBody(2), restore_version: 1 });
  assert.equal(restored.data.version, 3);
  assert.deepEqual(restored.data.document, document());
  await call('admin', `characters/${id}/assignments`, 'DELETE', { user_id: 'player' });
  assert.equal((await call('player', `characters/${id}`, 'PUT', saveBody(3))).status, 404);
  assert.equal((await call('player', `characters/${id}`)).status, 404);
});

test('save retries are idempotent, including a creation retry after later edits', async () => {
  const { call } = fixture();
  const input = saveBody(1);
  const original = (await call('admin', 'characters', 'POST', input)).data;
  const edit = saveBody(1, document('Edited'));
  assert.equal((await call('admin', `characters/${original.id}`, 'PUT', edit)).data.version, 2);
  assert.equal((await call('admin', `characters/${original.id}`, 'PUT', edit)).data.version, 2);
  assert.equal((await call('admin', 'characters', 'POST', input)).data.id, original.id);
  assert.equal((await call('admin', 'characters')).data.length, 1);
  assert.equal((await call('admin', `characters/${original.id}/revisions`)).data.length, 2);
});

test('history failure rolls back the character write', async () => {
  const { call, sql } = fixture();
  const { id } = (await call('admin', 'characters', 'POST', saveBody(1))).data;
  sql.exec("CREATE TRIGGER reject_revision BEFORE INSERT ON revisions WHEN NEW.version=2 BEGIN SELECT RAISE(ABORT,'test failure'); END");
  const failed = await call('admin', `characters/${id}`, 'PUT', saveBody(1, document('Lost')));
  assert.ok(failed.data.error.includes('test failure'));
  const current = (await call('admin', `characters/${id}`)).data;
  assert.equal(current.version, 1);
  assert.equal(current.document.player.name, 'Arthur');
});

test('revocation between the permission read and write cannot save a character', async () => {
  const { call, sql, env } = fixture();
  await call('player', 'session');
  const { id } = (await call('admin', 'characters', 'POST', saveBody(1))).data;
  await call('admin', `characters/${id}/assignments`, 'PUT', { user_id: 'player' });
  const prepare = env.DB.prepare.bind(env.DB);
  env.DB.prepare = query => {
    const statement = prepare(query);
    const first = statement.first.bind(statement);
    if (query.startsWith('UPDATE characters')) statement.first = async () => {
      sql.prepare('DELETE FROM assignments WHERE character_id=? AND user_id=?').run(id, 'player');
      return first();
    };
    return statement;
  };
  assert.equal((await call('player', `characters/${id}`, 'PUT', saveBody(1))).status, 404);
  assert.equal((await call('admin', `characters/${id}`)).data.version, 1);
});

test('concurrent edits produce one commit and one conflict; older history can be paged', async () => {
  const { call } = fixture();
  const { id } = (await call('admin', 'characters', 'POST', saveBody(1))).data;
  const attempts = await Promise.all([
    call('admin', `characters/${id}`, 'PUT', saveBody(1, document('A'))),
    call('admin', `characters/${id}`, 'PUT', saveBody(1, document('B'))),
  ]);
  assert.deepEqual(attempts.map(a => a.status).sort(), [200, 409]);
  const older = await call('admin', `characters/${id}/revisions?before=2`);
  assert.deepEqual(older.data.map(r => r.version), [1]);
});

test('cross-origin writes, malformed documents and alternative hostnames are rejected', async () => {
  const { call, request, env } = fixture();
  assert.equal((await call('admin', 'characters', 'POST', saveBody(1), 'https://evil.example')).status, 403);
  assert.equal((await call('admin', 'characters', 'POST', { ...saveBody(1), document: { schema_version: 2, player: {} } })).status, 400);
  await assert.rejects(characterApi(new Request('https://alternative.workers.dev/api/session'), env, { id: 'admin', email: 'admin@example.com' }), /hostname/);
  assert.equal((await worker.fetch(request('session'), env)).status, 401);
  assert.equal((await worker.fetch(request('session'), { ...env, ACCESS_AUD: '' })).status, 503);
});

test('real signed Access tokens: reject forgery, expiry, wrong audience, issuer and missing claims', async () => {
  const { env } = fixture();
  const { privateKey, publicKey } = await generateKeyPair('RS256');
  const jwk = { ...await exportJWK(publicKey), kid: 'fixture', alg: 'RS256', use: 'sig' };
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async (url) => {
    assert.equal(String(url), 'https://test.cloudflareaccess.com/cdn-cgi/access/certs');
    return Response.json({ keys: [jwk] });
  };
  try {
    const token = (overrides = {}) => new SignJWT({ sub: 'signed', email: 'signed@example.com', iat: Math.floor(Date.now() / 1000),
      exp: Math.floor(Date.now() / 1000) + 300, iss: env.ACCESS_ISSUER, aud: [env.ACCESS_AUD], ...overrides })
      .setProtectedHeader({ alg: 'RS256', kid: 'fixture' }).sign(privateKey);
    const send = async (jwt: string) => worker.fetch(new Request(`${env.APP_ORIGIN}/api/session`, { headers: { 'Cf-Access-Jwt-Assertion': jwt } }), env);
    assert.equal((await send(await token())).status, 200);
    for (const change of [{ exp: 1 }, { aud: ['another-app'] }, { iss: 'https://other.cloudflareaccess.com' }, { email: undefined }, { sub: '' }]) {
      assert.equal((await send(await token(change))).status, 401);
    }
    const signed = await token();
    assert.equal((await send(signed.slice(0, -5) + 'AAAAA')).status, 401);
  } finally { globalThis.fetch = originalFetch; }
});
