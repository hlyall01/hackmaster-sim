//! Gameplay orchestration (runs, loot, progression, and spawning).

pub mod config;
pub mod events;
pub mod harness;
pub mod loot;
pub mod progression;
pub mod run;
pub mod spawner;

pub use config::{AutobattlerConfig, LootConfig, LootItemConfig};
pub use events::{
    EventCatalog, EventCheck, EventCheckDifficulty, EventChoice, EventOutcome, EventResolution,
    EventResult, EventSpec, EventStat, EventTierGate, choose_event, resolve_event_choice,
    should_spawn_event,
};
pub use harness::{
    HarnessConfig, HarnessKpis, HarnessThresholds, evaluate_thresholds, run_seeded_harness,
};
pub use loot::{LootItemEntry, LootRoll, LootTable};
pub use progression::{LevelUpResult, XpCurve, apply_xp, apply_xp_with};
pub use run::{
    CombatantBuilder, DepthBand, EncounterTier, FightResult, Reward, RunOutcome, RunState, Wound,
    apply_downtime, apply_fight_result, depth_band_for_depth, encounter_tier_for_depth,
    run_next_fight,
};
pub use spawner::{EnemySpawnEntry, EnemySpawner};
