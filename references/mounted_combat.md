# Mounted Combat

Source: *HackMaster Player's Handbook*, user-supplied `HackMaster - Players Handbook.pdf`.

- Main rules: printed pp. 233-235 (PDF pages 235-237), "Mounted Combat (Advanced Rule)", "Tactical Mounted Movement", and "Mounted Fighting Styles and Special Combat Techniques".
- Supporting rules: printed p. 186 (PDF page 188), "Riding (Specify Mount Species)"; printed p. 201 (PDF page 203), ordinary mounts and warhorse profiles.
- This is an organized rules extraction, not a verbatim transcription. Narrative explanations and duplicate summaries are condensed; mechanical values, exceptions, and relevant sidebars are retained. Tables and fractions were checked against rendered source pages.
- Page references below use the book's printed page numbers. Source ambiguities are recorded at the end, without supplying new rulings. This file documents rules; it does not establish simulator implementation status.

## Riding skill and eligibility (pp. 186, 233, 235)

Riding is specific to a mount species. Relevant abilities: Wisdom and Dexterity. Cost: 4 BP. Universal: No. Prerequisite: None. The skill covers riding and handling mounts.

Fighting from horseback requires at least Average Riding mastery.

| Riding mastery | Mounted melee penalty | Mounted archery penalty | Other capability |
| --- | --- | --- | --- |
| Novice | Mounted fighting not granted | Mounted fighting not granted | Utilize a riding horse |
| Average | -2 | -6 | Fight mounted |
| Advanced | None | -4 | Vault into the saddle; attempt a special slide-stop |
| Expert | None | -2 | Fight mounted |
| Master | None | None | Fight mounted without penalty |

A mounted combatant may use a short bow or light crossbow (p. 235 sidebar). Advanced Riding is a prerequisite for knighthood (p. 186 sidebar).

## Mounted advantages and attack resolution (pp. 233-234)

| Benefit | Rule |
| --- | --- |
| Reach | +2 feet |
| Attack | +2 mounted Attack bonus, with a special lance rule below |
| Defense, trotting or faster | +6 mounted Defense bonus |
| Defense, walking or slower | +2 mounted Defense bonus |
| Standard weapon damage against size M opponents | One extra damage die; use the smaller die if the weapon uses different dice |
| Saddle-borne weapon damage against size M opponents | Two extra damage dice instead of one; horseman's flail is the source example |

The ordinary extra-dice benefit applies against Medium opponents only. Larger opponents negate the height advantage; reaching smaller opponents requires shifting in the saddle enough to lose it.

Attacks against the rider that miss by 1-4 wound the mount instead. If the rider uses a shield, misses by 5-10 hit the shield (p. 233 footnote; see directional shield restrictions below).

The mount can also be attacked directly. The rider's direction of its defense cancels the loss of agility from carrying a rider.

### Lances (p. 233)

- At a trot or faster, the mounted Attack bonus becomes +6 instead of +2; reach is as indicated by the lance's length/weapon description, and the number of damage dice rolled is doubled.
- When the lance is not used from reach and with momentum, it instead receives the standard +2 Attack bonus and one extra damage die, regardless of the opponent's size, because the mount's mass aids the attack.
- The text does not specify the order for combining doubled lance dice with other damage-die bonuses; see source questions below.

### Momentum and knock-backs (p. 233)

| Mount's movement | Opponent's effective size for knock-back purposes |
| --- | --- |
| Canter/run | One size smaller |
| Full charge: gallop/sprint | Two sizes smaller |

A successful knock-back prevents a counter-attack.

### Charging mount impact and trampling (pp. 233-234)

Even if the rider misses, the defender must make an opposed dodge check to avoid the charging mount: `d20p + dex` versus `d20p + 10` (source notation).

- Success: leap 5 feet to one side, chosen randomly.
- Failure: suffer `d6p` impact damage. Multiply that damage result by four only for determining knock-back.
- If knocked back, roll `d4` for the direction and trample outcome:

| d4 | Result |
| --- | --- |
| 1 | Straight back and trampled |
| 2 | 45 degrees left |
| 3 | 45 degrees right |
| 4 | Straight back, no trample |

Trampling deals an additional `d6p` damage.

Whether or not the rider's attack succeeds, the mounted attacker continues in a straight line for another 5 feet when loping to 20 feet when galloping/charging before turning or stopping, potentially trampling multiple foes in that path. See the tactical movement rules and the source question about turning below.

## Dismounting, injured mounts, and entrapment (pp. 233-234)

- An attack on the rider that causes a knock-back knocks the rider out of the saddle. The fall inflicts `d4p` damage with no armor damage reduction.
- Any injury to the mount triggers a Tenacity check. On failure, it bolts in a random direction and gallops/sprints for 3 seconds. The rider may then attempt a Difficult Riding check to regain control, repeating every 3 seconds thereafter if necessary.
- If the mount is knocked back, fails a Trauma check, or is slain, the rider falls and suffers the same falling damage.
- For a fall caused by those events affecting the mount, any penetration roll on the falling damage indicates the rider's leg is trapped beneath the mount. Escape requires a Feat of Strength against `d20p + 13`.
- Escape Artist provides no benefit when freeing oneself from beneath a mount (p. 233 sidebar).

Ordinary riding animals have additional flight/control rules on p. 201, reproduced below. Their relationship to the general injury procedure is not explicitly resolved in this extract's source pages.

## Infantry counters (p. 234)

### Weapons set against a charge

A group of at least 10 with spears or polearms longer than 6 feet, set for charge against the ground, causes any standard horse to halt. The rider must make a Difficult Riding check to avoid being vaulted forward onto the spears.

Even if a polearm does not deter the mount, setting it against the ground or a similarly immovable object doubles its damage dice against the charging mount or rider. Pikes can retain a reach advantage over lances.

### Weapons designed to dismount riders

Certain polearms described in the equipment chapter can drag an armored rider from the saddle. A successful hit, excluding a shield hit, dismounts the rider. Inflict the falling damage above, but no damage for the weapon's actual attack.

| Target rider's movement | Dismounting weapon's Attack modifier |
| --- | --- |
| Stationary | +6 |
| Trot/jog | -2 |
| Canter/run | -4 |
| Gallop/sprint | -8 |

This passage supplies no separate modifier for a walking rider.

## Tactical mounted movement (pp. 234-235)

### Stopping

| Gait | Time and distance to stop |
| --- | --- |
| Walk or trot | Immediately |
| Lope | 1 second and 5 feet |
| Gallop | 4 seconds and 20 feet: 10 feet in the first second, then 10 feet over the next 3 seconds |

### Turning

A standing horse can change facing at any time, subject to the action times below. A walking horse can change up to three facings per 5 feet moved.

Above a walk, the horse may turn no more than once per second, at the end of a full second with no prior turn.

| Gait | Safe turn | Turn with a successful Riding check | Check difficulty stated in prose |
| --- | --- | --- | --- |
| Walk | Table says "Any"; prose limits it to 3 facings per 5 feet | Any | None specified |
| Trot | 60 degrees (1 facing) | 90 degrees (1 1/2 facings) | Average |
| Lope/canter | 45 degrees (3/4 facing) | 60 degrees (1 facing) | Average |
| Gallop | 30 degrees (1/2 facing) | 45 degrees (3/4 facing) | Not specified |

On a failed check for the larger turn, the horse turns only the safe amount. A second Riding check is required; failure throws the rider from the saddle. The trot paragraph explicitly calls the second check Average; the canter and gallop paragraphs do not restate a difficulty for that second check.

### Mounted action times

The table below preserves the printed entries, including the apparent inconsistency in the moving-horse rows.

| Action | Time |
| --- | --- |
| Turn standing horse, 2-4 facings | 1 second |
| Turn standing horse, 5+ facings | 2 seconds |
| Turn moving horse, >=2 facings (as printed) | 0 seconds |
| Turn moving horse, 3 facings | 1 second |
| Mount horse | 2 seconds |
| Jump from horse's back | 1 second |
| Vault into saddle | 1 second |

Vaulting requires Advanced Riding mastery (p. 186). The standing-horse table has no explicit one-facing entry.

### Slide-stop

Advanced Riding mastery permits attempting this maneuver (p. 186). At a full gallop, make a Difficult Riding check to stop/turn faster than normal.

- Success: the horse slides to a near-complete stop over 3 seconds and 15 feet, turning 90 degrees from its original direction.
- The horse or rider may choose to canter directly out of the stop without progressing through the intervening gaits. This decision must be immediate.
- The source's example states that 1 second after completing the slide-stop, the horse moves 35 feet in the new direction.
- Failure: the rider is thrown.

## Mounted fighting styles (p. 235)

| Style | Mounted availability and restrictions |
| --- | --- |
| Weapon and shield | Allowed; shield protection depends on direction and shield size |
| One-handed weapon only | Allowed freely; leaves the other hand available |
| Two one-handed weapons, attacking with both | Allowed; otherwise functions as on foot, with opportunities against foes on either side of the mount |
| Two one-handed weapons, defending with secondary weapon | Allowed; secondary-weapon Defense bonus applies only against foes on that weapon's side of the mount; otherwise as on foot |
| Shield only | Allowed; as on foot, subject to mounted shield coverage |
| Two shields only | Allowed; each shield protects only its own flank |
| Two-handed weapon | Cannot be employed by a standard mounted rider |
| One-handed weapon used two-handed | Cannot be employed by a standard mounted rider |

### Shield coverage

This table assumes a right-handed primary weapon. Reverse it for a left-handed primary weapon. No listed shield protects the rear weapon-arm flank.

| Shield | Directions protected while mounted |
| --- | --- |
| Buckler | All except rear right flank |
| Small shield | All except rear right flank |
| Medium shield | Left rear and left side flank, plus front on both sides |
| Large shield | Left front, left side, and left rear flank only |
| Body shield | Cannot be used mounted |

### Special combat techniques

The normal techniques apply, with these exceptions:

- To disengage, turn the mount and move away.
- At a walk, disengaging counts as a fighting withdrawal.
- At a trot, it counts as a scamper back.
- Fleeing still allows a standard Defense roll if the rider is aware of opponents.
- Use the mounted charging rules above.
- Ready Against Charge cannot be used while mounted.

## Supporting mount rules (p. 201)

### Ordinary riding horses, ponies, and mules

These animals lack the training and temperament of war steeds. Outfitting them with barding and charging opponents with a lance is beyond their capability, although mounted combat on them can occur.

- On encountering a hostile creature, they try to flee in the opposite direction regardless of the rider's wishes. An Average Riding check is required to rein them in.
- Whenever wounded, they make a determined effort to flee. The rider must make a Very Difficult Riding check each time the animal is wounded to prevent flight.
- Intelligent enemies usually target the rider as the greater threat. Predatory animals and unintelligent monsters are equally likely to attack mount or rider.
- Mule-related skill checks are one difficulty category harder. Riding a mule requires at least Average Riding: Equine mastery.

### Horse profiles

All four profiles on p. 201 have Speed 10, Initiative -1, short Reach, `d6p-2` damage, and Animal, High intelligence. Movement values below are reproduced as printed, without imposing an additional timing model.

| Statistic | Light riding horse | Rounsey (light warhorse) | Courser (medium warhorse) | Destrier (heavy warhorse) |
| --- | --- | --- | --- | --- |
| Hit Points | 20+4d8 | 24+4d8 | 30+4d8 | 35+5d8 |
| Size / weight | H / 800 lb | H / 900 lb | H / 1200 lb | H / 1600 lb |
| Tenacity | Cowardly | Steady | Steady | Steady |
| Fatigue Factor | -4 | -4 | -5 | -6 |
| Attack | +2 | +4 | +5 | +7 |
| Defense | +3 | +3 | +3 | +2 |
| Damage Reduction | 2 | 2 | 2 | 3 |
| ToP Save | 5 | 6 | 6 | 7 |
| Physical / Mental / Dodge saves | +2 / +2 / +2 | +4 / +4 / +4 | +5 / +5 / +5 | +7 / +7 / +7 |
| Crawl | 5 | 5 | 5 | 5 |
| Walk | 25 | 25 | 25 | 20 |
| Trot | 30 | 30 | 30 | 25 |
| Canter | 35 | 35 | 35 | 30 |
| Gallop | 40 | 40 | 40 | 35 |
| Special rider damage bonus at trot or faster | None listed | None listed | +1 damage die | +2 damage dice |

## Source questions to resolve before implementation

These are extraction notes, not additional game rules.

1. **Moving-horse action-time symbol:** the rendered p. 234 table really prints `>=2 facings` at 0 seconds and separately `3 facings` at 1 second. These overlap. No correction to `<=2` has been assumed.
2. **Turn timing and limits:** the action-time table, the one-turn-per-second rule above walking speed, the walk table's "Any", and the walk prose's three-facings-per-five-feet limit need to be reconciled when defining simulator actions.
3. **Post-attack straight movement:** p. 234 requires 5-20 additional straight-line feet before turning or stopping, while tactical turning elsewhere on that page permits turns per second. The precise interaction is not stated.
4. **Galloping turn checks:** the text does not give an explicit difficulty for the galloping turn's Riding check. It also leaves some follow-up check difficulties unstated.
5. **Ordinary-mount injury checks:** p. 201 requires a Very Difficult Riding check on each wound; p. 234 gives Tenacity, bolting, and subsequent Difficult recovery checks. Their exact sequencing/interaction is not specified on these pages.
6. **Damage-die stacking:** the order of lance doubling, mounted weapon dice, and courser/destrier dice is not explicit in the source. **Local ruling selected by the user on 2026-09-13:** double base lance dice, then add mounted-weapon dice and horse dice. Against Medium targets at trot or faster with reach/momentum, this yields 6d8p on a rounsey, 7d8p on a courser, and 8d8p on a destrier, plus flat modifiers once. This is a table ruling, not an official clarification. See `docs/mounted_attack_damage.md` for research and implementation details.
7. **Other unstated details:** the dismounting-weapon paragraph supplies no walking-speed modifier; the opposed charging dodge uses "dex" without further defining that term there; the slide-stop's 35-foot example does not account explicitly for differing horse profiles.

General penetration, skill-check, Tenacity, Trauma, knock-back, and weapon-stat rules remain dependencies. Existing mounted talents and other supplementary-source exceptions have not been merged into this handbook extraction.
