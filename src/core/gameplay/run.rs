use crate::core::sim::CombatEvent;
use crate::core::types::{Inventory, PlayerProfile};

#[derive(Clone, Debug)]
pub struct RunState {
    pub player: PlayerProfile,
    pub inventory: Inventory,
    pub run_depth: u32,
}

impl RunState {
    pub fn new(player: PlayerProfile, inventory: Inventory) -> Self {
        Self {
            player,
            inventory,
            run_depth: 0,
        }
    }

    pub fn apply_reward(&mut self, reward: &Reward) {
        self.inventory.add_gold(reward.gold);
        self.inventory.items.extend(reward.items.iter().cloned());
        self.player.xp = self.player.xp.saturating_add(reward.xp);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reward {
    pub gold: u32,
    pub xp: u32,
    pub items: Vec<String>,
}

impl Reward {
    pub fn is_empty(&self) -> bool {
        self.gold == 0 && self.xp == 0 && self.items.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct FightResult {
    pub won: bool,
    pub remaining_hp: i32,
    pub turns: u32,
    pub events: Vec<CombatEvent>,
}

#[derive(Clone, Debug)]
pub struct RunOutcome {
    pub state: RunState,
    pub fight: FightResult,
    pub reward: Option<Reward>,
}
