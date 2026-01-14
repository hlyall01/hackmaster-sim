# Talent System + GUI Selector Plan

## Goals
- Implement a data-driven talent system with requirements (talent prereqs, level gates, stat gates).
- Provide a GUI talent selector that enforces requirements and shows why talents are locked.
- Keep logic in `game_logic.rs`, and keep `sim_gui.rs` focused on GUI wiring.
- Align with Phase 4/5 in `autobattler_rpg_plan.md` without changing existing plans.

## Current State (for context)
- `TalentSpec` and `TalentSelection` exist in `src/core/types.rs`.
- `TalentCatalog` loads from `data/talents.json`.
- `game_logic::resolve_talent_modifiers` applies effects based on selections.
- There is no talent selector in the GUI and no requirements in the data model.

## Data Model + JSON Schema
1) Extend `TalentSpec` with optional requirements and optional costs.
   - `requirements: Vec<TalentRequirement>` (serde default, empty means no gate).
   - `bp_cost` or `ap_cost` (per-rank cost, default 1 if used).
2) Add `TalentRequirement` enum in `src/core/types.rs` (serde tagged).
   - `min_level { level: u8 }`
   - `min_stat { stat: AbilityKind, min_base: Option<u8>, min_percentile: Option<u8> }`
   - `requires_talent { id: String, min_rank: Option<u8> }`
3) Add `AbilityKind` enum (Strength, Dexterity, Intelligence, Wisdom, Constitution, Looks, Charisma)
   so requirements can be described in JSON without leaking GUI details.
4) Decide how to represent weapon-locked talents:
   - Option A: add `requires_weapon_selection: bool` to `TalentSpec`.
   - Option B: derive it in logic if any effect uses a weapon-specific modifier.
5) Update `data/talents.json` with sample requirements.
   Example (for reference only):
   {
     "id": "tough_hide",
     "requirements": [
       { "type": "min_level", "level": 3 },
       { "type": "min_stat", "stat": "constitution", "min_base": 12 },
       { "type": "requires_talent", "id": "tough_as_nails", "min_rank": 1 }
     ]
   }

## Talent Requirement Evaluation (game_logic)
1) Add a `TalentContext` struct with:
   - `level`
   - `stats` (AbilitySet or a trait accessor)
   - `current_talents`
2) Add `evaluate_talent_requirements(spec, context, catalog)` that returns a list of failures.
   - Keep failures typed so the GUI can show user-friendly reasons.
3) Add `can_rank_talent(spec, context, delta_rank, points_available)` for gating rank changes.
4) Add sanitation helpers:
   - Clamp ranks to `max_rank`.
   - Drop unknown talent ids.
   - If a weapon is required but missing, mark as invalid and show in UI.

## Player State + Progression
1) Add `talents: Vec<TalentSelection>` and `talent_points` to `PlayerProfile`
   or a dedicated `TalentState` that is embedded in `PlayerProfile`.
2) Ensure the autobattler flow copies talents from `PlayerProfile` into `PlayerConfig`.
3) In `core/gameplay/progression.rs`, grant talent points on level up (if using points).
4) Keep serialization ready for future save/load work (per `autobattler_rpg_plan.md`).

## GUI Talent Selector (sim_gui)
1) Add a "Talents" section in `render_player_editor`.
2) For each talent:
   - Show name, description, rank, and max rank.
   - Show a +/- control to change rank; disable when requirements fail or points are insufficient.
   - If weapon selection is required, render a weapon dropdown with a clear default.
3) Provide quick filters:
   - Search box
   - "Show available only" toggle
4) Display available talent points and reasons a talent is locked
   (level/stat/talent prerequisites) using the failure list from `game_logic`.

## Data Validation + Tests
1) Unit tests for requirement evaluation:
   - min level gate
   - min stat gate (base and percentile)
   - talent prereq gate
2) JSON parse tests for `TalentRequirement` and `AbilityKind`.
3) Regression test to confirm `resolve_talent_modifiers` still applies existing effects.

## Deliverables
- Updated talent schema in `src/core/types.rs` and requirement evaluation in `src/game_logic.rs`.
- Updated `data/talents.json` with requirements and optional cost fields.
- GUI selector in `src/bin/sim_gui.rs` wired to requirement evaluation.
- Tests for requirement evaluation and JSON parsing.

## Open Questions
- Which stat value should requirements use: base, percentile, derived modifier, or total score?
- Are talent prerequisites strictly ANDed, or do we need OR groups?
- How many talent points are earned per level, and are they separate from build points?
- Can a talent be removed if it invalidates dependent talents (block removal or cascade)?
