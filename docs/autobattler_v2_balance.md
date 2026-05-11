# Autobattler v2 Balance + Regression Plan

## Goal
Do not tune v2 content from ad hoc playthroughs. First build a deterministic, headless run harness that replays the same seed corpus through the real v2 run loop, records stable KPIs, and fails only on clear regressions.

The balance harness is a regression gate first and a tuning tool second. Early thresholds should protect against broken pacing, impossible runs, economy blowouts, and nondeterminism; exact target values should be tightened only after the first real baseline is captured.

## Seeded Run Corpus
- PR smoke: 128 fixed run seeds, 20 resolved nodes per run, one balanced choice policy.
- PR gate: 512 fixed run seeds, 40 resolved nodes per run, `safe`, `balanced`, and `risky` choice policies.
- Nightly: 4,096 fixed run seeds, full configured run length, all supported quick-start/archetype profiles.
- Every result records `run_seed`, choice policy, player archetype, content/config version, terminal reason, final depth, final level, days elapsed, and the failing node index if any.
- A single seed must be reproducible from the CLI with the same player, config, choice policy, and node index.

## KPIs To Record
- Determinism: same-seed node summaries are byte-equivalent; autosave/resume does not change the next node or sub-seeds.
- Run outcomes: win, death, abandon, timeout, no-spawn/content-error, and panic counts.
- Depth pacing: average, median, p10, p90, and max depth; depth at death; depth by player archetype and choice policy.
- Encounter outcomes: win rate by depth band and tier, elite/boss survival rate, fight duration, timeout rate, HP remaining, wounds applied, wounds carried forward.
- Progression: fights per level, XP gained per depth, level gained per run, weapon mastery progress, unspent level-up resources.
- Economy: gold gained, gold spent, gold banked, item drops, item equips/replacements, sell/discard counts, and spend split by category.
- Downtime/events: event frequency by type, choice frequency, forced-fight rate, rest/activity usage, days elapsed, wounds healed.
- Content coverage: spawned enemy IDs, event IDs, loot IDs, blocked/missing content, and per-band distribution versus configured weights.

## Initial Thresholds
Hard gates:
- Zero panics, missing-content failures, invalid level-up commits, or impossible node transitions.
- Same seed + same choices must produce zero node-summary diffs.
- Autosave/resume from every checkpoint in the smoke corpus must produce zero next-node diffs.
- Any no-spawn/content-error terminal reason fails the gate.

Regression gates, compared to the checked-in baseline:
- Failure rate may not worsen by more than 3 percentage points in PR or 1.5 points nightly.
- Average depth may not drop by more than 0.75 in PR or 0.35 nightly.
- p10 depth may not drop by more than 1 depth band.
- Fights per level may not move by more than 10% unless the change is intentionally baselined.
- Gold gained per run and gold banked at end may not move by more than 15%.
- Resource spend split may not move by more than 10 percentage points per category.
- Event, elite, and boss frequencies may not drift more than 5 percentage points from configured weights in nightly runs.
- Fight timeout rate must stay below 2% overall and below 5% for any one tier/depth band.

Tuning targets, set after the first real baseline:
- Early balanced-policy runs should usually reach several normal encounters before death.
- Safe choices should trade lower rewards for lower wound/death pressure.
- Risky choices should improve reward/depth upside while visibly increasing wounds or failure rate.
- Elite and boss encounters should be distinct spikes, not just normal fights with invisible reward multipliers.

## What `autobattler_regression` Gets Wrong Today
- It uses `DummyBuilder`, so combatants are default shells rather than real player presets, NPC presets, gear, talents, wounds, and progression.
- It hard-codes one spawn entry, one loot table, and placeholder sim settings instead of loading the same content/config path as the app.
- It only exercises repeated fights. It does not cover v2 map choices, event nodes, downtime, level-up commits, shops, autosaves, or replay summaries.
- Its thresholds are placeholders: `max_failure_rate = 0.95`, `min_average_depth = 1.0`, and `max_resource_spend_split = 1.0` allow most broken balance to pass.
- `resource_spend_split` is not meaningful because there is no spend path in the current harness.
- It reports only aggregates, with no per-seed, per-tier, per-depth-band, terminal-reason, or percentile breakdown.
- It prints human text only; there is no machine-readable JSON/CSV output, checked-in baseline, or baseline-diff mode.
- The sample size and seed reporting are too small for a hard balance gate.

Keep the current binary as a smoke test until the v2 harness replaces it. Do not use it to approve balance tuning.

## Minimum Harness Before Tuning
1. Run the real v2 node resolver headlessly, using the same data loaders, combatant builder, config defaults, seed derivation, and reward/event/downtime code as the playable app.
2. Support deterministic choice policies: `safe`, `balanced`, `risky`, plus scripted per-seed overrides for reproducing edge cases.
3. Emit per-run JSON lines and aggregate JSON with all KPIs above, baseline deltas, threshold decisions, and enough seed context to replay any failed node.
4. Commit a baseline artifact generated from the PR-gate corpus; require intentional baseline refreshes for accepted balance changes.
5. Split checks into fast PR smoke, PR gate, and nightly long-run commands so CI remains stable without hiding distribution drift.
6. Include replay tests for same-seed equality and autosave/resume equality before enabling numeric balance thresholds.
7. Add a failure triage view that lists the worst seeds by depth loss, timeout, wound load, economy drift, and content errors.

Only after those pieces exist should tuning begin on encounter weights, rewards, rest cadence, event frequency, elite/boss scaling, and economy prices.
