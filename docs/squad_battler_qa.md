# Squad Battler QA

The squad battler demo is separate from `sim_gui`. Keep demo-specific QA work in
scripts, docs, and the `squad_battler_demo` binary path; do not route this flow
through the legacy simulator UI.

## Run The Demo

From the repository root:

```bash
cargo run --bin squad_battler_demo -- --port 8788
```

Then open:

```text
http://127.0.0.1:8788
```

## API Smoke

With the local server running:

```bash
python3 scripts/squad_battler_api_smoke.py --base-url http://127.0.0.1:8788 --seed 8788
```

The script checks:

- `GET /`, `/static/styles.css`, `/static/js/main.js`, and a missing static asset.
- JSON shapes for `/api/state`, `/api/new-run`, `/api/choose-node`,
  `/api/start-fight`, `/api/fight-command`, and `/api/recruit-choice`.
- Fixed-seed `/api/new-run` determinism by comparing two full responses.
- No overlapping living combatant positions after combat start, ticks, and finish.
- Recruit-choice handling through a deterministic recruit route.

Representative output:

```text
base_url=http://127.0.0.1:8788
ok static asset routes
ok GET /api/state
ok POST /api/new-run deterministic seed=8788 active=3
ok POST /api/choose-node node=0 kind=fight enemies=2
ok POST /api/start-fight
ok POST /api/fight-command tick seconds=1
ok POST /api/fight-command tick seconds=5
ok POST /api/fight-command finish phase=choose_node
ok POST /api/recruit-choice candidate=recruit-0-1 remaining=2
ok smoke complete phase=choose_node depth=1
```

The exact combat outcome can change as balance and AI evolve. Keep the script
assertions shape- and invariant-based rather than pinned to specific names,
damage rolls, or rewards.

## Compile Checklist

Run these before handing off squad battler demo changes:

```bash
cargo check --bin squad_battler_demo
cargo check --bin sim_gui
```

If the v2 autobattler demo binary exists in the checkout, include it:

```bash
test ! -f src/bin/autobattler_v2_demo.rs || cargo check --bin autobattler_v2_demo
```

## Integration Checklist

- Keep `src/squad_battler/` and `src/bin/squad_battler_demo/` independent from
  `sim_gui`; shared mechanics should go through library modules, not UI coupling.
- Preserve seeded entry points. `/api/new-run` currently accepts `{"seed": 8788}`,
  which enables replay smoke coverage.
- When adding endpoints, update `scripts/squad_battler_api_smoke.py` with JSON
  shape checks and at least one happy-path call.
- When changing combat movement, keep the no-overlap invariant for living units
  and add a smoke assertion if the response shape changes.
- When changing static assets or route paths, update the static route checks.
- Prefer robust sample expectations: phase names, required keys, counts, and
  invariants are better than exact combat logs.
- Add any known API limitation here before handing off, especially if a flow
  cannot be exercised through HTTP yet.
