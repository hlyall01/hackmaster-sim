# Boundary Audit (Starter)

## Current responsibilities
- `src/character.rs`: domain stat tables, derived stats, equipment types, materials, armor tables.
- `src/sim.rs`: combat loop, movement, state, RNG usage, damage parsing, string logging.
- `src/game_logic.rs`: JSON loading + embedded fallbacks, presets, `PlayerConfig` (UI),
  build Character/Combatant, material bonuses, threshold of pain, default catalogs.
- `src/bin/sim_gui.rs`: GUI, editor state, uses `game_logic` to build combatants.
- `src/bin/sim_cli.rs`: example CLI wiring.
- `src/main.rs`: separate weapon-plot UI with its own weapon/spec data and sim helpers.
- `data/*.json`: weapon/armor/material catalogs and presets.

## Coupling and leaks
- `game_logic` depends on `sim` and `character` types and pulls in UI-only `Color32`.
- `game_logic` mixes domain rules with file I/O (fs, include_str, serde).
- `sim` mixes combat rules with output formatting (string logs).
- `main.rs` duplicates weapon categories/data and rules that overlap with `game_logic`/`sim`.

## Target boundaries
- `core::types`: domain types (abilities, equipment, combatant sheet).
- `core::rules`: pure rule helpers (damage, mastery, thresholds, range math).
- `core::sim`: engine/state transitions; no UI, no file I/O; emits structured events.
- `core::catalog`: in-memory catalogs keyed by typed IDs.
- Data adapters: JSON loading, embedded fallbacks, validation; convert to catalog.
- UI/app: `sim_gui`, `main` plot, CLI; owns colors, widgets, and presentation.

## First moves (non-breaking)
- Split `PlayerConfig` into UI state (Color32, editor state) and core build config.
- Move JSON parsing/loading out of `game_logic` into data adapter modules.
- Replace index fields with typed IDs in the core API (UI keeps index mapping).
- Move log formatting out of `sim` into UI/CLI adapters (sim returns events).
