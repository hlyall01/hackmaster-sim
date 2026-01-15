# Easy Talent Implementation Plan

## Definition of easy
- Pure numeric modifiers to existing combat rolls (attack/defense/damage/speed/initiative/initiative die/armor DR/HP).
- Optional environment gating that already exists (temperature, natural surroundings).
- No new subsystems (skills, morale, saves, tracking, fumbles, criticals, etc.).

## Already done (no additional work)
- Natural Attunement (Armeroci): speed mod -1 in natural surroundings.
- Natural Protection (Armeroci): armor DR +1 in natural surroundings.
- Natural Awareness (Armeroci): initiative mod -2 in natural surroundings.
- Solid (Midlander): +8 HP.
- Armeroci race bonus: +1 defense and initiative die step.
- Vorova baseline temperature roll bonus/penalty.
- Heat Adaptation (Vorova): reduce hot penalty by 1.
- Frostheart (Vorova): triple cold bonus and hot penalty.

## Next easiest after review
1) Armored to the Teeth (Pather)
   - Add `TalentEffect::DamageBonusHeavyArmor { amount }` and a `TalentModifiers` field.
   - Apply the bonus in `roll_summary`/`build_combatant` only when armor is heavy.
   - Update `data/talents.json` with the new effect and add a unit test.

2) Presence (Vetlander)
   - Add a morale modifier field to `DerivedStats`/`PlayerSummary` (even if unused in sim).
   - Apply a +3 morale modifier when the talent is present.
   - Add a UI display in the Derived tab.

3) Lithe + Inheritor (saving throw bonuses)
   - Introduce save modifiers on `CombatantSheet` (dodge/mental/physical).
   - Apply talent/racial bonuses as flat modifiers and surface in Derived UI.
   - This enables other save-based talents later (Wetlands Immunity, etc.).

4) Critical hit system (unlocks multiple racials/class talents)
   - Add a critical check in `resolve_attack` (e.g., natural 20, or based on talent).
   - Track critical severity and allow damage modifiers (Defiant, Edge Counter, Critical Mastery, Wounding Criticals).
   - Requires new data on weapon crit ranges or talent-driven override.

5) Knockback size adjustments (Stout, Sturdy, racial size)
   - Add a size/knockback modifier to `CombatantSheet`.
   - Adjust knockback distance or thresholds (rule clarification needed).
