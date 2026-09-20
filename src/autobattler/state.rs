use bevy::prelude::Resource;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::autobattler::constants::{RUN_SAVE_VERSION, SAVE_VERSION, STAT_COUNT, TALENT_TAB_ALL};
use crate::character::{AbilityScore, AbilitySet, AbilitySetFull, Progression};
use crate::core::gameplay::{DepthBand, EncounterTier, EventSpec, RunOutcome, RunState, Wound};
use crate::core::rng::{SimRng, derive_seed};
use crate::core::sim::SimState;
use crate::core::types::{
    EnemyProfile, Inventory, PlayerProfile, PointPools, SkillProgress, TalentSelection,
    WeaponMasteryProgress,
};
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
    Alignment,
    FinalizeStats,
    Honor,
    Priors,
    QuirksFlaws,
    AdvancementTalents,
    SkillsTalents,
    HitPoints,
    DerivedStats,
    MoneyGear,
}

impl CreationStep {
    pub fn title(self) -> &'static str {
        match self {
            CreationStep::Points => "Step 1: Starting Points",
            CreationStep::RollStats => "Step 2: Roll Ability Scores",
            CreationStep::ChooseRace => "Step 3: Choose Race",
            CreationStep::Alignment => "Step 4: Choose Alignment",
            CreationStep::FinalizeStats => "Step 5: Finalize Ability Scores",
            CreationStep::Honor => "Step 6: Calculate Starting Honor",
            CreationStep::Priors => "Step 7: Priors and Particulars",
            CreationStep::QuirksFlaws => "Step 8: Quirks and Flaws",
            CreationStep::AdvancementTalents => "Step 9: Record Advancement Talents",
            CreationStep::SkillsTalents => "Step 10: Skills, Talents, Proficiencies",
            CreationStep::HitPoints => "Step 11: Determine Hit Points",
            CreationStep::DerivedStats => "Step 12: Record Derived Statistics",
            CreationStep::MoneyGear => "Step 13: Money and Gear",
        }
    }

    pub fn index(self) -> usize {
        match self {
            CreationStep::Points => 0,
            CreationStep::RollStats => 1,
            CreationStep::ChooseRace => 2,
            CreationStep::Alignment => 3,
            CreationStep::FinalizeStats => 4,
            CreationStep::Honor => 5,
            CreationStep::Priors => 6,
            CreationStep::QuirksFlaws => 7,
            CreationStep::AdvancementTalents => 8,
            CreationStep::SkillsTalents => 9,
            CreationStep::HitPoints => 10,
            CreationStep::DerivedStats => 11,
            CreationStep::MoneyGear => 12,
        }
    }

    pub fn next(self) -> Option<Self> {
        match self {
            CreationStep::Points => Some(CreationStep::RollStats),
            CreationStep::RollStats => Some(CreationStep::ChooseRace),
            CreationStep::ChooseRace => Some(CreationStep::Alignment),
            CreationStep::Alignment => Some(CreationStep::FinalizeStats),
            CreationStep::FinalizeStats => Some(CreationStep::Honor),
            CreationStep::Honor => Some(CreationStep::Priors),
            CreationStep::Priors => Some(CreationStep::QuirksFlaws),
            CreationStep::QuirksFlaws => Some(CreationStep::AdvancementTalents),
            CreationStep::AdvancementTalents => Some(CreationStep::SkillsTalents),
            CreationStep::SkillsTalents => Some(CreationStep::HitPoints),
            CreationStep::HitPoints => Some(CreationStep::DerivedStats),
            CreationStep::DerivedStats => Some(CreationStep::MoneyGear),
            CreationStep::MoneyGear => None,
        }
    }

    pub fn prev(self) -> Option<Self> {
        match self {
            CreationStep::Points => None,
            CreationStep::RollStats => Some(CreationStep::Points),
            CreationStep::ChooseRace => Some(CreationStep::RollStats),
            CreationStep::Alignment => Some(CreationStep::ChooseRace),
            CreationStep::FinalizeStats => Some(CreationStep::Alignment),
            CreationStep::Honor => Some(CreationStep::FinalizeStats),
            CreationStep::Priors => Some(CreationStep::Honor),
            CreationStep::QuirksFlaws => Some(CreationStep::Priors),
            CreationStep::AdvancementTalents => Some(CreationStep::QuirksFlaws),
            CreationStep::SkillsTalents => Some(CreationStep::AdvancementTalents),
            CreationStep::HitPoints => Some(CreationStep::SkillsTalents),
            CreationStep::DerivedStats => Some(CreationStep::HitPoints),
            CreationStep::MoneyGear => Some(CreationStep::DerivedStats),
        }
    }

    pub fn count() -> usize {
        13
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunAction {
    #[serde(alias = "FightOn", alias = "RestDay")]
    Rest,
    #[serde(alias = "Train")]
    Activity,
}

impl RunAction {
    pub fn label(self) -> &'static str {
        match self {
            RunAction::Rest => "Rest",
            RunAction::Activity => "Activity",
        }
    }

    pub fn is_resting(self) -> bool {
        matches!(self, RunAction::Rest)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DowntimeActivity {
    #[default]
    Acrobatics,
    AnimalTraining,
    Athletics,
    Begging,
    Carousing,
    Climbing,
    Crafting,
    Foraging,
    Gambling,
    Healing,
    Hunting,
    Jumping,
    Laboring,
    Meditating,
    Performing,
    Reading,
    RepairingRefitting,
    Riding,
    Scouting,
    SkillTutoring,
    SkillTraining,
    Sparring,
    Swimming,
    WeaponDrills,
}

impl DowntimeActivity {
    pub const ALL: [DowntimeActivity; 24] = [
        DowntimeActivity::Acrobatics,
        DowntimeActivity::AnimalTraining,
        DowntimeActivity::Athletics,
        DowntimeActivity::Begging,
        DowntimeActivity::Carousing,
        DowntimeActivity::Climbing,
        DowntimeActivity::Crafting,
        DowntimeActivity::Foraging,
        DowntimeActivity::Gambling,
        DowntimeActivity::Healing,
        DowntimeActivity::Hunting,
        DowntimeActivity::Jumping,
        DowntimeActivity::Laboring,
        DowntimeActivity::Meditating,
        DowntimeActivity::Performing,
        DowntimeActivity::Reading,
        DowntimeActivity::RepairingRefitting,
        DowntimeActivity::Riding,
        DowntimeActivity::Scouting,
        DowntimeActivity::SkillTutoring,
        DowntimeActivity::SkillTraining,
        DowntimeActivity::Sparring,
        DowntimeActivity::Swimming,
        DowntimeActivity::WeaponDrills,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DowntimeActivity::Acrobatics => "Acrobatics",
            DowntimeActivity::AnimalTraining => "Animal Training",
            DowntimeActivity::Athletics => "Athletics",
            DowntimeActivity::Begging => "Begging",
            DowntimeActivity::Carousing => "Carousing",
            DowntimeActivity::Climbing => "Climbing",
            DowntimeActivity::Crafting => "Crafting",
            DowntimeActivity::Foraging => "Foraging",
            DowntimeActivity::Gambling => "Gambling",
            DowntimeActivity::Healing => "Healing",
            DowntimeActivity::Hunting => "Hunting",
            DowntimeActivity::Jumping => "Jumping",
            DowntimeActivity::Laboring => "Laboring",
            DowntimeActivity::Meditating => "Meditating",
            DowntimeActivity::Performing => "Performing",
            DowntimeActivity::Reading => "Reading",
            DowntimeActivity::RepairingRefitting => "Repairing and Refitting",
            DowntimeActivity::Riding => "Riding",
            DowntimeActivity::Scouting => "Scouting",
            DowntimeActivity::SkillTutoring => "Skill Tutoring",
            DowntimeActivity::SkillTraining => "Skill Training",
            DowntimeActivity::Sparring => "Sparring",
            DowntimeActivity::Swimming => "Swimming",
            DowntimeActivity::WeaponDrills => "Weapon Drills",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            DowntimeActivity::Acrobatics => "+d12p DEX.",
            DowntimeActivity::AnimalTraining => {
                "+d6p WIS; animal progress +1 week (requires Animal Training skill)."
            }
            DowntimeActivity::Athletics => "+d6p STR, +d6p CON.",
            DowntimeActivity::Begging => {
                "+d6p CHA; difficult Persuasion check for coins; free Persuasion if unskilled."
            }
            DowntimeActivity::Carousing => "+d6p CON, +d6p CHA; spend 5d6p coins.",
            DowntimeActivity::Climbing => "+d6p STR, +d6p DEX; free Climbing if unskilled.",
            DowntimeActivity::Crafting => {
                "+d6p INT; difficult check for coin gain/failure cost (first reward branch for now)."
            }
            DowntimeActivity::Foraging => {
                "+d3p INT, +d3p WIS; difficult check for coin gain (first reward branch for now)."
            }
            DowntimeActivity::Gambling => {
                "-d6p WIS, +d12p CHA; difficult Gambling check for coin swing."
            }
            DowntimeActivity::Healing => "+d6p WIS; difficult First Aid check affects Honor.",
            DowntimeActivity::Hunting => {
                "+d3p WIS, +d3p DEX; difficult Hunting check for gains, failure can wound."
            }
            DowntimeActivity::Jumping => "+d12p STR; free Jumping if unskilled.",
            DowntimeActivity::Laboring => "+d6p CON; +1 Honor (first reward branch).",
            DowntimeActivity::Meditating => "+d12p WIS.",
            DowntimeActivity::Performing => {
                "+d6p CHA; difficult check for coin gain; Fame ignored."
            }
            DowntimeActivity::Reading => "+d12p INT; difficult Literacy check grants +1 LP.",
            DowntimeActivity::RepairingRefitting => {
                "+d6p INT; difficult check for coin gain (first reward branch)."
            }
            DowntimeActivity::Riding => "+d6p DEX; difficult Riding check or take d4p wound.",
            DowntimeActivity::Scouting => {
                "+d3p WIS, +d3p CON; Observation/Survival outcomes with possible d4p wound."
            }
            DowntimeActivity::SkillTutoring => "+d6p INT; +2 LP.",
            DowntimeActivity::SkillTraining => "+d6p INT; +1 LP.",
            DowntimeActivity::Sparring => {
                "+d3p STR/DEX/CON, take d4p wound; trauma save d20 vs floor(base CON/2) sets weapon XP result."
            }
            DowntimeActivity::Swimming => {
                "Difficult Swimming check: fail +d12p CON, success +d6p STR and +d6p CON."
            }
            DowntimeActivity::WeaponDrills => {
                "+d3p STR, +d3p DEX; weapon XP gain (first reward branch)."
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct DowntimeFeedback {
    pub title: String,
    pub activity: Option<DowntimeActivity>,
    pub lines: Vec<String>,
    pub animation_seconds: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LevelUpCheckpoint {
    pub levels_gained: u8,
    pub total_slots: u8,
    pub bp_slots: u8,
    pub lp_slots: u8,
    pub ap_slots: u8,
    pub rp_slots: u8,
}

impl LevelUpCheckpoint {
    pub fn new(levels_gained: u8) -> Self {
        let total_slots = levels_gained.saturating_mul(4);
        Self {
            levels_gained,
            total_slots,
            bp_slots: 0,
            lp_slots: 0,
            ap_slots: 0,
            rp_slots: 0,
        }
    }

    pub fn used_slots(&self) -> u8 {
        self.bp_slots
            .saturating_add(self.lp_slots)
            .saturating_add(self.ap_slots)
            .saturating_add(self.rp_slots)
    }

    pub fn remaining_slots(&self) -> u8 {
        self.total_slots.saturating_sub(self.used_slots())
    }

    pub fn grants(&self) -> PointPool {
        PointPool {
            bp: i32::from(self.bp_slots) * 5,
            lp: i32::from(self.lp_slots),
            ap: i32::from(self.ap_slots),
            rp: i32::from(self.rp_slots),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RunViewState {
    pub run_state: RunState,
    pub seed_context: SeedContext,
    pub pending_encounter: Option<EncounterPreview>,
    pub pending_event: Option<EventPreview>,
    pub last_outcome: Option<RunOutcome>,
    pub last_action: Option<RunAction>,
    pub last_log: Vec<String>,
    pub days_elapsed: u32,
    pub training_days: u32,
    pub run_over: bool,
    pub awaiting_downtime_choice: bool,
    pub pending_levelup: Option<LevelUpCheckpoint>,
    pub selected_activity: DowntimeActivity,
    pub downtime_feedback: Option<DowntimeFeedback>,
    pub live_fight: Option<LiveFight>,
}

impl RunViewState {
    pub fn new(run_state: RunState) -> Self {
        Self {
            run_state,
            seed_context: SeedContext::default(),
            pending_encounter: None,
            pending_event: None,
            last_outcome: None,
            last_action: None,
            last_log: Vec::new(),
            days_elapsed: 0,
            training_days: 0,
            run_over: false,
            awaiting_downtime_choice: false,
            pending_levelup: None,
            selected_activity: DowntimeActivity::default(),
            downtime_feedback: None,
            live_fight: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EncounterPreview {
    pub enemy: EnemyProfile,
    pub tier: EncounterTier,
    pub enemy_name: String,
    pub armor_label: String,
    pub weapon_name: String,
}

#[derive(Clone, Debug)]
pub struct EventPreview {
    pub event: EventSpec,
    pub tier: EncounterTier,
    pub resolve_seed: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SeedContext {
    pub run_seed: u64,
    pub spawn_seed: Option<u64>,
    pub combat_seed: Option<u64>,
    pub loot_seed: Option<u64>,
    pub event_seed: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct LiveFight {
    pub sim: SimState,
    pub enemy: EnemyProfile,
    pub tier: EncounterTier,
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
            percentile: score.percentile % 100,
        }
    }

    pub fn to_score(&self) -> AbilityScore {
        let percentile = self.percentile % 100;
        AbilityScore::new(self.base, percentile)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillProgressSave {
    pub id: String,
    #[serde(default)]
    pub level: u8,
}

impl SkillProgressSave {
    pub fn from_skill_progress(progress: &SkillProgress) -> Self {
        Self {
            id: progress.id.clone(),
            level: progress.level,
        }
    }

    pub fn to_skill_progress(&self) -> SkillProgress {
        SkillProgress {
            id: self.id.clone(),
            level: self.level.min(100),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CharacterSave {
    #[serde(default = "default_character_save_version")]
    pub version: u32,
    pub name: String,
    pub stats: Vec<AbilityScoreSave>,
    pub race_id: Option<String>,
    pub talents: Vec<TalentSelection>,
    pub bp_history: Vec<Vec<u8>>,
    #[serde(default)]
    pub weapon_name: String,
    #[serde(default)]
    pub armor_label: String,
    #[serde(default)]
    pub shield_name: String,
    #[serde(default)]
    pub alignment: String,
    #[serde(default)]
    pub honor: i32,
    #[serde(default)]
    pub background: String,
    #[serde(default)]
    pub height: String,
    #[serde(default)]
    pub weight: String,
    #[serde(default)]
    pub age: String,
    #[serde(default)]
    pub handedness: String,
    #[serde(default)]
    pub quirks: Vec<String>,
    #[serde(default)]
    pub flaws: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub skill_levels: Vec<SkillProgressSave>,
    #[serde(default)]
    pub proficiencies: Vec<String>,
    #[serde(default)]
    pub starting_money: u32,
    #[serde(default)]
    pub money_rolled: bool,
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
pub struct AbilitySetFullSave {
    pub strength: AbilityScoreSave,
    pub intelligence: AbilityScoreSave,
    pub wisdom: AbilityScoreSave,
    pub dexterity: AbilityScoreSave,
    pub constitution: AbilityScoreSave,
    pub looks: AbilityScoreSave,
    pub charisma: AbilityScoreSave,
}

impl AbilitySetFullSave {
    pub fn from_set(set: AbilitySetFull) -> Self {
        Self {
            strength: AbilityScoreSave::from_score(set.strength),
            intelligence: AbilityScoreSave::from_score(set.intelligence),
            wisdom: AbilityScoreSave::from_score(set.wisdom),
            dexterity: AbilityScoreSave::from_score(set.dexterity),
            constitution: AbilityScoreSave::from_score(set.constitution),
            looks: AbilityScoreSave::from_score(set.looks),
            charisma: AbilityScoreSave::from_score(set.charisma),
        }
    }

    pub fn to_set(&self) -> AbilitySetFull {
        AbilitySetFull {
            strength: self.strength.to_score(),
            intelligence: self.intelligence.to_score(),
            wisdom: self.wisdom.to_score(),
            dexterity: self.dexterity.to_score(),
            constitution: self.constitution.to_score(),
            looks: self.looks.to_score(),
            charisma: self.charisma.to_score(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerProfileSave {
    pub name: String,
    pub level: u8,
    pub xp: u32,
    pub base_stats: AbilitySetSave,
    #[serde(default)]
    pub ability_scores_full: Option<AbilitySetFullSave>,
    #[serde(default)]
    pub progression: Progression,
    #[serde(default)]
    pub points: PointPools,
    #[serde(default)]
    pub banked_points: PointPools,
    #[serde(default)]
    pub honor: i32,
    #[serde(default)]
    pub alignment: Option<String>,
    #[serde(default)]
    pub race_id: Option<String>,
    #[serde(default)]
    pub background: Option<String>,
    #[serde(default)]
    pub quirks: Vec<String>,
    #[serde(default)]
    pub flaws: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub skill_levels: Vec<SkillProgressSave>,
    #[serde(default)]
    pub proficiencies: Vec<String>,
    #[serde(default)]
    pub weapon_masteries: Vec<WeaponMasteryProgress>,
    pub talents: Vec<TalentSelection>,
}

impl PlayerProfileSave {
    pub fn from_profile(profile: &PlayerProfile) -> Self {
        Self {
            name: profile.name.clone(),
            level: profile.level,
            xp: profile.xp,
            base_stats: AbilitySetSave::from_set(profile.base_stats),
            ability_scores_full: Some(AbilitySetFullSave::from_set(profile.ability_scores_full)),
            progression: profile.progression,
            points: profile.points,
            banked_points: profile.banked_points,
            honor: profile.honor,
            alignment: profile.alignment.clone(),
            race_id: profile.race_id.clone(),
            background: profile.background.clone(),
            quirks: profile.quirks.clone(),
            flaws: profile.flaws.clone(),
            skills: profile.skills.clone(),
            skill_levels: profile
                .skill_levels
                .iter()
                .map(SkillProgressSave::from_skill_progress)
                .collect(),
            proficiencies: profile.proficiencies.clone(),
            weapon_masteries: profile.weapon_masteries.clone(),
            talents: profile.talents.clone(),
        }
    }

    pub fn to_profile(&self) -> PlayerProfile {
        let base_stats = self.base_stats.to_set();
        let ability_scores_full = self
            .ability_scores_full
            .as_ref()
            .map(AbilitySetFullSave::to_set)
            .unwrap_or_else(|| AbilitySetFull::from(base_stats));
        let skill_levels = if self.skill_levels.is_empty() {
            crate::core::skills::derive_skill_levels_from_legacy(&self.skills, &ability_scores_full)
        } else {
            self.skill_levels
                .iter()
                .map(SkillProgressSave::to_skill_progress)
                .collect()
        };
        let legacy_skills = if self.skills.is_empty() {
            crate::core::skills::legacy_skill_names(&skill_levels)
        } else {
            self.skills.clone()
        };
        PlayerProfile {
            name: self.name.clone(),
            level: self.level,
            xp: self.xp,
            base_stats: AbilitySet::from(ability_scores_full),
            ability_scores_full,
            progression: self.progression,
            points: self.points,
            banked_points: self.banked_points,
            honor: self.honor,
            alignment: self.alignment.clone(),
            race_id: self.race_id.clone(),
            background: self.background.clone(),
            quirks: self.quirks.clone(),
            flaws: self.flaws.clone(),
            skills: legacy_skills,
            skill_levels,
            proficiencies: self.proficiencies.clone(),
            weapon_masteries: self.weapon_masteries.clone(),
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
    #[serde(default, alias = "healing_progress_quarter_days")]
    pub healing_progress_steps: u32,
}

impl WoundSave {
    pub fn from_wound(wound: &Wound) -> Self {
        Self {
            damage: wound.damage,
            healing_progress_steps: wound.healing_progress_steps,
        }
    }

    pub fn to_wound(&self) -> Wound {
        Wound {
            damage: self.damage,
            healing_progress_steps: self.healing_progress_steps,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunStateSave {
    pub player: PlayerProfileSave,
    pub inventory: InventorySave,
    pub run_depth: u32,
    #[serde(default)]
    pub seed: u64,
    #[serde(default)]
    pub encounter_index: u32,
    #[serde(default)]
    pub last_encounter_tier: EncounterTier,
    #[serde(default)]
    pub last_encounter_band: DepthBand,
    #[serde(default)]
    pub event_flags: Vec<String>,
    #[serde(default)]
    pub seen_event_ids: Vec<String>,
    pub wounds: Vec<WoundSave>,
}

impl RunStateSave {
    pub fn from_state(state: &RunState) -> Self {
        Self {
            player: PlayerProfileSave::from_profile(&state.player),
            inventory: InventorySave::from_inventory(&state.inventory),
            run_depth: state.run_depth,
            seed: state.run_seed,
            encounter_index: state.encounter_index,
            last_encounter_tier: state.last_encounter_tier,
            last_encounter_band: state.last_encounter_band,
            event_flags: state.event_flags.clone(),
            seen_event_ids: state.seen_event_ids.clone(),
            wounds: state.wounds.iter().map(WoundSave::from_wound).collect(),
        }
    }

    pub fn to_state(&self) -> RunState {
        RunState {
            player: self.player.to_profile(),
            inventory: self.inventory.to_inventory(),
            run_depth: self.run_depth,
            run_seed: self.seed,
            encounter_index: self.encounter_index,
            last_encounter_tier: self.last_encounter_tier,
            last_encounter_band: self.last_encounter_band,
            event_flags: self.event_flags.clone(),
            seen_event_ids: self.seen_event_ids.clone(),
            wounds: self.wounds.iter().map(WoundSave::to_wound).collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunSave {
    #[serde(default = "default_run_save_version")]
    pub version: u32,
    pub name: String,
    pub character: CharacterSave,
    pub run_state: RunStateSave,
    pub days_elapsed: u32,
    pub training_days: u32,
    pub run_over: bool,
    #[serde(default)]
    pub awaiting_downtime_choice: bool,
    #[serde(default)]
    pub pending_levelup: Option<LevelUpCheckpoint>,
    pub last_action: Option<RunAction>,
    #[serde(default)]
    pub selected_activity: DowntimeActivity,
    pub last_log: Vec<String>,
}

fn default_character_save_version() -> u32 {
    SAVE_VERSION
}

fn default_run_save_version() -> u32 {
    RUN_SAVE_VERSION
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
            let percentile = rng.gen_range(0..100);
            *roll = AbilityScore::new(base, percentile);
        }
        Self { rolls }
    }
}

pub struct CreationState {
    pub name: String,
    pub run_seed: u64,
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
    pub alignment: String,
    pub honor: i32,
    pub background: String,
    pub height: String,
    pub weight: String,
    pub age: String,
    pub handedness: String,
    pub quirks: Vec<String>,
    pub flaws: Vec<String>,
    pub skill_levels: Vec<SkillProgress>,
    pub proficiencies: Vec<String>,
    pub quirk_input: String,
    pub flaw_input: String,
    pub skill_input: String,
    pub skill_feedback: Option<String>,
    pub proficiency_input: String,
    pub starting_money: u32,
    pub money_rolled: bool,
}

impl CreationState {
    pub fn new(weapon_id: WeaponId, run_seed: u64) -> Self {
        let mut rng = SimRng::from_seed(derive_seed(run_seed, "creation", 0));
        let rolled_sets = [RolledSet::roll(&mut rng), RolledSet::roll(&mut rng)];
        let player = PlayerConfig::new("Adventurer", weapon_id);
        Self {
            name: "Adventurer".to_string(),
            run_seed,
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
            alignment: "Unaligned".to_string(),
            honor: 0,
            background: String::new(),
            height: String::new(),
            weight: String::new(),
            age: String::new(),
            handedness: String::new(),
            quirks: Vec::new(),
            flaws: Vec::new(),
            skill_levels: Vec::new(),
            proficiencies: Vec::new(),
            quirk_input: String::new(),
            flaw_input: String::new(),
            skill_input: String::new(),
            skill_feedback: None,
            proficiency_input: String::new(),
            starting_money: 0,
            money_rolled: false,
        }
    }

    pub fn reseed(&mut self, run_seed: u64) {
        self.run_seed = run_seed;
        self.rng = SimRng::from_seed(derive_seed(run_seed, "creation", 0));
        self.reset_rolls();
    }

    pub fn reset_rolls(&mut self) {
        self.rolled_sets = [
            RolledSet::roll(&mut self.rng),
            RolledSet::roll(&mut self.rng),
        ];
        self.selected_set = 0;
        self.assignments = [None; STAT_COUNT];
        self.stats_locked = false;
        self.race_index = None;
        self.race_applied = false;
        self.bp_history = std::array::from_fn(|_| Vec::new());
        self.talent_category = TALENT_TAB_ALL.to_string();
        self.honor = 0;
        self.skill_levels.clear();
        self.skill_feedback = None;
        self.starting_money = 0;
        self.money_rolled = false;
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
        self.player.proficiencies = self.proficiencies.clone();
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
