# Autobattler v1 Backlog (Top 10)

## Scope
This is the prioritized v1 execution list. Each task includes:
- Implementation plan
- Test plan
- Done criteria

## 1) Save/Load Reliability + Autosave Checkpoints
- Implementation:
  - Add autosave checkpoints after fight resolve, post-fight choice resolve, and level-up confirm.
  - Version run/character save schema and add migration handlers.
  - Harden load path to fail gracefully with actionable messages.
- Test plan:
  - Unit tests for save serialization/deserialization round-trip.
  - Migration tests from previous schema versions to current.
  - Integration test: start run -> fight -> autosave -> reload -> continue deterministically.
- Done criteria:
  - No state loss across reload; autosave files recover full run state.

## 2) Deterministic Run Pipeline (Seed Contract)
- Implementation:
  - Enforce one seed contract for encounter generation, combat rng, loot rng, event rng.
  - Derive sub-seeds using stable keyed derivation by encounter/depth/index.
  - Expose seed and sub-seed context in run debug panel/log.
- Test plan:
  - Golden tests: same seed produces same encounter sequence, outcomes, and rewards.
  - Differential tests: changing seed changes at least one downstream output.
- Done criteria:
  - Two runs with same inputs are byte-equivalent in outcome logs/reward summaries.

## 3) Post-Fight Action System Finalization (Fight/Rest/Train)
- Implementation:
  - Finalize rules for `Fight On`, `Rest`, `Train`.
  - Ensure each action mutates run state via one resolver path.
  - Add clear effect preview in UI before confirming action.
- Test plan:
  - Unit tests per action resolver path (hp, wounds, days, resources, flags).
  - Integration test for multi-step sequence: fight -> rest -> train -> fight.
- Done criteria:
  - Action results are consistent and visible in UI/log.

## 4) Reward/Economy Resolver Unification
- Implementation:
  - Move xp/gold/item reward math behind one reward resolver.
  - Add encounter tier modifiers (normal/elite/boss).
  - Add economy invariants (no negative gold, bounded reward ranges).
- Test plan:
  - Unit tests for reward formulas across depth/tier ranges.
  - Property tests for invariants (gold >= 0, xp monotonic by level band).
- Done criteria:
  - Rewards are consistent, bounded, and scale predictably.

## 5) Level-Up Checkpoint + Allocation Validation
- Implementation:
  - Add blocking level-up checkpoint in run flow.
  - Validate AP/BP/LP/RP spend, talent requirements, and progression tier caps.
  - Recompute derived stats after commit.
- Test plan:
  - Unit tests for points accounting and tier cap validation.
  - Integration test: gain XP -> level-up -> allocate -> persist -> reload.
- Done criteria:
  - Invalid allocations are impossible; committed allocations survive reload.

## 6) Encounter Scaling + Spawn Bands
- Implementation:
  - Define depth bands and level scaling curve for enemy spawns.
  - Add encounter metadata (`normal`, `elite`, `boss`) to run state.
  - Tie band/tier into reward and event systems.
- Test plan:
  - Unit tests for depth->band mapping and level bounds.
  - Simulation tests over many seeds to validate distribution targets.
- Done criteria:
  - Encounter difficulty ramps smoothly and predictably by depth.

## 7) Event Framework v1 (Weighted, Deterministic)
- Implementation:
  - Add event engine with weighted pools by depth/state.
  - Implement a minimal v1 set of events (boon/hazard/trader/shrine).
  - Resolve events through explicit typed outcomes.
- Test plan:
  - Unit tests for weighted selection with deterministic seeds.
  - Integration tests for each event type’s state mutation.
- Done criteria:
  - Events trigger/resolve deterministically and affect run outcomes.

## 8) Combat UX Parity + Clarity Pass
- Implementation:
  - Lock fight UI parity with sim-style visuals (poses/icons/timeline/labels).
  - Add explicit combat state indicators (paused/running/step/result).
  - Ensure run panel + arena panel reflect same source-of-truth data.
- Test plan:
  - Snapshot tests for key UI combat states (normal, knocked, downed, paused).
  - Manual test checklist for interaction flow and visual sync.
- Done criteria:
  - Fight UI states are legible and consistent through entire encounter lifecycle.

## 9) Data Layout + Content Hygiene
- Implementation:
  - Complete migration to `data/sim` + `data/autobattler` with loader compatibility.
  - Validate required files on startup and provide clear missing-file errors.
  - Normalize quick-start and preset sources to one canonical path each.
- Test plan:
  - Loader tests for canonical paths and backward-compatible aliases.
  - Packaging test to verify runtime data discovery in release builds.
- Done criteria:
  - No accidental fallback to wrong dataset; packaged builds load correct content.

## 10) Balance + Regression Test Harness
- Implementation:
  - Add seeded run harness for bulk regression checks.
  - Track KPIs: run failure rate, fights-per-level, resource spend split, average depth.
  - Gate merges on regression thresholds.
- Test plan:
  - Long-run seeded simulations in CI/nightly.
  - Threshold assertions with tolerance windows.
- Done criteria:
  - Balance regressions are detected automatically before release.

## Execution Order
1. Save/Load Reliability + Autosave Checkpoints
2. Deterministic Run Pipeline
3. Post-Fight Action System
4. Reward/Economy Resolver
5. Level-Up Checkpoint
6. Encounter Scaling
7. Event Framework
8. Combat UX Parity + Clarity
9. Data Layout + Content Hygiene
10. Balance + Regression Harness
