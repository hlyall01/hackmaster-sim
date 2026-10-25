# Carried melee and ranged weapons

Enable **Carry both a melee and a ranged weapon** in Equipment. The existing weapon, offhand, shield, materials, iron flag and default style form the melee setup. Configure the ranged weapon, weapon/ammunition materials, iron flag, Called Shot, default style and starting setup separately. The Called Shot option in Combat Maneuvers controls the melee setup; bow aiming does not have to carry into melee. Existing saved setups inherit the old shared preference until edited. Existing single-weapon characters remain single-weapon characters until this option is enabled.

**Automatically draw melee before engagement** is enabled by default. Each second, the simulator estimates contact using current distance, weapon reach, movement speeds, ranged retreat and arena edges. It begins the melee draw at the latest whole second that leaves the weapon ready for the first engagement attack under the current movement conditions. It rechecks after a readied final ranged shot, allowing the shot before dropping the bow when doing so costs no draw time. This is a prediction using current motion, not knowledge of future random effects.

Dropping the ranged weapon is instantaneous. Drawing a Small, Medium or Large melee weapon takes 0, 1 or 2 seconds respectively, using the existing movement rules. If the character is already engaged when it begins drawing a weapon, its weapon speed is also added before its first attack. Switching early avoids that late-draw penalty. Reach advantages, Called Shot aim delays and other ordinary attack rules still apply.

The character holds position and uses unarmed defense until drawing completes. Melee opening-style bonuses remain available after earlier ranged shots. Swapping does not heal wounds, reset spell pools, remove active effects or repair a broken shield. Each setup retains its weapon recovery, preventing switching from resetting an already used weapon's clock. Abandoned aiming is cancelled; returning to Gravesend starts a fresh aim.

A dropped/not-dropped flag is tracked for the ranged weapon. No dropped-item position or distance restriction is tracked. **Equip ranged setup** takes 1 second to pick up a dropped weapon, or d4p+2 seconds to retrieve a carried bow. Its own recovery and any fresh aiming delay still apply. Drawing and spellcasting cannot occur concurrently.

Equipment tactics use their own channel and run **Each second**, including while ranged attacks are recovering or aiming:

- **Equip melee setup** starts the melee draw.
- **Equip ranged setup** retrieves the ranged weapon.
- **Retain current equipment** explicitly overrides automatic switching while its conditions match.
- An explicit equip-ranged rule can keep an archer using ranged attacks despite the automatic-switch checkbox.
- **Engaged in melee** is available as a Boolean condition; existing distance, HP, reach, styles and time-to-contact conditions also work.

An automatic switch stays in melee until an equipment tactic requests ranged again. The live Status panel shows the active setup, draw readiness and whether the ranged weapon was dropped. Both setups and all options survive fighter presets and online character documents. Runtime draw/drop/recovery state resets between fights.

Combat log entries retain the weapon and Called Shot setting used for each historical attack, including offhand attacks, after subsequent equipment changes.

When the opponent has longer reach, the melee setup starts its one-second opening delay at contact while closing to its own range. It does not add another second after closing. Explicit melee Called Shot aiming still adds its normal delay.
