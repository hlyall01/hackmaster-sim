use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::utils::Duration;
use bevy::window::WindowResizeConstraints;
use bevy::winit::WinitPlugin;
use bevy_egui::EguiPlugin;
use rand::Rng;

use crate::autobattler::args::AutobattlerArgs;
use crate::autobattler::constants::{
    AUTOBATTLER_CONFIG_PATH, CHARACTER_SAVE_EXTENSION, NPC_PRESETS_PATH, QUICK_STARTS_PATH,
    RUN_AUTOSAVE_FILE, RUN_SAVE_EXTENSION, RUN_SAVE_VERSION, SAVE_VERSION, START_AP, START_BP,
    START_LP, START_RP, STAT_COUNT, WINDOW_HEIGHT, WINDOW_WIDTH,
};
use crate::autobattler::logic::{
    apply_percentile, apply_stat_adjustment, clamp_stat_adjustment, scaled_enemy_level,
    subtract_percentile, total_talent_costs,
};
use crate::autobattler::persistence::{
    read_character_save, read_run_save, run_save_path_for, sanitize_filename, save_path_for,
    scan_run_save_entries, scan_save_entries, write_character_save, write_run_save,
};
use crate::autobattler::render::{setup_render_system, sync_render_system};
use crate::autobattler::screenshot::{
    HeadlessConfig, HeadlessScreenshotPlugin, ScreenshotState, screenshot_system,
};
use crate::autobattler::sprite_review::sprite_review_system;
use crate::autobattler::state::{
    AppScreen, AutobattlerState, CharacterSave, CreationState, CreationStep, DowntimeActivity,
    DowntimeFeedback, EncounterPreview, EventPreview, GearOfferPreview, MerchantPreview,
    PointPool, RunAction, RunRoomKind, RunSave, RunStateSave, RunViewState, SaveEntry,
    SeedContext, SpriteReviewState, TreasurePreview,
};
use crate::autobattler::ui::ui_system;
use crate::autobattler::weapon_mastery;

use crate::autobattler::logic;

use crate::character::{AbilityScore, AbilitySet, AbilitySetFull};
use crate::character::{Progression, ProgressionTier};
use crate::core::catalog::Catalog;
use crate::core::gameplay::{
    AutobattlerConfig, CombatantBuilder, DepthBand, EnemySpawnEntry, EnemySpawner, EventCatalog,
    FightResult, LootTable, RunState, XpCurve, apply_downtime, apply_fight_result, choose_event,
    depth_band_for_depth, encounter_tier_for_depth, resolve_event_choice,
};
use crate::core::rng::{SimRng, derive_seed};
use crate::core::rules::roll_damage_expr;
use crate::core::sim::SimConfig;
use crate::core::skills::{self, SkillCheckResult, SkillDifficulty};
use crate::core::types::{
    EnemyProfile, EquipmentLoadout, EquipmentSlot, Inventory, InventoryGear, PlayerProfile,
    PointPools, RaceSpec, SkillProgress,
};
use crate::game_logic::{
    ArmorCatalog, ArmorId, FighterPreset, FighterPresetCatalog, NpcPresetCatalog, PlayerConfig,
    ShieldCatalog, ShieldId, TalentCatalog, WeaponCatalog, WeaponId,
};
use crate::{character, data, game_logic, sim};

pub struct AutobattlerApp {
    pub screen: AppScreen,
    pub creation: CreationState,
    pub creation_step: CreationStep,
    pub creation_done: bool,
    pub save_entries: Vec<SaveEntry>,
    pub selected_save: Option<usize>,
    pub run_save_entries: Vec<SaveEntry>,
    pub selected_run_save: Option<usize>,
    pub quick_start_presets: FighterPresetCatalog,
    pub selected_quick_start: Option<usize>,
    pub quick_start_status: Option<String>,
    pub fast_start_name: String,
    pub fast_start_weapon: WeaponId,
    pub fast_start_status: Option<String>,
    pub save_name: String,
    pub save_status: Option<String>,
    pub run_save_name: String,
    pub run_save_status: Option<String>,
    pub needs_save_refresh: bool,
    pub run_state: Option<RunViewState>,
    pub autobattler_config: AutobattlerConfig,
    pub run_seed: u64,
    pub seed_dirty: bool,
    pub startup_data_issues: Vec<String>,
    pub weapon_catalog: WeaponCatalog,
    pub armor_catalog: ArmorCatalog,
    pub shield_catalog: ShieldCatalog,
    pub npc_presets: NpcPresetCatalog,
    pub enemy_spawner: EnemySpawner,
    pub loot_table: LootTable,
    pub event_catalog: EventCatalog,
    pub xp_curve: XpCurve,
    pub sim_config: SimConfig,
    pub enemy_weapon_id: WeaponId,
    pub race_catalog: Vec<RaceSpec>,
    pub talent_catalog: TalentCatalog,
}

impl AutobattlerApp {
    fn build_character_save(&self) -> CharacterSave {
        let weapon_name = self
            .weapon_catalog
            .get(self.creation.player.weapon_id)
            .map(|weapon| weapon.name.clone())
            .unwrap_or_default();
        let armor_label = self
            .armor_catalog
            .get(self.creation.player.armor_id)
            .map(|entry| entry.label.clone())
            .unwrap_or_default();
        let shield_name = self
            .shield_catalog
            .get(self.creation.player.shield_id)
            .map(|entry| entry.label.clone())
            .unwrap_or_default();
        CharacterSave {
            version: SAVE_VERSION,
            name: self.creation.name.clone(),
            stats: self
                .creation
                .stats
                .iter()
                .map(|score| crate::autobattler::state::AbilityScoreSave::from_score(*score))
                .collect(),
            race_id: self.creation.player.race_id.clone(),
            talents: self.creation.player.talents.clone(),
            bp_history: self.creation.bp_history.iter().cloned().collect(),
            weapon_name,
            armor_label,
            shield_name,
            alignment: self.creation.alignment.clone(),
            honor: self.creation.honor,
            background: self.creation.background.clone(),
            height: self.creation.height.clone(),
            weight: self.creation.weight.clone(),
            age: self.creation.age.clone(),
            handedness: self.creation.handedness.clone(),
            quirks: self.creation.quirks.clone(),
            flaws: self.creation.flaws.clone(),
            skills: skills::legacy_skill_names(&self.creation.skill_levels),
            skill_levels: self
                .creation
                .skill_levels
                .iter()
                .map(crate::autobattler::state::SkillProgressSave::from_skill_progress)
                .collect(),
            proficiencies: self.creation.proficiencies.clone(),
            starting_money: self.creation.starting_money,
            money_rolled: self.creation.money_rolled,
        }
    }

    fn build_run_save(&self, name: String, run_view: &RunViewState) -> RunSave {
        RunSave {
            version: RUN_SAVE_VERSION,
            name,
            character: self.build_character_save(),
            run_state: RunStateSave::from_state(&run_view.run_state),
            days_elapsed: run_view.days_elapsed,
            training_days: run_view.training_days,
            run_over: run_view.run_over,
            victory: run_view.victory,
            awaiting_downtime_choice: run_view.awaiting_downtime_choice,
            pending_levelup: run_view.pending_levelup.clone(),
            last_action: run_view.last_action,
            selected_activity: run_view.selected_activity,
            last_log: run_view.last_log.clone(),
        }
    }

    fn autosave_run_checkpoint(&mut self, checkpoint: &str) -> bool {
        let Some(run_view) = self.run_state.as_ref() else {
            return false;
        };
        if run_view.live_fight.is_some() {
            return false;
        }
        let path = run_save_path_for(RUN_AUTOSAVE_FILE);
        let run_save = self.build_run_save(format!("Autosave ({checkpoint})"), run_view);
        match write_run_save(&path, &run_save) {
            Ok(()) => {
                self.needs_save_refresh = true;
                true
            }
            Err(err) => {
                self.run_save_status = Some(format!("Autosave failed: {err}"));
                false
            }
        }
    }

    pub fn new() -> Self {
        let required_files = [
            "data/autobattler/autobattler_config.json",
            "data/autobattler/autobattler_quick_starts.json",
            "data/autobattler/events_v1.json",
            "data/autobattler/events_v1_handcrafted.json",
            "data/sim/weapons.json",
            "data/sim/armor.json",
            "data/sim/materials.json",
            "data/sim/npc_presets.json",
            "data/sim/races.json",
            data::TALENTS_PATH,
        ];
        let startup_data_issues = match data::validate_required_data_files(&required_files) {
            Ok(()) => Vec::new(),
            Err(missing) => {
                eprintln!("Required data files missing:");
                for path in &missing {
                    eprintln!("  - {path}");
                }
                missing
            }
        };
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
        let race_catalog = data::load_races("data/sim/races.json").unwrap_or_else(|err| {
            eprintln!("Failed to load races: {err}");
            Vec::new()
        });
        let quick_start_presets =
            data::load_fighter_presets(QUICK_STARTS_PATH).unwrap_or_else(|err| {
                eprintln!("Failed to load quick starts: {err}");
                Catalog::new(Vec::new())
            });
        let talent_catalog = match data::load_talents(data::TALENTS_PATH) {
            Ok(talents) => talents,
            Err(err) => {
                if cfg!(debug_assertions) {
                    panic!("Failed to load talents: {err}");
                }
                eprintln!("Failed to load talents: {err}");
                Catalog::new(Vec::new())
            }
        };
        let run_seed = autobattler_config.seed;
        let weapon_id = weapon_catalog
            .first_id()
            .unwrap_or_else(|| WeaponId::new(0));
        let creation = CreationState::new(weapon_id, run_seed);
        let selected_quick_start = if quick_start_presets.is_empty() {
            None
        } else {
            Some(0)
        };
        let enemy_weapon_id =
            find_weapon_id_by_name(&weapon_catalog, &autobattler_config.enemy_weapon)
                .or_else(|| weapon_catalog.first_id())
                .unwrap_or_else(|| WeaponId::new(0));
        let enemy_spawner = hobgoblin_spawner(&npc_presets);
        let loot_table = autobattler_config.to_loot_table();
        let event_catalog = data::load_autobattler_events("data/autobattler/events_v1.json")
            .unwrap_or_else(|err| {
                eprintln!("Failed to load autobattler events: {err}");
                EventCatalog::default()
            });
        let xp_curve = XpCurve {
            base: 50,
            per_level: 50,
        };
        let sim_config = SimConfig::new(
            autobattler_config.start_distance,
            autobattler_config.stop_distance,
        );
        Self {
            screen: AppScreen::Start,
            creation,
            creation_step: CreationStep::Points,
            creation_done: false,
            save_entries: Vec::new(),
            selected_save: None,
            run_save_entries: Vec::new(),
            selected_run_save: None,
            quick_start_presets,
            selected_quick_start,
            quick_start_status: None,
            fast_start_name: "Adventurer".to_string(),
            fast_start_weapon: weapon_id,
            fast_start_status: None,
            save_name: String::new(),
            save_status: None,
            run_save_name: String::new(),
            run_save_status: None,
            needs_save_refresh: true,
            run_state: None,
            autobattler_config,
            run_seed,
            seed_dirty: false,
            startup_data_issues,
            weapon_catalog,
            armor_catalog,
            shield_catalog,
            npc_presets,
            enemy_spawner,
            loot_table,
            event_catalog,
            xp_curve,
            sim_config,
            enemy_weapon_id,
            race_catalog,
            talent_catalog,
        }
    }

    pub fn available_points(&self) -> PointPool {
        let spent_stats = self
            .creation
            .bp_history
            .iter()
            .map(|history| history.len() as i32)
            .sum::<i32>();
        let spent_talents = total_talent_costs(&self.creation.player.talents, &self.talent_catalog);
        let spent_skills_lp = skills::total_lp_cost(&self.creation.skill_levels);
        PointPool::new(START_BP, START_LP, START_AP, START_RP)
            .sub(PointPool::new(spent_stats, 0, 0, 0))
            .sub(spent_talents)
            .sub(PointPool::new(0, spent_skills_lp, 0, 0))
    }

    pub fn effective_charisma(&self) -> (u8, i32) {
        let looks = self.creation.stats[5].base;
        let delta = character::looks_charisma_adjustment(looks);
        let base = clamp_stat_adjustment(self.creation.stats[6].base, delta);
        (base, delta)
    }

    pub fn starter_gear_cost(&self) -> u32 {
        let weapon = self.weapon_catalog.get(self.creation.player.weapon_id);
        let weapon_cost = weapon.map(|weapon| weapon.price_gp).unwrap_or(0);
        let weapon_two_handed = weapon
            .map(|weapon| weapon.handedness == crate::game_logic::WeaponHandedness::TwoHanded)
            .unwrap_or(false);
        let armor_cost = self
            .armor_catalog
            .get(self.creation.player.armor_id)
            .and_then(|entry| entry.armor.as_ref())
            .map(|armor| armor.price_gp)
            .unwrap_or(0);
        let shield_cost = if weapon_two_handed {
            0
        } else {
            self.shield_catalog
                .get(self.creation.player.shield_id)
                .and_then(|entry| entry.shield.as_ref())
                .map(|shield| shield.price_gp)
                .unwrap_or(0)
        };
        weapon_cost + armor_cost + shield_cost
    }

    pub fn starter_gear_remaining(&self) -> i32 {
        if !self.creation.money_rolled {
            return -1;
        }
        let cost = self.starter_gear_cost() as i32;
        self.creation.starting_money as i32 - cost
    }

    pub fn apply_race_adjustments(&mut self, race: &RaceSpec) {
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

    pub fn reset_creation(&mut self) {
        let weapon_id = self.creation.player.weapon_id;
        self.creation = CreationState::new(weapon_id, self.run_seed);
        self.creation_step = CreationStep::Points;
        self.creation_done = false;
        self.fast_start_name = "Adventurer".to_string();
        self.fast_start_weapon = weapon_id;
        self.fast_start_status = None;
        self.save_name.clear();
        self.save_status = None;
        self.run_save_name.clear();
        self.run_save_status = None;
        self.run_state = None;
        self.seed_dirty = false;
    }

    pub fn can_advance(&self) -> bool {
        match self.creation_step {
            CreationStep::Points => true,
            CreationStep::RollStats => self.creation.stats_locked || self.all_rolls_assigned(),
            CreationStep::ChooseRace => self.creation.race_applied,
            CreationStep::Alignment => self.creation.race_applied,
            CreationStep::FinalizeStats => self.creation.race_applied,
            CreationStep::Honor => true,
            CreationStep::Priors => true,
            CreationStep::QuirksFlaws => true,
            CreationStep::AdvancementTalents => true,
            CreationStep::SkillsTalents => true,
            CreationStep::HitPoints => true,
            CreationStep::DerivedStats => true,
            CreationStep::MoneyGear => false,
        }
    }

    pub fn can_finish(&self) -> bool {
        self.creation_step == CreationStep::MoneyGear
            && self.creation.money_rolled
            && self.starter_gear_remaining() >= 0
    }

    pub fn all_rolls_assigned(&self) -> bool {
        self.creation.assignments.iter().all(|slot| slot.is_some())
    }

    pub fn refresh_saves(&mut self) {
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

    pub fn save_character(&mut self) -> bool {
        let name = if self.save_name.trim().is_empty() {
            self.creation.name.trim()
        } else {
            self.save_name.trim()
        };
        if name.is_empty() {
            self.save_status = Some("Enter a save name.".to_string());
            return false;
        }
        let file_name = format!("{}.{}", sanitize_filename(name), CHARACTER_SAVE_EXTENSION);
        let path = save_path_for(&file_name);
        let save = self.build_character_save();
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

    pub fn save_run(&mut self) -> bool {
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
        let run_save = self.build_run_save(name.to_string(), run_view);
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

    pub fn load_selected_character(&mut self) {
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
                self.creation = CreationState::new(self.creation.player.weapon_id, self.run_seed);
                self.seed_dirty = false;
                self.creation.name = save.name;
                if save.stats.len() >= STAT_COUNT {
                    for (idx, score) in save.stats.iter().take(STAT_COUNT).enumerate() {
                        self.creation.stats[idx] = score.to_score();
                    }
                }
                self.creation.race_index = save
                    .race_id
                    .as_ref()
                    .and_then(|id| self.race_catalog.iter().position(|race| race.id == *id));
                self.creation.player.race_id = save.race_id.clone();
                self.creation.player.talents = save.talents.clone();
                if !save.weapon_name.trim().is_empty() {
                    if let Some(id) =
                        find_weapon_id_by_name(&self.weapon_catalog, &save.weapon_name)
                    {
                        self.creation.player.weapon_id = id;
                    }
                }
                if !save.armor_label.trim().is_empty() {
                    if let Some(id) = find_armor_id_by_label(&self.armor_catalog, &save.armor_label)
                    {
                        self.creation.player.armor_id = id;
                    }
                }
                if !save.shield_name.trim().is_empty() {
                    if let Some(id) =
                        find_shield_id_by_name(&self.shield_catalog, &save.shield_name)
                    {
                        self.creation.player.shield_id = id;
                    }
                }
                self.creation.bp_history = std::array::from_fn(|idx| {
                    save.bp_history.get(idx).cloned().unwrap_or_default()
                });
                self.creation.alignment = if save.alignment.trim().is_empty() {
                    "Unaligned".to_string()
                } else {
                    save.alignment.clone()
                };
                self.creation.honor = save.honor;
                self.creation.background = save.background.clone();
                self.creation.height = save.height.clone();
                self.creation.weight = save.weight.clone();
                self.creation.age = save.age.clone();
                self.creation.handedness = save.handedness.clone();
                self.creation.quirks = save.quirks.clone();
                self.creation.flaws = save.flaws.clone();
                self.creation.proficiencies = save.proficiencies.clone();
                self.creation.starting_money = save.starting_money;
                self.creation.money_rolled = save.money_rolled;
                self.creation.stats_locked = true;
                self.creation.race_applied = save.race_id.is_some();
                self.creation.player.race_applied = save.race_id.is_some();
                self.creation.sync_player_from_stats();
                let ability_scores_full = ability_scores_full_from_creation(&self.creation);
                self.creation.skill_levels = if save.skill_levels.is_empty() {
                    skills::derive_skill_levels_from_legacy(&save.skills, &ability_scores_full)
                } else {
                    save.skill_levels
                        .iter()
                        .map(crate::autobattler::state::SkillProgressSave::to_skill_progress)
                        .collect()
                };
                self.creation_step = CreationStep::MoneyGear;
                self.creation_done = true;
                self.screen = AppScreen::Creation;
                self.save_status = Some("Loaded save.".to_string());
            }
            Err(err) => {
                self.save_status = Some(format!("Failed to load save: {err}"));
            }
        }
    }

    pub fn load_selected_run(&mut self) {
        let Some(index) = self.selected_run_save else {
            return;
        };
        let Some(entry) = self.run_save_entries.get(index) else {
            self.run_save_status = Some("Selected run no longer exists.".to_string());
            return;
        };
        let path = run_save_path_for(&entry.file_name);
        match read_run_save(&path) {
            Ok(save) => {
                let mut run_state = save.run_state.to_state();
                self.run_seed = run_state.run_seed;
                self.creation = CreationState::new(self.creation.player.weapon_id, self.run_seed);
                self.seed_dirty = false;
                self.creation.name = save.character.name.clone();
                if save.character.stats.len() >= STAT_COUNT {
                    for (idx, score) in save.character.stats.iter().take(STAT_COUNT).enumerate() {
                        self.creation.stats[idx] = score.to_score();
                    }
                }
                self.creation.race_index = save
                    .character
                    .race_id
                    .as_ref()
                    .and_then(|id| self.race_catalog.iter().position(|race| race.id == *id));
                self.creation.player.race_id = save.character.race_id.clone();
                self.creation.player.talents = save.character.talents.clone();
                if !save.character.weapon_name.trim().is_empty() {
                    if let Some(id) =
                        find_weapon_id_by_name(&self.weapon_catalog, &save.character.weapon_name)
                    {
                        self.creation.player.weapon_id = id;
                    }
                }
                if !save.character.armor_label.trim().is_empty() {
                    if let Some(id) =
                        find_armor_id_by_label(&self.armor_catalog, &save.character.armor_label)
                    {
                        self.creation.player.armor_id = id;
                    }
                }
                if !save.character.shield_name.trim().is_empty() {
                    if let Some(id) =
                        find_shield_id_by_name(&self.shield_catalog, &save.character.shield_name)
                    {
                        self.creation.player.shield_id = id;
                    }
                }
                self.creation.bp_history = std::array::from_fn(|idx| {
                    save.character
                        .bp_history
                        .get(idx)
                        .cloned()
                        .unwrap_or_default()
                });
                self.creation.alignment = if save.character.alignment.trim().is_empty() {
                    "Unaligned".to_string()
                } else {
                    save.character.alignment.clone()
                };
                self.creation.honor = save.character.honor;
                self.creation.background = save.character.background.clone();
                self.creation.height = save.character.height.clone();
                self.creation.weight = save.character.weight.clone();
                self.creation.age = save.character.age.clone();
                self.creation.handedness = save.character.handedness.clone();
                self.creation.quirks = save.character.quirks.clone();
                self.creation.flaws = save.character.flaws.clone();
                self.creation.proficiencies = save.character.proficiencies.clone();
                self.creation.starting_money = save.character.starting_money;
                self.creation.money_rolled = save.character.money_rolled;
                self.creation.stats_locked = true;
                self.creation.race_applied = save.character.race_id.is_some();
                self.creation.player.race_applied = save.character.race_id.is_some();
                self.creation.sync_player_from_stats();
                let ability_scores_full = ability_scores_full_from_creation(&self.creation);
                self.creation.skill_levels = if save.character.skill_levels.is_empty() {
                    skills::derive_skill_levels_from_legacy(
                        &save.character.skills,
                        &ability_scores_full,
                    )
                } else {
                    save.character
                        .skill_levels
                        .iter()
                        .map(crate::autobattler::state::SkillProgressSave::to_skill_progress)
                        .collect()
                };
                self.creation_step = CreationStep::MoneyGear;
                self.creation_done = true;
                if run_state.player.weapon_masteries.is_empty() {
                    weapon_mastery::seed_profile_masteries_from_config(
                        &mut run_state.player,
                        &self.creation.player,
                        &self.weapon_catalog,
                        &self.shield_catalog,
                    );
                }
                self.run_state = Some(RunViewState {
                    run_state,
                    seed_context: SeedContext {
                        run_seed: self.run_seed,
                        ..SeedContext::default()
                    },
                    pending_encounter: None,
                    pending_event: None,
                    pending_treasure: None,
                    pending_merchant: None,
                    last_outcome: None,
                    last_action: save.last_action,
                    last_log: save.last_log.clone(),
                    days_elapsed: save.days_elapsed,
                    training_days: save.training_days,
                    run_over: save.run_over,
                    victory: save.victory,
                    awaiting_downtime_choice: save.awaiting_downtime_choice,
                    pending_levelup: save.pending_levelup.clone(),
                    selected_activity: save.selected_activity,
                    downtime_feedback: None,
                    live_fight: None,
                });
                if !save.run_over
                    && !save.awaiting_downtime_choice
                    && save.pending_levelup.is_none()
                {
                    self.prepare_next_encounter();
                }
                self.screen = AppScreen::Run;
                self.run_save_status = Some("Loaded run.".to_string());
            }
            Err(err) => {
                self.run_save_status = Some(format!("Failed to load run: {err}"));
            }
        }
    }

    pub fn start_run_from_creation(&mut self) {
        let available_points = self.available_points();
        let points = PointPools::new(
            available_points.bp,
            available_points.lp,
            available_points.ap,
            available_points.rp,
        );
        let ability_scores_full = ability_scores_full_from_creation(&self.creation);
        let alignment = if self.creation.alignment.trim().is_empty() {
            None
        } else {
            Some(self.creation.alignment.clone())
        };
        let background = if self.creation.background.trim().is_empty() {
            None
        } else {
            Some(self.creation.background.clone())
        };
        let mut player_profile = player_profile_from_config(
            &self.creation.player,
            ability_scores_full,
            points,
            self.creation.honor,
            alignment,
            background,
            self.creation.quirks.clone(),
            self.creation.flaws.clone(),
            self.creation.skill_levels.clone(),
            self.creation.proficiencies.clone(),
        );
        weapon_mastery::seed_profile_masteries_from_config(
            &mut player_profile,
            &self.creation.player,
            &self.weapon_catalog,
            &self.shield_catalog,
        );
        let starting_budget = if self.creation.money_rolled {
            self.creation.starting_money
        } else {
            self.autobattler_config.loot.gold_min
        };
        let gear_cost = if self.creation.money_rolled {
            self.starter_gear_cost()
        } else {
            0
        };
        let starting_gold = starting_budget.saturating_sub(gear_cost);
        let run_seed = self.creation.run_seed;
        let run_state = RunState {
            player: player_profile,
            inventory: Inventory {
                gold: starting_gold,
                items: Vec::new(),
                stash: Vec::new(),
                loadout: self.starting_loadout_from_creation(),
            },
            run_depth: 1,
            run_seed,
            encounter_index: 0,
            last_encounter_tier: crate::core::gameplay::EncounterTier::Normal,
            last_encounter_band: DepthBand::Novice,
            event_flags: Vec::new(),
            seen_event_ids: Vec::new(),
            wounds: Vec::new(),
        };
        let mut run_view = RunViewState::new(run_state);
        run_view.seed_context.run_seed = run_seed;
        self.run_state = Some(run_view);
        self.screen = AppScreen::Run;
        self.autosave_run_checkpoint("run-start");
        self.prepare_next_encounter();
    }

    pub fn prepare_next_encounter(&mut self) {
        let run_state = {
            let Some(run_view) = self.run_state.as_ref() else {
                return;
            };
            if run_view.run_over
                || run_view.live_fight.is_some()
                || run_view.awaiting_downtime_choice
                || run_view.pending_levelup.is_some()
                || run_view.pending_encounter.is_some()
                || run_view.pending_event.is_some()
                || run_view.pending_treasure.is_some()
                || run_view.pending_merchant.is_some()
            {
                return;
            }
            run_view.run_state.clone()
        };
        let room_kind = next_room_kind(&run_state, &self.autobattler_config);
        let encounter_index = run_state.encounter_index as u64;
        let encounter_tier = if matches!(room_kind, RunRoomKind::Boss) {
            crate::core::gameplay::EncounterTier::Boss
        } else {
            encounter_tier_for_depth(run_state.run_depth)
        };
        let reward_seed = match room_kind {
            RunRoomKind::Treasure => Some(derive_seed(
                run_state.run_seed,
                "treasure-room",
                encounter_index,
            )),
            RunRoomKind::Merchant => Some(derive_seed(
                run_state.run_seed,
                "merchant-room",
                encounter_index,
            )),
            _ => None,
        };
        let pending_treasure = if matches!(room_kind, RunRoomKind::Treasure) {
            reward_seed.map(|seed| self.build_treasure_preview(&run_state, seed))
        } else {
            None
        };
        let pending_merchant = if matches!(room_kind, RunRoomKind::Merchant) {
            reward_seed.map(|seed| self.build_merchant_preview(&run_state, seed))
        } else {
            None
        };
        let Some(run_view) = self.run_state.as_mut() else {
            return;
        };
        if let Some(seed) = reward_seed {
            match room_kind {
                RunRoomKind::Treasure => {
                    run_view.pending_treasure = pending_treasure;
                }
                RunRoomKind::Merchant => {
                    run_view.pending_merchant = pending_merchant;
                }
                _ => {}
            }
            run_view.seed_context = SeedContext {
                run_seed: run_state.run_seed,
                reward_seed: Some(seed),
                ..SeedContext::default()
            };
            return;
        }
        let player_level = run_state.player.level;
        let effective_level = scaled_enemy_level(player_level, run_state.run_depth)
            .saturating_add(match encounter_tier {
                crate::core::gameplay::EncounterTier::Normal => 0,
                crate::core::gameplay::EncounterTier::Elite => 1,
                crate::core::gameplay::EncounterTier::Boss => 2,
            });
        let allow_event = matches!(room_kind, RunRoomKind::Event);
        let event_seed = derive_seed(run_view.run_state.run_seed, "event-spawn", encounter_index);
        if allow_event {
            let kind_seed = derive_seed(run_view.run_state.run_seed, "event-kind", encounter_index);
            let resolve_seed = derive_seed(
                run_view.run_state.run_seed,
                "event-resolve",
                encounter_index,
            );
            let mut kind_rng = SimRng::from_seed(kind_seed);
            if let Some(event) = choose_event(
                &self.event_catalog,
                &run_view.run_state,
                encounter_tier,
                &mut kind_rng,
            ) {
                run_view.pending_event = Some(EventPreview {
                    event,
                    tier: encounter_tier,
                    resolve_seed,
                });
                run_view.seed_context = SeedContext {
                    run_seed: run_view.run_state.run_seed,
                    event_seed: Some(event_seed),
                    ..SeedContext::default()
                };
                return;
            }
        }
        let spawn_seed = derive_seed(run_view.run_state.run_seed, "spawn", encounter_index);
        let mut spawn_rng = SimRng::from_seed(spawn_seed);
        let Some(enemy_profile) = self
            .enemy_spawner
            .spawn_for_level(effective_level, &mut spawn_rng)
        else {
            return;
        };
        let default_weapon_name = self
            .weapon_catalog
            .get(self.enemy_weapon_id)
            .map(|w| w.name.clone())
            .unwrap_or_else(|| "weapon".to_string());
        let (enemy_name, weapon_name, armor_label) = self
            .npc_presets
            .get(enemy_profile.preset_id)
            .map(|preset| {
                (
                    preset.name.clone(),
                    default_weapon_name.clone(),
                    format!("armor (DR {})", preset.armor_dr),
                )
            })
            .unwrap_or_else(|| {
                (
                    "Hobgoblin".to_string(),
                    default_weapon_name,
                    "armor".to_string(),
                )
            });
        run_view.pending_encounter = Some(EncounterPreview {
            enemy: enemy_profile,
            tier: encounter_tier,
            enemy_name,
            armor_label,
            weapon_name,
        });
        let fight_seed = derive_seed(run_view.run_state.run_seed, "combat", encounter_index);
        run_view.seed_context = SeedContext {
            run_seed: run_view.run_state.run_seed,
            spawn_seed: Some(spawn_seed),
            combat_seed: Some(fight_seed),
            event_seed: Some(event_seed),
            reward_seed: None,
            ..SeedContext::default()
        };
    }

    pub fn start_live_fight(&mut self) {
        let Some(run_view) = self.run_state.as_mut() else {
            return;
        };
        if run_view.run_over || run_view.awaiting_downtime_choice {
            return;
        }
        let Some(encounter) = run_view.pending_encounter.take() else {
            return;
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

        let mut player_combatant = builder.build_player(&run_view.run_state);
        let mut enemy_combatant = builder.build_enemy(&encounter.enemy);
        player_combatant.team_id = 0;
        enemy_combatant.team_id = 1;
        let encounter_index = run_view.run_state.encounter_index as u64;
        let fight_seed = derive_seed(run_view.run_state.run_seed, "combat", encounter_index);
        let mut sim =
            crate::core::sim::SimState::with_rng(self.sim_config, SimRng::from_seed(fight_seed));
        sim.reset_with_combatants(vec![player_combatant, enemy_combatant]);

        run_view.live_fight = Some(crate::autobattler::state::LiveFight {
            sim,
            enemy: encounter.enemy,
            tier: encounter.tier,
            rest_days: 0,
            resting: false,
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

    pub fn run_action(&mut self, action: RunAction) {
        let Some(run_view) = self.run_state.as_mut() else {
            return;
        };
        if run_view.run_over
            || run_view.pending_levelup.is_some()
            || !run_view.awaiting_downtime_choice
        {
            return;
        }
        let rest_days = self.autobattler_config.rest_days_between_encounters.max(1);
        let resting = action.is_resting();
        apply_downtime(&mut run_view.run_state, rest_days, resting);
        run_view.last_action = Some(action);
        run_view.days_elapsed = run_view.days_elapsed.saturating_add(rest_days);
        if matches!(action, RunAction::Activity) {
            run_view.training_days = run_view.training_days.saturating_add(rest_days);
        }
        if matches!(action, RunAction::Activity) {
            let activity = run_view.selected_activity;
            let downtime_seed = derive_seed(
                run_view.run_state.run_seed,
                "downtime-activity",
                run_view.run_state.encounter_index.saturating_sub(1) as u64,
            );
            let feedback_lines =
                apply_downtime_activity(&mut run_view.run_state, activity, downtime_seed);
            run_view.downtime_feedback = Some(DowntimeFeedback {
                title: format!("Downtime: {}", activity.label()),
                activity: Some(activity),
                lines: feedback_lines,
                animation_seconds: 1.4,
            });
        } else {
            run_view.downtime_feedback = Some(DowntimeFeedback {
                title: "Downtime: Rest".to_string(),
                activity: None,
                lines: vec![format!("Recovered with full rest for {rest_days} days.")],
                animation_seconds: 1.0,
            });
        }
        run_view.awaiting_downtime_choice = false;
        self.autosave_run_checkpoint("post-choice");
        self.prepare_next_encounter();
    }

    pub fn confirm_level_up(&mut self) {
        let Some(run_view) = self.run_state.as_mut() else {
            return;
        };
        if run_view.run_over {
            return;
        }
        let Some(checkpoint) = run_view.pending_levelup.clone() else {
            return;
        };
        if checkpoint.remaining_slots() > 0 {
            return;
        }
        let grants = checkpoint.grants();
        run_view.run_state.player.points.bp = run_view
            .run_state
            .player
            .points
            .bp
            .saturating_add(grants.bp);
        run_view.run_state.player.points.lp = run_view
            .run_state
            .player
            .points
            .lp
            .saturating_add(grants.lp);
        run_view.run_state.player.points.ap = run_view
            .run_state
            .player
            .points
            .ap
            .saturating_add(grants.ap);
        run_view.run_state.player.points.rp = run_view
            .run_state
            .player
            .points
            .rp
            .saturating_add(grants.rp);
        run_view.pending_levelup = None;
        run_view.last_log.push(format!(
            "Level-up confirmed: +{} BP, +{} LP, +{} AP, +{} RP",
            grants.bp, grants.lp, grants.ap, grants.rp
        ));
        run_view.awaiting_downtime_choice = true;
        self.autosave_run_checkpoint("post-levelup");
    }

    pub fn skip_encounter(&mut self) {
        let Some(run_view) = self.run_state.as_mut() else {
            return;
        };
        if run_view.run_over || run_view.awaiting_downtime_choice {
            return;
        }
        let Some(encounter) = run_view.pending_encounter.take() else {
            return;
        };
        run_view.last_log = vec![format!(
            "You spot {} in {}, wielding {}. You avoid the encounter.",
            encounter.enemy_name, encounter.armor_label, encounter.weapon_name
        )];
        run_view.last_outcome = None;
        run_view.run_state.encounter_index = run_view.run_state.encounter_index.saturating_add(1);
        run_view.awaiting_downtime_choice = true;
        run_view.last_action = None;
        self.autosave_run_checkpoint("post-fight");
    }

    fn prepare_forced_fight_from_event(
        &mut self,
        tier: crate::core::gameplay::EncounterTier,
    ) -> bool {
        let Some(run_view) = self.run_state.as_mut() else {
            return false;
        };
        if run_view.run_over
            || run_view.live_fight.is_some()
            || run_view.pending_encounter.is_some()
            || run_view.pending_event.is_some()
            || run_view.pending_treasure.is_some()
            || run_view.pending_merchant.is_some()
        {
            return false;
        }
        let player_level = run_view.run_state.player.level;
        let effective_level = scaled_enemy_level(player_level, run_view.run_state.run_depth)
            .saturating_add(match tier {
                crate::core::gameplay::EncounterTier::Normal => 0,
                crate::core::gameplay::EncounterTier::Elite => 1,
                crate::core::gameplay::EncounterTier::Boss => 2,
            });
        let encounter_index = run_view.run_state.encounter_index as u64;
        let spawn_seed = derive_seed(
            run_view.run_state.run_seed,
            "event-forced-spawn",
            encounter_index,
        );
        let mut spawn_rng = SimRng::from_seed(spawn_seed);
        let Some(enemy_profile) = self
            .enemy_spawner
            .spawn_for_level(effective_level, &mut spawn_rng)
        else {
            return false;
        };
        let default_weapon_name = self
            .weapon_catalog
            .get(self.enemy_weapon_id)
            .map(|w| w.name.clone())
            .unwrap_or_else(|| "weapon".to_string());
        let (enemy_name, weapon_name, armor_label) = self
            .npc_presets
            .get(enemy_profile.preset_id)
            .map(|preset| {
                (
                    preset.name.clone(),
                    default_weapon_name.clone(),
                    format!("armor (DR {})", preset.armor_dr),
                )
            })
            .unwrap_or_else(|| {
                (
                    "Hobgoblin".to_string(),
                    default_weapon_name,
                    "armor".to_string(),
                )
            });
        run_view.pending_encounter = Some(EncounterPreview {
            enemy: enemy_profile,
            tier,
            enemy_name,
            armor_label,
            weapon_name,
        });
        let fight_seed = derive_seed(run_view.run_state.run_seed, "combat", encounter_index);
        run_view.seed_context.spawn_seed = Some(spawn_seed);
        run_view.seed_context.combat_seed = Some(fight_seed);
        true
    }

    pub fn resolve_pending_event_choice(&mut self, choice_id: &str) {
        let mut should_start_forced_fight = false;
        let mut forced_tier = crate::core::gameplay::EncounterTier::Normal;
        {
            let Some(run_view) = self.run_state.as_mut() else {
                return;
            };
            if run_view.run_over || run_view.awaiting_downtime_choice {
                return;
            }
            let Some(event) = run_view.pending_event.take() else {
                return;
            };
            let previous_level = run_view.run_state.player.level;
            let mut rng = SimRng::from_seed(event.resolve_seed);
            let resolution = resolve_event_choice(
                &mut run_view.run_state,
                &event.event,
                Some(choice_id),
                &mut rng,
            );
            let _ =
                crate::core::gameplay::apply_xp(&mut run_view.run_state.player, &self.xp_curve, 0);
            let levels_gained = run_view
                .run_state
                .player
                .level
                .saturating_sub(previous_level);
            run_view.run_state.last_encounter_tier = event.tier;
            run_view.run_state.last_encounter_band =
                depth_band_for_depth(run_view.run_state.run_depth);
            run_view.last_log = resolution.lines.clone();
            run_view.last_outcome = None;
            run_view.last_action = None;
            run_view.pending_levelup = if levels_gained > 0 {
                Some(crate::autobattler::state::LevelUpCheckpoint::new(
                    levels_gained,
                ))
            } else {
                None
            };
            let trigger_fight = resolution.trigger_fight && run_view.pending_levelup.is_none();
            if trigger_fight {
                run_view.awaiting_downtime_choice = false;
                forced_tier = event.tier;
                should_start_forced_fight = true;
            } else {
                run_view.run_state.encounter_index =
                    run_view.run_state.encounter_index.saturating_add(1);
                run_view.awaiting_downtime_choice = run_view.pending_levelup.is_none();
            }
            run_view.downtime_feedback = Some(DowntimeFeedback {
                title: format!("Event: {}", event.event.name),
                activity: None,
                lines: resolution.lines,
                animation_seconds: 1.2,
            });
        }
        if should_start_forced_fight {
            let _ = self.prepare_forced_fight_from_event(forced_tier);
        }
        self.autosave_run_checkpoint("post-fight");
    }

    pub fn ignore_pending_event(&mut self) {
        let Some(run_view) = self.run_state.as_mut() else {
            return;
        };
        if run_view.run_over || run_view.awaiting_downtime_choice {
            return;
        }
        let Some(event) = run_view.pending_event.take() else {
            return;
        };
        run_view.run_state.last_encounter_tier = event.tier;
        run_view.run_state.last_encounter_band = depth_band_for_depth(run_view.run_state.run_depth);
        run_view.run_state.encounter_index = run_view.run_state.encounter_index.saturating_add(1);
        run_view.last_log = vec![format!("You avoid the {} event.", event.event.name)];
        run_view.last_outcome = None;
        run_view.last_action = None;
        run_view.pending_levelup = None;
        run_view.awaiting_downtime_choice = true;
        run_view.downtime_feedback = Some(DowntimeFeedback {
            title: format!("Event: {}", event.event.name),
            activity: None,
            lines: vec!["You move on without interacting.".to_string()],
            animation_seconds: 0.8,
        });
        self.autosave_run_checkpoint("post-fight");
    }

    pub fn complete_live_fight(&mut self) {
        let Some(run_view) = self.run_state.as_mut() else {
            return;
        };
        let Some(live) = run_view.live_fight.take() else {
            return;
        };

        run_view.last_log = live
            .sim
            .combat_events
            .iter()
            .map(|event| sim::format_combat_event_line(event, &live.sim.combatants))
            .collect();
        let player_hp = live.sim.combatants[0].state.hp;
        let enemy_hp = live.sim.combatants[1].state.hp;
        let won = live.sim.done && player_hp > 0 && enemy_hp <= 0;
        let fight = FightResult {
            won,
            remaining_hp: player_hp,
            turns: live.sim.elapsed_seconds,
            events: live.sim.combat_events.clone(),
        };
        let previous_level = run_view.run_state.player.level;

        let outcome = apply_fight_result(
            run_view.run_state.clone(),
            Some(live.enemy),
            fight,
            &self.loot_table,
            Some(&self.xp_curve),
            0,
            false,
            live.tier,
        );

        run_view.run_state = outcome.state.clone();
        run_view.last_outcome = Some(outcome);
        run_view.downtime_feedback = None;
        let weapon_xp_seed = derive_seed(
            run_view.run_state.run_seed,
            "weapon-xp",
            run_view.run_state.encounter_index.saturating_sub(1) as u64,
        );
        let mut weapon_xp_rng = SimRng::from_seed(weapon_xp_seed);
        let weapon_xp_lines = weapon_mastery::apply_weapon_experience_from_fight(
            &mut run_view.run_state.player,
            &self.creation.player,
            &self.weapon_catalog,
            &self.shield_catalog,
            &live.sim.combat_events,
            live.enemy.level,
            &mut weapon_xp_rng,
        );
        run_view.last_log.extend(weapon_xp_lines);
        let levels_gained = run_view
            .run_state
            .player
            .level
            .saturating_sub(previous_level);
        run_view.pending_levelup = if levels_gained > 0 {
            Some(crate::autobattler::state::LevelUpCheckpoint::new(
                levels_gained,
            ))
        } else {
            None
        };
        let won_fight = run_view
            .last_outcome
            .as_ref()
            .map(|outcome| outcome.fight.won)
            .unwrap_or(false);
        run_view.victory = won_fight
            && matches!(live.tier, crate::core::gameplay::EncounterTier::Boss)
            && run_view.run_state.run_depth > self.autobattler_config.fights_to_run;
        if run_view.victory {
            run_view.last_log.push(format!(
                "Run complete at depth {}. The boss is down.",
                self.autobattler_config.fights_to_run
            ));
            run_view.pending_levelup = None;
        }
        run_view.run_over = !won_fight || run_view.victory;
        run_view.awaiting_downtime_choice =
            !run_view.run_over && run_view.pending_levelup.is_none();
        run_view.seed_context.loot_seed = Some(derive_seed(
            run_view.run_state.run_seed,
            "loot",
            run_view.run_state.encounter_index.saturating_sub(1) as u64,
        ));
        self.autosave_run_checkpoint("post-fight");
    }

    pub fn start_new_character(&mut self) {
        self.reset_creation();
        self.creation_step = CreationStep::Points;
        self.creation_done = false;
        self.screen = AppScreen::Creation;
    }

    pub fn start_run_from_selected_quick_start(&mut self) {
        let Some(index) = self.selected_quick_start else {
            self.quick_start_status = Some("Select a quick start preset first.".to_string());
            return;
        };
        let Some(preset_id) = self.quick_start_presets.id_from_index(index) else {
            self.quick_start_status = Some("Quick start selection is out of date.".to_string());
            return;
        };
        let Some(preset) = self.quick_start_presets.get(preset_id).cloned() else {
            self.quick_start_status = Some("Quick start preset no longer exists.".to_string());
            return;
        };

        self.apply_quick_start_preset(&preset);
        self.quick_start_status = Some(format!("Starting run with {}.", preset.name));
        self.start_run_from_creation();
    }

    pub fn start_fast_run(&mut self) {
        let preset_index = self.selected_quick_start.unwrap_or(0);
        let Some(preset_id) = self.quick_start_presets.id_from_index(preset_index) else {
            self.fast_start_status = Some("No quick-start archetype is available.".to_string());
            return;
        };
        let Some(preset) = self.quick_start_presets.get(preset_id).cloned() else {
            self.fast_start_status = Some("The selected archetype is unavailable.".to_string());
            return;
        };
        self.apply_quick_start_preset(&preset);
        let custom_name = self.fast_start_name.trim();
        if !custom_name.is_empty() {
            self.creation.name = custom_name.to_string();
            self.creation.player.name = custom_name.to_string();
        }
        self.creation.player.weapon_id = self.fast_start_weapon;
        if let Some(weapon) = self.weapon_catalog.get(self.creation.player.weapon_id) {
            let shield_allowed = self
                .shield_catalog
                .get(self.creation.player.shield_id)
                .and_then(|entry| entry.shield.as_ref())
                .map(|shield| {
                    game_logic::shield_option_allowed(
                        &self.creation.player,
                        weapon,
                        Some(shield),
                        &self.talent_catalog,
                        &self.weapon_catalog,
                    )
                })
                .unwrap_or(true);
            if !shield_allowed {
                self.creation.player.shield_id = ShieldId::new(0);
            }
        }
        self.creation.sync_player_from_stats();
        self.fast_start_status = Some(format!(
            "Fast run ready: {} with {}.",
            self.creation.name,
            self.weapon_catalog
                .get(self.creation.player.weapon_id)
                .map(|weapon| weapon.name.as_str())
                .unwrap_or("Unknown weapon")
        ));
        self.start_run_from_creation();
    }

    pub fn claim_treasure(&mut self, index: usize) {
        let selected = self
            .run_state
            .as_ref()
            .and_then(|run_view| run_view.pending_treasure.as_ref())
            .and_then(|preview| preview.options.get(index).cloned());
        let Some(choice) = selected else {
            return;
        };
        let Some(run_view) = self.run_state.as_mut() else {
            return;
        };
        if run_view.run_over || run_view.live_fight.is_some() {
            return;
        }
        let title = run_view
            .pending_treasure
            .as_ref()
            .map(|preview| preview.title.clone())
            .unwrap_or_else(|| "Treasure".to_string());
        run_view.pending_treasure = None;
        run_view.run_state.inventory.add_gear(choice.gear.clone());
        run_view.run_state.encounter_index = run_view.run_state.encounter_index.saturating_add(1);
        run_view.last_outcome = None;
        run_view.last_action = None;
        run_view.last_log = vec![format!("Recovered {}.", choice.gear.label())];
        run_view.awaiting_downtime_choice = true;
        run_view.downtime_feedback = Some(DowntimeFeedback {
            title,
            activity: None,
            lines: vec![
                format!("Added {} to your stash.", choice.gear.label()),
                choice.blurb,
            ],
            animation_seconds: 0.9,
        });
        self.autosave_run_checkpoint("post-fight");
    }

    pub fn skip_treasure(&mut self) {
        let Some(run_view) = self.run_state.as_mut() else {
            return;
        };
        if run_view.run_over || run_view.live_fight.is_some() {
            return;
        }
        let Some(preview) = run_view.pending_treasure.take() else {
            return;
        };
        run_view.run_state.encounter_index = run_view.run_state.encounter_index.saturating_add(1);
        run_view.last_outcome = None;
        run_view.last_action = None;
        run_view.last_log = vec!["You leave the cache untouched.".to_string()];
        run_view.awaiting_downtime_choice = true;
        run_view.downtime_feedback = Some(DowntimeFeedback {
            title: preview.title,
            activity: None,
            lines: vec!["You move on without taking any gear.".to_string()],
            animation_seconds: 0.7,
        });
        self.autosave_run_checkpoint("post-fight");
    }

    pub fn buy_merchant_offer(&mut self, index: usize) {
        let selected = self
            .run_state
            .as_ref()
            .and_then(|run_view| run_view.pending_merchant.as_ref())
            .and_then(|preview| preview.stock.get(index).cloned());
        let Some(choice) = selected else {
            return;
        };
        let Some(run_view) = self.run_state.as_mut() else {
            return;
        };
        if run_view.run_over || run_view.live_fight.is_some() {
            return;
        }
        if run_view.run_state.inventory.gold < choice.price_gp {
            run_view.last_log = vec![format!(
                "Not enough gold for {} (need {}, have {}).",
                choice.gear.label(),
                choice.price_gp,
                run_view.run_state.inventory.gold
            )];
            return;
        }
        let title = run_view
            .pending_merchant
            .as_ref()
            .map(|preview| preview.title.clone())
            .unwrap_or_else(|| "Merchant".to_string());
        run_view.pending_merchant = None;
        run_view.run_state.inventory.gold =
            run_view.run_state.inventory.gold.saturating_sub(choice.price_gp);
        run_view.run_state.inventory.add_gear(choice.gear.clone());
        run_view.run_state.encounter_index = run_view.run_state.encounter_index.saturating_add(1);
        run_view.last_outcome = None;
        run_view.last_action = None;
        run_view.last_log = vec![format!(
            "Bought {} for {} gp.",
            choice.gear.label(),
            choice.price_gp
        )];
        run_view.awaiting_downtime_choice = true;
        run_view.downtime_feedback = Some(DowntimeFeedback {
            title,
            activity: None,
            lines: vec![
                format!("Purchased {}.", choice.gear.label()),
                choice.blurb,
            ],
            animation_seconds: 0.9,
        });
        self.autosave_run_checkpoint("post-fight");
    }

    pub fn leave_merchant(&mut self) {
        let Some(run_view) = self.run_state.as_mut() else {
            return;
        };
        if run_view.run_over || run_view.live_fight.is_some() {
            return;
        }
        let Some(preview) = run_view.pending_merchant.take() else {
            return;
        };
        run_view.run_state.encounter_index = run_view.run_state.encounter_index.saturating_add(1);
        run_view.last_outcome = None;
        run_view.last_action = None;
        run_view.last_log = vec!["You leave the merchant and keep your gold.".to_string()];
        run_view.awaiting_downtime_choice = true;
        run_view.downtime_feedback = Some(DowntimeFeedback {
            title: preview.title,
            activity: None,
            lines: vec!["No purchase made.".to_string()],
            animation_seconds: 0.7,
        });
        self.autosave_run_checkpoint("post-fight");
    }

    pub fn equip_stash_gear(&mut self, stash_index: usize) {
        let selected = self
            .run_state
            .as_ref()
            .and_then(|run_view| run_view.run_state.inventory.stash.get(stash_index).cloned());
        let Some(gear) = selected else {
            return;
        };
        if matches!(gear.slot, EquipmentSlot::Shield) {
            let weapon = self
                .run_state
                .as_ref()
                .and_then(|run_view| run_view.run_state.inventory.loadout.weapon.clone());
            if let Some(weapon) = weapon.as_ref() {
                if !self.shield_gear_allowed_with_weapon(weapon, &gear) {
                    if let Some(run_view) = self.run_state.as_mut() {
                        run_view.last_log = vec![format!(
                            "{} cannot be used with your current weapon.",
                            gear.label()
                        )];
                    }
                    return;
                }
            }
        }
        let drop_shield = if matches!(gear.slot, EquipmentSlot::Weapon) {
            self.run_state
                .as_ref()
                .and_then(|run_view| run_view.run_state.inventory.loadout.shield.clone())
                .map(|shield| !self.shield_gear_allowed_with_weapon(&gear, &shield))
                .unwrap_or(false)
        } else {
            false
        };
        let Some(run_view) = self.run_state.as_mut() else {
            return;
        };
        if run_view.live_fight.is_some() {
            return;
        }
        let selected_gear = run_view.run_state.inventory.stash.remove(stash_index);
        let mut lines = vec![format!(
            "Equipped {} ({})",
            selected_gear.label(),
            selected_gear.slot.label()
        )];
        match selected_gear.slot {
            EquipmentSlot::Weapon => {
                if let Some(previous) = run_view.run_state.inventory.loadout.weapon.replace(selected_gear)
                {
                    run_view.run_state.inventory.stash.push(previous);
                }
                if drop_shield {
                    if let Some(previous_shield) = run_view.run_state.inventory.loadout.shield.take()
                    {
                        lines.push(format!(
                            "Moved {} to the stash because the new weapon needs both hands.",
                            previous_shield.label()
                        ));
                        run_view.run_state.inventory.stash.push(previous_shield);
                    }
                }
            }
            EquipmentSlot::Armor => {
                if let Some(previous) = run_view.run_state.inventory.loadout.armor.replace(selected_gear)
                {
                    run_view.run_state.inventory.stash.push(previous);
                }
            }
            EquipmentSlot::Shield => {
                if let Some(previous) = run_view.run_state.inventory.loadout.shield.replace(selected_gear)
                {
                    run_view.run_state.inventory.stash.push(previous);
                }
            }
        }
        run_view.last_log = lines.clone();
        run_view.downtime_feedback = Some(DowntimeFeedback {
            title: "Loadout Updated".to_string(),
            activity: None,
            lines,
            animation_seconds: 0.6,
        });
    }

    fn apply_quick_start_preset(&mut self, preset: &FighterPreset) {
        let mut creation = CreationState::new(self.creation.player.weapon_id, self.run_seed);
        creation.name = preset.name.clone();
        creation.player.name = preset.name.clone();
        creation.player.level = preset.level.max(1);
        creation.player.progression = Progression::new(
            tier_from_label(&preset.progression.attack),
            tier_from_label(&preset.progression.speed),
            tier_from_label(&preset.progression.initiative),
            tier_from_label(&preset.progression.health),
        );
        creation.player.mastery_attack = game_logic::clamp_mastery(preset.masteries.attack);
        creation.player.mastery_defense = game_logic::clamp_mastery(preset.masteries.defense);
        creation.player.mastery_damage = game_logic::clamp_mastery(preset.masteries.damage);
        creation.player.mastery_speed = game_logic::clamp_mastery(preset.masteries.speed);
        creation.player.shield_mastery_defense =
            game_logic::clamp_mastery(preset.masteries.shield_defense);
        creation.player.shield_mastery_speed =
            game_logic::clamp_mastery(preset.masteries.shield_speed);
        creation.player.base_hp = preset.base_hp.max(1);
        creation.player.move_speed = preset.move_speed;
        creation.player.strength_base = preset.strength_base;
        creation.player.strength_pct = game_logic::normalize_percentile(preset.strength_pct);
        creation.player.dex_base = preset.dex_base;
        creation.player.dex_pct = game_logic::normalize_percentile(preset.dex_pct);
        creation.player.intelligence = preset.intelligence;
        creation.player.wisdom = preset.wisdom;
        creation.player.constitution = preset.constitution;
        creation.player.looks = preset.looks;
        creation.player.charisma = preset.charisma;
        creation.player.weapon_material_tier = preset.weapon_material_tier;
        creation.player.offhand_weapon_material_tier = preset.offhand_weapon_material_tier;
        creation.player.armor_material_tier = preset.armor_material_tier;
        creation.player.projectile_material_tier = preset.projectile_material_tier;
        creation.player.offhand_projectile_material_tier = preset.offhand_projectile_material_tier;
        creation.player.shield_material_tier = preset.shield_material_tier;
        creation.player.two_hand_grip = preset.two_hand_grip;
        creation.proficiencies = preset.proficiencies.clone();
        creation.player.proficiencies = preset.proficiencies.clone();
        creation.player.talents = preset.talents.clone();
        creation.player.race_id = preset.race_id.clone();
        creation.player.race_applied = preset.race_id.is_some();
        creation.player.knockback_step =
            game_logic::knockback_step_for_race_id(preset.race_id.as_deref(), &self.race_catalog);
        creation.player.use_jab = preset.maneuvers.use_jab;
        creation.player.hold_at_bay = preset.maneuvers.hold_at_bay;
        creation.player.called_shot = preset.maneuvers.called_shot;
        creation.player.aggressive_attack = preset.maneuvers.aggressive_attack;
        creation.player.charge = preset.maneuvers.charge;
        creation.player.ready_against_charge = preset.maneuvers.ready_against_charge;
        creation.player.tactical_move = preset.maneuvers.tactical_move;
        creation.player.fight_defensively = preset.maneuvers.fight_defensively;
        creation.player.fight_defensively_penalty = preset.maneuvers.fight_defensively_penalty;
        creation.player.full_parry = preset.maneuvers.full_parry;
        creation.player.give_ground = preset.maneuvers.give_ground;
        creation.player.scamper_back = preset.maneuvers.scamper_back;
        creation.player.fighting_withdrawal = preset.maneuvers.fighting_withdrawal;
        creation.player.flee = preset.maneuvers.flee;
        creation.player.mounted = preset.maneuvers.mounted;
        creation.player.defensive_dualwielding = preset.defensive_dualwielding;
        creation.player.offensive_dualwielding = preset.offensive_dualwielding;
        creation.player.environment = game_logic::EnvironmentConfig::default();
        creation.player.misc_modifiers = game_logic::MiscRollModifiers::default();

        if let Some(id) = find_weapon_id_by_name(&self.weapon_catalog, &preset.weapon) {
            creation.player.weapon_id = id;
        } else if let Some(id) = self.weapon_catalog.first_id() {
            creation.player.weapon_id = id;
        }
        if let Some(id) = find_armor_id_by_label(&self.armor_catalog, &preset.armor) {
            creation.player.armor_id = id;
        } else if let Some(id) = self.armor_catalog.first_id() {
            creation.player.armor_id = id;
        }
        if let Some(id) = find_shield_id_by_name(&self.shield_catalog, &preset.shield) {
            creation.player.shield_id = id;
        } else if let Some(id) = self.shield_catalog.first_id() {
            creation.player.shield_id = id;
        }
        creation.race_index = preset
            .race_id
            .as_ref()
            .and_then(|id| self.race_catalog.iter().position(|race| race.id == *id));
        creation.race_applied = creation.player.race_applied;

        creation.stats[0] = AbilityScore::new(
            preset.strength_base,
            game_logic::normalize_percentile(preset.strength_pct),
        );
        creation.stats[1] = AbilityScore::new(preset.intelligence.max(1), 1);
        creation.stats[2] = AbilityScore::new(preset.wisdom.max(1), 1);
        creation.stats[3] = AbilityScore::new(
            preset.dex_base,
            game_logic::normalize_percentile(preset.dex_pct),
        );
        creation.stats[4] = AbilityScore::new(preset.constitution.max(1), 1);
        creation.stats[5] = AbilityScore::new(preset.looks.max(1), 1);
        creation.stats[6] = AbilityScore::new(preset.charisma.max(1), 1);
        creation.stats_locked = true;
        creation.money_rolled = false;
        creation.starting_money = 0;
        creation.sync_player_from_stats();
        creation.player.charisma = preset.charisma;
        creation.player.level = preset.level.max(1);
        creation.player.progression = Progression::new(
            tier_from_label(&preset.progression.attack),
            tier_from_label(&preset.progression.speed),
            tier_from_label(&preset.progression.initiative),
            tier_from_label(&preset.progression.health),
        );
        creation.player.base_hp = preset.base_hp.max(1);
        creation.player.proficiencies = preset.proficiencies.clone();
        creation.player.talents = preset.talents.clone();
        creation.player.race_id = preset.race_id.clone();
        creation.player.race_applied = preset.race_id.is_some();

        self.creation = creation;
        self.creation_step = CreationStep::MoneyGear;
        self.creation_done = true;
    }

    fn starting_loadout_from_creation(&self) -> EquipmentLoadout {
        let weapon = self
            .weapon_catalog
            .get(self.creation.player.weapon_id)
            .map(|entry| InventoryGear {
                slot: EquipmentSlot::Weapon,
                name: entry.name.clone(),
                material_tier: self.creation.player.weapon_material_tier,
                value_gp: gear_value_for_price(entry.price_gp, self.creation.player.weapon_material_tier),
            });
        let armor = self
            .armor_catalog
            .get(self.creation.player.armor_id)
            .and_then(|entry| entry.armor.as_ref().map(|armor| (entry.label.clone(), armor.price_gp)))
            .map(|(label, price_gp)| InventoryGear {
                slot: EquipmentSlot::Armor,
                name: label,
                material_tier: self.creation.player.armor_material_tier,
                value_gp: gear_value_for_price(price_gp, self.creation.player.armor_material_tier),
            });
        let shield = self
            .shield_catalog
            .get(self.creation.player.shield_id)
            .and_then(|entry| entry.shield.as_ref().map(|shield| (entry.label.clone(), shield.price_gp)))
            .map(|(label, price_gp)| InventoryGear {
                slot: EquipmentSlot::Shield,
                name: label,
                material_tier: self.creation.player.shield_material_tier,
                value_gp: gear_value_for_price(price_gp, self.creation.player.shield_material_tier),
            });
        EquipmentLoadout {
            weapon,
            armor,
            shield,
        }
    }

    fn build_treasure_preview(&self, state: &RunState, seed: u64) -> TreasurePreview {
        let mut rng = SimRng::from_seed(seed);
        TreasurePreview {
            title: "Treasure Cache".to_string(),
            options: vec![
                GearOfferPreview {
                    gear: self.sample_weapon_gear(state, &mut rng),
                    price_gp: 0,
                    blurb: "A weapon upgrade or sidegrade for the next fights.".to_string(),
                },
                GearOfferPreview {
                    gear: self.sample_armor_gear(state, &mut rng),
                    price_gp: 0,
                    blurb: "A sturdier set of armor to blunt incoming damage.".to_string(),
                },
                GearOfferPreview {
                    gear: self.sample_shield_gear(state, &mut rng),
                    price_gp: 0,
                    blurb: "Defensive gear for tighter melee exchanges.".to_string(),
                },
            ],
        }
    }

    fn build_merchant_preview(&self, state: &RunState, seed: u64) -> MerchantPreview {
        let mut rng = SimRng::from_seed(seed);
        let price_markup = state.run_depth.saturating_mul(5);
        let with_price = |gear: InventoryGear, blurb: &str| GearOfferPreview {
            price_gp: gear.value_gp.saturating_mul(6) / 5 + price_markup,
            gear,
            blurb: blurb.to_string(),
        };
        MerchantPreview {
            title: "Wandering Merchant".to_string(),
            stock: vec![
                with_price(
                    self.sample_weapon_gear(state, &mut rng),
                    "Steel, weight, and reach for sale.",
                ),
                with_price(
                    self.sample_armor_gear(state, &mut rng),
                    "A tougher shell for the next room.",
                ),
                with_price(
                    self.sample_shield_gear(state, &mut rng),
                    "Guarding gear for a more defensive line.",
                ),
            ],
        }
    }

    fn sample_weapon_gear(&self, state: &RunState, rng: &mut SimRng) -> InventoryGear {
        let current = state.inventory.loadout.weapon.as_ref();
        let max_tier = max_run_material_tier(state.run_depth);
        let current_tier = current.map(|gear| gear.material_tier.max(0)).unwrap_or(0);
        if let Some(current) = current {
            if current_tier < max_tier && rng.gen_range(0..100) < 55 {
                let next_tier = (current_tier + 1).min(max_tier);
                let base_price = self
                    .weapon_catalog
                    .entries()
                    .iter()
                    .find(|entry| entry.name.eq_ignore_ascii_case(&current.name))
                    .map(|entry| entry.price_gp)
                    .unwrap_or(current.value_gp);
                return InventoryGear {
                    slot: EquipmentSlot::Weapon,
                    name: current.name.clone(),
                    material_tier: next_tier,
                    value_gp: gear_value_for_price(base_price, next_tier),
                };
            }
        }
        let candidates: Vec<_> = self
            .weapon_catalog
            .entries()
            .iter()
            .filter(|weapon| !weapon.name.eq_ignore_ascii_case("Unarmed"))
            .collect();
        let weapon = candidates
            .get(rng.gen_range(0..candidates.len()))
            .copied()
            .unwrap_or_else(|| &self.weapon_catalog.entries()[0]);
        InventoryGear {
            slot: EquipmentSlot::Weapon,
            name: weapon.name.clone(),
            material_tier: max_tier.max(current_tier),
            value_gp: gear_value_for_price(weapon.price_gp, max_tier.max(current_tier)),
        }
    }

    fn sample_armor_gear(&self, state: &RunState, rng: &mut SimRng) -> InventoryGear {
        let current = state.inventory.loadout.armor.as_ref();
        let max_tier = max_run_material_tier(state.run_depth);
        let current_tier = current.map(|gear| gear.material_tier.max(0)).unwrap_or(0);
        if let Some(current) = current {
            if current_tier < max_tier && rng.gen_range(0..100) < 60 {
                let next_tier = (current_tier + 1).min(max_tier);
                let base_price = self
                    .armor_catalog
                    .entries()
                    .iter()
                    .find(|entry| entry.label.eq_ignore_ascii_case(&current.name))
                    .and_then(|entry| entry.armor.as_ref())
                    .map(|entry| entry.price_gp)
                    .unwrap_or(current.value_gp);
                return InventoryGear {
                    slot: EquipmentSlot::Armor,
                    name: current.name.clone(),
                    material_tier: next_tier,
                    value_gp: gear_value_for_price(base_price, next_tier),
                };
            }
        }
        let candidates: Vec<_> = self
            .armor_catalog
            .entries()
            .iter()
            .filter(|entry| entry.armor.is_some())
            .collect();
        let armor = candidates
            .get(rng.gen_range(0..candidates.len()))
            .copied()
            .unwrap_or_else(|| &self.armor_catalog.entries()[0]);
        let price_gp = armor.armor.as_ref().map(|entry| entry.price_gp).unwrap_or(0);
        InventoryGear {
            slot: EquipmentSlot::Armor,
            name: armor.label.clone(),
            material_tier: max_tier.max(current_tier),
            value_gp: gear_value_for_price(price_gp, max_tier.max(current_tier)),
        }
    }

    fn sample_shield_gear(&self, state: &RunState, rng: &mut SimRng) -> InventoryGear {
        let current = state.inventory.loadout.shield.as_ref();
        let max_tier = max_run_material_tier(state.run_depth);
        let current_tier = current.map(|gear| gear.material_tier.max(0)).unwrap_or(0);
        if let Some(current) = current {
            if current_tier < max_tier && rng.gen_range(0..100) < 50 {
                let next_tier = (current_tier + 1).min(max_tier);
                let base_price = self
                    .shield_catalog
                    .entries()
                    .iter()
                    .find(|entry| entry.label.eq_ignore_ascii_case(&current.name))
                    .and_then(|entry| entry.shield.as_ref())
                    .map(|entry| entry.price_gp)
                    .unwrap_or(current.value_gp);
                return InventoryGear {
                    slot: EquipmentSlot::Shield,
                    name: current.name.clone(),
                    material_tier: next_tier,
                    value_gp: gear_value_for_price(base_price, next_tier),
                };
            }
        }
        let candidates: Vec<_> = self
            .shield_catalog
            .entries()
            .iter()
            .filter(|entry| entry.shield.is_some())
            .collect();
        let shield = candidates
            .get(rng.gen_range(0..candidates.len()))
            .copied()
            .unwrap_or_else(|| &self.shield_catalog.entries()[0]);
        let price_gp = shield.shield.as_ref().map(|entry| entry.price_gp).unwrap_or(0);
        InventoryGear {
            slot: EquipmentSlot::Shield,
            name: shield.label.clone(),
            material_tier: max_tier.max(current_tier),
            value_gp: gear_value_for_price(price_gp, max_tier.max(current_tier)),
        }
    }

    fn shield_gear_allowed_with_weapon(
        &self,
        weapon: &InventoryGear,
        shield: &InventoryGear,
    ) -> bool {
        let Some(weapon_entry) = self
            .weapon_catalog
            .entries()
            .iter()
            .find(|entry| entry.name.eq_ignore_ascii_case(&weapon.name))
        else {
            return true;
        };
        let Some(shield_entry) = self
            .shield_catalog
            .entries()
            .iter()
            .find(|entry| entry.label.eq_ignore_ascii_case(&shield.name))
            .and_then(|entry| entry.shield.as_ref())
        else {
            return true;
        };
        game_logic::shield_option_allowed(
            &self.creation.player,
            weapon_entry,
            Some(shield_entry),
            &self.talent_catalog,
            &self.weapon_catalog,
        )
    }

    pub fn upcoming_room_sequence(&self, count: usize) -> Vec<RunRoomKind> {
        let Some(run_view) = self.run_state.as_ref() else {
            return Vec::new();
        };
        let mut rooms = Vec::new();
        let mut depth = run_view.run_state.run_depth;
        let mut encounter_index = run_view.run_state.encounter_index;
        while rooms.len() < count {
            let preview_state = RunState {
                player: run_view.run_state.player.clone(),
                inventory: run_view.run_state.inventory.clone(),
                run_depth: depth,
                run_seed: run_view.run_state.run_seed,
                encounter_index,
                last_encounter_tier: run_view.run_state.last_encounter_tier,
                last_encounter_band: run_view.run_state.last_encounter_band,
                event_flags: run_view.run_state.event_flags.clone(),
                seen_event_ids: run_view.run_state.seen_event_ids.clone(),
                wounds: run_view.run_state.wounds.clone(),
            };
            let room = next_room_kind(&preview_state, &self.autobattler_config);
            rooms.push(room);
            encounter_index = encounter_index.saturating_add(1);
            if matches!(room, RunRoomKind::Encounter | RunRoomKind::Boss) {
                depth = depth.saturating_add(1);
            }
            if matches!(room, RunRoomKind::Boss) {
                break;
            }
        }
        rooms
    }
}

fn next_room_kind(state: &RunState, config: &AutobattlerConfig) -> RunRoomKind {
    if state.run_depth >= config.fights_to_run.max(1) {
        return RunRoomKind::Boss;
    }
    if state.encounter_index > 0 && state.encounter_index % 5 == 0 {
        return RunRoomKind::Merchant;
    }
    let seed = derive_seed(state.run_seed, "room-kind", state.encounter_index as u64);
    let mut rng = SimRng::from_seed(seed);
    match rng.gen_range(0..100) {
        0..=17 => RunRoomKind::Treasure,
        18..=37 => RunRoomKind::Event,
        _ => RunRoomKind::Encounter,
    }
}

fn max_run_material_tier(run_depth: u32) -> i32 {
    ((run_depth + 1) / 2).clamp(0, 3) as i32
}

fn gear_value_for_price(base_price: u32, material_tier: i32) -> u32 {
    let tier = material_tier.max(0) as u32;
    base_price
        .saturating_add(tier.saturating_mul(base_price / 3 + 8))
        .max(base_price)
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
    fn build_player(&self, state: &RunState) -> crate::core::sim::Combatant {
        let mut player = self.player_base.clone();
        player.name = state.player.name.clone();
        player.level = state.player.level;
        player.progression = state.player.progression;
        player.strength_base = state.player.base_stats.strength.base;
        player.strength_pct = state.player.base_stats.strength.percentile;
        player.dex_base = state.player.base_stats.dexterity.base;
        player.dex_pct = state.player.base_stats.dexterity.percentile;
        player.intelligence = state.player.base_stats.intelligence;
        player.wisdom = state.player.base_stats.wisdom;
        player.constitution = state.player.base_stats.constitution;
        player.looks = state.player.base_stats.looks;
        player.charisma = state.player.base_stats.charisma;
        player.race_id = state.player.race_id.clone();
        player.race_applied = player.race_id.is_some();
        player.proficiencies = state.player.proficiencies.clone();
        player.talents = state.player.talents.clone();
        if let Some(weapon) = state.inventory.loadout.weapon.as_ref() {
            if let Some(id) = find_weapon_id_by_name(self.weapon_catalog, &weapon.name) {
                player.weapon_id = id;
                player.weapon_material_tier = weapon.material_tier;
            }
        }
        if let Some(armor) = state.inventory.loadout.armor.as_ref() {
            if let Some(id) = find_armor_id_by_label(self.armor_catalog, &armor.name) {
                player.armor_id = id;
                player.armor_material_tier = armor.material_tier;
            }
        }
        if let Some(shield) = state.inventory.loadout.shield.as_ref() {
            if let Some(id) = find_shield_id_by_name(self.shield_catalog, &shield.name) {
                player.shield_id = id;
                player.shield_material_tier = shield.material_tier;
            }
        }
        weapon_mastery::apply_profile_masteries_to_config(
            &state.player,
            &mut player,
            self.weapon_catalog,
            self.shield_catalog,
        );
        player.charge = true;
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

    fn build_enemy(&self, enemy: &EnemyProfile) -> crate::core::sim::Combatant {
        let mut npc = PlayerConfig::new("Hobgoblin", self.enemy_weapon_id);
        npc.level = enemy.level;
        npc.npc_preset = Some(enemy.preset_id);
        npc.charge = true;
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

fn player_profile_from_config(
    config: &PlayerConfig,
    ability_scores_full: AbilitySetFull,
    points: PointPools,
    honor: i32,
    alignment: Option<String>,
    background: Option<String>,
    quirks: Vec<String>,
    flaws: Vec<String>,
    skill_levels: Vec<SkillProgress>,
    proficiencies: Vec<String>,
) -> PlayerProfile {
    PlayerProfile {
        name: config.name.clone(),
        level: config.level,
        xp: 0,
        base_stats: AbilitySet::from(ability_scores_full),
        ability_scores_full,
        progression: config.progression,
        points,
        banked_points: PointPools::default(),
        honor,
        alignment,
        race_id: config.race_id.clone(),
        background,
        quirks,
        flaws,
        skills: skills::legacy_skill_names(&skill_levels),
        skill_levels,
        proficiencies,
        weapon_masteries: Vec::new(),
        talents: config.talents.clone(),
    }
}

fn ability_scores_full_from_creation(creation: &CreationState) -> AbilitySetFull {
    AbilitySetFull {
        strength: creation.stats[0],
        intelligence: creation.stats[1],
        wisdom: creation.stats[2],
        dexterity: creation.stats[3],
        constitution: creation.stats[4],
        looks: creation.stats[5],
        charisma: AbilityScore::new(creation.player.charisma, creation.stats[6].percentile),
    }
}

fn hobgoblin_spawner(npc_presets: &NpcPresetCatalog) -> EnemySpawner {
    let mut spawner = EnemySpawner::default();
    for (index, preset) in npc_presets.entries().iter().enumerate() {
        if let Some(level) = logic::hobgoblin_level(&preset.name) {
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

fn find_weapon_id_by_name(catalog: &WeaponCatalog, name: &str) -> Option<WeaponId> {
    catalog
        .entries()
        .iter()
        .position(|weapon| weapon.name.eq_ignore_ascii_case(name))
        .and_then(|idx| catalog.id_from_index(idx))
}

fn find_armor_id_by_label(catalog: &ArmorCatalog, label: &str) -> Option<ArmorId> {
    catalog
        .entries()
        .iter()
        .position(|entry| {
            entry.label.eq_ignore_ascii_case(label)
                || entry
                    .armor
                    .as_ref()
                    .map(|armor| armor.name.eq_ignore_ascii_case(label))
                    .unwrap_or(false)
        })
        .and_then(|idx| catalog.id_from_index(idx))
}

fn find_shield_id_by_name(catalog: &ShieldCatalog, name: &str) -> Option<ShieldId> {
    catalog
        .entries()
        .iter()
        .position(|entry| {
            entry.label.eq_ignore_ascii_case(name)
                || entry
                    .shield
                    .as_ref()
                    .map(|shield| shield.name.eq_ignore_ascii_case(name))
                    .unwrap_or(false)
        })
        .and_then(|idx| catalog.id_from_index(idx))
}

fn tier_from_label(label: &str) -> ProgressionTier {
    match label.trim().to_ascii_uppercase().as_str() {
        "I" => ProgressionTier::I,
        "II" => ProgressionTier::II,
        "III" => ProgressionTier::III,
        "IV" => ProgressionTier::IV,
        "V" => ProgressionTier::V,
        "VI" => ProgressionTier::VI,
        _ => ProgressionTier::I,
    }
}

fn add_stat_percentile_gain(
    player: &mut PlayerProfile,
    ability: &str,
    expr: &str,
    rng: &mut SimRng,
    lines: &mut Vec<String>,
) {
    let gain = roll_damage_expr(expr, rng, false).max(0) as u8;
    apply_stat_percentile_delta(player, ability, i32::from(gain), lines);
}

fn apply_stat_percentile_delta(
    player: &mut PlayerProfile,
    ability: &str,
    delta: i32,
    lines: &mut Vec<String>,
) {
    let Some(score) = (match ability {
        "str" => Some(&mut player.ability_scores_full.strength),
        "dex" => Some(&mut player.ability_scores_full.dexterity),
        "int" => Some(&mut player.ability_scores_full.intelligence),
        "wis" => Some(&mut player.ability_scores_full.wisdom),
        "con" => Some(&mut player.ability_scores_full.constitution),
        "cha" => Some(&mut player.ability_scores_full.charisma),
        _ => None,
    }) else {
        return;
    };
    match ability {
        _ if delta >= 0 => apply_percentile(score, delta.clamp(0, u8::MAX as i32) as u8),
        _ => subtract_percentile(score, delta.saturating_abs().clamp(0, u8::MAX as i32) as u8),
    }
    player.base_stats = AbilitySet::from(player.ability_scores_full);
    if delta >= 0 {
        lines.push(format!("{ability} +{delta}p"));
    } else {
        lines.push(format!("{ability} -{}p", delta.saturating_abs()));
    }
}

fn has_skill(player: &PlayerProfile, skill: &str) -> bool {
    skills::player_skill_level(player, skill) > 0
}

fn add_skill_if_unskilled(player: &mut PlayerProfile, skill: &str, lines: &mut Vec<String>) {
    if has_skill(player, skill) {
        return;
    }
    let abilities = player.ability_scores_full;
    match skills::ensure_skill(
        &mut player.skill_levels,
        &abilities,
        player.level.max(1),
        skill,
    ) {
        Ok(progress) => {
            let tier = skills::mastery_tier_for_level(progress.level).label();
            let name = skills::skill_spec(skill)
                .map(|spec| spec.name)
                .unwrap_or(skill);
            lines.push(format!("Gained skill: {name} {}% ({tier})", progress.level));
            player.skills = skills::legacy_skill_names(&player.skill_levels);
        }
        Err(err) => {
            lines.push(format!("Could not learn {skill}: {err}"));
        }
    }
}

fn resolve_skill_check(
    player: &PlayerProfile,
    skill: &str,
    difficulty: SkillDifficulty,
    require_trained: bool,
    rng: &mut SimRng,
    lines: &mut Vec<String>,
) -> bool {
    let result: SkillCheckResult =
        skills::roll_skill_check(player, skill, difficulty, require_trained, rng);
    lines.push(result.summary_line());
    result.success
}

fn add_wound(state: &mut RunState, expr: &str, rng: &mut SimRng, lines: &mut Vec<String>) {
    let damage = roll_damage_expr(expr, rng, false).max(0) as u32;
    if damage > 0 {
        state.wounds.push(crate::core::gameplay::Wound {
            damage,
            healing_progress_steps: 0,
        });
        lines.push(format!("Suffered wound: {damage}"));
    }
}

fn apply_downtime_activity(
    state: &mut RunState,
    activity: DowntimeActivity,
    seed: u64,
) -> Vec<String> {
    let mut rng = SimRng::from_seed(seed);
    let mut lines = Vec::new();
    match activity {
        DowntimeActivity::Acrobatics => {
            add_stat_percentile_gain(&mut state.player, "dex", "d12p", &mut rng, &mut lines);
        }
        DowntimeActivity::AnimalTraining => {
            add_stat_percentile_gain(&mut state.player, "wis", "d6p", &mut rng, &mut lines);
            lines.push("Animal progress +1 week".to_string());
        }
        DowntimeActivity::Athletics => {
            add_stat_percentile_gain(&mut state.player, "str", "d6p", &mut rng, &mut lines);
            add_stat_percentile_gain(&mut state.player, "con", "d6p", &mut rng, &mut lines);
        }
        DowntimeActivity::Begging => {
            add_stat_percentile_gain(&mut state.player, "cha", "d6p", &mut rng, &mut lines);
            let success = resolve_skill_check(
                &state.player,
                "Persuasion",
                SkillDifficulty::Hard,
                false,
                &mut rng,
                &mut lines,
            );
            let amount = if success {
                roll_damage_expr("2d20p", &mut rng, false)
            } else {
                roll_damage_expr("d20p", &mut rng, false)
            };
            state.inventory.gold = state.inventory.gold.saturating_add(amount.max(0) as u32);
            lines.push(format!("Coins +{amount}"));
            add_skill_if_unskilled(&mut state.player, "Persuasion", &mut lines);
        }
        DowntimeActivity::Carousing => {
            add_stat_percentile_gain(&mut state.player, "con", "d6p", &mut rng, &mut lines);
            add_stat_percentile_gain(&mut state.player, "cha", "d6p", &mut rng, &mut lines);
            let cost = roll_damage_expr("5d6p", &mut rng, false).max(0) as u32;
            state.inventory.gold = state.inventory.gold.saturating_sub(cost);
            lines.push(format!("Coins -{cost}"));
        }
        DowntimeActivity::Climbing => {
            add_stat_percentile_gain(&mut state.player, "str", "d6p", &mut rng, &mut lines);
            add_stat_percentile_gain(&mut state.player, "dex", "d6p", &mut rng, &mut lines);
            add_skill_if_unskilled(&mut state.player, "Climbing", &mut lines);
        }
        DowntimeActivity::Crafting => {
            add_stat_percentile_gain(&mut state.player, "int", "d6p", &mut rng, &mut lines);
            let success = resolve_skill_check(
                &state.player,
                "Craft",
                SkillDifficulty::Hard,
                true,
                &mut rng,
                &mut lines,
            );
            if success {
                let gain = roll_damage_expr("2d6p", &mut rng, false).max(0) as u32;
                state.inventory.gold = state.inventory.gold.saturating_add(gain);
                lines.push(format!("Coins +{gain}"));
            } else {
                let cost = roll_damage_expr("d6p", &mut rng, false).max(0) as u32;
                state.inventory.gold = state.inventory.gold.saturating_sub(cost);
                lines.push(format!("Coins -{cost}"));
            }
        }
        DowntimeActivity::Foraging => {
            add_stat_percentile_gain(&mut state.player, "int", "d3p", &mut rng, &mut lines);
            add_stat_percentile_gain(&mut state.player, "wis", "d3p", &mut rng, &mut lines);
            let success = resolve_skill_check(
                &state.player,
                "Survival",
                SkillDifficulty::Hard,
                true,
                &mut rng,
                &mut lines,
            );
            if success {
                let gain = roll_damage_expr("2d6p", &mut rng, false).max(0) as u32;
                state.inventory.gold = state.inventory.gold.saturating_add(gain);
                lines.push(format!("Coins +{gain}"));
            } else {
                lines.push("No finds.".to_string());
            }
        }
        DowntimeActivity::Gambling => {
            let loss = roll_damage_expr("d6p", &mut rng, false);
            apply_stat_percentile_delta(&mut state.player, "wis", -loss, &mut lines);
            add_stat_percentile_gain(&mut state.player, "cha", "d12p", &mut rng, &mut lines);
            let success = resolve_skill_check(
                &state.player,
                "Gambling",
                SkillDifficulty::Hard,
                true,
                &mut rng,
                &mut lines,
            );
            if success {
                let gain = roll_damage_expr("d20p", &mut rng, false).max(0) as u32;
                state.inventory.gold = state.inventory.gold.saturating_add(gain);
                lines.push(format!("Coins +{gain}"));
            } else {
                let cost = roll_damage_expr("d20p", &mut rng, false).max(0) as u32;
                state.inventory.gold = state.inventory.gold.saturating_sub(cost);
                lines.push(format!("Coins -{cost}"));
            }
        }
        DowntimeActivity::Healing => {
            add_stat_percentile_gain(&mut state.player, "wis", "d6p", &mut rng, &mut lines);
            let success = resolve_skill_check(
                &state.player,
                "First Aid",
                SkillDifficulty::Hard,
                true,
                &mut rng,
                &mut lines,
            );
            if success {
                state.player.honor = state.player.honor.saturating_add(1);
                lines.push("Honor +1".to_string());
            } else {
                state.player.honor = state.player.honor.saturating_sub(1);
                lines.push("Honor -1".to_string());
            }
        }
        DowntimeActivity::Hunting => {
            add_stat_percentile_gain(&mut state.player, "wis", "d3p", &mut rng, &mut lines);
            add_stat_percentile_gain(&mut state.player, "dex", "d3p", &mut rng, &mut lines);
            let success = resolve_skill_check(
                &state.player,
                "Hunting",
                SkillDifficulty::Hard,
                true,
                &mut rng,
                &mut lines,
            );
            if success {
                let gain = roll_damage_expr("2d6p", &mut rng, false).max(0) as u32;
                state.inventory.gold = state.inventory.gold.saturating_add(gain);
                lines.push(format!("Coins +{gain}"));
            } else {
                add_wound(state, "d4p", &mut rng, &mut lines);
            }
        }
        DowntimeActivity::Jumping => {
            add_stat_percentile_gain(&mut state.player, "str", "d12p", &mut rng, &mut lines);
            add_skill_if_unskilled(&mut state.player, "Jumping", &mut lines);
        }
        DowntimeActivity::Laboring => {
            add_stat_percentile_gain(&mut state.player, "con", "d6p", &mut rng, &mut lines);
            state.player.honor = state.player.honor.saturating_add(1);
            lines.push("Honor +1".to_string());
        }
        DowntimeActivity::Meditating => {
            add_stat_percentile_gain(&mut state.player, "wis", "d12p", &mut rng, &mut lines);
        }
        DowntimeActivity::Performing => {
            add_stat_percentile_gain(&mut state.player, "cha", "d6p", &mut rng, &mut lines);
            let success = resolve_skill_check(
                &state.player,
                "Acting",
                SkillDifficulty::Hard,
                true,
                &mut rng,
                &mut lines,
            );
            if success {
                let gain = roll_damage_expr("d6p", &mut rng, false).max(0) as u32;
                state.inventory.gold = state.inventory.gold.saturating_add(gain);
                lines.push(format!("Coins +{gain}"));
            }
        }
        DowntimeActivity::Reading => {
            add_stat_percentile_gain(&mut state.player, "int", "d12p", &mut rng, &mut lines);
            let success = resolve_skill_check(
                &state.player,
                "Literacy",
                SkillDifficulty::Hard,
                true,
                &mut rng,
                &mut lines,
            );
            if success {
                state.player.points.lp = state.player.points.lp.saturating_add(1);
                lines.push("LP +1".to_string());
            }
        }
        DowntimeActivity::RepairingRefitting => {
            add_stat_percentile_gain(&mut state.player, "int", "d6p", &mut rng, &mut lines);
            let success = resolve_skill_check(
                &state.player,
                "Craft",
                SkillDifficulty::Hard,
                true,
                &mut rng,
                &mut lines,
            );
            if success {
                let gain = roll_damage_expr("2d6p", &mut rng, false).max(0) as u32;
                state.inventory.gold = state.inventory.gold.saturating_add(gain);
                lines.push(format!("Coins +{gain}"));
            }
        }
        DowntimeActivity::Riding => {
            add_stat_percentile_gain(&mut state.player, "dex", "d6p", &mut rng, &mut lines);
            let success = resolve_skill_check(
                &state.player,
                "Riding",
                SkillDifficulty::Hard,
                false,
                &mut rng,
                &mut lines,
            );
            if success {
                add_skill_if_unskilled(&mut state.player, "Riding", &mut lines);
            } else {
                add_wound(state, "d4p", &mut rng, &mut lines);
            }
        }
        DowntimeActivity::Scouting => {
            add_stat_percentile_gain(&mut state.player, "wis", "d3p", &mut rng, &mut lines);
            add_stat_percentile_gain(&mut state.player, "con", "d3p", &mut rng, &mut lines);
            let observation_success = resolve_skill_check(
                &state.player,
                "Observation",
                SkillDifficulty::Hard,
                false,
                &mut rng,
                &mut lines,
            );
            if observation_success {
                lines.push("Scouting info gained.".to_string());
            }
            let survival_success = resolve_skill_check(
                &state.player,
                "Survival",
                SkillDifficulty::Easy,
                false,
                &mut rng,
                &mut lines,
            );
            if survival_success {
                add_skill_if_unskilled(&mut state.player, "Survival", &mut lines);
            } else {
                add_wound(state, "d4p", &mut rng, &mut lines);
            }
        }
        DowntimeActivity::SkillTutoring => {
            add_stat_percentile_gain(&mut state.player, "int", "d6p", &mut rng, &mut lines);
            state.player.points.lp = state.player.points.lp.saturating_add(2);
            lines.push("LP +2".to_string());
        }
        DowntimeActivity::SkillTraining => {
            add_stat_percentile_gain(&mut state.player, "int", "d6p", &mut rng, &mut lines);
            state.player.points.lp = state.player.points.lp.saturating_add(1);
            lines.push("LP +1".to_string());
        }
        DowntimeActivity::Sparring => {
            add_stat_percentile_gain(&mut state.player, "str", "d3p", &mut rng, &mut lines);
            add_stat_percentile_gain(&mut state.player, "dex", "d3p", &mut rng, &mut lines);
            add_stat_percentile_gain(&mut state.player, "con", "d3p", &mut rng, &mut lines);
            add_wound(state, "d4p", &mut rng, &mut lines);
            let trauma_roll = roll_damage_expr("d20", &mut rng, false);
            let con_half = (i32::from(state.player.base_stats.constitution)) / 2;
            if trauma_roll <= con_half {
                let xp = roll_damage_expr("6d6p", &mut rng, false);
                lines.push(format!("Weapon XP +{xp} (tracked later)"));
            } else {
                let xp = roll_damage_expr("3d6p", &mut rng, false);
                lines.push(format!("Weapon XP +{xp} (tracked later)"));
            }
            lines.push(format!("Trauma save: {trauma_roll} vs {con_half}"));
        }
        DowntimeActivity::Swimming => {
            let success = resolve_skill_check(
                &state.player,
                "Swimming",
                SkillDifficulty::Hard,
                true,
                &mut rng,
                &mut lines,
            );
            if success {
                add_stat_percentile_gain(&mut state.player, "str", "d6p", &mut rng, &mut lines);
                add_stat_percentile_gain(&mut state.player, "con", "d6p", &mut rng, &mut lines);
            } else {
                add_stat_percentile_gain(&mut state.player, "con", "d12p", &mut rng, &mut lines);
            }
        }
        DowntimeActivity::WeaponDrills => {
            add_stat_percentile_gain(&mut state.player, "str", "d3p", &mut rng, &mut lines);
            add_stat_percentile_gain(&mut state.player, "dex", "d3p", &mut rng, &mut lines);
            let xp = roll_damage_expr("3d6p", &mut rng, false);
            lines.push(format!("Weapon XP +{xp} (tracked later)"));
        }
    }
    if lines.is_empty() {
        lines.push("No reward.".to_string());
    }
    lines
}

pub fn run_app() {
    crate::console::maybe_enable_console();
    let args = AutobattlerArgs::parse();
    let headless = args.headless_screenshots || args.sprite_review;
    let window = Window {
        title: "HackMaster Autobattler".to_string(),
        resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
        resize_constraints: WindowResizeConstraints {
            min_width: 1360.0,
            min_height: 840.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut app = AutobattlerApp::new();
    let review_races = if args.sprite_review {
        app.screen = AppScreen::SpriteReview;
        let mut races = app
            .race_catalog
            .iter()
            .map(|race| race.id.clone())
            .collect::<Vec<_>>();
        races.push("hobgoblin".to_string());
        races
    } else {
        Vec::new()
    };
    if args.auto_start_run && !args.sprite_review {
        app.start_run_from_creation();
    }
    let mut screenshot_state = ScreenshotState::default();
    screenshot_state.headless_enabled = headless;
    let auto_allowed = headless && (args.auto_screenshots || args.auto_screenshot_count.is_some());
    screenshot_state.auto_allowed = auto_allowed;
    if auto_allowed {
        screenshot_state.auto_enabled = true;
        screenshot_state.use_latest_path = true;
        if let Some(interval) = args.auto_screenshot_interval {
            screenshot_state.interval_seconds = interval.max(0.1);
        }
        screenshot_state.max_auto_captures = args.auto_screenshot_count;
    }
    let mut app_builder = App::new();
    app_builder
        .insert_resource(AutobattlerState { app })
        .insert_resource(screenshot_state)
        .insert_resource(ClearColor(Color::rgb(0.08, 0.09, 0.1)));
    if args.sprite_review {
        app_builder.insert_resource(SpriteReviewState::new(review_races));
    }
    let plugins = DefaultPlugins.set(WindowPlugin {
        primary_window: Some(window),
        ..Default::default()
    });
    if headless {
        app_builder.add_plugins(plugins.disable::<WinitPlugin>());
        app_builder.add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        )));
        app_builder.insert_resource(HeadlessConfig {
            size: UVec2::new(WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32),
            format: TextureFormat::Rgba8UnormSrgb,
        });
        app_builder.add_plugins(HeadlessScreenshotPlugin);
    } else {
        app_builder.add_plugins(plugins);
    }
    app_builder
        .add_plugins(EguiPlugin)
        .add_systems(Startup, setup_render_system)
        .add_systems(
            Update,
            (
                ui_system,
                sprite_review_system.after(ui_system),
                screenshot_system.after(sprite_review_system),
                sync_render_system.after(screenshot_system),
            ),
        )
        .run();
}
