# Weapon style implementation audit

Date: 6 September 2026

Scope: the 25 weapon styles in the supplied pasted text, compared with the current working tree, including its pre-existing uncommitted changes. This is an audit, not an implementation. No existing source, data, or plan files were changed.

## Result

All 25 styles are technically implementable. That does not mean all are currently functional or that all can be implemented faithfully from the supplied text alone. Twenty-one have catalog entries; four are absent: **Left Hand of Evonia, One Path, Pilgrim's Path, and Reaper of Termon**.

The best first new addition is **Reaper of Termon**. The best small repairs are **Fymblwnger** and **Rohavalan Bridge**. Several other styles already have their main mechanics, while disarming, location-specific critical effects, hemorrhaging, and directional defense need broader support.

Effort below describes the remaining work in the main simulator. It is a relative engineering assessment, not a time estimate. Squad battler scheduling needs separate integration, described below. “Mostly present” is not certification of complete rule fidelity.

## All 25 styles

| Style | Current implementation | Remaining work / feasibility |
|---|---|---|
| Armeroci Pole | Partial: reach +1, speed +2, and an extra opening damage die exist. | **Medium.** Replace the once-per-activation opening flag with engagement tracking per opponent, including enemies entering reach while the user is already engaged. Enforce equal-or-longer reach for the extra engagement opportunity. |
| Crescent Moon | Catalog, prerequisite/loadout checks, and an internal modifier flag; the combat resolver does not consume that flag. | **Large.** Add the 18/17 near-perfect threshold, choice of disarm instead of damage, an immediate +6-defense weapon called shot, dropped-weapon state, random 2d6p displacement, and recovery/equipment consequences. Existing Eyesmite counter replacement is a useful precedent. |
| Doomrazor | Partial: removes Strength/mastery damage and disables damage penetration. “Hemorrhage” currently means +1 immediate HP damage when a wound is dealt. It also accepts hacking-only weapons. | **Large; rules needed.** Preserve piercing separately from hacking, correct eligibility, and implement the actual internal-hemorrhaging condition. The paste does not define its damage, timing, accumulation, or treatment. |
| Falling Sun | Mostly present: +2 speed, expanded attack/d20 defense penetration, and expanded primary damage penetration. | **Small follow-through.** Apply the rule consistently to shield-damage expressions and non-d20 defense rolls; those still use ordinary penetration. Check interaction with nonpenetrating jab/defensive-stance rules before claiming complete coverage. |
| Fymblwnger | Catalog effect becomes a flag, but combat never reads it. Give Ground's +5 defense still applies. | **Small for Give Ground.** Exclude the movement bonus when this attacker has the style. Full coverage also requires executable Scamper Back support; it currently exists as configuration/reference text rather than a resolved reaction. |
| Hammerer | Main resolver resets both weapon timers on any positive knockback, including counterattacks. Existing targeted test passes. | **Main mechanic present.** Add scheduler parity in squad battles, where a different initiative timer controls attacks. |
| Hobbler | Partial: -4 attack and generic severity effects on landed hits without extra critical dice. | **Large; rules needed.** There is no d2000 hit-location roll or location-specific injury table. Current critical effects depend only on severity. Also settle how a natural critical interacts with the style's extra-dice exclusion. |
| Ithican Prince | Buckler/small-sword gating and half the Intelligence attack bonus applied to damage and defense exist; targeted test passes. | **Main mechanic present.** Low effort for additional edge-case verification and consistent proficiency enforcement. |
| Kanian Impaler | Size L spear gating and a -5 adjustment to the target's knockback threshold exist. No selectable launch direction. | **Medium.** Express the adjustment as a size-category operation where necessary, add direction selection at double-or-greater knockback, and use normal collision/boundary handling. Current knockback only moves directly away. |
| Left Hand of Evonia | Absent. | **Large.** Permit a large two-handed weapon plus a smaller sword, retain the primary's two-handed defense behavior, retain an offhand attack profile, and schedule a reach-checked sword counter on a parry. Define precisely which resolved defense constitutes a “parry”; Full Parry currently does not supply a distinct parry-result event. |
| One Path | Absent. Called-shot precision and heavy-armor DR reduction provide some building blocks. | **Large; rules needed.** Preserve Hacking/Piercing/Crushing types and select attack mode, define the die-step reduction, implement -1.5 ft reach and a total 5-point DR bypass, and add unarmored-location critical effects. Requires the relevant critical/location rules and a die-step mapping. |
| Pilgrim's Path | Absent. Flat defense/speed modifiers and a Give Ground reaction already have extension points. | **Medium.** Add +4 defense/+2 speed and a reach-checked reaction when an attacker actually pursues. Model pursuit as an explicit event/choice and support Scamper Back. Avoid granting the reaction merely because a defender selected retreat. |
| Quiet River | Partial: half damage, direct-hit DR bypass, and doubled unarmed mastery defense exist. | **Larger unarmed/shield work.** Add the full-defense override and d4p-2 shield-damage profile; Fist currently has no shield-damage expression. Implement the armed/unarmed counterattack exception and apply DR bypass consistently on shield contacts. The loadout check currently disables the entire style with armor/shield, although the paste specifically attaches that restriction to the extra defensive benefits. |
| Reaper of Termon | Absent; scythe/sickle data, critical thresholds, Improved Critical, Critical Mastery, and extra critical dice already exist. | **Small; best new addition.** Double the count of extra critical dice after severity is determined, before rolling them, and apply the conditional natural-18/natural-17 thresholds to the qualifying weapon. Reuse the rule in normal and counterattacks. |
| Regenstat | +1 attack/defense stacks up to +8 and hit/miss resets exist; targeted test passes. | **Medium.** Count currently engaged opponents and require exactly one; the current activation test checks only the style flag. Establish stack behavior when the opponent changes or another enemy engages. |
| Returner | -4 defense, a limited return counter, and a doubled counter after skipping the opening attack exist. | **Medium.** Make the opening sacrifice optional instead of automatic. Audit attack-window refresh across all attack rolls and simultaneous resolution: the current return counter is attempted after incoming damage/trauma, and is blocked if that damage incapacitates or kills its user. Confirm the intended timing of “as they attack you.” |
| Rhdwng Flow | Sets a style flag but no combat code reads it. Thrown damage currently has no movement-pace-based Strength adjustment for the style to override. | **Medium baseline work.** Implement the normal movement/Strength relationship, then exempt this style. Otherwise selecting it has no distinct combat effect. Use the applicable movement/thrown-weapon rules to specify the baseline. |
| Rohavalan Bridge | Speed/minimum-speed/reach multipliers and 2d4p for margins below 10 exist; targeted test passes. | **Small repair.** Require a polearm that grants full d20p defense. Current checks accept every polearm and explicitly accept Staff, which the catalog places in Basic. Qualifying catalog polearms include Halberd, Poleaxe, Polehammer, and Swordstaff. |
| Scorn of the Dissendri | Catalog and internal modifier flag; the flag does not reach combat resolution. | **Medium.** Allow two attacks for opening/perfect/near-perfect opportunities, with the correct weapon profiles and reach. Build a real offhand attack profile in defensive dual wield as well as offensive dual wield. Counter outcomes currently contain only one optional counterattack. |
| Shield of Blades | Partial: the main simulator enables the persistent melee weapon-defense bonus and supports stacking with Storm through Perfect Two-Weapon Fighting. | **Larger defense work.** Ranged defense still follows ordinary shield/dodge logic and discards the weapon-defense bonus. Add missile visibility and frontal/flank/rear eligibility; current combat has no facing-aware defense resolution. Enforce the required sword size/reach on the wielded equipment. |
| Six Paths | The five-point shield-contact window exists. A hit currently pulls the next primary attack forward to the next second. | **Medium; current behavior is wrong.** Queue a separate shield bash at +1 second with its own attack/damage profile, keeping the ordinary sword timer. The current shortcut creates another sword attack and can repeat on subsequent hits. Obtain/use the actual shield-bash profile rather than substituting sword damage. |
| Storm of Blades | Main simulator schedules the first offhand attack at primary +2 seconds and reduces subsequent recovery by one; targeted timing test passes. Stacking with Shield is supported. | **Main mechanic present.** Tighten equipment size/reach checks and add squad scheduler parity. |
| Three Mountains | Tracks a hit streak and forces the next trauma roll to 20 at three or more hits. | **Medium; current behavior is wrong.** Track the target as well as the streak and require a failed Physical Saving Throw against the latest adjusted attack. Constitution has a physical-save modifier, but no save is performed here. Specify repeated triggers after the third hit and ensure changing styles cannot erase an already imposed effect. |
| Twelve Paths | Large sword plus small shield/buckler, -3 damage, combined defense mastery, and -2 seconds on shield contact exist; targeted tests pass. | **Main mechanic present with limits.** Add actual defense-arc handling, verify same-second attacks when the timer becomes due, and propagate speedups into the squad scheduler. |
| Unbreakable Wall | +2 large/tower shield DR and shield-DR subtraction before breakage checks exist in normal and counterattack resolution. | **Main mechanic present.** Clarify whether the second sentence's breakage benefit also applies to smaller shields; current implementation restricts both benefits to large/tower shields. |

## Shared issues to handle before declaring styles complete

1. **Training versus BP purchasing.** All 21 catalog entries still specify `cost_bp: 20`. The GUI displays that value and generic talent costing charges configured BP without excluding weapon styles. Represent them as learned/trained options and keep teaching acquisition separate from paid talents. A combat-only configuration can record a style as already taught without simulating weeks of tuition.
2. **Learned versus active styles.** This separation already exists. Multiple styles can be learned, only one is active except the permitted Storm/Shield pair with Perfect Two-Weapon Fighting, and tactical selection can change style at an attack opportunity. Complete the weapon-ready/engagement decision points and equipment/proficiency revalidation. Several filters currently accept a weapon group or a learned proficiency more broadly than the pasted wielding requirement.
3. **Capability reporting overstates support.** `sim_capability_report` includes every catalog item in the Weapon Styles category in `supported_weapon_style_ids`; it does not verify implementation. In particular, Crescent Moon and Scorn have unused flags. Report complete, partial, and catalog-only support separately.
4. **Main simulator and squad battler have different schedulers.** Squad battles call the shared primary attack resolver but schedule actions using `initiative_ready_at`; the main simulator uses primary/secondary attack timers and separate post-attack hooks. Shared damage modifiers carry across, but dual attacks, followups, shield speedups, and timer resets cannot be assumed to do so. This affects Hammerer, Storm, Six Paths, Twelve Paths, and new reaction attacks.
5. **Damage types and positions are simplified.** Weapon JSON has H/P/C information, but loading reduces it to `hacking_or_piercing`. Shield arcs are read into a field marked unused. A faithful implementation of all rules requires retaining information that the current combat model drops.

## Suggested implementation sequence

1. Correct acquisition metadata and capability labels; use targeted behavioral tests rather than catalog presence as proof of support.
2. Add Reaper; repair Fymblwnger and Rohavalan; finish Falling Sun's roll coverage. Preserve existing main mechanics in Hammerer, Ithican Prince, Storm, Twelve Paths, and Unbreakable Wall.
3. Introduce a shared scheduled/reaction attack representation, including source weapon and target. Use it for Scorn, a real Six Paths shield bash, Pilgrim, and Left Hand, and integrate both combat schedulers.
4. Add engagement/target tracking for Armeroci and Regenstat, directed knockback for Kanian, configurable Returner choices/timing, and the actual Physical Saving Throw for Three Mountains.
5. Implement the movement baseline for Rhdwng, complete unarmed/shield handling for Quiet River, and add directional/visible-missile defense for Shield of Blades and Twelve Paths.
6. Add disarmed equipment, hemorrhage conditions, distinct damage types, and hit-location critical tables before claiming full Crescent Moon, Doomrazor, Hobbler, or One Path support.

The missing hemorrhage definition, location tables, die-step rule, shield-bash profile, and interpretations identified above should be resolved from authoritative rules/user decisions when those implementations are taken up. This audit does not invent replacements for them.

## Evidence anchors

Paths and line numbers refer to the audited working tree and may move after edits.

| Evidence | Location |
|---|---|
| Catalog entries and descriptions | `data/sim/talents.json:714` onward; Kanian at `:1414` |
| Style capability report includes the whole category | `src/game_logic.rs:1051` |
| Known/active selection and equipment checks | `src/game_logic.rs:1782`, `:1852`, `:1901` |
| Crescent and Scorn flags are only set | `src/game_logic.rs:3005`, `:3038` |
| Fymblwnger/Rhdwng flags are exported but never read by combat | `src/game_logic.rs:5754`, `:5775`; `src/core/sim/modifiers.rs:37`, `:44` |
| Falling Sun ordinary shield-damage cache | `src/game_logic.rs:5555`; `src/core/sim/combat.rs:1771` |
| Defensive offhand attack profile limitation | `src/game_logic.rs:5572` |
| Regenstat activation lacks opponent count | `src/core/sim/combat.rs:106` |
| Generic severity-only critical effects | `src/core/sim/combat.rs:343`, `:1998` |
| Doomrazor immediate damage addition | `src/core/sim/combat.rs:2046` |
| Three Mountains has no save or target identity | `src/core/sim/combat.rs:1438`, `:2221`; `src/core/sim/types.rs:349` |
| One optional counter, ordinary perfect/near-perfect branches | `src/core/sim/combat.rs:2232`; `src/core/sim/mod.rs:30` |
| Returner post-damage counter conditions | `src/core/sim/combat.rs:2310`; initial skip at `src/core/sim/types.rs:674` |
| Six Paths changes the sword timer | `src/core/sim/engine.rs:375` |
| Give Ground and automatic pursuit | `src/core/sim/engine.rs:954` |
| Knockback has fixed away direction | `src/core/sim/engine.rs:1161` |
| Armeroci opening state is activation-based | `src/core/sim/types.rs:910`; `src/core/sim/combat.rs:1890` |
| Separate squad scheduling | `src/squad_battler/combat.rs:987`, `:1006`; `src/core/sim/mod.rs:37` |
| Generic BP cost and GUI cost label | `src/autobattler/logic.rs:186`; `src/bin/sim_gui.rs:5655` |
| Damage type simplification / unused shield arc | `src/data/weapons.rs:49`, `:112` |

## Validation

Ran the existing library tests selected by the style names and related selection/timing filters:

```bash
cargo test --lib -- armeroci crescent_moon doomrazor falling_sun fymblwnger hammerer hobbler ithican_prince kanian_impaler quiet_river regenstat returner rhdwng_flow rohavalan scorn_of_the_dissendri shield_of_blades six_paths storm_of_blades three_mountains twelve_paths unbreakable_wall weapon_style style_switch shield_strike_speedup
```

Result: **43 passed, 0 failed, 317 filtered out**. Some selected tests are related race/preset tests. There are no behavioral tests in this selection for several absent/unused mechanics; passing results validate existing assertions, not fidelity to every clause in the new rules. No tests or combat implementation were added.

Repository guidance was read from `.codex/AGENTS.md`, `autobattler_rpg_plan.md`, and local combat references. The referenced `battle_sim_plan.md` was not present in the checkout.
