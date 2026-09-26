// Production-only Pages Worker. Preview deployments never include this file or its secrets.
const REPO = 'hlyall01/hackmaster-sim';
const ORIGIN = 'https://feature.sim-gui.com';
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
export async function verifyRevision(comment, number, secret) {
  const match = comment.body?.match(/\n<!-- sim-revision:v1:([A-Za-z0-9_-]+)\.([A-Za-z0-9_-]+) -->$/);
  if (!match || comment.user?.login !== 'hlyall01' ||
      !comment.issue_url?.endsWith(`/repos/${REPO}/issues/${number}`)) return null;
  const description = comment.body.slice(0, match.index);
  if (!await verify(secret, `revision:${number}\n${description}\n${match[1]}`, match[2])) return null;
  try { return { metadata: unpack(match[1]), description }; } catch { return null; }
}
// Preserve the last working preview while a newer revision is queued or building.
export async function ticketState(comments, number, secret) {
  let state = { status: 'queued', message: 'Waiting for the coding agent.', revision: 0 };
  const history = [];
  for (const comment of comments) {
    const revision = await verifyRevision(comment, number, secret);
    if (revision) {
      state = { ...state, status: 'queued', message: 'Your changes are waiting for the coding agent.', revision: comment.id };
      history.push({ id: comment.id, description: revision.description, created: comment.created_at });
      continue;
    }
    if (comment.user?.login !== 'github-actions[bot]') continue;
    const match = comment.body?.match(/<!-- sim-status:(\{[^\n]+\}) -->/);
    if (!match) continue;
    try {
      const next = JSON.parse(match[1]);
      if ((next.revision || 0) !== state.revision || !['screening', 'rejected', 'coding', 'building', 'ready', 'failed', 'needs-info'].includes(next.status)) continue;
      state = { ...state, status: next.status, message: String(next.message || '').slice(0, 1000) };
      if (/^https:\/\/[a-z0-9-]+\.hackmaster-sim-previews\.pages\.dev\/?$/.test(next.preview || '')) state.preview = next.preview;
      if (/^https:\/\/github\.com\/hlyall01\/hackmaster-sim\/pull\/\d+$/.test(next.pr || '')) state.pr = next.pr;
      if (/^[a-f0-9]{40}$/.test(next.sha || '')) state.sha = next.sha;
    } catch { /* Ignore unrelated comments. */ }
  }
  return { ...state, history: history.slice(-10) };
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
    // Cloudflare's edge runtime supports manual/follow, not the browser's "error" mode.
    // Treat redirects as failures below so Authorization can never cross hosts.
    signal: AbortSignal.timeout(15000), redirect: 'manual',
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
async function submission(request, env, scope) {
  requireEnabled(env);
  if (request.headers.get('Origin') !== ORIGIN) throw new HttpError(403, 'Please submit from feature.sim-gui.com.');
  if (!request.headers.get('Content-Type')?.startsWith('application/json')) throw new HttpError(415, 'Send JSON.');
  const input = await boundedJson(request, 16000);
  if (!input || typeof input !== 'object' || input.website) throw new HttpError(400, 'Could not accept this request.');
  const ipHash = await fingerprint(request, env.GITHUB_ISSUES_TOKEN);
  let challenge;
  const parts = typeof input.challenge === 'string' ? input.challenge.split('.') : [];
  if (parts.length !== 2 || input.challenge.length > 1000 ||
      !await verify(env.GITHUB_ISSUES_TOKEN, `challenge:${parts[0]}`, parts[1]))
    throw new HttpError(400, 'The form expired. Reload it and try again.');
  try { challenge = unpack(parts[0]); } catch { throw new HttpError(400, 'Invalid form token.'); }
  const age = Date.now() - challenge.t;
  if ((challenge.scope || 'request') !== scope || challenge.ip !== ipHash || !Number.isFinite(age) || age < 3000 || age > 3600_000)
    throw new HttpError(400, 'Please wait a few seconds, or reload the form if it has expired.');
  return { input, challenge, ipHash };
}
async function recentSubmissions(env) {
  const since = new Date(Date.now() - 86400_000).toISOString();
  const [issues, comments] = await Promise.all([
    github(env, `/issues?state=all&labels=${LABEL}&sort=created&direction=desc&since=${since}&per_page=100`),
    github(env, `/issues/comments?sort=created&direction=desc&since=${since}&per_page=100`),
  ]);
  // Fail closed rather than miss submissions outside the bounded history.
  if (issues.length >= 100 || comments.length >= 100) throw new HttpError(429, 'The request queue is busy. Please try again tomorrow.');
  const entries = await Promise.all([
    ...issues.map(async issue => ({ number: issue.number, created: issue.created_at, proof: await verifyIssue(issue, env.GITHUB_ISSUES_TOKEN) })),
    ...comments.map(async comment => {
      const number = comment.issue_url?.match(/\/issues\/([1-9][0-9]{0,8})$/)?.[1];
      return { number: Number(number), revision: comment.id, created: comment.created_at,
        proof: number ? await verifyRevision(comment, number, env.GITHUB_ISSUES_TOKEN) : null };
    }),
  ]);
  return entries.filter(entry => entry.proof && Date.parse(entry.created) > Date.now() - 86400_000);
}
function cooldown(entries, ipHash) {
  const own = entries.filter(entry => entry.proof.metadata.ipHash === ipHash);
  if (own.length >= 5 || own.some(entry => Date.now() - Date.parse(entry.created) < 600_000))
    throw new HttpError(429, 'Please wait 10 minutes between requests. The daily limit is five per connection.');
}
async function createRequest(request, env) {
  const { input, challenge, ipHash } = await submission(request, env, 'request');
  const title = '[Request] ' + clean(input.title, 10, 120, 'Title').replace(/[\r\n]/g, ' ');
  const description = clean(input.description, 30, 6000, 'Description');
  const contentHash = await sign(env.GITHUB_ISSUES_TOKEN, `content:${title}\n${description}`);
  const recent = await recentSubmissions(env);
  const duplicate = recent.find(entry => !entry.revision && (entry.proof.metadata.requestId === challenge.id || entry.proof.metadata.contentHash === contentHash));
  if (duplicate) return json({ number: duplicate.number, url: `/${duplicate.number}`, duplicate: true });
  cooldown(recent, ipHash);
  const metadata = pack({ requestId: challenge.id, ipHash, contentHash });
  const body = `Submitted through the public feature request form at ${ORIGIN}/.\n\n` + description;
  const signature = await sign(env.GITHUB_ISSUES_TOKEN, `issue:${title}\n${body}\n${metadata}`);
  const issue = await github(env, '/issues', { method: 'POST', body: JSON.stringify({
    title, body: `${body}\n<!-- sim-request:v1:${metadata}.${signature} -->`, labels: [LABEL],
  }) });
  return json({ number: issue.number, url: `/${issue.number}` }, 201);
}
async function loadTicket(env, number) {
  if (!env.GITHUB_ISSUES_TOKEN) throw new HttpError(503, 'Ticket tracking is temporarily unavailable.');
  const issue = await github(env, `/issues/${number}`);
  if (!await verifyIssue(issue, env.GITHUB_ISSUES_TOKEN) || !issue.labels?.some(label => label.name === LABEL)) throw new HttpError(404, 'Ticket not found.');
  const page = Math.max(1, Math.ceil(issue.comments / 100));
  const pages = page > 1 ? [page - 1, page] : [page];
  const comments = (await Promise.all(pages.map(p => github(env, `/issues/${number}/comments?per_page=100&page=${p}`)))).flat();
  const state = await ticketState(comments, number, env.GITHUB_ISSUES_TOKEN);
  return { issue, state };
}
async function ticket(env, number) {
  const { issue, state } = await loadTicket(env, number);
  const status = issue.state === 'closed' ? 'closed' : state.status;
  return json({ number: Number(number), title: issue.title, status, message: state.message,
    preview: state.preview || null, pr: state.pr || null, revision: state.revision,
    canRevise: !issue.locked && env.FEATURE_REQUESTS_ENABLED === 'true' && ['ready', 'failed', 'needs-info', 'rejected'].includes(status),
    history: state.history, issue: `https://github.com/${REPO}/issues/${number}` });
}
async function revise(request, env, number) {
  const { input, challenge, ipHash } = await submission(request, env, `revision:${number}`);
  const description = clean(input.description, 10, 6000, 'Changes');
  const { issue, state } = await loadTicket(env, number);
  const contentHash = await sign(env.GITHUB_ISSUES_TOKEN, `revision:${number}:${input.revision}\n${description}`);
  const recent = await recentSubmissions(env);
  const duplicate = recent.find(entry => entry.number === Number(number) && entry.revision &&
    (entry.proof.metadata.requestId === challenge.id || entry.proof.metadata.contentHash === contentHash));
  if (duplicate) return json({ number: Number(number), revision: duplicate.revision, duplicate: true });
  if (issue.state !== 'open' || issue.locked) throw new HttpError(409, 'This request is closed. Please start a new feature request.');
  if (!['ready', 'failed', 'needs-info', 'rejected'].includes(state.status)) throw new HttpError(409, 'A revision is already in progress. Please wait for it to finish.');
  if (input.revision !== state.revision) throw new HttpError(409, 'This request has changed. Refresh the page before submitting.');
  cooldown(recent, ipHash);
  const metadata = pack({ requestId: challenge.id, ipHash, contentHash, sha: state.sha || null, previousRevision: state.revision });
  const signature = await sign(env.GITHUB_ISSUES_TOKEN, `revision:${number}\n${description}\n${metadata}`);
  const comment = await github(env, `/issues/${number}/comments`, { method: 'POST', body: JSON.stringify({
    body: `${description}\n<!-- sim-revision:v1:${metadata}.${signature} -->`,
  }) });
  return json({ number: Number(number), revision: comment.id }, 201);
}
export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    try {
      const isTicket = /^\/[1-9][0-9]{0,8}\/?$/.test(url.pathname);
      const isApi = url.pathname.startsWith('/api/');
      if ((isTicket || isApi) && url.origin !== ORIGIN) {
        if (['GET', 'HEAD'].includes(request.method)) {
          return Response.redirect(`${ORIGIN}${url.pathname}${url.search}`, 308);
        }
        throw new HttpError(403, 'Please submit from feature.sim-gui.com.');
      }
      if (url.pathname === '/api/request-challenge' && request.method === 'GET') {
        requireEnabled(env);
        const scope = url.searchParams.get('scope') || 'request';
        if (scope !== 'request' && !/^revision:[1-9][0-9]{0,8}$/.test(scope)) throw new HttpError(400, 'Invalid form.');
        const payload = pack({ t: Date.now(), id: crypto.randomUUID(), scope, ip: await fingerprint(request, env.GITHUB_ISSUES_TOKEN) });
        return json({ challenge: `${payload}.${await sign(env.GITHUB_ISSUES_TOKEN, `challenge:${payload}`)}` });
      }
      if (url.pathname === '/api/feature-requests' && request.method === 'POST') return await createRequest(request, env);
      const match = url.pathname.match(/^\/api\/tickets\/([1-9][0-9]{0,8})$/);
      if (match && request.method === 'GET') return await ticket(env, match[1]);
      const revision = url.pathname.match(/^\/api\/tickets\/([1-9][0-9]{0,8})\/revisions$/);
      if (revision && request.method === 'POST') return await revise(request, env, revision[1]);
      if (url.pathname.startsWith('/api/')) return json({ error: 'Not found.' }, 404);
      if (isTicket && ['GET', 'HEAD'].includes(request.method)) {
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
