#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui::{self, Color32, Pos2, Rect};
use hackmaster_sim::{character, data, game_logic, sim};
use hackmaster_sim::character::{AbilityScore, AbilitySet, WeaponGroup};
use hackmaster_sim::core::catalog::Catalog;
use hackmaster_sim::core::gameplay::{
    apply_fight_result, AutobattlerConfig, CombatantBuilder, EnemySpawnEntry, EnemySpawner,
    FightResult, LootTable, RunOutcome, RunState,
};
use hackmaster_sim::core::rng::SimRng;
use hackmaster_sim::core::sim::SimConfig;
use hackmaster_sim::core::types::{
    EnemyProfile, Inventory, PlayerProfile, RaceSpec, TalentSelection, TalentSpec,
};
use hackmaster_sim::game_logic::{
    ArmorCatalog, NpcPresetCatalog, PlayerConfig, ShieldCatalog, TalentCatalog, WeaponCatalog,
    WeaponId,
};
use rand::{Rng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const START_BP: i32 = 65;
const START_LP: i32 = 15;
const START_AP: i32 = 15;
const START_RP: i32 = 6;
const SAVE_VERSION: u32 = 1;
const RUN_SAVE_VERSION: u32 = 1;
const CHARACTER_SAVE_DIR: &str = "saves/autobattler";
const CHARACTER_SAVE_EXTENSION: &str = "json";
const RUN_SAVE_DIR: &str = "saves/autobattler_runs";
const RUN_SAVE_EXTENSION: &str = "json";
const AUTOBATTLER_CONFIG_PATH: &str = "data/autobattler_config.json";
const NPC_PRESETS_PATH: &str = "data/npc_presets.json";
const LOG_DISPLAY_LIMIT: usize = 200;

const STAT_COUNT: usize = 7;
const STAT_LABELS: [&str; STAT_COUNT] = ["STR", "INT", "WIS", "DEX", "CON", "LKS", "CHA"];

const TALENT_TAB_ALL: &str = "All";
const TALENT_TAB_RACIALS: &str = "Racials";
const WEAPON_GROUP_LABELS: [&str; 13] = [
    "Unarmed",
    "Axes",
    "Basic",
    "Blunt",
    "Bows",
    "Crossbows",
    "Double",
    "Ensnaring",
    "Lashes",
    "Large swords",
    "Small swords",
    "Polearms",
    "Spears",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppScreen {
    Start,
    Creation,
    Run,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CreationStep {
    Points,
    RollStats,
    ChooseRace,
    SpendBp,
    Talents,
}

impl CreationStep {
    fn title(self) -> &'static str {
        match self {
            CreationStep::Points => "Step 1: Starting Points",
            CreationStep::RollStats => "Step 2: Roll Ability Scores",
            CreationStep::ChooseRace => "Step 3: Choose Race",
            CreationStep::SpendBp => "Step 4: Spend BP on Stats",
            CreationStep::Talents => "Step 5: Purchase Talents",
        }
    }

    fn index(self) -> usize {
        match self {
            CreationStep::Points => 0,
            CreationStep::RollStats => 1,
            CreationStep::ChooseRace => 2,
            CreationStep::SpendBp => 3,
            CreationStep::Talents => 4,
        }
    }

    fn next(self) -> Option<Self> {
        match self {
            CreationStep::Points => Some(CreationStep::RollStats),
            CreationStep::RollStats => Some(CreationStep::ChooseRace),
            CreationStep::ChooseRace => Some(CreationStep::SpendBp),
            CreationStep::SpendBp => Some(CreationStep::Talents),
            CreationStep::Talents => None,
        }
    }

    fn prev(self) -> Option<Self> {
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
enum RunAction {
    FightOn,
    RestDay,
    Train,
}

impl RunAction {
    fn label(self) -> &'static str {
        match self {
            RunAction::FightOn => "Fight on",
            RunAction::RestDay => "Rest a day",
            RunAction::Train => "Train",
        }
    }

    fn rest_days(self) -> u32 {
        1
    }

    fn is_resting(self) -> bool {
        matches!(self, RunAction::RestDay)
    }
}

#[derive(Clone, Debug)]
struct RunViewState {
    run_state: RunState,
    last_outcome: Option<RunOutcome>,
    last_action: Option<RunAction>,
    last_log: Vec<String>,
    days_elapsed: u32,
    training_days: u32,
    run_over: bool,
    live_fight: Option<LiveFight>,
}

impl RunViewState {
    fn new(run_state: RunState) -> Self {
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
struct LiveFight {
    sim: hackmaster_sim::core::sim::SimState,
    enemy: EnemyProfile,
    action: Option<RunAction>,
    rest_days: u32,
    resting: bool,
    running: bool,
    time_scale: f32,
    max_seconds: u32,
    ui_elapsed: f32,
    seen_events: usize,
    log_lines: Vec<String>,
    float_seed: u32,
    floaters: Vec<DamageFloat>,
    pending_step: bool,
}

#[derive(Clone, Debug)]
struct DamageFloat {
    value: i32,
    target_idx: usize,
    start_time: f32,
    offset: f32,
    is_shield: bool,
}

#[derive(Clone, Debug)]
struct SaveEntry {
    file_name: String,
    display_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AbilityScoreSave {
    base: u8,
    percentile: u8,
}

impl AbilityScoreSave {
    fn from_score(score: AbilityScore) -> Self {
        Self {
            base: score.base,
            percentile: score.percentile,
        }
    }

    fn to_score(&self) -> AbilityScore {
        AbilityScore::new(self.base, self.percentile)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CharacterSave {
    version: u32,
    name: String,
    stats: Vec<AbilityScoreSave>,
    race_id: Option<String>,
    talents: Vec<TalentSelection>,
    bp_history: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AbilitySetSave {
    strength: AbilityScoreSave,
    intelligence: u8,
    wisdom: u8,
    dexterity: AbilityScoreSave,
    constitution: u8,
    looks: u8,
    charisma: u8,
}

impl AbilitySetSave {
    fn from_set(set: AbilitySet) -> Self {
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

    fn to_set(&self) -> AbilitySet {
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
struct PlayerProfileSave {
    name: String,
    level: u8,
    xp: u32,
    base_stats: AbilitySetSave,
    talents: Vec<TalentSelection>,
}

impl PlayerProfileSave {
    fn from_profile(profile: &PlayerProfile) -> Self {
        Self {
            name: profile.name.clone(),
            level: profile.level,
            xp: profile.xp,
            base_stats: AbilitySetSave::from_set(profile.base_stats),
            talents: profile.talents.clone(),
        }
    }

    fn to_profile(&self) -> PlayerProfile {
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
struct InventorySave {
    gold: u32,
    items: Vec<String>,
}

impl InventorySave {
    fn from_inventory(inventory: &Inventory) -> Self {
        Self {
            gold: inventory.gold,
            items: inventory.items.clone(),
        }
    }

    fn to_inventory(&self) -> Inventory {
        Inventory {
            gold: self.gold,
            items: self.items.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct WoundSave {
    damage: u32,
    healing_progress_quarter_days: u32,
}

impl WoundSave {
    fn from_wound(wound: &hackmaster_sim::core::gameplay::Wound) -> Self {
        Self {
            damage: wound.damage,
            healing_progress_quarter_days: wound.healing_progress_quarter_days,
        }
    }

    fn to_wound(&self) -> hackmaster_sim::core::gameplay::Wound {
        hackmaster_sim::core::gameplay::Wound {
            damage: self.damage,
            healing_progress_quarter_days: self.healing_progress_quarter_days,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RunStateSave {
    player: PlayerProfileSave,
    inventory: InventorySave,
    run_depth: u32,
    wounds: Vec<WoundSave>,
}

impl RunStateSave {
    fn from_state(state: &RunState) -> Self {
        Self {
            player: PlayerProfileSave::from_profile(&state.player),
            inventory: InventorySave::from_inventory(&state.inventory),
            run_depth: state.run_depth,
            wounds: state.wounds.iter().map(WoundSave::from_wound).collect(),
        }
    }

    fn to_state(&self) -> RunState {
        RunState {
            player: self.player.to_profile(),
            inventory: self.inventory.to_inventory(),
            run_depth: self.run_depth,
            wounds: self.wounds.iter().map(WoundSave::to_wound).collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RunSave {
    version: u32,
    name: String,
    character: CharacterSave,
    run_state: RunStateSave,
    days_elapsed: u32,
    training_days: u32,
    run_over: bool,
    last_action: Option<RunAction>,
    last_log: Vec<String>,
}

#[derive(Clone, Copy, Debug, Default)]
struct PointPool {
    bp: i32,
    lp: i32,
    ap: i32,
    rp: i32,
}

impl PointPool {
    fn new(bp: i32, lp: i32, ap: i32, rp: i32) -> Self {
        Self { bp, lp, ap, rp }
    }

    fn add(self, other: Self) -> Self {
        Self {
            bp: self.bp + other.bp,
            lp: self.lp + other.lp,
            ap: self.ap + other.ap,
            rp: self.rp + other.rp,
        }
    }

    fn sub(self, other: Self) -> Self {
        Self {
            bp: self.bp - other.bp,
            lp: self.lp - other.lp,
            ap: self.ap - other.ap,
            rp: self.rp - other.rp,
        }
    }

    fn can_afford(self, cost: Self) -> bool {
        self.bp >= cost.bp && self.lp >= cost.lp && self.ap >= cost.ap && self.rp >= cost.rp
    }
}

#[derive(Clone, Copy, Debug)]
struct RolledSet {
    rolls: [AbilityScore; STAT_COUNT],
}

impl RolledSet {
    fn roll(rng: &mut SimRng) -> Self {
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

struct CreationState {
    name: String,
    rng: SimRng,
    rolled_sets: [RolledSet; 2],
    selected_set: usize,
    assignments: [Option<usize>; STAT_COUNT],
    stats: [AbilityScore; STAT_COUNT],
    stats_locked: bool,
    race_index: Option<usize>,
    race_applied: bool,
    bp_history: [Vec<u8>; STAT_COUNT],
    talent_category: String,
    player: PlayerConfig,
}

impl CreationState {
    fn new(weapon_id: WeaponId) -> Self {
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

    fn reset_rolls(&mut self) {
        self.rolled_sets = [RolledSet::roll(&mut self.rng), RolledSet::roll(&mut self.rng)];
        self.selected_set = 0;
        self.assignments = [None; STAT_COUNT];
        self.stats_locked = false;
        self.race_index = None;
        self.race_applied = false;
        self.stats = [AbilityScore::new(10, 1); STAT_COUNT];
        self.bp_history = std::array::from_fn(|_| Vec::new());
        self.player.talents.clear();
        self.player.race_id = None;
        self.player.race_applied = false;
        self.player.base_hp = 10;
    }

    fn apply_save(&mut self, save: &CharacterSave, race_catalog: &[RaceSpec]) {
        self.name = save.name.clone();
        self.assignments = [None; STAT_COUNT];
        self.stats_locked = true;
        self.race_index = save.race_id.as_ref().and_then(|id| {
            race_catalog.iter().position(|race| race.id == *id)
        });
        self.race_applied = save.race_id.is_some();
        self.bp_history = std::array::from_fn(|idx| {
            save.bp_history
                .get(idx)
                .cloned()
                .unwrap_or_default()
        });
        let mut stats = [AbilityScore::new(10, 1); STAT_COUNT];
        for (idx, slot) in stats.iter_mut().enumerate() {
            if let Some(saved) = save.stats.get(idx) {
                *slot = saved.to_score();
            }
        }
        self.stats = stats;
        self.player.talents = save.talents.clone();
        self.player.race_id = save.race_id.clone();
        self.player.race_applied = self.race_applied;
        if let Some(race_idx) = self.race_index {
            if let Some(race) = race_catalog.get(race_idx) {
                self.player.base_hp = race.base_hp.max(1);
                self.player.knockback_step = game_logic::knockback_step_for_race(race);
            }
        }
        self.sync_player_from_stats();
    }

    fn assign_roll(&mut self, stat_idx: usize, roll_idx: usize) {
        for (idx, slot) in self.assignments.iter_mut().enumerate() {
            if idx != stat_idx && *slot == Some(roll_idx) {
                *slot = None;
            }
        }
        self.assignments[stat_idx] = Some(roll_idx);
    }

    fn lock_assignments(&mut self) {
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

    fn sync_player_from_stats(&mut self) {
        self.player.name = self.name.clone();
        self.player.strength_base = self.stats[0].base;
        self.player.strength_pct = self.stats[0].percentile;
        self.player.intelligence = self.stats[1].base;
        self.player.wisdom = self.stats[2].base;
        self.player.dex_base = self.stats[3].base;
        self.player.dex_pct = self.stats[3].percentile;
        self.player.constitution = self.stats[4].base;
        self.player.looks = self.stats[5].base;
        let charisma_delta = character::looks_charisma_adjustment(self.stats[5].base);
        self.player.charisma = clamp_stat_adjustment(self.stats[6].base, charisma_delta);
    }
}

struct AutobattlerApp {
    screen: AppScreen,
    creation: CreationState,
    creation_step: CreationStep,
    creation_done: bool,
    save_entries: Vec<SaveEntry>,
    selected_save: Option<usize>,
    run_save_entries: Vec<SaveEntry>,
    selected_run_save: Option<usize>,
    save_name: String,
    save_status: Option<String>,
    run_save_name: String,
    run_save_status: Option<String>,
    needs_save_refresh: bool,
    run_state: Option<RunViewState>,
    autobattler_config: AutobattlerConfig,
    run_rng: SimRng,
    weapon_catalog: WeaponCatalog,
    armor_catalog: ArmorCatalog,
    shield_catalog: ShieldCatalog,
    npc_presets: NpcPresetCatalog,
    enemy_spawner: EnemySpawner,
    loot_table: LootTable,
    sim_config: SimConfig,
    enemy_weapon_id: WeaponId,
    race_catalog: Vec<RaceSpec>,
    talent_catalog: TalentCatalog,
}

impl AutobattlerApp {
    fn new() -> Self {
        let autobattler_config = data::load_autobattler_config(AUTOBATTLER_CONFIG_PATH)
            .unwrap_or_else(|err| {
                eprintln!("Failed to load autobattler config: {err}");
                AutobattlerConfig::default()
            });
        let (weapon_catalog, armor_catalog, shield_catalog) = data::load_catalogs()
            .unwrap_or_else(|err| panic!("Failed to load JSON catalogs: {err}"));
        let npc_presets = data::load_npc_presets(NPC_PRESETS_PATH).unwrap_or_else(|err| {
            eprintln!("Failed to load NPC presets: {err}");
            Catalog::new(Vec::new())
        });
        let race_catalog = data::load_races("data/races.json").unwrap_or_else(|err| {
            eprintln!("Failed to load races: {err}");
            Vec::new()
        });
        let talent_catalog = match data::load_talents("data/talents.json") {
            Ok(talents) => talents,
            Err(err) => {
                eprintln!("Failed to load talents: {err}");
                Catalog::new(Vec::new())
            }
        };
        let weapon_id = weapon_catalog
            .first_id()
            .unwrap_or_else(|| WeaponId::new(0));
        let creation = CreationState::new(weapon_id);
        let enemy_weapon_id = find_weapon_id_by_name(&weapon_catalog, &autobattler_config.enemy_weapon)
            .or_else(|| weapon_catalog.first_id())
            .unwrap_or_else(|| WeaponId::new(0));
        let enemy_spawner = hobgoblin_spawner(&npc_presets);
        let loot_table = autobattler_config.to_loot_table();
        let sim_config =
            SimConfig::new(autobattler_config.start_distance, autobattler_config.stop_distance);
        let app = Self {
            screen: AppScreen::Start,
            creation,
            creation_step: CreationStep::Points,
            creation_done: false,
            save_entries: Vec::new(),
            selected_save: None,
            run_save_entries: Vec::new(),
            selected_run_save: None,
            save_name: String::new(),
            save_status: None,
            run_save_name: String::new(),
            run_save_status: None,
            needs_save_refresh: true,
            run_state: None,
            autobattler_config,
            run_rng: SimRng::default(),
            weapon_catalog,
            armor_catalog,
            shield_catalog,
            npc_presets,
            enemy_spawner,
            loot_table,
            sim_config,
            enemy_weapon_id,
            race_catalog,
            talent_catalog,
        };
        app
    }

    fn available_points(&self) -> PointPool {
        let spent_stats = self
            .creation
            .bp_history
            .iter()
            .map(|history| history.len() as i32)
            .sum::<i32>();
        let spent_talents = total_talent_costs(&self.creation.player.talents, &self.talent_catalog);
        PointPool::new(START_BP, START_LP, START_AP, START_RP)
            .sub(PointPool::new(spent_stats, 0, 0, 0))
            .sub(spent_talents)
    }

    fn effective_charisma(&self) -> (u8, i32) {
        let looks = self.creation.stats[5].base;
        let delta = character::looks_charisma_adjustment(looks);
        let base = clamp_stat_adjustment(self.creation.stats[6].base, delta);
        (base, delta)
    }

    fn apply_race_adjustments(&mut self, race: &RaceSpec) {
        let adjustments = &race.ability_adjustments;
        apply_stat_adjustment(&mut self.creation.stats[0], adjustments.strength);
        apply_stat_adjustment(&mut self.creation.stats[1], adjustments.intelligence);
        apply_stat_adjustment(&mut self.creation.stats[2], adjustments.wisdom);
        apply_stat_adjustment(&mut self.creation.stats[3], adjustments.dexterity);
        apply_stat_adjustment(&mut self.creation.stats[4], adjustments.constitution);
        apply_stat_adjustment(&mut self.creation.stats[5], adjustments.looks);
        apply_stat_adjustment(&mut self.creation.stats[6], adjustments.charisma);
        self.creation.player.base_hp = race.base_hp.max(1);
        self.creation.player.race_id = Some(race.id.clone());
        self.creation.player.knockback_step = game_logic::knockback_step_for_race(race);
        self.creation.player.race_applied = true;
        self.creation.race_applied = true;
        self.creation.sync_player_from_stats();
    }

    fn reset_creation(&mut self) {
        let weapon_id = self.creation.player.weapon_id;
        self.creation = CreationState::new(weapon_id);
        self.creation_step = CreationStep::Points;
        self.creation_done = false;
        self.save_name.clear();
        self.save_status = None;
        self.run_save_name.clear();
        self.run_save_status = None;
        self.run_state = None;
    }

    fn can_advance(&self) -> bool {
        match self.creation_step {
            CreationStep::Points => true,
            CreationStep::RollStats => self.creation.stats_locked || self.all_rolls_assigned(),
            CreationStep::ChooseRace => self.creation.race_applied,
            CreationStep::SpendBp => self.creation.race_applied,
            CreationStep::Talents => false,
        }
    }

    fn can_finish(&self) -> bool {
        self.creation_step == CreationStep::Talents
    }

    fn all_rolls_assigned(&self) -> bool {
        self.creation.assignments.iter().all(|slot| slot.is_some())
    }

    fn refresh_saves(&mut self) {
        self.save_entries = scan_save_entries();
        if self.selected_save.is_some()
            && self
                .selected_save
                .map(|idx| idx >= self.save_entries.len())
                .unwrap_or(false)
        {
            self.selected_save = None;
        }
        self.run_save_entries = scan_run_save_entries();
        if self.selected_run_save.is_some()
            && self
                .selected_run_save
                .map(|idx| idx >= self.run_save_entries.len())
                .unwrap_or(false)
        {
            self.selected_run_save = None;
        }
    }

    fn save_character(&mut self) -> bool {
        let name = if self.save_name.trim().is_empty() {
            self.creation.name.trim()
        } else {
            self.save_name.trim()
        };
        if name.is_empty() {
            self.save_status = Some("Enter a save name.".to_string());
            return false;
        }
        let file_name = format!(
            "{}.{}",
            sanitize_filename(name),
            CHARACTER_SAVE_EXTENSION
        );
        let path = save_path_for(&file_name);
        let save = CharacterSave {
            version: SAVE_VERSION,
            name: self.creation.name.clone(),
            stats: self
                .creation
                .stats
                .iter()
                .map(|score| AbilityScoreSave::from_score(*score))
                .collect(),
            race_id: self.creation.player.race_id.clone(),
            talents: self.creation.player.talents.clone(),
            bp_history: self.creation.bp_history.iter().cloned().collect(),
        };
        match write_character_save(&path, &save) {
            Ok(()) => {
                self.save_status = Some(format!("Saved to {}", path.display()));
                self.needs_save_refresh = true;
                true
            }
            Err(err) => {
                self.save_status = Some(format!("Save failed: {err}"));
                false
            }
        }
    }

    fn save_run(&mut self) -> bool {
        let Some(run_view) = self.run_state.as_ref() else {
            self.run_save_status = Some("No active run to save.".to_string());
            return false;
        };
        if run_view.live_fight.is_some() {
            self.run_save_status = Some("Finish the live fight before saving.".to_string());
            return false;
        }
        let suggested = format!(
            "{}-depth{}",
            self.creation.name.trim(),
            run_view.run_state.run_depth
        );
        if self.run_save_name.trim().is_empty() {
            self.run_save_name = suggested;
        }
        let name = self.run_save_name.trim();
        if name.is_empty() {
            self.run_save_status = Some("Enter a run save name.".to_string());
            return false;
        }

        let file_name = format!("{}.{}", sanitize_filename(name), RUN_SAVE_EXTENSION);
        let path = run_save_path_for(&file_name);
        let character = CharacterSave {
            version: SAVE_VERSION,
            name: self.creation.name.clone(),
            stats: self
                .creation
                .stats
                .iter()
                .map(|score| AbilityScoreSave::from_score(*score))
                .collect(),
            race_id: self.creation.player.race_id.clone(),
            talents: self.creation.player.talents.clone(),
            bp_history: self.creation.bp_history.iter().cloned().collect(),
        };
        let run_save = RunSave {
            version: RUN_SAVE_VERSION,
            name: name.to_string(),
            character,
            run_state: RunStateSave::from_state(&run_view.run_state),
            days_elapsed: run_view.days_elapsed,
            training_days: run_view.training_days,
            run_over: run_view.run_over,
            last_action: run_view.last_action,
            last_log: run_view.last_log.clone(),
        };
        match write_run_save(&path, &run_save) {
            Ok(()) => {
                self.run_save_status = Some(format!("Run saved to {}", path.display()));
                self.needs_save_refresh = true;
                true
            }
            Err(err) => {
                self.run_save_status = Some(format!("Run save failed: {err}"));
                false
            }
        }
    }

    fn load_selected_character(&mut self) {
        let Some(index) = self.selected_save else {
            return;
        };
        let Some(entry) = self.save_entries.get(index) else {
            self.save_status = Some("Selected save no longer exists.".to_string());
            return;
        };
        let path = save_path_for(&entry.file_name);
        match read_character_save(&path) {
            Ok(save) => {
                self.creation.apply_save(&save, &self.race_catalog);
                self.save_name = entry.display_name.clone();
                self.save_status = None;
                self.start_run_from_creation();
            }
            Err(err) => {
                self.save_status = Some(format!("Load failed: {err}"));
            }
        }
    }

    fn load_selected_run(&mut self) {
        let Some(index) = self.selected_run_save else {
            return;
        };
        let Some(entry) = self.run_save_entries.get(index) else {
            self.run_save_status = Some("Selected run save no longer exists.".to_string());
            return;
        };
        let path = run_save_path_for(&entry.file_name);
        match read_run_save(&path) {
            Ok(save) => {
                self.creation.apply_save(&save.character, &self.race_catalog);
                self.creation_step = CreationStep::Talents;
                self.creation_done = true;
                let mut run_view = RunViewState::new(save.run_state.to_state());
                run_view.days_elapsed = save.days_elapsed;
                run_view.training_days = save.training_days;
                run_view.run_over = save.run_over;
                run_view.last_action = save.last_action;
                run_view.last_log = save.last_log;
                self.run_state = Some(run_view);
                self.run_rng = SimRng::from_seed(self.autobattler_config.seed);
                self.screen = AppScreen::Run;
                self.run_save_name = save.name;
                self.run_save_status = None;
            }
            Err(err) => {
                self.run_save_status = Some(format!("Run load failed: {err}"));
            }
        }
    }

    fn start_run_from_creation(&mut self) {
        let player_profile = player_profile_from_config(&self.creation.player);
        let run_state = RunState::new(player_profile, Inventory::default());
        self.run_state = Some(RunViewState::new(run_state));
        self.run_rng = SimRng::from_seed(self.autobattler_config.seed);
        self.run_save_name.clear();
        self.run_save_status = None;
        self.screen = AppScreen::Run;
        self.start_live_fight(0, false, None);
    }

    fn start_live_fight(
        &mut self,
        rest_days: u32,
        resting: bool,
        action: Option<RunAction>,
    ) {
        let Some(run_view) = self.run_state.as_mut() else {
            return;
        };
        if run_view.run_over || run_view.live_fight.is_some() {
            return;
        }
        if rest_days > 0 {
            run_view.days_elapsed = run_view.days_elapsed.saturating_add(rest_days);
            if matches!(action, Some(RunAction::Train)) {
                run_view.training_days = run_view.training_days.saturating_add(rest_days);
            }
        }
        run_view.last_log.clear();

        let effective_level = scaled_enemy_level(
            run_view.run_state.player.level,
            run_view.run_state.run_depth,
        );
        let enemy_profile = match self
            .enemy_spawner
            .spawn_for_level(effective_level, &mut self.run_rng)
        {
            Some(enemy) => enemy,
            None => {
                run_view.run_over = true;
                return;
            }
        };

        let builder = AutobattlerBuilder {
            player_base: self.creation.player.clone(),
            enemy_weapon_id: self.enemy_weapon_id,
            weapon_catalog: &self.weapon_catalog,
            armor_catalog: &self.armor_catalog,
            shield_catalog: &self.shield_catalog,
            npc_presets: &self.npc_presets,
            talent_catalog: &self.talent_catalog,
        };

        let player_combatant = builder.build_player(&run_view.run_state);
        let enemy_combatant = builder.build_enemy(&enemy_profile);
        let fight_seed = self.run_rng.next_u64();
        let mut sim = hackmaster_sim::core::sim::SimState::with_rng(
            self.sim_config,
            SimRng::from_seed(fight_seed),
        );
        sim.reset_with_combatants([player_combatant, enemy_combatant]);

        run_view.live_fight = Some(LiveFight {
            sim,
            enemy: enemy_profile,
            action,
            rest_days,
            resting,
            running: true,
            time_scale: 1.0,
            max_seconds: self.autobattler_config.max_fight_seconds,
            ui_elapsed: 0.0,
            seen_events: 0,
            log_lines: Vec::new(),
            float_seed: 0,
            floaters: Vec::new(),
            pending_step: false,
        });
    }

    fn run_action(&mut self, action: RunAction) {
        let rest_days = if action == RunAction::FightOn {
            0
        } else {
            action.rest_days()
        };
        let resting = action.is_resting();
        self.start_live_fight(rest_days, resting, Some(action));
    }

    fn complete_live_fight(&mut self) {
        let Some(run_view) = self.run_state.as_mut() else {
            return;
        };
        let Some(live) = run_view.live_fight.take() else {
            return;
        };

        run_view.last_log = live.log_lines.clone();
        let player_hp = live.sim.combatants[0].state.hp;
        let enemy_hp = live.sim.combatants[1].state.hp;
        let won = live.sim.done && player_hp > 0 && enemy_hp <= 0;
        let fight = FightResult {
            won,
            remaining_hp: player_hp,
            turns: live.sim.elapsed_seconds,
            events: live.sim.combat_events.clone(),
        };

        let outcome = apply_fight_result(
            run_view.run_state.clone(),
            Some(live.enemy),
            fight,
            &self.loot_table,
            None,
            live.rest_days,
            live.resting,
            &mut self.run_rng,
        );

        run_view.run_state = outcome.state.clone();
        run_view.last_outcome = Some(outcome);
        run_view.last_action = live.action;
        run_view.run_over = !run_view
            .last_outcome
            .as_ref()
            .map(|outcome| outcome.fight.won)
            .unwrap_or(false);
    }

    fn start_new_character(&mut self) {
        self.reset_creation();
        self.creation_step = CreationStep::Points;
        self.creation_done = false;
        self.screen = AppScreen::Creation;
    }
}

impl eframe::App for AutobattlerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        match self.screen {
            AppScreen::Start => {
                if self.needs_save_refresh {
                    self.refresh_saves();
                    self.needs_save_refresh = false;
                }
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.heading("Autobattler");
                    ui.separator();
                    ui.label("Load an existing character or start a new one.");
                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui.button("New character").clicked() {
                            self.start_new_character();
                        }
                    });

                    ui.separator();
                    ui.label("Saved characters");
                    if self.save_entries.is_empty() {
                        ui.label("No saves found.");
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(240.0)
                            .show(ui, |ui| {
                                for (idx, entry) in self.save_entries.iter().enumerate() {
                                    let selected = self.selected_save == Some(idx);
                                    let label = format!("{} ({})", entry.display_name, entry.file_name);
                                    if ui.selectable_label(selected, label).clicked() {
                                        self.selected_save = Some(idx);
                                    }
                                }
                            });
                    }
                    ui.horizontal(|ui| {
                        let can_load = self.selected_save.is_some();
                        if ui.add_enabled(can_load, egui::Button::new("Load selected")).clicked() {
                            self.load_selected_character();
                        }
                    });
                    if let Some(status) = self.save_status.as_ref() {
                        ui.separator();
                        ui.label(status);
                    }

                    ui.separator();
                    ui.label("Saved runs");
                    if self.run_save_entries.is_empty() {
                        ui.label("No run saves found.");
                    } else {
                        egui::ScrollArea::vertical()
                            .max_height(240.0)
                            .show(ui, |ui| {
                                for (idx, entry) in self.run_save_entries.iter().enumerate() {
                                    let selected = self.selected_run_save == Some(idx);
                                    let label =
                                        format!("{} ({})", entry.display_name, entry.file_name);
                                    if ui.selectable_label(selected, label).clicked() {
                                        self.selected_run_save = Some(idx);
                                    }
                                }
                            });
                    }
                    ui.horizontal(|ui| {
                        let can_load = self.selected_run_save.is_some();
                        if ui.add_enabled(can_load, egui::Button::new("Load run")).clicked() {
                            self.load_selected_run();
                        }
                    });
                    if let Some(status) = self.run_save_status.as_ref() {
                        ui.separator();
                        ui.label(status);
                    }
                });
            }
            AppScreen::Creation => {
                let available_points = self.available_points();
                let (effective_cha, looks_delta) = self.effective_charisma();
                render_character_summary(
                    ctx,
                    &self.creation,
                    &self.race_catalog,
                    &self.talent_catalog,
                    available_points,
                    effective_cha,
                    looks_delta,
                    None,
                );

                egui::TopBottomPanel::top("creation_header").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Autobattler Character Creation");
                        ui.separator();
                        let step_number = self.creation_step.index() + 1;
                        ui.label(format!("Step {step_number} of 5"));
                        ui.separator();
                        ui.label(self.creation_step.title());
                        if self.creation_done {
                            ui.separator();
                            ui.label("Creation complete");
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Reset to start").clicked() {
                                self.reset_creation();
                                self.screen = AppScreen::Start;
                                self.needs_save_refresh = true;
                            }
                        });
                    });
                });

                egui::CentralPanel::default().show(ctx, |ui| {
                    let step = self.creation_step;
                    let available_height = ui.available_height();
                    match step {
                        CreationStep::Points => {
                            ui.heading("Starting Points and Name");
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label("Name");
                                let response = ui.text_edit_singleline(&mut self.creation.name);
                                if response.changed() {
                                    self.creation.sync_player_from_stats();
                                }
                            });
                            ui.separator();
                            ui.label(format!(
                                "Start: {START_BP} BP, {START_LP} LP, {START_AP} AP, {START_RP} RP"
                            ));
                            ui.label(format!(
                                "Remaining: {} BP, {} LP, {} AP, {} RP",
                                available_points.bp,
                                available_points.lp,
                                available_points.ap,
                                available_points.rp
                            ));
                        }
                        CreationStep::RollStats => {
                            ui.heading("Roll Ability Scores");
                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button("Reroll").clicked() {
                                    self.creation.reset_rolls();
                                }
                            });
                            ui.label("Pick a set, then assign each roll to a stat.");
                            ui.horizontal(|ui| {
                                for set_idx in 0..2 {
                                    ui.group(|ui| {
                                        let label = format!("Set {}", set_idx + 1);
                                        ui.radio_value(&mut self.creation.selected_set, set_idx, label);
                                        let set = &self.creation.rolled_sets[set_idx];
                                        for (idx, roll) in set.rolls.iter().enumerate() {
                                            ui.label(format!("{}: {}", idx + 1, format_score(*roll)));
                                        }
                                    });
                                }
                            });

                            ui.separator();
                            ui.label("Assignments");
                            let selected_set = self.creation.rolled_sets[self.creation.selected_set];
                            ui.add_enabled_ui(!self.creation.stats_locked, |ui| {
                                for stat_idx in 0..STAT_COUNT {
                                    ui.horizontal(|ui| {
                                        ui.label(STAT_LABELS[stat_idx]);
                                        let selection = self.creation.assignments[stat_idx];
                                        let selected_text = selection
                                            .map(|idx| format_score(selected_set.rolls[idx]))
                                            .unwrap_or_else(|| "Select roll".to_string());
                                        egui::ComboBox::from_id_source(format!("assign_{stat_idx}"))
                                            .selected_text(selected_text)
                                            .show_ui(ui, |ui| {
                                                for roll_idx in 0..STAT_COUNT {
                                                    let taken_elsewhere = self
                                                        .creation
                                                        .assignments
                                                        .iter()
                                                        .enumerate()
                                                        .any(|(idx, slot)| {
                                                            idx != stat_idx
                                                                && *slot == Some(roll_idx)
                                                        });
                                                    let roll = selected_set.rolls[roll_idx];
                                                    let label = format_score(roll);
                                                    ui.add_enabled_ui(!taken_elsewhere, |ui| {
                                                        if ui
                                                            .selectable_label(
                                                                selection == Some(roll_idx),
                                                                label,
                                                            )
                                                            .clicked()
                                                        {
                                                            self.creation.assign_roll(
                                                                stat_idx, roll_idx,
                                                            );
                                                        }
                                                    });
                                                }
                                            });
                                    });
                                }
                            });
                            if self.creation.stats_locked {
                                ui.label("Rolls locked. Use Reroll to generate new sets.");
                            }
                        }
                        CreationStep::ChooseRace => {
                            ui.heading("Choose Race");
                            ui.separator();
                            ui.add_enabled_ui(self.creation.stats_locked, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Race");
                                    let mut selection = self.creation.race_index.unwrap_or(usize::MAX);
                                    ui.add_enabled_ui(!self.creation.race_applied, |ui| {
                                        egui::ComboBox::from_id_source("race_select")
                                            .selected_text(
                                                self.race_catalog
                                                    .get(selection)
                                                    .map(|race| race.name.as_str())
                                                    .unwrap_or("None"),
                                            )
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(&mut selection, usize::MAX, "None");
                                                for (idx, race) in self.race_catalog.iter().enumerate() {
                                                    ui.selectable_value(
                                                        &mut selection,
                                                        idx,
                                                        race.name.as_str(),
                                                    );
                                                }
                                            });
                                    });
                                    if !self.creation.race_applied {
                                        if selection == usize::MAX {
                                            self.creation.race_index = None;
                                        } else {
                                            self.creation.race_index = Some(selection);
                                        }
                                    }
                                    let can_confirm = self.creation.race_index.is_some()
                                        && !self.creation.race_applied;
                                    if ui
                                        .add_enabled(
                                            can_confirm,
                                            egui::Button::new("Confirm race"),
                                        )
                                        .clicked()
                                    {
                                        if let Some(index) = self.creation.race_index {
                                            if let Some(race) = self.race_catalog.get(index).cloned() {
                                                self.apply_race_adjustments(&race);
                                            }
                                        }
                                    }
                                });
                                if let Some(index) = self.creation.race_index {
                                    if let Some(race) = self.race_catalog.get(index) {
                                        ui.label(format!(
                                            "Base HP {} | {}",
                                            race.base_hp,
                                            race_adjustment_summary(race)
                                        ));
                                        if !race.pros.is_empty() {
                                            ui.separator();
                                            ui.label("Pros:");
                                            for entry in &race.pros {
                                                ui.label(format!("- {entry}"));
                                            }
                                        }
                                        if !race.cons.is_empty() {
                                            ui.separator();
                                            ui.label("Cons:");
                                            for entry in &race.cons {
                                                ui.label(format!("- {entry}"));
                                            }
                                        }
                                    }
                                }
                                if self.creation.race_applied {
                                    ui.separator();
                                    ui.label("Race confirmed. To change race, reroll stats.");
                                }
                            });
                        }
                        CreationStep::SpendBp => {
                            ui.heading("Spend BP on Stats");
                            ui.separator();
                            ui.label(format!("Remaining: {} BP", available_points.bp));
                            ui.label(format!(
                                "Current CHA after Looks: {} (Looks {:+})",
                                effective_cha, looks_delta
                            ));
                            ui.separator();
                            ui.add_enabled_ui(self.creation.race_applied, |ui| {
                                for stat_idx in 0..STAT_COUNT {
                                    let label = STAT_LABELS[stat_idx];
                                    let score = &mut self.creation.stats[stat_idx];
                                    let history = &mut self.creation.bp_history[stat_idx];
                                    let increment = bp_increment(score);
                                    let can_add = available_points.bp > 0 && !stat_at_cap(score);
                                    let can_remove = !history.is_empty();
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{label}: {}", format_score(*score)));
                                        if ui
                                            .add_enabled(can_remove, egui::Button::new("-1 BP"))
                                            .clicked()
                                        {
                                            if let Some(delta) = history.pop() {
                                                subtract_percentile(score, delta);
                                            }
                                        }
                                        if ui
                                            .add_enabled(can_add, egui::Button::new("+1 BP"))
                                            .clicked()
                                        {
                                            apply_percentile(score, increment);
                                            history.push(increment);
                                        }
                                        ui.label(format!(
                                            "Next: +0/{}",
                                            format_percentile(increment)
                                        ));
                                    });
                                }
                                self.creation.sync_player_from_stats();
                            });
                        }
                        CreationStep::Talents => {
                            ui.heading("Purchase Talents");
                            ui.separator();
                            ui.label(format!(
                                "Remaining: {} BP, {} LP, {} AP, {} RP",
                                available_points.bp,
                                available_points.lp,
                                available_points.ap,
                                available_points.rp
                            ));
                            ui.separator();
                            ui.label("Spend BP/LP/RP on talents. Requirements are enforced.");
                            let talent_height = (available_height - 120.0).max(320.0);
                            ui.add_enabled_ui(self.creation.race_applied, |ui| {
                                render_talent_selector(
                                    ui,
                                    "creation",
                                    &mut self.creation.player,
                                    &self.weapon_catalog,
                                    &self.race_catalog,
                                    &self.talent_catalog,
                                    &mut self.creation.talent_category,
                                    available_points,
                                    talent_height,
                                );
                            });
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label("Save name (auto-saved on Finish)");
                                if self.save_name.is_empty() {
                                    self.save_name = self.creation.name.clone();
                                }
                                ui.text_edit_singleline(&mut self.save_name);
                            });
                            if let Some(status) = self.save_status.as_ref() {
                                ui.label(status);
                            }
                            if self.creation_done {
                                ui.separator();
                                ui.label("Character creation complete.");
                            }
                        }
                    }
                });

                egui::TopBottomPanel::bottom("creation_nav").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if let Some(prev) = self.creation_step.prev() {
                            if ui.button("Back").clicked() {
                                self.creation_step = prev;
                                self.creation_done = false;
                            }
                        } else {
                            ui.add_enabled(false, egui::Button::new("Back"));
                        }
                        ui.separator();
                        if let Some(next) = self.creation_step.next() {
                            if ui.add_enabled(self.can_advance(), egui::Button::new("Next")).clicked() {
                                if self.creation_step == CreationStep::RollStats
                                    && !self.creation.stats_locked
                                {
                                    self.creation.lock_assignments();
                                }
                                self.creation_step = next;
                                self.creation_done = false;
                            }
                        } else if ui
                            .add_enabled(self.can_finish(), egui::Button::new("Finish"))
                            .clicked()
                        {
                            if self.save_character() {
                                self.creation_done = true;
                                self.start_run_from_creation();
                            }
                        }
                    });
                });
            }
            AppScreen::Run => {
                let dt = ctx.input(|i| i.unstable_dt).min(0.05);
                let mut finalize_live = false;
                let mut repaint = false;
                if let Some(run_view) = self.run_state.as_mut() {
                    if let Some(live) = run_view.live_fight.as_mut() {
                        let mut sim_advanced = false;
                        if live.pending_step {
                            live.pending_step = false;
                            live.sim.tick();
                            live.ui_elapsed += 1.0;
                            sim_advanced = true;
                        } else if live.running {
                            let step = dt * live.time_scale;
                            live.sim.update(step);
                            live.ui_elapsed += step;
                            sim_advanced = true;
                        }
                        if sim_advanced {
                            ingest_live_events(live);
                            prune_floaters(live);
                            repaint = true;
                        } else if live.running {
                            repaint = true;
                        }
                        if live.sim.done || live.sim.elapsed_seconds >= live.max_seconds {
                            finalize_live = true;
                        }
                    }
                }
                if repaint {
                    ctx.request_repaint();
                }
                if finalize_live {
                    self.complete_live_fight();
                }

                let available_points = self.available_points();
                let (effective_cha, looks_delta) = self.effective_charisma();
                render_character_summary(
                    ctx,
                    &self.creation,
                    &self.race_catalog,
                    &self.talent_catalog,
                    available_points,
                    effective_cha,
                    looks_delta,
                    self.run_state.as_ref(),
                );

                egui::TopBottomPanel::top("run_header").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Autobattler Run");
                        ui.separator();
                        if let Some(run_view) = self.run_state.as_ref() {
                            ui.label(format!("Depth {}", run_view.run_state.run_depth));
                            ui.separator();
                            ui.label(format!("Days {}", run_view.days_elapsed));
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Back to start").clicked() {
                                self.screen = AppScreen::Start;
                                self.needs_save_refresh = true;
                            }
                        });
                    });
                });

                let mut next_action: Option<RunAction> = None;
                egui::CentralPanel::default().show(ctx, |ui| {
                    let Some(run_view) = self.run_state.as_mut() else {
                        ui.label("No run loaded.");
                        return;
                    };

                    if let Some(live) = run_view.live_fight.as_mut() {
                        ui.heading("Live Fight");
                        ui.separator();
                        ui.horizontal(|ui| {
                            let label = if live.running { "Pause" } else { "Resume" };
                            if ui.button(label).clicked() {
                                live.running = !live.running;
                            }
                            if !live.running && ui.button("Next second").clicked() {
                                live.pending_step = true;
                                ctx.request_repaint();
                            }
                            ui.label("Speed");
                            ui.add(egui::Slider::new(&mut live.time_scale, 0.25..=4.0).step_by(0.25));
                            ui.label(format!("Time: {}s", live.sim.elapsed_seconds));
                        });
                        let enemy_name = self
                            .npc_presets
                            .get(live.enemy.preset_id)
                            .map(|preset| preset.name.as_str())
                            .unwrap_or("Unknown");
                        ui.label(format!("Enemy: {enemy_name}"));
                        let arena_height = ui.available_height().min(360.0).max(220.0);
                        let (rect, _response) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), arena_height),
                            egui::Sense::hover(),
                        );
                        draw_live_arena(
                            ui,
                            rect,
                            &live.sim,
                            &self.creation.name,
                            enemy_name,
                            &live.floaters,
                            live.ui_elapsed,
                        );
                        ui.separator();
                        ui.label("Fight in progress...");
                        ui.separator();
                        ui.label("Combat log");
                        egui::ScrollArea::vertical()
                            .max_height(ui.available_height().min(180.0).max(120.0))
                            .show(ui, |ui| {
                                let start = live
                                    .log_lines
                                    .len()
                                    .saturating_sub(LOG_DISPLAY_LIMIT);
                                for line in &live.log_lines[start..] {
                                    ui.label(line);
                                }
                            });
                    } else {
                        ui.heading("Last Fight");
                        ui.separator();
                        if let Some(outcome) = run_view.last_outcome.as_ref() {
                            let enemy_name = outcome
                                .enemy
                                .as_ref()
                                .and_then(|enemy| self.npc_presets.get(enemy.preset_id))
                                .map(|preset| preset.name.as_str())
                                .unwrap_or("Unknown");
                            let result = if outcome.fight.won { "WIN" } else { "LOSS" };
                            ui.label(format!("Result: {result} vs {enemy_name}"));
                            ui.label(format!("Remaining HP: {}", outcome.fight.remaining_hp));
                            ui.label(format!("Turns: {}", outcome.fight.turns));
                            if let Some(reward) = outcome.reward.as_ref() {
                                if reward.is_empty() {
                                    ui.label("Reward: none");
                                } else {
                                    ui.label(format!(
                                        "Reward: +{}g +{}xp",
                                        reward.gold, reward.xp
                                    ));
                                    if !reward.items.is_empty() {
                                        ui.label(format!(
                                            "Items: {}",
                                            reward.items.join(", ")
                                        ));
                                    }
                                }
                            } else {
                                ui.label("Reward: none");
                            }
                            let wound_list = if run_view.run_state.wounds.is_empty() {
                                "none".to_string()
                            } else {
                                run_view
                                    .run_state
                                    .wounds
                                    .iter()
                                    .map(|wound| wound.damage.to_string())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            };
                            ui.label(format!("Wounds: {wound_list}"));
                            if let Some(action) = run_view.last_action {
                                ui.label(format!("Last choice: {}", action.label()));
                            }
                        } else {
                            ui.label("No fights resolved yet.");
                        }

                        ui.separator();
                        if run_view.run_over {
                            ui.colored_label(Color32::from_rgb(180, 70, 70), "Defeated.");
                            ui.label("Return to start to begin again.");
                        } else {
                            ui.label("Choose next action:");
                            ui.horizontal(|ui| {
                                if ui.button("Fight on").clicked() {
                                    next_action = Some(RunAction::FightOn);
                                }
                                if ui.button("Rest a day").clicked() {
                                    next_action = Some(RunAction::RestDay);
                                }
                                if ui.button("Train").clicked() {
                                    next_action = Some(RunAction::Train);
                                }
                            });
                            ui.label("Wound healing is halved unless you rest.");
                        }
                        ui.separator();
                        ui.label("Combat log");
                        if run_view.last_log.is_empty() {
                            ui.label("No combat log available.");
                        } else {
                            egui::ScrollArea::vertical()
                                .max_height(ui.available_height().min(180.0).max(120.0))
                                .show(ui, |ui| {
                                    let start = run_view
                                        .last_log
                                        .len()
                                        .saturating_sub(LOG_DISPLAY_LIMIT);
                                    for line in &run_view.last_log[start..] {
                                        ui.label(line);
                                    }
                                });
                        }
                    }
                });

                if let Some(action) = next_action {
                    self.run_action(action);
                }

                egui::TopBottomPanel::bottom("run_footer").show(ctx, |ui| {
                    let (run_depth, can_save) = match self.run_state.as_ref() {
                        Some(run_view) => (
                            run_view.run_state.run_depth,
                            run_view.live_fight.is_none(),
                        ),
                        None => return,
                    };
                    ui.horizontal(|ui| {
                        let suggested =
                            format!("{}-depth{}", self.creation.name.trim(), run_depth);
                        if self.run_save_name.trim().is_empty() {
                            self.run_save_name = suggested;
                        }
                        ui.label("Run save");
                        ui.text_edit_singleline(&mut self.run_save_name);
                        if ui.add_enabled(can_save, egui::Button::new("Save run")).clicked() {
                            self.save_run();
                        }
                        if !can_save {
                            ui.label("Finish the fight to save.");
                        }
                    });
                    if let Some(status) = self.run_save_status.as_ref() {
                        ui.separator();
                        ui.label(status);
                    }
                });
            }
        }
    }
}

fn clamp_stat_adjustment(base: u8, delta: i32) -> u8 {
    let adjusted = base as i32 + delta;
    adjusted.clamp(1, 25) as u8
}

fn apply_stat_adjustment(score: &mut AbilityScore, delta: i32) {
    score.base = clamp_stat_adjustment(score.base, delta);
}

fn apply_percentile(score: &mut AbilityScore, delta: u8) {
    let total = score_total(score).saturating_add(delta as i32);
    let capped = total.clamp(1, 25 * 100);
    *score = score_from_total(capped);
}

fn subtract_percentile(score: &mut AbilityScore, delta: u8) {
    let total = score_total(score).saturating_sub(delta as i32);
    let capped = total.max(1);
    *score = score_from_total(capped);
}

fn stat_at_cap(score: &AbilityScore) -> bool {
    score_total(score) >= 25 * 100
}

fn bp_increment(score: &AbilityScore) -> u8 {
    if score.base < 10 {
        10
    } else if score.base >= 16 {
        3
    } else {
        5
    }
}

fn score_total(score: &AbilityScore) -> i32 {
    let base = score.base.max(1) as i32;
    let percentile = if score.percentile == 0 { 100 } else { score.percentile } as i32;
    (base - 1) * 100 + percentile
}

fn score_from_total(total: i32) -> AbilityScore {
    let total = total.max(1);
    let base = ((total - 1) / 100 + 1).min(25) as u8;
    let percentile = ((total - 1) % 100 + 1) as u8;
    AbilityScore::new(base, percentile)
}

fn format_percentile(value: u8) -> String {
    if value == 0 || value >= 100 {
        "00".to_string()
    } else {
        format!("{:02}", value)
    }
}

fn format_score(score: AbilityScore) -> String {
    format!("{}/{}", score.base, format_percentile(score.percentile))
}

fn talent_display_label(selection: &TalentSelection, talent_catalog: &TalentCatalog) -> String {
    let talent_name = talent_catalog
        .entries()
        .iter()
        .find(|talent| talent.id == selection.id)
        .map(|talent| talent.name.as_str())
        .unwrap_or(selection.id.as_str());
    let mut details: Vec<String> = Vec::new();
    if let Some(weapon) = selection.weapon.as_ref() {
        details.push(weapon.clone());
    }
    if selection.rank > 1 {
        details.push(format!("rank {}", selection.rank));
    }
    if details.is_empty() {
        talent_name.to_string()
    } else {
        format!("{talent_name} ({})", details.join(", "))
    }
}

fn render_character_summary(
    ctx: &egui::Context,
    creation: &CreationState,
    race_catalog: &[RaceSpec],
    talent_catalog: &TalentCatalog,
    available_points: PointPool,
    effective_cha: u8,
    looks_delta: i32,
    run_view: Option<&RunViewState>,
) {
    egui::SidePanel::right("character_summary")
        .min_width(260.0)
        .show(ctx, |ui| {
            ui.heading("Character Summary");
            ui.separator();
            ui.label(format!("Name: {}", creation.name));
            if let Some(race_idx) = creation.race_index {
                if let Some(race) = race_catalog.get(race_idx) {
                    ui.label(format!("Race: {}", race.name));
                    ui.label(format!("Base HP: {}", race.base_hp));
                } else {
                    ui.label("Race: None");
                }
            } else {
                ui.label("Race: None");
            }
            ui.separator();
            ui.label("Abilities");
            for (idx, label) in STAT_LABELS.iter().enumerate() {
                let score = creation.stats[idx];
                if idx == 6 {
                    let text = format!(
                        "{label}: {}/{} (raw {}/{}, looks {:+})",
                        effective_cha,
                        format_percentile(score.percentile),
                        score.base,
                        format_percentile(score.percentile),
                        looks_delta
                    );
                    ui.label(text);
                } else {
                    ui.label(format!("{label}: {}", format_score(score)));
                }
            }
            ui.separator();
            ui.label("Points");
            ui.label(format!("BP: {}", available_points.bp));
            ui.label(format!("LP: {}", available_points.lp));
            ui.label(format!("AP: {}", available_points.ap));
            ui.label(format!("RP: {}", available_points.rp));
            ui.separator();
            ui.label("Talents");
            if creation.player.talents.is_empty() {
                ui.label("None");
            } else {
                let mut labels: Vec<String> = creation
                    .player
                    .talents
                    .iter()
                    .map(|selection| talent_display_label(selection, talent_catalog))
                    .collect();
                labels.sort();
                egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                    for label in labels {
                        ui.label(label);
                    }
                });
            }
            if let Some(run_view) = run_view {
                ui.separator();
                ui.label("Run");
                ui.label(format!("Depth: {}", run_view.run_state.run_depth));
                ui.label(format!("Days: {}", run_view.days_elapsed));
                ui.label(format!("Gold: {}", run_view.run_state.inventory.gold));
                ui.label(format!(
                    "Wounds: {}",
                    run_view.run_state.total_wound_damage()
                ));
                ui.label(format!("XP: {}", run_view.run_state.player.xp));
                if run_view.training_days > 0 {
                    ui.label(format!("Training days: {}", run_view.training_days));
                }
            }
        });
}

fn scaled_enemy_level(player_level: u8, run_depth: u32) -> u8 {
    let depth_bonus = (run_depth / 2) as u8;
    player_level.saturating_add(depth_bonus)
}

fn draw_live_arena(
    ui: &mut egui::Ui,
    rect: Rect,
    sim: &hackmaster_sim::core::sim::SimState,
    player_name: &str,
    enemy_name: &str,
    floaters: &[DamageFloat],
    ui_time: f32,
) {
    let padding = 16.0;
    if rect.width() <= padding * 2.0 || rect.height() <= padding * 2.0 {
        return;
    }
    let painter = ui.painter();
    let bg = ui.style().visuals.panel_fill;
    painter.rect_filled(rect, 0.0, bg);

    let left = rect.left() + padding;
    let right = rect.right() - padding;
    let arena_width = (right - left).max(1.0);
    let scale = arena_width / sim.config.start_distance.max(1.0);
    if !scale.is_finite() {
        return;
    }

    let bar_height = 8.0;
    let gap = 16.0;
    let bar_width = ((right - left) - gap).max(1.0) * 0.5;
    let bar_y = rect.top() + padding * 0.5;
    let timeline_y = bar_y + bar_height + 18.0;

    draw_swing_timeline(ui, left, right, timeline_y, sim);

    let ground_y = rect.center().y + rect.height() * 0.1;
    painter.line_segment(
        [Pos2::new(left, ground_y), Pos2::new(right, ground_y)],
        (2.0, Color32::from_gray(80)),
    );

    let mut x0 = left + sim.actors[0].position * scale;
    let mut x1 = left + sim.actors[1].position * scale;
    x0 = x0.clamp(left, right);
    x1 = x1.clamp(left, right);
    let min_gap = 24.0;
    if (x1 - x0).abs() < min_gap {
        let dir = if x1 >= x0 { 1.0 } else { -1.0 };
        x1 = (x0 + dir * min_gap).clamp(left, right);
    }

    let player_color = Color32::from_rgb(214, 93, 69);
    let enemy_color = Color32::from_rgb(70, 140, 210);
    painter.circle_filled(Pos2::new(x0, ground_y - 12.0), 7.0, player_color);
    painter.circle_filled(Pos2::new(x1, ground_y - 12.0), 7.0, enemy_color);

    for idx in 0..2 {
        let hp = sim.combatants[idx].state.hp.max(0) as f32;
        let max_hp = sim.combatants[idx].sheet.vitals.max_hp.max(1) as f32;
        let ratio = (hp / max_hp).clamp(0.0, 1.0);
        let bar_x = if idx == 0 { left } else { right - bar_width };
        let bg_rect =
            Rect::from_min_size(Pos2::new(bar_x, bar_y), egui::vec2(bar_width, bar_height));
        painter.rect_filled(bg_rect, 2.0, Color32::from_gray(40));
        let fill_width = bar_width * ratio;
        let fill_x = if idx == 0 {
            bar_x
        } else {
            bar_x + (bar_width - fill_width)
        };
        let fill_rect =
            Rect::from_min_size(Pos2::new(fill_x, bar_y), egui::vec2(fill_width, bar_height));
        let bar_color = if idx == 0 { player_color } else { enemy_color };
        painter.rect_filled(fill_rect, 2.0, bar_color);
        let name = if idx == 0 { player_name } else { enemy_name };
        let align = if idx == 0 {
            egui::Align2::LEFT_CENTER
        } else {
            egui::Align2::RIGHT_CENTER
        };
        let text_x = if idx == 0 { bar_x } else { bar_x + bar_width };
        painter.text(
            Pos2::new(text_x, bar_y - 4.0),
            align,
            name,
            egui::TextStyle::Body.resolve(ui.style()),
            Color32::from_gray(220),
        );
    }

    draw_damage_floaters(
        ui,
        floaters,
        ui_time,
        x0,
        x1,
        ground_y - 26.0,
    );
}

fn draw_swing_timeline(
    ui: &egui::Ui,
    left: f32,
    right: f32,
    y: f32,
    sim: &hackmaster_sim::core::sim::SimState,
) {
    let painter = ui.painter();
    if right <= left {
        return;
    }
    let horizon = 8.0;
    let now = sim.elapsed_seconds as f32;
    let scale = (right - left) / horizon;
    let line_color = Color32::from_gray(70);
    painter.line_segment([Pos2::new(left, y), Pos2::new(right, y)], (2.0, line_color));

    for tick in 0..=8 {
        let x = left + tick as f32 * scale;
        let tick_h = if tick % 2 == 0 { 6.0 } else { 4.0 };
        painter.line_segment(
            [Pos2::new(x, y - tick_h), Pos2::new(x, y + tick_h)],
            (1.0, line_color),
        );
    }

    let player_color = Color32::from_rgb(214, 93, 69);
    let enemy_color = Color32::from_rgb(70, 140, 210);
    for idx in 0..2 {
        let color = if idx == 0 { player_color } else { enemy_color };
        if let Some(next) = sim.combatants[idx].state.next_attack_time_primary {
            let t = (next - now).max(0.0).min(horizon);
            let x = left + t * scale;
            let pos = Pos2::new(x, y - 14.0);
            painter.circle_filled(pos, 6.0, color);
        }
        if let Some(next) = sim.combatants[idx].state.next_attack_time_secondary {
            let t = (next - now).max(0.0).min(horizon);
            let x = left + t * scale;
            let pos = Pos2::new(x, y - 4.0);
            let secondary_color = Color32::from_rgb(
                ((color.r() as u16 + 255) / 2) as u8,
                ((color.g() as u16 + 255) / 2) as u8,
                ((color.b() as u16 + 255) / 2) as u8,
            );
            painter.circle_filled(pos, 4.0, secondary_color);
        }
    }
}

fn draw_damage_floaters(
    ui: &egui::Ui,
    floaters: &[DamageFloat],
    ui_time: f32,
    player_x: f32,
    enemy_x: f32,
    base_y: f32,
) {
    let painter = ui.painter();
    let lifetime = 1.2;
    let rise_per_sec = 26.0;
    for floater in floaters {
        let age = ui_time - floater.start_time;
        if age < 0.0 || age > lifetime {
            continue;
        }
        let alpha = 1.0 - (age / lifetime);
        let alpha_u8 = (alpha * 255.0).clamp(0.0, 255.0) as u8;
        let color = if floater.is_shield {
            Color32::from_rgba_premultiplied(80, 180, 220, alpha_u8)
        } else {
            Color32::from_rgba_premultiplied(230, 70, 70, alpha_u8)
        };
        let x = if floater.target_idx == 0 {
            player_x
        } else {
            enemy_x
        } + floater.offset;
        let y = base_y - age * rise_per_sec;
        painter.text(
            Pos2::new(x, y),
            egui::Align2::CENTER_CENTER,
            floater.value.to_string(),
            egui::TextStyle::Heading.resolve(ui.style()),
            color,
        );
    }
}

fn ingest_live_events(live: &mut LiveFight) {
    let end = live.sim.combat_events.len();
    for idx in live.seen_events..end {
        let event = &live.sim.combat_events[idx];
        live.log_lines
            .push(sim::format_combat_event_line(event, &live.sim.combatants));
        let (damage, shield_damage) = match &event.kind {
            sim::CombatEventKind::Attack(attack) => (attack.damage, attack.shield_damage),
            _ => (0, 0),
        };
        let defender_idx = event.defender_idx;
        if damage > 0 {
            push_damage_float(live, damage, defender_idx, false);
        }
        if shield_damage > 0 {
            push_damage_float(live, shield_damage, defender_idx, true);
        }
    }
    live.seen_events = end;
}

fn prune_floaters(live: &mut LiveFight) {
    let lifetime = 1.2;
    live.floaters
        .retain(|floater| live.ui_elapsed - floater.start_time <= lifetime);
}

fn push_damage_float(live: &mut LiveFight, value: i32, target_idx: usize, is_shield: bool) {
    let offset = ((live.float_seed % 5) as f32 - 2.0) * 8.0;
    live.float_seed = live.float_seed.wrapping_add(1);
    live.floaters.push(DamageFloat {
        value,
        target_idx,
        start_time: live.ui_elapsed,
        offset,
        is_shield,
    });
}

struct AutobattlerBuilder<'a> {
    player_base: PlayerConfig,
    enemy_weapon_id: WeaponId,
    weapon_catalog: &'a WeaponCatalog,
    armor_catalog: &'a ArmorCatalog,
    shield_catalog: &'a ShieldCatalog,
    npc_presets: &'a NpcPresetCatalog,
    talent_catalog: &'a TalentCatalog,
}

impl CombatantBuilder for AutobattlerBuilder<'_> {
    fn build_player(&self, state: &RunState) -> hackmaster_sim::core::sim::Combatant {
        let mut player = self.player_base.clone();
        player.name = state.player.name.clone();
        player.level = state.player.level;
        player.strength_base = state.player.base_stats.strength.base;
        player.strength_pct = state.player.base_stats.strength.percentile;
        player.dex_base = state.player.base_stats.dexterity.base;
        player.dex_pct = state.player.base_stats.dexterity.percentile;
        player.intelligence = state.player.base_stats.intelligence;
        player.wisdom = state.player.base_stats.wisdom;
        player.constitution = state.player.base_stats.constitution;
        player.looks = state.player.base_stats.looks;
        player.charisma = state.player.base_stats.charisma;
        player.talents = state.player.talents.clone();
        let mut combatant = game_logic::build_combatant(
            &player,
            self.weapon_catalog,
            self.armor_catalog,
            self.shield_catalog,
            self.npc_presets,
            self.talent_catalog,
        );
        let wound_total = state.total_wound_damage();
        if wound_total > 0 {
            let wound_total = i32::try_from(wound_total).unwrap_or(i32::MAX);
            let adjusted_hp = (combatant.sheet.vitals.max_hp - wound_total).max(0);
            combatant.state.hp = adjusted_hp;
        }
        combatant
    }

    fn build_enemy(&self, enemy: &EnemyProfile) -> hackmaster_sim::core::sim::Combatant {
        let mut npc = PlayerConfig::new("Hobgoblin", self.enemy_weapon_id);
        npc.level = enemy.level;
        npc.npc_preset = Some(enemy.preset_id);
        game_logic::build_combatant(
            &npc,
            self.weapon_catalog,
            self.armor_catalog,
            self.shield_catalog,
            self.npc_presets,
            self.talent_catalog,
        )
    }
}

fn player_profile_from_config(config: &PlayerConfig) -> PlayerProfile {
    PlayerProfile {
        name: config.name.clone(),
        level: config.level,
        xp: 0,
        base_stats: AbilitySet {
            strength: AbilityScore::new(config.strength_base, config.strength_pct),
            intelligence: config.intelligence,
            wisdom: config.wisdom,
            dexterity: AbilityScore::new(config.dex_base, config.dex_pct),
            constitution: config.constitution,
            looks: config.looks,
            charisma: config.charisma,
        },
        talents: config.talents.clone(),
    }
}

fn hobgoblin_spawner(npc_presets: &NpcPresetCatalog) -> EnemySpawner {
    let mut spawner = EnemySpawner::default();
    for (index, preset) in npc_presets.entries().iter().enumerate() {
        if let Some(level) = hobgoblin_level(&preset.name) {
            spawner.push(EnemySpawnEntry {
                preset_id: game_logic::NpcPresetId::new(index),
                min_level: level,
                max_level: u8::MAX,
                weight: 1,
            });
        }
    }
    spawner
}

fn hobgoblin_level(name: &str) -> Option<u8> {
    let lower = name.to_ascii_lowercase();
    if lower == "hobgoblin" {
        Some(1)
    } else if let Some(rest) = lower.strip_prefix("hobgoblin ") {
        rest.trim().parse::<u8>().ok()
    } else {
        None
    }
}

fn find_weapon_id_by_name(catalog: &WeaponCatalog, name: &str) -> Option<WeaponId> {
    catalog
        .entries()
        .iter()
        .position(|weapon| weapon.name.eq_ignore_ascii_case(name))
        .and_then(|idx| catalog.id_from_index(idx))
}

fn sanitize_filename(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == ' ' || ch == '-' || ch == '_' {
            if !out.ends_with('_') {
                out.push('_');
            }
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "character".to_string()
    } else {
        trimmed
    }
}

fn save_path_for(file_name: &str) -> PathBuf {
    let dir = data::resolve_writable_data_path(CHARACTER_SAVE_DIR);
    dir.join(file_name)
}

fn run_save_path_for(file_name: &str) -> PathBuf {
    let dir = data::resolve_writable_data_path(RUN_SAVE_DIR);
    dir.join(file_name)
}

fn scan_save_entries() -> Vec<SaveEntry> {
    let dir = data::resolve_writable_data_path(CHARACTER_SAVE_DIR);
    let mut entries = Vec::new();
    let Ok(read_dir) = fs::read_dir(&dir) else {
        return entries;
    };
    for item in read_dir.flatten() {
        let path = item.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some(CHARACTER_SAVE_EXTENSION) {
            continue;
        }
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        let display_name = read_character_save(&path)
            .map(|save| save.name)
            .unwrap_or_else(|_| file_name.clone());
        entries.push(SaveEntry {
            file_name,
            display_name,
        });
    }
    entries.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    entries
}

fn scan_run_save_entries() -> Vec<SaveEntry> {
    let dir = data::resolve_writable_data_path(RUN_SAVE_DIR);
    let mut entries = Vec::new();
    let Ok(read_dir) = fs::read_dir(&dir) else {
        return entries;
    };
    for item in read_dir.flatten() {
        let path = item.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some(RUN_SAVE_EXTENSION) {
            continue;
        }
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        let display_name = read_run_save(&path)
            .map(|save| save.name)
            .unwrap_or_else(|_| file_name.clone());
        entries.push(SaveEntry {
            file_name,
            display_name,
        });
    }
    entries.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    entries
}

fn write_character_save(path: &Path, save: &CharacterSave) -> Result<(), String> {
    data::ensure_parent_dir(path)?;
    let json = serde_json::to_string_pretty(save).map_err(|err| err.to_string())?;
    fs::write(path, json).map_err(|err| err.to_string())
}

fn write_run_save(path: &Path, save: &RunSave) -> Result<(), String> {
    data::ensure_parent_dir(path)?;
    let json = serde_json::to_string_pretty(save).map_err(|err| err.to_string())?;
    fs::write(path, json).map_err(|err| err.to_string())
}

fn read_character_save(path: &Path) -> Result<CharacterSave, String> {
    let contents = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&contents).map_err(|err| err.to_string())
}

fn read_run_save(path: &Path) -> Result<RunSave, String> {
    let contents = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&contents).map_err(|err| err.to_string())
}

fn race_adjustment_summary(race: &RaceSpec) -> String {
    let mut parts = Vec::new();
    let adj = &race.ability_adjustments;
    if adj.strength != 0 {
        parts.push(format!("STR {:+}", adj.strength));
    }
    if adj.dexterity != 0 {
        parts.push(format!("DEX {:+}", adj.dexterity));
    }
    if adj.intelligence != 0 {
        parts.push(format!("INT {:+}", adj.intelligence));
    }
    if adj.wisdom != 0 {
        parts.push(format!("WIS {:+}", adj.wisdom));
    }
    if adj.constitution != 0 {
        parts.push(format!("CON {:+}", adj.constitution));
    }
    if adj.looks != 0 {
        parts.push(format!("LKS {:+}", adj.looks));
    }
    if adj.charisma != 0 {
        parts.push(format!("CHA {:+}", adj.charisma));
    }
    if parts.is_empty() {
        "No stat adjustments".to_string()
    } else {
        parts.join(", ")
    }
}

fn total_talent_costs(
    selections: &[TalentSelection],
    talent_catalog: &TalentCatalog,
) -> PointPool {
    let mut total = PointPool::default();
    for selection in selections {
        let Some(spec) = find_talent(talent_catalog, &selection.id) else {
            continue;
        };
        let cost = talent_cost_for_rank(spec, selection.rank.max(1));
        total = total.add(cost);
    }
    total
}

fn find_talent<'a>(talent_catalog: &'a TalentCatalog, id: &str) -> Option<&'a TalentSpec> {
    talent_catalog
        .entries()
        .iter()
        .find(|talent| talent.id == id)
}

fn talent_cost_for_rank(spec: &TalentSpec, rank: u8) -> PointPool {
    let rank = rank.max(1) as i32;
    PointPool {
        bp: spec.cost_bp.unwrap_or(0) as i32 * rank,
        lp: spec.cost_lp.unwrap_or(0) as i32 * rank,
        ap: 0,
        rp: spec.cost_rp.unwrap_or(0) as i32 * rank,
    }
}

fn max_affordable_rank(spec: &TalentSpec, budget: PointPool) -> u8 {
    let max_rank = spec.max_rank.max(1);
    for rank in (1..=max_rank).rev() {
        let cost = talent_cost_for_rank(spec, rank);
        if budget.can_afford(cost) {
            return rank;
        }
    }
    1
}

fn render_talent_selector(
    ui: &mut egui::Ui,
    id_prefix: &str,
    player: &mut PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    race_catalog: &[RaceSpec],
    talent_catalog: &TalentCatalog,
    active_category: &mut String,
    available_points: PointPool,
    max_height: f32,
) {
    if talent_catalog.is_empty() {
        ui.label("No talents loaded.");
        return;
    }

    let active_race = race_for_player(player, race_catalog);
    let mut categories: BTreeMap<String, Vec<&TalentSpec>> = BTreeMap::new();
    for spec in talent_catalog.entries() {
        if !racial_talent_matches(spec, active_race) {
            continue;
        }
        let category = if spec.category.trim().is_empty() {
            "Uncategorized"
        } else {
            spec.category.as_str()
        };
        categories
            .entry(category.to_string())
            .or_default()
            .push(spec);
    }
    categories.entry(TALENT_TAB_RACIALS.to_string()).or_default();
    let mut categories: Vec<(String, Vec<&TalentSpec>)> = categories.into_iter().collect();
    let total_count: usize = categories.iter().map(|(_, specs)| specs.len()).sum();
    categories.sort_by(|a, b| a.0.cmp(&b.0));

    if active_category != TALENT_TAB_ALL
        && !categories.iter().any(|(name, _)| name == active_category)
    {
        active_category.clear();
        active_category.push_str(TALENT_TAB_ALL);
    }

    ui.horizontal_wrapped(|ui| {
        let all_label = format!("{TALENT_TAB_ALL} ({total_count})");
        if ui
            .selectable_label(active_category.as_str() == TALENT_TAB_ALL, all_label)
            .clicked()
        {
            active_category.clear();
            active_category.push_str(TALENT_TAB_ALL);
        }
        for (category, specs) in &categories {
            let label = format!("{category} ({})", specs.len());
            if ui
                .selectable_label(active_category.as_str() == category.as_str(), label)
                .clicked()
            {
                active_category.clear();
                active_category.push_str(category);
            }
        }
    });
    ui.separator();

    let abilities = game_logic::ability_set_from_player(player);
    let talent_snapshot = player.talents.clone();
    let context = game_logic::TalentContext {
        level: player.level,
        stats: &abilities,
        talents: &talent_snapshot,
    };
    let mut add_queue: Vec<TalentSelection> = Vec::new();
    let mut remove_queue: Vec<usize> = Vec::new();
    let default_group = weapon_catalog
        .get(player.weapon_id)
        .map(|weapon| weapon_group_label(weapon.group))
        .unwrap_or(WEAPON_GROUP_LABELS[0]);

    egui::ScrollArea::vertical().max_height(max_height).show(ui, |ui| {
        if active_category.as_str() == TALENT_TAB_ALL {
            for (category, specs) in &categories {
                if specs.is_empty() {
                    continue;
                }
                ui.separator();
                ui.label(category.as_str());
                for spec in specs {
                    render_talent_entry(
                        ui,
                        id_prefix,
                        player,
                        default_group,
                        weapon_catalog,
                        talent_catalog,
                        spec,
                        &context,
                        available_points,
                        &mut add_queue,
                        &mut remove_queue,
                    );
                }
            }
        } else if let Some((name, specs)) =
            categories.iter().find(|(name, _)| name == active_category)
        {
            if name == TALENT_TAB_RACIALS && specs.is_empty() {
                if active_race.is_some() {
                    ui.label("No racial talents available for the selected race.");
                } else {
                    ui.label("Select a race to view racial talents.");
                }
            }
            for spec in specs {
                render_talent_entry(
                    ui,
                    id_prefix,
                    player,
                    default_group,
                    weapon_catalog,
                    talent_catalog,
                    spec,
                    &context,
                    available_points,
                    &mut add_queue,
                    &mut remove_queue,
                );
            }
        }
    });

    if !add_queue.is_empty() {
        player.talents.extend(add_queue);
    }

    if !remove_queue.is_empty() {
        remove_queue.sort_unstable();
        remove_queue.dedup();
        for index in remove_queue.into_iter().rev() {
            if index < player.talents.len() {
                player.talents.remove(index);
            }
        }
    }
}

fn render_talent_entry(
    ui: &mut egui::Ui,
    id_prefix: &str,
    player: &mut PlayerConfig,
    default_group: &str,
    weapon_catalog: &WeaponCatalog,
    talent_catalog: &TalentCatalog,
    spec: &TalentSpec,
    context: &game_logic::TalentContext<'_>,
    available_points: PointPool,
    add_queue: &mut Vec<TalentSelection>,
    remove_queue: &mut Vec<usize>,
) {
    let selected_index = player.talents.iter().position(|sel| sel.id == spec.id);
    let requirement_failures = game_logic::evaluate_talent_requirements(spec, context);
    let locked = !requirement_failures.is_empty();
    let is_nyi = spec.effects.is_empty();
    let requires_group = game_logic::talent_requires_weapon_group(spec);
    let muted_color = ui.visuals().weak_text_color();
    let can_adjust = !locked && (!is_nyi || requires_group);
    let allow_add = !locked && (!is_nyi || requires_group);
    let base_cost = talent_cost_for_rank(spec, 1);
    let can_afford_base = available_points.can_afford(base_cost);

    ui.group(|ui| {
        ui.horizontal(|ui| {
            if is_nyi {
                ui.colored_label(muted_color, spec.name.as_str());
            } else {
                ui.label(spec.name.as_str());
            }
            if let Some(index) = selected_index {
                if ui.button("Remove").clicked() {
                    remove_queue.push(index);
                }
            } else if ui
                .add_enabled(allow_add && can_afford_base, egui::Button::new("Add"))
                .clicked()
            {
                let weapon = if requires_group {
                    Some(default_group.to_string())
                } else if game_logic::talent_requires_weapon(spec) {
                    weapon_catalog
                        .get(player.weapon_id)
                        .map(|weapon| weapon.name.clone())
                        .or_else(|| {
                            weapon_catalog
                                .entries()
                                .first()
                                .map(|weapon| weapon.name.clone())
                        })
                } else {
                    None
                };
                add_queue.push(TalentSelection {
                    id: spec.id.clone(),
                    rank: 1,
                    weapon,
                });
            }
        });
        if let Some(cost) = spec.cost_bp {
            let text = format!("Cost: {cost} BP");
            if is_nyi {
                ui.colored_label(muted_color, text);
            } else {
                ui.label(text);
            }
        }
        if let Some(cost) = spec.cost_lp {
            let text = format!("Cost: {cost} LP");
            if is_nyi {
                ui.colored_label(muted_color, text);
            } else {
                ui.label(text);
            }
        }
        if let Some(cost) = spec.cost_rp {
            let text = format!("Cost: {cost} RP");
            if is_nyi {
                ui.colored_label(muted_color, text);
            } else {
                ui.label(text);
            }
        }
        if is_nyi {
            ui.colored_label(muted_color, "NYI");
        } else {
            ui.label(spec.description.as_str());
        }
        if locked && !is_nyi {
            ui.colored_label(
                Color32::from_rgb(180, 70, 70),
                "Requirements not met:",
            );
            for failure in &requirement_failures {
                ui.label(format!(
                    "- {}",
                    format_talent_requirement_failure(failure, talent_catalog)
                ));
            }
        }

        if let Some(index) = selected_index {
            let selection = &mut player.talents[index];
            let max_rank = spec.max_rank.max(1);
            if selection.rank == 0 || selection.rank > max_rank {
                selection.rank = selection.rank.clamp(1, max_rank);
            }

            let current_cost = talent_cost_for_rank(spec, selection.rank);
            let budget = available_points.add(current_cost);
            let max_affordable = max_affordable_rank(spec, budget);
            if max_rank > 1 {
                let max_rank = max_rank.min(max_affordable);
                ui.add_enabled(
                    can_adjust,
                    egui::Slider::new(&mut selection.rank, 1..=max_rank)
                        .step_by(1.0)
                        .text("Rank"),
                );
            } else {
                selection.rank = 1;
                if is_nyi {
                    ui.colored_label(muted_color, "Rank: 1");
                } else {
                    ui.label("Rank: 1");
                }
            }

            if requires_group {
                if selection.weapon.is_none()
                    || !WEAPON_GROUP_LABELS
                        .iter()
                        .any(|label| Some(*label) == selection.weapon.as_deref())
                {
                    selection.weapon = Some(default_group.to_string());
                }
                let selected_text = selection
                    .weapon
                    .clone()
                    .unwrap_or_else(|| "Select group".to_string());
                ui.horizontal(|ui| {
                    if is_nyi {
                        ui.colored_label(muted_color, "Group");
                    } else {
                        ui.label("Group");
                    }
                    ui.add_enabled_ui(can_adjust, |ui| {
                        egui::ComboBox::from_id_source(format!(
                            "{id_prefix}_talent_group_{}",
                            spec.id
                        ))
                        .selected_text(selected_text)
                        .show_ui(ui, |ui| {
                            for label in WEAPON_GROUP_LABELS {
                                ui.selectable_value(
                                    &mut selection.weapon,
                                    Some(label.to_string()),
                                    label,
                                );
                            }
                        });
                    });
                });
            } else if game_logic::talent_requires_weapon(spec) {
                if selection.weapon.is_none() {
                    selection.weapon = weapon_catalog
                        .get(player.weapon_id)
                        .map(|weapon| weapon.name.clone())
                        .or_else(|| {
                            weapon_catalog
                                .entries()
                                .first()
                                .map(|weapon| weapon.name.clone())
                        });
                }
                let selected_text = selection
                    .weapon
                    .clone()
                    .unwrap_or_else(|| "Select weapon".to_string());
                ui.horizontal(|ui| {
                    if is_nyi {
                        ui.colored_label(muted_color, "Weapon");
                    } else {
                        ui.label("Weapon");
                    }
                    ui.add_enabled_ui(can_adjust, |ui| {
                        egui::ComboBox::from_id_source(format!(
                            "{id_prefix}_talent_weapon_{}",
                            spec.id
                        ))
                        .selected_text(selected_text)
                        .show_ui(ui, |ui| {
                            for weapon in weapon_catalog.entries() {
                                ui.selectable_value(
                                    &mut selection.weapon,
                                    Some(weapon.name.clone()),
                                    weapon.name.as_str(),
                                );
                            }
                        });
                    });
                });
            }
        }
    });
}

fn race_for_player<'a>(player: &PlayerConfig, race_catalog: &'a [RaceSpec]) -> Option<&'a RaceSpec> {
    player
        .race_id
        .as_ref()
        .and_then(|id| race_catalog.iter().find(|race| race.id == *id))
}

fn weapon_group_label(group: WeaponGroup) -> &'static str {
    match group {
        WeaponGroup::Unarmed => "Unarmed",
        WeaponGroup::Axes => "Axes",
        WeaponGroup::Basic => "Basic",
        WeaponGroup::Blunt => "Blunt",
        WeaponGroup::Bows => "Bows",
        WeaponGroup::Crossbows => "Crossbows",
        WeaponGroup::Double => "Double",
        WeaponGroup::Ensnaring => "Ensnaring",
        WeaponGroup::Lashes => "Lashes",
        WeaponGroup::LargeSwords => "Large swords",
        WeaponGroup::SmallSwords => "Small swords",
        WeaponGroup::Polearms => "Polearms",
        WeaponGroup::Spears => "Spears",
        WeaponGroup::Shields => "Shields",
    }
}

fn racial_talent_matches(spec: &TalentSpec, race: Option<&RaceSpec>) -> bool {
    if spec.category != TALENT_TAB_RACIALS {
        return true;
    }
    let Some(race) = race else {
        return false;
    };
    if spec
        .race_ids
        .iter()
        .any(|race_id| race_id == &race.id)
    {
        return true;
    }
    if spec
        .race_categories
        .iter()
        .any(|category| category.eq_ignore_ascii_case(&race.category))
    {
        return true;
    }
    false
}

fn format_talent_requirement_failure(
    failure: &game_logic::TalentRequirementFailure,
    talent_catalog: &TalentCatalog,
) -> String {
    match failure {
        game_logic::TalentRequirementFailure::MinLevel { required, current } => {
            format!("Requires level {required} (current {current}).")
        }
        game_logic::TalentRequirementFailure::MinStatBase {
            stat,
            required,
            current,
        } => {
            format!("Requires {} {required}+ (current {current}).", stat.label())
        }
        game_logic::TalentRequirementFailure::MinStatPercentile {
            stat,
            required,
            current,
        } => {
            let current_label = current
                .map(format_percentile)
                .unwrap_or_else(|| "??".to_string());
            format!(
                "Requires {} percentile {required}+ (current {current_label}).",
                stat.label()
            )
        }
        game_logic::TalentRequirementFailure::RequiresTalent {
            id,
            required_rank,
            current_rank,
        } => {
            let talent_name = talent_catalog
                .entries()
                .iter()
                .find(|talent| talent.id == *id)
                .map(|talent| talent.name.as_str())
                .unwrap_or(id.as_str());
            format!(
                "Requires {talent_name} rank {required_rank} (current {current_rank})."
            )
        }
    }
}

fn main() -> eframe::Result<()> {
    hackmaster_sim::console::maybe_enable_console();
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1100.0, 720.0])
        .with_min_inner_size([720.0, 480.0]);
    if let Some(icon) = hackmaster_sim::assets::app_icon(hackmaster_sim::assets::AppIcon::Autobattler) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "HackMaster Autobattler",
        options,
        Box::new(|_cc| Ok(Box::new(AutobattlerApp::new()))),
    )
}
