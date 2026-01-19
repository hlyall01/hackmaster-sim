//! Core domain types (abilities, equipment, combatant sheet).

use crate::character::{AbilitySet, AbilitySetFull, Progression};
use crate::core::ids::NpcPresetId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct PlayerProfile {
    pub name: String,
    pub level: u8,
    pub xp: u32,
    pub base_stats: AbilitySet,
    pub ability_scores_full: AbilitySetFull,
    pub progression: Progression,
    pub points: PointPools,
    pub banked_points: PointPools,
    pub honor: i32,
    pub alignment: Option<String>,
    pub race_id: Option<String>,
    pub background: Option<String>,
    pub quirks: Vec<String>,
    pub flaws: Vec<String>,
    pub skills: Vec<String>,
    pub proficiencies: Vec<String>,
    pub talents: Vec<TalentSelection>,
}

impl PlayerProfile {
    pub fn new(name: impl Into<String>, base_stats: AbilitySet) -> Self {
        Self {
            name: name.into(),
            level: 1,
            xp: 0,
            base_stats,
            ability_scores_full: AbilitySetFull::from(base_stats),
            progression: Progression::default(),
            points: PointPools::default(),
            banked_points: PointPools::default(),
            honor: 0,
            alignment: None,
            race_id: None,
            background: None,
            quirks: Vec::new(),
            flaws: Vec::new(),
            skills: Vec::new(),
            proficiencies: Vec::new(),
            talents: Vec::new(),
        }
    }
}

impl Default for PlayerProfile {
    fn default() -> Self {
        Self::new("Player", AbilitySet::default())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct PointPools {
    pub bp: i32,
    pub lp: i32,
    pub ap: i32,
    pub rp: i32,
}

impl PointPools {
    pub fn new(bp: i32, lp: i32, ap: i32, rp: i32) -> Self {
        Self { bp, lp, ap, rp }
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AbilityAdjustments {
    #[serde(default)]
    pub strength: i32,
    #[serde(default)]
    pub dexterity: i32,
    #[serde(default)]
    pub intelligence: i32,
    #[serde(default)]
    pub wisdom: i32,
    #[serde(default)]
    pub constitution: i32,
    #[serde(default)]
    pub looks: i32,
    #[serde(default)]
    pub charisma: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RaceSpec {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub category: String,
    pub base_hp: u32,
    pub size: String,
    #[serde(default)]
    pub knockback_size: Option<String>,
    #[serde(default)]
    pub ability_adjustments: AbilityAdjustments,
    #[serde(default)]
    pub pros: Vec<String>,
    #[serde(default)]
    pub cons: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AbilityKind {
    Strength,
    Dexterity,
    Intelligence,
    Wisdom,
    Constitution,
    Looks,
    Charisma,
}

impl AbilityKind {
    pub fn label(self) -> &'static str {
        match self {
            AbilityKind::Strength => "Strength",
            AbilityKind::Dexterity => "Dexterity",
            AbilityKind::Intelligence => "Intelligence",
            AbilityKind::Wisdom => "Wisdom",
            AbilityKind::Constitution => "Constitution",
            AbilityKind::Looks => "Looks",
            AbilityKind::Charisma => "Charisma",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TalentRequirement {
    MinLevel { level: u8 },
    MinStat {
        stat: AbilityKind,
        min_base: Option<u8>,
        min_percentile: Option<u8>,
    },
    RequiresTalent {
        id: String,
        min_rank: Option<u8>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TalentSpec {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub cost_bp: Option<u32>,
    #[serde(default)]
    pub cost_lp: Option<u32>,
    #[serde(default)]
    pub cost_rp: Option<u32>,
    #[serde(default = "default_talent_category")]
    pub category: String,
    #[serde(default)]
    pub race_categories: Vec<String>,
    #[serde(default)]
    pub race_ids: Vec<String>,
    #[serde(default)]
    pub requirements: Vec<TalentRequirement>,
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
    SpeedModBonus { amount: i32 },
    InitiativeModBonus { amount: i32 },
    AttackBonusWeapon { amount: i32 },
    DamageBonusWeapon { amount: i32 },
    DamageBonusWeaponGroup { amount: i32, weapon_group: String },
    DefenseBonusWeapon { amount: i32 },
    InitiativeDieBonus { steps: i32 },
    Dodge {
        defense_bonus: i32,
        allow_dex_ranged: bool,
    },
    TraumaDieOverride {
        sides: i32,
        penetrating: bool,
    },
    ThresholdOfPainMultiplier { multiplier: f32 },
    ThresholdOfPainLevelBonus { per_level_pct: f32 },
    FastHealer,
    WeaponSpeedBonus {
        amount: i32,
        #[serde(default)]
        ranged_only: bool,
        #[serde(default)]
        weapon_group: Option<String>,
    },
    WeaponReachBonus { amount: i32 },
    RangeDistanceMultiplier { multiplier: f32 },
    ArmorInitiativePenaltyNegation,
    ArmorSpeedPenaltyNegation,
    ArmorDrBonusArmored { amount: i32 },
    LightArmorDefenseBonusFromDr { divisor: i32 },
    MediumArmorDrBonus { amount: i32 },
    MediumArmorDefensePenaltyReduction { amount: i32 },
    HeavyArmorDamageBonusFromDr { divisor: i32 },
    HeavyArmorDamageBonus { amount: i32 },
    ShieldDefenseBonus { amount: i32 },
    ShieldCoverValueAdjustment { amount: i32 },
}

fn default_talent_rank() -> u8 {
    1
}

fn default_talent_category() -> String {
    "Uncategorized".to_string()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnemyProfile {
    pub level: u8,
    pub preset_id: NpcPresetId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ability_kind_from_json() {
        let parsed: AbilityKind = serde_json::from_str("\"dexterity\"").unwrap();
        assert_eq!(parsed, AbilityKind::Dexterity);
    }

    #[test]
    fn parse_talent_requirements_from_json() {
        let json = r#"
        {
          "id": "tough_hide",
          "name": "Tough Hide",
          "description": "Test",
          "category": "Defense",
          "requirements": [
            { "type": "min_level", "level": 3 },
            { "type": "min_stat", "stat": "constitution", "min_base": 12 },
            { "type": "requires_talent", "id": "tough_as_nails", "min_rank": 1 }
          ],
          "max_rank": 1,
          "effects": []
        }
        "#;
        let parsed: TalentSpec = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.requirements.len(), 3);
        match &parsed.requirements[1] {
            TalentRequirement::MinStat {
                stat,
                min_base,
                min_percentile,
            } => {
                assert_eq!(*stat, AbilityKind::Constitution);
                assert_eq!(*min_base, Some(12));
                assert_eq!(*min_percentile, None);
            }
            _ => panic!("expected min_stat requirement"),
        }
    }
}
