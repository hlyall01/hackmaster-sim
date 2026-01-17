# Bevy Transition Plan (Autobattler)

## Goals
- Replace the current `autobattler` eframe app with a Bevy-based sprite renderer.
- Move combat to a 2D grid while keeping the game 1v1 for now.
- Remove hardcoded 1v1 assumptions from sim core so N-combatant scaling is possible.
- Keep deterministic simulation and replay-friendly logging.

## Non-Goals (initial)
- Full multi-unit AI/targeting logic beyond 1v1.
- Replacing other tools (`sim_gui`, weapon plot, CLI) unless necessary.

## Decisions Needed
- Grid size (width/height) and tile size (feet per tile).
- Movement: orthogonal only or diagonals allowed.
- Distance metric for reach/range (Manhattan, Chebyshev, Euclidean).
- Combat range rules on a grid (adjacent melee, reach in tiles, ranged bands in tiles).
- UI direction: use `bevy_egui` for panels or native Bevy UI components.

## Phase 0: Prep + Dependency Strategy
- Keep `eframe` for `sim_gui` and weapon plot unless you want to migrate those too.
- Add Bevy deps for the new autobattler:
  - `bevy` (renderer + ECS).
  - `bevy_egui` (panels and debug UI).
  - Optional: `bevy_asset_loader` for icons/atlases.

## Phase 1: Sim Core Refactor (Grid + N-Combatant)
- Replace fixed `[2]` arrays with vectors + stable IDs:
  - `SimState` fields: `actors: Vec<SimActor>`, `combatants: Vec<Combatant>`.
  - Add `CombatantId` or use `usize` with explicit mapping in events.
  - Store team info (e.g., `TeamId` enum) to support later 2v2+.
- Introduce grid position and distance helpers:
  - `SimActor { pos: GridPos }` where `GridPos { x: i32, y: i32 }`.
  - Convert reach/range to tile distance via a helper (configurable metric).
  - Replace `distance()` with grid distance.
- Keep 1v1 run loop by spawning exactly two combatants for now.
- Update movement logic:
  - Move toward/away on grid by 1 tile step per tick (or derived from move speed).
  - Keep knockback as grid displacement with clamping to bounds.
- Update combat events to use `attacker_id`, `defender_id` instead of fixed indices.
- Preserve determinism:
  - RNG and event ordering should be stable for a given seed.

## Phase 2: Gameplay Orchestration Updates
- Update `run_next_fight` to build grid-aware combatants.
- Update `apply_fight_result` and log formatting to use IDs or names.
- Update any `build_combatants` helpers to return `Vec<Combatant>`.

## Phase 3: Bevy Autobattler App (Replace `src/bin/autobattler.rs`)
- Create Bevy app entry under `src/bin/autobattler.rs`.
- Systems:
  - `SimTickSystem`: advances sim based on fixed timestep.
  - `SpawnSystem`: spawns sprites for combatants and grid tiles.
  - `SyncSystem`: maps sim positions to sprite transforms.
  - `LogSystem`: pushes new combat events into UI buffers.
  - `InputSystem`: pause/resume/step, speed slider, etc.
- Rendering:
  - Simple sprite per combatant (colored circle/square).
  - Grid drawn via sprite tiles or a single mesh.
  - Damage floaters as text sprites.
- UI:
  - Use `bevy_egui` to render the existing run/creation panels.
  - Keep combat log in an egui panel (cap to N lines).

## Phase 4: Data + Save/Load Integration
- Reuse existing save formats where possible.
- Ensure `RunState` serialization remains unchanged for now.
- Map save data to combatants on grid (default positions).

## Phase 5: Cleanup + Tests
- Update sim tests for new vector-based structures and grid distances.
- Add a small grid-specific test suite for distance and movement.
- Keep unit tests for combat math intact.

## Phase 6: Iteration + Future Expansion
- Add multi-combatant targeting selection logic (priority rules).
- Add board placement (start rows, ranged backline).
- Add pathfinding if obstacles are introduced.

## Deliverables
- Bevy-based `autobattler` binary with 2D grid rendering.
- Sim core no longer assumes `[2]` combatants.
- Deterministic sim preserved.
- Plan for scaling to multi-combatant later.
