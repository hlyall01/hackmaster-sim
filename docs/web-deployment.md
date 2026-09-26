# Deploy the simulator and feature requests

Production is https://sim-gui.com/. Every push/merge into `main` runs native tests,
checks the Rust targets, builds WASM, and publishes the successful build to the
`hackmaster-sim` Cloudflare Pages project. Pull requests do not publish production.

Feature requests live at https://feature.sim-gui.com/. Both custom domains attach
to the same production Pages project; `feature.sim-gui.com` has a proxied CNAME to
`hackmaster-sim.pages.dev`. Its landing page opens the form without downloading
WASM. The web simulator has no Feature requests tab; open the feature site directly.
The native desktop app retains its link to the feature site.
Old `sim-gui.com/N` ticket links redirect permanently to `feature.sim-gui.com/N`;
the old `sim-gui.com/?tab=requests` link redirects in the browser. The API accepts
submissions only on the feature host and from that exact origin. Preview builds
remain on the isolated `hackmaster-sim-previews.pages.dev` origins.
Production assets use `max-age=0, must-revalidate` because their filenames are
stable across deployments. The feature-domain migration also versions the main
and form script URLs to invalidate copies already cached under the old policy.

## Credentials and settings

GitHub repository secrets:

| Name | Scope and destination |
| --- | --- |
| `CLOUDFLARE_API_TOKEN` | Cloudflare Pages Edit in the hosting account |
| `OPENAI_API_KEY` | OpenAI `sim-gui` project; List models Read, Responses Write |
| `ISSUES_TOKEN` | Fine-grained GitHub token; only `hlyall01/hackmaster-sim`, Issues read/write |

GitHub reserves names beginning with `GITHUB_`. The deployment copies
`ISSUES_TOKEN` into the production Pages binding `GITHUB_ISSUES_TOKEN` as an
**encrypted secret**. It is never bundled into static assets. OpenAI's key is
used only by the coding job's protected API proxy.

Repository variables:

| Name | Value |
| --- | --- |
| `CLOUDFLARE_ACCOUNT_ID` | Hosting account ID |
| `CLOUDFLARE_PAGES_PROJECT` | `hackmaster-sim` |
| `FEATURE_REQUESTS_ENABLED` | `true` to accept and process requests |
| `FEATURE_AGENT_MODEL` | Optional Codex model override; empty uses the action default |

`CLOUDFLARE_DEPLOY_BRANCH` is obsolete: production is always `main`.
GitHub Actions must be allowed to create pull requests in repository settings.
The `site-request` label must exist before accepting submissions.

The OpenAI `sim-gui` project is configured with a **US$10 monthly hard limit**.
Enforcement may slightly lag usage. Existing prepaid credit is shared across
projects; auto-reload is off. Check billing before increasing request volume.
The current issue token expires **26 October 2026**. Replace `ISSUES_TOKEN` before
then and rerun production deployment to update the Cloudflare binding. Existing
form challenges become invalid when the token changes; users can reload.

## Feature request flow

1. Open https://feature.sim-gui.com/ directly.
2. The production Pages Worker validates a honeypot, payload limits, exact origin,
   and a signed, IP-bound challenge with a three-second minimum age and one-hour
   lifetime. It checks recent signed GitHub issues and revision comments for
   duplicates, a shared ten-minute cooldown and a five-per-day limit per connection.
   If either bounded history reaches 100 entries, submissions stop rather than
   bypassing the check.
3. The Worker creates a public issue with a signed marker and `site-request` label.
   The GitHub Actions workflow independently verifies the signature. Editing the
   signed issue title/body invalidates automatic processing; put clarifications
   through the ticket's change form instead. Plain GitHub issues and unsigned
   comments do not trigger paid coding runs.
4. A separate trusted screening job uses one bounded, tool-free Responses call
   (`gpt-4.1-mini-2025-04-14`, at most 300 output tokens) to decide whether the
   request is an actual sim-gui feature, improvement, or bug fix. This applies to
   every follow-up too. Unrelated requests show **Request rejected** and a reason;
   vague relevant requests ask for clarification. Corrected feedback can be sent
   from the same ticket. API errors, refusals and invalid results fail closed.
   Screening uses the existing OpenAI key and billing cap; no coding, PR update,
   build or preview deployment starts without explicit acceptance. The classifier
   is a relevance filter, not a replacement for the sandbox or publisher restrictions.
   Every accepted request then starts a Codex job in a workspace sandbox with no
   repository write token or Cloudflare secret. It returns an untrusted patch.
5. A fresh publishing job validates the patch, rejecting infrastructure changes,
   path escapes, symlinks, executables, binaries, oversized output, and changes
   outside the allowed Rust, JSON data, and web source paths. It creates
   `codex/request-N` and a **draft PR**. It never executes candidate source.
6. A fresh build job runs tests and builds WASM without deployment/API secrets.
   Successful static artifacts deploy to `hackmaster-sim-previews`, a separate
   Pages project with no production bindings. Failed builds keep their draft PR
   and report failure. Requests that produce no viable patch report needs-info.
7. `https://feature.sim-gui.com/N` tracks the issue, links its PR, and embeds the preview
   in an iframe on the isolated Pages origin. The owner reviews and merges the
   PR to release it to production. Previews have separate browser saves.
8. Once a run finishes, anyone with the ticket link can use **Request changes**
   beneath the preview. This creates a signed comment on the same issue. The
   agent receives the original request, recent follow-ups, and the latest revision,
   starting from the existing PR's immutable head. The trusted publisher validates
   that branch and adds a commit to the same PR using a normal fast-forward push.
   If the PR changed during the run, publishing stops instead of overwriting it.
9. The ticket keeps its last working preview visible while the revision runs,
   then loads the new immutable deployment automatically. It shows recent feedback,
   disables further submissions during a run, and retains drafts after errors.
   Failed/needs-info runs accept clarifications too. Closed tickets require a new
   request. Production changes only when the owner merges the PR.

The ticket's **Agent activity** panel shows public commentary, generic tool activity,
and the final summary. A trusted background reporter tails only the dedicated
Codex session files and sends a bounded snapshot every 15 seconds when it changes.
The page polls every 15 seconds while active (up to roughly 30 seconds end-to-end).
It keeps the latest 40 events, preserves a reader's scroll position, and lets them
collapse the panel. Reasoning, prompts, raw commands, tool outputs, and credentials
are not included. Existing runs started before this reporter was deployed have no
detailed history; future requests and revisions do.

The code job has `id-token: write`, but no repository write token. The Worker
verifies GitHub's short-lived OIDC signature, audience, immutable repository IDs,
main workflow identity, and the run/attempt recorded by trusted status comments.
Only the matching active ticket revision accepts updates. The Worker edits one
HMAC-signed issue comment per run, using its existing issue-only token; this needs
no additional secrets or paid infrastructure. Activity is untrusted display data
and cannot set ticket status, PRs, or preview URLs. Reporter outages do not fail
the agent, and the GitHub run link remains available. This is a lightweight feed;
larger traffic would warrant dedicated storage to avoid GitHub API rate limits.

Basic spam protection is deliberately lightweight, not a distributed atomic
rate limiter: simultaneous requests can race the GitHub history check, and
shared connections share limits. Add Turnstile plus durable rate limiting if
abuse appears. Raw IP addresses are not stored; keyed fingerprints are included
in public issue metadata. All feature descriptions and agent results are public.

The workflow explicitly builds its new PR because pushes/PRs created with
`GITHUB_TOKEN` do not trigger further workflows. It publishes a `Feature preview`
commit status so reviewers can find the validation result from the PR.
Other same-repository PRs also receive `pr-N` Pages previews after passing checks.
Fork PRs build without deployment secrets.

## Operation and recovery

- Workflows: `web.yml` (main/PRs), `feature-request.yml` (signed issues and revisions),
  `build-web.yml` and `deploy-preview.yml` (shared jobs).
- Agent jobs have a 25-minute timeout; jobs are serialized per issue, not globally,
  so submitting another request does not cancel someone else's pending run.
- A failed run can be retried from Actions. If publishing already created a
  branch/PR, rerun only the failed jobs; never force-push generated branches.
- For an agent failure before publishing, use **Implement site feature request →
  Run workflow → issue number** to retry the original signed request.
  For a failed revision, also supply its signed comment ID in **revision**.
  Completed revisions are not replayed; submit fresh feedback on the ticket instead.
- Set `FEATURE_REQUESTS_ENABLED=false` and deploy main to disable new submissions;
  the repository variable also stops new agent starts immediately.
- Reaching the API cap/balance limit produces a failed run with an issue status;
  raise the cap/add credit only if you want more runs, then retry deliberately.
- Production rollback uses the Cloudflare Pages deployment rollback UI. Keep the
  project and custom domain so visitors retain browser-local presets.
- Artifacts are kept seven days. Cloudflare previews remain available until their
  deployments are removed. An immutable deployment URL is stored with the ticket.

## Local validation

For a no-spend deployment smoke check, manually run **Implement site feature
request** with an existing signed issue number and **activity_check** enabled.
It checks live GitHub OIDC verification and rejection of a mismatched run/revision.
It does not start an agent or write a ticket. Unit tests cover signed comment
updates, replay/stale-run rejection, event filtering, and partial session records.

**Evaluate request screening** is a manual Actions workflow for nine representative
valid/invalid requests. It uses the real screening API (and a small amount of API
credit), but creates no issues, PRs or coding runs. Run it when changing the screening
policy or model. Local tests mock API responses and verify that only acceptance
can open the coding gate, including failure and rejection cases.

```sh
cargo test --lib --bin sim_gui
cargo check --all-targets
node --test server/*.test.mjs
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts -p 'test_feature*.py'
python3 scripts/build_web.py
```

`build_web.py` uses a matching wasm-bindgen CLI and clears only `target/web` to
avoid stale server assets. Serve that folder for UI checks. Local/preview forms
link to production instead of creating public issues. `_worker.js` is added only
by the trusted production deployment job. Deployment runners only consume static
candidate artifacts; they never run candidate build scripts.

References: [Cloudflare Pages CI](https://developers.cloudflare.com/pages/how-to/use-direct-upload-with-continuous-integration/),
[Pages advanced mode](https://developers.cloudflare.com/pages/functions/advanced-mode/),
[Codex GitHub Action](https://learn.chatgpt.com/docs/github-action),
[API spending limits](https://developers.openai.com/api/docs/guides/spend-limits).
