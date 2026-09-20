//! Squad rewards, XP, and recruit offers.

use crate::core::gameplay::EncounterTier;
use serde::{Deserialize, Serialize};

pub const DEFAULT_RECRUIT_OFFER_SIZE: usize = 3;
pub const MAX_RECRUIT_OFFER_SIZE: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecruitRarity {
    Common,
    Veteran,
    Elite,
}

impl RecruitRarity {
    pub fn label(self) -> &'static str {
        match self {
            RecruitRarity::Common => "Common",
            RecruitRarity::Veteran => "Veteran",
            RecruitRarity::Elite => "Elite",
        }
    }

    pub fn level_bonus(self) -> u8 {
        match self {
            RecruitRarity::Common => 0,
            RecruitRarity::Veteran => 1,
            RecruitRarity::Elite => 2,
        }
    }

    pub fn hp_bonus(self) -> u8 {
        match self {
            RecruitRarity::Common => 0,
            RecruitRarity::Veteran => 1,
            RecruitRarity::Elite => 3,
        }
    }

    pub fn stat_bonus(self) -> u8 {
        match self {
            RecruitRarity::Common => 0,
            RecruitRarity::Veteran => 1,
            RecruitRarity::Elite => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct RecruitRarityWeights {
    pub common: u32,
    pub veteran: u32,
    pub elite: u32,
}

impl RecruitRarityWeights {
    pub fn total(self) -> u32 {
        self.common
            .saturating_add(self.veteran)
            .saturating_add(self.elite)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct RecruitOfferScaling {
    pub offer_size: usize,
    pub min_level: u8,
    pub max_level: u8,
    pub rarity_weights: RecruitRarityWeights,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SquadReward {
    pub gold: u32,
    pub xp_per_survivor: u32,
    pub deaths: Vec<String>,
    pub level_ups: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecruitDestination {
    Active,
    Bench,
    Replace,
    Decline,
}

pub fn recruit_offer_scaling(depth: u32, tier: EncounterTier) -> RecruitOfferScaling {
    let tier_bonus = tier_recruit_bonus(tier);
    let depth_level_bonus = (depth / 3).min(u32::from(u8::MAX)) as u8;
    let min_level = 1_u8
        .saturating_add(depth_level_bonus)
        .saturating_add(tier_bonus);
    let max_level = min_level
        .saturating_add(1)
        .saturating_add((depth / 5).min(1) as u8)
        .saturating_add(tier_bonus.min(1));
    let offer_size = (DEFAULT_RECRUIT_OFFER_SIZE + (depth >= 4) as usize + tier_bonus as usize)
        .min(MAX_RECRUIT_OFFER_SIZE);

    RecruitOfferScaling {
        offer_size,
        min_level,
        max_level,
        rarity_weights: recruit_rarity_weights(depth, tier),
    }
}

pub fn recruit_level_for(depth: u32, tier: EncounterTier, rarity: RecruitRarity) -> u8 {
    let scaling = recruit_offer_scaling(depth, tier);
    scaling
        .min_level
        .saturating_add(rarity.level_bonus())
        .min(scaling.max_level)
}

pub fn recruit_rarity_weights(depth: u32, tier: EncounterTier) -> RecruitRarityWeights {
    let tier_bonus = u32::from(tier_recruit_bonus(tier));
    let depth_pressure = depth.min(10);
    RecruitRarityWeights {
        common: 80_u32
            .saturating_sub(depth_pressure.saturating_mul(4))
            .saturating_sub(tier_bonus.saturating_mul(12))
            .max(30),
        veteran: 18_u32
            .saturating_add(depth_pressure.saturating_mul(3))
            .saturating_add(tier_bonus.saturating_mul(7))
            .min(55),
        elite: 2_u32
            .saturating_add(depth_pressure)
            .saturating_add(tier_bonus.saturating_mul(5))
            .min(30),
    }
}

pub fn recruit_rarity_for_roll(roll: u32, weights: RecruitRarityWeights) -> RecruitRarity {
    let total = weights.total().max(1);
    let roll = roll % total;
    if roll < weights.elite {
        RecruitRarity::Elite
    } else if roll < weights.elite.saturating_add(weights.veteran) {
        RecruitRarity::Veteran
    } else {
        RecruitRarity::Common
    }
}

fn tier_recruit_bonus(tier: EncounterTier) -> u8 {
    match tier {
        EncounterTier::Normal => 0,
        EncounterTier::Elite => 1,
        EncounterTier::Boss => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recruit_scaling_increases_offer_quality_with_depth_and_tier() {
        let early = recruit_offer_scaling(0, EncounterTier::Normal);
        let late_elite = recruit_offer_scaling(6, EncounterTier::Elite);

        assert!(late_elite.min_level > early.min_level);
        assert!(late_elite.offer_size > early.offer_size);
        assert!(late_elite.rarity_weights.elite > early.rarity_weights.elite);
        assert!(late_elite.rarity_weights.common < early.rarity_weights.common);
    }

    #[test]
    fn recruit_rarity_roll_uses_weight_bands() {
        let weights = RecruitRarityWeights {
            common: 70,
            veteran: 20,
            elite: 10,
        };

        assert_eq!(recruit_rarity_for_roll(0, weights), RecruitRarity::Elite);
        assert_eq!(recruit_rarity_for_roll(10, weights), RecruitRarity::Veteran);
        assert_eq!(recruit_rarity_for_roll(30, weights), RecruitRarity::Common);
    }
}
