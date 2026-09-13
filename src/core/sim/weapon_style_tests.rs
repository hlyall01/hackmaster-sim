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
    let presets = data::load_fighter_presets("data/sim/fighter_presets.json").unwrap();
    let mut preset = presets.entries().first().unwrap().clone();
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
