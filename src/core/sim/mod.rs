//! Simulation engine and state transitions.

mod combat;
mod engine;
mod modifiers;
mod movement;
mod types;

pub use engine::{BulkSimResult, SimState, bulk_simulate, bulk_simulate_with_seed};
pub use modifiers::{
    ModifierOpF32, ModifierOpI32, ModifierStack, StatIdF32, StatIdI32, TemporaryEffect,
    modifiers_for_magic_item,
};
pub use movement::{max_range_for_bands, max_range_for_weapon_name, range_bands_for_weapon_name};
pub use types::{
    AttackEvent, AttackRollBreakdown, CalledShotDelayProfile, CombatEvent, CombatEventKind,
    Combatant, CombatantCache, CombatantSheet, CombatantState, CriticalHit, DamageBreakdown,
    DamageDie, DefenseProfile, GridPos, KnockAsideEvent, KnockAsideRollBreakdown, ManeuverProfile,
    MobilityProfile, OffenseProfile, OffhandProfile, ShieldBreakageStep, ShieldDamageBreakdown,
    SimActor, SimConfig, Vitals, WeaponCache, WeaponProfile, WeaponSlot,
};

#[derive(Clone, Debug)]
pub(crate) struct BasicAttackResult {
    pub event: AttackEvent,
    pub counter_attack: Option<AttackEvent>,
}

pub(crate) fn resolve_basic_attack(
    combatants: &mut [Combatant],
    attacker_idx: usize,
    defender_idx: usize,
    range_mod: i32,
    is_ranged: bool,
    distance_ft: f32,
    now: f32,
    rng: &mut impl rand::Rng,
) -> BasicAttackResult {
    let outcome = combat::resolve_attack(
        combatants,
        attacker_idx,
        defender_idx,
        range_mod,
        is_ranged,
        distance_ft,
        combat::AttackMode::Normal,
        WeaponSlot::Primary,
        now,
        None,
        rng,
    );
    let event = AttackEvent {
        hit: outcome.hit,
        shield_block: outcome.shield_block,
        damage: outcome.damage,
        shield_damage: outcome.shield_damage,
        knockback_ft: outcome.knockback_ft,
        hold_at_bay: outcome.hold_at_bay,
        is_charge: false,
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
    };
    let counter_attack = outcome.counter_attack.map(|counter| AttackEvent {
        hit: counter.hit,
        shield_block: counter.shield_block,
        damage: counter.damage,
        shield_damage: counter.shield_damage,
        knockback_ft: counter.knockback_ft,
        hold_at_bay: false,
        is_charge: false,
        weapon_slot: counter.weapon_slot,
        use_jab: counter.use_jab,
        is_ranged: counter.is_ranged,
        trauma_applied: counter.trauma_applied,
        trauma_seconds: counter.trauma_seconds,
        roll: counter.roll,
        damage_breakdown: counter.damage_breakdown,
        shield_damage_breakdown: counter.shield_damage_breakdown,
        defender_hp_after: counter.defender_hp_after,
        critical: counter.critical,
    });
    BasicAttackResult {
        event,
        counter_attack,
    }
}

#[cfg(test)]
mod tests;
