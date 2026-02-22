# Handcrafted Event Pack (200)

Event count: 200

Weapon-specific talent rewards are encoded with explicit weapon names in `add_talents` entries.

## 1. Gilded Censer - Whisper (`evt_hand_t01_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=13; xp_delta=16; flags=hand_t01_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t01_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=16; xp_delta=18; item=brass censer ash; flags=hand_t01_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t01_s1_done; fight=true

## 2. Gilded Censer - Trace (`evt_hand_t01_s2`)
- Requires: hand_t01_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=17; xp_delta=22; item=brass censer ash; flags=hand_t01_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t01_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=21; xp_delta=24; flags=hand_t01_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t01_s2_done; fight=true

## 3. Gilded Censer - Crossroads (`evt_hand_t01_s3`)
- Requires: hand_t01_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=21; xp_delta=28; bp_delta=1; flags=hand_t01_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t01_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=26; xp_delta=30; honor_delta=1; item=brass censer ash; flags=hand_t01_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t01_s3_done; fight=true

## 4. Gilded Censer - Siege (`evt_hand_t01_s4`)
- Requires: hand_t01_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=25; xp_delta=34; lp_delta=1; item=brass censer ash; flags=hand_t01_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t01_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=31; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t01_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t01_s4_done; fight=true

## 5. Gilded Censer - Reckoning (`evt_hand_t01_s5`)
- Requires: hand_t01_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=29; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_iron_heart; flags=hand_t01_s5_done,hand_t01_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t01_s5_done,hand_t01_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=36; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=brass censer ash; talents=event_iron_heart; flags=hand_t01_s5_done,hand_t01_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t01_s5_done,hand_t01_complete; fight=true

## 6. Red Tallow - Whisper (`evt_hand_t02_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=14; xp_delta=16; flags=hand_t02_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t02_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=17; xp_delta=18; item=red tallow wick; flags=hand_t02_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t02_s1_done; fight=true

## 7. Red Tallow - Trace (`evt_hand_t02_s2`)
- Requires: hand_t02_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=18; xp_delta=22; item=red tallow wick; flags=hand_t02_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t02_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=22; xp_delta=24; flags=hand_t02_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t02_s2_done; fight=true

## 8. Red Tallow - Crossroads (`evt_hand_t02_s3`)
- Requires: hand_t02_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=22; xp_delta=28; bp_delta=1; flags=hand_t02_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t02_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=27; xp_delta=30; honor_delta=1; item=red tallow wick; flags=hand_t02_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t02_s3_done; fight=true

## 9. Red Tallow - Siege (`evt_hand_t02_s4`)
- Requires: hand_t02_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=26; xp_delta=34; lp_delta=1; item=red tallow wick; flags=hand_t02_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t02_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=32; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t02_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t02_s4_done; fight=true

## 10. Red Tallow - Reckoning (`evt_hand_t02_s5`)
- Requires: hand_t02_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=30; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_ashen_stride@Spear; flags=hand_t02_s5_done,hand_t02_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t02_s5_done,hand_t02_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=37; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=red tallow wick; talents=event_ashen_stride@Spear; flags=hand_t02_s5_done,hand_t02_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t02_s5_done,hand_t02_complete; fight=true

## 11. Hollow Tribunal - Whisper (`evt_hand_t03_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=15; xp_delta=16; flags=hand_t03_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t03_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=18; xp_delta=18; item=hollow verdict seal; flags=hand_t03_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t03_s1_done; fight=true

## 12. Hollow Tribunal - Trace (`evt_hand_t03_s2`)
- Requires: hand_t03_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=19; xp_delta=22; item=hollow verdict seal; flags=hand_t03_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t03_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=23; xp_delta=24; flags=hand_t03_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t03_s2_done; fight=true

## 13. Hollow Tribunal - Crossroads (`evt_hand_t03_s3`)
- Requires: hand_t03_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=23; xp_delta=28; bp_delta=1; flags=hand_t03_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t03_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=28; xp_delta=30; honor_delta=1; item=hollow verdict seal; flags=hand_t03_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t03_s3_done; fight=true

## 14. Hollow Tribunal - Siege (`evt_hand_t03_s4`)
- Requires: hand_t03_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=27; xp_delta=34; lp_delta=1; item=hollow verdict seal; flags=hand_t03_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t03_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=33; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t03_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t03_s4_done; fight=true

## 15. Hollow Tribunal - Reckoning (`evt_hand_t03_s5`)
- Requires: hand_t03_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=31; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_quickened_sight; flags=hand_t03_s5_done,hand_t03_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t03_s5_done,hand_t03_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=38; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=hollow verdict seal; talents=event_quickened_sight; flags=hand_t03_s5_done,hand_t03_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t03_s5_done,hand_t03_complete; fight=true

## 16. Shattered Pike - Whisper (`evt_hand_t04_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=12; xp_delta=16; flags=hand_t04_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t04_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=19; xp_delta=18; item=pikehead relic; flags=hand_t04_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t04_s1_done; fight=true

## 17. Shattered Pike - Trace (`evt_hand_t04_s2`)
- Requires: hand_t04_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=16; xp_delta=22; item=pikehead relic; flags=hand_t04_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t04_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=24; xp_delta=24; flags=hand_t04_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t04_s2_done; fight=true

## 18. Shattered Pike - Crossroads (`evt_hand_t04_s3`)
- Requires: hand_t04_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=20; xp_delta=28; bp_delta=1; flags=hand_t04_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t04_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=29; xp_delta=30; honor_delta=1; item=pikehead relic; flags=hand_t04_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t04_s3_done; fight=true

## 19. Shattered Pike - Siege (`evt_hand_t04_s4`)
- Requires: hand_t04_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=24; xp_delta=34; lp_delta=1; item=pikehead relic; flags=hand_t04_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t04_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=34; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t04_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t04_s4_done; fight=true

## 20. Shattered Pike - Reckoning (`evt_hand_t04_s5`)
- Requires: hand_t04_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=28; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_stoneguard; flags=hand_t04_s5_done,hand_t04_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t04_s5_done,hand_t04_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=39; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=pikehead relic; talents=event_stoneguard; flags=hand_t04_s5_done,hand_t04_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t04_s5_done,hand_t04_complete; fight=true

## 21. Broken Astrolabe - Whisper (`evt_hand_t05_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=13; xp_delta=16; flags=hand_t05_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t05_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=15; xp_delta=18; item=astrolabe tooth; flags=hand_t05_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t05_s1_done; fight=true

## 22. Broken Astrolabe - Trace (`evt_hand_t05_s2`)
- Requires: hand_t05_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=17; xp_delta=22; item=astrolabe tooth; flags=hand_t05_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t05_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=20; xp_delta=24; flags=hand_t05_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t05_s2_done; fight=true

## 23. Broken Astrolabe - Crossroads (`evt_hand_t05_s3`)
- Requires: hand_t05_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=21; xp_delta=28; bp_delta=1; flags=hand_t05_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t05_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=25; xp_delta=30; honor_delta=1; item=astrolabe tooth; flags=hand_t05_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t05_s3_done; fight=true

## 24. Broken Astrolabe - Siege (`evt_hand_t05_s4`)
- Requires: hand_t05_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=25; xp_delta=34; lp_delta=1; item=astrolabe tooth; flags=hand_t05_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t05_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=30; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t05_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t05_s4_done; fight=true

## 25. Broken Astrolabe - Reckoning (`evt_hand_t05_s5`)
- Requires: hand_t05_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=29; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_river_footing@Dagger; flags=hand_t05_s5_done,hand_t05_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t05_s5_done,hand_t05_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=35; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=astrolabe tooth; talents=event_river_footing@Dagger; flags=hand_t05_s5_done,hand_t05_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t05_s5_done,hand_t05_complete; fight=true

## 26. Salt Widow - Whisper (`evt_hand_t06_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=14; xp_delta=16; flags=hand_t06_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t06_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=16; xp_delta=18; item=salt widow token; flags=hand_t06_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t06_s1_done; fight=true

## 27. Salt Widow - Trace (`evt_hand_t06_s2`)
- Requires: hand_t06_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=18; xp_delta=22; item=salt widow token; flags=hand_t06_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t06_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=21; xp_delta=24; flags=hand_t06_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t06_s2_done; fight=true

## 28. Salt Widow - Crossroads (`evt_hand_t06_s3`)
- Requires: hand_t06_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=22; xp_delta=28; bp_delta=1; flags=hand_t06_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t06_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=26; xp_delta=30; honor_delta=1; item=salt widow token; flags=hand_t06_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t06_s3_done; fight=true

## 29. Salt Widow - Siege (`evt_hand_t06_s4`)
- Requires: hand_t06_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=26; xp_delta=34; lp_delta=1; item=salt widow token; flags=hand_t06_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t06_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=31; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t06_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t06_s4_done; fight=true

## 30. Salt Widow - Reckoning (`evt_hand_t06_s5`)
- Requires: hand_t06_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=30; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_blooded_edge@Short sword; flags=hand_t06_s5_done,hand_t06_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t06_s5_done,hand_t06_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=36; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=salt widow token; talents=event_blooded_edge@Short sword; flags=hand_t06_s5_done,hand_t06_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t06_s5_done,hand_t06_complete; fight=true

## 31. Black Marsh Bell - Whisper (`evt_hand_t07_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=15; xp_delta=16; flags=hand_t07_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t07_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=17; xp_delta=18; item=marsh bell clapper; flags=hand_t07_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t07_s1_done; fight=true

## 32. Black Marsh Bell - Trace (`evt_hand_t07_s2`)
- Requires: hand_t07_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=19; xp_delta=22; item=marsh bell clapper; flags=hand_t07_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t07_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=22; xp_delta=24; flags=hand_t07_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t07_s2_done; fight=true

## 33. Black Marsh Bell - Crossroads (`evt_hand_t07_s3`)
- Requires: hand_t07_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=23; xp_delta=28; bp_delta=1; flags=hand_t07_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t07_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=27; xp_delta=30; honor_delta=1; item=marsh bell clapper; flags=hand_t07_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t07_s3_done; fight=true

## 34. Black Marsh Bell - Siege (`evt_hand_t07_s4`)
- Requires: hand_t07_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=27; xp_delta=34; lp_delta=1; item=marsh bell clapper; flags=hand_t07_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t07_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=32; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t07_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t07_s4_done; fight=true

## 35. Black Marsh Bell - Reckoning (`evt_hand_t07_s5`)
- Requires: hand_t07_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=31; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_storm_arm@Greatsword; flags=hand_t07_s5_done,hand_t07_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t07_s5_done,hand_t07_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=37; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=marsh bell clapper; talents=event_storm_arm@Greatsword; flags=hand_t07_s5_done,hand_t07_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t07_s5_done,hand_t07_complete; fight=true

## 36. Sunken Patent - Whisper (`evt_hand_t08_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=12; xp_delta=16; flags=hand_t08_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t08_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=18; xp_delta=18; item=sealed patent strip; flags=hand_t08_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t08_s1_done; fight=true

## 37. Sunken Patent - Trace (`evt_hand_t08_s2`)
- Requires: hand_t08_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=16; xp_delta=22; item=sealed patent strip; flags=hand_t08_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t08_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=23; xp_delta=24; flags=hand_t08_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t08_s2_done; fight=true

## 38. Sunken Patent - Crossroads (`evt_hand_t08_s3`)
- Requires: hand_t08_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=20; xp_delta=28; bp_delta=1; flags=hand_t08_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t08_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=28; xp_delta=30; honor_delta=1; item=sealed patent strip; flags=hand_t08_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t08_s3_done; fight=true

## 39. Sunken Patent - Siege (`evt_hand_t08_s4`)
- Requires: hand_t08_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=24; xp_delta=34; lp_delta=1; item=sealed patent strip; flags=hand_t08_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t08_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=33; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t08_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t08_s4_done; fight=true

## 40. Sunken Patent - Reckoning (`evt_hand_t08_s5`)
- Requires: hand_t08_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=28; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_far_mark; flags=hand_t08_s5_done,hand_t08_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t08_s5_done,hand_t08_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=38; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=sealed patent strip; talents=event_far_mark; flags=hand_t08_s5_done,hand_t08_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t08_s5_done,hand_t08_complete; fight=true

## 41. Ragpicker Prince - Whisper (`evt_hand_t09_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=13; xp_delta=16; flags=hand_t09_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t09_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=19; xp_delta=18; item=ragpicker signet; flags=hand_t09_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t09_s1_done; fight=true

## 42. Ragpicker Prince - Trace (`evt_hand_t09_s2`)
- Requires: hand_t09_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=17; xp_delta=22; item=ragpicker signet; flags=hand_t09_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t09_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=24; xp_delta=24; flags=hand_t09_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t09_s2_done; fight=true

## 43. Ragpicker Prince - Crossroads (`evt_hand_t09_s3`)
- Requires: hand_t09_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=21; xp_delta=28; bp_delta=1; flags=hand_t09_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t09_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=29; xp_delta=30; honor_delta=1; item=ragpicker signet; flags=hand_t09_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t09_s3_done; fight=true

## 44. Ragpicker Prince - Siege (`evt_hand_t09_s4`)
- Requires: hand_t09_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=25; xp_delta=34; lp_delta=1; item=ragpicker signet; flags=hand_t09_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t09_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=34; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t09_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t09_s4_done; fight=true

## 45. Ragpicker Prince - Reckoning (`evt_hand_t09_s5`)
- Requires: hand_t09_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=29; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_steady_guard; flags=hand_t09_s5_done,hand_t09_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t09_s5_done,hand_t09_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=39; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=ragpicker signet; talents=event_steady_guard; flags=hand_t09_s5_done,hand_t09_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t09_s5_done,hand_t09_complete; fight=true

## 46. Frost Ledger - Whisper (`evt_hand_t10_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=14; xp_delta=16; flags=hand_t10_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t10_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=15; xp_delta=18; item=frosted tally slate; flags=hand_t10_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t10_s1_done; fight=true

## 47. Frost Ledger - Trace (`evt_hand_t10_s2`)
- Requires: hand_t10_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=18; xp_delta=22; item=frosted tally slate; flags=hand_t10_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t10_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=20; xp_delta=24; flags=hand_t10_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t10_s2_done; fight=true

## 48. Frost Ledger - Crossroads (`evt_hand_t10_s3`)
- Requires: hand_t10_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=22; xp_delta=28; bp_delta=1; flags=hand_t10_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t10_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=25; xp_delta=30; honor_delta=1; item=frosted tally slate; flags=hand_t10_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t10_s3_done; fight=true

## 49. Frost Ledger - Siege (`evt_hand_t10_s4`)
- Requires: hand_t10_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=26; xp_delta=34; lp_delta=1; item=frosted tally slate; flags=hand_t10_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t10_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=30; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t10_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t10_s4_done; fight=true

## 50. Frost Ledger - Reckoning (`evt_hand_t10_s5`)
- Requires: hand_t10_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=30; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_fast_healing; flags=hand_t10_s5_done,hand_t10_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t10_s5_done,hand_t10_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=35; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=frosted tally slate; talents=event_fast_healing; flags=hand_t10_s5_done,hand_t10_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t10_s5_done,hand_t10_complete; fight=true

## 51. Ivory Pilgrims - Whisper (`evt_hand_t11_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=15; xp_delta=16; flags=hand_t11_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t11_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=16; xp_delta=18; item=ivory pilgrim bead; flags=hand_t11_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t11_s1_done; fight=true

## 52. Ivory Pilgrims - Trace (`evt_hand_t11_s2`)
- Requires: hand_t11_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=19; xp_delta=22; item=ivory pilgrim bead; flags=hand_t11_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t11_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=21; xp_delta=24; flags=hand_t11_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t11_s2_done; fight=true

## 53. Ivory Pilgrims - Crossroads (`evt_hand_t11_s3`)
- Requires: hand_t11_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=23; xp_delta=28; bp_delta=1; flags=hand_t11_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t11_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=26; xp_delta=30; honor_delta=1; item=ivory pilgrim bead; flags=hand_t11_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t11_s3_done; fight=true

## 54. Ivory Pilgrims - Siege (`evt_hand_t11_s4`)
- Requires: hand_t11_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=27; xp_delta=34; lp_delta=1; item=ivory pilgrim bead; flags=hand_t11_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t11_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=31; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t11_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t11_s4_done; fight=true

## 55. Ivory Pilgrims - Reckoning (`evt_hand_t11_s5`)
- Requires: hand_t11_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=31; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_braced_reach@Battle axe; flags=hand_t11_s5_done,hand_t11_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t11_s5_done,hand_t11_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=36; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=ivory pilgrim bead; talents=event_braced_reach@Battle axe; flags=hand_t11_s5_done,hand_t11_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t11_s5_done,hand_t11_complete; fight=true

## 56. Knotted Banner - Whisper (`evt_hand_t12_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=12; xp_delta=16; flags=hand_t12_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t12_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=17; xp_delta=18; item=banner knot; flags=hand_t12_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t12_s1_done; fight=true

## 57. Knotted Banner - Trace (`evt_hand_t12_s2`)
- Requires: hand_t12_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=16; xp_delta=22; item=banner knot; flags=hand_t12_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t12_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=22; xp_delta=24; flags=hand_t12_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t12_s2_done; fight=true

## 58. Knotted Banner - Crossroads (`evt_hand_t12_s3`)
- Requires: hand_t12_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=20; xp_delta=28; bp_delta=1; flags=hand_t12_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t12_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=27; xp_delta=30; honor_delta=1; item=banner knot; flags=hand_t12_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t12_s3_done; fight=true

## 59. Knotted Banner - Siege (`evt_hand_t12_s4`)
- Requires: hand_t12_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=24; xp_delta=34; lp_delta=1; item=banner knot; flags=hand_t12_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t12_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=32; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t12_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t12_s4_done; fight=true

## 60. Knotted Banner - Reckoning (`evt_hand_t12_s5`)
- Requires: hand_t12_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=28; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_hardened_plate; flags=hand_t12_s5_done,hand_t12_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t12_s5_done,hand_t12_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=37; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=banner knot; talents=event_hardened_plate; flags=hand_t12_s5_done,hand_t12_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t12_s5_done,hand_t12_complete; fight=true

## 61. Lantern Tax - Whisper (`evt_hand_t13_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=13; xp_delta=16; flags=hand_t13_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t13_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=18; xp_delta=18; item=tax lantern lens; flags=hand_t13_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t13_s1_done; fight=true

## 62. Lantern Tax - Trace (`evt_hand_t13_s2`)
- Requires: hand_t13_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=17; xp_delta=22; item=tax lantern lens; flags=hand_t13_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t13_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=23; xp_delta=24; flags=hand_t13_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t13_s2_done; fight=true

## 63. Lantern Tax - Crossroads (`evt_hand_t13_s3`)
- Requires: hand_t13_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=21; xp_delta=28; bp_delta=1; flags=hand_t13_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t13_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=28; xp_delta=30; honor_delta=1; item=tax lantern lens; flags=hand_t13_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t13_s3_done; fight=true

## 64. Lantern Tax - Siege (`evt_hand_t13_s4`)
- Requires: hand_t13_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=25; xp_delta=34; lp_delta=1; item=tax lantern lens; flags=hand_t13_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t13_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=33; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t13_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t13_s4_done; fight=true

## 65. Lantern Tax - Reckoning (`evt_hand_t13_s5`)
- Requires: hand_t13_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=29; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_iron_heart; flags=hand_t13_s5_done,hand_t13_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t13_s5_done,hand_t13_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=38; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=tax lantern lens; talents=event_iron_heart; flags=hand_t13_s5_done,hand_t13_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t13_s5_done,hand_t13_complete; fight=true

## 66. Verdigris Choir - Whisper (`evt_hand_t14_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=14; xp_delta=16; flags=hand_t14_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t14_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=19; xp_delta=18; item=verdigris tuning fork; flags=hand_t14_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t14_s1_done; fight=true

## 67. Verdigris Choir - Trace (`evt_hand_t14_s2`)
- Requires: hand_t14_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=18; xp_delta=22; item=verdigris tuning fork; flags=hand_t14_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t14_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=24; xp_delta=24; flags=hand_t14_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t14_s2_done; fight=true

## 68. Verdigris Choir - Crossroads (`evt_hand_t14_s3`)
- Requires: hand_t14_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=22; xp_delta=28; bp_delta=1; flags=hand_t14_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t14_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=29; xp_delta=30; honor_delta=1; item=verdigris tuning fork; flags=hand_t14_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t14_s3_done; fight=true

## 69. Verdigris Choir - Siege (`evt_hand_t14_s4`)
- Requires: hand_t14_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=26; xp_delta=34; lp_delta=1; item=verdigris tuning fork; flags=hand_t14_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t14_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=34; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t14_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t14_s4_done; fight=true

## 70. Verdigris Choir - Reckoning (`evt_hand_t14_s5`)
- Requires: hand_t14_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=30; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_ashen_stride@Short sword; flags=hand_t14_s5_done,hand_t14_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t14_s5_done,hand_t14_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=39; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=verdigris tuning fork; talents=event_ashen_stride@Short sword; flags=hand_t14_s5_done,hand_t14_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t14_s5_done,hand_t14_complete; fight=true

## 71. Amber Gaol - Whisper (`evt_hand_t15_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=15; xp_delta=16; flags=hand_t15_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t15_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=15; xp_delta=18; item=amber lock tongue; flags=hand_t15_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t15_s1_done; fight=true

## 72. Amber Gaol - Trace (`evt_hand_t15_s2`)
- Requires: hand_t15_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=19; xp_delta=22; item=amber lock tongue; flags=hand_t15_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t15_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=20; xp_delta=24; flags=hand_t15_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t15_s2_done; fight=true

## 73. Amber Gaol - Crossroads (`evt_hand_t15_s3`)
- Requires: hand_t15_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=23; xp_delta=28; bp_delta=1; flags=hand_t15_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t15_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=25; xp_delta=30; honor_delta=1; item=amber lock tongue; flags=hand_t15_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t15_s3_done; fight=true

## 74. Amber Gaol - Siege (`evt_hand_t15_s4`)
- Requires: hand_t15_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=27; xp_delta=34; lp_delta=1; item=amber lock tongue; flags=hand_t15_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t15_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=30; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t15_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t15_s4_done; fight=true

## 75. Amber Gaol - Reckoning (`evt_hand_t15_s5`)
- Requires: hand_t15_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=31; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_quickened_sight; flags=hand_t15_s5_done,hand_t15_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t15_s5_done,hand_t15_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=35; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=amber lock tongue; talents=event_quickened_sight; flags=hand_t15_s5_done,hand_t15_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t15_s5_done,hand_t15_complete; fight=true

## 76. Cobalt Orchard - Whisper (`evt_hand_t16_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=12; xp_delta=16; flags=hand_t16_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t16_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=16; xp_delta=18; item=cobalt orchard stone; flags=hand_t16_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t16_s1_done; fight=true

## 77. Cobalt Orchard - Trace (`evt_hand_t16_s2`)
- Requires: hand_t16_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=16; xp_delta=22; item=cobalt orchard stone; flags=hand_t16_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t16_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=21; xp_delta=24; flags=hand_t16_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t16_s2_done; fight=true

## 78. Cobalt Orchard - Crossroads (`evt_hand_t16_s3`)
- Requires: hand_t16_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=20; xp_delta=28; bp_delta=1; flags=hand_t16_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t16_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=26; xp_delta=30; honor_delta=1; item=cobalt orchard stone; flags=hand_t16_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t16_s3_done; fight=true

## 79. Cobalt Orchard - Siege (`evt_hand_t16_s4`)
- Requires: hand_t16_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=24; xp_delta=34; lp_delta=1; item=cobalt orchard stone; flags=hand_t16_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t16_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=31; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t16_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t16_s4_done; fight=true

## 80. Cobalt Orchard - Reckoning (`evt_hand_t16_s5`)
- Requires: hand_t16_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=28; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_stoneguard; flags=hand_t16_s5_done,hand_t16_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t16_s5_done,hand_t16_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=36; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=cobalt orchard stone; talents=event_stoneguard; flags=hand_t16_s5_done,hand_t16_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t16_s5_done,hand_t16_complete; fight=true

## 81. Moth Archive - Whisper (`evt_hand_t17_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=13; xp_delta=16; flags=hand_t17_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t17_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=17; xp_delta=18; item=moth-bitten index; flags=hand_t17_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t17_s1_done; fight=true

## 82. Moth Archive - Trace (`evt_hand_t17_s2`)
- Requires: hand_t17_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=17; xp_delta=22; item=moth-bitten index; flags=hand_t17_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t17_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=22; xp_delta=24; flags=hand_t17_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t17_s2_done; fight=true

## 83. Moth Archive - Crossroads (`evt_hand_t17_s3`)
- Requires: hand_t17_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=21; xp_delta=28; bp_delta=1; flags=hand_t17_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t17_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=27; xp_delta=30; honor_delta=1; item=moth-bitten index; flags=hand_t17_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t17_s3_done; fight=true

## 84. Moth Archive - Siege (`evt_hand_t17_s4`)
- Requires: hand_t17_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=25; xp_delta=34; lp_delta=1; item=moth-bitten index; flags=hand_t17_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t17_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=32; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t17_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t17_s4_done; fight=true

## 85. Moth Archive - Reckoning (`evt_hand_t17_s5`)
- Requires: hand_t17_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=29; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_river_footing@Longsword; flags=hand_t17_s5_done,hand_t17_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t17_s5_done,hand_t17_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=37; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=moth-bitten index; talents=event_river_footing@Longsword; flags=hand_t17_s5_done,hand_t17_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t17_s5_done,hand_t17_complete; fight=true

## 86. Thorn Mint - Whisper (`evt_hand_t18_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=14; xp_delta=16; flags=hand_t18_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t18_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=18; xp_delta=18; item=thorn mint blank; flags=hand_t18_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t18_s1_done; fight=true

## 87. Thorn Mint - Trace (`evt_hand_t18_s2`)
- Requires: hand_t18_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=18; xp_delta=22; item=thorn mint blank; flags=hand_t18_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t18_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=23; xp_delta=24; flags=hand_t18_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t18_s2_done; fight=true

## 88. Thorn Mint - Crossroads (`evt_hand_t18_s3`)
- Requires: hand_t18_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=22; xp_delta=28; bp_delta=1; flags=hand_t18_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t18_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=28; xp_delta=30; honor_delta=1; item=thorn mint blank; flags=hand_t18_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t18_s3_done; fight=true

## 89. Thorn Mint - Siege (`evt_hand_t18_s4`)
- Requires: hand_t18_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=26; xp_delta=34; lp_delta=1; item=thorn mint blank; flags=hand_t18_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t18_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=33; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t18_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t18_s4_done; fight=true

## 90. Thorn Mint - Reckoning (`evt_hand_t18_s5`)
- Requires: hand_t18_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=30; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_blooded_edge@Spear; flags=hand_t18_s5_done,hand_t18_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t18_s5_done,hand_t18_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=38; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=thorn mint blank; talents=event_blooded_edge@Spear; flags=hand_t18_s5_done,hand_t18_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t18_s5_done,hand_t18_complete; fight=true

## 91. Brass Wake - Whisper (`evt_hand_t19_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=15; xp_delta=16; flags=hand_t19_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t19_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=19; xp_delta=18; item=wake ferryman coin; flags=hand_t19_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t19_s1_done; fight=true

## 92. Brass Wake - Trace (`evt_hand_t19_s2`)
- Requires: hand_t19_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=19; xp_delta=22; item=wake ferryman coin; flags=hand_t19_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t19_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=24; xp_delta=24; flags=hand_t19_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t19_s2_done; fight=true

## 93. Brass Wake - Crossroads (`evt_hand_t19_s3`)
- Requires: hand_t19_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=23; xp_delta=28; bp_delta=1; flags=hand_t19_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t19_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=29; xp_delta=30; honor_delta=1; item=wake ferryman coin; flags=hand_t19_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t19_s3_done; fight=true

## 94. Brass Wake - Siege (`evt_hand_t19_s4`)
- Requires: hand_t19_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=27; xp_delta=34; lp_delta=1; item=wake ferryman coin; flags=hand_t19_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t19_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=34; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t19_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t19_s4_done; fight=true

## 95. Brass Wake - Reckoning (`evt_hand_t19_s5`)
- Requires: hand_t19_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=31; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_storm_arm@Battle axe; flags=hand_t19_s5_done,hand_t19_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t19_s5_done,hand_t19_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=39; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=wake ferryman coin; talents=event_storm_arm@Battle axe; flags=hand_t19_s5_done,hand_t19_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t19_s5_done,hand_t19_complete; fight=true

## 96. Pale Menagerie - Whisper (`evt_hand_t20_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=12; xp_delta=16; flags=hand_t20_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t20_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=15; xp_delta=18; item=menagerie collar; flags=hand_t20_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t20_s1_done; fight=true

## 97. Pale Menagerie - Trace (`evt_hand_t20_s2`)
- Requires: hand_t20_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=16; xp_delta=22; item=menagerie collar; flags=hand_t20_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t20_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=20; xp_delta=24; flags=hand_t20_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t20_s2_done; fight=true

## 98. Pale Menagerie - Crossroads (`evt_hand_t20_s3`)
- Requires: hand_t20_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=20; xp_delta=28; bp_delta=1; flags=hand_t20_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t20_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=25; xp_delta=30; honor_delta=1; item=menagerie collar; flags=hand_t20_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t20_s3_done; fight=true

## 99. Pale Menagerie - Siege (`evt_hand_t20_s4`)
- Requires: hand_t20_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=24; xp_delta=34; lp_delta=1; item=menagerie collar; flags=hand_t20_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t20_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=30; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t20_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t20_s4_done; fight=true

## 100. Pale Menagerie - Reckoning (`evt_hand_t20_s5`)
- Requires: hand_t20_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=28; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_far_mark; flags=hand_t20_s5_done,hand_t20_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t20_s5_done,hand_t20_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=35; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=menagerie collar; talents=event_far_mark; flags=hand_t20_s5_done,hand_t20_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t20_s5_done,hand_t20_complete; fight=true

## 101. Iron Wake - Whisper (`evt_hand_t21_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=13; xp_delta=16; flags=hand_t21_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t21_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=16; xp_delta=18; item=wake rivet; flags=hand_t21_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t21_s1_done; fight=true

## 102. Iron Wake - Trace (`evt_hand_t21_s2`)
- Requires: hand_t21_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=17; xp_delta=22; item=wake rivet; flags=hand_t21_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t21_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=21; xp_delta=24; flags=hand_t21_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t21_s2_done; fight=true

## 103. Iron Wake - Crossroads (`evt_hand_t21_s3`)
- Requires: hand_t21_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=21; xp_delta=28; bp_delta=1; flags=hand_t21_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t21_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=26; xp_delta=30; honor_delta=1; item=wake rivet; flags=hand_t21_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t21_s3_done; fight=true

## 104. Iron Wake - Siege (`evt_hand_t21_s4`)
- Requires: hand_t21_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=25; xp_delta=34; lp_delta=1; item=wake rivet; flags=hand_t21_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t21_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=31; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t21_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t21_s4_done; fight=true

## 105. Iron Wake - Reckoning (`evt_hand_t21_s5`)
- Requires: hand_t21_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=29; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_steady_guard; flags=hand_t21_s5_done,hand_t21_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t21_s5_done,hand_t21_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=36; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=wake rivet; talents=event_steady_guard; flags=hand_t21_s5_done,hand_t21_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t21_s5_done,hand_t21_complete; fight=true

## 106. Dust Chancery - Whisper (`evt_hand_t22_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=14; xp_delta=16; flags=hand_t22_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t22_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=17; xp_delta=18; item=chancery docket shard; flags=hand_t22_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t22_s1_done; fight=true

## 107. Dust Chancery - Trace (`evt_hand_t22_s2`)
- Requires: hand_t22_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=18; xp_delta=22; item=chancery docket shard; flags=hand_t22_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t22_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=22; xp_delta=24; flags=hand_t22_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t22_s2_done; fight=true

## 108. Dust Chancery - Crossroads (`evt_hand_t22_s3`)
- Requires: hand_t22_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=22; xp_delta=28; bp_delta=1; flags=hand_t22_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t22_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=27; xp_delta=30; honor_delta=1; item=chancery docket shard; flags=hand_t22_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t22_s3_done; fight=true

## 109. Dust Chancery - Siege (`evt_hand_t22_s4`)
- Requires: hand_t22_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=26; xp_delta=34; lp_delta=1; item=chancery docket shard; flags=hand_t22_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t22_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=32; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t22_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t22_s4_done; fight=true

## 110. Dust Chancery - Reckoning (`evt_hand_t22_s5`)
- Requires: hand_t22_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=30; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_fast_healing; flags=hand_t22_s5_done,hand_t22_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t22_s5_done,hand_t22_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=37; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=chancery docket shard; talents=event_fast_healing; flags=hand_t22_s5_done,hand_t22_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t22_s5_done,hand_t22_complete; fight=true

## 111. Crow Reliquary - Whisper (`evt_hand_t23_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=15; xp_delta=16; flags=hand_t23_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t23_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=18; xp_delta=18; item=crow reliquary pin; flags=hand_t23_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t23_s1_done; fight=true

## 112. Crow Reliquary - Trace (`evt_hand_t23_s2`)
- Requires: hand_t23_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=19; xp_delta=22; item=crow reliquary pin; flags=hand_t23_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t23_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=23; xp_delta=24; flags=hand_t23_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t23_s2_done; fight=true

## 113. Crow Reliquary - Crossroads (`evt_hand_t23_s3`)
- Requires: hand_t23_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=23; xp_delta=28; bp_delta=1; flags=hand_t23_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t23_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=28; xp_delta=30; honor_delta=1; item=crow reliquary pin; flags=hand_t23_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t23_s3_done; fight=true

## 114. Crow Reliquary - Siege (`evt_hand_t23_s4`)
- Requires: hand_t23_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=27; xp_delta=34; lp_delta=1; item=crow reliquary pin; flags=hand_t23_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t23_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=33; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t23_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t23_s4_done; fight=true

## 115. Crow Reliquary - Reckoning (`evt_hand_t23_s5`)
- Requires: hand_t23_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=31; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_braced_reach@Greatsword; flags=hand_t23_s5_done,hand_t23_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t23_s5_done,hand_t23_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=38; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=crow reliquary pin; talents=event_braced_reach@Greatsword; flags=hand_t23_s5_done,hand_t23_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t23_s5_done,hand_t23_complete; fight=true

## 116. Sable Cart - Whisper (`evt_hand_t24_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=12; xp_delta=16; flags=hand_t24_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t24_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=19; xp_delta=18; item=sable axle token; flags=hand_t24_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t24_s1_done; fight=true

## 117. Sable Cart - Trace (`evt_hand_t24_s2`)
- Requires: hand_t24_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=16; xp_delta=22; item=sable axle token; flags=hand_t24_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t24_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=24; xp_delta=24; flags=hand_t24_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t24_s2_done; fight=true

## 118. Sable Cart - Crossroads (`evt_hand_t24_s3`)
- Requires: hand_t24_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=20; xp_delta=28; bp_delta=1; flags=hand_t24_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t24_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=29; xp_delta=30; honor_delta=1; item=sable axle token; flags=hand_t24_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t24_s3_done; fight=true

## 119. Sable Cart - Siege (`evt_hand_t24_s4`)
- Requires: hand_t24_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=24; xp_delta=34; lp_delta=1; item=sable axle token; flags=hand_t24_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t24_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=34; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t24_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t24_s4_done; fight=true

## 120. Sable Cart - Reckoning (`evt_hand_t24_s5`)
- Requires: hand_t24_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=28; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_hardened_plate; flags=hand_t24_s5_done,hand_t24_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t24_s5_done,hand_t24_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=39; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=sable axle token; talents=event_hardened_plate; flags=hand_t24_s5_done,hand_t24_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t24_s5_done,hand_t24_complete; fight=true

## 121. Varnish Court - Whisper (`evt_hand_t25_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=13; xp_delta=16; flags=hand_t25_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t25_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=15; xp_delta=18; item=varnish court stamp; flags=hand_t25_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t25_s1_done; fight=true

## 122. Varnish Court - Trace (`evt_hand_t25_s2`)
- Requires: hand_t25_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=17; xp_delta=22; item=varnish court stamp; flags=hand_t25_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t25_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=20; xp_delta=24; flags=hand_t25_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t25_s2_done; fight=true

## 123. Varnish Court - Crossroads (`evt_hand_t25_s3`)
- Requires: hand_t25_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=21; xp_delta=28; bp_delta=1; flags=hand_t25_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t25_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=25; xp_delta=30; honor_delta=1; item=varnish court stamp; flags=hand_t25_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t25_s3_done; fight=true

## 124. Varnish Court - Siege (`evt_hand_t25_s4`)
- Requires: hand_t25_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=25; xp_delta=34; lp_delta=1; item=varnish court stamp; flags=hand_t25_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t25_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=30; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t25_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t25_s4_done; fight=true

## 125. Varnish Court - Reckoning (`evt_hand_t25_s5`)
- Requires: hand_t25_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=29; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_iron_heart; flags=hand_t25_s5_done,hand_t25_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t25_s5_done,hand_t25_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=35; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=varnish court stamp; talents=event_iron_heart; flags=hand_t25_s5_done,hand_t25_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t25_s5_done,hand_t25_complete; fight=true

## 126. Soot Embassy - Whisper (`evt_hand_t26_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=14; xp_delta=16; flags=hand_t26_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t26_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=16; xp_delta=18; item=embassy pass shard; flags=hand_t26_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t26_s1_done; fight=true

## 127. Soot Embassy - Trace (`evt_hand_t26_s2`)
- Requires: hand_t26_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=18; xp_delta=22; item=embassy pass shard; flags=hand_t26_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t26_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=21; xp_delta=24; flags=hand_t26_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t26_s2_done; fight=true

## 128. Soot Embassy - Crossroads (`evt_hand_t26_s3`)
- Requires: hand_t26_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=22; xp_delta=28; bp_delta=1; flags=hand_t26_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t26_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=26; xp_delta=30; honor_delta=1; item=embassy pass shard; flags=hand_t26_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t26_s3_done; fight=true

## 129. Soot Embassy - Siege (`evt_hand_t26_s4`)
- Requires: hand_t26_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=26; xp_delta=34; lp_delta=1; item=embassy pass shard; flags=hand_t26_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t26_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=31; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t26_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t26_s4_done; fight=true

## 130. Soot Embassy - Reckoning (`evt_hand_t26_s5`)
- Requires: hand_t26_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=30; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_ashen_stride@Spear; flags=hand_t26_s5_done,hand_t26_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t26_s5_done,hand_t26_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=36; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=embassy pass shard; talents=event_ashen_stride@Spear; flags=hand_t26_s5_done,hand_t26_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t26_s5_done,hand_t26_complete; fight=true

## 131. Stone Verdict - Whisper (`evt_hand_t27_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=15; xp_delta=16; flags=hand_t27_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t27_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=17; xp_delta=18; item=verdict pebble; flags=hand_t27_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t27_s1_done; fight=true

## 132. Stone Verdict - Trace (`evt_hand_t27_s2`)
- Requires: hand_t27_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=19; xp_delta=22; item=verdict pebble; flags=hand_t27_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t27_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=22; xp_delta=24; flags=hand_t27_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t27_s2_done; fight=true

## 133. Stone Verdict - Crossroads (`evt_hand_t27_s3`)
- Requires: hand_t27_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=23; xp_delta=28; bp_delta=1; flags=hand_t27_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t27_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=27; xp_delta=30; honor_delta=1; item=verdict pebble; flags=hand_t27_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t27_s3_done; fight=true

## 134. Stone Verdict - Siege (`evt_hand_t27_s4`)
- Requires: hand_t27_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=27; xp_delta=34; lp_delta=1; item=verdict pebble; flags=hand_t27_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t27_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=32; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t27_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t27_s4_done; fight=true

## 135. Stone Verdict - Reckoning (`evt_hand_t27_s5`)
- Requires: hand_t27_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=31; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_quickened_sight; flags=hand_t27_s5_done,hand_t27_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t27_s5_done,hand_t27_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=37; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=verdict pebble; talents=event_quickened_sight; flags=hand_t27_s5_done,hand_t27_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t27_s5_done,hand_t27_complete; fight=true

## 136. Wren Battery - Whisper (`evt_hand_t28_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=12; xp_delta=16; flags=hand_t28_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t28_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=18; xp_delta=18; item=battery firing wedge; flags=hand_t28_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t28_s1_done; fight=true

## 137. Wren Battery - Trace (`evt_hand_t28_s2`)
- Requires: hand_t28_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=16; xp_delta=22; item=battery firing wedge; flags=hand_t28_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t28_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=23; xp_delta=24; flags=hand_t28_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t28_s2_done; fight=true

## 138. Wren Battery - Crossroads (`evt_hand_t28_s3`)
- Requires: hand_t28_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=20; xp_delta=28; bp_delta=1; flags=hand_t28_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t28_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=28; xp_delta=30; honor_delta=1; item=battery firing wedge; flags=hand_t28_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t28_s3_done; fight=true

## 139. Wren Battery - Siege (`evt_hand_t28_s4`)
- Requires: hand_t28_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=24; xp_delta=34; lp_delta=1; item=battery firing wedge; flags=hand_t28_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t28_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=33; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t28_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t28_s4_done; fight=true

## 140. Wren Battery - Reckoning (`evt_hand_t28_s5`)
- Requires: hand_t28_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=28; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_stoneguard; flags=hand_t28_s5_done,hand_t28_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t28_s5_done,hand_t28_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=38; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=battery firing wedge; talents=event_stoneguard; flags=hand_t28_s5_done,hand_t28_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t28_s5_done,hand_t28_complete; fight=true

## 141. Bleached Meridian - Whisper (`evt_hand_t29_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=13; xp_delta=16; flags=hand_t29_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t29_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=19; xp_delta=18; item=meridian survey pin; flags=hand_t29_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t29_s1_done; fight=true

## 142. Bleached Meridian - Trace (`evt_hand_t29_s2`)
- Requires: hand_t29_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=17; xp_delta=22; item=meridian survey pin; flags=hand_t29_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t29_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=24; xp_delta=24; flags=hand_t29_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t29_s2_done; fight=true

## 143. Bleached Meridian - Crossroads (`evt_hand_t29_s3`)
- Requires: hand_t29_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=21; xp_delta=28; bp_delta=1; flags=hand_t29_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t29_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=29; xp_delta=30; honor_delta=1; item=meridian survey pin; flags=hand_t29_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t29_s3_done; fight=true

## 144. Bleached Meridian - Siege (`evt_hand_t29_s4`)
- Requires: hand_t29_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=25; xp_delta=34; lp_delta=1; item=meridian survey pin; flags=hand_t29_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t29_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=34; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t29_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t29_s4_done; fight=true

## 145. Bleached Meridian - Reckoning (`evt_hand_t29_s5`)
- Requires: hand_t29_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=29; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_river_footing@Dagger; flags=hand_t29_s5_done,hand_t29_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t29_s5_done,hand_t29_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=39; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=meridian survey pin; talents=event_river_footing@Dagger; flags=hand_t29_s5_done,hand_t29_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t29_s5_done,hand_t29_complete; fight=true

## 146. Ashglass Market - Whisper (`evt_hand_t30_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=14; xp_delta=16; flags=hand_t30_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t30_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=15; xp_delta=18; item=ashglass trade chit; flags=hand_t30_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t30_s1_done; fight=true

## 147. Ashglass Market - Trace (`evt_hand_t30_s2`)
- Requires: hand_t30_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=18; xp_delta=22; item=ashglass trade chit; flags=hand_t30_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t30_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=20; xp_delta=24; flags=hand_t30_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t30_s2_done; fight=true

## 148. Ashglass Market - Crossroads (`evt_hand_t30_s3`)
- Requires: hand_t30_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=22; xp_delta=28; bp_delta=1; flags=hand_t30_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t30_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=25; xp_delta=30; honor_delta=1; item=ashglass trade chit; flags=hand_t30_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t30_s3_done; fight=true

## 149. Ashglass Market - Siege (`evt_hand_t30_s4`)
- Requires: hand_t30_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=26; xp_delta=34; lp_delta=1; item=ashglass trade chit; flags=hand_t30_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t30_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=30; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t30_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t30_s4_done; fight=true

## 150. Ashglass Market - Reckoning (`evt_hand_t30_s5`)
- Requires: hand_t30_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=30; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_blooded_edge@Short sword; flags=hand_t30_s5_done,hand_t30_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t30_s5_done,hand_t30_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=35; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=ashglass trade chit; talents=event_blooded_edge@Short sword; flags=hand_t30_s5_done,hand_t30_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t30_s5_done,hand_t30_complete; fight=true

## 151. Bramble Oath - Whisper (`evt_hand_t31_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=15; xp_delta=16; flags=hand_t31_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t31_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=16; xp_delta=18; item=bramble oath cord; flags=hand_t31_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t31_s1_done; fight=true

## 152. Bramble Oath - Trace (`evt_hand_t31_s2`)
- Requires: hand_t31_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=19; xp_delta=22; item=bramble oath cord; flags=hand_t31_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t31_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=21; xp_delta=24; flags=hand_t31_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t31_s2_done; fight=true

## 153. Bramble Oath - Crossroads (`evt_hand_t31_s3`)
- Requires: hand_t31_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=23; xp_delta=28; bp_delta=1; flags=hand_t31_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t31_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=26; xp_delta=30; honor_delta=1; item=bramble oath cord; flags=hand_t31_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t31_s3_done; fight=true

## 154. Bramble Oath - Siege (`evt_hand_t31_s4`)
- Requires: hand_t31_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=27; xp_delta=34; lp_delta=1; item=bramble oath cord; flags=hand_t31_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t31_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=31; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t31_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t31_s4_done; fight=true

## 155. Bramble Oath - Reckoning (`evt_hand_t31_s5`)
- Requires: hand_t31_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=31; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_storm_arm@Greatsword; flags=hand_t31_s5_done,hand_t31_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t31_s5_done,hand_t31_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=36; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=bramble oath cord; talents=event_storm_arm@Greatsword; flags=hand_t31_s5_done,hand_t31_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t31_s5_done,hand_t31_complete; fight=true

## 156. Moss Tribunal - Whisper (`evt_hand_t32_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=12; xp_delta=16; flags=hand_t32_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t32_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=17; xp_delta=18; item=moss verdict token; flags=hand_t32_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t32_s1_done; fight=true

## 157. Moss Tribunal - Trace (`evt_hand_t32_s2`)
- Requires: hand_t32_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=16; xp_delta=22; item=moss verdict token; flags=hand_t32_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t32_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=22; xp_delta=24; flags=hand_t32_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t32_s2_done; fight=true

## 158. Moss Tribunal - Crossroads (`evt_hand_t32_s3`)
- Requires: hand_t32_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=20; xp_delta=28; bp_delta=1; flags=hand_t32_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t32_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=27; xp_delta=30; honor_delta=1; item=moss verdict token; flags=hand_t32_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t32_s3_done; fight=true

## 159. Moss Tribunal - Siege (`evt_hand_t32_s4`)
- Requires: hand_t32_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=24; xp_delta=34; lp_delta=1; item=moss verdict token; flags=hand_t32_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t32_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=32; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t32_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t32_s4_done; fight=true

## 160. Moss Tribunal - Reckoning (`evt_hand_t32_s5`)
- Requires: hand_t32_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=28; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_far_mark; flags=hand_t32_s5_done,hand_t32_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t32_s5_done,hand_t32_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=37; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=moss verdict token; talents=event_far_mark; flags=hand_t32_s5_done,hand_t32_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t32_s5_done,hand_t32_complete; fight=true

## 161. Tithe of Keys - Whisper (`evt_hand_t33_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=13; xp_delta=16; flags=hand_t33_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t33_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=18; xp_delta=18; item=tithe key sliver; flags=hand_t33_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t33_s1_done; fight=true

## 162. Tithe of Keys - Trace (`evt_hand_t33_s2`)
- Requires: hand_t33_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=17; xp_delta=22; item=tithe key sliver; flags=hand_t33_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t33_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=23; xp_delta=24; flags=hand_t33_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t33_s2_done; fight=true

## 163. Tithe of Keys - Crossroads (`evt_hand_t33_s3`)
- Requires: hand_t33_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=21; xp_delta=28; bp_delta=1; flags=hand_t33_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t33_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=28; xp_delta=30; honor_delta=1; item=tithe key sliver; flags=hand_t33_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t33_s3_done; fight=true

## 164. Tithe of Keys - Siege (`evt_hand_t33_s4`)
- Requires: hand_t33_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=25; xp_delta=34; lp_delta=1; item=tithe key sliver; flags=hand_t33_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t33_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=33; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t33_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t33_s4_done; fight=true

## 165. Tithe of Keys - Reckoning (`evt_hand_t33_s5`)
- Requires: hand_t33_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=29; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_steady_guard; flags=hand_t33_s5_done,hand_t33_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t33_s5_done,hand_t33_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=38; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=tithe key sliver; talents=event_steady_guard; flags=hand_t33_s5_done,hand_t33_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t33_s5_done,hand_t33_complete; fight=true

## 166. Gray Aquarium - Whisper (`evt_hand_t34_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=14; xp_delta=16; flags=hand_t34_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t34_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=19; xp_delta=18; item=aquarium valve wheel; flags=hand_t34_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t34_s1_done; fight=true

## 167. Gray Aquarium - Trace (`evt_hand_t34_s2`)
- Requires: hand_t34_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=18; xp_delta=22; item=aquarium valve wheel; flags=hand_t34_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t34_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=24; xp_delta=24; flags=hand_t34_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t34_s2_done; fight=true

## 168. Gray Aquarium - Crossroads (`evt_hand_t34_s3`)
- Requires: hand_t34_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=22; xp_delta=28; bp_delta=1; flags=hand_t34_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t34_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=29; xp_delta=30; honor_delta=1; item=aquarium valve wheel; flags=hand_t34_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t34_s3_done; fight=true

## 169. Gray Aquarium - Siege (`evt_hand_t34_s4`)
- Requires: hand_t34_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=26; xp_delta=34; lp_delta=1; item=aquarium valve wheel; flags=hand_t34_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t34_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=34; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t34_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t34_s4_done; fight=true

## 170. Gray Aquarium - Reckoning (`evt_hand_t34_s5`)
- Requires: hand_t34_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=30; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_fast_healing; flags=hand_t34_s5_done,hand_t34_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t34_s5_done,hand_t34_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=39; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=aquarium valve wheel; talents=event_fast_healing; flags=hand_t34_s5_done,hand_t34_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t34_s5_done,hand_t34_complete; fight=true

## 171. Serpent Courier - Whisper (`evt_hand_t35_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=15; xp_delta=16; flags=hand_t35_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t35_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=15; xp_delta=18; item=courier cipher ring; flags=hand_t35_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t35_s1_done; fight=true

## 172. Serpent Courier - Trace (`evt_hand_t35_s2`)
- Requires: hand_t35_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=19; xp_delta=22; item=courier cipher ring; flags=hand_t35_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t35_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=20; xp_delta=24; flags=hand_t35_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t35_s2_done; fight=true

## 173. Serpent Courier - Crossroads (`evt_hand_t35_s3`)
- Requires: hand_t35_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=23; xp_delta=28; bp_delta=1; flags=hand_t35_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t35_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=25; xp_delta=30; honor_delta=1; item=courier cipher ring; flags=hand_t35_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t35_s3_done; fight=true

## 174. Serpent Courier - Siege (`evt_hand_t35_s4`)
- Requires: hand_t35_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=27; xp_delta=34; lp_delta=1; item=courier cipher ring; flags=hand_t35_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t35_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=30; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t35_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t35_s4_done; fight=true

## 175. Serpent Courier - Reckoning (`evt_hand_t35_s5`)
- Requires: hand_t35_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=31; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_braced_reach@Battle axe; flags=hand_t35_s5_done,hand_t35_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t35_s5_done,hand_t35_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=35; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=courier cipher ring; talents=event_braced_reach@Battle axe; flags=hand_t35_s5_done,hand_t35_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t35_s5_done,hand_t35_complete; fight=true

## 176. Tallow Citadel - Whisper (`evt_hand_t36_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=12; xp_delta=16; flags=hand_t36_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t36_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=16; xp_delta=18; item=citadel drip mold; flags=hand_t36_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t36_s1_done; fight=true

## 177. Tallow Citadel - Trace (`evt_hand_t36_s2`)
- Requires: hand_t36_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=16; xp_delta=22; item=citadel drip mold; flags=hand_t36_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t36_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=21; xp_delta=24; flags=hand_t36_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t36_s2_done; fight=true

## 178. Tallow Citadel - Crossroads (`evt_hand_t36_s3`)
- Requires: hand_t36_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=20; xp_delta=28; bp_delta=1; flags=hand_t36_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t36_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=26; xp_delta=30; honor_delta=1; item=citadel drip mold; flags=hand_t36_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t36_s3_done; fight=true

## 179. Tallow Citadel - Siege (`evt_hand_t36_s4`)
- Requires: hand_t36_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=24; xp_delta=34; lp_delta=1; item=citadel drip mold; flags=hand_t36_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t36_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=31; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t36_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t36_s4_done; fight=true

## 180. Tallow Citadel - Reckoning (`evt_hand_t36_s5`)
- Requires: hand_t36_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=28; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_hardened_plate; flags=hand_t36_s5_done,hand_t36_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t36_s5_done,hand_t36_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=36; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=citadel drip mold; talents=event_hardened_plate; flags=hand_t36_s5_done,hand_t36_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t36_s5_done,hand_t36_complete; fight=true

## 181. Silt Monastery - Whisper (`evt_hand_t37_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=13; xp_delta=16; flags=hand_t37_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t37_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=17; xp_delta=18; item=silt monk bead; flags=hand_t37_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t37_s1_done; fight=true

## 182. Silt Monastery - Trace (`evt_hand_t37_s2`)
- Requires: hand_t37_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=17; xp_delta=22; item=silt monk bead; flags=hand_t37_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t37_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=22; xp_delta=24; flags=hand_t37_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t37_s2_done; fight=true

## 183. Silt Monastery - Crossroads (`evt_hand_t37_s3`)
- Requires: hand_t37_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=21; xp_delta=28; bp_delta=1; flags=hand_t37_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t37_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=27; xp_delta=30; honor_delta=1; item=silt monk bead; flags=hand_t37_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t37_s3_done; fight=true

## 184. Silt Monastery - Siege (`evt_hand_t37_s4`)
- Requires: hand_t37_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=25; xp_delta=34; lp_delta=1; item=silt monk bead; flags=hand_t37_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t37_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=32; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t37_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t37_s4_done; fight=true

## 185. Silt Monastery - Reckoning (`evt_hand_t37_s5`)
- Requires: hand_t37_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=29; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_iron_heart; flags=hand_t37_s5_done,hand_t37_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t37_s5_done,hand_t37_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=37; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=silt monk bead; talents=event_iron_heart; flags=hand_t37_s5_done,hand_t37_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t37_s5_done,hand_t37_complete; fight=true

## 186. Moonwell Bailiff - Whisper (`evt_hand_t38_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=14; xp_delta=16; flags=hand_t38_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t38_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=18; xp_delta=18; item=bailiff moonstamp; flags=hand_t38_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t38_s1_done; fight=true

## 187. Moonwell Bailiff - Trace (`evt_hand_t38_s2`)
- Requires: hand_t38_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=18; xp_delta=22; item=bailiff moonstamp; flags=hand_t38_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t38_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=23; xp_delta=24; flags=hand_t38_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t38_s2_done; fight=true

## 188. Moonwell Bailiff - Crossroads (`evt_hand_t38_s3`)
- Requires: hand_t38_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=22; xp_delta=28; bp_delta=1; flags=hand_t38_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t38_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=28; xp_delta=30; honor_delta=1; item=bailiff moonstamp; flags=hand_t38_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t38_s3_done; fight=true

## 189. Moonwell Bailiff - Siege (`evt_hand_t38_s4`)
- Requires: hand_t38_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=26; xp_delta=34; lp_delta=1; item=bailiff moonstamp; flags=hand_t38_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t38_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=33; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t38_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t38_s4_done; fight=true

## 190. Moonwell Bailiff - Reckoning (`evt_hand_t38_s5`)
- Requires: hand_t38_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=30; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_ashen_stride@Short sword; flags=hand_t38_s5_done,hand_t38_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t38_s5_done,hand_t38_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=38; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=bailiff moonstamp; talents=event_ashen_stride@Short sword; flags=hand_t38_s5_done,hand_t38_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t38_s5_done,hand_t38_complete; fight=true

## 191. Tin Basilica - Whisper (`evt_hand_t39_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=15; xp_delta=16; flags=hand_t39_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t39_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=19; xp_delta=18; item=tin basilica rivet; flags=hand_t39_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t39_s1_done; fight=true

## 192. Tin Basilica - Trace (`evt_hand_t39_s2`)
- Requires: hand_t39_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=19; xp_delta=22; item=tin basilica rivet; flags=hand_t39_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t39_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=24; xp_delta=24; flags=hand_t39_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t39_s2_done; fight=true

## 193. Tin Basilica - Crossroads (`evt_hand_t39_s3`)
- Requires: hand_t39_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=23; xp_delta=28; bp_delta=1; flags=hand_t39_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t39_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=29; xp_delta=30; honor_delta=1; item=tin basilica rivet; flags=hand_t39_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t39_s3_done; fight=true

## 194. Tin Basilica - Siege (`evt_hand_t39_s4`)
- Requires: hand_t39_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=27; xp_delta=34; lp_delta=1; item=tin basilica rivet; flags=hand_t39_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t39_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=34; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t39_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t39_s4_done; fight=true

## 195. Tin Basilica - Reckoning (`evt_hand_t39_s5`)
- Requires: hand_t39_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=31; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_quickened_sight; flags=hand_t39_s5_done,hand_t39_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t39_s5_done,hand_t39_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=39; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=tin basilica rivet; talents=event_quickened_sight; flags=hand_t39_s5_done,hand_t39_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t39_s5_done,hand_t39_complete; fight=true

## 196. Harbor of Teeth - Whisper (`evt_hand_t40_s1`)
- Requires: none
- Choice: Work contacts and verify details [medium wisdom]
  - Success: gold_delta=12; xp_delta=16; flags=hand_t40_s1_done
  - Failure: gold_delta=-3; add_wound=1; flags=hand_t40_s1_done
- Choice: Force momentum and seize control [medium strength]
  - Success: gold_delta=15; xp_delta=18; item=harbor tooth token; flags=hand_t40_s1_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t40_s1_done; fight=true

## 197. Harbor of Teeth - Trace (`evt_hand_t40_s2`)
- Requires: hand_t40_s1_done
- Choice: Work contacts and verify details [medium intelligence]
  - Success: gold_delta=16; xp_delta=22; item=harbor tooth token; flags=hand_t40_s2_done
  - Failure: gold_delta=-4; add_wound=1; flags=hand_t40_s2_done
- Choice: Force momentum and seize control [hard dexterity]
  - Success: gold_delta=20; xp_delta=24; flags=hand_t40_s2_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t40_s2_done; fight=true

## 198. Harbor of Teeth - Crossroads (`evt_hand_t40_s3`)
- Requires: hand_t40_s2_done
- Choice: Work contacts and verify details [hard charisma]
  - Success: gold_delta=20; xp_delta=28; bp_delta=1; flags=hand_t40_s3_done
  - Failure: gold_delta=-5; add_wound=2; flags=hand_t40_s3_done; fight=true
- Choice: Force momentum and seize control [hard constitution]
  - Success: gold_delta=25; xp_delta=30; honor_delta=1; item=harbor tooth token; flags=hand_t40_s3_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t40_s3_done; fight=true

## 199. Harbor of Teeth - Siege (`evt_hand_t40_s4`)
- Requires: hand_t40_s3_done
- Choice: Work contacts and verify details [hard dexterity]
  - Success: gold_delta=24; xp_delta=34; lp_delta=1; item=harbor tooth token; flags=hand_t40_s4_done
  - Failure: gold_delta=-6; add_wound=2; flags=hand_t40_s4_done; fight=true
- Choice: Force momentum and seize control [hard strength]
  - Success: gold_delta=30; xp_delta=36; bp_delta=1; honor_delta=1; flags=hand_t40_s4_done
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t40_s4_done; fight=true

## 200. Harbor of Teeth - Reckoning (`evt_hand_t40_s5`)
- Requires: hand_t40_s4_done
- Choice: Work contacts and verify details [hard wisdom]
  - Success: gold_delta=28; xp_delta=40; bp_delta=2; ap_delta=1; talents=event_stoneguard; flags=hand_t40_s5_done,hand_t40_complete
  - Failure: gold_delta=-7; add_wound=2; flags=hand_t40_s5_done,hand_t40_complete; fight=true
- Choice: Force momentum and seize control [very_hard strength]
  - Success: gold_delta=35; xp_delta=42; bp_delta=3; rp_delta=1; honor_delta=1; item=harbor tooth token; talents=event_stoneguard; flags=hand_t40_s5_done,hand_t40_complete
  - Failure: gold_delta=-8; add_wound=2; flags=hand_t40_s5_done,hand_t40_complete; fight=true
