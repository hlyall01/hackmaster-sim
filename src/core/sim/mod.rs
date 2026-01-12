//! Simulation engine and state transitions.

mod combat;
mod engine;
mod movement;
mod types;

pub use engine::{bulk_simulate, BulkSimResult, SimState};
pub use movement::{max_range_for_bands, max_range_for_weapon_name};
pub use types::{
    AttackEvent, CombatEvent, CombatEventKind, Combatant, CombatantSheet, CombatantState,
    DefenseProfile, KnockAsideEvent, ManeuverProfile, MobilityProfile, OffenseProfile,
    ShieldBreakageStep, SimActor, SimConfig, Vitals, WeaponProfile,
};

#[cfg(test)]
mod tests;
