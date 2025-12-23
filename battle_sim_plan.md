# 1v1 HackMaster 5e Battle Simulator Plan

## Goals and Scope
- Build a deterministic 1v1 combat simulator using HackMaster 5e core melee rules: count-up initiative, weapon speeds, penetrating dice, armor as DR (min 1), active defenses.
- Keep scope to melee only for now (skip ranged), no crit/fumble tables, no morale, no inside-reach penalty.
- Start with an engine/CLI; UI can be layered later. Keep data-driven tables (weapons, armor, shields) separate from logic.

## Rules Coverage (melee basics to model)
- **Time and Initiative:** Second-by-second count-up; initial penetrating d12 + modifiers sets first action; each action schedules the next at current count + weapon speed; readying/drawing blocks other actions; surprise/hesitation offsets start count.
- **Movement and Position:** Track distance; walking vs. running affects DV; charging adds attack bonus and DV penalty; closing/reach determines who can strike; retreat/withdraw costs movement and can trigger free attacks if sloppy.
- **Facing and Reach:** Longer reach threatens first on close; shields cover frontal arc (simplify to front in 1v1); prone targets are easier to hit and slower to act.
- **Attack Resolution:** Penetrating d20 + attack bonus vs. DV (base 10 + Dex mod + shield + situational); optional called-shot penalties as a toggle.
- **Active Defense Options:** Parry or dodge consume an action (or next action) to boost DV; only one active defense per incoming attack; shield block integrates into DV, with potential shield HP loss.
- **Weapons and Speed:** Weapon table with speed, damage die (penetrating), size, reach, two-handed options; ready time for drawing/swapping; off-hand/dual-wield penalties noted for completeness.
- **Damage and Mitigation:** Penetrating weapon damage + STR mod + two-handed bonus; armor gives DR/soak, reducing damage to minimum 1; shields can absorb damage before bearer; apply knockdown/threshold-of-pain checks when a single blow exceeds thresholds.
- **Wounds and Status:** Track HP; stunned/prone delay next action and penalize DV; bleeding rules if applicable; KO/death at 0 or below per 5e guidance.

## System Design
- **Data Model:** `Character` (stats, skills, DV components, speed mods, current HP/status), `Weapon`, `Armor`, `Shield`, `Action` (attack, parry, dodge, move, ready), `Event` (scheduled resolution at count), `BattleState` (time count, queue, positions).
- **Rules Modules:** Calculators for initiative, DV, attack modifiers, damage/penetration, movement/reach, status updates; tables for weapons/armor/shields loaded from data files.
- **Config and Inputs:** JSON configs for combatants (stats, gear, tactics script), weapon/armor/shield tables, and rule toggles (called shots, two-handed bonus, threshold-of-pain).

## Simulation Flow
- **Setup:** Load combatant builds and rule toggles; compute starting DV, movement, reach.
- **Initiative Start:** Roll penetrating d12 + modifiers; seed event queue with first actions; apply surprise/hesitation offsets.
- **Main Loop:** Pop next event by count; resolve action (movement, attack, parry/dodge), apply hit/miss and damage with DR, schedule next action based on weapon speed/recovery; check defeat/KO; stop when one is down or max time reached.
- **Logging/Telemetry:** Per-event log (time, actor, roll, modifiers, result, damage, statuses) and aggregate stats (TTK distribution, hit rate).

## Implementation Steps
1) Import weapon/armor/shield tables (e.g., from `weapon_stats.md`) into structured data (JSON/CSV) with speed, reach, damage dice, DR, two-handed flags.
2) Implement penetrating dice roller (d20 and weapon dice) with tests.
3) Build DV/attack calculators (base, Dex, shield, prone/charge, active defense) and two-handed bonus toggle.
4) Implement initiative/time engine with priority queue; support ready/draw durations.
5) Add movement/position and reach logic (no inside-reach penalty); include charge and retreat handling.
6) Implement melee attack resolution, damage with DR min 1, threshold-of-pain/knockdown checks, stun/prone updates.
7) Create tactics scripting (e.g., close, attack, parry when threatened) for automated duels.
8) Build CLI runner to load combatants/configs and run N simulations; output logs and summary stats.
9) Add validation harness with canned melee scenarios (reach advantage, heavy armor vs. light) and unit tests for math modules.

## Deliverables
- Engine code (Rust) under `src/` for core simulation and CLI.
- Data tables for weapons/armor/shields in `data/`.
- Example combatant configs and scenario scripts in `examples/`.
- README describing rules coverage, assumptions, and how to run simulations.
