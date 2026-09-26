# Character cloud saves — isolated test environment

Test app: https://characters-test.sim-gui.com

Branch: `codex/character-cloud-save`. This does not replace the production
simulator or its feature-request Worker. Native EXE builds continue using their
existing local presets; the HTML cloud controls run only in the marked test build.

## Using it

1. Sign in with Google. The test Access application allows anyone authenticated
   through the configured Google identity provider. No email whitelist is needed;
   the Worker independently enforces character assignments.
2. The player signs in once so their verified identity appears in the user list.
   New users have no character permissions.
3. An administrator opens **TEST · Cloud characters**, selects a character, and
   chooses **Assign** beside that user's email under **Editing access**.
   **Revoke** removes editing access, including for already-open editors at save time.
4. The player selects their character and **Load selected** for either simulator
   slot, edits using the existing editor, and chooses **Save online**.
   Only a server acknowledgment displays “saved online”. Loading is explicit after
   a reload so a default/local character cannot overwrite a cloud character.

Administrators can edit every character regardless of assignments. There is no
client-facing administrator-grant endpoint. `BOOTSTRAP_ADMIN_EMAIL` is a trusted
deployment variable; a matching, verified Access identity is inserted into the
administrator table. Keep this value restricted to the intended administrator.
To remove that administrator, first clear the deployment variable and then remove
the corresponding administrator row through trusted D1 administration.

## Import, recovery and export

To import an existing preset, select it in the simulator's Fighter preset menu,
then choose **Create from current / imported preset** for that slot. This takes a
complete snapshot of the loaded character. **Import character JSON** accepts this
app's complete document/export format; it validates in Rust before replacing the
workspace. It does not interpret a whole legacy `fighter_presets.json` file.

Each character has a UUID independent of its name. Equipment and NPC references
use unique catalog names instead of array positions. Spell, race, talent and style
IDs retain their existing string identifiers. Catalog reordering is safe; renaming
or removing a catalog entry requires a document migration. Missing/ambiguous
references and unsupported fields fail visibly instead of silently substituting
equipment or discarding data.

Local drafts are scoped to the signed-in user and separately stored for each
editing session/slot/tab. On a network/authentication failure, sign in again and
use **Recover local draft**. A selector distinguishes multiple saved drafts.
Creation drafts have their own recovery control. Drafts are browser-local, whereas
acknowledged cloud saves are available on other devices. Browser storage clearing,
private-browsing cleanup or storage failure can remove/prevent drafts: **Export
current** provides a file backup. Storage failures are reported and an online save
is not attempted if its draft cannot first be stored.

A stale save returns a conflict and preserves the draft. Export the draft, load the
latest online version, then reconcile the changes before saving. There is no
automatic merge. Retries use mutation IDs so a lost response does not duplicate a
successful save. **Revision history → Restore revision** creates another revision;
it never erases history. **Load older revisions** pages beyond the newest 200.
**Export selected saved character** downloads the server snapshot.

## Preset-format audit

The old `FighterPreset` conversion omits the player's `environment`,
`misc_modifiers`, `knockback_step`, `npc_preset` and `race_applied` state. It also
normalizes mastery and defensive-penalty values. Cloud documents serialize the
entire `PlayerConfig` directly, including progression, attributes, gear/materials,
masteries, maneuvers/mount configuration, race, proficiencies, talents, styles,
magic loadout/spell parameters and applied tactical policy. Only `fighter_preset`
(a local menu selection) and `active_weapon_style_ids` (an internal temporary
profile override) are excluded. Equipment/NPC positions are converted to names.
Unapplied tactical-editor drafts still require the existing Apply action; combat
run state, window positions and bulk-simulation settings are not character data.

## Deployment configuration

All resources below are dedicated to this test:

| Resource | Value |
| --- | --- |
| Pages project | `hackmaster-character-test` |
| API Worker | `hackmaster-character-test` |
| D1 database | `hackmaster-character-test` |
| D1 ID | `a3c1abb7-d10c-40fd-adf0-203943196a70` |
| Access application | `HackmasterSim character test` |
| Access application ID | `a217f349-952a-4290-9c06-546b659285ce` |
| Access team | `weathered-sunset-80df.cloudflareaccess.com` |
| Hostname | `characters-test.sim-gui.com` |

Worker bindings, issuer and audience are in `wrangler.jsonc`. Set the administrator
email during deployment rather than putting personal configuration in Git:

Use Node.js 24 or newer for the service tests (TypeScript execution and SQLite).
Install the locked dependencies with `npm ci`.

```sh
cd cloud
npm ci
npm test
npm run check
npx wrangler d1 migrations apply hackmaster-character-test --remote
npx wrangler deploy --var BOOTSTRAP_ADMIN_EMAIL:YOUR_ADMIN_EMAIL
```

The initial test schema was applied through Cloudflare's D1 API and recorded in
Wrangler's `d1_migrations` table. The migration command is safe on this existing
test database and is also the normal setup path for a fresh database. Never point
this configuration at production's database or Worker.

Build and deploy the frontend from the repository root:

```sh
python3 scripts/build_web.py
python3 scripts/build_character_test.py
cd cloud
npx wrangler pages deploy ../target/character-test \
  --project-name hackmaster-character-test --branch codex/character-cloud-save
```

Install the wasm-bindgen CLI version matching Cargo.lock, or set `WASM_BINDGEN`
to that executable. `build_character_test.py` copies the WASM build, marks it as
TEST and adds `pages-worker.js` plus all-path Functions routing. Production's
`target/web` remains a separate artifact and retains the existing hosting approach.

In both test Pages deployment configurations, set:

- `APP_ORIGIN=https://characters-test.sim-gui.com` (plain-text variable).
- Service binding `CHARACTERS` → `hackmaster-character-test`, production environment.
- Compatibility date `2026-09-26`; `fail_open=false`.

Attach the custom hostname to the test Pages project, then use a proxied CNAME
to `hackmaster-character-test.pages.dev`. The Access application covers the entire
custom hostname, uses Google only, and has an Allow policy with Include Everyone
and Require the configured Google login method. This is not an authentication bypass.
Google's client secret belongs only in Cloudflare's identity-provider configuration.

The Pages Worker rejects `pages.dev`, branch and immutable deployment URLs. The
API Worker has `workers_dev=false`, `preview_urls=false`, no public route and is
called through the service binding. It additionally verifies Access JWT signature,
issuer, audience, expiry and identity on every request, and checks exact request
origin for writes. Assignment checks are repeated inside the conditional SQL
update. Revision triggers share the write transaction. Keep Pages fail-closed;
do not add bypass policies or public Worker routes.

## Checks and limits

```sh
cargo test --locked --lib --bin sim_gui
cargo check --locked --all-targets
cargo build --locked --release --bin sim_gui --target x86_64-pc-windows-gnu
cd cloud && npm test && npm run check
```

The service tests exercise real SQLite constraints/triggers, signed JWTs,
unassigned/revoked/foreign access, permission escalation, write races, conflicts,
rollback, idempotent retries and recovery. DOM tests exercise the actual client
script with simulated network/auth/server failures. Rust tests cover complete
documents, catalog reordering, missing references and unknown-field protection.
Live verification uses the Google administrator account; a separate real player
account can sign in without an invitation for a second-person login test. It starts
without character access until an administrator assigns a character.

This uses Pages, Workers, Access and D1 with no paid-only feature requirement.
Free-tier quotas still apply, including Access seats, Worker requests/CPU and D1
reads/writes/storage. History is retained indefinitely and counts toward D1 storage.
See [Workers limits](https://developers.cloudflare.com/workers/platform/limits/),
[D1 limits](https://developers.cloudflare.com/d1/platform/limits/) and
[Access plans](https://www.cloudflare.com/plans/zero-trust-services/).
No billing plan upgrade or production policy change is part of this deployment.
