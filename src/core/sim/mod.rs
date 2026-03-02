//! Simulation engine and state transitions.

mod combat;
mod engine;
mod modifiers;
mod movement;
mod types;

pub use engine::{BulkSimResult, SimState, bulk_simulate};
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

#[cfg(test)]
mod tests;
