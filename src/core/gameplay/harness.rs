use crate::core::gameplay::{
    CombatantBuilder, EnemySpawner, LootTable, RunState, encounter_tier_for_depth, run_next_fight,
};
use crate::core::rng::derive_seed;
use crate::core::sim::SimConfig;
use crate::core::types::{Inventory, PlayerProfile};

#[derive(Clone, Copy, Debug)]
pub struct HarnessConfig {
    pub runs: u32,
    pub max_encounters_per_run: u32,
    pub base_seed: u64,
    pub max_fight_seconds: u32,
    pub rest_days: u32,
    pub resting: bool,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            runs: 64,
            max_encounters_per_run: 40,
            base_seed: 7,
            max_fight_seconds: 90,
            rest_days: 8,
            resting: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HarnessKpis {
    pub runs: u32,
    pub failures: u32,
    pub average_depth: f32,
    pub fights_per_level: f32,
    pub resource_spend_split: f32,
    pub average_gold_gained: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct HarnessThresholds {
    pub max_failure_rate: f32,
    pub min_average_depth: f32,
    pub max_resource_spend_split: f32,
}

impl Default for HarnessThresholds {
    fn default() -> Self {
        Self {
            max_failure_rate: 0.95,
            min_average_depth: 1.0,
            max_resource_spend_split: 1.0,
        }
    }
}

pub fn evaluate_thresholds(kpis: &HarnessKpis, thresholds: &HarnessThresholds) -> Vec<String> {
    let mut failures = Vec::new();
    let failure_rate = if kpis.runs == 0 {
        0.0
    } else {
        kpis.failures as f32 / kpis.runs as f32
    };
    if failure_rate > thresholds.max_failure_rate {
        failures.push(format!(
            "failure rate {:.3} exceeds {:.3}",
            failure_rate, thresholds.max_failure_rate
        ));
    }
    if kpis.average_depth < thresholds.min_average_depth {
        failures.push(format!(
            "average depth {:.2} below {:.2}",
            kpis.average_depth, thresholds.min_average_depth
        ));
    }
    if kpis.resource_spend_split > thresholds.max_resource_spend_split {
        failures.push(format!(
            "resource spend split {:.3} exceeds {:.3}",
            kpis.resource_spend_split, thresholds.max_resource_spend_split
        ));
    }
    failures
}

pub fn run_seeded_harness<B: CombatantBuilder>(
    base_player: &PlayerProfile,
    base_inventory: &Inventory,
    spawner: &EnemySpawner,
    loot_table: &LootTable,
    sim_config: SimConfig,
    builder: &B,
    config: HarnessConfig,
) -> HarnessKpis {
    let mut total_depth = 0u64;
    let mut total_fights = 0u64;
    let mut total_levels_gained = 0u64;
    let mut total_gold_gained = 0u64;
    let mut total_gold_spent = 0u64;
    let mut failures = 0u32;

    for run_idx in 0..config.runs {
        let run_seed = derive_seed(config.base_seed, "harness-run", run_idx as u64);
        let mut state = RunState::new(base_player.clone(), base_inventory.clone(), run_seed);
        let start_level = state.player.level;
        let mut prev_gold = state.inventory.gold;
        let mut run_failed = true;

        for _ in 0..config.max_encounters_per_run {
            let tier = encounter_tier_for_depth(state.run_depth);
            let outcome = run_next_fight(
                state,
                spawner,
                loot_table,
                None,
                sim_config,
                config.max_fight_seconds,
                config.rest_days,
                config.resting,
                tier,
                builder,
            );
            total_fights = total_fights.saturating_add(1);
            let next_gold = outcome.state.inventory.gold;
            if next_gold >= prev_gold {
                total_gold_gained =
                    total_gold_gained.saturating_add((next_gold - prev_gold) as u64);
            } else {
                total_gold_spent = total_gold_spent.saturating_add((prev_gold - next_gold) as u64);
            }
            prev_gold = next_gold;
            state = outcome.state;
            if !outcome.fight.won {
                run_failed = true;
                break;
            }
            run_failed = false;
        }

        if run_failed {
            failures = failures.saturating_add(1);
        }
        total_depth = total_depth.saturating_add(state.run_depth as u64);
        total_levels_gained = total_levels_gained
            .saturating_add(state.player.level.saturating_sub(start_level) as u64);
    }

    let runs = config.runs.max(1);
    let denom_levels = total_levels_gained.max(1) as f32;
    let spend_total = total_gold_gained.saturating_add(total_gold_spent);
    let resource_spend_split = if spend_total == 0 {
        0.0
    } else {
        total_gold_spent as f32 / spend_total as f32
    };

    HarnessKpis {
        runs: config.runs,
        failures,
        average_depth: total_depth as f32 / runs as f32,
        fights_per_level: total_fights as f32 / denom_levels,
        resource_spend_split,
        average_gold_gained: total_gold_gained as f32 / runs as f32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::gameplay::EnemySpawnEntry;
    use crate::core::ids::NpcPresetId;
    use crate::core::sim::Combatant;
    use crate::core::types::EnemyProfile;

    #[derive(Clone, Copy)]
    struct DummyBuilder;

    impl CombatantBuilder for DummyBuilder {
        fn build_player(&self, _state: &RunState) -> Combatant {
            let mut combatant = Combatant::default();
            combatant.sheet.name = "Player".to_string();
            combatant
        }

        fn build_enemy(&self, _enemy: &EnemyProfile) -> Combatant {
            let mut combatant = Combatant::default();
            combatant.sheet.name = "Enemy".to_string();
            combatant
        }
    }

    #[test]
    fn harness_is_deterministic_for_seed() {
        let spawner = EnemySpawner::new(vec![EnemySpawnEntry {
            preset_id: NpcPresetId::new(0),
            min_level: 1,
            max_level: 99,
            weight: 1,
        }]);
        let loot_table = LootTable {
            gold_range: 5..=10,
            xp_per_level: 1,
            item_table: Vec::new(),
        };
        let sim_config = SimConfig::new(20.0, 1.0);
        let builder = DummyBuilder;
        let cfg = HarnessConfig {
            runs: 8,
            max_encounters_per_run: 12,
            base_seed: 42,
            max_fight_seconds: 45,
            rest_days: 8,
            resting: true,
        };
        let a = run_seeded_harness(
            &PlayerProfile::default(),
            &Inventory::default(),
            &spawner,
            &loot_table,
            sim_config,
            &builder,
            cfg,
        );
        let b = run_seeded_harness(
            &PlayerProfile::default(),
            &Inventory::default(),
            &spawner,
            &loot_table,
            sim_config,
            &builder,
            cfg,
        );
        assert_eq!(a, b);
    }

    #[test]
    fn thresholds_flag_out_of_bounds_metrics() {
        let kpis = HarnessKpis {
            runs: 10,
            failures: 10,
            average_depth: 0.2,
            fights_per_level: 20.0,
            resource_spend_split: 0.8,
            average_gold_gained: 1.0,
        };
        let thresholds = HarnessThresholds {
            max_failure_rate: 0.5,
            min_average_depth: 1.0,
            max_resource_spend_split: 0.3,
        };
        let issues = evaluate_thresholds(&kpis, &thresholds);
        assert_eq!(issues.len(), 3);
    }
}
