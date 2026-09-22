# Agent Instructions

## Simulator scope
- This project ships only `sim_gui`. Preserve all simulator, character-editor, plotting, preset, tactical, spell, and calculator functionality.
- Keep `sim_gui.rs` for GUI work and calculation/domain logic in `game_logic` or the pure `core` modules.
- The old autobattler/Bevy plans are historical; do not reintroduce those applications or use their commands to validate this project.
- Use `references/` for game rules. Do not edit historical plans unless explicitly asked.
- Run `cargo test --lib --bin sim_gui` and `cargo check --all-targets` for shared-code changes. The GUI smoke tests render all tabs without a native window.
- Validate Windows packaging with `cargo build --release --bin sim_gui --target x86_64-pc-windows-gnu` when changing builds or assets.
- Store visual verification outputs in `screenshots/`.
- Do not run `sudo` commands; ask the user to perform privileged steps.

## Analysis and Working Tree Hygiene
- The user requires generated analysis, simulation results, reports, logs, and scratch helpers to be Git-ignored every time. Before creating them, choose an ignored destination or add a narrowly scoped `.gitignore` entry.
- Prefer `docs/analysis/` for local reports/results, `scripts/analysis/` for analysis scripts, and `target/` for temporary files. Cargo study runners can use `examples/analysis_*.rs`.
- Verify artifact paths with `git check-ignore` and check `git status --short` before finishing. Generated analysis must not appear as untracked or modified project files.
- Keep intentional source, tests, configuration, and fighter preset changes visible for review. Preserve existing user changes; do not discard, stage, commit, or hide them just to make Git status look clean.

## References
- Use the `references/` materials (tables, rules notes) as authoritative inputs when implementing mechanics.
- If a rule is missing or ambiguous, ask for clarification before guessing.
