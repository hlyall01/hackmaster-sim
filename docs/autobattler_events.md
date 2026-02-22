# Autobattler Event Catalog v1

Generated event count: 220

Roll model used by the resolver:
- `roll = d100` and success when `roll <= target`.
- `target = level + mastery_die_roll + ability_modifier + difficulty_shift`.
- Skill checks use the player's skill percentile level; stat-only checks use event stat level.
- Difficulty shifts: easy `+30`, medium `+15`, hard `+0`, very hard `-15`.

Each event section below documents availability, path gating, checks, and both result branches.

## 1. Ashen Sigil - Rumor on the Road (`evt_chain_01_01`)
- Path: Quest chain 01, step 1 of 5
- Availability: depth 1..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +10; xp +8; set flag `quest_chain_01_step_1_done`; set flag `quest_chain_01_step_1_success`; notes: Ashen Sigil: step 1 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -2; add wound 1; set flag `quest_chain_01_step_1_done`; notes: Ashen Sigil: step 1 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +13; xp +9; set flag `quest_chain_01_step_1_done`; set flag `quest_chain_01_step_1_success`; notes: Ashen Sigil: step 1 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -3; add wound 1; set flag `quest_chain_01_step_1_done`; triggers fight; notes: Ashen Sigil: step 1 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 2. Ashen Sigil - Hidden Mark (`evt_chain_01_02`)
- Path: Quest chain 01, step 2 of 5
- Availability: depth 2..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_01_step_1_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +13; xp +11; honor +1; set flag `quest_chain_01_step_2_done`; set flag `quest_chain_01_step_2_success`; notes: Ashen Sigil: step 2 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -3; add wound 2; set flag `quest_chain_01_step_2_done`; notes: Ashen Sigil: step 2 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + hard shift
Success: gold +17; xp +12; set flag `quest_chain_01_step_2_done`; set flag `quest_chain_01_step_2_success`; notes: Ashen Sigil: step 2 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -4; add wound 1; set flag `quest_chain_01_step_2_done`; triggers fight; notes: Ashen Sigil: step 2 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 3. Ashen Sigil - Complication (`evt_chain_01_03`)
- Path: Quest chain 01, step 3 of 5
- Availability: depth 3..999, tiers normal, elite, boss, unique_once=true
- Requires flags: `quest_chain_01_step_2_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +16; xp +14; item: healing salve; set flag `quest_chain_01_step_3_done`; set flag `quest_chain_01_step_3_success`; notes: Ashen Sigil: step 3 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -4; add wound 2; set flag `quest_chain_01_step_3_done`; notes: Ashen Sigil: step 3 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +21; xp +15; honor +1; set flag `quest_chain_01_step_3_done`; set flag `quest_chain_01_step_3_success`; notes: Ashen Sigil: step 3 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -5; add wound 2; set flag `quest_chain_01_step_3_done`; triggers fight; notes: Ashen Sigil: step 3 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 4. Ashen Sigil - Turning Point (`evt_chain_01_04`)
- Path: Quest chain 01, step 4 of 5
- Availability: depth 4..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_01_step_3_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +19; xp +17; set flag `quest_chain_01_step_4_done`; set flag `quest_chain_01_step_4_success`; notes: Ashen Sigil: step 4 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -5; add wound 3; set flag `quest_chain_01_step_4_done`; triggers fight; notes: Ashen Sigil: step 4 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +25; xp +18; honor +1; set flag `quest_chain_01_step_4_done`; set flag `quest_chain_01_step_4_success`; notes: Ashen Sigil: step 4 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -6; add wound 2; set flag `quest_chain_01_step_4_done`; triggers fight; notes: Ashen Sigil: step 4 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 5. Ashen Sigil - Final Reckoning (`evt_chain_01_05`)
- Path: Quest chain 01, step 5 of 5
- Availability: depth 5..999, tiers boss, elite, unique_once=true
- Requires flags: `quest_chain_01_step_4_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +22; xp +20; honor +1; item: lockpick set; set flag `quest_chain_01_step_5_done`; set flag `quest_chain_01_step_5_success`; set flag `quest_chain_01_complete`; notes: Ashen Sigil: step 5 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -6; add wound 3; set flag `quest_chain_01_step_5_done`; set flag `quest_chain_01_complete`; triggers fight; notes: Ashen Sigil: step 5 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +29; xp +21; honor +1; set flag `quest_chain_01_step_5_done`; set flag `quest_chain_01_step_5_success`; set flag `quest_chain_01_complete`; notes: Ashen Sigil: step 5 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -7; add wound 2; set flag `quest_chain_01_step_5_done`; set flag `quest_chain_01_complete`; triggers fight; notes: Ashen Sigil: step 5 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 6. Broken Oath - Rumor on the Road (`evt_chain_02_01`)
- Path: Quest chain 02, step 1 of 5
- Availability: depth 2..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +11; xp +8; set flag `quest_chain_02_step_1_done`; set flag `quest_chain_02_step_1_success`; notes: Broken Oath: step 1 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -3; add wound 1; set flag `quest_chain_02_step_1_done`; notes: Broken Oath: step 1 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +14; xp +9; set flag `quest_chain_02_step_1_done`; set flag `quest_chain_02_step_1_success`; notes: Broken Oath: step 1 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -3; add wound 1; set flag `quest_chain_02_step_1_done`; triggers fight; notes: Broken Oath: step 1 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 7. Broken Oath - Hidden Mark (`evt_chain_02_02`)
- Path: Quest chain 02, step 2 of 5
- Availability: depth 3..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_02_step_1_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +14; xp +11; honor +1; set flag `quest_chain_02_step_2_done`; set flag `quest_chain_02_step_2_success`; notes: Broken Oath: step 2 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -4; add wound 2; set flag `quest_chain_02_step_2_done`; notes: Broken Oath: step 2 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +18; xp +12; set flag `quest_chain_02_step_2_done`; set flag `quest_chain_02_step_2_success`; notes: Broken Oath: step 2 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -4; add wound 1; set flag `quest_chain_02_step_2_done`; triggers fight; notes: Broken Oath: step 2 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 8. Broken Oath - Complication (`evt_chain_02_03`)
- Path: Quest chain 02, step 3 of 5
- Availability: depth 4..999, tiers normal, elite, boss, unique_once=true
- Requires flags: `quest_chain_02_step_2_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + hard shift
Success: gold +17; xp +14; item: lantern oil; set flag `quest_chain_02_step_3_done`; set flag `quest_chain_02_step_3_success`; notes: Broken Oath: step 3 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -5; add wound 2; set flag `quest_chain_02_step_3_done`; notes: Broken Oath: step 3 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +22; xp +15; honor +1; set flag `quest_chain_02_step_3_done`; set flag `quest_chain_02_step_3_success`; notes: Broken Oath: step 3 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -5; add wound 2; set flag `quest_chain_02_step_3_done`; triggers fight; notes: Broken Oath: step 3 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 9. Broken Oath - Turning Point (`evt_chain_02_04`)
- Path: Quest chain 02, step 4 of 5
- Availability: depth 5..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_02_step_3_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +20; xp +17; set flag `quest_chain_02_step_4_done`; set flag `quest_chain_02_step_4_success`; notes: Broken Oath: step 4 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -6; add wound 3; set flag `quest_chain_02_step_4_done`; triggers fight; notes: Broken Oath: step 4 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +26; xp +18; honor +1; set flag `quest_chain_02_step_4_done`; set flag `quest_chain_02_step_4_success`; notes: Broken Oath: step 4 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -6; add wound 2; set flag `quest_chain_02_step_4_done`; triggers fight; notes: Broken Oath: step 4 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 10. Broken Oath - Final Reckoning (`evt_chain_02_05`)
- Path: Quest chain 02, step 5 of 5
- Availability: depth 6..999, tiers boss, elite, unique_once=true
- Requires flags: `quest_chain_02_step_4_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +23; xp +20; honor +1; item: map scrap; set flag `quest_chain_02_step_5_done`; set flag `quest_chain_02_step_5_success`; set flag `quest_chain_02_complete`; notes: Broken Oath: step 5 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -7; add wound 3; set flag `quest_chain_02_step_5_done`; set flag `quest_chain_02_complete`; triggers fight; notes: Broken Oath: step 5 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +30; xp +21; honor +1; add wound 1; set flag `quest_chain_02_step_5_done`; set flag `quest_chain_02_step_5_success`; set flag `quest_chain_02_complete`; notes: Broken Oath: step 5 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -7; add wound 2; set flag `quest_chain_02_step_5_done`; set flag `quest_chain_02_complete`; triggers fight; notes: Broken Oath: step 5 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 11. Salt Cartel - Rumor on the Road (`evt_chain_03_01`)
- Path: Quest chain 03, step 1 of 5
- Availability: depth 3..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +12; xp +8; set flag `quest_chain_03_step_1_done`; set flag `quest_chain_03_step_1_success`; notes: Salt Cartel: step 1 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -1; add wound 1; set flag `quest_chain_03_step_1_done`; notes: Salt Cartel: step 1 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +15; xp +9; set flag `quest_chain_03_step_1_done`; set flag `quest_chain_03_step_1_success`; notes: Salt Cartel: step 1 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -3; add wound 1; set flag `quest_chain_03_step_1_done`; triggers fight; notes: Salt Cartel: step 1 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 12. Salt Cartel - Hidden Mark (`evt_chain_03_02`)
- Path: Quest chain 03, step 2 of 5
- Availability: depth 4..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_03_step_1_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +15; xp +11; honor +1; set flag `quest_chain_03_step_2_done`; set flag `quest_chain_03_step_2_success`; notes: Salt Cartel: step 2 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -2; add wound 2; set flag `quest_chain_03_step_2_done`; notes: Salt Cartel: step 2 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +19; xp +12; set flag `quest_chain_03_step_2_done`; set flag `quest_chain_03_step_2_success`; notes: Salt Cartel: step 2 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -4; add wound 1; set flag `quest_chain_03_step_2_done`; triggers fight; notes: Salt Cartel: step 2 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 13. Salt Cartel - Complication (`evt_chain_03_03`)
- Path: Quest chain 03, step 3 of 5
- Availability: depth 5..999, tiers normal, elite, boss, unique_once=true
- Requires flags: `quest_chain_03_step_2_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +18; xp +14; item: lockpick set; set flag `quest_chain_03_step_3_done`; set flag `quest_chain_03_step_3_success`; notes: Salt Cartel: step 3 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -3; add wound 2; set flag `quest_chain_03_step_3_done`; notes: Salt Cartel: step 3 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +23; xp +15; honor +1; set flag `quest_chain_03_step_3_done`; set flag `quest_chain_03_step_3_success`; triggers fight; notes: Salt Cartel: step 3 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -5; add wound 2; set flag `quest_chain_03_step_3_done`; triggers fight; notes: Salt Cartel: step 3 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 14. Salt Cartel - Turning Point (`evt_chain_03_04`)
- Path: Quest chain 03, step 4 of 5
- Availability: depth 6..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_03_step_3_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +21; xp +17; set flag `quest_chain_03_step_4_done`; set flag `quest_chain_03_step_4_success`; notes: Salt Cartel: step 4 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -4; add wound 3; set flag `quest_chain_03_step_4_done`; triggers fight; notes: Salt Cartel: step 4 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +27; xp +18; honor +1; set flag `quest_chain_03_step_4_done`; set flag `quest_chain_03_step_4_success`; triggers fight; notes: Salt Cartel: step 4 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -6; add wound 2; set flag `quest_chain_03_step_4_done`; triggers fight; notes: Salt Cartel: step 4 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 15. Salt Cartel - Final Reckoning (`evt_chain_03_05`)
- Path: Quest chain 03, step 5 of 5
- Availability: depth 7..999, tiers boss, elite, unique_once=true
- Requires flags: `quest_chain_03_step_4_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +24; xp +20; honor +1; item: bone charm; set flag `quest_chain_03_step_5_done`; set flag `quest_chain_03_step_5_success`; set flag `quest_chain_03_complete`; notes: Salt Cartel: step 5 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -5; add wound 3; set flag `quest_chain_03_step_5_done`; set flag `quest_chain_03_complete`; triggers fight; notes: Salt Cartel: step 5 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +31; xp +21; honor +1; set flag `quest_chain_03_step_5_done`; set flag `quest_chain_03_step_5_success`; set flag `quest_chain_03_complete`; triggers fight; notes: Salt Cartel: step 5 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -7; add wound 2; set flag `quest_chain_03_step_5_done`; set flag `quest_chain_03_complete`; triggers fight; notes: Salt Cartel: step 5 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 16. Moonwell Pact - Rumor on the Road (`evt_chain_04_01`)
- Path: Quest chain 04, step 1 of 5
- Availability: depth 4..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +9; xp +8; set flag `quest_chain_04_step_1_done`; set flag `quest_chain_04_step_1_success`; notes: Moonwell Pact: step 1 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -2; add wound 1; set flag `quest_chain_04_step_1_done`; notes: Moonwell Pact: step 1 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +16; xp +9; set flag `quest_chain_04_step_1_done`; set flag `quest_chain_04_step_1_success`; notes: Moonwell Pact: step 1 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -3; add wound 1; set flag `quest_chain_04_step_1_done`; triggers fight; notes: Moonwell Pact: step 1 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 17. Moonwell Pact - Hidden Mark (`evt_chain_04_02`)
- Path: Quest chain 04, step 2 of 5
- Availability: depth 5..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_04_step_1_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +12; xp +11; honor +1; set flag `quest_chain_04_step_2_done`; set flag `quest_chain_04_step_2_success`; notes: Moonwell Pact: step 2 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -3; add wound 2; set flag `quest_chain_04_step_2_done`; notes: Moonwell Pact: step 2 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + hard shift
Success: gold +20; xp +12; set flag `quest_chain_04_step_2_done`; set flag `quest_chain_04_step_2_success`; notes: Moonwell Pact: step 2 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -4; add wound 1; set flag `quest_chain_04_step_2_done`; triggers fight; notes: Moonwell Pact: step 2 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 18. Moonwell Pact - Complication (`evt_chain_04_03`)
- Path: Quest chain 04, step 3 of 5
- Availability: depth 6..999, tiers normal, elite, boss, unique_once=true
- Requires flags: `quest_chain_04_step_2_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +15; xp +14; item: map scrap; set flag `quest_chain_04_step_3_done`; set flag `quest_chain_04_step_3_success`; notes: Moonwell Pact: step 3 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -4; add wound 2; set flag `quest_chain_04_step_3_done`; notes: Moonwell Pact: step 3 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +24; xp +15; honor +1; set flag `quest_chain_04_step_3_done`; set flag `quest_chain_04_step_3_success`; notes: Moonwell Pact: step 3 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -5; add wound 2; set flag `quest_chain_04_step_3_done`; triggers fight; notes: Moonwell Pact: step 3 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 19. Moonwell Pact - Turning Point (`evt_chain_04_04`)
- Path: Quest chain 04, step 4 of 5
- Availability: depth 7..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_04_step_3_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +18; xp +17; set flag `quest_chain_04_step_4_done`; set flag `quest_chain_04_step_4_success`; notes: Moonwell Pact: step 4 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -5; add wound 3; set flag `quest_chain_04_step_4_done`; triggers fight; notes: Moonwell Pact: step 4 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +28; xp +18; honor +1; set flag `quest_chain_04_step_4_done`; set flag `quest_chain_04_step_4_success`; notes: Moonwell Pact: step 4 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -6; add wound 2; set flag `quest_chain_04_step_4_done`; triggers fight; notes: Moonwell Pact: step 4 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 20. Moonwell Pact - Final Reckoning (`evt_chain_04_05`)
- Path: Quest chain 04, step 5 of 5
- Availability: depth 8..999, tiers boss, elite, unique_once=true
- Requires flags: `quest_chain_04_step_4_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +21; xp +20; honor +1; item: smithing nails; set flag `quest_chain_04_step_5_done`; set flag `quest_chain_04_step_5_success`; set flag `quest_chain_04_complete`; notes: Moonwell Pact: step 5 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -6; add wound 3; set flag `quest_chain_04_step_5_done`; set flag `quest_chain_04_complete`; triggers fight; notes: Moonwell Pact: step 5 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +32; xp +21; honor +1; add wound 1; set flag `quest_chain_04_step_5_done`; set flag `quest_chain_04_step_5_success`; set flag `quest_chain_04_complete`; notes: Moonwell Pact: step 5 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -7; add wound 2; set flag `quest_chain_04_step_5_done`; set flag `quest_chain_04_complete`; triggers fight; notes: Moonwell Pact: step 5 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 21. Iron Witness - Rumor on the Road (`evt_chain_05_01`)
- Path: Quest chain 05, step 1 of 5
- Availability: depth 5..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +10; xp +8; set flag `quest_chain_05_step_1_done`; set flag `quest_chain_05_step_1_success`; notes: Iron Witness: step 1 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -3; add wound 1; set flag `quest_chain_05_step_1_done`; notes: Iron Witness: step 1 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +12; xp +9; set flag `quest_chain_05_step_1_done`; set flag `quest_chain_05_step_1_success`; notes: Iron Witness: step 1 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -3; add wound 1; set flag `quest_chain_05_step_1_done`; triggers fight; notes: Iron Witness: step 1 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 22. Iron Witness - Hidden Mark (`evt_chain_05_02`)
- Path: Quest chain 05, step 2 of 5
- Availability: depth 6..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_05_step_1_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +13; xp +11; honor +1; set flag `quest_chain_05_step_2_done`; set flag `quest_chain_05_step_2_success`; notes: Iron Witness: step 2 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -4; add wound 2; set flag `quest_chain_05_step_2_done`; notes: Iron Witness: step 2 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +16; xp +12; set flag `quest_chain_05_step_2_done`; set flag `quest_chain_05_step_2_success`; notes: Iron Witness: step 2 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -4; add wound 1; set flag `quest_chain_05_step_2_done`; triggers fight; notes: Iron Witness: step 2 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 23. Iron Witness - Complication (`evt_chain_05_03`)
- Path: Quest chain 05, step 3 of 5
- Availability: depth 7..999, tiers normal, elite, boss, unique_once=true
- Requires flags: `quest_chain_05_step_2_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (INTELLIGENCE) + hard shift
Success: gold +16; xp +14; item: bone charm; set flag `quest_chain_05_step_3_done`; set flag `quest_chain_05_step_3_success`; notes: Iron Witness: step 3 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -5; add wound 2; set flag `quest_chain_05_step_3_done`; notes: Iron Witness: step 3 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +20; xp +15; honor +1; set flag `quest_chain_05_step_3_done`; set flag `quest_chain_05_step_3_success`; notes: Iron Witness: step 3 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -5; add wound 2; set flag `quest_chain_05_step_3_done`; triggers fight; notes: Iron Witness: step 3 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 24. Iron Witness - Turning Point (`evt_chain_05_04`)
- Path: Quest chain 05, step 4 of 5
- Availability: depth 8..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_05_step_3_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +19; xp +17; set flag `quest_chain_05_step_4_done`; set flag `quest_chain_05_step_4_success`; notes: Iron Witness: step 4 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -6; add wound 3; set flag `quest_chain_05_step_4_done`; triggers fight; notes: Iron Witness: step 4 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +24; xp +18; honor +1; set flag `quest_chain_05_step_4_done`; set flag `quest_chain_05_step_4_success`; notes: Iron Witness: step 4 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -6; add wound 2; set flag `quest_chain_05_step_4_done`; triggers fight; notes: Iron Witness: step 4 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 25. Iron Witness - Final Reckoning (`evt_chain_05_05`)
- Path: Quest chain 05, step 5 of 5
- Availability: depth 9..999, tiers boss, elite, unique_once=true
- Requires flags: `quest_chain_05_step_4_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +22; xp +20; honor +1; item: signal whistle; set flag `quest_chain_05_step_5_done`; set flag `quest_chain_05_step_5_success`; set flag `quest_chain_05_complete`; notes: Iron Witness: step 5 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -7; add wound 3; set flag `quest_chain_05_step_5_done`; set flag `quest_chain_05_complete`; triggers fight; notes: Iron Witness: step 5 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +28; xp +21; honor +1; set flag `quest_chain_05_step_5_done`; set flag `quest_chain_05_step_5_success`; set flag `quest_chain_05_complete`; notes: Iron Witness: step 5 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -7; add wound 2; set flag `quest_chain_05_step_5_done`; set flag `quest_chain_05_complete`; triggers fight; notes: Iron Witness: step 5 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 26. Hollow Banner - Rumor on the Road (`evt_chain_06_01`)
- Path: Quest chain 06, step 1 of 5
- Availability: depth 6..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +11; xp +8; set flag `quest_chain_06_step_1_done`; set flag `quest_chain_06_step_1_success`; notes: Hollow Banner: step 1 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -1; add wound 1; set flag `quest_chain_06_step_1_done`; notes: Hollow Banner: step 1 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +13; xp +9; set flag `quest_chain_06_step_1_done`; set flag `quest_chain_06_step_1_success`; notes: Hollow Banner: step 1 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -3; add wound 1; set flag `quest_chain_06_step_1_done`; triggers fight; notes: Hollow Banner: step 1 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 27. Hollow Banner - Hidden Mark (`evt_chain_06_02`)
- Path: Quest chain 06, step 2 of 5
- Availability: depth 7..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_06_step_1_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +14; xp +11; honor +1; set flag `quest_chain_06_step_2_done`; set flag `quest_chain_06_step_2_success`; notes: Hollow Banner: step 2 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -2; add wound 2; set flag `quest_chain_06_step_2_done`; notes: Hollow Banner: step 2 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +17; xp +12; set flag `quest_chain_06_step_2_done`; set flag `quest_chain_06_step_2_success`; notes: Hollow Banner: step 2 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -4; add wound 1; set flag `quest_chain_06_step_2_done`; triggers fight; notes: Hollow Banner: step 2 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 28. Hollow Banner - Complication (`evt_chain_06_03`)
- Path: Quest chain 06, step 3 of 5
- Availability: depth 8..999, tiers normal, elite, boss, unique_once=true
- Requires flags: `quest_chain_06_step_2_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +17; xp +14; item: smithing nails; set flag `quest_chain_06_step_3_done`; set flag `quest_chain_06_step_3_success`; notes: Hollow Banner: step 3 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -3; add wound 2; set flag `quest_chain_06_step_3_done`; notes: Hollow Banner: step 3 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +21; xp +15; honor +1; set flag `quest_chain_06_step_3_done`; set flag `quest_chain_06_step_3_success`; triggers fight; notes: Hollow Banner: step 3 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -5; add wound 2; set flag `quest_chain_06_step_3_done`; triggers fight; notes: Hollow Banner: step 3 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 29. Hollow Banner - Turning Point (`evt_chain_06_04`)
- Path: Quest chain 06, step 4 of 5
- Availability: depth 9..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_06_step_3_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +20; xp +17; set flag `quest_chain_06_step_4_done`; set flag `quest_chain_06_step_4_success`; notes: Hollow Banner: step 4 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -4; add wound 3; set flag `quest_chain_06_step_4_done`; triggers fight; notes: Hollow Banner: step 4 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +25; xp +18; honor +1; set flag `quest_chain_06_step_4_done`; set flag `quest_chain_06_step_4_success`; triggers fight; notes: Hollow Banner: step 4 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -6; add wound 2; set flag `quest_chain_06_step_4_done`; triggers fight; notes: Hollow Banner: step 4 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 30. Hollow Banner - Final Reckoning (`evt_chain_06_05`)
- Path: Quest chain 06, step 5 of 5
- Availability: depth 10..999, tiers boss, elite, unique_once=true
- Requires flags: `quest_chain_06_step_4_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +23; xp +20; honor +1; item: travel cloak; set flag `quest_chain_06_step_5_done`; set flag `quest_chain_06_step_5_success`; set flag `quest_chain_06_complete`; notes: Hollow Banner: step 5 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -5; add wound 3; set flag `quest_chain_06_step_5_done`; set flag `quest_chain_06_complete`; triggers fight; notes: Hollow Banner: step 5 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +29; xp +21; honor +1; add wound 1; set flag `quest_chain_06_step_5_done`; set flag `quest_chain_06_step_5_success`; set flag `quest_chain_06_complete`; triggers fight; notes: Hollow Banner: step 5 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -7; add wound 2; set flag `quest_chain_06_step_5_done`; set flag `quest_chain_06_complete`; triggers fight; notes: Hollow Banner: step 5 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 31. Cinder Choir - Rumor on the Road (`evt_chain_07_01`)
- Path: Quest chain 07, step 1 of 5
- Availability: depth 7..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +12; xp +8; set flag `quest_chain_07_step_1_done`; set flag `quest_chain_07_step_1_success`; notes: Cinder Choir: step 1 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -2; add wound 1; set flag `quest_chain_07_step_1_done`; notes: Cinder Choir: step 1 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +14; xp +9; set flag `quest_chain_07_step_1_done`; set flag `quest_chain_07_step_1_success`; notes: Cinder Choir: step 1 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -3; add wound 1; set flag `quest_chain_07_step_1_done`; triggers fight; notes: Cinder Choir: step 1 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 32. Cinder Choir - Hidden Mark (`evt_chain_07_02`)
- Path: Quest chain 07, step 2 of 5
- Availability: depth 8..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_07_step_1_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +15; xp +11; honor +1; set flag `quest_chain_07_step_2_done`; set flag `quest_chain_07_step_2_success`; notes: Cinder Choir: step 2 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -3; add wound 2; set flag `quest_chain_07_step_2_done`; notes: Cinder Choir: step 2 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + hard shift
Success: gold +18; xp +12; set flag `quest_chain_07_step_2_done`; set flag `quest_chain_07_step_2_success`; notes: Cinder Choir: step 2 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -4; add wound 1; set flag `quest_chain_07_step_2_done`; triggers fight; notes: Cinder Choir: step 2 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 33. Cinder Choir - Complication (`evt_chain_07_03`)
- Path: Quest chain 07, step 3 of 5
- Availability: depth 9..999, tiers normal, elite, boss, unique_once=true
- Requires flags: `quest_chain_07_step_2_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +18; xp +14; item: signal whistle; set flag `quest_chain_07_step_3_done`; set flag `quest_chain_07_step_3_success`; notes: Cinder Choir: step 3 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -4; add wound 2; set flag `quest_chain_07_step_3_done`; notes: Cinder Choir: step 3 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +22; xp +15; honor +1; set flag `quest_chain_07_step_3_done`; set flag `quest_chain_07_step_3_success`; notes: Cinder Choir: step 3 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -5; add wound 2; set flag `quest_chain_07_step_3_done`; triggers fight; notes: Cinder Choir: step 3 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 34. Cinder Choir - Turning Point (`evt_chain_07_04`)
- Path: Quest chain 07, step 4 of 5
- Availability: depth 10..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_07_step_3_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +21; xp +17; set flag `quest_chain_07_step_4_done`; set flag `quest_chain_07_step_4_success`; notes: Cinder Choir: step 4 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -5; add wound 3; set flag `quest_chain_07_step_4_done`; triggers fight; notes: Cinder Choir: step 4 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +26; xp +18; honor +1; set flag `quest_chain_07_step_4_done`; set flag `quest_chain_07_step_4_success`; notes: Cinder Choir: step 4 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -6; add wound 2; set flag `quest_chain_07_step_4_done`; triggers fight; notes: Cinder Choir: step 4 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 35. Cinder Choir - Final Reckoning (`evt_chain_07_05`)
- Path: Quest chain 07, step 5 of 5
- Availability: depth 11..999, tiers boss, elite, unique_once=true
- Requires flags: `quest_chain_07_step_4_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +24; xp +20; honor +1; item: bandage roll; set flag `quest_chain_07_step_5_done`; set flag `quest_chain_07_step_5_success`; set flag `quest_chain_07_complete`; notes: Cinder Choir: step 5 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -6; add wound 3; set flag `quest_chain_07_step_5_done`; set flag `quest_chain_07_complete`; triggers fight; notes: Cinder Choir: step 5 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +30; xp +21; honor +1; set flag `quest_chain_07_step_5_done`; set flag `quest_chain_07_step_5_success`; set flag `quest_chain_07_complete`; notes: Cinder Choir: step 5 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -7; add wound 2; set flag `quest_chain_07_step_5_done`; set flag `quest_chain_07_complete`; triggers fight; notes: Cinder Choir: step 5 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 36. Greenfire Idol - Rumor on the Road (`evt_chain_08_01`)
- Path: Quest chain 08, step 1 of 5
- Availability: depth 1..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +9; xp +8; set flag `quest_chain_08_step_1_done`; set flag `quest_chain_08_step_1_success`; notes: Greenfire Idol: step 1 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -3; add wound 1; set flag `quest_chain_08_step_1_done`; notes: Greenfire Idol: step 1 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +15; xp +9; set flag `quest_chain_08_step_1_done`; set flag `quest_chain_08_step_1_success`; notes: Greenfire Idol: step 1 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -3; add wound 1; set flag `quest_chain_08_step_1_done`; triggers fight; notes: Greenfire Idol: step 1 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 37. Greenfire Idol - Hidden Mark (`evt_chain_08_02`)
- Path: Quest chain 08, step 2 of 5
- Availability: depth 2..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_08_step_1_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +12; xp +11; honor +1; set flag `quest_chain_08_step_2_done`; set flag `quest_chain_08_step_2_success`; notes: Greenfire Idol: step 2 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -4; add wound 2; set flag `quest_chain_08_step_2_done`; notes: Greenfire Idol: step 2 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +19; xp +12; set flag `quest_chain_08_step_2_done`; set flag `quest_chain_08_step_2_success`; notes: Greenfire Idol: step 2 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -4; add wound 1; set flag `quest_chain_08_step_2_done`; triggers fight; notes: Greenfire Idol: step 2 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 38. Greenfire Idol - Complication (`evt_chain_08_03`)
- Path: Quest chain 08, step 3 of 5
- Availability: depth 3..999, tiers normal, elite, boss, unique_once=true
- Requires flags: `quest_chain_08_step_2_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + hard shift
Success: gold +15; xp +14; item: travel cloak; set flag `quest_chain_08_step_3_done`; set flag `quest_chain_08_step_3_success`; notes: Greenfire Idol: step 3 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -5; add wound 2; set flag `quest_chain_08_step_3_done`; notes: Greenfire Idol: step 3 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +23; xp +15; honor +1; set flag `quest_chain_08_step_3_done`; set flag `quest_chain_08_step_3_success`; notes: Greenfire Idol: step 3 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -5; add wound 2; set flag `quest_chain_08_step_3_done`; triggers fight; notes: Greenfire Idol: step 3 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 39. Greenfire Idol - Turning Point (`evt_chain_08_04`)
- Path: Quest chain 08, step 4 of 5
- Availability: depth 4..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_08_step_3_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +18; xp +17; set flag `quest_chain_08_step_4_done`; set flag `quest_chain_08_step_4_success`; notes: Greenfire Idol: step 4 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -6; add wound 3; set flag `quest_chain_08_step_4_done`; triggers fight; notes: Greenfire Idol: step 4 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +27; xp +18; honor +1; set flag `quest_chain_08_step_4_done`; set flag `quest_chain_08_step_4_success`; notes: Greenfire Idol: step 4 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -6; add wound 2; set flag `quest_chain_08_step_4_done`; triggers fight; notes: Greenfire Idol: step 4 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 40. Greenfire Idol - Final Reckoning (`evt_chain_08_05`)
- Path: Quest chain 08, step 5 of 5
- Availability: depth 5..999, tiers boss, elite, unique_once=true
- Requires flags: `quest_chain_08_step_4_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +21; xp +20; honor +1; item: iron ration; set flag `quest_chain_08_step_5_done`; set flag `quest_chain_08_step_5_success`; set flag `quest_chain_08_complete`; notes: Greenfire Idol: step 5 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -7; add wound 3; set flag `quest_chain_08_step_5_done`; set flag `quest_chain_08_complete`; triggers fight; notes: Greenfire Idol: step 5 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +31; xp +21; honor +1; add wound 1; set flag `quest_chain_08_step_5_done`; set flag `quest_chain_08_step_5_success`; set flag `quest_chain_08_complete`; notes: Greenfire Idol: step 5 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -7; add wound 2; set flag `quest_chain_08_step_5_done`; set flag `quest_chain_08_complete`; triggers fight; notes: Greenfire Idol: step 5 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 41. Raven Toll - Rumor on the Road (`evt_chain_09_01`)
- Path: Quest chain 09, step 1 of 5
- Availability: depth 2..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +10; xp +8; set flag `quest_chain_09_step_1_done`; set flag `quest_chain_09_step_1_success`; notes: Raven Toll: step 1 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -1; add wound 1; set flag `quest_chain_09_step_1_done`; notes: Raven Toll: step 1 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +16; xp +9; set flag `quest_chain_09_step_1_done`; set flag `quest_chain_09_step_1_success`; notes: Raven Toll: step 1 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -3; add wound 1; set flag `quest_chain_09_step_1_done`; triggers fight; notes: Raven Toll: step 1 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 42. Raven Toll - Hidden Mark (`evt_chain_09_02`)
- Path: Quest chain 09, step 2 of 5
- Availability: depth 3..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_09_step_1_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +13; xp +11; honor +1; set flag `quest_chain_09_step_2_done`; set flag `quest_chain_09_step_2_success`; notes: Raven Toll: step 2 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -2; add wound 2; set flag `quest_chain_09_step_2_done`; notes: Raven Toll: step 2 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +20; xp +12; set flag `quest_chain_09_step_2_done`; set flag `quest_chain_09_step_2_success`; notes: Raven Toll: step 2 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -4; add wound 1; set flag `quest_chain_09_step_2_done`; triggers fight; notes: Raven Toll: step 2 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 43. Raven Toll - Complication (`evt_chain_09_03`)
- Path: Quest chain 09, step 3 of 5
- Availability: depth 4..999, tiers normal, elite, boss, unique_once=true
- Requires flags: `quest_chain_09_step_2_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +16; xp +14; item: bandage roll; set flag `quest_chain_09_step_3_done`; set flag `quest_chain_09_step_3_success`; notes: Raven Toll: step 3 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -3; add wound 2; set flag `quest_chain_09_step_3_done`; notes: Raven Toll: step 3 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +24; xp +15; honor +1; set flag `quest_chain_09_step_3_done`; set flag `quest_chain_09_step_3_success`; triggers fight; notes: Raven Toll: step 3 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -5; add wound 2; set flag `quest_chain_09_step_3_done`; triggers fight; notes: Raven Toll: step 3 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 44. Raven Toll - Turning Point (`evt_chain_09_04`)
- Path: Quest chain 09, step 4 of 5
- Availability: depth 5..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_09_step_3_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +19; xp +17; set flag `quest_chain_09_step_4_done`; set flag `quest_chain_09_step_4_success`; notes: Raven Toll: step 4 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -4; add wound 3; set flag `quest_chain_09_step_4_done`; triggers fight; notes: Raven Toll: step 4 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +28; xp +18; honor +1; set flag `quest_chain_09_step_4_done`; set flag `quest_chain_09_step_4_success`; triggers fight; notes: Raven Toll: step 4 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -6; add wound 2; set flag `quest_chain_09_step_4_done`; triggers fight; notes: Raven Toll: step 4 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 45. Raven Toll - Final Reckoning (`evt_chain_09_05`)
- Path: Quest chain 09, step 5 of 5
- Availability: depth 6..999, tiers boss, elite, unique_once=true
- Requires flags: `quest_chain_09_step_4_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +22; xp +20; honor +1; item: sturdy rope; set flag `quest_chain_09_step_5_done`; set flag `quest_chain_09_step_5_success`; set flag `quest_chain_09_complete`; notes: Raven Toll: step 5 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -5; add wound 3; set flag `quest_chain_09_step_5_done`; set flag `quest_chain_09_complete`; triggers fight; notes: Raven Toll: step 5 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +32; xp +21; honor +1; set flag `quest_chain_09_step_5_done`; set flag `quest_chain_09_step_5_success`; set flag `quest_chain_09_complete`; triggers fight; notes: Raven Toll: step 5 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -7; add wound 2; set flag `quest_chain_09_step_5_done`; set flag `quest_chain_09_complete`; triggers fight; notes: Raven Toll: step 5 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 46. Glass Pilgrim - Rumor on the Road (`evt_chain_10_01`)
- Path: Quest chain 10, step 1 of 5
- Availability: depth 3..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +11; xp +8; set flag `quest_chain_10_step_1_done`; set flag `quest_chain_10_step_1_success`; notes: Glass Pilgrim: step 1 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -2; add wound 1; set flag `quest_chain_10_step_1_done`; notes: Glass Pilgrim: step 1 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +12; xp +9; set flag `quest_chain_10_step_1_done`; set flag `quest_chain_10_step_1_success`; notes: Glass Pilgrim: step 1 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -3; add wound 1; set flag `quest_chain_10_step_1_done`; triggers fight; notes: Glass Pilgrim: step 1 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 47. Glass Pilgrim - Hidden Mark (`evt_chain_10_02`)
- Path: Quest chain 10, step 2 of 5
- Availability: depth 4..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_10_step_1_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +14; xp +11; honor +1; set flag `quest_chain_10_step_2_done`; set flag `quest_chain_10_step_2_success`; notes: Glass Pilgrim: step 2 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -3; add wound 2; set flag `quest_chain_10_step_2_done`; notes: Glass Pilgrim: step 2 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + hard shift
Success: gold +16; xp +12; set flag `quest_chain_10_step_2_done`; set flag `quest_chain_10_step_2_success`; notes: Glass Pilgrim: step 2 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -4; add wound 1; set flag `quest_chain_10_step_2_done`; triggers fight; notes: Glass Pilgrim: step 2 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 48. Glass Pilgrim - Complication (`evt_chain_10_03`)
- Path: Quest chain 10, step 3 of 5
- Availability: depth 5..999, tiers normal, elite, boss, unique_once=true
- Requires flags: `quest_chain_10_step_2_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +17; xp +14; item: iron ration; set flag `quest_chain_10_step_3_done`; set flag `quest_chain_10_step_3_success`; notes: Glass Pilgrim: step 3 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -4; add wound 2; set flag `quest_chain_10_step_3_done`; notes: Glass Pilgrim: step 3 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +20; xp +15; honor +1; set flag `quest_chain_10_step_3_done`; set flag `quest_chain_10_step_3_success`; notes: Glass Pilgrim: step 3 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -5; add wound 2; set flag `quest_chain_10_step_3_done`; triggers fight; notes: Glass Pilgrim: step 3 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 49. Glass Pilgrim - Turning Point (`evt_chain_10_04`)
- Path: Quest chain 10, step 4 of 5
- Availability: depth 6..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_10_step_3_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +20; xp +17; set flag `quest_chain_10_step_4_done`; set flag `quest_chain_10_step_4_success`; notes: Glass Pilgrim: step 4 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -5; add wound 3; set flag `quest_chain_10_step_4_done`; triggers fight; notes: Glass Pilgrim: step 4 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +24; xp +18; honor +1; set flag `quest_chain_10_step_4_done`; set flag `quest_chain_10_step_4_success`; notes: Glass Pilgrim: step 4 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -6; add wound 2; set flag `quest_chain_10_step_4_done`; triggers fight; notes: Glass Pilgrim: step 4 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 50. Glass Pilgrim - Final Reckoning (`evt_chain_10_05`)
- Path: Quest chain 10, step 5 of 5
- Availability: depth 7..999, tiers boss, elite, unique_once=true
- Requires flags: `quest_chain_10_step_4_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +23; xp +20; honor +1; item: throwing knife; set flag `quest_chain_10_step_5_done`; set flag `quest_chain_10_step_5_success`; set flag `quest_chain_10_complete`; notes: Glass Pilgrim: step 5 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -6; add wound 3; set flag `quest_chain_10_step_5_done`; set flag `quest_chain_10_complete`; triggers fight; notes: Glass Pilgrim: step 5 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +28; xp +21; honor +1; add wound 1; set flag `quest_chain_10_step_5_done`; set flag `quest_chain_10_step_5_success`; set flag `quest_chain_10_complete`; notes: Glass Pilgrim: step 5 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -7; add wound 2; set flag `quest_chain_10_step_5_done`; set flag `quest_chain_10_complete`; triggers fight; notes: Glass Pilgrim: step 5 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 51. Pale Scriptorium - Rumor on the Road (`evt_chain_11_01`)
- Path: Quest chain 11, step 1 of 5
- Availability: depth 4..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +12; xp +8; set flag `quest_chain_11_step_1_done`; set flag `quest_chain_11_step_1_success`; notes: Pale Scriptorium: step 1 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -3; add wound 1; set flag `quest_chain_11_step_1_done`; notes: Pale Scriptorium: step 1 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +13; xp +9; set flag `quest_chain_11_step_1_done`; set flag `quest_chain_11_step_1_success`; notes: Pale Scriptorium: step 1 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -3; add wound 1; set flag `quest_chain_11_step_1_done`; triggers fight; notes: Pale Scriptorium: step 1 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 52. Pale Scriptorium - Hidden Mark (`evt_chain_11_02`)
- Path: Quest chain 11, step 2 of 5
- Availability: depth 5..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_11_step_1_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +15; xp +11; honor +1; set flag `quest_chain_11_step_2_done`; set flag `quest_chain_11_step_2_success`; notes: Pale Scriptorium: step 2 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -4; add wound 2; set flag `quest_chain_11_step_2_done`; notes: Pale Scriptorium: step 2 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +17; xp +12; set flag `quest_chain_11_step_2_done`; set flag `quest_chain_11_step_2_success`; notes: Pale Scriptorium: step 2 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -4; add wound 1; set flag `quest_chain_11_step_2_done`; triggers fight; notes: Pale Scriptorium: step 2 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 53. Pale Scriptorium - Complication (`evt_chain_11_03`)
- Path: Quest chain 11, step 3 of 5
- Availability: depth 6..999, tiers normal, elite, boss, unique_once=true
- Requires flags: `quest_chain_11_step_2_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + hard shift
Success: gold +18; xp +14; item: sturdy rope; set flag `quest_chain_11_step_3_done`; set flag `quest_chain_11_step_3_success`; notes: Pale Scriptorium: step 3 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -5; add wound 2; set flag `quest_chain_11_step_3_done`; notes: Pale Scriptorium: step 3 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +21; xp +15; honor +1; set flag `quest_chain_11_step_3_done`; set flag `quest_chain_11_step_3_success`; notes: Pale Scriptorium: step 3 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -5; add wound 2; set flag `quest_chain_11_step_3_done`; triggers fight; notes: Pale Scriptorium: step 3 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 54. Pale Scriptorium - Turning Point (`evt_chain_11_04`)
- Path: Quest chain 11, step 4 of 5
- Availability: depth 7..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_11_step_3_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +21; xp +17; set flag `quest_chain_11_step_4_done`; set flag `quest_chain_11_step_4_success`; notes: Pale Scriptorium: step 4 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -6; add wound 3; set flag `quest_chain_11_step_4_done`; triggers fight; notes: Pale Scriptorium: step 4 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +25; xp +18; honor +1; set flag `quest_chain_11_step_4_done`; set flag `quest_chain_11_step_4_success`; notes: Pale Scriptorium: step 4 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -6; add wound 2; set flag `quest_chain_11_step_4_done`; triggers fight; notes: Pale Scriptorium: step 4 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 55. Pale Scriptorium - Final Reckoning (`evt_chain_11_05`)
- Path: Quest chain 11, step 5 of 5
- Availability: depth 8..999, tiers boss, elite, unique_once=true
- Requires flags: `quest_chain_11_step_4_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +24; xp +20; honor +1; item: healing salve; set flag `quest_chain_11_step_5_done`; set flag `quest_chain_11_step_5_success`; set flag `quest_chain_11_complete`; notes: Pale Scriptorium: step 5 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -7; add wound 3; set flag `quest_chain_11_step_5_done`; set flag `quest_chain_11_complete`; triggers fight; notes: Pale Scriptorium: step 5 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +29; xp +21; honor +1; set flag `quest_chain_11_step_5_done`; set flag `quest_chain_11_step_5_success`; set flag `quest_chain_11_complete`; notes: Pale Scriptorium: step 5 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -7; add wound 2; set flag `quest_chain_11_step_5_done`; set flag `quest_chain_11_complete`; triggers fight; notes: Pale Scriptorium: step 5 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 56. Warden's Debt - Rumor on the Road (`evt_chain_12_01`)
- Path: Quest chain 12, step 1 of 5
- Availability: depth 5..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +9; xp +8; set flag `quest_chain_12_step_1_done`; set flag `quest_chain_12_step_1_success`; notes: Warden's Debt: step 1 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -1; add wound 1; set flag `quest_chain_12_step_1_done`; notes: Warden's Debt: step 1 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +14; xp +9; set flag `quest_chain_12_step_1_done`; set flag `quest_chain_12_step_1_success`; notes: Warden's Debt: step 1 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -3; add wound 1; set flag `quest_chain_12_step_1_done`; triggers fight; notes: Warden's Debt: step 1 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 57. Warden's Debt - Hidden Mark (`evt_chain_12_02`)
- Path: Quest chain 12, step 2 of 5
- Availability: depth 6..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_12_step_1_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +12; xp +11; honor +1; set flag `quest_chain_12_step_2_done`; set flag `quest_chain_12_step_2_success`; notes: Warden's Debt: step 2 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -2; add wound 2; set flag `quest_chain_12_step_2_done`; notes: Warden's Debt: step 2 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +18; xp +12; set flag `quest_chain_12_step_2_done`; set flag `quest_chain_12_step_2_success`; notes: Warden's Debt: step 2 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -4; add wound 1; set flag `quest_chain_12_step_2_done`; triggers fight; notes: Warden's Debt: step 2 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 58. Warden's Debt - Complication (`evt_chain_12_03`)
- Path: Quest chain 12, step 3 of 5
- Availability: depth 7..999, tiers normal, elite, boss, unique_once=true
- Requires flags: `quest_chain_12_step_2_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +15; xp +14; item: throwing knife; set flag `quest_chain_12_step_3_done`; set flag `quest_chain_12_step_3_success`; notes: Warden's Debt: step 3 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -3; add wound 2; set flag `quest_chain_12_step_3_done`; notes: Warden's Debt: step 3 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +22; xp +15; honor +1; set flag `quest_chain_12_step_3_done`; set flag `quest_chain_12_step_3_success`; triggers fight; notes: Warden's Debt: step 3 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -5; add wound 2; set flag `quest_chain_12_step_3_done`; triggers fight; notes: Warden's Debt: step 3 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 59. Warden's Debt - Turning Point (`evt_chain_12_04`)
- Path: Quest chain 12, step 4 of 5
- Availability: depth 8..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_12_step_3_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +18; xp +17; set flag `quest_chain_12_step_4_done`; set flag `quest_chain_12_step_4_success`; notes: Warden's Debt: step 4 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -4; add wound 3; set flag `quest_chain_12_step_4_done`; triggers fight; notes: Warden's Debt: step 4 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +26; xp +18; honor +1; set flag `quest_chain_12_step_4_done`; set flag `quest_chain_12_step_4_success`; triggers fight; notes: Warden's Debt: step 4 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -6; add wound 2; set flag `quest_chain_12_step_4_done`; triggers fight; notes: Warden's Debt: step 4 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 60. Warden's Debt - Final Reckoning (`evt_chain_12_05`)
- Path: Quest chain 12, step 5 of 5
- Availability: depth 9..999, tiers boss, elite, unique_once=true
- Requires flags: `quest_chain_12_step_4_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +21; xp +20; honor +1; item: lantern oil; set flag `quest_chain_12_step_5_done`; set flag `quest_chain_12_step_5_success`; set flag `quest_chain_12_complete`; notes: Warden's Debt: step 5 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -5; add wound 3; set flag `quest_chain_12_step_5_done`; set flag `quest_chain_12_complete`; triggers fight; notes: Warden's Debt: step 5 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +30; xp +21; honor +1; add wound 1; set flag `quest_chain_12_step_5_done`; set flag `quest_chain_12_step_5_success`; set flag `quest_chain_12_complete`; triggers fight; notes: Warden's Debt: step 5 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -7; add wound 2; set flag `quest_chain_12_step_5_done`; set flag `quest_chain_12_complete`; triggers fight; notes: Warden's Debt: step 5 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 61. Black Orchard - Rumor on the Road (`evt_chain_13_01`)
- Path: Quest chain 13, step 1 of 5
- Availability: depth 6..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +10; xp +8; set flag `quest_chain_13_step_1_done`; set flag `quest_chain_13_step_1_success`; notes: Black Orchard: step 1 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -2; add wound 1; set flag `quest_chain_13_step_1_done`; notes: Black Orchard: step 1 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +15; xp +9; set flag `quest_chain_13_step_1_done`; set flag `quest_chain_13_step_1_success`; notes: Black Orchard: step 1 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -3; add wound 1; set flag `quest_chain_13_step_1_done`; triggers fight; notes: Black Orchard: step 1 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 62. Black Orchard - Hidden Mark (`evt_chain_13_02`)
- Path: Quest chain 13, step 2 of 5
- Availability: depth 7..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_13_step_1_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +13; xp +11; honor +1; set flag `quest_chain_13_step_2_done`; set flag `quest_chain_13_step_2_success`; notes: Black Orchard: step 2 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -3; add wound 2; set flag `quest_chain_13_step_2_done`; notes: Black Orchard: step 2 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + hard shift
Success: gold +19; xp +12; set flag `quest_chain_13_step_2_done`; set flag `quest_chain_13_step_2_success`; notes: Black Orchard: step 2 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -4; add wound 1; set flag `quest_chain_13_step_2_done`; triggers fight; notes: Black Orchard: step 2 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 63. Black Orchard - Complication (`evt_chain_13_03`)
- Path: Quest chain 13, step 3 of 5
- Availability: depth 8..999, tiers normal, elite, boss, unique_once=true
- Requires flags: `quest_chain_13_step_2_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +16; xp +14; item: healing salve; set flag `quest_chain_13_step_3_done`; set flag `quest_chain_13_step_3_success`; notes: Black Orchard: step 3 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -4; add wound 2; set flag `quest_chain_13_step_3_done`; notes: Black Orchard: step 3 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +23; xp +15; honor +1; set flag `quest_chain_13_step_3_done`; set flag `quest_chain_13_step_3_success`; notes: Black Orchard: step 3 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -5; add wound 2; set flag `quest_chain_13_step_3_done`; triggers fight; notes: Black Orchard: step 3 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 64. Black Orchard - Turning Point (`evt_chain_13_04`)
- Path: Quest chain 13, step 4 of 5
- Availability: depth 9..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_13_step_3_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +19; xp +17; set flag `quest_chain_13_step_4_done`; set flag `quest_chain_13_step_4_success`; notes: Black Orchard: step 4 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -5; add wound 3; set flag `quest_chain_13_step_4_done`; triggers fight; notes: Black Orchard: step 4 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +27; xp +18; honor +1; set flag `quest_chain_13_step_4_done`; set flag `quest_chain_13_step_4_success`; notes: Black Orchard: step 4 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -6; add wound 2; set flag `quest_chain_13_step_4_done`; triggers fight; notes: Black Orchard: step 4 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 65. Black Orchard - Final Reckoning (`evt_chain_13_05`)
- Path: Quest chain 13, step 5 of 5
- Availability: depth 10..999, tiers boss, elite, unique_once=true
- Requires flags: `quest_chain_13_step_4_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +22; xp +20; honor +1; item: lockpick set; set flag `quest_chain_13_step_5_done`; set flag `quest_chain_13_step_5_success`; set flag `quest_chain_13_complete`; notes: Black Orchard: step 5 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -6; add wound 3; set flag `quest_chain_13_step_5_done`; set flag `quest_chain_13_complete`; triggers fight; notes: Black Orchard: step 5 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +31; xp +21; honor +1; set flag `quest_chain_13_step_5_done`; set flag `quest_chain_13_step_5_success`; set flag `quest_chain_13_complete`; notes: Black Orchard: step 5 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -7; add wound 2; set flag `quest_chain_13_step_5_done`; set flag `quest_chain_13_complete`; triggers fight; notes: Black Orchard: step 5 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 66. Thorn Tribunal - Rumor on the Road (`evt_chain_14_01`)
- Path: Quest chain 14, step 1 of 5
- Availability: depth 7..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +11; xp +8; set flag `quest_chain_14_step_1_done`; set flag `quest_chain_14_step_1_success`; notes: Thorn Tribunal: step 1 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -3; add wound 1; set flag `quest_chain_14_step_1_done`; notes: Thorn Tribunal: step 1 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +16; xp +9; set flag `quest_chain_14_step_1_done`; set flag `quest_chain_14_step_1_success`; notes: Thorn Tribunal: step 1 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -3; add wound 1; set flag `quest_chain_14_step_1_done`; triggers fight; notes: Thorn Tribunal: step 1 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 67. Thorn Tribunal - Hidden Mark (`evt_chain_14_02`)
- Path: Quest chain 14, step 2 of 5
- Availability: depth 8..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_14_step_1_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +14; xp +11; honor +1; set flag `quest_chain_14_step_2_done`; set flag `quest_chain_14_step_2_success`; notes: Thorn Tribunal: step 2 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -4; add wound 2; set flag `quest_chain_14_step_2_done`; notes: Thorn Tribunal: step 2 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +20; xp +12; set flag `quest_chain_14_step_2_done`; set flag `quest_chain_14_step_2_success`; notes: Thorn Tribunal: step 2 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -4; add wound 1; set flag `quest_chain_14_step_2_done`; triggers fight; notes: Thorn Tribunal: step 2 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 68. Thorn Tribunal - Complication (`evt_chain_14_03`)
- Path: Quest chain 14, step 3 of 5
- Availability: depth 9..999, tiers normal, elite, boss, unique_once=true
- Requires flags: `quest_chain_14_step_2_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + hard shift
Success: gold +17; xp +14; item: lantern oil; set flag `quest_chain_14_step_3_done`; set flag `quest_chain_14_step_3_success`; notes: Thorn Tribunal: step 3 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -5; add wound 2; set flag `quest_chain_14_step_3_done`; notes: Thorn Tribunal: step 3 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +24; xp +15; honor +1; set flag `quest_chain_14_step_3_done`; set flag `quest_chain_14_step_3_success`; notes: Thorn Tribunal: step 3 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -5; add wound 2; set flag `quest_chain_14_step_3_done`; triggers fight; notes: Thorn Tribunal: step 3 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 69. Thorn Tribunal - Turning Point (`evt_chain_14_04`)
- Path: Quest chain 14, step 4 of 5
- Availability: depth 10..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_14_step_3_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +20; xp +17; set flag `quest_chain_14_step_4_done`; set flag `quest_chain_14_step_4_success`; notes: Thorn Tribunal: step 4 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -6; add wound 3; set flag `quest_chain_14_step_4_done`; triggers fight; notes: Thorn Tribunal: step 4 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +28; xp +18; honor +1; set flag `quest_chain_14_step_4_done`; set flag `quest_chain_14_step_4_success`; notes: Thorn Tribunal: step 4 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -6; add wound 2; set flag `quest_chain_14_step_4_done`; triggers fight; notes: Thorn Tribunal: step 4 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 70. Thorn Tribunal - Final Reckoning (`evt_chain_14_05`)
- Path: Quest chain 14, step 5 of 5
- Availability: depth 11..999, tiers boss, elite, unique_once=true
- Requires flags: `quest_chain_14_step_4_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +23; xp +20; honor +1; item: map scrap; set flag `quest_chain_14_step_5_done`; set flag `quest_chain_14_step_5_success`; set flag `quest_chain_14_complete`; notes: Thorn Tribunal: step 5 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -7; add wound 3; set flag `quest_chain_14_step_5_done`; set flag `quest_chain_14_complete`; triggers fight; notes: Thorn Tribunal: step 5 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +32; xp +21; honor +1; add wound 1; set flag `quest_chain_14_step_5_done`; set flag `quest_chain_14_step_5_success`; set flag `quest_chain_14_complete`; notes: Thorn Tribunal: step 5 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -7; add wound 2; set flag `quest_chain_14_step_5_done`; set flag `quest_chain_14_complete`; triggers fight; notes: Thorn Tribunal: step 5 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 71. Frost Reliquary - Rumor on the Road (`evt_chain_15_01`)
- Path: Quest chain 15, step 1 of 5
- Availability: depth 1..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +12; xp +8; set flag `quest_chain_15_step_1_done`; set flag `quest_chain_15_step_1_success`; notes: Frost Reliquary: step 1 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -1; add wound 1; set flag `quest_chain_15_step_1_done`; notes: Frost Reliquary: step 1 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +12; xp +9; set flag `quest_chain_15_step_1_done`; set flag `quest_chain_15_step_1_success`; notes: Frost Reliquary: step 1 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -3; add wound 1; set flag `quest_chain_15_step_1_done`; triggers fight; notes: Frost Reliquary: step 1 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 72. Frost Reliquary - Hidden Mark (`evt_chain_15_02`)
- Path: Quest chain 15, step 2 of 5
- Availability: depth 2..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_15_step_1_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +15; xp +11; honor +1; set flag `quest_chain_15_step_2_done`; set flag `quest_chain_15_step_2_success`; notes: Frost Reliquary: step 2 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -2; add wound 2; set flag `quest_chain_15_step_2_done`; notes: Frost Reliquary: step 2 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +16; xp +12; set flag `quest_chain_15_step_2_done`; set flag `quest_chain_15_step_2_success`; notes: Frost Reliquary: step 2 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -4; add wound 1; set flag `quest_chain_15_step_2_done`; triggers fight; notes: Frost Reliquary: step 2 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 73. Frost Reliquary - Complication (`evt_chain_15_03`)
- Path: Quest chain 15, step 3 of 5
- Availability: depth 3..999, tiers normal, elite, boss, unique_once=true
- Requires flags: `quest_chain_15_step_2_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +18; xp +14; item: lockpick set; set flag `quest_chain_15_step_3_done`; set flag `quest_chain_15_step_3_success`; notes: Frost Reliquary: step 3 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -3; add wound 2; set flag `quest_chain_15_step_3_done`; notes: Frost Reliquary: step 3 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +20; xp +15; honor +1; set flag `quest_chain_15_step_3_done`; set flag `quest_chain_15_step_3_success`; triggers fight; notes: Frost Reliquary: step 3 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -5; add wound 2; set flag `quest_chain_15_step_3_done`; triggers fight; notes: Frost Reliquary: step 3 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 74. Frost Reliquary - Turning Point (`evt_chain_15_04`)
- Path: Quest chain 15, step 4 of 5
- Availability: depth 4..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_15_step_3_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +21; xp +17; set flag `quest_chain_15_step_4_done`; set flag `quest_chain_15_step_4_success`; notes: Frost Reliquary: step 4 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -4; add wound 3; set flag `quest_chain_15_step_4_done`; triggers fight; notes: Frost Reliquary: step 4 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +24; xp +18; honor +1; set flag `quest_chain_15_step_4_done`; set flag `quest_chain_15_step_4_success`; triggers fight; notes: Frost Reliquary: step 4 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -6; add wound 2; set flag `quest_chain_15_step_4_done`; triggers fight; notes: Frost Reliquary: step 4 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 75. Frost Reliquary - Final Reckoning (`evt_chain_15_05`)
- Path: Quest chain 15, step 5 of 5
- Availability: depth 5..999, tiers boss, elite, unique_once=true
- Requires flags: `quest_chain_15_step_4_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +24; xp +20; honor +1; item: bone charm; set flag `quest_chain_15_step_5_done`; set flag `quest_chain_15_step_5_success`; set flag `quest_chain_15_complete`; notes: Frost Reliquary: step 5 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -5; add wound 3; set flag `quest_chain_15_step_5_done`; set flag `quest_chain_15_complete`; triggers fight; notes: Frost Reliquary: step 5 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +28; xp +21; honor +1; set flag `quest_chain_15_step_5_done`; set flag `quest_chain_15_step_5_success`; set flag `quest_chain_15_complete`; triggers fight; notes: Frost Reliquary: step 5 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -7; add wound 2; set flag `quest_chain_15_step_5_done`; set flag `quest_chain_15_complete`; triggers fight; notes: Frost Reliquary: step 5 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 76. Brass Covenant - Rumor on the Road (`evt_chain_16_01`)
- Path: Quest chain 16, step 1 of 5
- Availability: depth 2..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +9; xp +8; set flag `quest_chain_16_step_1_done`; set flag `quest_chain_16_step_1_success`; notes: Brass Covenant: step 1 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -2; add wound 1; set flag `quest_chain_16_step_1_done`; notes: Brass Covenant: step 1 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +13; xp +9; set flag `quest_chain_16_step_1_done`; set flag `quest_chain_16_step_1_success`; notes: Brass Covenant: step 1 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -3; add wound 1; set flag `quest_chain_16_step_1_done`; triggers fight; notes: Brass Covenant: step 1 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 77. Brass Covenant - Hidden Mark (`evt_chain_16_02`)
- Path: Quest chain 16, step 2 of 5
- Availability: depth 3..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_16_step_1_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +12; xp +11; honor +1; set flag `quest_chain_16_step_2_done`; set flag `quest_chain_16_step_2_success`; notes: Brass Covenant: step 2 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -3; add wound 2; set flag `quest_chain_16_step_2_done`; notes: Brass Covenant: step 2 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + hard shift
Success: gold +17; xp +12; set flag `quest_chain_16_step_2_done`; set flag `quest_chain_16_step_2_success`; notes: Brass Covenant: step 2 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -4; add wound 1; set flag `quest_chain_16_step_2_done`; triggers fight; notes: Brass Covenant: step 2 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 78. Brass Covenant - Complication (`evt_chain_16_03`)
- Path: Quest chain 16, step 3 of 5
- Availability: depth 4..999, tiers normal, elite, boss, unique_once=true
- Requires flags: `quest_chain_16_step_2_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +15; xp +14; item: map scrap; set flag `quest_chain_16_step_3_done`; set flag `quest_chain_16_step_3_success`; notes: Brass Covenant: step 3 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -4; add wound 2; set flag `quest_chain_16_step_3_done`; notes: Brass Covenant: step 3 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +21; xp +15; honor +1; set flag `quest_chain_16_step_3_done`; set flag `quest_chain_16_step_3_success`; notes: Brass Covenant: step 3 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -5; add wound 2; set flag `quest_chain_16_step_3_done`; triggers fight; notes: Brass Covenant: step 3 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 79. Brass Covenant - Turning Point (`evt_chain_16_04`)
- Path: Quest chain 16, step 4 of 5
- Availability: depth 5..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_16_step_3_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +18; xp +17; set flag `quest_chain_16_step_4_done`; set flag `quest_chain_16_step_4_success`; notes: Brass Covenant: step 4 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -5; add wound 3; set flag `quest_chain_16_step_4_done`; triggers fight; notes: Brass Covenant: step 4 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +25; xp +18; honor +1; set flag `quest_chain_16_step_4_done`; set flag `quest_chain_16_step_4_success`; notes: Brass Covenant: step 4 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -6; add wound 2; set flag `quest_chain_16_step_4_done`; triggers fight; notes: Brass Covenant: step 4 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 80. Brass Covenant - Final Reckoning (`evt_chain_16_05`)
- Path: Quest chain 16, step 5 of 5
- Availability: depth 6..999, tiers boss, elite, unique_once=true
- Requires flags: `quest_chain_16_step_4_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +21; xp +20; honor +1; item: smithing nails; set flag `quest_chain_16_step_5_done`; set flag `quest_chain_16_step_5_success`; set flag `quest_chain_16_complete`; notes: Brass Covenant: step 5 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -6; add wound 3; set flag `quest_chain_16_step_5_done`; set flag `quest_chain_16_complete`; triggers fight; notes: Brass Covenant: step 5 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +29; xp +21; honor +1; add wound 1; set flag `quest_chain_16_step_5_done`; set flag `quest_chain_16_step_5_success`; set flag `quest_chain_16_complete`; notes: Brass Covenant: step 5 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -7; add wound 2; set flag `quest_chain_16_step_5_done`; set flag `quest_chain_16_complete`; triggers fight; notes: Brass Covenant: step 5 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 81. Shale Compass - Rumor on the Road (`evt_chain_17_01`)
- Path: Quest chain 17, step 1 of 5
- Availability: depth 3..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +10; xp +8; set flag `quest_chain_17_step_1_done`; set flag `quest_chain_17_step_1_success`; notes: Shale Compass: step 1 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -3; add wound 1; set flag `quest_chain_17_step_1_done`; notes: Shale Compass: step 1 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +14; xp +9; set flag `quest_chain_17_step_1_done`; set flag `quest_chain_17_step_1_success`; notes: Shale Compass: step 1 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -3; add wound 1; set flag `quest_chain_17_step_1_done`; triggers fight; notes: Shale Compass: step 1 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 82. Shale Compass - Hidden Mark (`evt_chain_17_02`)
- Path: Quest chain 17, step 2 of 5
- Availability: depth 4..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_17_step_1_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +13; xp +11; honor +1; set flag `quest_chain_17_step_2_done`; set flag `quest_chain_17_step_2_success`; notes: Shale Compass: step 2 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -4; add wound 2; set flag `quest_chain_17_step_2_done`; notes: Shale Compass: step 2 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +18; xp +12; set flag `quest_chain_17_step_2_done`; set flag `quest_chain_17_step_2_success`; notes: Shale Compass: step 2 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -4; add wound 1; set flag `quest_chain_17_step_2_done`; triggers fight; notes: Shale Compass: step 2 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 83. Shale Compass - Complication (`evt_chain_17_03`)
- Path: Quest chain 17, step 3 of 5
- Availability: depth 5..999, tiers normal, elite, boss, unique_once=true
- Requires flags: `quest_chain_17_step_2_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (INTELLIGENCE) + hard shift
Success: gold +16; xp +14; item: bone charm; set flag `quest_chain_17_step_3_done`; set flag `quest_chain_17_step_3_success`; notes: Shale Compass: step 3 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -5; add wound 2; set flag `quest_chain_17_step_3_done`; notes: Shale Compass: step 3 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +22; xp +15; honor +1; set flag `quest_chain_17_step_3_done`; set flag `quest_chain_17_step_3_success`; notes: Shale Compass: step 3 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -5; add wound 2; set flag `quest_chain_17_step_3_done`; triggers fight; notes: Shale Compass: step 3 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 84. Shale Compass - Turning Point (`evt_chain_17_04`)
- Path: Quest chain 17, step 4 of 5
- Availability: depth 6..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_17_step_3_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +19; xp +17; set flag `quest_chain_17_step_4_done`; set flag `quest_chain_17_step_4_success`; notes: Shale Compass: step 4 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -6; add wound 3; set flag `quest_chain_17_step_4_done`; triggers fight; notes: Shale Compass: step 4 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +26; xp +18; honor +1; set flag `quest_chain_17_step_4_done`; set flag `quest_chain_17_step_4_success`; notes: Shale Compass: step 4 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -6; add wound 2; set flag `quest_chain_17_step_4_done`; triggers fight; notes: Shale Compass: step 4 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 85. Shale Compass - Final Reckoning (`evt_chain_17_05`)
- Path: Quest chain 17, step 5 of 5
- Availability: depth 7..999, tiers boss, elite, unique_once=true
- Requires flags: `quest_chain_17_step_4_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +22; xp +20; honor +1; item: signal whistle; set flag `quest_chain_17_step_5_done`; set flag `quest_chain_17_step_5_success`; set flag `quest_chain_17_complete`; notes: Shale Compass: step 5 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -7; add wound 3; set flag `quest_chain_17_step_5_done`; set flag `quest_chain_17_complete`; triggers fight; notes: Shale Compass: step 5 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +30; xp +21; honor +1; set flag `quest_chain_17_step_5_done`; set flag `quest_chain_17_step_5_success`; set flag `quest_chain_17_complete`; notes: Shale Compass: step 5 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -7; add wound 2; set flag `quest_chain_17_step_5_done`; set flag `quest_chain_17_complete`; triggers fight; notes: Shale Compass: step 5 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 86. Mire Lantern - Rumor on the Road (`evt_chain_18_01`)
- Path: Quest chain 18, step 1 of 5
- Availability: depth 4..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +11; xp +8; set flag `quest_chain_18_step_1_done`; set flag `quest_chain_18_step_1_success`; notes: Mire Lantern: step 1 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -1; add wound 1; set flag `quest_chain_18_step_1_done`; notes: Mire Lantern: step 1 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +15; xp +9; set flag `quest_chain_18_step_1_done`; set flag `quest_chain_18_step_1_success`; notes: Mire Lantern: step 1 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -3; add wound 1; set flag `quest_chain_18_step_1_done`; triggers fight; notes: Mire Lantern: step 1 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 87. Mire Lantern - Hidden Mark (`evt_chain_18_02`)
- Path: Quest chain 18, step 2 of 5
- Availability: depth 5..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_18_step_1_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +14; xp +11; honor +1; set flag `quest_chain_18_step_2_done`; set flag `quest_chain_18_step_2_success`; notes: Mire Lantern: step 2 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -2; add wound 2; set flag `quest_chain_18_step_2_done`; notes: Mire Lantern: step 2 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +19; xp +12; set flag `quest_chain_18_step_2_done`; set flag `quest_chain_18_step_2_success`; notes: Mire Lantern: step 2 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -4; add wound 1; set flag `quest_chain_18_step_2_done`; triggers fight; notes: Mire Lantern: step 2 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 88. Mire Lantern - Complication (`evt_chain_18_03`)
- Path: Quest chain 18, step 3 of 5
- Availability: depth 6..999, tiers normal, elite, boss, unique_once=true
- Requires flags: `quest_chain_18_step_2_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +17; xp +14; item: smithing nails; set flag `quest_chain_18_step_3_done`; set flag `quest_chain_18_step_3_success`; notes: Mire Lantern: step 3 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -3; add wound 2; set flag `quest_chain_18_step_3_done`; notes: Mire Lantern: step 3 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +23; xp +15; honor +1; set flag `quest_chain_18_step_3_done`; set flag `quest_chain_18_step_3_success`; triggers fight; notes: Mire Lantern: step 3 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -5; add wound 2; set flag `quest_chain_18_step_3_done`; triggers fight; notes: Mire Lantern: step 3 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 89. Mire Lantern - Turning Point (`evt_chain_18_04`)
- Path: Quest chain 18, step 4 of 5
- Availability: depth 7..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_18_step_3_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +20; xp +17; set flag `quest_chain_18_step_4_done`; set flag `quest_chain_18_step_4_success`; notes: Mire Lantern: step 4 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -4; add wound 3; set flag `quest_chain_18_step_4_done`; triggers fight; notes: Mire Lantern: step 4 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +27; xp +18; honor +1; set flag `quest_chain_18_step_4_done`; set flag `quest_chain_18_step_4_success`; triggers fight; notes: Mire Lantern: step 4 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -6; add wound 2; set flag `quest_chain_18_step_4_done`; triggers fight; notes: Mire Lantern: step 4 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 90. Mire Lantern - Final Reckoning (`evt_chain_18_05`)
- Path: Quest chain 18, step 5 of 5
- Availability: depth 8..999, tiers boss, elite, unique_once=true
- Requires flags: `quest_chain_18_step_4_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +23; xp +20; honor +1; item: travel cloak; set flag `quest_chain_18_step_5_done`; set flag `quest_chain_18_step_5_success`; set flag `quest_chain_18_complete`; notes: Mire Lantern: step 5 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -5; add wound 3; set flag `quest_chain_18_step_5_done`; set flag `quest_chain_18_complete`; triggers fight; notes: Mire Lantern: step 5 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= stat level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +31; xp +21; honor +1; add wound 1; set flag `quest_chain_18_step_5_done`; set flag `quest_chain_18_step_5_success`; set flag `quest_chain_18_complete`; triggers fight; notes: Mire Lantern: step 5 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -7; add wound 2; set flag `quest_chain_18_step_5_done`; set flag `quest_chain_18_complete`; triggers fight; notes: Mire Lantern: step 5 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 91. Sable Tribunal - Rumor on the Road (`evt_chain_19_01`)
- Path: Quest chain 19, step 1 of 5
- Availability: depth 5..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +12; xp +8; set flag `quest_chain_19_step_1_done`; set flag `quest_chain_19_step_1_success`; notes: Sable Tribunal: step 1 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -2; add wound 1; set flag `quest_chain_19_step_1_done`; notes: Sable Tribunal: step 1 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +16; xp +9; set flag `quest_chain_19_step_1_done`; set flag `quest_chain_19_step_1_success`; notes: Sable Tribunal: step 1 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -3; add wound 1; set flag `quest_chain_19_step_1_done`; triggers fight; notes: Sable Tribunal: step 1 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 92. Sable Tribunal - Hidden Mark (`evt_chain_19_02`)
- Path: Quest chain 19, step 2 of 5
- Availability: depth 6..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_19_step_1_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +15; xp +11; honor +1; set flag `quest_chain_19_step_2_done`; set flag `quest_chain_19_step_2_success`; notes: Sable Tribunal: step 2 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -3; add wound 2; set flag `quest_chain_19_step_2_done`; notes: Sable Tribunal: step 2 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + hard shift
Success: gold +20; xp +12; set flag `quest_chain_19_step_2_done`; set flag `quest_chain_19_step_2_success`; notes: Sable Tribunal: step 2 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -4; add wound 1; set flag `quest_chain_19_step_2_done`; triggers fight; notes: Sable Tribunal: step 2 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 93. Sable Tribunal - Complication (`evt_chain_19_03`)
- Path: Quest chain 19, step 3 of 5
- Availability: depth 7..999, tiers normal, elite, boss, unique_once=true
- Requires flags: `quest_chain_19_step_2_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +18; xp +14; item: signal whistle; set flag `quest_chain_19_step_3_done`; set flag `quest_chain_19_step_3_success`; notes: Sable Tribunal: step 3 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -4; add wound 2; set flag `quest_chain_19_step_3_done`; notes: Sable Tribunal: step 3 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +24; xp +15; honor +1; set flag `quest_chain_19_step_3_done`; set flag `quest_chain_19_step_3_success`; notes: Sable Tribunal: step 3 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -5; add wound 2; set flag `quest_chain_19_step_3_done`; triggers fight; notes: Sable Tribunal: step 3 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 94. Sable Tribunal - Turning Point (`evt_chain_19_04`)
- Path: Quest chain 19, step 4 of 5
- Availability: depth 8..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_19_step_3_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +21; xp +17; set flag `quest_chain_19_step_4_done`; set flag `quest_chain_19_step_4_success`; notes: Sable Tribunal: step 4 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -5; add wound 3; set flag `quest_chain_19_step_4_done`; triggers fight; notes: Sable Tribunal: step 4 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +28; xp +18; honor +1; set flag `quest_chain_19_step_4_done`; set flag `quest_chain_19_step_4_success`; notes: Sable Tribunal: step 4 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -6; add wound 2; set flag `quest_chain_19_step_4_done`; triggers fight; notes: Sable Tribunal: step 4 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 95. Sable Tribunal - Final Reckoning (`evt_chain_19_05`)
- Path: Quest chain 19, step 5 of 5
- Availability: depth 9..999, tiers boss, elite, unique_once=true
- Requires flags: `quest_chain_19_step_4_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +24; xp +20; honor +1; item: bandage roll; set flag `quest_chain_19_step_5_done`; set flag `quest_chain_19_step_5_success`; set flag `quest_chain_19_complete`; notes: Sable Tribunal: step 5 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -6; add wound 3; set flag `quest_chain_19_step_5_done`; set flag `quest_chain_19_complete`; triggers fight; notes: Sable Tribunal: step 5 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +32; xp +21; honor +1; set flag `quest_chain_19_step_5_done`; set flag `quest_chain_19_step_5_success`; set flag `quest_chain_19_complete`; notes: Sable Tribunal: step 5 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -7; add wound 2; set flag `quest_chain_19_step_5_done`; set flag `quest_chain_19_complete`; triggers fight; notes: Sable Tribunal: step 5 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 96. Starless Mint - Rumor on the Road (`evt_chain_20_01`)
- Path: Quest chain 20, step 1 of 5
- Availability: depth 6..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +9; xp +8; set flag `quest_chain_20_step_1_done`; set flag `quest_chain_20_step_1_success`; notes: Starless Mint: step 1 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -3; add wound 1; set flag `quest_chain_20_step_1_done`; notes: Starless Mint: step 1 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +12; xp +9; set flag `quest_chain_20_step_1_done`; set flag `quest_chain_20_step_1_success`; notes: Starless Mint: step 1 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -3; add wound 1; set flag `quest_chain_20_step_1_done`; triggers fight; notes: Starless Mint: step 1 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 97. Starless Mint - Hidden Mark (`evt_chain_20_02`)
- Path: Quest chain 20, step 2 of 5
- Availability: depth 7..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_20_step_1_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +12; xp +11; honor +1; set flag `quest_chain_20_step_2_done`; set flag `quest_chain_20_step_2_success`; notes: Starless Mint: step 2 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -4; add wound 2; set flag `quest_chain_20_step_2_done`; notes: Starless Mint: step 2 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +16; xp +12; set flag `quest_chain_20_step_2_done`; set flag `quest_chain_20_step_2_success`; notes: Starless Mint: step 2 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -4; add wound 1; set flag `quest_chain_20_step_2_done`; triggers fight; notes: Starless Mint: step 2 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 98. Starless Mint - Complication (`evt_chain_20_03`)
- Path: Quest chain 20, step 3 of 5
- Availability: depth 8..999, tiers normal, elite, boss, unique_once=true
- Requires flags: `quest_chain_20_step_2_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + hard shift
Success: gold +15; xp +14; item: travel cloak; set flag `quest_chain_20_step_3_done`; set flag `quest_chain_20_step_3_success`; notes: Starless Mint: step 3 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -5; add wound 2; set flag `quest_chain_20_step_3_done`; notes: Starless Mint: step 3 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +20; xp +15; honor +1; set flag `quest_chain_20_step_3_done`; set flag `quest_chain_20_step_3_success`; notes: Starless Mint: step 3 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -5; add wound 2; set flag `quest_chain_20_step_3_done`; triggers fight; notes: Starless Mint: step 3 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 99. Starless Mint - Turning Point (`evt_chain_20_04`)
- Path: Quest chain 20, step 4 of 5
- Availability: depth 9..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_20_step_3_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +18; xp +17; set flag `quest_chain_20_step_4_done`; set flag `quest_chain_20_step_4_success`; notes: Starless Mint: step 4 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -6; add wound 3; set flag `quest_chain_20_step_4_done`; triggers fight; notes: Starless Mint: step 4 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +24; xp +18; honor +1; set flag `quest_chain_20_step_4_done`; set flag `quest_chain_20_step_4_success`; notes: Starless Mint: step 4 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -6; add wound 2; set flag `quest_chain_20_step_4_done`; triggers fight; notes: Starless Mint: step 4 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 100. Starless Mint - Final Reckoning (`evt_chain_20_05`)
- Path: Quest chain 20, step 5 of 5
- Availability: depth 10..999, tiers boss, elite, unique_once=true
- Requires flags: `quest_chain_20_step_4_done`
- Choices:
1. Scout and negotiate the situation
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +21; xp +20; honor +1; item: iron ration; set flag `quest_chain_20_step_5_done`; set flag `quest_chain_20_step_5_success`; set flag `quest_chain_20_complete`; notes: Starless Mint: step 5 advances through careful planning. | Your measured approach secures leverage for the next lead.
Failure: gold -7; add wound 3; set flag `quest_chain_20_step_5_done`; set flag `quest_chain_20_complete`; triggers fight; notes: Starless Mint: step 5 slips, but the trail stays alive. | You lose momentum and leave blood on the ground.
1. Press hard for an immediate result
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +28; xp +21; honor +1; add wound 1; set flag `quest_chain_20_step_5_done`; set flag `quest_chain_20_step_5_success`; set flag `quest_chain_20_complete`; notes: Starless Mint: step 5 is won by force. | You take the direct route and claim immediate gains.
Failure: gold -7; add wound 2; set flag `quest_chain_20_step_5_done`; set flag `quest_chain_20_complete`; triggers fight; notes: Starless Mint: step 5 backfires and turns hostile. | Your push creates enemies that answer with steel.

## 101. Wayfarer Caravan at Dusk [001] (`evt_world_001`)
- Path: Follow-up event gated by: `quest_chain_01_complete`
- Availability: depth 0..999, tiers any, unique_once=true
- Requires flags: `quest_chain_01_complete`
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + easy shift
Success: gold +5; xp +4; set flag `world_event_001_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; set flag `world_event_001_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +7; xp +5; set flag `world_event_001_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -3; add wound 2; set flag `world_event_001_resolved`; notes: The aggressive move collapses into losses and open danger.

## 102. Dustbound Cache Behind the Gate [002] (`evt_world_002`)
- Path: Follow-up event gated by: `quest_chain_02_complete`
- Availability: depth 1..999, tiers any, unique_once=true
- Requires flags: `quest_chain_02_complete`
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +6; xp +5; set flag `world_event_002_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; set flag `world_event_002_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +8; xp +6; set flag `world_event_002_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -4; add wound 3; set flag `world_event_002_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 103. Lantern Furnace in the Thicket [003] (`evt_world_003`)
- Path: Follow-up event gated by: `quest_chain_03_complete`
- Availability: depth 2..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_03_complete`
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +7; xp +6; set flag `world_event_003_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; set flag `world_event_003_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (LOOKS) + medium shift
Success: gold +9; xp +7; set flag `world_event_003_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -5; add wound 1; set flag `world_event_003_resolved`; notes: The aggressive move collapses into losses and open danger.

## 104. Riverside Reliquary in Fog [004] (`evt_world_004`)
- Path: Follow-up event gated by: `quest_chain_04_complete`
- Availability: depth 3..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_04_complete`
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +8; xp +7; set flag `world_event_004_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; set flag `world_event_004_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +10; xp +8; set flag `world_event_004_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -6; add wound 2; set flag `world_event_004_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 105. Hinterland Smithy of Quiet Knives [005] (`evt_world_005`)
- Path: Follow-up event gated by: `quest_chain_05_complete`
- Availability: depth 4..999, tiers any, unique_once=true
- Requires flags: `quest_chain_05_complete`
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +9; xp +8; item: lantern oil; set flag `world_event_005_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; set flag `world_event_005_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +11; xp +9; set flag `world_event_005_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -7; add wound 3; set flag `world_event_005_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 106. Coldwind Cairn at Low Tide [006] (`evt_world_006`)
- Path: Follow-up event gated by: `quest_chain_06_complete`
- Availability: depth 5..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_06_complete`
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (LOOKS) + hard shift
Success: gold +10; xp +9; heal wound 1; set flag `world_event_006_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; set flag `world_event_006_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (INTELLIGENCE) + hard shift
Success: gold +12; xp +10; set flag `world_event_006_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -8; add wound 1; set flag `world_event_006_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 107. Sunken Garden of Embers [007] (`evt_world_007`)
- Path: Follow-up event gated by: `quest_chain_07_complete`
- Availability: depth 6..999, tiers any, unique_once=true
- Requires flags: `quest_chain_07_complete`
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +11; xp +3; set flag `world_event_007_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; set flag `world_event_007_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + hard shift
Success: gold +13; xp +11; honor +1; set flag `world_event_007_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -2; add wound 2; set flag `world_event_007_resolved`; notes: The aggressive move collapses into losses and open danger.

## 108. Stonegate Shrine in Rain [008] (`evt_world_008`)
- Path: Follow-up event gated by: `quest_chain_08_complete`
- Availability: depth 7..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_08_complete`
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + easy shift
Success: gold +12; xp +4; set flag `world_event_008_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; set flag `world_event_008_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + very_hard shift
Success: gold +14; xp +4; set flag `world_event_008_resolved`; triggers fight; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -3; add wound 3; set flag `world_event_008_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 109. Highvale Patrol on Broken Stone [009] (`evt_world_009`)
- Path: Follow-up event gated by: `quest_chain_09_complete`
- Availability: depth 8..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_09_complete`
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + easy shift
Success: gold +4; xp +5; set flag `world_event_009_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 2; set flag `world_event_009_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +15; xp +5; set flag `world_event_009_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -4; add wound 1; set flag `world_event_009_resolved`; notes: The aggressive move collapses into losses and open danger.

## 110. Nightmarket Bridge Under Watch [010] (`evt_world_010`)
- Path: Follow-up event gated by: `quest_chain_10_complete`
- Availability: depth 9..999, tiers boss, unique_once=true
- Requires flags: `quest_chain_10_complete`
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +5; xp +6; item: signal whistle; set flag `world_event_010_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; set flag `world_event_010_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (LOOKS) + medium shift
Success: gold +16; xp +6; set flag `world_event_010_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -5; add wound 2; set flag `world_event_010_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 111. Marshroad Camp on the Ridge [011] (`evt_world_011`)
- Path: Follow-up event gated by: `quest_chain_11_complete`
- Availability: depth 10..999, tiers any, unique_once=true
- Requires flags: `quest_chain_11_complete`
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +6; xp +7; set flag `world_event_011_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; set flag `world_event_011_resolved`; triggers fight; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +17; xp +7; set flag `world_event_011_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -6; add wound 3; set flag `world_event_011_resolved`; notes: The aggressive move collapses into losses and open danger.

## 112. Blackbarrow Outpost in Bitter Wind [012] (`evt_world_012`)
- Path: Follow-up event gated by: `quest_chain_12_complete`
- Availability: depth 11..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_12_complete`
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +7; xp +8; heal wound 1; set flag `world_event_012_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; set flag `world_event_012_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +6; xp +8; set flag `world_event_012_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -7; add wound 1; set flag `world_event_012_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 113. Oldfort Tunnel Without Witness [013] (`evt_world_013`)
- Path: Follow-up event gated by: `quest_chain_13_complete`
- Availability: depth 12..999, tiers any, unique_once=true
- Requires flags: `quest_chain_13_complete`
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (LOOKS) + medium shift
Success: gold +8; xp +9; honor +1; set flag `world_event_013_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; set flag `world_event_013_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (INTELLIGENCE) + hard shift
Success: gold +7; xp +9; set flag `world_event_013_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -8; add wound 2; set flag `world_event_013_resolved`; notes: The aggressive move collapses into losses and open danger.

## 114. Whispering Mausoleum of the Last Bell [014] (`evt_world_014`)
- Path: Follow-up event gated by: `quest_chain_14_complete`
- Availability: depth 13..999, tiers any, unique_once=true
- Requires flags: `quest_chain_14_complete`
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +9; xp +3; set flag `world_event_014_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; set flag `world_event_014_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + hard shift
Success: gold +8; xp +10; honor +1; add wound 1; set flag `world_event_014_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -2; add wound 3; set flag `world_event_014_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 115. Borderland Messenger of Crooked Paths [015] (`evt_world_015`)
- Path: Follow-up event gated by: `quest_chain_15_complete`
- Availability: depth 14..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_15_complete`
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +10; xp +4; item: throwing knife; set flag `world_event_015_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; set flag `world_event_015_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + hard shift
Success: gold +9; xp +11; set flag `world_event_015_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -3; add wound 1; set flag `world_event_015_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 116. Crosswind Archive at the Ford [016] (`evt_world_016`)
- Path: Follow-up event gated by: `quest_chain_16_complete`
- Availability: depth 15..999, tiers elite, boss, unique_once=true
- Requires flags: `quest_chain_16_complete`
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + easy shift
Success: gold +11; xp +5; set flag `world_event_016_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; set flag `world_event_016_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +10; xp +4; set flag `world_event_016_resolved`; triggers fight; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -4; add wound 2; set flag `world_event_016_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 117. Deepwood Workshop at First Light [017] (`evt_world_017`)
- Path: Follow-up event gated by: `quest_chain_17_complete`
- Availability: depth 16..999, tiers any, unique_once=true
- Requires flags: `quest_chain_17_complete`
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + easy shift
Success: gold +12; xp +6; set flag `world_event_017_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; set flag `world_event_017_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (LOOKS) + very_hard shift
Success: gold +11; xp +5; set flag `world_event_017_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -5; add wound 3; set flag `world_event_017_resolved`; notes: The aggressive move collapses into losses and open danger.

## 118. Westwall Beacon Beyond the Wall [018] (`evt_world_018`)
- Path: Follow-up event gated by: `quest_chain_18_complete`
- Availability: depth 17..999, tiers normal, elite, unique_once=true
- Requires flags: `quest_chain_18_complete`
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +4; xp +7; heal wound 1; set flag `world_event_018_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 2; set flag `world_event_018_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +12; xp +6; set flag `world_event_018_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -6; add wound 1; set flag `world_event_018_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 119. Copperlane Market in Ruins [019] (`evt_world_019`)
- Path: Follow-up event gated by: `quest_chain_19_complete`
- Availability: depth 18..999, tiers any, unique_once=true
- Requires flags: `quest_chain_19_complete`
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +5; xp +8; set flag `world_event_019_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; set flag `world_event_019_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +13; xp +7; set flag `world_event_019_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -7; add wound 2; set flag `world_event_019_resolved`; notes: The aggressive move collapses into losses and open danger.

## 120. Grimwatch Harbor in Ash [020] (`evt_world_020`)
- Path: Follow-up event gated by: `quest_chain_20_complete`
- Availability: depth 19..999, tiers boss, unique_once=true
- Requires flags: `quest_chain_20_complete`
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (LOOKS) + medium shift
Success: gold +6; xp +9; item: bone charm; set flag `world_event_020_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; set flag `world_event_020_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +14; xp +8; set flag `world_event_020_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -8; add wound 3; set flag `world_event_020_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 121. Wayfarer Caravan at Dusk [021] (`evt_world_021`)
- Path: Standalone random event
- Availability: depth 20..999, tiers normal, elite, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +7; xp +3; set flag `world_event_021_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; set flag `world_event_021_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +15; xp +9; honor +1; set flag `world_event_021_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -2; add wound 1; set flag `world_event_021_resolved`; notes: The aggressive move collapses into losses and open danger.

## 122. Dustbound Cache Behind the Gate [022] (`evt_world_022`)
- Path: Standalone random event
- Availability: depth 21..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +8; xp +4; set flag `world_event_022_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; set flag `world_event_022_resolved`; triggers fight; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + hard shift
Success: gold +16; xp +10; set flag `world_event_022_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -3; add wound 2; set flag `world_event_022_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 123. Lantern Furnace in the Thicket [023] (`evt_world_023`)
- Path: Standalone random event
- Availability: depth 22..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + hard shift
Success: gold +9; xp +5; set flag `world_event_023_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; set flag `world_event_023_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +17; xp +11; set flag `world_event_023_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -4; add wound 3; set flag `world_event_023_resolved`; notes: The aggressive move collapses into losses and open danger.

## 124. Riverside Reliquary in Fog [024] (`evt_world_024`)
- Path: Standalone random event
- Availability: depth 23..999, tiers elite, boss, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (WISDOM) + easy shift
Success: gold +10; xp +6; heal wound 1; set flag `world_event_024_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; set flag `world_event_024_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (LOOKS) + hard shift
Success: gold +6; xp +4; set flag `world_event_024_resolved`; triggers fight; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -5; add wound 1; set flag `world_event_024_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 125. Hinterland Smithy of Quiet Knives [025] (`evt_world_025`)
- Path: Standalone random event
- Availability: depth 0..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + easy shift
Success: gold +11; xp +7; item: iron ration; set flag `world_event_025_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; set flag `world_event_025_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +7; xp +5; set flag `world_event_025_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -6; add wound 2; set flag `world_event_025_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 126. Coldwind Cairn at Low Tide [026] (`evt_world_026`)
- Path: Standalone random event
- Availability: depth 1..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +12; xp +8; honor +1; set flag `world_event_026_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; set flag `world_event_026_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + very_hard shift
Success: gold +8; xp +6; set flag `world_event_026_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -7; add wound 3; set flag `world_event_026_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 127. Sunken Garden of Embers [027] (`evt_world_027`)
- Path: Standalone random event
- Availability: depth 2..999, tiers normal, elite, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (LOOKS) + medium shift
Success: gold +4; xp +9; set flag `world_event_027_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 2; set flag `world_event_027_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +9; xp +7; set flag `world_event_027_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -8; add wound 1; set flag `world_event_027_resolved`; notes: The aggressive move collapses into losses and open danger.

## 128. Stonegate Shrine in Rain [028] (`evt_world_028`)
- Path: Standalone random event
- Availability: depth 3..999, tiers elite, boss, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +5; xp +3; set flag `world_event_028_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; set flag `world_event_028_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +10; xp +8; honor +1; add wound 1; set flag `world_event_028_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -2; add wound 2; set flag `world_event_028_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 129. Highvale Patrol on Broken Stone [029] (`evt_world_029`)
- Path: Standalone random event
- Availability: depth 4..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +6; xp +4; set flag `world_event_029_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; set flag `world_event_029_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +11; xp +9; set flag `world_event_029_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -3; add wound 3; set flag `world_event_029_resolved`; notes: The aggressive move collapses into losses and open danger.

## 130. Nightmarket Bridge Under Watch [030] (`evt_world_030`)
- Path: Standalone random event
- Availability: depth 5..999, tiers boss, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + hard shift
Success: gold +7; xp +5; heal wound 1; item: lockpick set; set flag `world_event_030_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; set flag `world_event_030_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +12; xp +10; set flag `world_event_030_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -4; add wound 1; set flag `world_event_030_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 131. Marshroad Camp on the Ridge [031] (`evt_world_031`)
- Path: Standalone random event
- Availability: depth 6..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + hard shift
Success: gold +8; xp +6; set flag `world_event_031_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; set flag `world_event_031_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (LOOKS) + hard shift
Success: gold +13; xp +11; set flag `world_event_031_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -5; add wound 2; set flag `world_event_031_resolved`; notes: The aggressive move collapses into losses and open danger.

## 132. Blackbarrow Outpost in Bitter Wind [032] (`evt_world_032`)
- Path: Standalone random event
- Availability: depth 7..999, tiers elite, boss, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + easy shift
Success: gold +9; xp +7; set flag `world_event_032_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; set flag `world_event_032_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +14; xp +4; set flag `world_event_032_resolved`; triggers fight; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -6; add wound 3; set flag `world_event_032_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 133. Oldfort Tunnel Without Witness [033] (`evt_world_033`)
- Path: Standalone random event
- Availability: depth 8..999, tiers normal, elite, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + easy shift
Success: gold +10; xp +8; set flag `world_event_033_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; set flag `world_event_033_resolved`; triggers fight; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +15; xp +5; set flag `world_event_033_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -7; add wound 1; set flag `world_event_033_resolved`; notes: The aggressive move collapses into losses and open danger.

## 134. Whispering Mausoleum of the Last Bell [034] (`evt_world_034`)
- Path: Standalone random event
- Availability: depth 9..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (LOOKS) + medium shift
Success: gold +11; xp +9; set flag `world_event_034_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; set flag `world_event_034_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + hard shift
Success: gold +16; xp +6; set flag `world_event_034_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -8; add wound 2; set flag `world_event_034_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 135. Borderland Messenger of Crooked Paths [035] (`evt_world_035`)
- Path: Standalone random event
- Availability: depth 10..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +12; xp +3; item: travel cloak; set flag `world_event_035_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; set flag `world_event_035_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + very_hard shift
Success: gold +17; xp +7; honor +1; set flag `world_event_035_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -2; add wound 3; set flag `world_event_035_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 136. Crosswind Archive at the Ford [036] (`evt_world_036`)
- Path: Standalone random event
- Availability: depth 11..999, tiers elite, boss, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +4; xp +4; heal wound 1; set flag `world_event_036_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 2; set flag `world_event_036_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +6; xp +8; set flag `world_event_036_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -3; add wound 1; set flag `world_event_036_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 137. Deepwood Workshop at First Light [037] (`evt_world_037`)
- Path: Standalone random event
- Availability: depth 12..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +5; xp +5; set flag `world_event_037_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; set flag `world_event_037_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +7; xp +9; set flag `world_event_037_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -4; add wound 2; set flag `world_event_037_resolved`; notes: The aggressive move collapses into losses and open danger.

## 138. Westwall Beacon Beyond the Wall [038] (`evt_world_038`)
- Path: Standalone random event
- Availability: depth 13..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (WISDOM) + hard shift
Success: gold +6; xp +6; set flag `world_event_038_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; set flag `world_event_038_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (LOOKS) + medium shift
Success: gold +8; xp +10; set flag `world_event_038_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -5; add wound 3; set flag `world_event_038_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 139. Copperlane Market in Ruins [039] (`evt_world_039`)
- Path: Standalone random event
- Availability: depth 14..999, tiers normal, elite, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + hard shift
Success: gold +7; xp +7; honor +1; set flag `world_event_039_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; set flag `world_event_039_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +9; xp +11; set flag `world_event_039_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -6; add wound 1; set flag `world_event_039_resolved`; notes: The aggressive move collapses into losses and open danger.

## 140. Grimwatch Harbor in Ash [040] (`evt_world_040`)
- Path: Standalone random event
- Availability: depth 15..999, tiers boss, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + easy shift
Success: gold +8; xp +8; item: healing salve; set flag `world_event_040_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; set flag `world_event_040_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +10; xp +4; set flag `world_event_040_resolved`; triggers fight; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -7; add wound 2; set flag `world_event_040_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 141. Wayfarer Caravan at Dusk [041] (`evt_world_041`)
- Path: Standalone random event
- Availability: depth 16..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (LOOKS) + easy shift
Success: gold +9; xp +9; set flag `world_event_041_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; set flag `world_event_041_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (INTELLIGENCE) + hard shift
Success: gold +11; xp +5; set flag `world_event_041_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -8; add wound 3; set flag `world_event_041_resolved`; notes: The aggressive move collapses into losses and open danger.

## 142. Dustbound Cache Behind the Gate [042] (`evt_world_042`)
- Path: Standalone random event
- Availability: depth 17..999, tiers normal, elite, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +10; xp +3; heal wound 1; set flag `world_event_042_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; set flag `world_event_042_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + hard shift
Success: gold +12; xp +6; honor +1; add wound 1; set flag `world_event_042_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -2; add wound 1; set flag `world_event_042_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 143. Lantern Furnace in the Thicket [043] (`evt_world_043`)
- Path: Standalone random event
- Availability: depth 18..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +11; xp +4; set flag `world_event_043_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; set flag `world_event_043_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + hard shift
Success: gold +13; xp +7; set flag `world_event_043_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -3; add wound 2; set flag `world_event_043_resolved`; notes: The aggressive move collapses into losses and open danger.

## 144. Riverside Reliquary in Fog [044] (`evt_world_044`)
- Path: Standalone random event
- Availability: depth 19..999, tiers elite, boss, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +12; xp +5; set flag `world_event_044_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; set flag `world_event_044_resolved`; triggers fight; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + very_hard shift
Success: gold +14; xp +8; set flag `world_event_044_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -4; add wound 3; set flag `world_event_044_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 145. Hinterland Smithy of Quiet Knives [045] (`evt_world_045`)
- Path: Standalone random event
- Availability: depth 20..999, tiers normal, elite, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +4; xp +6; item: smithing nails; set flag `world_event_045_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 2; set flag `world_event_045_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (LOOKS) + medium shift
Success: gold +15; xp +9; set flag `world_event_045_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -5; add wound 1; set flag `world_event_045_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 146. Coldwind Cairn at Low Tide [046] (`evt_world_046`)
- Path: Standalone random event
- Availability: depth 21..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + hard shift
Success: gold +5; xp +7; set flag `world_event_046_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; set flag `world_event_046_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +16; xp +10; set flag `world_event_046_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -6; add wound 2; set flag `world_event_046_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 147. Sunken Garden of Embers [047] (`evt_world_047`)
- Path: Standalone random event
- Availability: depth 22..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +6; xp +8; set flag `world_event_047_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; set flag `world_event_047_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +17; xp +11; set flag `world_event_047_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -7; add wound 3; set flag `world_event_047_resolved`; notes: The aggressive move collapses into losses and open danger.

## 148. Stonegate Shrine in Rain [048] (`evt_world_048`)
- Path: Standalone random event
- Availability: depth 23..999, tiers elite, boss, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (LOOKS) + easy shift
Success: gold +7; xp +9; heal wound 1; set flag `world_event_048_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; set flag `world_event_048_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +6; xp +4; set flag `world_event_048_resolved`; triggers fight; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -8; add wound 1; set flag `world_event_048_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 149. Highvale Patrol on Broken Stone [049] (`evt_world_049`)
- Path: Standalone random event
- Availability: depth 0..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + easy shift
Success: gold +8; xp +3; set flag `world_event_049_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; set flag `world_event_049_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + hard shift
Success: gold +7; xp +5; honor +1; set flag `world_event_049_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -2; add wound 2; set flag `world_event_049_resolved`; notes: The aggressive move collapses into losses and open danger.

## 150. Nightmarket Bridge Under Watch [050] (`evt_world_050`)
- Path: Standalone random event
- Availability: depth 1..999, tiers boss, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +9; xp +4; item: sturdy rope; set flag `world_event_050_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; set flag `world_event_050_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + hard shift
Success: gold +8; xp +6; set flag `world_event_050_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -3; add wound 3; set flag `world_event_050_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 151. Marshroad Camp on the Ridge [051] (`evt_world_051`)
- Path: Standalone random event
- Availability: depth 2..999, tiers normal, elite, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +10; xp +5; set flag `world_event_051_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; set flag `world_event_051_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +9; xp +7; set flag `world_event_051_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -4; add wound 1; set flag `world_event_051_resolved`; notes: The aggressive move collapses into losses and open danger.

## 152. Blackbarrow Outpost in Bitter Wind [052] (`evt_world_052`)
- Path: Standalone random event
- Availability: depth 3..999, tiers elite, boss, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +11; xp +6; honor +1; set flag `world_event_052_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; set flag `world_event_052_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (LOOKS) + hard shift
Success: gold +10; xp +8; set flag `world_event_052_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -5; add wound 2; set flag `world_event_052_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 153. Oldfort Tunnel Without Witness [053] (`evt_world_053`)
- Path: Standalone random event
- Availability: depth 4..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +12; xp +7; set flag `world_event_053_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; set flag `world_event_053_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + very_hard shift
Success: gold +11; xp +9; set flag `world_event_053_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -6; add wound 3; set flag `world_event_053_resolved`; notes: The aggressive move collapses into losses and open danger.

## 154. Whispering Mausoleum of the Last Bell [054] (`evt_world_054`)
- Path: Standalone random event
- Availability: depth 5..999, tiers normal, elite, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +4; xp +8; heal wound 1; set flag `world_event_054_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 2; set flag `world_event_054_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +12; xp +10; set flag `world_event_054_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -7; add wound 1; set flag `world_event_054_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 155. Borderland Messenger of Crooked Paths [055] (`evt_world_055`)
- Path: Standalone random event
- Availability: depth 6..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (LOOKS) + hard shift
Success: gold +5; xp +9; item: map scrap; set flag `world_event_055_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; set flag `world_event_055_resolved`; triggers fight; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +13; xp +11; set flag `world_event_055_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -8; add wound 2; set flag `world_event_055_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 156. Crosswind Archive at the Ford [056] (`evt_world_056`)
- Path: Standalone random event
- Availability: depth 7..999, tiers elite, boss, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + easy shift
Success: gold +6; xp +3; set flag `world_event_056_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; set flag `world_event_056_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +14; xp +4; honor +1; add wound 1; set flag `world_event_056_resolved`; triggers fight; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -2; add wound 3; set flag `world_event_056_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 157. Deepwood Workshop at First Light [057] (`evt_world_057`)
- Path: Standalone random event
- Availability: depth 8..999, tiers normal, elite, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + easy shift
Success: gold +7; xp +4; set flag `world_event_057_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; set flag `world_event_057_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +15; xp +5; set flag `world_event_057_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -3; add wound 1; set flag `world_event_057_resolved`; notes: The aggressive move collapses into losses and open danger.

## 158. Westwall Beacon Beyond the Wall [058] (`evt_world_058`)
- Path: Standalone random event
- Availability: depth 9..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +8; xp +5; set flag `world_event_058_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; set flag `world_event_058_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +16; xp +6; set flag `world_event_058_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -4; add wound 2; set flag `world_event_058_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 159. Copperlane Market in Ruins [059] (`evt_world_059`)
- Path: Standalone random event
- Availability: depth 10..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +9; xp +6; set flag `world_event_059_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; set flag `world_event_059_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (LOOKS) + hard shift
Success: gold +17; xp +7; set flag `world_event_059_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -5; add wound 3; set flag `world_event_059_resolved`; notes: The aggressive move collapses into losses and open danger.

## 160. Grimwatch Harbor in Ash [060] (`evt_world_060`)
- Path: Standalone random event
- Availability: depth 11..999, tiers boss, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +10; xp +7; heal wound 1; item: bandage roll; set flag `world_event_060_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; set flag `world_event_060_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +6; xp +8; set flag `world_event_060_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -6; add wound 1; set flag `world_event_060_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 161. Wayfarer Caravan at Dusk [061] (`evt_world_061`)
- Path: Standalone random event
- Availability: depth 12..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +11; xp +8; set flag `world_event_061_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; set flag `world_event_061_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +7; xp +9; set flag `world_event_061_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -7; add wound 2; set flag `world_event_061_resolved`; notes: The aggressive move collapses into losses and open danger.

## 162. Dustbound Cache Behind the Gate [062] (`evt_world_062`)
- Path: Standalone random event
- Availability: depth 13..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (LOOKS) + hard shift
Success: gold +12; xp +9; set flag `world_event_062_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; set flag `world_event_062_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + very_hard shift
Success: gold +8; xp +10; set flag `world_event_062_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -8; add wound 3; set flag `world_event_062_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 163. Lantern Furnace in the Thicket [063] (`evt_world_063`)
- Path: Standalone random event
- Availability: depth 14..999, tiers normal, elite, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +4; xp +3; set flag `world_event_063_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 2; set flag `world_event_063_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +9; xp +11; honor +1; set flag `world_event_063_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -2; add wound 1; set flag `world_event_063_resolved`; notes: The aggressive move collapses into losses and open danger.

## 164. Riverside Reliquary in Fog [064] (`evt_world_064`)
- Path: Standalone random event
- Availability: depth 15..999, tiers elite, boss, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (STRENGTH) + easy shift
Success: gold +5; xp +4; set flag `world_event_064_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; set flag `world_event_064_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +10; xp +4; set flag `world_event_064_resolved`; triggers fight; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -3; add wound 2; set flag `world_event_064_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 165. Hinterland Smithy of Quiet Knives [065] (`evt_world_065`)
- Path: Standalone random event
- Availability: depth 16..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + easy shift
Success: gold +6; xp +5; honor +1; item: lantern oil; set flag `world_event_065_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; set flag `world_event_065_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +11; xp +5; set flag `world_event_065_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -4; add wound 3; set flag `world_event_065_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 166. Coldwind Cairn at Low Tide [066] (`evt_world_066`)
- Path: Standalone random event
- Availability: depth 17..999, tiers normal, elite, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +7; xp +6; heal wound 1; set flag `world_event_066_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; set flag `world_event_066_resolved`; triggers fight; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (LOOKS) + medium shift
Success: gold +12; xp +6; set flag `world_event_066_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -5; add wound 1; set flag `world_event_066_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 167. Sunken Garden of Embers [067] (`evt_world_067`)
- Path: Standalone random event
- Availability: depth 18..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +8; xp +7; set flag `world_event_067_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; set flag `world_event_067_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +13; xp +7; set flag `world_event_067_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -6; add wound 2; set flag `world_event_067_resolved`; notes: The aggressive move collapses into losses and open danger.

## 168. Stonegate Shrine in Rain [068] (`evt_world_068`)
- Path: Standalone random event
- Availability: depth 19..999, tiers elite, boss, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +9; xp +8; set flag `world_event_068_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; set flag `world_event_068_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +14; xp +8; set flag `world_event_068_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -7; add wound 3; set flag `world_event_068_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 169. Highvale Patrol on Broken Stone [069] (`evt_world_069`)
- Path: Standalone random event
- Availability: depth 20..999, tiers normal, elite, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (LOOKS) + medium shift
Success: gold +10; xp +9; set flag `world_event_069_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; set flag `world_event_069_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (INTELLIGENCE) + hard shift
Success: gold +15; xp +9; set flag `world_event_069_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -8; add wound 1; set flag `world_event_069_resolved`; notes: The aggressive move collapses into losses and open danger.

## 170. Nightmarket Bridge Under Watch [070] (`evt_world_070`)
- Path: Standalone random event
- Availability: depth 21..999, tiers boss, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +11; xp +3; item: signal whistle; set flag `world_event_070_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; set flag `world_event_070_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (WISDOM) + hard shift
Success: gold +16; xp +10; honor +1; add wound 1; set flag `world_event_070_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -2; add wound 2; set flag `world_event_070_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 171. Marshroad Camp on the Ridge [071] (`evt_world_071`)
- Path: Standalone random event
- Availability: depth 22..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +12; xp +4; set flag `world_event_071_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; set flag `world_event_071_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + very_hard shift
Success: gold +17; xp +11; set flag `world_event_071_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -3; add wound 3; set flag `world_event_071_resolved`; notes: The aggressive move collapses into losses and open danger.

## 172. Blackbarrow Outpost in Bitter Wind [072] (`evt_world_072`)
- Path: Standalone random event
- Availability: depth 23..999, tiers elite, boss, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + easy shift
Success: gold +4; xp +5; heal wound 1; set flag `world_event_072_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 2; set flag `world_event_072_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +6; xp +4; set flag `world_event_072_resolved`; triggers fight; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -4; add wound 1; set flag `world_event_072_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 173. Oldfort Tunnel Without Witness [073] (`evt_world_073`)
- Path: Standalone random event
- Availability: depth 0..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + easy shift
Success: gold +5; xp +6; set flag `world_event_073_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; set flag `world_event_073_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (LOOKS) + medium shift
Success: gold +7; xp +5; set flag `world_event_073_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -5; add wound 2; set flag `world_event_073_resolved`; notes: The aggressive move collapses into losses and open danger.

## 174. Whispering Mausoleum of the Last Bell [074] (`evt_world_074`)
- Path: Standalone random event
- Availability: depth 1..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +6; xp +7; set flag `world_event_074_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; set flag `world_event_074_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +8; xp +6; set flag `world_event_074_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -6; add wound 3; set flag `world_event_074_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 175. Borderland Messenger of Crooked Paths [075] (`evt_world_075`)
- Path: Standalone random event
- Availability: depth 2..999, tiers normal, elite, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +7; xp +8; item: throwing knife; set flag `world_event_075_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; set flag `world_event_075_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +9; xp +7; set flag `world_event_075_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -7; add wound 1; set flag `world_event_075_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 176. Crosswind Archive at the Ford [076] (`evt_world_076`)
- Path: Standalone random event
- Availability: depth 3..999, tiers elite, boss, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (LOOKS) + medium shift
Success: gold +8; xp +9; set flag `world_event_076_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; set flag `world_event_076_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + hard shift
Success: gold +10; xp +8; set flag `world_event_076_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -8; add wound 2; set flag `world_event_076_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 177. Deepwood Workshop at First Light [077] (`evt_world_077`)
- Path: Standalone random event
- Availability: depth 4..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +9; xp +3; set flag `world_event_077_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; set flag `world_event_077_resolved`; triggers fight; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + hard shift
Success: gold +11; xp +9; honor +1; set flag `world_event_077_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -2; add wound 3; set flag `world_event_077_resolved`; notes: The aggressive move collapses into losses and open danger.

## 178. Westwall Beacon Beyond the Wall [078] (`evt_world_078`)
- Path: Standalone random event
- Availability: depth 5..999, tiers normal, elite, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +10; xp +4; honor +1; heal wound 1; set flag `world_event_078_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; set flag `world_event_078_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + hard shift
Success: gold +12; xp +10; set flag `world_event_078_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -3; add wound 1; set flag `world_event_078_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 179. Copperlane Market in Ruins [079] (`evt_world_079`)
- Path: Standalone random event
- Availability: depth 6..999, tiers any, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (INTELLIGENCE) + hard shift
Success: gold +11; xp +5; set flag `world_event_079_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; set flag `world_event_079_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +13; xp +11; set flag `world_event_079_resolved`; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -4; add wound 2; set flag `world_event_079_resolved`; notes: The aggressive move collapses into losses and open danger.

## 180. Grimwatch Harbor in Ash [080] (`evt_world_080`)
- Path: Standalone random event
- Availability: depth 7..999, tiers boss, unique_once=true
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (WISDOM) + easy shift
Success: gold +12; xp +6; item: bone charm; set flag `world_event_080_resolved`; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; set flag `world_event_080_resolved`; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (LOOKS) + very_hard shift
Success: gold +14; xp +4; set flag `world_event_080_resolved`; triggers fight; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -5; add wound 3; set flag `world_event_080_resolved`; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 181. Wayfarer Caravan at Dusk [081] (`evt_world_081`)
- Path: Standalone random event
- Availability: depth 8..999, tiers normal, elite, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + easy shift
Success: gold +4; xp +7; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 2; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +15; xp +5; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -6; add wound 1; notes: The aggressive move collapses into losses and open danger.

## 182. Dustbound Cache Behind the Gate [082] (`evt_world_082`)
- Path: Standalone random event
- Availability: depth 9..999, tiers any, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +5; xp +8; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +16; xp +6; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -7; add wound 2; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 183. Lantern Furnace in the Thicket [083] (`evt_world_083`)
- Path: Standalone random event
- Availability: depth 10..999, tiers any, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (LOOKS) + medium shift
Success: gold +6; xp +9; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +17; xp +7; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -8; add wound 3; notes: The aggressive move collapses into losses and open danger.

## 184. Riverside Reliquary in Fog [084] (`evt_world_084`)
- Path: Standalone random event
- Availability: depth 11..999, tiers elite, boss, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +7; xp +3; heal wound 1; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +6; xp +8; honor +1; add wound 1; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -2; add wound 1; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 185. Hinterland Smithy of Quiet Knives [085] (`evt_world_085`)
- Path: Standalone random event
- Availability: depth 12..999, tiers any, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +8; xp +4; item: iron ration; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + hard shift
Success: gold +7; xp +9; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -3; add wound 2; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 186. Coldwind Cairn at Low Tide [086] (`evt_world_086`)
- Path: Standalone random event
- Availability: depth 13..999, tiers any, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + hard shift
Success: gold +9; xp +5; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +8; xp +10; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -4; add wound 3; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 187. Sunken Garden of Embers [087] (`evt_world_087`)
- Path: Standalone random event
- Availability: depth 14..999, tiers normal, elite, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + hard shift
Success: gold +10; xp +6; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (LOOKS) + hard shift
Success: gold +9; xp +11; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -5; add wound 1; notes: The aggressive move collapses into losses and open danger.

## 188. Stonegate Shrine in Rain [088] (`evt_world_088`)
- Path: Standalone random event
- Availability: depth 15..999, tiers elite, boss, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + easy shift
Success: gold +11; xp +7; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; triggers fight; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +10; xp +4; triggers fight; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -6; add wound 2; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 189. Highvale Patrol on Broken Stone [089] (`evt_world_089`)
- Path: Standalone random event
- Availability: depth 16..999, tiers any, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + easy shift
Success: gold +12; xp +8; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + very_hard shift
Success: gold +11; xp +5; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -7; add wound 3; notes: The aggressive move collapses into losses and open danger.

## 190. Nightmarket Bridge Under Watch [090] (`evt_world_090`)
- Path: Standalone random event
- Availability: depth 17..999, tiers boss, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (LOOKS) + medium shift
Success: gold +4; xp +9; heal wound 1; item: lockpick set; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 2; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +12; xp +6; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -8; add wound 1; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 191. Marshroad Camp on the Ridge [091] (`evt_world_091`)
- Path: Standalone random event
- Availability: depth 18..999, tiers any, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +5; xp +3; honor +1; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +13; xp +7; honor +1; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -2; add wound 2; notes: The aggressive move collapses into losses and open danger.

## 192. Blackbarrow Outpost in Bitter Wind [092] (`evt_world_092`)
- Path: Standalone random event
- Availability: depth 19..999, tiers elite, boss, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +6; xp +4; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +14; xp +8; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -3; add wound 3; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 193. Oldfort Tunnel Without Witness [093] (`evt_world_093`)
- Path: Standalone random event
- Availability: depth 20..999, tiers normal, elite, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +7; xp +5; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +15; xp +9; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -4; add wound 1; notes: The aggressive move collapses into losses and open danger.

## 194. Whispering Mausoleum of the Last Bell [094] (`evt_world_094`)
- Path: Standalone random event
- Availability: depth 21..999, tiers any, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + hard shift
Success: gold +8; xp +6; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (LOOKS) + hard shift
Success: gold +16; xp +10; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -5; add wound 2; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 195. Borderland Messenger of Crooked Paths [095] (`evt_world_095`)
- Path: Standalone random event
- Availability: depth 22..999, tiers any, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + hard shift
Success: gold +9; xp +7; item: travel cloak; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +17; xp +11; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -6; add wound 3; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 196. Crosswind Archive at the Ford [096] (`evt_world_096`)
- Path: Standalone random event
- Availability: depth 23..999, tiers elite, boss, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + easy shift
Success: gold +10; xp +8; heal wound 1; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +6; xp +4; triggers fight; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -7; add wound 1; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 197. Deepwood Workshop at First Light [097] (`evt_world_097`)
- Path: Standalone random event
- Availability: depth 0..999, tiers any, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (LOOKS) + easy shift
Success: gold +11; xp +9; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (INTELLIGENCE) + hard shift
Success: gold +7; xp +5; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -8; add wound 2; notes: The aggressive move collapses into losses and open danger.

## 198. Westwall Beacon Beyond the Wall [098] (`evt_world_098`)
- Path: Standalone random event
- Availability: depth 1..999, tiers any, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +12; xp +3; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + very_hard shift
Success: gold +8; xp +6; honor +1; add wound 1; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -2; add wound 3; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 199. Copperlane Market in Ruins [099] (`evt_world_099`)
- Path: Standalone random event
- Availability: depth 2..999, tiers normal, elite, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +4; xp +4; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 2; triggers fight; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +9; xp +7; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -3; add wound 1; notes: The aggressive move collapses into losses and open danger.

## 200. Grimwatch Harbor in Ash [100] (`evt_world_100`)
- Path: Standalone random event
- Availability: depth 3..999, tiers boss, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +5; xp +5; item: healing salve; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +10; xp +8; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -4; add wound 2; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 201. Wayfarer Caravan at Dusk [101] (`evt_world_101`)
- Path: Standalone random event
- Availability: depth 4..999, tiers any, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +6; xp +6; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (LOOKS) + medium shift
Success: gold +11; xp +9; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -5; add wound 3; notes: The aggressive move collapses into losses and open danger.

## 202. Dustbound Cache Behind the Gate [102] (`evt_world_102`)
- Path: Standalone random event
- Availability: depth 5..999, tiers normal, elite, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + hard shift
Success: gold +7; xp +7; heal wound 1; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +12; xp +10; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -6; add wound 1; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 203. Lantern Furnace in the Thicket [103] (`evt_world_103`)
- Path: Standalone random event
- Availability: depth 6..999, tiers any, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +8; xp +8; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + hard shift
Success: gold +13; xp +11; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -7; add wound 2; notes: The aggressive move collapses into losses and open danger.

## 204. Riverside Reliquary in Fog [104] (`evt_world_104`)
- Path: Standalone random event
- Availability: depth 7..999, tiers elite, boss, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (LOOKS) + easy shift
Success: gold +9; xp +9; honor +1; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (INTELLIGENCE) + hard shift
Success: gold +14; xp +4; triggers fight; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -8; add wound 3; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 205. Hinterland Smithy of Quiet Knives [105] (`evt_world_105`)
- Path: Standalone random event
- Availability: depth 8..999, tiers normal, elite, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + easy shift
Success: gold +10; xp +3; item: smithing nails; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (WISDOM) + hard shift
Success: gold +15; xp +5; honor +1; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -2; add wound 1; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 206. Coldwind Cairn at Low Tide [106] (`evt_world_106`)
- Path: Standalone random event
- Availability: depth 9..999, tiers any, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +11; xp +4; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + hard shift
Success: gold +16; xp +6; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -3; add wound 2; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 207. Sunken Garden of Embers [107] (`evt_world_107`)
- Path: Standalone random event
- Availability: depth 10..999, tiers any, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +12; xp +5; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (CONSTITUTION) + very_hard shift
Success: gold +17; xp +7; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -4; add wound 3; notes: The aggressive move collapses into losses and open danger.

## 208. Stonegate Shrine in Rain [108] (`evt_world_108`)
- Path: Standalone random event
- Availability: depth 11..999, tiers elite, boss, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +4; xp +6; heal wound 1; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 2; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (LOOKS) + medium shift
Success: gold +6; xp +8; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -5; add wound 1; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 209. Highvale Patrol on Broken Stone [109] (`evt_world_109`)
- Path: Standalone random event
- Availability: depth 12..999, tiers any, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +5; xp +7; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + medium shift
Success: gold +7; xp +9; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -6; add wound 2; notes: The aggressive move collapses into losses and open danger.

## 210. Nightmarket Bridge Under Watch [110] (`evt_world_110`)
- Path: Standalone random event
- Availability: depth 13..999, tiers boss, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +6; xp +8; item: sturdy rope; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; triggers fight; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +8; xp +10; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -7; add wound 3; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 211. Marshroad Camp on the Ridge [111] (`evt_world_111`)
- Path: Standalone random event
- Availability: depth 14..999, tiers normal, elite, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (LOOKS) + hard shift
Success: gold +7; xp +9; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +9; xp +11; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -8; add wound 1; notes: The aggressive move collapses into losses and open danger.

## 212. Blackbarrow Outpost in Bitter Wind [112] (`evt_world_112`)
- Path: Standalone random event
- Availability: depth 15..999, tiers elite, boss, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + easy shift
Success: gold +8; xp +3; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + hard shift
Success: gold +10; xp +4; honor +1; add wound 1; triggers fight; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -2; add wound 2; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 213. Oldfort Tunnel Without Witness [113] (`evt_world_113`)
- Path: Standalone random event
- Availability: depth 16..999, tiers any, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + easy shift
Success: gold +9; xp +4; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + hard shift
Success: gold +11; xp +5; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -3; add wound 3; notes: The aggressive move collapses into losses and open danger.

## 214. Whispering Mausoleum of the Last Bell [114] (`evt_world_114`)
- Path: Standalone random event
- Availability: depth 17..999, tiers normal, elite, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +10; xp +5; heal wound 1; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + hard shift
Success: gold +12; xp +6; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -4; add wound 1; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 215. Borderland Messenger of Crooked Paths [115] (`evt_world_115`)
- Path: Standalone random event
- Availability: depth 18..999, tiers any, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +11; xp +6; item: map scrap; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (LOOKS) + hard shift
Success: gold +13; xp +7; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -5; add wound 2; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 216. Crosswind Archive at the Ford [116] (`evt_world_116`)
- Path: Standalone random event
- Availability: depth 19..999, tiers elite, boss, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +12; xp +7; notes: You keep control and extract steady value from the encounter.
Failure: gold -2; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + very_hard shift
Success: gold +14; xp +8; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -6; add wound 3; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 217. Deepwood Workshop at First Light [117] (`evt_world_117`)
- Path: Standalone random event
- Availability: depth 20..999, tiers normal, elite, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CONSTITUTION) + medium shift
Success: gold +4; xp +8; honor +1; notes: You keep control and extract steady value from the encounter.
Failure: gold -3; add wound 2; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= stat level + mastery die + ability mod (STRENGTH) + medium shift
Success: gold +15; xp +9; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -7; add wound 1; notes: The aggressive move collapses into losses and open danger.

## 218. Westwall Beacon Beyond the Wall [118] (`evt_world_118`)
- Path: Standalone random event
- Availability: depth 21..999, tiers any, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= stat level + mastery die + ability mod (LOOKS) + hard shift
Success: gold +5; xp +9; notes: You keep control and extract steady value from the encounter.
Failure: gold -4; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (INTELLIGENCE) + medium shift
Success: gold +16; xp +10; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -8; add wound 2; triggers fight; notes: The aggressive move collapses into losses and open danger.

## 219. Copperlane Market in Ruins [119] (`evt_world_119`)
- Path: Standalone random event
- Availability: depth 22..999, tiers any, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (CHARISMA) + hard shift
Success: gold +6; xp +3; notes: You keep control and extract steady value from the encounter.
Failure: gold -5; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (WISDOM) + medium shift
Success: gold +17; xp +11; honor +1; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -2; add wound 3; notes: The aggressive move collapses into losses and open danger.

## 220. Grimwatch Harbor in Ash [120] (`evt_world_120`)
- Path: Standalone random event
- Availability: depth 23..999, tiers boss, unique_once=false
- Requires flags: none
- Choices:
1. Take the careful approach
Roll: d100 <= skill level + mastery die + ability mod (STRENGTH) + easy shift
Success: gold +7; xp +4; heal wound 1; item: bandage roll; notes: You keep control and extract steady value from the encounter.
Failure: gold -1; add wound 1; notes: Your cautious plan stalls, and the situation turns against you.
1. Push for a bigger payoff
Roll: d100 <= skill level + mastery die + ability mod (DEXTERITY) + medium shift
Success: gold +6; xp +4; triggers fight; notes: You gamble on momentum and seize a larger payoff.
Failure: gold -3; add wound 1; triggers fight; notes: The aggressive move collapses into losses and open danger.
