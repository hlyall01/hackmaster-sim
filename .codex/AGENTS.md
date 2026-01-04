# Agent Instructions

## Core Plan
- Use `battle_sim_plan.md` as the source of truth for scope, rules coverage, and implementation steps.
- Keep changes aligned with the plan's goals and sequencing; update the plan only if explicitly asked.
- Ensure that the `sim_gui.rs` is JUST for gui stuff and the logic is kept in `game_logic.rs`
- Rethink implementations when beneficial; piping precomputed or different information through is acceptable and preferred if it improves clarity, correctness, or performance.

## References
- Use the `references/` materials (tables, rules notes) as authoritative inputs when implementing mechanics.
- If a rule is missing or ambiguous, ask for clarification before guessing.
