# Weapon group mastery — 2026-09-07

Fighter mastery bonuses now belong to weapon groups. Changing equipment selects the
matching group's bonuses; it no longer transfers the previous weapon's mastery.
The editor exposes ATK, DMG, SPD and DEF for every catalog group, including Basic.
Speed is stored as a positive reduction and displayed with a minus sign.
Bows and Crossbows have no DEF mastery; Shields have only SPD and DEF.

Arthur Du Randt's current simulator preset uses the supplied bonuses:

| Group | ATK | DMG | SPD | DEF |
| --- | ---: | ---: | ---: | ---: |
| Polearms | +5 | +4 | −4 | +4 |
| Spears | +2 | +2 | −2 | +2 |

His other groups are zero. Historical level-one quick starts retain their original
bonuses. No supplied WEXP or adjacencies were imported, and campaign experience
and point-award rules are unchanged.

Primary and offhand attacks use their own group's ATK, DMG and SPD. Defensive
dual wielding adds both groups' DEF; Left Hand of Evonia uses the secondary
sword's DEF. Existing shield replacement, speed limits and Twelve Paths rules
remain in effect. Unarmed counters, including Eyesmite, use Unarmed ATK and DMG.

Presets save a `weapon_masteries` object keyed by snake-case group names, with
`attack`, `damage`, `speed` and `defense` values in each entry. Missing groups are
zero. A present, empty object explicitly means no mastery. Legacy presets with
only `masteries` migrate those bonuses to their saved primary and offhand weapon
groups, plus the separate Shields entry. Other bundled presets were migrated
using the same rule. Every group's bonuses survive saving while unequipped.

Regression coverage includes Arthur switching groups, mixed-group dual wielding,
Evonia, unarmed counters and Eyesmite, legacy migration, editor save/load, and
campaign profile conversion while preserving existing experience and points.

Validation: the all-features, all-targets functional run passed 419 tests; the
six affected unarmed/Eyesmite checks also passed after the final counter fix.
The separate release benchmark took 838 ms for 100,000 fights, exceeding its
existing 800 ms limit (the previous implementation also exceeded that limit).
