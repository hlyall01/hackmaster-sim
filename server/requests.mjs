// Production-only Pages Worker. Preview deployments never include this file or its secrets.
const REPO = 'hlyall01/hackmaster-sim';
const ORIGIN = 'https://sim-gui.com';
const LABEL = 'site-request';
const encoder = new TextEncoder();
const marker = /\n<!-- sim-request:v1:([A-Za-z0-9_-]+)\.([A-Za-z0-9_-]+) -->$/;
class HttpError extends Error {
  constructor(status, message) { super(message); this.status = status; }
}
const encode = bytes => btoa(String.fromCharCode(...bytes)).replaceAll('+', '-').replaceAll('/', '_').replace(/=+$/, '');
const decode = value => Uint8Array.from(atob(value.replaceAll('-', '+').replaceAll('_', '/')), c => c.charCodeAt(0));
const pack = value => encode(encoder.encode(JSON.stringify(value)));
const unpack = value => JSON.parse(new TextDecoder().decode(decode(value)));
async function key(secret) {
  return crypto.subtle.importKey('raw', encoder.encode(secret), { name: 'HMAC', hash: 'SHA-256' }, false, ['sign', 'verify']);
}
export async function sign(secret, message) {
  return encode(new Uint8Array(await crypto.subtle.sign('HMAC', await key(secret), encoder.encode(message))));
}
async function verify(secret, message, signature) {
  try { return await crypto.subtle.verify('HMAC', await key(secret), decode(signature), encoder.encode(message)); }
  catch { return false; }
}
export async function verifyIssue(issue, secret) {
  const match = issue.body?.match(marker);
  if (!match || issue.user?.login !== 'hlyall01' || issue.pull_request) return null;
  const body = issue.body.slice(0, match.index);
  if (!await verify(secret, `issue:${issue.title}\n${body}\n${match[1]}`, match[2])) return null;
  try { return { metadata: unpack(match[1]), description: body, title: issue.title }; }
  catch { return null; }
}
async function boundedJson(response, limit) {
  if (!response.body) throw new HttpError(400, 'A JSON body is required.');
  const reader = response.body.getReader();
  const chunks = [];
  let size = 0;
  try {
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      size += value.length;
      if (size > limit) throw new HttpError(413, 'The request is too large.');
      chunks.push(value);
    }
  } finally { await reader.cancel(); }
  const bytes = new Uint8Array(size);
  let offset = 0;
  for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length; }
  try { return JSON.parse(new TextDecoder().decode(bytes)); }
  catch { throw new HttpError(400, 'Invalid JSON.'); }
}
async function github(env, path, options = {}) {
  const response = await fetch(`https://api.github.com/repos/${REPO}${path}`, {
    ...options,
    headers: {
      Authorization: `Bearer ${env.GITHUB_ISSUES_TOKEN}`, Accept: 'application/vnd.github+json',
      'User-Agent': 'sim-gui-feature-requests', 'X-GitHub-Api-Version': '2022-11-28',
      'Content-Type': 'application/json',
    },
    signal: AbortSignal.timeout(15000), redirect: 'error',
  });
  if (!response.ok) {
    await response.body?.cancel();
    throw new HttpError(response.status === 404 ? 404 : 503,
      response.status === 404 ? 'Ticket not found.' : 'GitHub is temporarily unavailable. Please try again later.');
  }
  return boundedJson(response, 2_000_000);
}
function json(value, status = 200) {
  return Response.json(value, { status, headers: {
    'Cache-Control': 'no-store', 'X-Content-Type-Options': 'nosniff', 'Referrer-Policy': 'no-referrer',
  } });
}
function requireEnabled(env) {
  if (!env.GITHUB_ISSUES_TOKEN || env.FEATURE_REQUESTS_ENABLED !== 'true')
    throw new HttpError(503, 'Feature requests are not available yet. Please check back soon.');
}
async function fingerprint(request, secret) {
  const ip = request.headers.get('CF-Connecting-IP');
  if (!ip) throw new HttpError(400, 'Could not verify this connection.');
  // Only a keyed fingerprint is stored in GitHub; the address itself is never stored.
  return sign(secret, `ip:${ip}`);
}
function clean(value, min, max, name) {
  if (typeof value !== 'string') throw new HttpError(400, `${name} is required.`);
  const text = value.trim().replace(/<!--[\s\S]*?-->/g, '').replace(/[\u0000-\u0008\u000B\u000C\u000E-\u001F]/g, '');
  if (text.length < min || text.length > max) throw new HttpError(400, `${name} must contain ${min}–${max} characters.`);
  return text;
}
async function createRequest(request, env) {
  requireEnabled(env);
  if (request.headers.get('Origin') !== ORIGIN) throw new HttpError(403, 'Please submit from sim-gui.com.');
  if (!request.headers.get('Content-Type')?.startsWith('application/json')) throw new HttpError(415, 'Send JSON.');
  const input = await boundedJson(request, 16000);
  if (!input || typeof input !== 'object' || input.website) throw new HttpError(400, 'Could not accept this request.');
  const title = '[Request] ' + clean(input.title, 10, 120, 'Title').replace(/[\r\n]/g, ' ');
  const description = clean(input.description, 30, 6000, 'Description');
  const ipHash = await fingerprint(request, env.GITHUB_ISSUES_TOKEN);
  let challenge;
  const parts = typeof input.challenge === 'string' ? input.challenge.split('.') : [];
  if (parts.length !== 2 || input.challenge.length > 1000 ||
      !await verify(env.GITHUB_ISSUES_TOKEN, `challenge:${parts[0]}`, parts[1]))
    throw new HttpError(400, 'The form expired. Reload it and try again.');
  try { challenge = unpack(parts[0]); } catch { throw new HttpError(400, 'Invalid form token.'); }
  const age = Date.now() - challenge.t;
  if (challenge.ip !== ipHash || !Number.isFinite(age) || age < 3000 || age > 3600_000)
    throw new HttpError(400, 'Please wait a few seconds, or reload the form if it has expired.');
  const contentHash = await sign(env.GITHUB_ISSUES_TOKEN, `content:${title}\n${description}`);
  const since = new Date(Date.now() - 86400_000).toISOString();
  const recent = await github(env, `/issues?state=all&labels=${LABEL}&sort=created&direction=desc&since=${since}&per_page=100`);
  const verified = (await Promise.all(recent.map(async issue => ({ issue, proof: await verifyIssue(issue, env.GITHUB_ISSUES_TOKEN) }))))
    .filter(({ issue, proof }) => proof && Date.parse(issue.created_at) > Date.now() - 86400_000);
  const duplicate = verified.find(({ proof }) => proof.metadata.requestId === challenge.id || proof.metadata.contentHash === contentHash);
  if (duplicate) return json({ number: duplicate.issue.number, url: `/${duplicate.issue.number}`, duplicate: true });
  const own = verified.filter(({ proof }) => proof.metadata.ipHash === ipHash);
  if (own.length >= 5 || own.some(({ issue }) => Date.now() - Date.parse(issue.created_at) < 600_000))
    throw new HttpError(429, 'Please wait 10 minutes between requests. The daily limit is five per connection.');
  // Fail closed if the bounded history cannot provide a reliable cooldown check.
  if (recent.length >= 100) throw new HttpError(429, 'The request queue is busy. Please try again tomorrow.');
  const metadata = pack({ requestId: challenge.id, ipHash, contentHash });
  const body = 'Submitted through the public feature request form at https://sim-gui.com/.\n\n' + description;
  const signature = await sign(env.GITHUB_ISSUES_TOKEN, `issue:${title}\n${body}\n${metadata}`);
  const issue = await github(env, '/issues', { method: 'POST', body: JSON.stringify({
    title, body: `${body}\n<!-- sim-request:v1:${metadata}.${signature} -->`, labels: [LABEL],
  }) });
  return json({ number: issue.number, url: `/${issue.number}` }, 201);
}
async function ticket(env, number) {
  if (!env.GITHUB_ISSUES_TOKEN) throw new HttpError(503, 'Ticket tracking is temporarily unavailable.');
  const issue = await github(env, `/issues/${number}`);
  if (issue.pull_request || !issue.labels?.some(label => label.name === LABEL)) throw new HttpError(404, 'Ticket not found.');
  const page = Math.max(1, Math.ceil(issue.comments / 100));
  const comments = await github(env, `/issues/${number}/comments?per_page=100&page=${page}`);
  let state = { status: 'queued', message: 'Waiting for the coding agent.' };
  for (const comment of comments) {
    if (comment.user?.login !== 'github-actions[bot]') continue;
    const match = comment.body.match(/<!-- sim-status:(\{[^\n]+\}) -->/);
    if (!match) continue;
    try {
      const candidate = JSON.parse(match[1]);
      if (['coding', 'building', 'ready', 'failed', 'needs-info'].includes(candidate.status)) state = candidate;
    } catch { /* Ignore unrelated comments. */ }
  }
  const preview = typeof state.preview === 'string' && /^https:\/\/[a-z0-9-]+\.hackmaster-sim-previews\.pages\.dev\/?$/.test(state.preview) ? state.preview : null;
  const pr = typeof state.pr === 'string' && /^https:\/\/github\.com\/hlyall01\/hackmaster-sim\/pull\/\d+$/.test(state.pr) ? state.pr : null;
  return json({ number: Number(number), title: issue.title, status: issue.state === 'closed' ? 'closed' : state.status,
    message: String(state.message || '').slice(0, 1000), preview, pr,
    issue: `https://github.com/${REPO}/issues/${number}` });
}
export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    try {
      if (url.pathname === '/api/request-challenge' && request.method === 'GET') {
        requireEnabled(env);
        const payload = pack({ t: Date.now(), id: crypto.randomUUID(), ip: await fingerprint(request, env.GITHUB_ISSUES_TOKEN) });
        return json({ challenge: `${payload}.${await sign(env.GITHUB_ISSUES_TOKEN, `challenge:${payload}`)}` });
      }
      if (url.pathname === '/api/feature-requests' && request.method === 'POST') return await createRequest(request, env);
      const match = url.pathname.match(/^\/api\/tickets\/([1-9][0-9]{0,8})$/);
      if (match && request.method === 'GET') return await ticket(env, match[1]);
      if (url.pathname.startsWith('/api/')) return json({ error: 'Not found.' }, 404);
      if (/^\/[1-9][0-9]{0,8}\/?$/.test(url.pathname) && ['GET', 'HEAD'].includes(request.method)) {
        url.pathname = '/ticket';
        return env.ASSETS.fetch(new Request(url, request));
      }
      return env.ASSETS.fetch(request);
    } catch (error) {
      if (error instanceof HttpError) return json({ error: error.message }, error.status);
      const diagnostic = String(error?.message || '').split(env.GITHUB_ISSUES_TOKEN || '\0').join('[redacted]').slice(0, 300);
      console.error(JSON.stringify({ event: 'feature_request_error', type: error?.name || 'unknown', message: diagnostic }));
      return json({ error: 'Something went wrong. Please try again later.' }, 503);
    }
  },
};
