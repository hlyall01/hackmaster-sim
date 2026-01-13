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

## Phase 3: Wound System
You dont full heal after every fight, you gets wounds in battle. Each time you take damage you track it as a wound. Wounds all heal independently, and current HP is reduced by the sum of wound damage (max HP stays the same).
After a battle each wound heals by 1. Then a wound takes half a day for each point of damage in its current size to heal. Example: a 7 point wound becomes 6 immediately, then takes 3 days to become 5 (half-day x 6), then 2.5 days to become 4, etc. We will be having a random amount of days between encounters in the game, but for now just implement wounds and say there is 8 days between each encounter.

## Phase 4: Growth Systems
Expand leveling and talents in small steps.

Work
- Implement XP curve + level increments.
- Earn weapon mastery after every fight. Roll for it. 8d6 penetrating points.
- Add talent unlock flow. earn bp (build points) and advancement points (ap) on levelup to spend on talents.
- Add stat allocation rules (costs build points, roll for stat increase).

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

## Open Questions (need answers to progress)
- When do we roll stats and apply them to the player profile?
- Are talents purely passive modifiers, or can they unlock actions?
- Do we want full campaign persistence (save/load) in v1?
