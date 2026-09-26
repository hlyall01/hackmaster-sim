import { createRemoteJWKSet, jwtVerify } from 'jose';

// Public signing keys only; no per-user or per-request state is cached here.
const keySets = new Map<string, ReturnType<typeof createRemoteJWKSet>>();
type Identity = { id: string; email: string };
type RuntimeEnv = { [K in keyof Env]: Env[K] extends string ? string : Env[K] };
type CharacterRow = { id: string; name: string; document: string; version: number; updated_by: string; updated_at: string; mutation_id: string };
class HttpError extends Error {
  status: number;
  constructor(status: number, message: string) { super(message); this.status = status; }
}
const fail = (status: number, message: string): never => { throw new HttpError(status, message); };
const json = (body: unknown, status = 200) => Response.json(body, { status, headers: {
  'Cache-Control': 'no-store', 'X-Content-Type-Options': 'nosniff',
  'Referrer-Policy': 'no-referrer',
} });

export async function authenticate(request: Request, env: RuntimeEnv): Promise<Identity> {
  if (!/^https:\/\/[a-z0-9-]+\.cloudflareaccess\.com$/.test(env.ACCESS_ISSUER) || !env.ACCESS_AUD) {
    fail(503, 'Test login is not configured yet.');
  }
  const token = request.headers.get('Cf-Access-Jwt-Assertion');
  if (!token || token.length > 16384) fail(401, 'Sign in to continue.');
  let keys = keySets.get(env.ACCESS_ISSUER);
  if (!keys) {
    keys = createRemoteJWKSet(new URL(`${env.ACCESS_ISSUER}/cdn-cgi/access/certs`), { timeoutDuration: 5000 });
    keySets.set(env.ACCESS_ISSUER, keys);
  }
  try {
    const { payload } = await jwtVerify(token!, keys, {
      issuer: env.ACCESS_ISSUER, audience: env.ACCESS_AUD, algorithms: ['RS256'],
      requiredClaims: ['sub', 'email', 'exp', 'iat'],
    });
    if (typeof payload.sub !== 'string' || !payload.sub || payload.sub.length > 255 ||
        typeof payload.email !== 'string' || payload.email.length > 320 || !payload.email.includes('@')) {
      fail(401, 'Invalid login identity.');
    }
    return { id: payload.sub!, email: String(payload.email).toLowerCase() };
  } catch { return fail(401, 'Your session expired or could not be verified. Sign in again.'); }
}

async function body(request: Request): Promise<Record<string, unknown>> {
  if (!(request.headers.get('Content-Type') || '').startsWith('application/json')) fail(415, 'JSON required.');
  const reader = request.body?.getReader();
  if (!reader) fail(400, 'Request body required.');
  const chunks: Uint8Array[] = [];
  let size = 0;
  while (true) {
    const item = await reader!.read();
    if (item.done) break;
    size += item.value.length;
    if (size > 256 * 1024) { await reader!.cancel(); fail(413, 'Character is too large.'); }
    chunks.push(item.value);
  }
  const bytes = new Uint8Array(size);
  let offset = 0;
  for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length; }
  try {
    const data: unknown = JSON.parse(new TextDecoder().decode(bytes));
    if (!data || typeof data !== 'object' || Array.isArray(data)) fail(400, 'JSON object required.');
    return data as Record<string, unknown>;
  } catch { return fail(400, 'Invalid JSON.'); }
}
function documentValue(value: unknown): { text: string; name: string } {
  if (!value || typeof value !== 'object' || Array.isArray(value)) fail(400, 'Character document required.');
  const doc = value as Record<string, unknown>;
  if (doc.schema_version !== 1 || !doc.player || typeof doc.player !== 'object' || Array.isArray(doc.player) ||
      Object.keys(doc).some(k => !['schema_version', 'player'].includes(k))) fail(400, 'Unsupported character document.');
  const p = doc.player as Record<string, unknown>;
  if (typeof p.name !== 'string' || !p.name.trim() || p.name.length > 120) fail(400, 'Character name must contain 1–120 characters.');
  for (const key of ['weapon_id', 'armor_id', 'shield_id']) {
    if (typeof p[key] !== 'string' || !p[key] || String(p[key]).length > 200) fail(400, 'Stable equipment names required.');
  }
  for (const key of ['offhand_weapon_id', 'npc_preset']) {
    if (p[key] !== null && (typeof p[key] !== 'string' || String(p[key]).length > 200)) fail(400, 'Invalid catalog reference.');
  }
  if ('fighter_preset' in p || 'active_weapon_style_ids' in p) fail(400, 'Runtime state cannot be saved as a character.');
  const text = JSON.stringify(doc);
  if (text.length > 200000) fail(413, 'Character is too large.');
  return { text, name: String(p.name).trim() };
}
function integer(value: unknown): number {
  if (!Number.isSafeInteger(value) || Number(value) < 1) fail(400, 'A valid version is required.');
  return Number(value);
}
function mutation(value: unknown): string {
  if (typeof value !== 'string' || !/^[a-f0-9-]{36}$/.test(value)) fail(400, 'A mutation UUID is required.');
  return String(value);
}
function unpack(row: CharacterRow) { return { ...row, document: JSON.parse(row.document) }; }

// Only fetch() calls this in the deployed Worker, after token verification.
export async function characterApi(request: Request, env: RuntimeEnv, identity: Identity): Promise<Response> {
  const url = new URL(request.url);
  if (url.origin !== env.APP_ORIGIN) fail(403, 'This hostname is not enabled for character access.');
  if (!['GET', 'POST', 'PUT', 'DELETE'].includes(request.method)) fail(405, 'Method not allowed.');
  if (request.method !== 'GET' && request.headers.get('Origin') !== env.APP_ORIGIN) fail(403, 'Same-origin request required.');
  const db = env.DB;
  await db.prepare('INSERT INTO users(id,email) VALUES(?,?) ON CONFLICT(id) DO UPDATE SET email=excluded.email')
    .bind(identity.id, identity.email).run();
  if (env.BOOTSTRAP_ADMIN_EMAIL && identity.email === env.BOOTSTRAP_ADMIN_EMAIL.toLowerCase()) {
    await db.prepare('INSERT OR IGNORE INTO administrators(user_id) VALUES(?)').bind(identity.id).run();
  }
  const admin = !!await db.prepare('SELECT 1 FROM administrators WHERE user_id=?').bind(identity.id).first();
  const adminOnly = () => { if (!admin) fail(403, 'Administrator access required.'); };
  const canEdit = async (id: string) => {
    const row = await db.prepare(`SELECT c.* FROM characters c WHERE c.id=? AND
      (?=1 OR EXISTS(SELECT 1 FROM character_owners o WHERE o.character_id=c.id AND o.user_id=?) OR EXISTS(SELECT 1 FROM assignments a WHERE a.character_id=c.id AND a.user_id=?))`)
      .bind(id, Number(admin), identity.id, identity.id).first<CharacterRow>();
    if (!row) fail(404, 'Character not found or access was revoked.');
    return row!;
  };
  const path = url.pathname.replace(/\/$/, '');
  if (path === '/api/session' && request.method === 'GET') return json({ user: { ...identity, admin }, environment: env.ENVIRONMENT });
  if (path === '/api/users' && request.method === 'GET') {
    adminOnly();
    return json((await db.prepare('SELECT id,email FROM users ORDER BY email').all()).results);
  }
  // Signed-in members can use the shared roster in simulations. No account emails,
  // identity IDs or revision metadata are exposed by these read-only endpoints.
  if (path === '/api/roster' && request.method === 'GET') {
    const rows = (await db.prepare(`SELECT c.id,c.name,c.version,
      EXISTS(SELECT 1 FROM character_owners o WHERE o.character_id=c.id AND o.user_id=?) AS is_owner,
      (?=1 OR EXISTS(SELECT 1 FROM character_owners o WHERE o.character_id=c.id AND o.user_id=?) OR
      EXISTS(SELECT 1 FROM assignments a WHERE a.character_id=c.id AND a.user_id=?)) AS can_edit
      FROM characters c ORDER BY c.name,c.id`).bind(identity.id, Number(admin), identity.id, identity.id).all()).results;
    return json(rows);
  }
  const rosterId = /^\/api\/roster\/([a-f0-9-]{36})$/.exec(path)?.[1];
  if (rosterId && request.method === 'GET') {
    const row = await db.prepare(`SELECT c.id,c.name,c.version,c.document,
      EXISTS(SELECT 1 FROM character_owners o WHERE o.character_id=c.id AND o.user_id=?) AS is_owner,
      (?=1 OR EXISTS(SELECT 1 FROM character_owners o WHERE o.character_id=c.id AND o.user_id=?) OR
      EXISTS(SELECT 1 FROM assignments a WHERE a.character_id=c.id AND a.user_id=?)) AS can_edit
      FROM characters c WHERE c.id=?`).bind(identity.id, Number(admin), identity.id, identity.id, rosterId).first<CharacterRow>();
    if (!row) fail(404, 'Character not found.');
    return json(unpack(row!));
  }
  if (path === '/api/characters' && request.method === 'GET') {
    return json((await db.prepare(`SELECT c.id,c.name,c.version,c.updated_at FROM characters c WHERE
      ?=1 OR EXISTS(SELECT 1 FROM character_owners o WHERE o.character_id=c.id AND o.user_id=?) OR EXISTS(SELECT 1 FROM assignments a WHERE a.character_id=c.id AND a.user_id=?) ORDER BY c.name,c.id`)
      .bind(Number(admin), identity.id, identity.id).all()).results);
  }
  if (path === '/api/characters' && request.method === 'POST') {
    const input = await body(request);
    const doc = documentValue(input.document);
    const mutationId = mutation(input.mutation_id);
    const existing = await db.prepare('SELECT *,character_id AS id FROM revisions WHERE mutation_id=? AND updated_by=?').bind(mutationId, identity.id).first<CharacterRow>();
    if (existing) return json(unpack(existing));
    const id = crypto.randomUUID();
    await db.prepare('INSERT INTO characters VALUES(?,?,?,1,?,?,?)')
      .bind(id, doc.name, doc.text, identity.id, new Date().toISOString(), mutationId).run();
    return json(unpack(await canEdit(id)), 201);
  }
  const match = /^\/api\/characters\/([a-f0-9-]{36})(?:\/(assignments|revisions|restore|export))?$/.exec(path);
  if (!match) fail(404, 'Unknown endpoint.');
  const [, id, action] = match!;
  const current = await canEdit(id);
  if (!action && request.method === 'GET') return json(unpack(current));
  if (action === 'export' && request.method === 'GET') return json({ schema_version: 1, character: unpack(current) });
  if (action === 'revisions' && request.method === 'GET') {
    adminOnly();
    const before = url.searchParams.has('before') ? integer(Number(url.searchParams.get('before'))) : Number.MAX_SAFE_INTEGER;
    return json((await db.prepare('SELECT version,name,updated_by,updated_at FROM revisions WHERE character_id=? AND version<? ORDER BY version DESC LIMIT 200')
      .bind(id, before).all()).results);
  }
  if (action === 'assignments') {
    adminOnly();
    if (request.method === 'GET') return json((await db.prepare('SELECT u.id,u.email,EXISTS(SELECT 1 FROM character_owners o WHERE o.character_id=? AND o.user_id=u.id) AS is_owner FROM users u WHERE EXISTS(SELECT 1 FROM assignments a WHERE a.character_id=? AND a.user_id=u.id) OR EXISTS(SELECT 1 FROM character_owners o WHERE o.character_id=? AND o.user_id=u.id) ORDER BY u.email').bind(id, id, id).all()).results);
    const input = await body(request);
    if (typeof input.user_id !== 'string') fail(400, 'User ID required.');
    if (!await db.prepare('SELECT 1 FROM users WHERE id=?').bind(input.user_id).first()) fail(404, 'The player must sign in first.');
    if (request.method === 'PUT') await db.prepare('INSERT OR IGNORE INTO assignments VALUES(?,?,?)').bind(id, input.user_id, identity.id).run();
    else if (request.method === 'DELETE') await db.prepare('DELETE FROM assignments WHERE character_id=? AND user_id=?').bind(id, input.user_id).run();
    else fail(405, 'Method not allowed.');
    return json({ saved: true });
  }
  if ((!action && request.method === 'PUT') || (action === 'restore' && request.method === 'POST')) {
    if (action === 'restore') adminOnly();
    const input = await body(request);
    const expected = integer(input.version);
    const mutationId = mutation(input.mutation_id);
    // A retry after a lost response must not create a second revision.
    const prior = await db.prepare('SELECT * FROM revisions WHERE character_id=? AND mutation_id=? AND updated_by=?').bind(id, mutationId, identity.id).first<CharacterRow>();
    if (prior) return json({ ...unpack(prior), id });
    let doc: { text: string; name: string };
    if (action === 'restore') {
      const historical = await db.prepare('SELECT document FROM revisions WHERE character_id=? AND version=?').bind(id, integer(input.restore_version)).first<{ document: string }>();
      if (!historical) fail(404, 'Revision not found.');
      doc = documentValue(JSON.parse(historical!.document));
    } else doc = documentValue(input.document);
    // Check assignment again inside the write, so revocation cannot race the earlier read.
    const saved = await db.prepare(`UPDATE characters SET name=?,document=?,version=version+1,updated_by=?,updated_at=?,mutation_id=?
      WHERE id=? AND version=? AND (EXISTS(SELECT 1 FROM administrators WHERE user_id=?) OR
      EXISTS(SELECT 1 FROM character_owners WHERE character_id=? AND user_id=?) OR
      EXISTS(SELECT 1 FROM assignments WHERE character_id=? AND user_id=?)) RETURNING *`)
      .bind(doc.name, doc.text, identity.id, new Date().toISOString(), mutationId, id, expected, identity.id, id, identity.id, id, identity.id).first<CharacterRow>();
    if (!saved) {
      await canEdit(id);
      fail(409, 'This character has changed. Your draft is preserved; load the latest version before saving again.');
    }
    return json(unpack(saved!));
  }
  return fail(405, 'Method not allowed.');
}

export default {
  async fetch(request: Request, env: RuntimeEnv): Promise<Response> {
    try {
      const identity = await authenticate(request, env);
      return await characterApi(request, env, identity);
    } catch (error) {
      if (error instanceof HttpError) return json({ error: error.message }, error.status);
      console.error(JSON.stringify({ event: 'character_api_failure', name: error instanceof Error ? error.name : 'unknown' }));
      return json({ error: 'The save service could not complete the request. Your draft has not been discarded.' }, 500);
    }
  },
};
