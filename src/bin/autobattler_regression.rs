use hackmaster_sim::core::gameplay::{
    CombatantBuilder, EnemySpawnEntry, EnemySpawner, HarnessConfig, HarnessThresholds, LootTable,
    RunState, evaluate_thresholds, run_seeded_harness,
};
use hackmaster_sim::core::ids::NpcPresetId;
use hackmaster_sim::core::sim::{Combatant, SimConfig};
use hackmaster_sim::core::types::{EnemyProfile, Inventory, PlayerProfile};

#[derive(Clone, Copy)]
struct DummyBuilder;

impl CombatantBuilder for DummyBuilder {
    fn build_player(&self, _state: &RunState) -> Combatant {
        let mut c = Combatant::default();
        c.sheet.name = "Player".to_string();
        c
    }

    fn build_enemy(&self, _enemy: &EnemyProfile) -> Combatant {
        let mut c = Combatant::default();
        c.sheet.name = "Enemy".to_string();
        c
    }
}

fn main() {
    let cfg = HarnessConfig::default();
    let thresholds = HarnessThresholds::default();
    let spawner = EnemySpawner::new(vec![EnemySpawnEntry {
        preset_id: NpcPresetId::new(0),
        min_level: 1,
        max_level: 99,
        weight: 1,
    }]);
    let loot = LootTable {
        gold_range: 6..=16,
        xp_per_level: 2,
        item_table: Vec::new(),
    };
    let kpis = run_seeded_harness(
        &PlayerProfile::default(),
        &Inventory::default(),
        &spawner,
        &loot,
        SimConfig::new(20.0, 1.0),
        &DummyBuilder,
        cfg,
    );
    println!("Regression harness:");
    println!("  runs: {}", kpis.runs);
    println!("  failures: {}", kpis.failures);
    println!("  avg_depth: {:.2}", kpis.average_depth);
    println!("  fights_per_level: {:.2}", kpis.fights_per_level);
    println!("  resource_spend_split: {:.3}", kpis.resource_spend_split);
    println!("  avg_gold_gained: {:.2}", kpis.average_gold_gained);
    let issues = evaluate_thresholds(&kpis, &thresholds);
    if issues.is_empty() {
        println!("Thresholds: OK");
    } else {
        println!("Thresholds: FAILED");
        for issue in issues {
            println!("  - {issue}");
        }
        std::process::exit(2);
    }
}
