//! Simulation engine and state transitions.

mod combat;
mod engine;
mod movement;
mod types;

pub use engine::{bulk_simulate, BulkSimResult, SimState};
pub use movement::{max_range_for_bands, max_range_for_weapon_name, range_bands_for_weapon_name};
pub use types::{
    AttackEvent, AttackRollBreakdown, CombatEvent, CombatEventKind, Combatant, CombatantSheet,
    CombatantState, CriticalHit, DamageBreakdown, DefenseProfile, KnockAsideEvent,
    KnockAsideRollBreakdown, ManeuverProfile, MobilityProfile, OffenseProfile, ShieldBreakageStep,
    ShieldDamageBreakdown, SimActor, SimConfig, Vitals, WeaponProfile,
};

#[cfg(test)]
mod tests;
