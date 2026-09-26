# Deploy the simulator and feature requests

Production is https://sim-gui.com/. Every push/merge into `main` runs native tests,
checks the Rust targets, builds WASM, and publishes the successful build to the
`hackmaster-sim` Cloudflare Pages project. Pull requests do not publish production.

Feature requests live at https://feature.sim-gui.com/. Both custom domains attach
to the same production Pages project; `feature.sim-gui.com` has a proxied CNAME to
`hackmaster-sim.pages.dev`. Its landing page opens the form without downloading
WASM. The simulator keeps its Feature requests tab, linking to the new site.
Old `sim-gui.com/N` ticket links redirect permanently to `feature.sim-gui.com/N`;
the old `sim-gui.com/?tab=requests` link redirects in the browser. The API accepts
submissions only on the feature host and from that exact origin. Preview builds
remain on the isolated `hackmaster-sim-previews.pages.dev` origins.

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

1. Open https://feature.sim-gui.com/ directly, or follow the link from the simulator's **Feature requests** tab.
2. The production Pages Worker validates a honeypot, payload limits, exact origin,
   and a signed, IP-bound challenge with a three-second minimum age and one-hour
   lifetime. It checks recent signed GitHub issues for duplicates, a ten-minute
   cooldown and a five-per-day limit per connection. At 100 recent matching
   issues it stops accepting submissions rather than bypassing the bounded check.
3. The Worker creates a public issue with a signed marker and `site-request` label.
   The GitHub Actions workflow independently verifies the signature. Editing the
   signed issue title/body invalidates automatic processing; put clarifications
   in comments instead. Plain GitHub issues do not trigger paid coding runs.
4. Every accepted request starts a Codex job in a workspace sandbox with no
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

- Workflows: `web.yml` (main/PRs), `feature-request.yml` (signed issues),
  `build-web.yml` and `deploy-preview.yml` (shared jobs).
- Agent jobs have a 25-minute timeout; jobs are serialized per issue, not globally,
  so submitting another request does not cancel someone else's pending run.
- A failed run can be retried from Actions. If publishing already created a
  branch/PR, rerun only the failed jobs; never force-push generated branches.
- For an agent failure before publishing, use **Implement site feature request →
  Run workflow → issue number** to retry the original signed request.
- Set `FEATURE_REQUESTS_ENABLED=false` and deploy main to disable new submissions;
  the repository variable also stops new agent starts immediately.
- Reaching the API cap/balance limit produces a failed run with an issue status;
  raise the cap/add credit only if you want more runs, then retry deliberately.
- Production rollback uses the Cloudflare Pages deployment rollback UI. Keep the
  project and custom domain so visitors retain browser-local presets.
- Artifacts are kept seven days. Cloudflare previews remain available until their
  deployments are removed. An immutable deployment URL is stored with the ticket.

## Local validation

```sh
cargo test --lib --bin sim_gui
cargo check --all-targets
node --test server/requests.test.mjs
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
