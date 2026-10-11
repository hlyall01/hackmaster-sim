# Online character saves

Production: https://sim-gui.com. The character-test system was promoted with its
existing database and Access application; character IDs, owners, accounts, deleted
records, and revision history are preserved. Sign in using the same Google account.

## Using it

Open **Customize → Core → Online Characters**. **My Characters** shows characters
you currently own, including characters transferred to you. **Party Members** shows
other characters you can load for simulation. Guests can read the shared roster;
only an owner or administrator can save an existing character. Each account may
own up to ten active characters. **Create my character** saves a new copy.

**Drafts / Export** opens local draft recovery and JSON export for every signed-in
player. Failed saves keep drafts; a conflict requires loading the latest version
and reconciling edits. Same-tab sign-in preserves the open workspace. Browser
storage failures stop saves rather than risking an unprotected draft.

Administrators can transfer the single owner, review revision history, and restore
deleted characters. A transfer removes the previous owner's editing rights.
Deletion keeps revision history; restoration creates a new revision.

Cloud documents capture the full PlayerConfig, including magic, talents, tactics,
environment and miscellaneous modifiers. Equipment is stored by catalog name.
Unknown fields and missing catalog references fail visibly. Local preset menu
selection, temporary profile overrides, combat run state, unapplied tactical
editor changes, window positions and bulk settings are not character documents.

## Production resources

- Pages: `hackmaster-sim`, branch `main`.
- Character API Worker: `hackmaster-characters`, private service binding only.
- Pages binding: `CHARACTERS` → `hackmaster-characters`, environment `production`.
- D1: `hackmaster-character-test`, ID `a3c1abb7-d10c-40fd-adf0-203943196a70`.
  The historical name is retained deliberately to preserve all existing data.
  **This is now production data. Never use it for test resets or destructive tests.**
- Access application: `a217f349-952a-4290-9c06-546b659285ce`; existing audience and
  Google identity provider retained. It protects `sim-gui.com/api` and the legacy
  `characters-test.sim-gui.com/api` path, with the existing Google-only policy.
- Production Worker settings: `cloud/wrangler.production.jsonc`.
- Legacy test settings: `cloud/wrangler.jsonc`. That hostname shares the promoted
  data; it is a compatibility entry point, no longer an isolated testing database.

The production Pages Worker dispatches character routes on sim-gui.com to the
private character Worker. The feature-request API remains on feature.sim-gui.com.
Guest character reads are public; authenticated writes verify Access signatures,
issuer, audience, identity, exact origin, ownership and expected revision.
Alternate Pages/Worker hostnames cannot access the character service.

The production deployment injects the cloud marker into HTML. Preview builds have
no marker or service binding and cannot connect to production characters. Local
and native simulators continue to use local presets.

## Release and verification

Use Node.js 24+ and the locked dependencies. Build/test jobs validate the cloud
API, browser recovery UI, routing, Rust, and WebAssembly before publishing Pages.

```sh
npm ci --prefix cloud
npm test --prefix cloud
npm run check --prefix cloud
cargo test --locked --lib --bin sim_gui
cargo check --locked --all-targets
```

Backend releases are explicit and precede a frontend release that depends on them:

```sh
cd cloud
npx wrangler deploy --config wrangler.production.jsonc
```

The Pages CI token is not assumed to have Worker-edit permissions. Do not replace
it or reuse a developer OAuth token in CI. The existing administrator table is
retained; the production bootstrap-admin variable is intentionally empty.

All five current D1 migrations were already applied before promotion. No schema
or ownership migration is needed for the hostname change. Before future migrations,
export the database into an ignored, access-controlled backup location; do not
commit snapshots. Apply migrations only after review and backup.

The frontend publishes through `.github/workflows/web.yml`. It keeps the existing
feature-request Worker and adds the character service binding with fail-open off.
After release, verify guest roster IDs, Google sign-in, My Characters ownership,
read-only Party Members, and character/draft controls. To roll back the frontend,
use a prior Pages deployment; retain the database and Access identity settings.

## Data across hostnames

Online characters are the same records on production and the legacy test host.
Browser-local presets and drafts remain scoped to their original hostname and do
not automatically migrate. Save outstanding edits online before switching hosts,
or export a JSON backup from the original browser session. Nothing in promotion
clears the old browser storage. New staging work must use a different database.
