use bevy::prelude::Resource;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::autobattler::constants::{STAT_COUNT, TALENT_TAB_ALL};
use crate::character::{AbilityScore, AbilitySet};
use crate::core::gameplay::{RunOutcome, RunState, Wound};
use crate::core::rng::SimRng;
use crate::core::sim::SimState;
use crate::core::types::{EnemyProfile, Inventory, PlayerProfile, TalentSelection};
use crate::game_logic::{PlayerConfig, WeaponId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AppScreen {
    Start,
    Creation,
    Run,
    SpriteReview,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreationStep {
    Points,
    RollStats,
    ChooseRace,
    SpendBp,
    Talents,
}

impl CreationStep {
    pub fn title(self) -> &'static str {
        match self {
            CreationStep::Points => "Step 1: Starting Points",
            CreationStep::RollStats => "Step 2: Roll Ability Scores",
            CreationStep::ChooseRace => "Step 3: Choose Race",
            CreationStep::SpendBp => "Step 4: Spend BP on Stats",
            CreationStep::Talents => "Step 5: Purchase Talents",
        }
    }

    pub fn index(self) -> usize {
        match self {
            CreationStep::Points => 0,
            CreationStep::RollStats => 1,
            CreationStep::ChooseRace => 2,
            CreationStep::SpendBp => 3,
            CreationStep::Talents => 4,
        }
    }

    pub fn next(self) -> Option<Self> {
        match self {
            CreationStep::Points => Some(CreationStep::RollStats),
            CreationStep::RollStats => Some(CreationStep::ChooseRace),
            CreationStep::ChooseRace => Some(CreationStep::SpendBp),
            CreationStep::SpendBp => Some(CreationStep::Talents),
            CreationStep::Talents => None,
        }
    }

    pub fn prev(self) -> Option<Self> {
        match self {
            CreationStep::Points => None,
            CreationStep::RollStats => Some(CreationStep::Points),
            CreationStep::ChooseRace => Some(CreationStep::RollStats),
            CreationStep::SpendBp => Some(CreationStep::ChooseRace),
            CreationStep::Talents => Some(CreationStep::SpendBp),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunAction {
    FightOn,
    RestDay,
    Train,
}

impl RunAction {
    pub fn label(self) -> &'static str {
        match self {
            RunAction::FightOn => "Fight on",
            RunAction::RestDay => "Rest a day",
            RunAction::Train => "Train",
        }
    }

    pub fn rest_days(self) -> u32 {
        1
    }

    pub fn is_resting(self) -> bool {
        matches!(self, RunAction::RestDay)
    }
}

#[derive(Clone, Debug)]
pub struct RunViewState {
    pub run_state: RunState,
    pub last_outcome: Option<RunOutcome>,
    pub last_action: Option<RunAction>,
    pub last_log: Vec<String>,
    pub days_elapsed: u32,
    pub training_days: u32,
    pub run_over: bool,
    pub live_fight: Option<LiveFight>,
}

impl RunViewState {
    pub fn new(run_state: RunState) -> Self {
        Self {
            run_state,
            last_outcome: None,
            last_action: None,
            last_log: Vec::new(),
            days_elapsed: 0,
            training_days: 0,
            run_over: false,
            live_fight: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LiveFight {
    pub sim: SimState,
    pub enemy: EnemyProfile,
    pub action: Option<RunAction>,
    pub rest_days: u32,
    pub resting: bool,
    pub running: bool,
    pub time_scale: f32,
    pub max_seconds: u32,
    pub ui_elapsed: f32,
    pub seen_events: usize,
    pub log_lines: Vec<String>,
    pub float_seed: u32,
    pub floaters: Vec<DamageFloat>,
    pub pending_step: bool,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct DamageFloat {
    pub value: i32,
    pub target_idx: usize,
    pub start_time: f32,
    pub offset: f32,
    pub is_shield: bool,
}

#[derive(Clone, Debug)]
pub struct SaveEntry {
    pub file_name: String,
    pub display_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbilityScoreSave {
    pub base: u8,
    pub percentile: u8,
}

impl AbilityScoreSave {
    pub fn from_score(score: AbilityScore) -> Self {
        Self {
            base: score.base,
            percentile: score.percentile,
        }
    }

    pub fn to_score(&self) -> AbilityScore {
        AbilityScore::new(self.base, self.percentile)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterSave {
    pub version: u32,
    pub name: String,
    pub stats: Vec<AbilityScoreSave>,
    pub race_id: Option<String>,
    pub talents: Vec<TalentSelection>,
    pub bp_history: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AbilitySetSave {
    pub strength: AbilityScoreSave,
    pub intelligence: u8,
    pub wisdom: u8,
    pub dexterity: AbilityScoreSave,
    pub constitution: u8,
    pub looks: u8,
    pub charisma: u8,
}

impl AbilitySetSave {
    pub fn from_set(set: AbilitySet) -> Self {
        Self {
            strength: AbilityScoreSave::from_score(set.strength),
            intelligence: set.intelligence,
            wisdom: set.wisdom,
            dexterity: AbilityScoreSave::from_score(set.dexterity),
            constitution: set.constitution,
            looks: set.looks,
            charisma: set.charisma,
        }
    }

    pub fn to_set(&self) -> AbilitySet {
        AbilitySet {
            strength: self.strength.to_score(),
            intelligence: self.intelligence,
            wisdom: self.wisdom,
            dexterity: self.dexterity.to_score(),
            constitution: self.constitution,
            looks: self.looks,
            charisma: self.charisma,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerProfileSave {
    pub name: String,
    pub level: u8,
    pub xp: u32,
    pub base_stats: AbilitySetSave,
    pub talents: Vec<TalentSelection>,
}

impl PlayerProfileSave {
    pub fn from_profile(profile: &PlayerProfile) -> Self {
        Self {
            name: profile.name.clone(),
            level: profile.level,
            xp: profile.xp,
            base_stats: AbilitySetSave::from_set(profile.base_stats),
            talents: profile.talents.clone(),
        }
    }

    pub fn to_profile(&self) -> PlayerProfile {
        PlayerProfile {
            name: self.name.clone(),
            level: self.level,
            xp: self.xp,
            base_stats: self.base_stats.to_set(),
            talents: self.talents.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InventorySave {
    pub gold: u32,
    pub items: Vec<String>,
}

impl InventorySave {
    pub fn from_inventory(inventory: &Inventory) -> Self {
        Self {
            gold: inventory.gold,
            items: inventory.items.clone(),
        }
    }

    pub fn to_inventory(&self) -> Inventory {
        Inventory {
            gold: self.gold,
            items: self.items.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WoundSave {
    pub damage: u32,
    pub healing_progress_quarter_days: u32,
}

impl WoundSave {
    pub fn from_wound(wound: &Wound) -> Self {
        Self {
            damage: wound.damage,
            healing_progress_quarter_days: wound.healing_progress_quarter_days,
        }
    }

    pub fn to_wound(&self) -> Wound {
        Wound {
            damage: self.damage,
            healing_progress_quarter_days: self.healing_progress_quarter_days,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunStateSave {
    pub player: PlayerProfileSave,
    pub inventory: InventorySave,
    pub run_depth: u32,
    pub wounds: Vec<WoundSave>,
}

impl RunStateSave {
    pub fn from_state(state: &RunState) -> Self {
        Self {
            player: PlayerProfileSave::from_profile(&state.player),
            inventory: InventorySave::from_inventory(&state.inventory),
            run_depth: state.run_depth,
            wounds: state.wounds.iter().map(WoundSave::from_wound).collect(),
        }
    }

    pub fn to_state(&self) -> RunState {
        RunState {
            player: self.player.to_profile(),
            inventory: self.inventory.to_inventory(),
            run_depth: self.run_depth,
            wounds: self.wounds.iter().map(WoundSave::to_wound).collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunSave {
    pub version: u32,
    pub name: String,
    pub character: CharacterSave,
    pub run_state: RunStateSave,
    pub days_elapsed: u32,
    pub training_days: u32,
    pub run_over: bool,
    pub last_action: Option<RunAction>,
    pub last_log: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PointPool {
    pub bp: i32,
    pub lp: i32,
    pub ap: i32,
    pub rp: i32,
}

impl PointPool {
    pub fn new(bp: i32, lp: i32, ap: i32, rp: i32) -> Self {
        Self { bp, lp, ap, rp }
    }

    pub fn add(self, other: Self) -> Self {
        Self {
            bp: self.bp + other.bp,
            lp: self.lp + other.lp,
            ap: self.ap + other.ap,
            rp: self.rp + other.rp,
        }
    }

    pub fn sub(self, other: Self) -> Self {
        Self {
            bp: self.bp - other.bp,
            lp: self.lp - other.lp,
            ap: self.ap - other.ap,
            rp: self.rp - other.rp,
        }
    }

    pub fn can_afford(self, cost: Self) -> bool {
        self.bp >= cost.bp && self.lp >= cost.lp && self.ap >= cost.ap && self.rp >= cost.rp
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RolledSet {
    pub rolls: [AbilityScore; STAT_COUNT],
}

impl RolledSet {
    pub fn roll(rng: &mut SimRng) -> Self {
        let mut counts = [0u8; STAT_COUNT];
        for _ in 0..56 {
            loop {
                let face = rng.gen_range(0..STAT_COUNT);
                if counts[face] < 15 {
                    counts[face] += 1;
                    break;
                }
            }
        }
        let mut rolls = [AbilityScore::new(10, 0); STAT_COUNT];
        for (idx, roll) in rolls.iter_mut().enumerate() {
            let base = counts[idx].saturating_add(3);
            let percentile = rng.gen_range(1..=100);
            *roll = AbilityScore::new(base, percentile);
        }
        Self { rolls }
    }
}

pub struct CreationState {
    pub name: String,
    pub rng: SimRng,
    pub rolled_sets: [RolledSet; 2],
    pub selected_set: usize,
    pub assignments: [Option<usize>; STAT_COUNT],
    pub stats: [AbilityScore; STAT_COUNT],
    pub stats_locked: bool,
    pub race_index: Option<usize>,
    pub race_applied: bool,
    pub bp_history: [Vec<u8>; STAT_COUNT],
    pub talent_category: String,
    pub player: PlayerConfig,
}

impl CreationState {
    pub fn new(weapon_id: WeaponId) -> Self {
        let mut rng = SimRng::default();
        let rolled_sets = [RolledSet::roll(&mut rng), RolledSet::roll(&mut rng)];
        let player = PlayerConfig::new("Adventurer", weapon_id);
        Self {
            name: "Adventurer".to_string(),
            rng,
            rolled_sets,
            selected_set: 0,
            assignments: [None; STAT_COUNT],
            stats: [AbilityScore::new(10, 1); STAT_COUNT],
            stats_locked: false,
            race_index: None,
            race_applied: false,
            bp_history: std::array::from_fn(|_| Vec::new()),
            talent_category: TALENT_TAB_ALL.to_string(),
            player,
        }
    }

    pub fn reset_rolls(&mut self) {
        self.rolled_sets = [RolledSet::roll(&mut self.rng), RolledSet::roll(&mut self.rng)];
        self.selected_set = 0;
        self.assignments = [None; STAT_COUNT];
        self.stats_locked = false;
        self.race_index = None;
        self.race_applied = false;
        self.bp_history = std::array::from_fn(|_| Vec::new());
        self.talent_category = TALENT_TAB_ALL.to_string();
    }

    pub fn assign_roll(&mut self, stat_idx: usize, roll_idx: usize) {
        for (idx, slot) in self.assignments.iter_mut().enumerate() {
            if idx != stat_idx && *slot == Some(roll_idx) {
                *slot = None;
            }
        }
        self.assignments[stat_idx] = Some(roll_idx);
    }

    pub fn lock_assignments(&mut self) {
        if !self.assignments.iter().all(|slot| slot.is_some()) {
            return;
        }
        let selected = &self.rolled_sets[self.selected_set];
        for (stat_idx, slot) in self.assignments.iter().enumerate() {
            let roll_idx = slot.unwrap_or(0);
            self.stats[stat_idx] = selected.rolls[roll_idx];
        }
        self.stats_locked = true;
        self.bp_history = std::array::from_fn(|_| Vec::new());
        self.race_index = None;
        self.race_applied = false;
        self.player.talents.clear();
        self.player.race_id = None;
        self.player.race_applied = false;
        self.player.base_hp = 10;
        self.sync_player_from_stats();
    }

    pub fn sync_player_from_stats(&mut self) {
        self.player.name = self.name.clone();
        self.player.strength_base = self.stats[0].base;
        self.player.strength_pct = self.stats[0].percentile;
        self.player.intelligence = self.stats[1].base;
        self.player.wisdom = self.stats[2].base;
        self.player.dex_base = self.stats[3].base;
        self.player.dex_pct = self.stats[3].percentile;
        self.player.constitution = self.stats[4].base;
        self.player.looks = self.stats[5].base;
        let charisma_delta = crate::character::looks_charisma_adjustment(self.stats[5].base);
        self.player.charisma =
            crate::autobattler::logic::clamp_stat_adjustment(self.stats[6].base, charisma_delta);
    }
}

#[derive(Resource)]
pub struct AutobattlerState {
    pub app: crate::autobattler::app::AutobattlerApp,
}

#[derive(Resource)]
pub struct SpriteReviewState {
    pub stage: SpriteReviewStage,
    pub race_index: usize,
    pub races: Vec<String>,
    pub frames_since_refresh: u32,
    pub awaiting_capture: bool,
    pub needs_refresh: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpriteReviewStage {
    Weapons,
    Pained,
}

impl SpriteReviewState {
    pub fn new(races: Vec<String>) -> Self {
        Self {
            stage: SpriteReviewStage::Weapons,
            race_index: 0,
            races,
            frames_since_refresh: 0,
            awaiting_capture: false,
            needs_refresh: true,
        }
    }

    pub fn current_race(&self) -> Option<&str> {
        self.races.get(self.race_index).map(String::as_str)
    }
}
