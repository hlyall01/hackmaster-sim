# Autobattler RPG Cleanup + MVP Plan

## Goals
- Separate core rules/sim from UI/data loading so the codebase stays reusable.
- Establish a small, deterministic gameplay loop: fight random enemies, get loot, gain XP.
- Create a foundation for leveling, talents, and stat growth without rewriting core systems.

## Guiding Principles
- Core modules are pure (no UI, no file I/O).
- Data adapters own JSON/file loading and validation.
- Gameplay loop is deterministic with injectable RNG.
- All cross-layer data flow uses typed IDs + explicit conversion.

## Phase 0: Baseline Cleanup (stabilize boundaries)
- Inventory current responsibilities (done in `boundary_audit.md`).
- Ensure sim emits structured events only (already started).
- Replace remaining UI-only types in core (keep colors in UI).
- Remove duplicate rules/data in `src/main.rs` or re-home them into core.

Deliverables
- Core modules compile without `eframe/egui` imports.
- One path for weapon/material data (no duplicates).

## Phase 1: Split Data Loading from Rules
Move JSON and embedded fallback logic out of `src/game_logic.rs`.

Work
- Create `src/data/` (or `src/adapters/`) with:
  - `weapons.rs`, `armor.rs`, `materials.rs`, `npc_presets.rs`, `fighter_presets.rs`
  - a thin `load_*` API that returns catalogs and parse errors
- Keep `game_logic` focused on pure transforms:
  - `build_combatant`, material bonuses, mastery rules
- If needed, rename `game_logic` to `builders` or `core_builders`.

Deliverables
- `game_logic` has no `std::fs` or `include_str!`.
- UI/CLI call `data::*` loaders and pass catalogs into core.

## Phase 2: Domain Types + Gameplay Layer
Introduce a gameplay layer to orchestrate fights, loot, and progression.

Work
- Expand `src/core/types.rs` with new domain types:
  - `PlayerProfile` (base stats, level, xp)
  - `Inventory` (gold, items)
  - `Talent` and `TalentSpec` (enum + metadata)
  - `EnemyProfile` (level, preset ID)
- Add `src/core/gameplay/`:
  - `run.rs`: `RunState`, `RunOutcome`, `FightResult`, `Reward`
  - `spawner.rs`: `EnemySpawner` (level-appropriate selection)
  - `loot.rs`: `LootTable` and `LootRoll`
  - `progression.rs`: XP curve, level-up + stat growth hooks
- Use `core::rng::SimRng` for deterministic rolls.

Deliverables
- A pure gameplay API that can be called from CLI/GUI.
- Minimal unit tests for spawner + loot determinism.

## Phase 3: MVP Autobattler Loop
Implement a first-pass loop for "fight -> loot -> xp".

Work
- Provide `gameplay::run_next_fight`:
  - Pick enemy from spawner (based on player level or run depth)
  - Convert `PlayerProfile` + gear into `CombatantSheet`
  - Sim a single fight via `core::sim`
  - Produce `FightResult` (win/lose, hp left, turns, events)
  - Apply rewards (gold + xp) if win
- Keep talents as passive placeholders (no effects yet).
- Store run state: `RunState { player, inventory, run_depth }`.

Deliverables
- CLI entry (or small test) that runs N fights in a row.
- Deterministic output with seeded RNG.

## Phase 4: Growth Systems
Expand leveling and talents in small steps.

Work
- Implement XP curve + level increments.
- Add talent unlock flow (choose one of three on level-up).
- Add stat allocation rules (fixed gain per level or choice).
- Add item upgrades (basic tiers for weapons/armor).

Deliverables
- Level-up decision point in the loop.
- Serialized `PlayerProfile` and `Inventory` for saving.

## Phase 5: UI Integration
Expose the gameplay loop through the GUI.

Work
- Add "Run" panel to `sim_gui`:
  - Run depth, current level, gold, last reward
  - Button: "Fight Next"
  - Optional auto-run toggle
- Use the structured combat events for display.

Deliverables
- A simple, visible autobattler loop in UI.

## Very First Steps (next 1-2 PRs)
1) Extract JSON loading into `src/data/*` and simplify `game_logic`.
2) Create `core/gameplay/run.rs` with:
   - `RunState`, `RunOutcome`, `FightResult`, `Reward`
   - a stub `run_next_fight` that calls `core::sim`
3) Add minimal loot + XP in `core/gameplay/loot.rs` and `progression.rs`.

## Open Questions (need answers to progress)
- How should enemy strength scale (player level, run depth, or both)?
- Should loot be flat gold only for MVP, or include item drops?
- Are talents purely passive modifiers, or can they unlock actions?
- Do we want full campaign persistence (save/load) in v1?
