use super::*;
use crate::sim::{
    CombatantSheet, MountType, MountedCombatConfig, OffhandProfile, WeaponProfile,
};
use rand::{SeedableRng, rngs::StdRng};

#[test]
fn mounted_defense_applies_once_to_melee_ranged_shields_and_counters() {
    for (moving, expected) in [(false, 2), (true, 6)] {
        for shield in [false, true] {
            for ranged_modifier in [0, 8] {
                for kind in 0..4 {
                    let roll = |mounted: bool| {
                        let mut actors = fighters();
                        let defender = &mut actors[1];
                        defender.sheet.maneuvers.mounted = mounted;
                        defender.sheet.maneuvers.mounted_combat.trot_or_faster = moving;
                        defender.sheet.maneuvers.mounted_combat.mount = MountType::RidingHorse;
                        defender.sheet.defense.ranged_defense_mod = ranged_modifier;
                        defender.sheet.defense.shield_defense_bonus = 4;
                        defender.state.shield_intact = shield;
                        let mut rng = StdRng::seed_from_u64(7);
                        match kind {
                            0 | 1 => resolve_attack(&mut actors, 0, 1, 0, kind == 1, 10.,
                                AttackMode::Normal, WeaponSlot::Primary, 0., None, &mut rng).roll,
                            2 => resolve_counter_attack(&mut actors, 0, 1, 0., WeaponSlot::Primary,
                                true, false, false, false, false, 1, &mut rng).roll,
                            _ => resolve_eyesmite(&mut actors, 0, 1, 0., &mut rng).roll,
                        }
                    };
                    let baseline = roll(false);
                    let mounted = roll(true);
                    assert_eq!(mounted.defense_die, baseline.defense_die);
                    assert_eq!(mounted.defense_base - baseline.defense_base, expected);
                    assert_eq!(mounted.defense_total - baseline.defense_total, expected);
                    assert_eq!(mounted.shield_defense_bonus, baseline.shield_defense_bonus);
                }
            }
        }
    }
}

#[test]
fn legacy_lance_settings_cannot_override_fixed_rule_or_automatic_detection() {
    for stacking in ["base_then_warhorse", "base_then_all_bonuses", "double_all_dice"] {
        for manual_lance in [false, true] {
            let settings: MountedCombatConfig = serde_json::from_value(serde_json::json!({
                "mount": "courser",
                "trot_or_faster": true,
                "lance_stacking": stacking,
                "lance_reach_and_momentum": manual_lance,
            })).unwrap();
            assert_eq!(average(&weapon("Lance", "2d8p"), settings, true), 31); // 7d8
            let saved = serde_json::to_value(settings).unwrap();
            assert!(saved.get("lance_stacking").is_none());
            assert!(saved.get("lance_reach_and_momentum").is_none());
        }
    }
}

#[test]
fn lance_bonuses_follow_the_weapon_used_for_each_attack() {
    let mut actors = fighters();
    actors[0].sheet.maneuvers.mounted_combat.mount = MountType::Courser;
    actors[0].sheet.maneuvers.mounted_combat.riding = crate::sim::RidingMastery::Advanced;
    let lance = weapon("Lance", "2d8p");
    let mace = weapon("Mace", "2d6p");
    for (weapon, double_base, attack_bonus, expression) in [
        (&lance, true, 6, "7d8"),
        (&mace, false, 2, "4d6"),
        (&lance, true, 6, "7d8"),
    ] {
        assert_eq!(mounted_damage_plan(&actors[0], &actors[1], weapon, false).double_base_dice, double_base);
        assert_eq!(actors[0].sheet.maneuvers.mounted_combat.attack_bonus(&weapon.name, false), attack_bonus);
        assert_eq!(weapon_damage_expression(&actors[0], &actors[1], weapon, false, false), expression);
    }
}

#[test]
fn derived_damage_expression_tracks_mounted_toggles_and_target_size() {
    let mut actors = fighters();
    actors[0].sheet.maneuvers.mounted_combat = MountedCombatConfig {
        mount: MountType::Courser,
        trot_or_faster: true,
        ..Default::default()
    };
    let mut lance = weapon("Lance", "2d8p");
    lance.force_nonpenetrating_damage = false;
    let expression = |actors: &[Combatant]| {
        weapon_damage_expression(&actors[0], &actors[1], &lance, false, false)
    };
    assert_eq!(expression(&actors), "7d8p");
    actors[1].sheet.defense.is_medium_sized = false;
    assert_eq!(expression(&actors), "5d8p");
    actors[0].sheet.maneuvers.mounted_combat.target_size = crate::sim::MountedTargetSize::Medium;
    assert_eq!(expression(&actors), "7d8p");
    actors[0].sheet.maneuvers.mounted_combat.mount = MountType::Destrier;
    assert_eq!(expression(&actors), "8d8p");
    actors[0].sheet.maneuvers.mounted_combat.trot_or_faster = false;
    assert_eq!(expression(&actors), "3d8p");
    actors[0].sheet.maneuvers.mounted = false;
    assert_eq!(expression(&actors), "2d8p");
    actors[0].sheet.maneuvers.mounted = true;
    assert_eq!(weapon_damage_expression(&actors[0], &actors[1], &lance, true, false), "2d8p");
}

#[test]
fn derived_damage_expression_preserves_flat_terms_and_matches_combat_dice() {
    let mut actors = fighters();
    actors[0].sheet.maneuvers.mounted_combat = MountedCombatConfig {
        mount: MountType::Courser,
        trot_or_faster: true,
        ..Default::default()
    };
    for (name, base, expected) in [
        ("Lance", "2d8p+3", "7d8p + 3"),
        ("Lance", "2d8p-3", "7d8p - 3"),
        ("Horseman's pick", "d4p+d6p+2", "4d4p + 1d6p + 2"),
    ] {
        let mut profile = weapon(name, base);
        profile.force_nonpenetrating_damage = false;
        profile.shield_damage_expr = Some(base.into());
        profile.shield_damage_expr_cache = Some(DamageExprCache::new(base));
        for shield in [false, true] {
            let expression = weapon_damage_expression(&actors[0], &actors[1], &profile, false, shield);
            assert_eq!(expression, expected);
            let actual = roll_mounted_weapon_damage(&profile,
                mounted_damage_plan(&actors[0], &actors[1], &profile, false),
                false, shield, true, &mut StdRng::seed_from_u64(1));
            assert_eq!(actual, DamageExprCache::new(&expression).expected(false).floor() as i32);
        }
    }
    let mut profile = weapon("Lance", "2d8p");
    assert_eq!(weapon_damage_expression(&actors[0], &actors[1], &profile, false, false), "7d8");
    profile.shield_damage_expr = None;
    assert_eq!(weapon_damage_expression(&actors[0], &actors[1], &profile, false, true), "-");
    profile.is_unarmed = true;
    assert_eq!(weapon_damage_expression(&actors[0], &actors[1], &profile, false, false), "2d8");
}

fn weapon(name: &str, expr: &str) -> WeaponProfile {
    WeaponProfile {
        name: name.into(),
        damage_expr: expr.into(),
        damage_expr_cache: DamageExprCache::new(expr),
        has_weapon: true,
        force_nonpenetrating_damage: true,
        crit_min_roll: 100,
        ..Default::default()
    }
}

fn average(weapon: &WeaponProfile, settings: MountedCombatConfig, medium: bool) -> i32 {
    roll_mounted_weapon_damage(
        weapon,
        settings.damage_plan(&weapon.name, medium),
        false,
        false,
        true,
        &mut StdRng::seed_from_u64(1),
    )
}

#[test]
fn mounted_damage_uses_smaller_die_and_respects_medium_target() {
    let settings = MountedCombatConfig::default();
    let ordinary = weapon("Hand axe", "d4p+d6p");
    let saddle = weapon("Horseman's pick", "d4p+d6p");
    assert_eq!(average(&ordinary, settings, true), 8); // 2d4+d6: 8.5, rounded down once.
    assert_eq!(average(&saddle, settings, true), 11); // 3d4+d6
    assert_eq!(average(&ordinary, settings, false), 6);
    assert_eq!(average(&saddle, settings, false), 6);
}

#[test]
fn mounted_lance_always_doubles_base_then_adds_mounted_and_horse_dice() {
    let lance = weapon("Lance", "2d8p");
    for (mount, dice) in [
        (MountType::Rounsey, 6),
        (MountType::Courser, 7),
        (MountType::Destrier, 8),
    ] {
        let settings = MountedCombatConfig {
            mount,
            trot_or_faster: true,
            ..Default::default()
        };
        assert_eq!(average(&lance, settings, true), (dice as f64 * 4.5).floor() as i32);
    }
}

#[test]
fn mounted_default_matches_selected_table_ruling() {
    let settings = MountedCombatConfig {
        mount: MountType::Courser,
        trot_or_faster: true,
        ..Default::default()
    };
    assert_eq!(average(&weapon("Lance", "2d8p"), settings, true), 31); // 7d8, floor(31.5)
    assert_eq!(average(&weapon("Lance", "2d8p"), settings, false), 22); // 5d8, no Medium bonus
}

#[test]
fn mounted_lance_momentum_requires_speed_and_a_warhorse() {
    let lance = weapon("Lance", "2d8p+3");
    let mut settings = MountedCombatConfig {
        ..Default::default()
    };
    assert_eq!(average(&lance, settings, false), 16); // 3d8 + 3
    settings.trot_or_faster = true;
    assert_eq!(average(&lance, settings, false), 21); // 4d8 + 3, not 4d8 + 6
    settings.mount = MountType::RidingHorse;
    assert_eq!(average(&lance, settings, false), 16);
}

#[test]
fn mounted_warhorse_dice_need_trot_but_not_a_medium_target() {
    let mace = weapon("Mace", "2d6p");
    let mut settings = MountedCombatConfig {
        mount: MountType::Destrier,
        ..Default::default()
    };
    assert_eq!(average(&mace, settings, false), 7);
    settings.trot_or_faster = true;
    assert_eq!(average(&mace, settings, false), 14);
}

fn fighters() -> Vec<Combatant> {
    let mut sheet = CombatantSheet::default();
    sheet.offense.weapon = Arc::new(weapon("Lance", "2d8p+3"));
    sheet.offense.attack_bonus = 100;
    sheet.offense.strength_damage = 7;
    sheet.maneuvers.mounted = true;
    sheet.maneuvers.mounted_combat = MountedCombatConfig {
        trot_or_faster: true,
        ..Default::default()
    };
    let attacker = Combatant::new(sheet);
    let mut target = Combatant::new(CombatantSheet::default());
    target.sheet.vitals.infinite_hp = true;
    target.state.streamline_averages_incoming_damage = true;
    vec![attacker, target]
}

#[test]
fn mounted_bonus_reaches_real_primary_offhand_and_counter_attacks() {
    for slot in [WeaponSlot::Primary, WeaponSlot::Secondary] {
        let mut actors = fighters();
        actors[0].sheet.offense.offhand = Some(OffhandProfile {
            attack_bonus: 100,
            strength_damage: 7,
            weapon: actors[0].sheet.offense.weapon.clone(),
        });
        actors[0].sheet.maneuvers.dualwield_offhand_damage_penalty = 0;
        let result = resolve_attack(
            &mut actors,
            0,
            1,
            0,
            false,
            10.,
            AttackMode::Normal,
            slot,
            0.,
            None,
            &mut StdRng::seed_from_u64(7),
        );
        assert!(result.hit);
        assert_eq!(result.damage, 37); // 6d8 + 3 (weapon) + 7 (flat modifiers)
        let mut actors = fighters();
        let result = resolve_counter_attack(
            &mut actors,
            0,
            1,
            0.,
            WeaponSlot::Primary,
            true,
            false,
            false,
            false,
            false,
            1,
            &mut StdRng::seed_from_u64(7),
        );
        assert!(result.hit);
        assert_eq!(result.damage, 37);
    }
}

#[test]
fn mounted_toggle_off_and_ranged_attacks_do_not_gain_damage_dice() {
    let mut actors = fighters();
    let w = actors[0].sheet.offense.weapon.clone();
    actors[0].sheet.maneuvers.mounted = false;
    let plan = mounted_damage_plan(&actors[0], &actors[1], &w, false);
    assert_eq!(
        roll_mounted_weapon_damage(&w, plan, false, false, true, &mut StdRng::seed_from_u64(1)),
        12
    );
    actors[0].sheet.maneuvers.mounted = true;
    assert!(!mounted_damage_plan(&actors[0], &actors[1], &w, true).double_base_dice);
    assert_eq!(
        mounted_damage_plan(&actors[0], &actors[1], &w, true).extra_smallest,
        0
    );
}

#[test]
fn mounted_damage_is_independent_of_stout_and_sturdy_resistance() {
    let mut actors = fighters();
    actors[0].sheet.offense.weapon = Arc::new(weapon("Mace", "2d6p"));
    actors[1].sheet.defense.knockback_step = 25;
    assert_eq!(
        mounted_damage_plan(
            &actors[0],
            &actors[1],
            &actors[0].sheet.offense.weapon,
            false
        )
        .extra_smallest,
        1
    );
    actors[1].sheet.defense.is_medium_sized = false;
    assert_eq!(
        mounted_damage_plan(
            &actors[0],
            &actors[1],
            &actors[0].sheet.offense.weapon,
            false
        )
        .extra_smallest,
        0
    );
}

#[test]
fn mounted_shield_damage_doubles_only_dice_and_preserves_flat_modifiers() {
    let mut lance = weapon("Lance", "2d8p+3");
    lance.shield_damage_expr = Some("d8+3".into());
    lance.shield_damage_expr_cache = Some(DamageExprCache::new("d8+3"));
    let plan = MountedCombatConfig {
        trot_or_faster: true,
        ..Default::default()
    }
    .damage_plan("Lance", true);
    let (rolled, raw) =
        shield_block_raw_damage(&lance, plan, 7, 0, 1, true, &mut StdRng::seed_from_u64(1));
    assert_eq!(rolled, 21); // 4d8+3
    assert_eq!(raw, 28);
}

#[test]
fn mounted_random_penetrating_dice_match_the_equivalent_expression() {
    let mut lance = weapon("Lance", "2d8p");
    lance.force_nonpenetrating_damage = false;
    let plan = MountedCombatConfig {
        mount: MountType::Courser,
        trot_or_faster: true,
        ..Default::default()
    }
    .damage_plan("Lance", true);
    for seed in 0..100 {
        let actual = roll_mounted_weapon_damage(
            &lance,
            plan,
            false,
            false,
            false,
            &mut StdRng::seed_from_u64(seed),
        );
        let expected = DamageExprCache::new("7d8p").roll(&mut StdRng::seed_from_u64(seed), false);
        assert_eq!(actual, expected, "seed {seed}");
    }
}
