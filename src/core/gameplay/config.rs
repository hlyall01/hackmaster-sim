use super::loot::{LootItemEntry, LootTable};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct AutobattlerConfig {
    pub seed: u64,
    pub fights_to_run: u32,
    pub max_fight_seconds: u32,
    pub rest_days_between_encounters: u32,
    pub enemy_weapon: String,
    pub player_preset_name: String,
    pub start_distance: f32,
    pub stop_distance: f32,
    pub loot: LootConfig,
}

impl Default for AutobattlerConfig {
    fn default() -> Self {
        Self {
            seed: 7,
            fights_to_run: 6,
            max_fight_seconds: 120,
            rest_days_between_encounters: 8,
            enemy_weapon: "Battle axe".to_string(),
            player_preset_name: "Arthur Du Randt".to_string(),
            start_distance: 20.0,
            stop_distance: 1.0,
            loot: LootConfig::default(),
        }
    }
}

impl AutobattlerConfig {
    pub fn to_loot_table(&self) -> LootTable {
        LootTable {
            gold_range: self.loot.gold_min..=self.loot.gold_max,
            xp_per_level: self.loot.xp_per_level,
            item_table: self
                .loot
                .items
                .iter()
                .filter(|entry| !entry.name.is_empty())
                .map(|entry| LootItemEntry {
                    name: entry.name.clone(),
                    weight: entry.weight,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct LootConfig {
    pub gold_min: u32,
    pub gold_max: u32,
    pub xp_per_level: u32,
    pub items: Vec<LootItemConfig>,
}

impl Default for LootConfig {
    fn default() -> Self {
        Self {
            gold_min: 8,
            gold_max: 16,
            xp_per_level: 18,
            items: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct LootItemConfig {
    pub name: String,
    pub weight: u32,
}

impl Default for LootItemConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            weight: 0,
        }
    }
}
