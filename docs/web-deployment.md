# Deploy the simulator with GitHub Actions

The `Build and deploy simulator` workflow builds the existing WASM application
and publishes the static files to Cloudflare Pages. Simulation runs on visitors'
computers; this project needs no Cloudflare Functions, database or paid plan.

## One-time setup

1. Sign in to a Cloudflare account and verify your email if requested. You do not
   need to add or buy a domain to use a `pages.dev` address.
2. In Cloudflare **My Profile → API Tokens → Create Token → Custom token**, create
   a token named `hackmaster-sim GitHub Actions`. Grant **Account → Cloudflare
   Pages → Edit**, restricted to the account that will host this simulator.
3. In the GitHub repository, open **Settings → Secrets and variables → Actions**.
   Store that token as the repository **secret** `CLOUDFLARE_API_TOKEN`. Do not put
   the token in source files, a repository variable, an issue or a chat message.
4. Add these repository **variables**:

   | Name | Value |
   | --- | --- |
   | `CLOUDFLARE_ACCOUNT_ID` | The 32-character Cloudflare account ID from your account dashboard URL |
   | `CLOUDFLARE_PAGES_PROJECT` | `hackmaster-sim` (or your chosen Pages project name) |
   | `CLOUDFLARE_DEPLOY_BRANCH` | `codex/sim-gui-wasm` while developing this port; change to `main` after merging |

5. Push to the selected deployment branch. The workflow creates the Pages project
   if needed and deploys the site. Open the deployment URL in the run summary or
   the `cloudflare-pages` GitHub environment.

The Cloudflare project uses `main` as its production branch label. The workflow
explicitly publishes the selected GitHub branch to that label, so switching
`CLOUDFLARE_DEPLOY_BRANCH` does not change the site's address or browser saves.
An existing Cloudflare project with a different production branch is rejected
instead of silently publishing to a preview address.

## Updates and checks

- Pushes to `main` and `codex/sim-gui-wasm`, and pull requests targeting either,
  build a release and retain a `simulator-web` downloadable artifact for 7 days.
- Only the branch selected by `CLOUDFLARE_DEPLOY_BRANCH` deploys. If unset, it
  defaults to `main`. Pull requests, including forks, never receive deployment
  credentials or publish a site.
- If the account/project variables are missing, builds still run and deployment
  is skipped. Once configured, a missing deployment token causes a clear failure.
- The workflow can also be run manually once it exists on the default branch.
  Before then, push another commit or rerun the branch's existing Actions run.
- Rust, wasm-bindgen, Wrangler and Actions versions are pinned. When upgrading
  wasm-bindgen in `Cargo.lock`, update the workflow CLI version and the SHA-256
  digest from the official GitHub release asset together.

To publish a previous version, use Cloudflare Pages' deployment rollback UI.
Do not delete the project: it owns the site's stable origin, and presets are
stored separately in each visitor's browser for that origin.

## MCP connection

Cloudflare's official remote MCP server is `https://mcp.cloudflare.com/mcp`.
Configure it with `codex mcp add cloudflare --url https://mcp.cloudflare.com/mcp`
and authenticate with `codex mcp login cloudflare`. If it is already configured,
only the login is needed. Reconnect it in Codex if an existing session still
reports authentication required after login.

MCP OAuth and the GitHub Actions token are separate credentials. The MCP
connection can use Pages metadata access for inspection. The deployment token
needs Pages Edit and belongs only in the GitHub Actions secret.

References: [Cloudflare Pages CI](https://developers.cloudflare.com/pages/how-to/use-direct-upload-with-continuous-integration/),
[Cloudflare MCP](https://developers.cloudflare.com/agents/model-context-protocol/cloudflare/servers-for-cloudflare/).
