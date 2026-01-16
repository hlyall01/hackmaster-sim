//! Simulation engine and state transitions.

mod combat;
mod engine;
mod modifiers;
mod movement;
mod types;

pub use engine::{bulk_simulate, BulkSimResult, SimState};
pub use movement::{max_range_for_bands, max_range_for_weapon_name, range_bands_for_weapon_name};
pub use modifiers::{
    modifiers_for_magic_item, ModifierOpF32, ModifierOpI32, ModifierStack, StatIdF32, StatIdI32,
    TemporaryEffect,
};
pub use types::{
    AttackEvent, AttackRollBreakdown, CombatEvent, CombatEventKind, Combatant, CombatantSheet,
    CombatantState, CombatantCache, CriticalHit, DamageBreakdown, DamageDie, DefenseProfile,
    KnockAsideEvent, KnockAsideRollBreakdown, ManeuverProfile, MobilityProfile, OffenseProfile,
    OffhandProfile, ShieldBreakageStep, ShieldDamageBreakdown, SimActor, SimConfig, Vitals,
    WeaponCache, WeaponProfile, WeaponSlot,
};

#[cfg(test)]
mod tests;
