# Four new weapon styles

Implemented on 6 September 2026, following the supplied rules and the user's three clarifications. The earlier `weapon_style_audit_2026-09-06.md` records the state before this implementation.

All four styles are trained selections with no BP cost. They use the existing learned/default/active style controls and enforce their weapon proficiency and equipment requirements. Existing style acquisition metadata is outside this change.

| Style | Implemented behavior |
|---|---|
| Left Hand of Evonia | A size L large sword or polearm can be paired with a size S/M one-handed sword. The primary retains two-handed defense, with the secondary's mastery and weapon defense bonuses added. Every successful melee defense grants a reach-checked secondary sword attack, including when a normal counter misses. This is additional to ordinary perfect/near-perfect counters and does not start regular offensive dual-wield attacks or reset the primary attack timer. |
| One Path | A sword capable of both hacking and piercing, wielded in two hands, loses one step from each damage die and 1.5 feet of reach. Crushing mode ignores five total DR against heavy armor or an NPC/monster with at least 5 DR. Medium armor does not qualify merely because it has 5 DR. Piercing mode turns a precise called shot into a normal severity-based critical, including its extra damage dice. |
| Pilgrim's Path | Staff use gains +4 Defense against melee and ranged attacks, with weapon speed increased by two seconds. A legal Give Ground or Scamper Back triggers an immediate staff attack only if the opponent actually pursues and remains within reach. This reaction preserves the regular attack timer. |
| Reaper of Termon | Scythe/sickle criticals double the extra damage dice after severity is calculated. Matching Improved Critical expands the threshold to 18; Critical Mastery expands it to 17. These rolls must still hit. Applies to normal and counterattacks, including a qualifying offhand sickle. |

## Choices and controls

- **Parry:** the user defined this as any successful melee weapon defense; Evonia's strike is added to any ordinary perfect/near-perfect counter.
- **Die steps:** d12 → d10 → d8 → d6 → d4 → d3 → d2 → d1. Counts, penetration notation, and flat modifiers are preserved.
- **Precise piercing hits:** use existing critical severity/effect rules, including extra damage dice, as requested. No location-specific injury tables were invented.
- Select **One Path: piercing strikes** in Equipment to use piercing mode. Leaving it unchecked uses crushing mode; selecting no active style restores the original weapon profile.
- Select **Give ground** or **Scamper back** in Combat Maneuvers, or choose them in Tactical Directives when tactics are enabled. Pursuit is enabled by default; **Do not pursue retreating opponents** disables it.
- Evonia enables the offhand sword selector for its otherwise two-handed loadout. Both new mode/pursuit choices are saved in fighter presets and default safely when loading older presets.

The shared combat resolver supplies the damage and counter rules to duels and squad battles. Squad pursuit also updates grid positions and its event log. Duels use exact weapon reach; squad combat uses its existing minimum melee range of one grid cell, so a short sword can strike an adjacent unit but cannot reach a second cell.

## Verification

- 16 new tests cover eligibility, damage dice, DR qualification, called-shot criticals, Reaper critical thresholds/dice, additive Evonia counters and reach, pursuit/declined pursuit, unchanged reaction timers, tactical style switching, and preset compatibility.
- Release suite: 374 library tests and 21 binary tests pass with the two known failing checks excluded.
- Every target compiles with all features enabled.
- The existing Arthur preset test expects level 8, while the pre-existing edited preset has level 9. The preset was preserved.
- The existing 100,000-duel performance test requires at most 800 ms. The final implementation measured approximately 847 ms, versus 833 ms against the saved pre-change source on this machine; both exceed the threshold. Earlier implementation runs measured 826–827 ms.

No further rules decisions are pending. Broader gaps in the other 21 styles remain documented in the original audit.
