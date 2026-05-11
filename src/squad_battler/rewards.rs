//! Squad rewards, XP, and recruit offers.

use serde::{Deserialize, Serialize};

pub const DEFAULT_RECRUIT_OFFER_SIZE: usize = 3;

#[derive(Clone, Debug, Serialize)]
pub struct SquadReward {
    pub gold: u32,
    pub xp_per_survivor: u32,
    pub deaths: Vec<String>,
    pub level_ups: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecruitDestination {
    Active,
    Bench,
    Replace,
    Decline,
}
