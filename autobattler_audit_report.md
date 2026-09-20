# HackmasterSim Autobattler Audit Report

## Scope
- Plans reviewed: `autobattler_ui_plan.md`, `autobattler_rpg_plan.md`, `autobattler_plan.md`.
- Code reviewed: `src/autobattler/*`, `src/core/gameplay/*`, `src/core/types.rs`, `src/character.rs`,
  `src/bin/sim_gui.rs`, `data/autobattler_config.json`, `data/talents.json`.

## Executive Summary
- The current app delivers a basic Bevy+egui autobattler with a short creation flow and fight loop,
  but large sections of the UI/RPG/autobattler plans are missing or diverge from the plan docs.
- Core blockers are the missing full-ability data model, lack of deterministic seed control, no
  level-up/growth pipeline, and incomplete between-fight decisions.

## Prioritized Milestones / Issue List
- P0 Foundation (blocking)
  - Decide UI architecture: keep Bevy+egui and update plans, or refactor to eframe as planned.
  - Add full-ability model + extend `PlayerProfile` with progression, point pools, honor/alignment,
    race/background, quirks/flaws, skills/proficiencies.
  - Implement percentile carry/normalization helpers (0-99 storage, carry on >= 100).
  - Wire deterministic seed end-to-end (RunState seed, UI/CLI input, SimRng usage, save fields).
  - Update persistence to include new profile fields + seed/config for replay.

- P1 Character creation completion
  - Implement creation steps 1-13 from the UI plan (alignment, honor, priors, quirks/flaws,
    advancement talents, skills/proficiencies, HP/derived stats, money/gear).
  - Add ledger/banking for BP/LP/AP/RP and spend audit history in the UI.
  - Add `free_at_start` talent flag in `data/talents.json` + auto-grant logic.
  - Reduce talent UI duplication by extracting shared components.

- P2 Run loop & RPG loop
  - Implement between-fight menu outcomes (rest/train/equip/allocate) and track days elapsed.
  - Use `rest_days_between_encounters` and randomize encounter gaps (8 days default).
  - Expand spawner to data-driven pools/biomes and difficulty ramping.
  - Add shops/economy hooks for purchases and pricing.

- P3 Level-up/growth systems
  - Implement XP curve and level-up triggers, stat bump dice, BP/AP/LP/RP grants, mastery gains.
  - Add AP spending for progression tiers and refresh derived stats.
  - Provide a level-up UI flow tied into the run loop.

- P4 Reporting/testing
  - Add run summaries, deterministic replay support, and seed/config persistence.
  - Add tests for stat rolling, BP thresholds, honor math, and deterministic runs.

## UI Plan Gaps (autobattler_ui_plan.md)
1) Creation flow only covers 5 steps; 8+ required steps are missing (alignment, honor, priors,
   quirks/flaws, advancement talents, skills/proficiencies, HP/derived stats, money/gear).
   - Evidence: `src/autobattler/state.rs:21`, `src/autobattler/state.rs:33`,
     `src/autobattler/ui.rs:132`, `src/autobattler/ui.rs:154`.

2) UI is Bevy+egui, not the planned eframe app.
   - Evidence: `src/autobattler/app.rs:1`, `src/autobattler/mod.rs:1`.

3) Full-ability score model (base+percentile for all stats) is not in the core data model.
   - Evidence: `src/character.rs:133`, `src/core/types.rs:7`.
   - Impact: percentiles for INT/WIS/CON/LOOKS/CHA are dropped when creating `PlayerProfile`
     (`src/autobattler/app.rs:620`), blocking future advancement and roll mechanics.

4) Percentile handling and carry rules do not match the plan (0-99 storage, carry-on-100).
   - Evidence: `src/autobattler/state.rs:381`, `src/autobattler/logic.rs:42`,
     `src/autobattler/logic.rs:55`.
   - Impact: 100 is rendered as "00" but never carried into base; 00 is not a stable representation.

5) BP spending thresholds ignore percentile edge cases (10/01, 16/01 boundaries).
   - Evidence: `src/autobattler/logic.rs:32`.

6) Points ledger and banked pools are missing.
   - Evidence: `src/autobattler/app.rs:142`, `src/autobattler/state.rs:340`.

7) Looks -> Charisma adjustment is not persisted in the stored stats.
   - Evidence: `src/autobattler/state.rs:479`, `src/autobattler/app.rs:155`.
   - Impact: `CreationState.stats` and `CharacterSave` keep raw CHA while `PlayerConfig` uses
     adjusted CHA, which complicates later audit/level-up math.

8) AP spending and progression tier changes are not exposed during creation or level-up.
   - Evidence: `src/autobattler/state.rs:403` (no progression fields), `src/game_logic.rs:270`
     (`PlayerConfig` has `progression` but UI never changes it).

9) Talent UI is duplicated instead of reused (plan says reuse sim_gui components).
   - Evidence: `src/autobattler/ui.rs:922`, `src/bin/sim_gui.rs:2258`.

10) `free_at_start` talent flag and auto-grant logic are missing.
    - Evidence: `data/talents.json` (no `free_at_start` fields), no handling in UI.

## RPG Plan Gaps (autobattler_rpg_plan.md)
1) Level-up decision point and growth systems are not implemented.
   - Evidence: `src/core/gameplay/progression.rs:1`, `src/autobattler/app.rs:534`
     (xp curve is always `None`), `src/autobattler/state.rs:13` (no LevelUp screen).

2) Training action is effectively a no-op (just starts another fight with rest-days).
   - Evidence: `src/autobattler/state.rs:72`, `src/autobattler/app.rs:505`.
   - `training_days` and `days_elapsed` are stored but never incremented.
     - Evidence: `src/autobattler/state.rs:103`, `src/autobattler/ui.rs:681`,
       `src/autobattler/app.rs:315`.

3) Time between encounters is not tracked or randomized; UI defaults to 0/1 days.
   - Evidence: `src/autobattler/state.rs:88`, `src/autobattler/app.rs:505`,
     `data/autobattler_config.json` (rest_days_between_encounters=8).

4) Weapon mastery growth and stat allocation rules are missing.
   - Evidence: no mastery updates in `src/core/gameplay/run.rs` or UI flow (`src/autobattler/app.rs:515`).

5) PlayerProfile serialization lacks progression, point pools, race/background, skills, etc.
   - Evidence: `src/core/types.rs:7`, `src/autobattler/state.rs:224`.

## Autobattler Plan Gaps (autobattler_plan.md)
1) Seeded determinism is not wired into the UI.
   - Evidence: `src/autobattler/app.rs:128` (entropy RNG), `src/autobattler/args.rs:3` (no seed
     option), `src/core/gameplay/run.rs:11` (RunState has no seed field).

2) Autobattler config lacks planned fields (enemy pools, biomes, difficulty ramp) and is only
   partially consumed by UI.
   - Evidence: `src/core/gameplay/config.rs:6`, `src/autobattler/app.rs:105`.

3) Between-battle decision loop is incomplete (no equip, allocate stats, or shops).
   - Evidence: `src/autobattler/ui.rs:464`, `src/autobattler/state.rs:72`.

4) Encounter variety and progression are limited to hobgoblin presets.
   - Evidence: `src/autobattler/app.rs:638`.

5) Reporting/persistence are incomplete for run sharing and replay.
   - Evidence: `src/autobattler/state.rs:300` (no seed/config/history fields).

## Additional Technical Risks
- Run state depends on CreationState/PlayerConfig for equipment and progression. If gear,
  progression, or point pools change during a run, RunState cannot persist them.
  - Evidence: `src/core/types.rs:7`, `src/autobattler/app.rs:573`.
- Run depth initialization differs between UI and core defaults (0 vs 1).
  - Evidence: `src/core/gameplay/run.rs:19`, `src/autobattler/app.rs:431`.

## Testing Gaps
- No tests for stat rolling rules, BP spending thresholds, honor math, or deterministic run replay.
  Existing tests cover loot determinism and wound healing only.
