//! Attack adapter used only by simulator regression tests.

use crate::core::sim::{AttackEvent, Combatant, WeaponSlot};

pub(crate) fn resolve_basic_attack(
    combatants: &mut [Combatant],
    attacker_idx: usize,
    defender_idx: usize,
    range_mod: i32,
    is_ranged: bool,
    distance_ft: f32,
    now: f32,
    rng: &mut impl rand::Rng,
) -> AttackEvent {
    let outcome = crate::core::sim::combat::resolve_attack(
        combatants,
        attacker_idx,
        defender_idx,
        range_mod,
        is_ranged,
        distance_ft,
        crate::core::sim::combat::AttackMode::Normal,
        WeaponSlot::Primary,
        now,
        None,
        rng,
    );
    AttackEvent {
        source: crate::core::sim::AttackSource::Weapon,
        hit: outcome.hit,
        shield_block: outcome.shield_block,
        damage: outcome.damage,
        shield_damage: outcome.shield_damage,
        knockback_ft: outcome.knockback_ft,
        hold_at_bay: outcome.hold_at_bay,
        is_charge: false,
        is_aggressive: outcome.is_aggressive,
        weapon_slot: outcome.weapon_slot,
        use_jab: outcome.use_jab,
        is_ranged: outcome.is_ranged,
        trauma_applied: outcome.trauma_applied,
        trauma_seconds: outcome.trauma_seconds,
        roll: outcome.roll,
        damage_breakdown: outcome.damage_breakdown,
        shield_damage_breakdown: outcome.shield_damage_breakdown,
        defender_hp_after: outcome.defender_hp_after,
        critical: outcome.critical,
    }
}
