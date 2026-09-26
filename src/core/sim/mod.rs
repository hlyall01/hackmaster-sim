//! Simulation engine and state transitions.

pub(crate) mod combat;
pub(crate) mod magic;
mod spell_effects;
pub use spell_effects::ConfiguredEffect;
pub use combat::weapon_damage_expression;
pub use magic::{
    MagicProfile, MagicState, ScheduledEcho, SpellDefinition, SpellEffect, SpellEvent,
    SpellEventKind, SpellRequest,
};
mod damage_sources;
pub use damage_sources::{DamageSource, DamageSourceStats};
mod engine;
mod intimidation;
mod knockback;
pub use knockback::{KnockbackRule, knockback_rule_for_attack};
mod modifiers;
mod movement;
mod mounted;
pub use mounted::{HorseGender, MountedCombatConfig, MountType, RidingMastery, MountedTargetSize};
mod types;

pub use engine::{
    BulkSimResult, DetailedSimStats, DetailedTeamStats, SimState, bulk_simulate,
    bulk_simulate_with_seed, bulk_simulate_with_seed_controlled,
};
pub use modifiers::{
    CHRONOBLUR_DURATION_SECONDS, CHRONOBLUR_EFFECT_ID, CHRONOBLUR_MELEE_DEFENSE_BONUS,
    CHRONOBLUR_RANGED_DISTANCE_FEET, ModifierOpF32, ModifierOpI32, ModifierStack,
    STREAMLINE_DURATION_SECONDS, STREAMLINE_EFFECT_ID, STREAMLINE_RADIUS_FEET, StatIdF32,
    StatIdI32, TemporaryEffect, modifiers_for_magic_item,
};
pub use movement::{max_range_for_bands, max_range_for_weapon_name, range_bands_for_weapon_name};
pub use types::{
    AttackEvent, AttackRollBreakdown, AttackSource, CalledShotDelayProfile, CombatEvent,
    CombatEventKind, Combatant, CombatantCache, CombatantSheet, CombatantState,
    CombatantTacticalProfile, CriticalHit, DamageBreakdown, DamageDie, DefenseProfile, GridPos,
    KnockAsideEvent, KnockAsideRollBreakdown, ManeuverProfile, MobilityProfile, OffenseProfile,
    OffhandProfile, ShieldBreakageStep, ShieldDamageBreakdown, SimActor, SimConfig, TacticalEvent,
    TacticalProfileKey, Vitals, WeaponCache, WeaponProfile, WeaponSlot,
};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod weapon_style_tests;
#[cfg(test)]
mod magic_tests;

pub(crate) fn counter_event(counter: combat::CounterAttackOutcome) -> AttackEvent {
    AttackEvent {
        source: crate::core::sim::AttackSource::Weapon,
        hit: counter.hit,
        shield_block: counter.shield_block,
        damage: counter.damage,
        shield_damage: counter.shield_damage,
        knockback_ft: counter.knockback_ft,
        hold_at_bay: false,
        is_charge: false,
        is_aggressive: false,
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
    }
}

#[cfg(test)]
pub(crate) mod test_support;
