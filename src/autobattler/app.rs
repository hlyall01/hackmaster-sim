use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::utils::Duration;
use bevy::winit::WinitPlugin;
use bevy_egui::EguiPlugin;
use rand::RngCore;

use crate::autobattler::args::AutobattlerArgs;
use crate::autobattler::constants::{
    AUTOBATTLER_CONFIG_PATH, CHARACTER_SAVE_EXTENSION, NPC_PRESETS_PATH, RUN_SAVE_EXTENSION,
    RUN_SAVE_VERSION, SAVE_VERSION, START_AP, START_BP, START_LP, START_RP, STAT_COUNT,
    WINDOW_HEIGHT, WINDOW_WIDTH,
};
use crate::autobattler::logic::{
    apply_stat_adjustment, clamp_stat_adjustment, scaled_enemy_level, total_talent_costs,
};
use crate::autobattler::persistence::{
    read_character_save, read_run_save, run_save_path_for, sanitize_filename, save_path_for,
    scan_run_save_entries, scan_save_entries, write_character_save, write_run_save,
};
use crate::autobattler::render::{setup_render_system, sync_render_system};
use crate::autobattler::screenshot::{
    screenshot_system, HeadlessConfig, HeadlessScreenshotPlugin, ScreenshotState,
};
use crate::autobattler::sprite_review::sprite_review_system;
use crate::autobattler::state::{
    AppScreen, AutobattlerState, CharacterSave, CreationState, CreationStep, PointPool, RunAction,
    RunSave, RunStateSave, RunViewState, SaveEntry, SpriteReviewState,
};
use crate::autobattler::ui::ui_system;

use crate::autobattler::logic;

use crate::{character, data, game_logic};
use crate::character::{AbilityScore, AbilitySet};
use crate::core::catalog::Catalog;
use crate::core::gameplay::{
    apply_fight_result, AutobattlerConfig, CombatantBuilder, EnemySpawnEntry, EnemySpawner,
    FightResult, LootTable, RunState,
};
use crate::core::rng::SimRng;
use crate::core::sim::SimConfig;
use crate::core::types::{EnemyProfile, Inventory, PlayerProfile, RaceSpec};
use crate::game_logic::{
    ArmorCatalog, NpcPresetCatalog, PlayerConfig, ShieldCatalog, TalentCatalog, WeaponCatalog,
    WeaponId,
};

pub struct AutobattlerApp {
    pub screen: AppScreen,
    pub creation: CreationState,
    pub creation_step: CreationStep,
    pub creation_done: bool,
    pub save_entries: Vec<SaveEntry>,
    pub selected_save: Option<usize>,
    pub run_save_entries: Vec<SaveEntry>,
    pub selected_run_save: Option<usize>,
    pub save_name: String,
    pub save_status: Option<String>,
    pub run_save_name: String,
    pub run_save_status: Option<String>,
    pub needs_save_refresh: bool,
    pub run_state: Option<RunViewState>,
    pub autobattler_config: AutobattlerConfig,
    pub run_rng: SimRng,
    pub weapon_catalog: WeaponCatalog,
    pub armor_catalog: ArmorCatalog,
    pub shield_catalog: ShieldCatalog,
    pub npc_presets: NpcPresetCatalog,
    pub enemy_spawner: EnemySpawner,
    pub loot_table: LootTable,
    pub sim_config: SimConfig,
    pub enemy_weapon_id: WeaponId,
    pub race_catalog: Vec<RaceSpec>,
    pub talent_catalog: TalentCatalog,
}

impl AutobattlerApp {
    pub fn new() -> Self {
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
        let weapon_id = weapon_catalog.first_id().unwrap_or_else(|| WeaponId::new(0));
        let creation = CreationState::new(weapon_id);
        let enemy_weapon_id = find_weapon_id_by_name(&weapon_catalog, &autobattler_config.enemy_weapon)
            .or_else(|| weapon_catalog.first_id())
            .unwrap_or_else(|| WeaponId::new(0));
        let enemy_spawner = hobgoblin_spawner(&npc_presets);
        let loot_table = autobattler_config.to_loot_table();
        let sim_config =
            SimConfig::new(autobattler_config.start_distance, autobattler_config.stop_distance);
        Self {
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
        PointPool::new(START_BP, START_LP, START_AP, START_RP)
            .sub(PointPool::new(spent_stats, 0, 0, 0))
            .sub(spent_talents)
    }

    pub fn effective_charisma(&self) -> (u8, i32) {
        let looks = self.creation.stats[5].base;
        let delta = character::looks_charisma_adjustment(looks);
        let base = clamp_stat_adjustment(self.creation.stats[6].base, delta);
        (base, delta)
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
        self.creation = CreationState::new(weapon_id);
        self.creation_step = CreationStep::Points;
        self.creation_done = false;
        self.save_name.clear();
        self.save_status = None;
        self.run_save_name.clear();
        self.run_save_status = None;
        self.run_state = None;
    }

    pub fn can_advance(&self) -> bool {
        match self.creation_step {
            CreationStep::Points => true,
            CreationStep::RollStats => self.creation.stats_locked || self.all_rolls_assigned(),
            CreationStep::ChooseRace => self.creation.race_applied,
            CreationStep::SpendBp => self.creation.race_applied,
            CreationStep::Talents => false,
        }
    }

    pub fn can_finish(&self) -> bool {
        self.creation_step == CreationStep::Talents
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
                .map(|score| crate::autobattler::state::AbilityScoreSave::from_score(*score))
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
        let character = CharacterSave {
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
                self.creation = CreationState::new(self.creation.player.weapon_id);
                self.creation.name = save.name;
                if save.stats.len() >= STAT_COUNT {
                    for (idx, score) in save.stats.iter().take(STAT_COUNT).enumerate() {
                        self.creation.stats[idx] = score.to_score();
                    }
                }
                self.creation.race_index = save.race_id.as_ref().and_then(|id| {
                    self.race_catalog
                        .iter()
                        .position(|race| race.id == *id)
                });
                self.creation.player.race_id = save.race_id.clone();
                self.creation.player.talents = save.talents.clone();
                self.creation.bp_history = std::array::from_fn(|idx| {
                    save.bp_history.get(idx).cloned().unwrap_or_default()
                });
                self.creation.stats_locked = true;
                self.creation.race_applied = save.race_id.is_some();
                self.creation.player.race_applied = save.race_id.is_some();
                self.creation.sync_player_from_stats();
                self.creation_step = CreationStep::Talents;
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
                let run_state = save.run_state.to_state();
                self.creation = CreationState::new(self.creation.player.weapon_id);
                self.creation.name = save.character.name.clone();
                if save.character.stats.len() >= STAT_COUNT {
                    for (idx, score) in save.character.stats.iter().take(STAT_COUNT).enumerate() {
                        self.creation.stats[idx] = score.to_score();
                    }
                }
                self.creation.race_index = save.character.race_id.as_ref().and_then(|id| {
                    self.race_catalog
                        .iter()
                        .position(|race| race.id == *id)
                });
                self.creation.player.race_id = save.character.race_id.clone();
                self.creation.player.talents = save.character.talents.clone();
                self.creation.bp_history = std::array::from_fn(|idx| {
                    save.character.bp_history.get(idx).cloned().unwrap_or_default()
                });
                self.creation.stats_locked = true;
                self.creation.race_applied = save.character.race_id.is_some();
                self.creation.player.race_applied = save.character.race_id.is_some();
                self.creation.sync_player_from_stats();
                self.creation_step = CreationStep::Talents;
                self.creation_done = true;
                self.run_state = Some(RunViewState {
                    run_state,
                    last_outcome: None,
                    last_action: save.last_action,
                    last_log: save.last_log.clone(),
                    days_elapsed: save.days_elapsed,
                    training_days: save.training_days,
                    run_over: save.run_over,
                    live_fight: None,
                });
                self.screen = AppScreen::Run;
                self.run_save_status = Some("Loaded run.".to_string());
            }
            Err(err) => {
                self.run_save_status = Some(format!("Failed to load run: {err}"));
            }
        }
    }

    pub fn start_run_from_creation(&mut self) {
        let player_profile = player_profile_from_config(&self.creation.player);
        let run_state = RunState {
            player: player_profile,
            inventory: Inventory {
                gold: self.autobattler_config.loot.gold_min,
                items: Vec::new(),
            },
            run_depth: 1,
            wounds: Vec::new(),
        };
        self.run_state = Some(RunViewState::new(run_state));
        self.screen = AppScreen::Run;
    }

    pub fn start_live_fight(
        &mut self,
        rest_days: u32,
        resting: bool,
        action: Option<RunAction>,
    ) {
        let Some(run_view) = self.run_state.as_mut() else {
            return;
        };
        if run_view.run_over {
            return;
        }
        let player_level = run_view.run_state.player.level;
        let effective_level = scaled_enemy_level(player_level, run_view.run_state.run_depth);
        let Some(enemy_profile) = self
            .enemy_spawner
            .spawn_for_level(effective_level, &mut self.run_rng)
        else {
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
        let mut enemy_combatant = builder.build_enemy(&enemy_profile);
        player_combatant.team_id = 0;
        enemy_combatant.team_id = 1;
        let fight_seed = self.run_rng.next_u64();
        let mut sim = crate::core::sim::SimState::with_rng(
            self.sim_config,
            SimRng::from_seed(fight_seed),
        );
        sim.reset_with_combatants(vec![player_combatant, enemy_combatant]);

        run_view.live_fight = Some(crate::autobattler::state::LiveFight {
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

    pub fn run_action(&mut self, action: RunAction) {
        let rest_days = if action == RunAction::FightOn {
            0
        } else {
            action.rest_days()
        };
        let resting = action.is_resting();
        self.start_live_fight(rest_days, resting, Some(action));
    }

    pub fn complete_live_fight(&mut self) {
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

    pub fn start_new_character(&mut self) {
        self.reset_creation();
        self.creation_step = CreationStep::Points;
        self.creation_done = false;
        self.screen = AppScreen::Creation;
    }
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

    fn build_enemy(&self, enemy: &EnemyProfile) -> crate::core::sim::Combatant {
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

pub fn run_app() {
    crate::console::maybe_enable_console();
    let args = AutobattlerArgs::parse();
    let headless = args.headless_screenshots || args.sprite_review;
    let window = Window {
        title: "HackMaster Autobattler".to_string(),
        resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
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
    let auto_allowed =
        headless && (args.auto_screenshots || args.auto_screenshot_count.is_some());
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
