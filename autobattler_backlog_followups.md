# Autobattler Backlog Follow-Ups

These are items I could not fully close without additional design decisions or broader refactors.

## 1) Level-Up Rules Finalization
- Current implementation adds a blocking level-up checkpoint with slot allocation to BP/LP/AP/RP and validation that all slots are assigned before confirm.
- Open decision:
  - Exact per-level grant table for BP/LP/AP/RP (currently slot-based with `1 slot = +5 BP or +1 LP/AP/RP`).
  - Whether level-up should validate and lock talent purchases and progression tier caps in the same checkpoint UI.

## 2) Event Content Depth
- Event framework is implemented and deterministic (`boon/hazard/trader/shrine`) with typed outcomes.
- Open decision:
  - Expand each event into richer branches/choices and content-specific assets.
  - Define final event frequency targets by depth/tier (currently weighted baseline + state adjustments).

## 3) Encounter Scaling Curve Tuning
- Depth bands and encounter tiers are in place and wired into level/reward logic.
- Open decision:
  - Final breakpoints and tuning targets for each band/tier.
  - Whether boss encounters should be fixed-set/handcrafted instead of weighted procedural.

## 4) Reward Formula Tuning
- Unified reward resolver now scales by encounter tier.
- Open decision:
  - Final multipliers for `normal/elite/boss`.
  - Item drop scaling by tier (currently keeps one rolled item unchanged by tier).

## 5) Combat UX Parity Completion
- Added lifecycle clarity indicators and synchronized run-state usage.
- Open decision:
  - Final parity checklist for exact `sim_gui` look/pose/timeline behavior and whether to add snapshot/golden-image UI tests.

## 6) Regression Gate Integration
- Added seeded harness module and CLI (`autobattler_regression`) with KPI + threshold evaluation.
- Open decision:
  - CI integration policy (what command runs in CI, fail thresholds, and branch protections).
  - KPI target values for failure rate/avg depth/spend split.

## 7) Mirror Symmetry Test Stability
- Current test `core::sim::tests::arthur_mirror_symmetry_with_swapped_order` intermittently fails due Monte Carlo variance with `runs = 1000` and strict `max_diff <= 30`.
- Open decision:
  - Increase sample size and/or threshold for deterministic CI stability.
  - Seed the simulation runs for strict reproducibility if this is intended as a hard regression gate.
