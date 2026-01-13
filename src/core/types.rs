//! Core domain types (abilities, equipment, combatant sheet).

use crate::character::AbilitySet;
use crate::core::ids::NpcPresetId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct PlayerProfile {
    pub name: String,
    pub level: u8,
    pub xp: u32,
    pub base_stats: AbilitySet,
}

impl PlayerProfile {
    pub fn new(name: impl Into<String>, base_stats: AbilitySet) -> Self {
        Self {
            name: name.into(),
            level: 1,
            xp: 0,
            base_stats,
        }
    }
}

impl Default for PlayerProfile {
    fn default() -> Self {
        Self::new("Player", AbilitySet::default())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Inventory {
    pub gold: u32,
    pub items: Vec<String>,
}

impl Inventory {
    pub fn add_gold(&mut self, amount: u32) {
        self.gold = self.gold.saturating_add(amount);
    }

    pub fn add_item(&mut self, item: impl Into<String>) {
        self.items.push(item.into());
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TalentSpec {
    pub id: String,
    pub name: String,
    pub description: String,
    pub max_rank: u8,
    pub effects: Vec<TalentEffect>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TalentSelection {
    pub id: String,
    #[serde(default = "default_talent_rank")]
    pub rank: u8,
    #[serde(default)]
    pub weapon: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TalentEffect {
    HitPointBonus { amount: i32 },
    ArmorDrBonus { amount: i32 },
    AttackBonusWeapon { amount: i32 },
    DamageBonusWeapon { amount: i32 },
    Dodge {
        defense_bonus: i32,
        allow_dex_ranged: bool,
    },
    TraumaDieOverride {
        sides: i32,
        penetrating: bool,
    },
}

fn default_talent_rank() -> u8 {
    1
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemyProfile {
    pub level: u8,
    pub preset_id: NpcPresetId,
}
