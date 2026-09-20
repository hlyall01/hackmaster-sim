use super::*;
use crate::core::sim::{
    CombatantSheet, MountType, OffhandProfile, WeaponProfile, knockback_rule_for_attack,
};
use rand::{SeedableRng, rngs::StdRng};

fn fighters(raw: i32, mounted: bool, impaler: bool) -> Vec<Combatant> {
    let mut sheet = CombatantSheet::default();
    sheet.vitals.max_hp = 10_000;
    sheet.vitals.threshold_of_pain = 10_000;
    sheet.offense.attack_bonus = 100;
    sheet.offense.weapon = Arc::new(WeaponProfile {
        name: "Spear".into(),
        has_weapon: true,
        damage_expr: raw.to_string(),
        damage_expr_cache: DamageExprCache::new(&raw.to_string()),
        shield_damage_expr: Some(raw.to_string()),
        shield_damage_expr_cache: Some(DamageExprCache::new(&raw.to_string())),
        crit_min_roll: 100,
        defender_knockback_step_adjustment: if impaler { -5 } else { 0 },
        ..WeaponProfile::default()
    });
    sheet.maneuvers.mounted = mounted;
    sheet.maneuvers.mounted_combat.trot_or_faster = true;
    let attacker = Combatant::new_with_team(sheet.clone(), 0);
    sheet.defense.knockback_step = 15;
    sheet.maneuvers.mounted = false;
    sheet.maneuvers.passive = true;
    vec![attacker, Combatant::new_with_team(sheet, 1)]
}

#[test]
fn knockback_threshold_reductions_stack_and_charge_multiplies_damage_before_rounding() {
    for raw in [4, 5, 7, 9, 10, 14, 15, 19, 20, 29, 30] {
        for mounted in [false, true] {
            for impaler in [false, true] {
                for charge in [false, true] {
                    let mut actors = fighters(raw, mounted, impaler);
                    let rule = knockback_rule_for_attack(
                        &actors[0],
                        &actors[1],
                        Some(WeaponSlot::Primary),
                        false,
                        charge,
                    );
                    let threshold = 15 - if mounted { 5 } else { 0 } - if impaler { 5 } else { 0 };
                    let expected = (raw * if charge { 2 } else { 1 } / threshold * 5) as f32;
                    let outcome = resolve_attack(
                        &mut actors,
                        0,
                        1,
                        0,
                        false,
                        5.0,
                        if charge {
                            AttackMode::Charge
                        } else {
                            AttackMode::Normal
                        },
                        WeaponSlot::Primary,
                        0.0,
                        None,
                        &mut StdRng::seed_from_u64(7),
                    );
                    assert!(outcome.hit);
                    assert_eq!(outcome.damage_breakdown.unwrap().raw_damage, raw);
                    assert_eq!(rule.damage_per_step, threshold);
                    assert_eq!(
                        outcome.knockback_ft, expected,
                        "raw {raw}, mounted {mounted}, impaler {impaler}, charge {charge}"
                    );
                    assert_eq!(rule.distance_ft(raw), outcome.knockback_ft);
                    assert_eq!(
                        outcome.damage, raw,
                        "charge's knockback multiplier must not double HP damage"
                    );
                }
            }
        }
    }
}

#[test]
fn shield_blocks_knock_back_even_when_dr_prevents_all_hp_damage() {
    for counter in [false, true] {
        for charge in [false, true] {
            if counter && charge {
                continue;
            }
            for slot in [WeaponSlot::Primary, WeaponSlot::Secondary] {
                let mut found = false;
                for seed in 0..500 {
                    let mut actors = fighters(26, true, true);
                    actors[0].sheet.offense.attack_bonus = 0;
                    actors[0].sheet.offense.offhand = Some(OffhandProfile {
                        attack_bonus: 0,
                        strength_damage: 0,
                        weapon: actors[0].sheet.offense.weapon.clone(),
                    });
                    actors[0].sheet.maneuvers.dualwield_offhand_damage_penalty = 0;
                    actors[1].sheet.defense.shield_name = Some("Test shield".into());
                    actors[1].sheet.defense.shield_dr = 1000;
                    actors[1].sheet.defense.armor_dr = 1000;
                    actors[1].state.shield_intact = true;
                    let (blocked, damage, knockback, shield_damage) = if counter {
                        let hit = resolve_counter_attack(
                            &mut actors,
                            0,
                            1,
                            0.0,
                            slot,
                            true,
                            false,
                            false,
                            false,
                            false,
                            1,
                            &mut StdRng::seed_from_u64(seed),
                        );
                        (
                            hit.shield_block,
                            hit.damage,
                            hit.knockback_ft,
                            hit.shield_damage,
                        )
                    } else {
                        let hit = resolve_attack(
                            &mut actors,
                            0,
                            1,
                            0,
                            false,
                            5.0,
                            if charge {
                                AttackMode::Charge
                            } else {
                                AttackMode::Normal
                            },
                            slot,
                            0.0,
                            None,
                            &mut StdRng::seed_from_u64(seed),
                        );
                        (
                            hit.shield_block,
                            hit.damage,
                            hit.knockback_ft,
                            hit.shield_damage,
                        )
                    };
                    if !blocked {
                        continue;
                    }
                    assert_eq!(shield_damage, 26);
                    assert_eq!(damage, 0);
                    assert_eq!(actors[1].state.hp, 10_000);
                    assert_eq!(knockback, if charge { 50.0 } else { 25.0 });
                    assert_eq!(actors[1].state.next_attack_time_primary, Some(10.0));
                    found = true;
                    break;
                }
                assert!(
                    found,
                    "no shield block for counter {counter}, charge {charge}, slot {slot:?}"
                );
            }
        }
    }
}

#[test]
fn mounted_knockback_reduction_requires_moving_mount_and_melee() {
    for mount in [
        MountType::RidingHorse,
        MountType::Rounsey,
        MountType::Courser,
        MountType::Destrier,
    ] {
        for mounted in [false, true] {
            for moving in [false, true] {
                for ranged in [false, true] {
                    let mut actors = fighters(14, mounted, false);
                    actors[0].sheet.maneuvers.mounted_combat.mount = mount;
                    actors[0].sheet.maneuvers.mounted_combat.trot_or_faster = moving;
                    let rule = knockback_rule_for_attack(
                        &actors[0],
                        &actors[1],
                        Some(WeaponSlot::Primary),
                        ranged,
                        false,
                    );
                    assert_eq!(
                        rule.mounted_adjustment,
                        if mounted && moving && !ranged { -5 } else { 0 }
                    );
                }
            }
        }
    }
}

#[test]
fn knockback_uses_actual_weapon_slot_and_preserves_defender_resistance() {
    let mut actors = fighters(29, true, true);
    actors[1].sheet.defense.knockback_step = 25;
    let mut offhand = (*actors[0].sheet.offense.weapon).clone();
    offhand.defender_knockback_step_adjustment = 0;
    actors[0].sheet.offense.offhand = Some(OffhandProfile {
        attack_bonus: 0,
        strength_damage: 0,
        weapon: Arc::new(offhand),
    });
    for (slot, threshold) in [
        (Some(WeaponSlot::Primary), 15),
        (Some(WeaponSlot::Secondary), 20),
        (None, 20),
    ] {
        let rule = knockback_rule_for_attack(&actors[0], &actors[1], slot, false, false);
        assert_eq!(rule.damage_per_step, threshold);
        assert_eq!(rule.distance_ft(29), 5.0);
    }
}

#[test]
fn knockback_handles_zero_damage_and_threshold_floor_without_overflow() {
    let mut actors = fighters(0, true, true);
    actors[1].sheet.defense.knockback_step = 10;
    let rule = knockback_rule_for_attack(
        &actors[0],
        &actors[1],
        Some(WeaponSlot::Primary),
        false,
        true,
    );
    assert_eq!(rule.damage_per_step, 1);
    assert_eq!(rule.distance_ft(0), 0.0);
    assert_eq!(rule.distance_ft(-1), 0.0);
    assert!(rule.distance_ft(i32::MAX).is_finite());
}

#[test]
fn knockback_at_ten_feet_knocks_down_and_resets_both_weapons_for_hits_and_blocks() {
    for counter in [false, true] {
        for shield in [false, true] {
            for (raw, distance) in [(0, 0.0), (15, 5.0), (30, 10.0), (45, 15.0)] {
                let mut found = false;
                for seed in 0..500 {
                    let mut actors = fighters(raw, false, false);
                    actors[1].state.next_attack_time_primary = Some(1.0);
                    actors[1].state.next_attack_time_secondary = Some(2.0);
                    if shield {
                        actors[0].sheet.offense.attack_bonus = 0;
                        actors[1].sheet.defense.shield_name = Some("Test shield".into());
                        actors[1].sheet.defense.shield_dr = 1000;
                        actors[1].sheet.defense.armor_dr = 1000;
                        actors[1].state.shield_intact = true;
                    }
                    let (hit, blocked, knockback) = if counter {
                        let result = resolve_counter_attack(
                            &mut actors,
                            0,
                            1,
                            5.0,
                            WeaponSlot::Primary,
                            true,
                            false,
                            false,
                            false,
                            false,
                            1,
                            &mut StdRng::seed_from_u64(seed),
                        );
                        (result.hit, result.shield_block, result.knockback_ft)
                    } else {
                        let result = resolve_attack(
                            &mut actors,
                            0,
                            1,
                            0,
                            false,
                            5.0,
                            AttackMode::Normal,
                            WeaponSlot::Primary,
                            5.0,
                            None,
                            &mut StdRng::seed_from_u64(seed),
                        );
                        (result.hit, result.shield_block, result.knockback_ft)
                    };
                    if (shield && !blocked) || (!shield && !hit) {
                        continue;
                    }
                    assert_eq!(knockback, distance);
                    let down = distance >= 10.0;
                    assert_eq!(
                        actors[1].state.knockback_immobile_seconds,
                        i32::from(down),
                        "counter={counter}, shield={shield}, distance={distance}"
                    );
                    assert_eq!(
                        actors[1].state.next_attack_time_primary,
                        Some(if down { 15.0 } else { 1.0 })
                    );
                    assert_eq!(
                        actors[1].state.next_attack_time_secondary,
                        Some(if down { 15.0 } else { 2.0 })
                    );
                    if shield {
                        assert_eq!(actors[1].state.hp, 10_000);
                    }
                    found = true;
                    break;
                }
                assert!(
                    found,
                    "no matching impact: counter={counter}, shield={shield}, raw={raw}"
                );
            }
        }
    }
}

#[test]
fn hammerer_small_knockback_resets_weapons_without_knocking_down() {
    let mut actors = fighters(15, false, false);
    let defender = &mut actors[1];
    apply_knockback_recovery(defender, 5.0, 5.0, 10.0, true);
    assert_eq!(defender.state.knockback_immobile_seconds, 0);
    assert_eq!(defender.state.next_attack_time_primary, Some(15.0));
    assert_eq!(defender.state.next_attack_time_secondary, Some(15.0));
}

#[test]
fn knockdown_does_not_shorten_existing_incapacitation() {
    let mut actors = fighters(30, false, false);
    actors[1].state.knockback_immobile_seconds = 3;
    apply_knockback_recovery(&mut actors[1], 10.0, 0.0, 10.0, false);
    assert_eq!(actors[1].state.knockback_immobile_seconds, 3);
}

#[test]
fn moving_mounted_impaler_38_raw_30_damage_knocks_back_35_feet() {
    for counter in [false, true] {
        let mut actors = fighters(38, true, true);
        actors[1].sheet.defense.armor_dr = 8;
        let (hit, damage, distance) = if counter {
            let result = resolve_counter_attack(
                &mut actors,
                0,
                1,
                5.0,
                WeaponSlot::Primary,
                true,
                false,
                false,
                false,
                false,
                1,
                &mut StdRng::seed_from_u64(7),
            );
            (result.hit, result.damage, result.knockback_ft)
        } else {
            let result = resolve_attack(
                &mut actors,
                0,
                1,
                0,
                false,
                10.0,
                AttackMode::Normal,
                WeaponSlot::Primary,
                5.0,
                None,
                &mut StdRng::seed_from_u64(7),
            );
            (result.hit, result.damage, result.knockback_ft)
        };
        assert!(hit);
        assert_eq!(damage, 30);
        assert_eq!(distance, 35.0);
        assert_eq!(actors[1].state.knockback_immobile_seconds, 1);
    }
}
