# Throwing Weapons Plan

## Goal
Add throwing-weapon support so combatants use thrown attacks at range, then transition to melee once within melee reach.

## Data Updates
- Add a new property to `data/weapons.json` for ranged band thresholds (feet) derived from `references/ranged_weapons_range_increments.md`.
- Populate that property for throwing weapons (Throwing axe, Throwing knife, Dart, Javelin, Pilum, Bola, Lasso, Net).
- Decide on a consistent schema (example: `range_bands_feet: [20, 30, 40, 60]` where each value is the max distance for the band).

## Rules/Logic Changes
- Treat throwing weapons as ranged weapons when distance is greater than melee reach.
- Throwing weapons gain strength to damage.
- Use the range-band attack modifiers from `references/ranged_weapons_range_increments.md` (d20p, d20p-4, d20p-6, d20p-8).
- Beyond the max band, prohibit attacks.
- Once distance is within melee reach, stop throwing and use melee rules.

## Simulation Flow Changes
- Extend action selection to prefer throwing attacks while distance > melee reach and a throwing weapon is readied.
- Add a transition check after each movement/attack step to switch to melee once melee reach is entered.
- Ensure thrown attack speed uses the existing weapon speed logic.

## Validation
- Add scenarios where combatants start at ranged distance with throwing weapons and verify:
  - Correct band modifier by distance.
  - Attacks stop beyond max range.
  - Melee attacks begin once reach is entered.
