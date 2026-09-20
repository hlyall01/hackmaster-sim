# Mounted attack and damage

## Selected table ruling

On 2026-09-13, the user selected: **double base lance damage dice, then add mounted-weapon dice and horse dice**. On 2026-09-14, they made this the simulator's fixed rule and requested automatic lance detection. This is a table ruling, not a claimed official erratum.

For a lance (`2d8p`) used on a warhorse at a trot or faster against a Medium target:

| Warhorse | Doubled base | Mounted weapon | Horse | Total dice |
| --- | --- | --- | --- | --- |
| Rounsey / light | 4d8p | 2d8p | None | 6d8p |
| Courser / medium | 4d8p | 2d8p | 1d8p | 7d8p |
| Destrier / heavy | 4d8p | 2d8p | 2d8p | 8d8p |

Add Strength, mastery, material, and other flat damage modifiers once afterward. Against a non-Medium target, the ordinary mounted-weapon bonus does not apply: the corresponding totals are 4d8p, 5d8p, and 6d8p. When stationary or using an ordinary riding horse, a mounted lance instead adds one die against any target size.

The lance's mounted Attack bonus is +6 instead of +2 on a warhorse at trot or faster. Average Riding imposes -2 melee Attack; Advanced or better imposes no melee penalty. The ordinary infantry Charge attack modifier is not added on top of mounted Attack.

## Controls

In the simulator's player editor, enable **Mounted** in the combat maneuver controls. Configure:

- Mount: ordinary riding horse, rounsey, courser, or destrier.
- Riding mastery: Average through Master.
- Target size: use the defender's body size, or explicitly treat targets as Medium/non-Medium for the scenario. This is useful for NPC presets without size metadata.
- Moving at a trot or faster.

Lance attacks are detected from the weapon used for each attack, including offhand and counterattacks. There is no manual lance checkbox or stacking selector. The moving-lance bonus requires a warhorse and trot-or-faster movement.

These are fixed conditions for a simulation, not automatic gait or charge-path tracking. Switching Mounted off disables all effects of these settings. Mounted damage remains dependent on the weapon being used and on the actual defender (unless the target-size override is selected). Body size is captured before Stout/Sturdy or temporary knock-back modifiers.

The settings persist in `maneuvers.mounted_combat` in fighter presets and load through the simulator, CLI, demo, and Bevy preset adapters. Old presets continue to load with defaults. Legacy `lance_stacking` and `lance_reach_and_momentum` fields are ignored when loading and omitted when saving, so old selections cannot override the fixed rule or automatic detection.

## Mounted Defense

Mounted riders receive +6 Defense at trot or faster, or +2 at walking speed or slower (PHB p. 233). This applies with any mount type and weapon, to melee and ranged Defense, including shield defense and counterattacks. It is added once during attack resolution and shown in Derived roll totals and their breakdowns. Base DV excludes this situational bonus. Switching Mounted off removes it.

## Damage handling

Mounted melee bonus dice apply to primary, secondary, and armed counterattacks. Mixed-die bonus rolls repeat the smaller die. Doubling duplicates only base dice, keeping the weapon's own flat term once. Penetration behavior is retained. The same dice adjustments apply to the weapon's shield-damage expression on a shield hit; a weapon without a shield-damage expression retains zero shield damage. Critical extra dice continue to use the original weapon pool.

The changes cover attack, damage, and Defense conditions. They do not implement mount hit points, missed attacks striking mounts, mounted reach bonuses, gait movement, trampling, dismounting, or mounted talent exceptions. Ranged attacks gain no extra mounted damage dice; their Riding penalties are handled separately from mounted melee Attack.

## Research

The supplied *HackMaster Player's Handbook* is the main source: printed pp. 186 (Riding), 201 (horse bonuses), 233 (mounted combat), and the weapon tables marking the lance and horseman's weapons with an H suffix. The extracted source material is in `references/mounted_combat.md`.

Web research on 2026-09-13 did not establish a reliable community consensus on stacking:

- [Kenzer & Company's official errata forum](https://kenzerco.com/forums/forum/hackmaster-rpg/hackmaster-errata/) displayed no topics. It provided no clarification for this interaction.
- A [fan's Zazahni cavalry stat blocks](https://snoutch.nekoweb.org/hackmaster/zazahni/simplified_troop_table.html) list mounted lance damage as 4d8p versus 2d8p unmounted for both light and heavy cavalry. This supports an example of simple base doubling, but its identical light/heavy damage and simplified attack numbers do not establish how courser/destrier bonuses stack.
- Searches of indexed forum and Reddit material mostly returned other game systems or HackMaster's older edition. Those results were not used to settle this edition's rules.

The 7d8p courser result above follows the user's selected ruling. It is not presented as a discovered consensus.
