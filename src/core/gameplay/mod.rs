//! Gameplay orchestration (runs, loot, progression, and spawning).

pub mod loot;
pub mod progression;
pub mod run;
pub mod spawner;
pub mod config;

pub use config::{AutobattlerConfig, LootConfig, LootItemConfig};
pub use loot::{LootItemEntry, LootRoll, LootTable};
pub use progression::{apply_xp, apply_xp_with, LevelUpResult, XpCurve};
pub use run::{run_next_fight, CombatantBuilder, FightResult, Reward, RunOutcome, RunState, Wound};
pub use spawner::{EnemySpawnEntry, EnemySpawner};
