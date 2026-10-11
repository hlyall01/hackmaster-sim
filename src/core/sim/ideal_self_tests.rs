use super::*;
use crate::core::rules::DamageExprCache;
use rand::{SeedableRng, rngs::StdRng};
use std::sync::Arc;

fn actor(ideal: bool) -> Combatant {
    let mut actor = Combatant::default();
    actor.magic.level = 20;
    actor.sheet.vitals.max_hp = 1000;
    actor.sheet.vitals.threshold_of_pain = 1000;
    actor.sheet.defense.knockback_step = 1000;
    actor.sheet.offense.attack_bonus = 100;
    actor.sheet.offense.strength_damage = 7;
    let weapon = Arc::make_mut(&mut actor.sheet.offense.weapon);
    weapon.damage_expr = "20".into();
    weapon.damage_expr_cache = DamageExprCache::new("20");
    weapon.crit_min_roll = 1000;
    if ideal {
        actor.sheet.modifiers.add_i32(StatIdI32::FlagIdealSelf, ModifierOpI32::Set(1));
    }
    actor.reset_state();
    actor
}

#[test]
fn ideal_self_matches_every_level_in_the_supplied_table() {
    let expected = [
        (0, 0, 0), (0, 0, 0), (0, 0, 0), (0, 0, 0),
        (0, 1, 0), (1, 1, 0), (1, 2, 0), (1, 3, 1),
        (2, 3, 1), (2, 4, 1), (2, 5, 1), (3, 5, 2),
        (3, 6, 2), (3, 7, 2), (4, 7, 2), (4, 8, 3),
        (4, 9, 3), (5, 9, 3), (5, 10, 3), (5, 11, 4),
    ];
    for (index, (damage, feat, dr)) in expected.into_iter().enumerate() {
        let mut actor = actor(true);
        actor.magic.level = index as u8 + 1;
        assert_eq!(actor.ideal_self_bonuses(), IdealSelfBonuses {
            damage, feat_of_strength: feat, damage_reduction: dr,
        }, "level {}", index + 1);
    }
}

#[test]
fn ideal_self_tracks_health_and_applies_once_to_each_attack_kind() {
    let mut actor = actor(true);
    actor.sheet.offense.offhand = Some(OffhandProfile {
        weapon: actor.sheet.offense.weapon.clone(), attack_bonus: 100, strength_damage: 7,
    });
    for hp in [1000, 999, 900, 999, 1000] {
        actor.state.hp = hp;
        let (damage, feat, dr) = if hp == 1000 { (5, 11, 4) } else { (0, 0, 0) };
        assert_eq!(actor.ideal_self_bonuses().feat_of_strength, feat);
        assert_eq!(actor.spell_damage_reduction(false), dr);
        assert_eq!(actor.spell_damage_reduction(true), dr);
        assert_eq!(actor.apply_i32(StatIdI32::StrengthDamageBase, 7), 7 + damage);
        for slot in [WeaponSlot::Primary, WeaponSlot::Secondary] {
            assert_eq!(combat::preview_damage_bonus(&actor, slot, false), 7 + damage);
        }
        // Projectile attacks omit Strength but retain the separate talent modifier.
        Arc::make_mut(&mut actor.sheet.offense.weapon).uses_projectiles = true;
        assert_eq!(combat::preview_damage_bonus(&actor, WeaponSlot::Primary, true), damage);
        Arc::make_mut(&mut actor.sheet.offense.weapon).uses_projectiles = false;
        // Power Attack doubles Strength, not Ideal Self.
        actor.sheet.maneuvers.power_attack = true;
        actor.sheet.offense.strength_damage = 14;
        assert_eq!(combat::preview_damage_bonus(&actor, WeaponSlot::Primary, false), 14 + damage);
        actor.sheet.maneuvers.power_attack = false;
        actor.sheet.offense.strength_damage = 7;
    }
    actor.state.hp = 0;
    assert_eq!(actor.ideal_self_bonuses(), IdealSelfBonuses::default());
}

fn strike(pair: &mut [Combatant]) -> AttackEvent {
    test_support::resolve_basic_attack(pair, 0, 1, 0, false, 3.0, 10.0,
        &mut StdRng::seed_from_u64(7))
}

#[test]
fn ideal_self_protects_the_first_wound_then_expires_and_full_healing_restores_it() {
    let mut pair = vec![actor(false), actor(true)];
    pair[1].sheet.defense.armor_dr = 3;
    pair[1].sheet.defense.natural_dr = 1;
    let first = strike(&mut pair);
    assert!(first.hit);
    assert_eq!(first.damage_breakdown.unwrap().effective_armor_dr, 7);
    assert_eq!(first.damage, 20);
    assert_eq!(pair[1].ideal_self_bonuses(), IdealSelfBonuses::default());
    let second = strike(&mut pair);
    assert_eq!(second.damage, 24);
    pair[1].state.hp = pair[1].sheet.vitals.max_hp;
    assert_eq!(strike(&mut pair).damage, 20);
    // Fully absorbed hits leave the talent active.
    pair[1].state.hp = pair[1].sheet.vitals.max_hp;
    pair[1].sheet.defense.armor_dr = 100;
    assert_eq!(strike(&mut pair).damage, 0);
    assert_eq!(pair[1].ideal_self_bonuses().damage_reduction, 4);
}

#[test]
fn ideal_self_changes_real_attack_damage_and_survives_a_fight_reset() {
    let mut pair = vec![actor(true), actor(false)];
    assert_eq!(strike(&mut pair).damage, 32);
    pair[0].state.hp -= 1;
    assert_eq!(strike(&mut pair).damage, 27);
    pair[0].reset_state();
    assert_eq!(strike(&mut pair).damage, 32);
}
