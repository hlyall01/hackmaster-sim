use super::combat::{AttackMode, resolve_attack, resolve_style_strike};
use super::*;
use crate::core::rules::DamageExprCache;
use crate::core::types::TalentSelection;
use crate::{data, game_logic as logic};
use rand::{SeedableRng, rngs::StdRng};
use std::sync::Arc;

fn fixture(
    style: &str,
    primary: &str,
    secondary: Option<&str>,
    configure: impl FnOnce(&mut logic::PlayerConfig),
) -> Combatant {
    let weapons = data::load_weapon_catalog("data/sim/weapons.json").unwrap();
    let armor = data::load_armor_catalog("data/sim/armor.json").unwrap();
    let shields = data::load_shield_catalog("data/sim/weapons.json").unwrap();
    let talents = data::load_talents(data::TALENTS_PATH).unwrap();
    let weapon_id = |name: &str| {
        weapons
            .id_from_index(
                weapons
                    .entries()
                    .iter()
                    .position(|w| w.name == name)
                    .unwrap(),
            )
            .unwrap()
    };
    let mut player = logic::PlayerConfig::new("Style test", weapon_id(primary));
    player.weapon_id = weapon_id(primary);
    player.offhand_weapon_id = secondary.map(weapon_id);
    player.proficiencies = std::iter::once(primary)
        .chain(secondary)
        .map(str::to_string)
        .collect();
    player.talents.push(TalentSelection {
        id: style.into(),
        rank: 1,
        weapon: None,
    });
    player.default_weapon_style_ids = Some(vec![style.into()]);
    configure(&mut player);
    logic::build_combatant(
        &player,
        &weapons,
        &armor,
        &shields,
        &logic::NpcPresetCatalog::new(vec![]),
        &talents,
    )
}

fn target() -> Combatant {
    let mut actor = Combatant::default();
    actor.sheet.vitals.max_hp = 1000;
    actor.sheet.vitals.threshold_of_pain = 1000;
    actor.sheet.defense.defense_mod = 0;
    actor.sheet.defense.knockback_step = 10000;
    actor.sheet.defense.shield_name = None;
    actor.reset_state();
    actor
}

#[test]
fn quiet_river_shield_hits_halve_damage_and_bypass_shield_armor_and_natural_dr() {
    let attacker = fixture("quiet_river", "Fist", None, |p| {
        p.strength_base = 16;
        p.mastery_mut(crate::character::WeaponGroup::Unarmed).damage = 4;
    });
    assert_eq!(attacker.sheet.offense.weapon.shield_damage_expr.as_deref(), Some("max(1,d4p-2)"));
    for counter in [false, true] {
        for ignore_dr in [false, true] {
            let mut checked = 0;
            for seed in 0..200 {
                let mut striker = attacker.clone();
                Arc::make_mut(&mut striker.sheet.offense.weapon).ignore_all_dr = ignore_dr;
                let mut defender = target();
                defender.sheet.defense.shield_name = Some("Test shield".into());
                defender.sheet.defense.shield_defense_bonus = 3;
                defender.sheet.defense.shield_dr = 100;
                defender.sheet.defense.armor_dr = 50;
                defender.sheet.defense.natural_dr = 30;
                defender.reset_state();
                let mut actors = vec![striker, defender];
                let mut rng = StdRng::seed_from_u64(seed);
                let (damage, shield_damage, roll) = if counter {
                    let outcome = resolve_style_strike(&mut actors, 0, 1,
                        WeaponSlot::Primary, 1.0, 0.0, &mut rng).unwrap();
                    (outcome.damage, outcome.shield_damage_breakdown, outcome.roll)
                } else {
                    let outcome = resolve_attack(&mut actors, 0, 1, 0, false, 1.0,
                        AttackMode::Normal, WeaponSlot::Primary, 0.0, None, &mut rng);
                    (outcome.damage, outcome.shield_damage_breakdown, outcome.roll)
                };
                let Some(shield) = shield_damage else { continue; };
                assert_eq!(roll.shield_defense_bonus, 7, "Shield still contributes to defense");
                assert_eq!(shield.raw_damage,
                    ((shield.rolled_damage + shield.strength_damage) / 2).max(0));
                assert!(shield.raw_damage > 0, "Normal modifiers apply to the shield pool");
                if ignore_dr {
                    assert_eq!(shield.shield_dr, 0);
                    assert_eq!(shield.armor_dr, 0);
                    assert_eq!(shield.effective_armor_dr, 0);
                    assert_eq!(damage, shield.raw_damage);
                    assert_eq!(actors[1].state.hp, 1000 - damage);
                } else {
                    assert_eq!(shield.shield_dr, 100);
                    assert_eq!(shield.effective_armor_dr, 50);
                    assert_eq!(damage, 0);
                }
                checked += 1;
            }
            assert!(checked >= 10, "Exercise shield hits on attacks and counters");
        }
    }
}

#[test]
fn quiet_river_fist_minimums_apply_before_modifiers_and_style_halving() {
    let mut attacker = fixture("quiet_river", "Fist", None, |_| {});
    attacker.sheet.offense.attack_bonus = 100;
    attacker.sheet.offense.strength_damage = 4;
    let mut actors = vec![attacker.clone(), target()];
    let event = resolve_attack(&mut actors, 0, 1, 0, false, 1.0, AttackMode::Normal,
        WeaponSlot::Primary, 0.0, None, &mut rand::rngs::mock::StepRng::new(0, 0));
    let hit = event.damage_breakdown.unwrap();
    assert_eq!(hit.rolled_damage, 2, "Each 1d4p-2 roll contributes at least one");
    assert_eq!(event.damage, 3, "Add +4, then halve (2+4)/2");

    attacker.sheet.offense.attack_bonus = 0;
    let mut defender = target();
    defender.sheet.defense.shield_name = Some("Shield".into());
    defender.sheet.defense.shield_dr = 100;
    defender.reset_state();
    let mut actors = vec![attacker, defender];
    let event = resolve_attack(&mut actors, 0, 1, 0, false, 1.0, AttackMode::Normal,
        WeaponSlot::Primary, 0.0, None, &mut rand::rngs::mock::StepRng::new(0, 0));
    let hit = event.shield_damage_breakdown.unwrap();
    assert_eq!(hit.rolled_damage, 1);
    assert_eq!(event.damage, 2, "Shield pool is floor((1+4)/2), ignoring DR");
}

#[test]
fn quiet_river_near_perfect_defense_punches_keep_damage_and_dr_rules() {
    for superior in [false, true] {
        let mut quiet = fixture("quiet_river", "Fist", None, |p| {
            p.strength_base = 16;
            p.mastery_mut(crate::character::WeaponGroup::Unarmed).damage = 4;
        });
        quiet.sheet.defense.defense_mod = 100;
        if superior {
            quiet.sheet.modifiers.add_i32(StatIdI32::FlagSuperiorDefense, ModifierOpI32::Set(1));
        }
        let mut direct_hits = 0;
        let mut shield_hits = 0;
        for seed in 0..2000 {
            let mut opponent = target();
            opponent.sheet.defense.shield_name = Some("Test shield".into());
            opponent.sheet.defense.shield_dr = 100;
            opponent.sheet.defense.armor_dr = 50;
            opponent.sheet.defense.natural_dr = 30;
            opponent.reset_state();
            let mut actors = vec![opponent, quiet.clone()];
            let event = resolve_attack(&mut actors, 0, 1, 0, false, 1.0,
                AttackMode::Normal, WeaponSlot::Primary, 0.0, None,
                &mut StdRng::seed_from_u64(seed));
            if event.roll.defense_die != 19 { continue; }
            let Some(counter) = event.counter_attack else { continue; };
            if let Some(hit) = counter.damage_breakdown {
                assert_eq!(hit.effective_armor_dr, 0);
                assert_eq!(counter.damage, ((hit.rolled_damage + hit.strength_damage) / 2).max(0));
                direct_hits += 1;
            }
            if let Some(hit) = counter.shield_damage_breakdown {
                assert_eq!(hit.shield_dr, 0);
                assert_eq!(hit.effective_armor_dr, 0);
                assert_eq!(counter.damage, ((hit.rolled_damage + hit.strength_damage) / 2).max(0));
                assert!(counter.damage > 0);
                shield_hits += 1;
            }
        }
        assert!(direct_hits > 0 && shield_hits > 0);
    }
}

#[test]
fn new_styles_are_trained_and_have_catalog_effects() {
    let talents = data::load_talents(data::TALENTS_PATH).unwrap();
    for id in [
        "left_hand_of_evonia",
        "one_path",
        "pilgrims_path",
        "reaper_of_termon",
    ] {
        let style = talents.entries().iter().find(|s| s.id == id).unwrap();
        assert_eq!(style.category, "Weapon Styles");
        assert_eq!(style.cost_bp, None);
        assert!(logic::talent_is_implemented(style));
    }
}

#[test]
fn one_path_requires_both_damage_types_and_a_two_handed_grip() {
    let crushing = fixture("one_path", "Longsword", None, |p| p.two_hand_grip = true);
    assert_eq!(crushing.sheet.offense.weapon.damage_expr, "2d6p");
    assert_eq!(crushing.sheet.offense.weapon.reach_ft, 2.0);
    assert_eq!(crushing.sheet.offense.weapon.armor_penetration, 5);
    assert!(!crushing.sheet.offense.weapon.hacking_or_piercing);
    let one_hand = fixture("one_path", "Longsword", None, |_| {});
    assert_eq!(one_hand.sheet.offense.weapon.damage_expr, "2d8p");
    let piercing_only = fixture("one_path", "Thrusting sword", None, |_| {});
    assert_eq!(piercing_only.sheet.offense.weapon.armor_penetration, 0);
    let untrained = fixture("one_path", "Longsword", None, |p| {
        p.two_hand_grip = true;
        p.proficiencies.clear();
    });
    assert_eq!(untrained.sheet.offense.weapon.damage_expr, "2d8p");
}

#[test]
fn one_path_crushing_bypasses_five_total_dr_only_for_heavy_armor_or_monsters() {
    let attacker = fixture("one_path", "Greatsword", None, |_| {});
    for counter in [false, true] {
        for (dr, heavy, monster, expected) in [
            (4, false, false, 4),
            (4, true, false, 0),
            (8, false, true, 3),
            (5, false, false, 5),
            (4, false, true, 4),
        ] {
            let mut verified = false;
            for seed in 0..200 {
                let mut defender = target();
                defender.sheet.defense.armor_dr = dr;
                defender.sheet.defense.armor_is_heavy = heavy;
                if monster {
                    defender
                        .sheet
                        .modifiers
                        .add_i32(StatIdI32::FlagNpcCombatant, ModifierOpI32::Set(1));
                }
                let mut actors = vec![attacker.clone(), defender];
                let mut rng = StdRng::seed_from_u64(seed);
                let damage = if counter {
                    resolve_style_strike(&mut actors, 0, 1, WeaponSlot::Primary, 1.0, 0.0, &mut rng)
                        .unwrap()
                        .damage_breakdown
                } else {
                    resolve_attack(
                        &mut actors,
                        0,
                        1,
                        0,
                        false,
                        1.0,
                        AttackMode::Normal,
                        WeaponSlot::Primary,
                        0.0,
                        None,
                        &mut rng,
                    )
                    .damage_breakdown
                };
                if let Some(damage) = damage {
                    assert_eq!(damage.effective_armor_dr, expected);
                    verified = true;
                    break;
                }
            }
            assert!(verified);
        }
    }
}

#[test]
fn one_path_precise_piercing_called_shots_get_full_critical_dice() {
    let mut attacker = fixture("one_path", "Longsword", None, |p| {
        p.two_hand_grip = true;
        p.one_path_piercing = true;
        p.called_shot = true;
    });
    Arc::make_mut(&mut attacker.sheet.offense.weapon).crit_min_roll = 100;
    let mut saw_precise = false;
    let mut saw_glance = false;
    for seed in 0..1000 {
        let mut defender = target();
        defender.sheet.defense.armor_dr = 8;
        let mut actors = vec![attacker.clone(), defender];
        let event = resolve_attack(
            &mut actors,
            0,
            1,
            0,
            false,
            1.0,
            AttackMode::Normal,
            WeaponSlot::Primary,
            0.0,
            None,
            &mut StdRng::seed_from_u64(seed),
        );
        if let Some(damage) = event.damage_breakdown {
            if damage.effective_armor_dr == 0 {
                let critical = event
                    .critical
                    .expect("precise piercing hit must be critical");
                if !critical.instant_kill {
                    assert!(critical.extra_dice > 0);
                }
                saw_precise = true;
            } else {
                assert!(event.critical.is_none());
                saw_glance = true;
            }
        }
        if saw_precise && saw_glance {
            break;
        }
    }
    assert!(saw_precise && saw_glance);
}

#[test]
fn reaper_critical_thresholds_require_the_matching_critical_talents() {
    for (talent, expected) in [
        (None, 20),
        (Some("improved_critical"), 18),
        (Some("critical_mastery"), 17),
    ] {
        let actor = fixture("reaper_of_termon", "Scythe", None, |p| {
            p.level = 15;
            p.strength_base = 14;
            p.dex_base = 14;
            if let Some(id) = talent {
                p.talents.push(TalentSelection {
                    id: "improved_critical".into(),
                    rank: 1,
                    weapon: Some("Axes".into()),
                });
                if id == "critical_mastery" {
                    p.talents.push(TalentSelection {
                        id: id.into(),
                        rank: 1,
                        weapon: Some("Axes".into()),
                    });
                }
            }
        });
        assert_eq!(actor.sheet.offense.weapon.crit_min_roll, expected);
    }
}

#[test]
fn reaper_doubles_extra_dice_without_changing_severity_on_attacks_and_counters() {
    let mut reaper = fixture("reaper_of_termon", "Sickle", None, |_| {});
    let weapon = Arc::make_mut(&mut reaper.sheet.offense.weapon);
    weapon.damage_expr = "d4p".into();
    weapon.damage_expr_cache = DamageExprCache::new("d4p");
    let mut ordinary = reaper.clone();
    ordinary.sheet.modifiers = ModifierStack::default();
    for counter in [false, true] {
        let mut checked = false;
        for seed in 0..1000 {
            let run = |attacker: Combatant| {
                let mut actors = vec![attacker, target()];
                let mut rng = StdRng::seed_from_u64(seed);
                if counter {
                    resolve_style_strike(&mut actors, 0, 1, WeaponSlot::Primary, 1.0, 0.0, &mut rng)
                        .unwrap()
                        .critical
                } else {
                    resolve_attack(
                        &mut actors,
                        0,
                        1,
                        0,
                        false,
                        1.0,
                        AttackMode::Normal,
                        WeaponSlot::Primary,
                        0.0,
                        None,
                        &mut rng,
                    )
                    .critical
                }
            };
            if let (Some(base), Some(extra)) = (run(ordinary.clone()), run(reaper.clone())) {
                if base.instant_kill {
                    continue;
                }
                assert_eq!(extra.severity, base.severity);
                assert_eq!(extra.extra_dice, base.extra_dice * 2);
                checked = true;
                break;
            }
        }
        assert!(checked);
    }
}

#[test]
fn evonia_keeps_two_handed_defense_and_adds_offhand_mastery_without_offensive_timers() {
    let actor = fixture(
        "left_hand_of_evonia",
        "Greatsword",
        Some("Short sword"),
        |p| {
            p.mastery_mut(crate::character::WeaponGroup::SmallSwords)
                .defense = 3
        },
    );
    let neutral = fixture(
        "left_hand_of_evonia",
        "Greatsword",
        Some("Short sword"),
        |p| {
            p.mastery_mut(crate::character::WeaponGroup::SmallSwords)
                .defense = 3;
            p.default_weapon_style_ids = Some(vec![]);
        },
    );
    assert!(actor.sheet.offense.weapon.two_hand_grip);
    assert!(actor.sheet.offense.offhand.is_some());
    assert!(!actor.sheet.maneuvers.offensive_dualwielding);
    assert_eq!(
        actor.sheet.defense.defense_mod - neutral.sheet.defense.defense_mod,
        3
    );
    assert!(neutral.sheet.offense.offhand.is_none());
}

#[test]
fn evonia_counters_melee_misses_in_sword_reach_and_stacks_with_normal_counters() {
    let mut defender = fixture(
        "left_hand_of_evonia",
        "Greatsword",
        Some("Short sword"),
        |_| {},
    );
    defender.sheet.vitals.max_hp = 1000;
    defender.sheet.vitals.threshold_of_pain = 1000;
    defender.sheet.defense.defense_mod = 100;
    defender.reset_state();
    defender.state.next_attack_time_primary = Some(30.0);
    let mut found_plain = false;
    let mut found_stacked = false;
    for seed in 0..1000 {
        let mut actors = vec![target(), defender.clone()];
        let event = resolve_attack(
            &mut actors,
            0,
            1,
            0,
            false,
            1.5,
            AttackMode::Normal,
            WeaponSlot::Primary,
            0.0,
            None,
            &mut StdRng::seed_from_u64(seed),
        );
        if event.hit {
            continue;
        }
        assert_eq!(event.additional_counters.len(), 1);
        assert_eq!(
            event.additional_counters[0].weapon_slot,
            WeaponSlot::Secondary
        );
        assert_eq!(actors[1].state.next_attack_time_primary, Some(30.0));
        found_plain |= event.counter_attack.is_none();
        found_stacked |= event.counter_attack.is_some();
        if found_plain && found_stacked {
            break;
        }
    }
    assert!(found_plain && found_stacked);
    for ranged in [false, true] {
        let mut actors = vec![target(), defender.clone()];
        let event = resolve_attack(
            &mut actors,
            0,
            1,
            0,
            ranged,
            if ranged { 1.5 } else { 4.0 },
            AttackMode::Normal,
            WeaponSlot::Primary,
            0.0,
            None,
            &mut StdRng::seed_from_u64(1),
        );
        assert!(event.additional_counters.is_empty());
    }
}

#[test]
fn pilgrim_adds_speed_and_defense_against_melee_and_ranged_attacks() {
    let pilgrim = fixture("pilgrims_path", "Staff", None, |_| {});
    let neutral = fixture("pilgrims_path", "Staff", None, |p| {
        p.default_weapon_style_ids = Some(vec![])
    });
    assert_eq!(
        pilgrim.sheet.offense.weapon.speed,
        neutral.sheet.offense.weapon.speed + 2.0
    );
    for ranged in [false, true] {
        let run = |defender| {
            let mut actors = vec![target(), defender];
            resolve_attack(
                &mut actors,
                0,
                1,
                0,
                ranged,
                1.0,
                AttackMode::Normal,
                WeaponSlot::Primary,
                0.0,
                None,
                &mut StdRng::seed_from_u64(20),
            )
            .roll
            .defense_total
        };
        assert_eq!(run(pilgrim.clone()) - run(neutral.clone()), 4);
    }
}

#[test]
fn pilgrim_pursuit_attacks_require_actual_legal_pursuit_and_keep_regular_timer() {
    for (scamper, decline, blocked, faster, expect_attack) in [
        (false, false, false, false, true),
        (true, false, false, false, true),
        (false, true, false, false, false),
        (false, false, true, false, false),
        (false, false, false, true, false),
    ] {
        let mut pursuer = target();
        pursuer.team_id = 0;
        pursuer.sheet.mobility.move_speed = if faster { 6.0 } else { 5.0 };
        if decline {
            pursuer
                .sheet
                .modifiers
                .add_i32(StatIdI32::FlagDeclinePursuit, ModifierOpI32::Set(1));
        }
        let mut pilgrim = fixture("pilgrims_path", "Staff", None, |p| {
            p.give_ground = !scamper;
            p.scamper_back = scamper;
            p.move_speed = 5.0;
        });
        pilgrim.team_id = 1;
        let mut config = SimConfig::new(5.0, 1.0);
        config.tile_size_ft = 5.0;
        let mut sim = SimState::with_rng(config, crate::core::rng::SimRng::from_seed(18));
        sim.reset_with_combatants(vec![pursuer, pilgrim]);
        sim.actors[0].position = GridPos::new(if blocked { 1 } else { 5 }, 5);
        sim.actors[1].position = GridPos::new(if blocked { 0 } else { 6 }, 5);
        sim.combatants[1].state.next_attack_time_primary = Some(50.0);
        let before = sim.actors[1].position;
        sim.apply_incoming_attack_tactics(0, 1, AttackMode::Normal);
        let attacks = sim
            .combat_events
            .iter()
            .filter(|event| {
                event.attacker_idx == 1 && matches!(event.kind, CombatEventKind::Attack(_))
            })
            .count();
        assert_eq!(attacks, usize::from(expect_attack));
        assert_eq!(sim.combatants[1].state.next_attack_time_primary, Some(50.0));
        if expect_attack {
            assert_eq!(
                sim.actors[1].position.manhattan_distance(before),
                if scamper { 2 } else { 1 }
            );
        }
    }
}

#[test]
fn reaper_expanded_critical_range_still_requires_a_hit() {
    let mut attacker = fixture("reaper_of_termon", "Scythe", None, |p| {
        p.level = 15;
        p.strength_base = 14;
        p.dex_base = 14;
        p.talents.push(TalentSelection {
            id: "improved_critical".into(),
            rank: 1,
            weapon: Some("Axes".into()),
        });
    });
    attacker.sheet.offense.attack_bonus = -100;
    let mut checked = false;
    for seed in 0..1000 {
        let mut defender = target();
        defender.sheet.defense.defense_mod = 100;
        let mut actors = vec![attacker.clone(), defender];
        let event = resolve_attack(
            &mut actors,
            0,
            1,
            0,
            false,
            1.0,
            AttackMode::Normal,
            WeaponSlot::Primary,
            0.0,
            None,
            &mut StdRng::seed_from_u64(seed),
        );
        if matches!(event.roll.attack_die, 18 | 19) {
            assert!(!event.hit);
            assert!(event.critical.is_none());
            checked = true;
            break;
        }
    }
    assert!(checked);
}

#[test]
fn new_style_switches_remove_and_restore_effects_without_changing_hp_or_timers() {
    for (style, primary, secondary, flag) in [
        (
            "left_hand_of_evonia",
            "Greatsword",
            Some("Short sword"),
            StatIdI32::FlagLeftHandOfEvoniaStyle,
        ),
        (
            "one_path",
            "Greatsword",
            None,
            StatIdI32::FlagOnePathPiercingStyle,
        ),
        (
            "pilgrims_path",
            "Staff",
            None,
            StatIdI32::FlagPilgrimsPathStyle,
        ),
        (
            "reaper_of_termon",
            "Sickle",
            None,
            StatIdI32::FlagReaperOfTermonStyle,
        ),
    ] {
        let mut actor = fixture(style, primary, secondary, |p| {
            p.one_path_piercing = true;
            p.tactical_policy.enabled = true;
        });
        actor.state.hp = 7;
        actor.state.next_attack_time_primary = Some(50.0);
        let original_damage = actor.sheet.offense.weapon.damage_expr.clone();
        assert_eq!(actor.apply_i32(flag, 0), 1);
        assert!(actor.switch_tactical_style(vec![]));
        assert_eq!(actor.apply_i32(flag, 0), 0);
        assert!(actor.switch_tactical_style(vec![style.into()]));
        assert_eq!(actor.apply_i32(flag, 0), 1);
        assert_eq!(actor.sheet.offense.weapon.damage_expr, original_damage);
        assert_eq!(actor.state.hp, 7);
        assert_eq!(actor.state.next_attack_time_primary, Some(50.0));
    }
}

#[test]
fn new_style_options_survive_preset_round_trips_and_default_for_legacy_presets() {
    let mut preset = crate::test_support::fighter("Halberd fixture");
    preset.one_path_piercing = true;
    preset.decline_pursuit = true;
    let mut json = serde_json::to_value(&preset).unwrap();
    let loaded: logic::FighterPreset = serde_json::from_value(json.clone()).unwrap();
    assert!(loaded.one_path_piercing && loaded.decline_pursuit);
    json.as_object_mut().unwrap().remove("one_path_piercing");
    json.as_object_mut().unwrap().remove("decline_pursuit");
    let legacy: logic::FighterPreset = serde_json::from_value(json).unwrap();
    assert!(!legacy.one_path_piercing && !legacy.decline_pursuit);
}

#[test]
fn unarmed_near_perfect_counters_use_unarmed_mastery_instead_of_the_held_sword() {
    use crate::character::WeaponGroup;
    let mut defender = fixture("", "Longsword", None, |p| {
        p.mastery_mut(WeaponGroup::LargeSwords).attack = 6;
        p.mastery_mut(WeaponGroup::LargeSwords).damage = 6;
        p.mastery_mut(WeaponGroup::Unarmed).attack = 2;
        p.mastery_mut(WeaponGroup::Unarmed).damage = 3;
    });
    let mut neutral = fixture("", "Longsword", None, |_| {});
    defender.sheet.defense.defense_mod = 100;
    neutral.sheet.defense.defense_mod = 100;
    for eyesmite in [false, true] {
        defender.sheet.defense.eyesmite = eyesmite;
        neutral.sheet.defense.eyesmite = eyesmite;
        let mut verified = false;
        for seed in 0..1000 {
            let run = |actor| {
                let mut actors = vec![target(), actor];
                resolve_attack(
                    &mut actors,
                    0,
                    1,
                    0,
                    false,
                    1.0,
                    AttackMode::Normal,
                    WeaponSlot::Primary,
                    0.0,
                    None,
                    &mut StdRng::seed_from_u64(seed),
                )
            };
            let original = run(neutral.clone());
            if original.roll.defense_die != 19 {
                continue;
            }
            let Some(base_counter) = original.counter_attack else {
                continue;
            };
            let trained_counter = run(defender.clone()).counter_attack.unwrap();
            assert_eq!(
                trained_counter.roll.attack_bonus - base_counter.roll.attack_bonus,
                2
            );
            if let (Some(trained), Some(base)) = (
                trained_counter.damage_breakdown,
                base_counter.damage_breakdown,
            ) {
                assert_eq!(trained.strength_damage - base.strength_damage, 3);
                verified = true;
                break;
            }
        }
        assert!(
            verified,
            "No shared successful near-perfect counter was exercised (Eyesmite: {eyesmite})"
        );
    }
}

#[test]
fn ranged_styles_require_the_right_proficiency_and_weapon_group() {
    for (style, valid_weapon) in [
        ("whistling_wind", "Shortbow"),
        ("gravesend", "Light crossbow"),
    ] {
        let valid = fixture(style, valid_weapon, None, |_| {});
        let profile = valid.sheet.offense.weapon.ranged_styles;
        assert!(profile.whistling_wind_pace.is_some() || profile.gravesend_aim.is_some());
        let missing = fixture(style, valid_weapon, None, |p| p.proficiencies.clear());
        assert!(missing.active_style_ids.is_empty());
        let melee = fixture(style, "Halberd", None, |p| {
            p.proficiencies.push(valid_weapon.into())
        });
        assert!(
            melee
                .sheet
                .offense
                .weapon
                .ranged_styles
                .whistling_wind_pace
                .is_none()
        );
        assert!(
            melee
                .sheet
                .offense
                .weapon
                .ranged_styles
                .gravesend_aim
                .is_none()
        );
    }
    let crossbow = fixture("whistling_wind", "Light crossbow", None, |p| {
        p.proficiencies.push("Shortbow".into())
    });
    assert!(
        crossbow
            .sheet
            .offense
            .weapon
            .ranged_styles
            .whistling_wind_pace
            .is_none()
    );
}

#[test]
fn gravesend_reverses_all_four_range_bands_without_extending_range() {
    let attacker = fixture("gravesend", "Shortbow", None, |_| {});
    for (distance, normal, inverted) in [
        (50.0, 0, -8),
        (80.0, -4, -6),
        (120.0, -6, -4),
        (150.0, -8, 0),
    ] {
        let mut actors = vec![attacker.clone(), target()];
        let outcome = resolve_attack(
            &mut actors,
            0,
            1,
            normal,
            true,
            distance,
            AttackMode::Normal,
            WeaponSlot::Primary,
            5.0,
            None,
            &mut StdRng::seed_from_u64(42),
        );
        assert_eq!(outcome.roll.range_mod, inverted);
    }
    let weapon = &attacker.sheet.offense.weapon;
    assert_eq!(
        super::movement::range_modifier_for_weapon_with_scale(weapon, 150.1, 1.0),
        None
    );
    assert_eq!(
        super::movement::range_modifier_for_weapon_with_scale(weapon, 75.0, 2.0),
        Some(-8)
    );
}

#[test]
fn gravesend_second_tier_adds_two_attack_and_preserves_natural_and_shield_dr() {
    let attacker = fixture("gravesend", "Shortbow", None, |p| {
        p.ranged_styles.gravesend_aim = GravesendAim::PierceArmor
    });
    let mut body_hits = 0;
    let mut covered_misses = 0;
    for seed in 0..500 {
        let mut defender = target();
        defender.sheet.defense.armor_dr = 103;
        defender.sheet.defense.natural_dr = 3;
        defender.sheet.defense.shield_name = Some("Shield".into());
        defender.sheet.defense.shield_defense_bonus = 4;
        defender.sheet.defense.shield_dr = 5;
        if seed % 2 == 0 {
            defender.sheet.defense.shield_cover_value = Some(6);
        }
        defender.reset_state();
        let mut actors = vec![attacker.clone(), defender];
        let outcome = resolve_attack(
            &mut actors,
            0,
            1,
            -8,
            true,
            140.0,
            AttackMode::Normal,
            WeaponSlot::Primary,
            10.0,
            None,
            &mut StdRng::seed_from_u64(seed),
        );
        assert_eq!(
            outcome.roll.attack_bonus,
            attacker.sheet.offense.attack_bonus + 2
        );
        if let Some(damage) = outcome.damage_breakdown {
            assert_eq!(damage.effective_armor_dr, 3);
            body_hits += 1;
        }
        // Ranged shields provide cover rather than the melee shield-hit window.
        assert!(outcome.shield_damage_breakdown.is_none());
        assert_eq!(actors[1].sheet.defense.shield_dr, 5);
        if seed % 2 == 0 {
            assert!(outcome.roll.attack_total <= 6);
            covered_misses += usize::from(!outcome.hit);
        }
    }
    assert!(
        body_hits > 20 && covered_misses > 20,
        "exercise hits and shield cover: {body_hits}/{covered_misses}"
    );
}

#[test]
fn gravesend_forces_failure_only_when_actual_damage_exceeds_pain_threshold() {
    for aim in [
        GravesendAim::InvertRange,
        GravesendAim::PierceArmor,
        GravesendAim::ForceTrauma,
    ] {
        for threshold in [19, 20, 21] {
            let mut attacker = fixture("gravesend", "Shortbow", None, |p| {
                p.ranged_styles.gravesend_aim = aim
            });
            let weapon = Arc::make_mut(&mut attacker.sheet.offense.weapon);
            weapon.damage_expr = "20".into();
            weapon.damage_expr_cache = DamageExprCache::new("20");
            weapon.crit_min_roll = 21;
            let mut defender = target();
            defender.sheet.defense.ranged_defense_mod = -1000;
            defender.sheet.vitals.threshold_of_pain = threshold;
            defender.sheet.vitals.constitution = 40;
            defender.sheet.vitals.trauma_die_sides = 1;
            defender.sheet.vitals.trauma_die_penetrating = false;
            defender.reset_state();
            let mut actors = vec![attacker, defender];
            let outcome = resolve_attack(
                &mut actors,
                0,
                1,
                -8,
                true,
                140.0,
                AttackMode::Normal,
                WeaponSlot::Primary,
                20.0,
                None,
                &mut StdRng::seed_from_u64(9),
            );
            assert_eq!(outcome.damage, 20);
            assert_eq!(
                outcome.trauma_applied,
                aim == GravesendAim::ForceTrauma && threshold < 20
            );
        }
    }
}

#[test]
fn gravesend_charges_total_aim_time_on_first_and_every_subsequent_shot() {
    for aim in [
        GravesendAim::InvertRange,
        GravesendAim::PierceArmor,
        GravesendAim::ForceTrauma,
    ] {
        let mut attacker = fixture("gravesend", "Shortbow", None, |p| {
            p.ranged_styles.gravesend_aim = aim;
            p.move_speed = 0.0;
        });
        attacker.team_id = 0;
        let speed = attacker.sheet.offense.weapon.speed;
        let mut defender = target();
        defender.team_id = 1;
        defender.sheet.maneuvers.passive = true;
        defender.sheet.offense.weapon = Arc::new(WeaponProfile {
            has_weapon: true,
            ..WeaponProfile::default()
        });
        let mut sim = SimState::with_rng(
            SimConfig::new(100.0, 1.0),
            crate::core::rng::SimRng::from_seed(52),
        );
        sim.log_events = true;
        sim.reset_with_combatants(vec![attacker, defender]);
        for _ in 0..90 {
            sim.tick();
        }
        let times: Vec<_> = sim
            .combat_events
            .iter()
            .filter_map(|e| match &e.kind {
                CombatEventKind::Attack(a) if e.attacker_idx == 0 && a.is_ranged => Some(e.time),
                _ => None,
            })
            .collect();
        assert!(times.len() >= 2, "{aim:?}: {times:?}");
        assert_eq!(times[0], aim.seconds() as u32);
        assert_eq!(times[1] - times[0], (speed + aim.seconds()).ceil() as u32);
    }
}

#[test]
fn whistling_wind_allows_jogging_then_running_with_a_lingering_next_attack_penalty() {
    use super::ranged_styles::shooting_move_speed;
    let mut jogger = fixture("whistling_wind", "Shortbow", None, |p| p.move_speed = 20.0);
    assert_eq!(shooting_move_speed(&jogger, 20.0), 10.0);
    assert_eq!(
        super::combat::preview_attack_bonus(&jogger, WeaponSlot::Primary),
        jogger.sheet.offense.attack_bonus
    );
    let mut runner = fixture("whistling_wind", "Shortbow", None, |p| {
        p.move_speed = 20.0;
        p.ranged_styles.whistling_wind_pace = BowPace::Run;
    });
    assert_eq!(shooting_move_speed(&runner, 20.0), 10.0, "start at a jog");
    runner.state.moved_last_tick = true;
    assert_eq!(shooting_move_speed(&runner, 20.0), 15.0);
    runner.state.whistling_wind_running = true;
    runner.state.whistling_wind_next_attack_penalty = true;
    let mut actors = vec![runner, target()];
    for shot in 0..3 {
        if shot == 1 {
            actors[0].state.whistling_wind_running = false;
            actors[0].state.whistling_wind_next_attack_penalty = true;
        }
        let expected = actors[0].sheet.offense.attack_bonus - if shot < 2 { 4 } else { 0 };
        let outcome = resolve_attack(
            &mut actors,
            0,
            1,
            0,
            true,
            50.0,
            AttackMode::Normal,
            WeaponSlot::Primary,
            shot as f32,
            None,
            &mut StdRng::seed_from_u64(101),
        );
        assert_eq!(outcome.roll.attack_bonus, expected);
    }
    Arc::make_mut(&mut jogger.sheet.offense.weapon).ranged_styles = RangedStyleProfile::default();
    assert_eq!(shooting_move_speed(&jogger, 20.0), 5.0);
    assert_eq!(
        shooting_move_speed(&jogger, 2.5),
        2.5,
        "slowing magic is not applied twice"
    );
}

#[test]
fn whistling_wind_real_movement_accelerates_and_does_not_penalize_a_blocked_retreat() {
    for blocked in [false, true] {
        let mut attacker = fixture("whistling_wind", "Shortbow", None, |p| {
            p.move_speed = 20.0;
            p.ranged_styles.whistling_wind_pace = BowPace::Run;
        });
        attacker.team_id = 0;
        let mut defender = target();
        defender.team_id = 1;
        defender.sheet.maneuvers.passive = true;
        let mut config = SimConfig::new(100.0, 1.0);
        config.tile_size_ft = 5.0;
        config.grid_width = 200;
        let mut sim = SimState::with_rng(config, crate::core::rng::SimRng::from_seed(42));
        sim.reset_with_combatants(vec![attacker, defender]);
        let start_x = if blocked { 0 } else { 50 };
        sim.actors[0].position = GridPos::new(start_x, 0);
        sim.actors[1].position = GridPos::new(start_x + 20, 0);
        sim.tick();
        assert_eq!(sim.actors[0].position.x, if blocked { 0 } else { 48 });
        assert!(!sim.combatants[0].state.whistling_wind_running);
        sim.tick();
        assert_eq!(sim.actors[0].position.x, if blocked { 0 } else { 45 });
        assert_eq!(sim.combatants[0].state.whistling_wind_running, !blocked);
        assert_eq!(
            sim.combatants[0].state.whistling_wind_next_attack_penalty,
            !blocked
        );
    }
}

#[test]
fn gravesend_tactical_activation_pays_the_full_aim_delay() {
    use crate::core::tactics::{TacticalAction, TacticalCondition, TacticalRule};
    let mut attacker = fixture("gravesend", "Shortbow", None, |p| {
        p.move_speed = 0.0;
        p.ranged_styles.gravesend_aim = GravesendAim::ForceTrauma;
        p.default_weapon_style_ids = Some(vec![]);
        p.tactical_policy.enabled = true;
        p.tactical_policy.rules = vec![TacticalRule::new(
            TacticalAction::UseWeaponStyle {
                style_ids: vec!["gravesend".into()],
            },
            vec![TacticalCondition::Always],
        )];
    });
    attacker.team_id = 0;
    let speed = attacker.sheet.offense.weapon.speed;
    let mut defender = target();
    defender.team_id = 1;
    defender.sheet.maneuvers.passive = true;
    let mut sim = SimState::with_rng(
        SimConfig::new(100.0, 1.0),
        crate::core::rng::SimRng::from_seed(43),
    );
    sim.log_events = true;
    sim.reset_with_combatants(vec![attacker, defender]);
    sim.tick();
    assert!(!sim.combatants[0].state.has_attacked);
    assert_eq!(sim.combatants[0].state.next_attack_time_primary, Some(20.0));
    for _ in 0..80 {
        sim.tick();
    }
    let times: Vec<_> = sim
        .combat_events
        .iter()
        .filter_map(|e| match &e.kind {
            CombatEventKind::Attack(a) if e.attacker_idx == 0 && a.is_ranged => Some(e.time),
            _ => None,
        })
        .collect();
    assert!(times.len() >= 2);
    assert_eq!(times[0], 20);
    assert_eq!(times[1] - times[0], (speed + 20.0).ceil() as u32);
}

#[test]
fn a_static_whistling_wind_style_is_recorded_without_enabling_tactics() {
    let actor = fixture("whistling_wind", "Longbow", None, |_| {});
    assert!(!actor.tactical_policy.enabled);
    assert_eq!(actor.active_style_ids, vec!["whistling_wind"]);
    assert_eq!(actor.state.activated_style_ids, actor.active_style_ids);
    assert_eq!(actor.sheet.offense.weapon.ranged_styles.whistling_wind_pace, Some(BowPace::Jog));
}
