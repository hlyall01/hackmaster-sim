// Runs only from the trusted default branch; never execute this from a candidate PR.
import fs from 'node:fs';
import { verifyIssue } from '../server/requests.mjs';

const repo = 'hlyall01/hackmaster-sim';
const env = process.env;
const number = env.ISSUE_NUMBER;
if (!/^[1-9][0-9]{0,8}$/.test(number || '')) throw new Error('Invalid issue number');
async function api(path, method = 'GET', body) {
  const response = await fetch(`https://api.github.com/repos/${repo}${path}`, {
    method, headers: { Authorization: `Bearer ${env.GH_TOKEN}`, Accept: 'application/vnd.github+json',
      'X-GitHub-Api-Version': '2022-11-28', 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body), signal: AbortSignal.timeout(30000),
  });
  if (!response.ok) throw new Error(`GitHub ${method} failed: ${response.status}`);
  return response.status === 204 ? null : response.json();
}
function output(name, value) {
  if (String(value).includes('\n')) throw new Error('Unsafe output');
  fs.appendFileSync(env.GITHUB_OUTPUT, `${name}=${value}\n`);
}
async function comment(state) {
  // This marker is consumed only when GitHub reports github-actions[bot] as the author.
  const body = `${state.message}\n\n${state.pr ? `Pull request: ${state.pr}\n\n` : ''}` +
    `Track progress and preview: https://feature.sim-gui.com/${number}\n\n` +
    `<!-- sim-status:${JSON.stringify(state)} -->`;
  await api(`/issues/${number}/comments`, 'POST', { body });
}

switch (process.argv[2]) {
  case 'prepare': {
    output('accepted', 'false');
    if (!env.ISSUES_TOKEN || !env.OPENAI_CONFIGURED) throw new Error('Configure ISSUES_TOKEN and OPENAI_API_KEY first');
    const issue = await api(`/issues/${number}`);
    const proof = await verifyIssue(issue, env.ISSUES_TOKEN);
    if (!proof || issue.state !== 'open' || !issue.labels.some(label => label.name === 'site-request')) {
      console.log('Ignoring an unsigned, edited, closed, or non-site issue.');
      break;
    }
    fs.mkdirSync('target/request-input', { recursive: true });
    fs.writeFileSync('target/request-input/request.json', JSON.stringify({ number: Number(number), title: proof.title, description: proof.description }));
    output('accepted', 'true');
    await comment({ status: 'coding', message: 'The coding agent has started. It will propose changes in a pull request for review.' });
    break;
  }
  case 'prompt': {
    const request = JSON.parse(fs.readFileSync('target/request-input/request.json', 'utf8'));
    fs.mkdirSync('target/agent', { recursive: true });
    fs.writeFileSync('target/agent/prompt.md', fs.readFileSync('.github/codex/feature-request.md', 'utf8') + '\n' + JSON.stringify(request));
    fs.writeFileSync('target/agent/changes.patch', '');
    break;
  }
  case 'status': {
    const states = {
      building: 'The agent opened a pull request. Its preview is being built and tested.',
      ready: 'The preview is ready to try. The main site stays on the reviewed version until this pull request is merged.',
      failed: 'The automation could not finish successfully. Check the linked workflow run for details; the maintainer can retry it.',
      'needs-info': 'The agent could not make a safe, complete change. The request needs clarification or maintainer attention.',
    };
    if (!(env.REQUEST_STATUS in states)) throw new Error('Invalid status');
    const state = { status: env.REQUEST_STATUS, message: states[env.REQUEST_STATUS] };
    if (env.PR_URL && /^https:\/\/github\.com\/hlyall01\/hackmaster-sim\/pull\/\d+$/.test(env.PR_URL)) state.pr = env.PR_URL;
    if (env.PREVIEW_URL && /^https:\/\/[a-z0-9-]+\.hackmaster-sim-previews\.pages\.dev\/?$/.test(env.PREVIEW_URL)) state.preview = env.PREVIEW_URL;
    await comment(state);
    if (/^[a-f0-9]{40}$/.test(env.CANDIDATE_SHA || '')) await api(`/statuses/${env.CANDIDATE_SHA}`, 'POST', {
      state: env.REQUEST_STATUS === 'ready' ? 'success' : env.REQUEST_STATUS === 'building' ? 'pending' : 'failure',
      context: 'Feature preview', description: states[env.REQUEST_STATUS].slice(0, 140),
      target_url: `https://github.com/${repo}/actions/runs/${env.GITHUB_RUN_ID}`,
    });
    if (env.REQUEST_STATUS === 'failed') await api(`/issues/${number}/comments`, 'POST', {
      body: `Workflow run: https://github.com/${repo}/actions/runs/${env.GITHUB_RUN_ID}`,
    });
    break;
  }
  default: throw new Error('Unknown command');
}
