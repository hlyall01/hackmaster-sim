# Autobattler Plan

## 1) Audit current autobattler entrypoint and run loop (completed)
- Confirmed `src/bin/autobattler.rs` runs a fixed set of fights with a single spawner and minimal run state.
- Identified gaps vs. a full autobattler: data-driven config, progression, AI policy layer, and reporting/persistence.

## Phase 1) Seeded RNG + run config foundations
- Make RNG seeding explicit and universal:
  - All RNG sources flow from a single seed (config/CLI/UI), stored in `RunState`.
  - Deterministic sub-seeds for combat, loot, encounters, shops.
  - Default to a random seed, but allow manual entry before starting.
  - Add a seed display/export in UI for sharing.
- Add a config file (e.g., `data/autobattler_config.json`) with:
  - Seed, fights per run, max fight duration, rest days, base difficulty, difficulty ramp, loot curves.
  - Enemy pool/biomes, encounter weights, and progression parameters.
- Load config in `src/bin/autobattler.rs` and add CLI flags to override key settings.
- Add deterministic replay tests for a fixed seed.

## Phase 2) UI entrypoint + character creation
- Create a UI flow to:
  - Create a character (base stats, talents, starting gear).
  - Auto-generate a seed, with an option to enter one before starting.
  - Start a run and transition into the battle loop.
- Hook UI selections into `RunState` and config overrides.

## Phase 3) Between-battle decision loop
- Implement a post-fight menu with choices:
  - Rest (heal/wound recovery).
  - Train (stat upgrades, skill/talent allocation).
  - Equip gear (inventory management, loadouts).
  - Allocate stats/talents (level-up style choices).
- Track resources needed for choices (time, gold, fatigue).
- Ensure all outcomes are seeded and deterministic.

## Phase 4) Encounter progression + pacing
- Extend `RunState` and spawner logic to support:
  - Run depth that affects level ranges, weights, and encounter composition.
  - Biomes/tiers with escalating enemy presets and loot.
  - Post-fight rest logic and wound recovery pacing.
- Tune difficulty ramp and encounter weights for early balance.

## Phase 5) Random shop encounters + economy
- Add random shop encounters between fights:
  - Seeded spawn chance and shop inventory generation.
  - Buy/sell flow, gold tracking, and item pricing curves.
- Integrate shop choices into the between-battle loop.

## Phase 6) Policy/AI layer for auto decisions
- Add a policy interface for:
  - Target selection, stance/tactics, maneuver choice, and risk thresholds.
  - Hooks for temporary effects and item usage.
- Keep policies data-driven for future balancing.

## Phase 7) Reporting + persistence
- Add combat summaries and run stats.
- Save/load run state and seeds for replay and sharing.
- Persist the chosen seed in save files and on run summaries.
