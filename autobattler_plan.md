# Autobattler Plan

## 1) Audit current autobattler entrypoint and run loop (completed)
- Confirmed `src/bin/autobattler.rs` runs a fixed set of fights with a single spawner and minimal run state.
- Identified gaps vs. a full autobattler: data-driven config, progression, AI policy layer, and reporting/persistence.

## 2) Data-driven autobattler config + CLI/UI hooks
- Add a config file (e.g., `data/autobattler_config.json`) with:
  - Seed, fights per run, max fight duration, rest days, base difficulty, difficulty ramp, loot curves.
  - Enemy pool/biomes, encounter weights, and progression parameters.
- Load config in `src/bin/autobattler.rs` and add CLI flags to override key settings.

## 3) Encounter progression + pacing
- Extend `RunState` and spawner logic to support:
  - Run depth that affects level ranges, weights, and encounter composition.
  - Biomes/tiers with escalating enemy presets and loot.
  - Post-fight rest logic and wound recovery pacing.

## 4) Policy/AI layer for auto decisions
- Add a policy interface for:
  - Target selection, stance/tactics, maneuver choice, and risk thresholds.
  - Hooks for temporary effects and item usage.
- Keep policies data-driven for future balancing.

## 5) Reporting + persistence + determinism
- Add combat summaries and run stats.
- Save/load run state and seeds for replay.
- Deterministic replay tests for regression protection.
