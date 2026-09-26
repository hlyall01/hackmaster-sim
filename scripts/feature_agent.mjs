// Runs only from the trusted default branch; never execute this from a candidate PR.
import fs from 'node:fs';
import { execFileSync } from 'node:child_process';
import { verifyIssue, verifyRevision } from '../server/requests.mjs';
import { screenFeature } from './screen_feature.mjs';

const repo = 'hlyall01/hackmaster-sim';
const env = process.env;
const number = env.ISSUE_NUMBER;
if (!/^[1-9][0-9]{0,8}$/.test(number || '')) throw new Error('Invalid issue number');
async function api(path, method = 'GET', body) {
  const response = await fetch(`https://api.github.com/repos/${repo}${path}`, {
    method, headers: { Authorization: `Bearer ${env.GH_TOKEN}`, Accept: 'application/vnd.github+json',
      'X-GitHub-Api-Version': '2022-11-28', 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body), signal: AbortSignal.timeout(30000), redirect: 'error',
  });
  if (!response.ok) throw new Error(`GitHub ${method} failed: ${response.status}`);
  return response.status === 204 ? null : response.json();
}
function output(name, value) {
  if (String(value).includes('\n')) throw new Error('Unsafe output');
  fs.appendFileSync(env.GITHUB_OUTPUT, `${name}=${value}\n`);
}
async function comment(state) {
  state.revision = Number(env.REVISION_ID || 0);
  state.run = env.GITHUB_RUN_ID;
  state.attempt = env.GITHUB_RUN_ATTEMPT;
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
    if (!proof || issue.state !== 'open' || issue.locked || !issue.labels.some(label => label.name === 'site-request')) {
      console.log('Ignoring an unsigned, edited, closed, or non-site issue.');
      break;
    }
    const revisionId = env.REVISION_ID || '';
    if (revisionId && !/^[1-9][0-9]{0,15}$/.test(revisionId)) throw new Error('Invalid revision');
    const comments = [];
    // Bound both the context and replay check. Very long tickets need maintainer attention.
    if (issue.comments > 1000) throw new Error('Ticket history is too long');
    for (let page = 1; page <= Math.max(1, Math.ceil(issue.comments / 100)); page++)
      comments.push(...await api(`/issues/${number}/comments?per_page=100&page=${page}`));
    const revisions = [];
    for (const item of comments) {
      const signed = await verifyRevision(item, number, env.ISSUES_TOKEN);
      if (signed) revisions.push({ id: item.id, ...signed });
    }
    const revision = revisions.find(item => String(item.id) === revisionId);
    if (revisionId && (!revision || revisions.at(-1)?.id !== revision.id)) {
      console.log('Ignoring an unsigned, edited, or superseded revision.'); break;
    }
    // Event redelivery or rerunning a completed revision must not spend another agent run.
    if (revisionId && comments.some(item => {
      if (item.user?.login !== 'github-actions[bot]') return false;
      try {
        const status = JSON.parse(item.body.match(/<!-- sim-status:(\{[^\n]+\}) -->/)?.[1] || '{}');
        return status.revision === revision.id && ['ready', 'needs-info', 'rejected'].includes(status.status);
      } catch { return false; }
    })) { console.log('Revision already completed.'); break; }
    const pulls = await api(`/pulls?state=all&head=hlyall01:codex/request-${number}&base=main&per_page=10`);
    const pr = pulls[0];
    if (pr && (pr.state !== 'open' || pr.head.repo?.full_name !== repo)) {
      await comment({ status: 'needs-info', message: 'The proposed changes have been closed or merged. Please start a new feature request.' }); break;
    }
    if (!revisionId && (pr || revisions.length)) { console.log('Use a new signed follow-up to revise this ticket.'); break; }
    if (revision?.metadata.sha && revision.metadata.sha !== pr?.head.sha) {
      await comment({ status: 'needs-info', sha: pr?.head.sha, pr: pr?.html_url,
        message: 'The branch changed after this feedback was submitted. Review the latest proposed changes and send another revision.' }); break;
    }
    const base = execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
    const source = pr?.head.sha || base;
    if (pr) {
      if (!/^[a-f0-9]{40}$/.test(source)) throw new Error('Invalid PR head');
      try {
        execFileSync('git', ['fetch', 'origin', source], { stdio: 'inherit' });
        execFileSync('python3', ['scripts/publish_feature.py', 'validate-source', base, source], { stdio: 'inherit' });
      } catch {
        await comment({ status: 'needs-info', message: 'This branch needs maintainer attention before another automated revision can run.' });
        break;
      }
    }
    fs.mkdirSync('target/request-input', { recursive: true });
    const request = { number: Number(number), title: proof.title, description: proof.description,
      previousRevisions: revisions.filter(item => item.id !== revision?.id).slice(-10).map(item => item.description),
      revision: revision?.description || null };
    fs.writeFileSync('target/request-input/prompt.md', fs.readFileSync('.github/codex/feature-request.md', 'utf8') + '\n' + JSON.stringify(request));
    fs.writeFileSync('target/request-input/request.json', JSON.stringify(request));
    output('source', source);
    output('pr_number', pr?.number || '');
    output('accepted', 'true');
    await comment({ status: 'screening', pr: pr?.html_url, sha: pr?.head.sha,
      message: 'Checking that this improves sim-gui and is small enough for one coding run.' });
    break;
  }
  case 'screen': {
    output('approved', 'false');
    try {
      const request = JSON.parse(fs.readFileSync('target/request-input/request.json', 'utf8'));
      const result = await screenFeature(request, env.OPENAI_API_KEY);
      if (result.decision !== 'accept') {
        await comment({ status: result.decision === 'reject' ? 'rejected' : 'needs-info',
          message: `${result.decision === 'reject' ? 'Request rejected.' : result.scope === 'large' ? 'Please choose a smaller first step.' : 'More detail needed.'} ${result.reason} No coding run was started. You can submit corrected feedback below.` });
        break;
      }
      await comment({ status: 'coding', message: 'This request passed the relevance and small-scope checks. The coding agent is starting; any existing preview stays available.' });
      output('approved', 'true');
    } catch {
      // Never turn an API error, truncated response or invalid result into permission to build.
      await comment({ status: 'failed', message: 'Request screening is temporarily unavailable. No coding run was started. Please retry later.' });
      process.exitCode = 1;
    }
    break;
  }
  case 'status': {
    const states = {
      building: 'The proposed changes have been updated. The preview is being built and tested.',
      ready: 'The preview is ready to try. The main site stays on the reviewed version until this pull request is merged.',
      failed: 'The automation could not finish successfully. Check the linked workflow run for details; the maintainer can retry it.',
      'needs-info': 'The agent could not make a safe, complete change. The request needs clarification or maintainer attention.',
    };
    if (!(env.REQUEST_STATUS in states)) throw new Error('Invalid status');
    const state = { status: env.REQUEST_STATUS, message: states[env.REQUEST_STATUS] };
    if (env.REQUEST_STATUS === 'needs-info' && env.AGENT_SUMMARY) state.message += ' Agent feedback: ' + env.AGENT_SUMMARY.slice(0, 1000);
    if (/^[a-f0-9]{40}$/.test(env.CANDIDATE_SHA || '')) state.sha = env.CANDIDATE_SHA;
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
