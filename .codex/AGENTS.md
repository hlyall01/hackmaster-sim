# Agent Instructions

## Core Plan
- Use `battle_sim_plan.md` and `autobattler_rpg_plan.md` as the source of truth for scope, rules coverage, and implementation steps.
- Keep changes aligned with the plan's goals and sequencing; update the plan only if explicitly asked.
- Ensure that the `sim_gui.rs` is JUST for gui stuff and the logic is kept in `game_logic.rs`
- Rethink implementations when beneficial; piping precomputed or different information through is acceptable and preferred if it improves clarity, correctness, or performance.
- Render scenes headlessly via `cargo run --bin autobattler -- --headless-screenshots --auto-screenshots --auto-start-run --auto-screenshot-count 1` and inspect `screenshots/latest.png` (renders the game view only, no egui).
- For sprite/weapon verification, run `cargo run --bin autobattler -- --sprite-review --headless-screenshots` and inspect `screenshots/sprite_review_*`.
- You can generate and inspect screenshots as part of visual verification; store outputs in `screenshots/`.
- Do not run `sudo` commands; ask the user to perform privileged steps.

## References
- Use the `references/` materials (tables, rules notes) as authoritative inputs when implementing mechanics.
- If a rule is missing or ambiguous, ask for clarification before guessing.
