//! Core domain types (abilities, equipment, combatant sheet).

use crate::character::AbilitySet;
use crate::core::ids::NpcPresetId;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Talent {
    Placeholder,
}

#[derive(Clone, Copy, Debug)]
pub struct TalentSpec {
    pub talent: Talent,
    pub name: &'static str,
    pub description: &'static str,
    pub max_rank: u8,
}

impl Talent {
    pub fn spec(self) -> TalentSpec {
        match self {
            Talent::Placeholder => TalentSpec {
                talent: self,
                name: "Placeholder",
                description: "No effect yet.",
                max_rank: 1,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemyProfile {
    pub level: u8,
    pub preset_id: NpcPresetId,
}
