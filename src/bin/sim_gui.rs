#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use character::{Progression, ProgressionTier, WeaponGroup};
use eframe::egui::{self, Color32, Pos2, Rect};
use egui_plot::{
    GridInput, GridMark, HLine, Legend, Line, Plot, PlotPoint, PlotPoints, Points, Text, VLine,
};
use game_logic::{
    ArmorCatalog, ArmorEntry, ArmorId, FighterMasteries, FighterPreset, FighterPresetCatalog,
    FighterProgression, NpcPresetCatalog, PlayerConfig, ShieldCatalog, ShieldId, TalentCatalog,
    WeaponCatalog, WeaponHandedness, WeaponId, WeaponSize,
};
use hackmaster_sim::core::catalog::Catalog;
use hackmaster_sim::core::gameplay::run::{Wound, heal_wounds, required_healing_steps};
use hackmaster_sim::core::rng::SimRng;
use hackmaster_sim::core::tactics::{
    MAX_TACTICAL_CONDITIONS, NumericComparison, RelativeComparison, SpeedComparison,
    TacticalAction, TacticalCondition, TacticalPolicy, TacticalPreset, TacticalRule,
    validate_policy,
};
use hackmaster_sim::core::types::{RaceSpec, TalentSelection, TalentSpec};
use hackmaster_sim::ui_widgets::searchable_select;
use hackmaster_sim::{character, data, game_logic, sim};
use rand::SeedableRng;
use rand::rngs::StdRng;
use sim::{BulkSimResult, SimConfig, SimState, bulk_simulate_with_seed};
use std::{collections::BTreeMap, time::Instant};

#[derive(Clone, Copy)]
enum WeaponIcon {
    Sword,
    Dagger,
    Axe,
    Spear,
    Polearm,
    Bow,
    Crossbow,
    Blunt,
    Lash,
    Basic,
    Double,
    Ensnaring,
    Shield,
    Unarmed,
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MainTab {
    Simulator,
    DetailedStats,
    Tools,
}

impl MainTab {
    fn label(self) -> &'static str {
        match self {
            MainTab::Simulator => "Simulator",
            MainTab::DetailedStats => "Detailed Stats",
            MainTab::Tools => "Tools",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ToolTab {
    WoundHealing,
    EssenceWounds,
}

impl ToolTab {
    fn label(self) -> &'static str {
        match self {
            ToolTab::WoundHealing => "Wound Healing",
            ToolTab::EssenceWounds => "Essence Wounds",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PlayerEditorTab {
    Core,
    Gear,
    CombatManeuvers,
    Tactics,
    Stats,
    Talents,
    Derived,
    Tools,
}

impl PlayerEditorTab {
    fn label(self) -> &'static str {
        match self {
            PlayerEditorTab::Core => "Core",
            PlayerEditorTab::Gear => "Gear",
            PlayerEditorTab::CombatManeuvers => "Combat Maneuvers",
            PlayerEditorTab::Tactics => "Tactics",
            PlayerEditorTab::Stats => "Stats",
            PlayerEditorTab::Talents => "Talents",
            PlayerEditorTab::Derived => "Derived",
            PlayerEditorTab::Tools => "Tools",
        }
    }
}

const PLAYER_EDITOR_TABS: [PlayerEditorTab; 8] = [
    PlayerEditorTab::Core,
    PlayerEditorTab::Gear,
    PlayerEditorTab::CombatManeuvers,
    PlayerEditorTab::Tactics,
    PlayerEditorTab::Stats,
    PlayerEditorTab::Talents,
    PlayerEditorTab::Derived,
    PlayerEditorTab::Tools,
];

const FIGHTER_PRESETS_PATH: &str = "data/sim/fighter_presets.json";
const TACTICAL_PRESETS_PATH: &str = "data/sim/tactical_presets.json";
const BULK_SIM_MAX_SECONDS: u32 = u32::MAX;
const MAX_START_DISTANCE_FT: f32 = 4_000.0;
const MAX_DAMAGE_PLOT_ITERATIONS: usize = 1_000_000;
const TALENT_TAB_ALL: &str = "All";
const TALENT_TAB_LEARNED: &str = "Learned Talents";
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

struct SimGuiApp {
    running: bool,
    sim: SimState,
    players: [PlayerConfig; 2],
    player_colors: [Color32; 2],
    weapon_catalog: WeaponCatalog,
    armor_catalog: ArmorCatalog,
    shield_catalog: ShieldCatalog,
    race_catalog: Vec<RaceSpec>,
    talent_catalog: TalentCatalog,
    npc_presets: NpcPresetCatalog,
    fighter_presets: FighterPresetCatalog,
    fighter_preset_names: [String; 2],
    tactical_presets: Vec<TacticalPreset>,
    tactical_drafts: [TacticalPolicy; 2],
    tactical_preset_indices: [usize; 2],
    tactical_preset_names: [String; 2],
    tactical_messages: [Option<String>; 2],
    tactical_pending_loads: [Option<usize>; 2],
    tactical_confirm_overwrites: [bool; 2],
    tactical_confirm_deletes: [bool; 2],
    time_scale: f32,
    show_player_editor: [bool; 2],
    player_editor_tabs: [PlayerEditorTab; 2],
    talent_category_tabs: [String; 2],
    last_screen_size: egui::Vec2,
    bulk_runs: u32,
    bulk_seed: u64,
    bulk_last_seed: Option<u64>,
    bulk_result: Option<BulkSimResult>,
    bulk_sim_duration: Option<std::time::Duration>,
    dps_attacker_idx: usize,
    dps_defender_idx: usize,
    dps_iterations: u32,
    dps_duration_seconds: u32,
    dps_seed: u64,
    dps_result: Option<DpsTestResult>,
    dps_sim_duration: Option<std::time::Duration>,
    active_tab: MainTab,
    active_tool_tab: ToolTab,
    wound_tool_damage: u32,
    wound_tool_days_until_point_healed: f32,
    wound_tool_days: u32,
    wound_tool_tended: bool,
    wound_tool_fast_healer: bool,
    essence_tool_maximum: u32,
    essence_tool_starting: u32,
    essence_tool_change: i64,
    damage_plot_iterations: [String; 2],
    damage_roll_plots: [Option<DamageRollPlotData>; 2],
}

#[derive(Clone, Debug)]
struct DamageRollLine {
    name: String,
    color: Color32,
    points: Vec<[f64; 2]>,
    values: Vec<f64>,
    average: f64,
}

#[derive(Clone, Debug)]
struct DamageRollPlotData {
    lines: Vec<DamageRollLine>,
    iterations: usize,
    x_max: usize,
    y_max: f64,
}

#[derive(Clone, Debug)]
struct DpsTestResult {
    attacker_idx: usize,
    defender_idx: usize,
    iterations: u32,
    duration_seconds: u32,
    total_damage: u64,
    total_landed_damage: i64,
    total_rolled_damage: i64,
    damage_rolls: u64,
    attacks: u64,
    highest_crit_hit: i32,
    highest_noncrit_hit: i32,
    highest_shield_hit: i32,
    instakills: u32,
    dps: f64,
    avg_damage_per_run: f64,
    avg_attacks_per_run: f64,
}

impl SimGuiApp {
    fn new() -> Self {
        let (weapon_catalog, armor_catalog, shield_catalog) = data::load_catalogs()
            .unwrap_or_else(|err| panic!("Failed to load JSON catalogs: {err}"));
        let npc_presets = match data::load_npc_presets("data/npc_presets.json") {
            Ok(presets) => presets,
            Err(err) => {
                eprintln!("Failed to load NPC presets: {err}");
                Catalog::new(Vec::new())
            }
        };
        let fighter_presets = match data::load_fighter_presets(FIGHTER_PRESETS_PATH) {
            Ok(presets) => presets,
            Err(err) => {
                eprintln!("Failed to load fighter presets: {err}");
                Catalog::new(Vec::new())
            }
        };
        let (tactical_presets, tactical_load_error) =
            match data::load_tactical_presets(TACTICAL_PRESETS_PATH) {
                Ok(presets) => (presets, None),
                Err(err) => {
                    eprintln!("Failed to load tactical presets: {err}");
                    (Vec::new(), Some(err))
                }
            };
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
        let race_catalog = match data::load_races("data/races.json") {
            Ok(races) => races,
            Err(err) => {
                eprintln!("Failed to load races: {err}");
                Vec::new()
            }
        };
        let sim = SimState::new(SimConfig::new(200.0, 1.0));
        let weapon_a = weapon_catalog
            .id_from_index(1)
            .or_else(|| weapon_catalog.first_id())
            .unwrap_or(WeaponId::new(0));
        let weapon_b = weapon_catalog
            .id_from_index(2)
            .or_else(|| weapon_catalog.first_id())
            .unwrap_or(WeaponId::new(0));
        let mut app = Self {
            running: false,
            sim,
            players: [
                PlayerConfig::new("Arthur Du Randt", weapon_a),
                PlayerConfig::new("Zorya", weapon_b),
            ],
            player_colors: [
                Color32::from_rgb(214, 93, 69),
                Color32::from_rgb(70, 140, 210),
            ],
            weapon_catalog,
            armor_catalog,
            shield_catalog,
            race_catalog,
            talent_catalog,
            npc_presets,
            fighter_presets,
            fighter_preset_names: ["Arthur Du Randt".to_string(), "Zorya".to_string()],
            tactical_presets,
            tactical_drafts: [TacticalPolicy::default(), TacticalPolicy::default()],
            tactical_preset_indices: [0, 0],
            tactical_preset_names: [String::new(), String::new()],
            tactical_messages: [tactical_load_error.clone(), tactical_load_error],
            tactical_pending_loads: [None, None],
            tactical_confirm_overwrites: [false, false],
            tactical_confirm_deletes: [false, false],
            time_scale: 1.0,
            show_player_editor: [false, false],
            player_editor_tabs: [PlayerEditorTab::Core, PlayerEditorTab::Core],
            talent_category_tabs: [TALENT_TAB_ALL.to_string(), TALENT_TAB_ALL.to_string()],
            last_screen_size: egui::vec2(0.0, 0.0),
            bulk_runs: 1000,
            bulk_seed: 1,
            bulk_last_seed: None,
            bulk_result: None,
            bulk_sim_duration: None,
            dps_attacker_idx: 0,
            dps_defender_idx: 1,
            dps_iterations: 1000,
            dps_duration_seconds: 60,
            dps_seed: 1,
            dps_result: None,
            dps_sim_duration: None,
            active_tab: MainTab::Simulator,
            active_tool_tab: ToolTab::WoundHealing,
            wound_tool_damage: 7,
            wound_tool_days_until_point_healed: required_healing_steps(7, false) as f32 / 4.0,
            wound_tool_days: 1,
            wound_tool_tended: true,
            wound_tool_fast_healer: false,
            essence_tool_maximum: 1000,
            essence_tool_starting: 500,
            essence_tool_change: 0,
            damage_plot_iterations: ["10000".to_string(), "10000".to_string()],
            damage_roll_plots: [None, None],
        };
        app.apply_default_fighter_preset(0, "Arthur Du Randt");
        app.apply_default_fighter_preset(1, "Zorya");
        app.tactical_drafts = [
            app.players[0].tactical_policy.clone(),
            app.players[1].tactical_policy.clone(),
        ];
        app.reset_positions();
        app
    }

    fn apply_default_fighter_preset(&mut self, idx: usize, name: &str) {
        let preset_id = self
            .fighter_presets
            .entries()
            .iter()
            .position(|preset| preset.name.eq_ignore_ascii_case(name))
            .and_then(|index| self.fighter_presets.id_from_index(index));
        if let Some(id) = preset_id {
            if let Some(preset) = self.fighter_presets.get(id) {
                apply_fighter_preset(
                    &mut self.players[idx],
                    preset,
                    &self.weapon_catalog,
                    &self.armor_catalog,
                    &self.shield_catalog,
                    &self.race_catalog,
                );
                self.players[idx].fighter_preset = Some(id);
                self.players[idx].npc_preset = None;
                self.fighter_preset_names[idx] = preset.name.clone();
            }
        }
    }

    fn reset_positions(&mut self) {
        self.sanitize_players();
        let combatants = game_logic::build_combatants(
            &self.players,
            &self.weapon_catalog,
            &self.armor_catalog,
            &self.shield_catalog,
            &self.npc_presets,
            &self.talent_catalog,
        );
        self.sim.reset_with_combatants(combatants);
    }

    fn run_bulk_sim(&mut self) {
        self.bulk_result = None;
        self.bulk_sim_duration = None;
        self.sanitize_players();
        let combatants = game_logic::build_combatants(
            &self.players,
            &self.weapon_catalog,
            &self.armor_catalog,
            &self.shield_catalog,
            &self.npc_presets,
            &self.talent_catalog,
        );
        let config = SimConfig::new(
            self.sim.config.start_distance,
            self.sim.config.stop_distance,
        );
        let seed = self.bulk_seed;
        self.bulk_last_seed = Some(seed);
        self.bulk_seed = self.bulk_seed.wrapping_add(1).max(1);
        let start = Instant::now();
        let result = bulk_simulate_with_seed(
            config,
            combatants,
            self.bulk_runs,
            BULK_SIM_MAX_SECONDS,
            seed,
        );
        self.bulk_result = Some(result);
        self.bulk_sim_duration = Some(start.elapsed());
    }

    fn run_dps_test(&mut self) {
        self.dps_result = None;
        self.dps_sim_duration = None;
        self.sanitize_players();

        let attacker_idx = self.dps_attacker_idx.min(1);
        let mut defender_idx = self.dps_defender_idx.min(1);
        if defender_idx == attacker_idx {
            defender_idx = 1usize.saturating_sub(attacker_idx);
        }
        self.dps_attacker_idx = attacker_idx;
        self.dps_defender_idx = defender_idx;
        let iterations = self.dps_iterations.max(1);
        let duration_seconds = self.dps_duration_seconds.max(1);
        self.dps_iterations = iterations;
        self.dps_duration_seconds = duration_seconds;

        let mut combatants = game_logic::build_combatants(
            &self.players,
            &self.weapon_catalog,
            &self.armor_catalog,
            &self.shield_catalog,
            &self.npc_presets,
            &self.talent_catalog,
        );
        if let Some(defender) = combatants.get_mut(defender_idx) {
            defender.sheet.maneuvers.passive = true;
            defender.sheet.vitals.infinite_hp = true;
        }

        let config = SimConfig::new(
            self.sim.config.start_distance,
            game_logic::stop_distance_for_players(
                &self.players,
                &self.weapon_catalog,
                &self.talent_catalog,
            ),
        );
        let seed = self.dps_seed;
        self.dps_seed = self.dps_seed.wrapping_add(1).max(1);

        let start = Instant::now();
        let mut total_damage = 0u64;
        let mut total_landed_damage = 0i64;
        let mut total_rolled_damage = 0i64;
        let mut damage_rolls = 0u64;
        let mut attacks = 0u64;
        let mut highest_crit_hit = 0i32;
        let mut highest_noncrit_hit = 0i32;
        let mut highest_shield_hit = 0i32;
        let mut instakills = 0u32;

        for run_idx in 0..iterations {
            let run_seed = seed.wrapping_add(u64::from(run_idx));
            let mut sim = SimState::with_rng(config, SimRng::from_seed(run_seed));
            sim.log_events = true;
            sim.reset_with_combatants(combatants.clone());
            while sim.elapsed_seconds < duration_seconds {
                sim.tick();
            }

            let attacker_state = &sim.combatants[attacker_idx].state;
            total_damage =
                total_damage.saturating_add(u64::from(attacker_state.total_hp_damage_dealt));
            total_landed_damage += attacker_state.total_damage_landed_dealt;
            total_rolled_damage += attacker_state.total_damage_rolled_dealt;
            damage_rolls =
                damage_rolls.saturating_add(u64::from(attacker_state.damage_rolls_dealt));
            highest_crit_hit = highest_crit_hit.max(attacker_state.max_crit_hit_dealt);
            highest_noncrit_hit = highest_noncrit_hit.max(attacker_state.max_noncrit_hit_dealt);
            highest_shield_hit = highest_shield_hit.max(attacker_state.max_shield_hit_dealt);
            instakills = instakills.saturating_add(attacker_state.total_instakills_dealt);
            attacks = attacks.saturating_add(
                sim.combat_events
                    .iter()
                    .filter(|event| event.attacker_idx == attacker_idx)
                    .count() as u64,
            );
        }

        let total_seconds = iterations as f64 * duration_seconds as f64;
        let dps = total_damage as f64 / total_seconds.max(1.0);
        self.dps_result = Some(DpsTestResult {
            attacker_idx,
            defender_idx,
            iterations,
            duration_seconds,
            total_damage,
            total_landed_damage,
            total_rolled_damage,
            damage_rolls,
            attacks,
            highest_crit_hit,
            highest_noncrit_hit,
            highest_shield_hit,
            instakills,
            dps,
            avg_damage_per_run: total_damage as f64 / iterations as f64,
            avg_attacks_per_run: attacks as f64 / iterations as f64,
        });
        self.dps_sim_duration = Some(start.elapsed());
    }

    fn sanitize_players(&mut self) {
        for player in &mut self.players {
            game_logic::sanitize_player_ids(
                player,
                &self.weapon_catalog,
                &self.armor_catalog,
                &self.shield_catalog,
                &self.talent_catalog,
            );
        }
    }

    fn update_sim(&mut self, dt: f32) {
        if !self.running {
            return;
        }
        self.sim.update(dt);
        if self.sim.done {
            self.running = false;
        }
    }

    fn draw_arena(&self, ui: &mut egui::Ui, rect: Rect) {
        let padding = 20.0;
        if rect.width() <= padding * 2.0 || rect.height() <= 0.0 {
            return;
        }
        if self.sim.actors.len() < 2 {
            return;
        }
        let bg = ui.style().visuals.panel_fill;
        ui.painter().rect_filled(rect, 0.0, bg);
        let hud_bottom = self.draw_hud(ui, rect, padding);
        self.draw_timeline(ui, rect, padding, hud_bottom + 25.0);
        let painter = ui.painter();
        let ground_y = rect.center().y + rect.height() * 0.15;
        let left = rect.left() + padding;
        let right = rect.right() - padding;
        let arena_width = (right - left).max(1.0);
        let scale = arena_width / self.sim.config.start_distance.max(1.0);
        if !scale.is_finite() {
            return;
        }

        painter.line_segment(
            [Pos2::new(left, ground_y), Pos2::new(right, ground_y)],
            (2.0, Color32::from_gray(80)),
        );

        let tile_size = self.sim.config.tile_size_ft.max(0.01);
        let start_tiles = (self.sim.config.start_distance / tile_size).ceil() as i32;
        let padding_tiles = ((self.sim.config.grid_width - 1 - start_tiles) / 2).max(0);
        let x0_ft = (self.sim.actors[0].position.x - padding_tiles) as f32 * tile_size;
        let x1_ft = (self.sim.actors[1].position.x - padding_tiles) as f32 * tile_size;
        let mut x0 = left + x0_ft * scale;
        let mut x1 = left + x1_ft * scale;
        x0 = x0.clamp(left, right);
        x1 = x1.clamp(left, right);
        let gap = (x1 - x0).abs();
        let min_gap = 28.0;
        if gap < min_gap {
            let dir = if x1 >= x0 { 1.0 } else { -1.0 };
            if self.sim.combatants[0].sheet.offense.weapon.reach_ft
                >= self.sim.combatants[1].sheet.offense.weapon.reach_ft
            {
                x1 = x0 + dir * min_gap;
            } else {
                x0 = x1 - dir * min_gap;
            }
        }

        let fighter_positions = [(0usize, x0, 1.0_f32), (1usize, x1, -1.0_f32)];
        for (idx, x, facing) in fighter_positions {
            let combatant = &self.sim.combatants[idx];
            let player = &self.players[idx];
            let player_color = self.player_colors[idx];
            let knocked_back = combatant.state.knockback_immobile_seconds > 0;
            let downed = combatant.state.hp <= 0 || combatant.state.trauma_remaining_seconds > 0;
            let weapon_icon = self
                .weapon_catalog
                .get(player.weapon_id)
                .map(|weapon| weapon_icon_kind(weapon.group))
                .unwrap_or(WeaponIcon::Other);
            self.draw_person(
                painter,
                Pos2::new(x, ground_y),
                facing,
                player_color,
                downed,
                knocked_back,
                weapon_icon,
                player.mounted,
            );
            if knocked_back && !downed {
                painter.text(
                    Pos2::new(x, ground_y - 56.0),
                    egui::Align2::CENTER_CENTER,
                    "knocked",
                    egui::TextStyle::Small.resolve(ui.style()),
                    Color32::from_rgb(230, 160, 90),
                );
            }
        }
    }

    fn draw_hud(&self, ui: &mut egui::Ui, rect: Rect, padding: f32) -> f32 {
        let painter = ui.painter();
        let left = rect.left() + padding;
        let right = rect.right() - padding;
        let y = rect.top() + padding * 0.5;
        let total_width = (right - left).max(1.0);
        let bar_height = 10.0;
        let gap = 16.0;
        let bar_width = (total_width - gap).max(1.0) * 0.5;

        for (idx, player) in self.players.iter().enumerate() {
            let player_color = self.player_colors[idx];
            let hp = self.sim.combatants[idx].state.hp.max(0) as f32;
            let max_hp = self.sim.combatants[idx].sheet.vitals.max_hp.max(1) as f32;
            let hp_ratio = (hp / max_hp).clamp(0.0, 1.0);
            let bar_x = if idx == 0 { left } else { right - bar_width };
            let bg_rect =
                Rect::from_min_size(Pos2::new(bar_x, y), egui::vec2(bar_width, bar_height));
            painter.rect_filled(bg_rect, 3.0, Color32::from_gray(40));
            let fill_width = bar_width * hp_ratio;
            let fill_x = if idx == 0 {
                bar_x
            } else {
                bar_x + (bar_width - fill_width)
            };
            let fill_rect =
                Rect::from_min_size(Pos2::new(fill_x, y), egui::vec2(fill_width, bar_height));
            painter.rect_filled(fill_rect, 3.0, player_color);
            let name_x = if idx == 0 { bar_x } else { bar_x + bar_width };
            let align = if idx == 0 {
                egui::Align2::LEFT_CENTER
            } else {
                egui::Align2::RIGHT_CENTER
            };
            painter.text(
                Pos2::new(name_x, y - 4.0),
                align,
                &player.name,
                egui::TextStyle::Body.resolve(ui.style()),
                Color32::from_gray(220),
            );
        }
        y + bar_height
    }

    fn draw_timeline(&self, ui: &mut egui::Ui, rect: Rect, padding: f32, y: f32) {
        let painter = ui.painter();
        let left = rect.left() + padding;
        let right = rect.right() - padding;
        if right <= left {
            return;
        }

        let horizon = 8.0;
        let now = self.sim.elapsed_seconds as f32;
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

        for (idx, _) in self.players.iter().enumerate() {
            let player_color = self.player_colors[idx];
            if let Some(next) = self.sim.combatants[idx].state.next_attack_time_primary {
                let t = (next - now).max(0.0).min(horizon);
                let x = left + t * scale;
                let pos = Pos2::new(x, y - 14.0);
                painter.circle_filled(pos, 6.0, player_color);
            }
            if let Some(next) = self.sim.combatants[idx].state.next_attack_time_secondary {
                let secondary_color = Color32::from_rgb(
                    ((player_color.r() as u16 + 255) / 2) as u8,
                    ((player_color.g() as u16 + 255) / 2) as u8,
                    ((player_color.b() as u16 + 255) / 2) as u8,
                );
                let t = (next - now).max(0.0).min(horizon);
                let x = left + t * scale;
                let pos = Pos2::new(x, y - 4.0);
                painter.circle_filled(pos, 4.0, secondary_color);
            }
        }
    }

    fn draw_person(
        &self,
        painter: &egui::Painter,
        base: Pos2,
        facing: f32,
        color: Color32,
        downed: bool,
        knocked_back: bool,
        weapon_icon: WeaponIcon,
        mounted: bool,
    ) {
        let head_color = color;
        let body_color = Color32::from_gray(230);
        let stroke = (2.0, body_color);

        if downed {
            let torso_start = Pos2::new(base.x - facing * 2.0, base.y - 4.0);
            let torso_end = Pos2::new(base.x + facing * 16.0, base.y - 4.0);
            let head = Pos2::new(base.x + facing * 22.0, base.y - 6.0);
            painter.line_segment([torso_start, torso_end], stroke);
            painter.line_segment(
                [torso_start, Pos2::new(base.x - facing * 6.0, base.y - 1.0)],
                stroke,
            );
            painter.line_segment(
                [torso_start, Pos2::new(base.x + facing * 4.0, base.y - 10.0)],
                stroke,
            );
            painter.circle_filled(head, 6.0, head_color);
            painter.line_segment(
                [torso_end, Pos2::new(base.x + facing * 10.0, base.y - 12.0)],
                stroke,
            );
            draw_weapon_icon(
                painter,
                Pos2::new(base.x + facing * 6.0, base.y - 10.0),
                facing,
                weapon_icon,
            );
            return;
        }

        if knocked_back {
            let torso_start = Pos2::new(base.x, base.y - 8.0);
            let torso_end = Pos2::new(base.x + facing * 16.0, base.y - 20.0);
            let head = Pos2::new(base.x + facing * 20.0, base.y - 24.0);
            painter.line_segment([torso_start, torso_end], stroke);
            painter.line_segment(
                [torso_start, Pos2::new(base.x - facing * 6.0, base.y - 2.0)],
                stroke,
            );
            painter.line_segment(
                [torso_start, Pos2::new(base.x + facing * 6.0, base.y - 2.0)],
                stroke,
            );
            painter.circle_filled(head, 6.0, head_color);
            painter.line_segment(
                [torso_end, Pos2::new(base.x + facing * 26.0, base.y - 14.0)],
                stroke,
            );
            draw_weapon_icon(
                painter,
                Pos2::new(base.x + facing * 26.0, base.y - 14.0),
                facing,
                weapon_icon,
            );
            return;
        }

        if mounted {
            self.draw_mounted_person(painter, base, facing, head_color, weapon_icon);
            return;
        }

        let head = Pos2::new(base.x, base.y - 34.0);
        let neck = Pos2::new(base.x, base.y - 26.0);
        let torso = Pos2::new(base.x, base.y - 14.0);
        painter.circle_filled(head, 6.5, head_color);
        painter.line_segment([neck, torso], stroke);
        painter.line_segment([torso, Pos2::new(base.x - 6.0, base.y - 2.0)], stroke);
        painter.line_segment([torso, Pos2::new(base.x + 6.0, base.y - 2.0)], stroke);
        let arm_start = Pos2::new(base.x, base.y - 22.0);
        let arm_end = Pos2::new(base.x + facing * 12.0, base.y - 18.0);
        painter.line_segment([arm_start, arm_end], stroke);
        draw_weapon_icon(painter, arm_end, facing, weapon_icon);
    }

    fn draw_mounted_person(
        &self,
        painter: &egui::Painter,
        base: Pos2,
        facing: f32,
        rider_color: Color32,
        weapon_icon: WeaponIcon,
    ) {
        let scale = 3.0_f32;
        let pt = |dx: f32, dy: f32| Pos2::new(base.x + facing * dx * scale, base.y + dy * scale);

        let horse_line = Color32::from_rgb(205, 176, 136);
        let horse_accent = Color32::from_rgb(140, 113, 84);
        let outline = (2.2, horse_line);
        let leg_stroke = (2.3, horse_line);

        // Large line-art horse body outline with an explicit neck silhouette.
        let rump = pt(-21.0, -17.0);
        let croup = pt(-14.0, -21.2);
        let back_mid = pt(-6.0, -23.3);
        let withers = pt(1.5, -24.5);
        let neck_crest = pt(10.5, -29.0);
        let poll = pt(17.2, -30.2);
        let forehead = pt(21.0, -29.2);
        let nose_bridge = pt(27.0, -26.8);
        let muzzle_tip = pt(31.4, -23.6);
        let chin = pt(29.4, -19.8);
        let jaw = pt(23.0, -17.9);
        let throat_latch = pt(16.6, -19.6);
        let neck_base_front = pt(10.8, -21.8);
        let chest = pt(7.8, -17.7);
        let belly_front = pt(5.0, -14.0);
        let belly_mid = pt(-4.8, -13.2);
        let belly_rear = pt(-15.5, -14.2);

        painter.line_segment([rump, croup], outline);
        painter.line_segment([croup, back_mid], outline);
        painter.line_segment([back_mid, withers], outline);
        painter.line_segment([withers, neck_crest], outline);
        painter.line_segment([neck_crest, poll], outline);
        painter.line_segment([poll, forehead], outline);
        painter.line_segment([forehead, nose_bridge], outline);
        painter.line_segment([nose_bridge, muzzle_tip], outline);
        painter.line_segment([muzzle_tip, chin], outline);
        painter.line_segment([chin, jaw], outline);
        painter.line_segment([jaw, throat_latch], outline);
        painter.line_segment([throat_latch, neck_base_front], outline);
        painter.line_segment([neck_base_front, chest], outline);
        painter.line_segment([chest, belly_front], outline);
        painter.line_segment([belly_front, belly_mid], outline);
        painter.line_segment([belly_mid, belly_rear], outline);
        painter.line_segment([belly_rear, rump], outline);

        // Tail.
        painter.line_segment([pt(-20.8, -18.1), pt(-28.6, -27.8)], outline);
        painter.line_segment([pt(-20.2, -16.9), pt(-28.0, -22.1)], (1.6, horse_accent));

        // Head and face details.
        painter.line_segment([pt(19.3, -31.0), pt(20.8, -34.4)], outline);
        painter.line_segment([pt(16.8, -31.0), pt(17.3, -34.1)], outline);
        painter.line_segment([pt(25.8, -25.4), pt(29.3, -24.1)], (1.2, horse_line));
        painter.circle_filled(pt(24.2, -25.8), 1.25, Color32::from_rgb(38, 30, 24));
        painter.circle_filled(pt(30.2, -22.6), 0.95, Color32::from_rgb(66, 46, 32));

        // All hoof points are exactly on the ground line (base.y).
        let legs = [
            (
                -15.5_f32, -14.0_f32, -17.2_f32, -5.8_f32, -16.6_f32, 0.0_f32,
            ),
            (-7.5, -13.6, -6.8, -5.2, -7.4, 0.0),
            (4.5, -16.3, 6.5, -6.5, 6.1, 0.0),
            (9.6, -17.2, 12.4, -6.0, 11.8, 0.0),
        ];
        for (sx, sy, kx, ky, hx, hy) in legs {
            let hip = pt(sx, sy);
            let knee = pt(kx, ky);
            let hoof = pt(hx, hy);
            painter.line_segment([hip, knee], leg_stroke);
            painter.line_segment([knee, hoof], leg_stroke);
            painter.circle_filled(knee, 1.6, horse_accent);
            painter.line_segment(
                [
                    Pos2::new(hoof.x - facing * 2.6, hoof.y),
                    Pos2::new(hoof.x + facing * 2.6, hoof.y),
                ],
                (2.3, horse_accent),
            );
        }

        // Rider line art.
        let rider_stroke = (2.0, Color32::from_gray(232));
        let saddle = pt(-2.2, -24.2);
        let rider_hips = Pos2::new(saddle.x - facing * 1.5, saddle.y + 3.0);
        let rider_torso = Pos2::new(rider_hips.x - facing * 0.5, rider_hips.y - 12.0);
        let rider_neck = Pos2::new(rider_torso.x, rider_torso.y - 8.0);
        let rider_head = Pos2::new(rider_neck.x, rider_neck.y - 8.0);
        painter.circle_filled(rider_head, 6.5, rider_color);
        painter.line_segment([rider_neck, rider_torso], rider_stroke);
        painter.line_segment([rider_torso, rider_hips], rider_stroke);
        painter.line_segment(
            [
                rider_hips,
                Pos2::new(rider_hips.x - facing * 6.0, rider_hips.y + 4.5),
            ],
            rider_stroke,
        );
        painter.line_segment(
            [
                rider_hips,
                Pos2::new(rider_hips.x + facing * 5.0, rider_hips.y + 4.2),
            ],
            rider_stroke,
        );
        let arm_start = Pos2::new(rider_torso.x + facing * 0.8, rider_torso.y + 2.0);
        let arm_end = Pos2::new(arm_start.x + facing * 12.0, arm_start.y + 1.5);
        painter.line_segment([arm_start, arm_end], rider_stroke);
        let weapon_anchor = arm_end;
        draw_weapon_icon(painter, weapon_anchor, facing, weapon_icon);
    }
}

fn weapon_icon_kind(group: WeaponGroup) -> WeaponIcon {
    match group {
        WeaponGroup::Unarmed => WeaponIcon::Unarmed,
        WeaponGroup::Axes => WeaponIcon::Axe,
        WeaponGroup::Basic => WeaponIcon::Basic,
        WeaponGroup::Blunt => WeaponIcon::Blunt,
        WeaponGroup::Bows => WeaponIcon::Bow,
        WeaponGroup::Crossbows => WeaponIcon::Crossbow,
        WeaponGroup::Double => WeaponIcon::Double,
        WeaponGroup::Ensnaring => WeaponIcon::Ensnaring,
        WeaponGroup::Lashes => WeaponIcon::Lash,
        WeaponGroup::LargeSwords => WeaponIcon::Sword,
        WeaponGroup::SmallSwords => WeaponIcon::Dagger,
        WeaponGroup::Polearms => WeaponIcon::Polearm,
        WeaponGroup::Spears => WeaponIcon::Spear,
        WeaponGroup::Shields => WeaponIcon::Shield,
    }
}

fn draw_weapon_icon(painter: &egui::Painter, pos: Pos2, facing: f32, icon: WeaponIcon) {
    let stroke = (2.0, Color32::from_gray(220));
    let accent = Color32::from_gray(170);
    match icon {
        WeaponIcon::Sword => {
            let tip = Pos2::new(pos.x + facing * 12.0, pos.y - 10.0);
            let guard = Pos2::new(pos.x + facing * 3.0, pos.y - 2.0);
            painter.line_segment([pos, tip], stroke);
            painter.line_segment(
                [
                    Pos2::new(guard.x - facing * 3.0, guard.y + 2.0),
                    Pos2::new(guard.x + facing * 3.0, guard.y - 2.0),
                ],
                stroke,
            );
            painter.circle_filled(Pos2::new(pos.x - facing * 1.0, pos.y + 1.0), 1.5, accent);
        }
        WeaponIcon::Dagger => {
            let tip = Pos2::new(pos.x + facing * 8.0, pos.y - 6.0);
            let guard = Pos2::new(pos.x + facing * 2.0, pos.y - 1.0);
            painter.line_segment([pos, tip], stroke);
            painter.line_segment(
                [
                    Pos2::new(guard.x - facing * 2.0, guard.y + 1.5),
                    Pos2::new(guard.x + facing * 2.0, guard.y - 1.5),
                ],
                stroke,
            );
        }
        WeaponIcon::Axe => {
            let handle_end = Pos2::new(pos.x + facing * 8.0, pos.y - 6.0);
            painter.line_segment([pos, handle_end], stroke);
            let blade_top = Pos2::new(handle_end.x + facing * 3.0, handle_end.y - 3.0);
            let blade_bottom = Pos2::new(handle_end.x + facing * 3.0, handle_end.y + 3.0);
            painter.line_segment([handle_end, blade_top], stroke);
            painter.line_segment([handle_end, blade_bottom], stroke);
        }
        WeaponIcon::Spear => {
            let tip = Pos2::new(pos.x + facing * 16.0, pos.y - 10.0);
            let base = Pos2::new(pos.x + facing * 11.0, pos.y - 7.0);
            painter.line_segment([pos, tip], stroke);
            painter.line_segment([tip, Pos2::new(base.x, base.y + 3.0)], stroke);
            painter.line_segment([tip, Pos2::new(base.x, base.y - 3.0)], stroke);
        }
        WeaponIcon::Polearm => {
            let tip = Pos2::new(pos.x + facing * 18.0, pos.y - 12.0);
            let blade_back = Pos2::new(tip.x - facing * 4.0, tip.y + 2.0);
            let blade_low = Pos2::new(tip.x - facing * 2.0, tip.y + 7.0);
            painter.line_segment([pos, tip], stroke);
            painter.line_segment([tip, blade_back], stroke);
            painter.line_segment([blade_back, blade_low], stroke);
            painter.line_segment([blade_low, tip], stroke);
            painter.line_segment(
                [
                    blade_back,
                    Pos2::new(blade_back.x - facing * 3.0, blade_back.y - 2.0),
                ],
                stroke,
            );
        }
        WeaponIcon::Bow => {
            let grip = Pos2::new(pos.x + facing * 2.0, pos.y - 4.0);
            let top = Pos2::new(grip.x, grip.y - 8.0);
            let mid = Pos2::new(grip.x + facing * 4.0, grip.y);
            let bottom = Pos2::new(grip.x, grip.y + 8.0);
            painter.line_segment([pos, grip], stroke);
            painter.line_segment([top, mid], stroke);
            painter.line_segment([mid, bottom], stroke);
            painter.line_segment([top, bottom], (1.0, accent));
        }
        WeaponIcon::Crossbow => {
            let stock_end = Pos2::new(pos.x + facing * 10.0, pos.y - 2.0);
            let limb_center = Pos2::new(pos.x + facing * 6.0, pos.y - 4.0);
            painter.line_segment([pos, stock_end], stroke);
            painter.line_segment(
                [
                    Pos2::new(limb_center.x, limb_center.y - 4.0),
                    Pos2::new(limb_center.x, limb_center.y + 4.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    Pos2::new(limb_center.x - facing * 3.0, limb_center.y - 4.0),
                    Pos2::new(limb_center.x - facing * 3.0, limb_center.y + 4.0),
                ],
                (1.0, accent),
            );
            painter.line_segment(
                [
                    Pos2::new(pos.x + facing * 5.0, pos.y - 4.0),
                    Pos2::new(pos.x + facing * 9.0, pos.y - 4.0),
                ],
                (1.0, accent),
            );
        }
        WeaponIcon::Blunt => {
            let end = Pos2::new(pos.x + facing * 8.0, pos.y - 6.0);
            painter.line_segment([pos, end], stroke);
            let head = Rect::from_center_size(end, egui::vec2(5.0, 5.0));
            painter.rect_filled(head, 1.0, accent);
        }
        WeaponIcon::Lash => {
            let p1 = Pos2::new(pos.x + facing * 4.0, pos.y - 6.0);
            let p2 = Pos2::new(pos.x + facing * 8.0, pos.y - 2.0);
            let p3 = Pos2::new(pos.x + facing * 12.0, pos.y - 8.0);
            painter.line_segment([pos, p1], stroke);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
            painter.circle_filled(p3, 2.0, accent);
        }
        WeaponIcon::Basic => {
            let end = Pos2::new(pos.x + facing * 9.0, pos.y - 7.0);
            painter.line_segment([pos, end], stroke);
            painter.circle_filled(Pos2::new(end.x - facing * 1.0, end.y), 1.5, accent);
        }
        WeaponIcon::Double => {
            let end = Pos2::new(pos.x + facing * 14.0, pos.y - 10.0);
            painter.line_segment([pos, end], stroke);
            painter.line_segment([pos, Pos2::new(pos.x + facing * 2.5, pos.y - 4.0)], stroke);
            painter.line_segment([end, Pos2::new(end.x - facing * 2.5, end.y + 4.0)], stroke);
        }
        WeaponIcon::Ensnaring => {
            let end = Pos2::new(pos.x + facing * 10.0, pos.y - 6.0);
            painter.line_segment([pos, end], stroke);
            let ring = Rect::from_center_size(end, egui::vec2(6.0, 6.0));
            painter.rect_stroke(ring, 3.0, (1.0, accent));
            painter.line_segment(
                [
                    Pos2::new(ring.left(), ring.center().y),
                    Pos2::new(ring.right(), ring.center().y),
                ],
                (1.0, accent),
            );
        }
        WeaponIcon::Shield => {
            let rect = Rect::from_center_size(
                Pos2::new(pos.x + facing * 6.0, pos.y - 6.0),
                egui::vec2(8.0, 12.0),
            );
            painter.rect_filled(rect, 2.0, Color32::from_gray(180));
            painter.line_segment(
                [
                    Pos2::new(rect.center().x, rect.top() + 2.0),
                    Pos2::new(rect.center().x, rect.bottom() - 2.0),
                ],
                (1.0, Color32::from_gray(120)),
            );
        }
        WeaponIcon::Unarmed | WeaponIcon::Other => {
            painter.circle_filled(pos, 2.5, Color32::from_gray(200));
        }
    }
}

impl eframe::App for SimGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let player_editors_were_open = self.show_player_editor;
        let dt = ctx.input(|i| i.unstable_dt).min(0.05) * self.time_scale;
        let screen_rect = ctx.input(|i| i.screen_rect);
        if screen_rect.size() != self.last_screen_size {
            self.last_screen_size = screen_rect.size();
            ctx.request_repaint();
        }
        self.sim.config.stop_distance = game_logic::stop_distance_for_players(
            &self.players,
            &self.weapon_catalog,
            &self.talent_catalog,
        );
        self.update_sim(dt);

        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.active_tab,
                    MainTab::Simulator,
                    MainTab::Simulator.label(),
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    MainTab::DetailedStats,
                    MainTab::DetailedStats.label(),
                );
                ui.selectable_value(&mut self.active_tab, MainTab::Tools, MainTab::Tools.label());
                if self.active_tab == MainTab::Simulator {
                    ui.separator();
                    if ui
                        .button(if self.running { "Pause" } else { "Start" })
                        .clicked()
                    {
                        if !self.running && (self.sim.done || self.sim.elapsed_seconds == 0) {
                            self.reset_positions();
                        }
                        self.running = !self.running;
                    }
                    if ui.button("Reset").clicked() {
                        self.running = false;
                        self.reset_positions();
                    }
                    if !self.running {
                        if ui.button("Next second").clicked() {
                            if self.sim.done || self.sim.elapsed_seconds == 0 {
                                self.reset_positions();
                            }
                            self.sim.tick();
                        }
                    }
                    ui.separator();
                    ui.label("Start distance (ft)");
                    let mut start_distance = self.sim.config.start_distance;
                    if ui
                        .add(
                            egui::Slider::new(&mut start_distance, 0.0..=MAX_START_DISTANCE_FT)
                                .step_by(5.0),
                        )
                        .changed()
                    {
                        self.sim.config.set_start_distance(start_distance);
                        if !self.running {
                            self.reset_positions();
                        }
                    }
                    ui.label("Timescale");
                    ui.add(egui::Slider::new(&mut self.time_scale, 0.25..=4.0).step_by(0.25));
                }
            });
        });

        if self.active_tab == MainTab::Tools {
            egui::CentralPanel::default().show(ctx, |ui| {
                render_tools_tab(
                    ui,
                    &mut self.active_tool_tab,
                    &mut self.wound_tool_damage,
                    &mut self.wound_tool_days_until_point_healed,
                    &mut self.wound_tool_days,
                    &mut self.wound_tool_tended,
                    &mut self.wound_tool_fast_healer,
                    &mut self.essence_tool_maximum,
                    &mut self.essence_tool_starting,
                    &mut self.essence_tool_change,
                );
            });
            return;
        }

        if self.active_tab == MainTab::DetailedStats {
            let mut run_bulk = false;
            egui::CentralPanel::default().show(ctx, |ui| {
                render_detailed_stats_tab(
                    ui,
                    &self.players,
                    self.sim.config.start_distance,
                    &mut self.bulk_runs,
                    &mut self.bulk_seed,
                    self.bulk_last_seed,
                    &self.bulk_result,
                    self.bulk_sim_duration,
                    &mut run_bulk,
                );
            });
            if run_bulk {
                self.running = false;
                self.run_bulk_sim();
            }
            return;
        }

        egui::SidePanel::left("players")
            .resizable(true)
            .min_width(220.0)
            .show(ctx, |ui| {
                ui.heading("Characters");
                ui.separator();
                for idx in 0..self.players.len() {
                    let weapon_name = self
                        .weapon_catalog
                        .get(self.players[idx].weapon_id)
                        .map(|weapon| weapon.name.as_str())
                        .unwrap_or("Unarmed");
                    ui.horizontal(|ui| {
                        ui.label(format!("{} ({})", self.players[idx].name, weapon_name));
                        if ui.button("Customize").clicked() {
                            self.show_player_editor[idx] = true;
                        }
                    });
                    ui.label(format!("Move: {:.0} ft/s", self.players[idx].move_speed));
                    if idx == 0 {
                        ui.separator();
                    }
                }
                ui.separator();
                ui.heading("Bulk sim");
                ui.horizontal(|ui| {
                    ui.label("Runs");
                    ui.add(
                        egui::DragValue::new(&mut self.bulk_runs)
                            .clamp_range(1..=u32::MAX)
                            .speed(100.0),
                    );
                });
                if ui.button("Run bulk").clicked() {
                    self.running = false;
                    self.run_bulk_sim();
                }
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if let Some(result) = &self.bulk_result {
                            let total_runs = (result.wins.iter().copied().sum::<u32>()
                                + result.ties)
                                .max(1) as f32;
                            let wins_a = result.wins.get(0).copied().unwrap_or(0);
                            let wins_b = result.wins.get(1).copied().unwrap_or(0);
                            ui.label(format!(
                                "{} wins: {} ({:.1}%)",
                                self.players[0].name,
                                wins_a,
                                wins_a as f32 * 100.0 / total_runs
                            ));
                            ui.label(format!(
                                "{} wins: {} ({:.1}%)",
                                self.players[1].name,
                                wins_b,
                                wins_b as f32 * 100.0 / total_runs
                            ));
                            if result.ties > 0 {
                                ui.label(format!(
                                    "Ties/timeouts: {} ({:.1}%)",
                                    result.ties,
                                    result.ties as f32 * 100.0 / total_runs
                                ));
                            }
                            ui.label(format!("Avg duration: {:.1}s", result.avg_duration));
                            egui::CollapsingHeader::new("Detailed metrics")
                                .default_open(true)
                                .show(ui, |ui| {
                                    ui.label(format!(
                                        "Shortest fight: {}s",
                                        result.shortest_duration
                                    ));
                                    ui.label(format!(
                                        "Longest fight: {}s",
                                        result.longest_duration
                                    ));
                                    ui.label(format!(
                                        "Highest crit HP hit: {}",
                                        result.highest_single_crit_hit
                                    ));
                                    ui.label(format!(
                                        "Highest non-crit HP hit: {}",
                                        result.highest_single_noncrit_hit
                                    ));
                                    ui.label(format!(
                                        "Highest shield hit: {}",
                                        result.highest_single_shield_hit
                                    ));
                                    if result.shields_present {
                                        ui.label(format!(
                                            "Shield breaks: {}",
                                            result.shield_breaks
                                        ));
                                        if result.shield_breaks > 0 {
                                            ui.label(format!(
                                                "Avg hits shield survived: {:.2}",
                                                result.avg_hits_shield_survived
                                            ));
                                        } else {
                                            ui.label("Avg hits shield survived: n/a (no breaks)");
                                        }
                                    }
                                    ui.label(format!("Instakills: {}", result.instakills));
                                    ui.label(format!(
                                        "Max one-side knockback in a fight: {:.1} ft",
                                        result.max_total_knockback_one_side_ft
                                    ));
                                    ui.separator();
                                    ui.label(format!(
                                        "Fights w/ 2+ charges: {}",
                                        result.fights_with_second_charge
                                    ));
                                    ui.label(format!(
                                        "Fights w/ trauma: {}",
                                        result.fights_with_trauma
                                    ));
                                    ui.label(format!(
                                        "Fights w/ trauma on first exchange: {}",
                                        result.fights_with_trauma_first_exchange
                                    ));
                                    ui.label(format!(
                                        "Fights w/ 20ft knockback: {}",
                                        result.fights_with_knockback_20ft
                                    ));
                                    for idx in 0..self.players.len() {
                                        let name = &self.players[idx].name;
                                        let avg_dealt = result
                                            .avg_damage_dealt_by_team
                                            .get(idx)
                                            .copied()
                                            .unwrap_or(0.0);
                                        let avg_taken = result
                                            .avg_damage_taken_by_team
                                            .get(idx)
                                            .copied()
                                            .unwrap_or(0.0);
                                        let avg_rolled = result
                                            .avg_damage_rolled_by_team
                                            .get(idx)
                                            .copied()
                                            .unwrap_or(0.0);
                                        let avg_landed = result
                                            .avg_damage_landed_by_team
                                            .get(idx)
                                            .copied()
                                            .unwrap_or(0.0);
                                        let avg_hp = result
                                            .avg_remaining_hp_by_team
                                            .get(idx)
                                            .copied()
                                            .unwrap_or(0.0);
                                        let top_crit_hit = result
                                            .highest_single_crit_hit_by_team
                                            .get(idx)
                                            .copied()
                                            .unwrap_or(0);
                                        let top_noncrit_hit = result
                                            .highest_single_noncrit_hit_by_team
                                            .get(idx)
                                            .copied()
                                            .unwrap_or(0);
                                        let top_shield_hit = result
                                            .highest_single_shield_hit_by_team
                                            .get(idx)
                                            .copied()
                                            .unwrap_or(0);
                                        let instakills = result
                                            .instakills_by_team
                                            .get(idx)
                                            .copied()
                                            .unwrap_or(0);
                                        ui.separator();
                                        ui.label(format!("{name} avg dmg dealt: {:.1}", avg_dealt));
                                        ui.label(format!("{name} avg dmg taken: {:.1}", avg_taken));
                                        ui.label(format!(
                                            "{name} average damage rolled: {:.1}",
                                            avg_rolled
                                        ));
                                        ui.label(format!(
                                            "{name} average damage landed: {:.1}",
                                            avg_landed
                                        ));
                                        ui.label(format!("{name} avg remaining HP: {:.1}", avg_hp));
                                        ui.label(format!(
                                            "{name} highest crit hit: {}",
                                            top_crit_hit
                                        ));
                                        ui.label(format!(
                                            "{name} highest non-crit hit: {}",
                                            top_noncrit_hit
                                        ));
                                        ui.label(format!(
                                            "{name} highest shield hit: {}",
                                            top_shield_hit
                                        ));
                                        ui.label(format!("{name} instakills: {}", instakills));
                                    }
                                });
                            if let Some(duration) = self.bulk_sim_duration {
                                ui.label(format!("Sim time: {:.2}s", duration.as_secs_f64()));
                            }
                        } else {
                            ui.label("No bulk results yet.");
                            ui.label("Click 'Run bulk' to generate metrics.");
                        }
                    });
            });

        egui::SidePanel::right("status")
            .resizable(false)
            .min_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Status");
                ui.separator();
                ui.label(format!("Elapsed: {}s", self.sim.elapsed_seconds));
                ui.label(format!(
                    "Distance: {:.1} ft | Timescale: {:.2}x",
                    self.sim.distance(),
                    self.time_scale
                ));
                ui.label(format!(
                    "Stop distance: {:.1} ft",
                    self.sim.config.stop_distance
                ));
                ui.separator();
                ui.label(format!(
                    "{} HP: {}",
                    self.sim.combatants[0].sheet.name, self.sim.combatants[0].state.hp
                ));
                ui.label(format!(
                    "{} HP: {}",
                    self.sim.combatants[1].sheet.name, self.sim.combatants[1].state.hp
                ));
                for combatant in &self.sim.combatants {
                    if combatant.tactical_policy.enabled {
                        let styles = if combatant.active_style_ids.is_empty() {
                            "No style".to_string()
                        } else {
                            combatant.active_style_ids.join(" + ")
                        };
                        ui.label(format!("{} style: {styles}", combatant.sheet.name));
                        if let Some(directive) = combatant.last_tactical_directive.as_ref() {
                            ui.small(format!("Last: {directive}"));
                        }
                    }
                }
                if let Some(event) = &self.sim.last_event {
                    ui.separator();
                    ui.label(sim::format_combat_event_line(event, &self.sim.combatants));
                }
                ui.label(if self.sim.done {
                    "State: Done"
                } else if self.running {
                    "State: Running"
                } else {
                    "State: Idle"
                });
                ui.separator();
                ui.label(format!(
                    "{}: {}",
                    self.players[0].name,
                    self.weapon_catalog
                        .get(self.players[0].weapon_id)
                        .map(|weapon| weapon.name.as_str())
                        .unwrap_or("Unarmed")
                ));
                ui.label(format!(
                    "{}: {}",
                    self.players[1].name,
                    self.weapon_catalog
                        .get(self.players[1].weapon_id)
                        .map(|weapon| weapon.name.as_str())
                        .unwrap_or("Unarmed")
                ));
                ui.separator();
                ui.label("Combat log");
                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show(ui, |ui| {
                        for event in &self.sim.combat_events {
                            ui.label(sim::format_combat_event_line(event, &self.sim.combatants));
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.max_rect();
            let response = ui.allocate_rect(rect, egui::Sense::hover());
            self.draw_arena(ui, response.rect);
        });

        for idx in 0..self.players.len() {
            if sync_tactical_draft_on_open(
                player_editors_were_open[idx],
                self.show_player_editor[idx],
                &self.players[idx].tactical_policy,
                &mut self.tactical_drafts[idx],
            ) {
                self.tactical_pending_loads[idx] = None;
                self.tactical_confirm_overwrites[idx] = false;
                self.tactical_confirm_deletes[idx] = false;
            }
            let name = self.players[idx].name.clone();
            let player_names = [self.players[0].name.clone(), self.players[1].name.clone()];
            let mut open = self.show_player_editor[idx];
            let title = format!("Customize {name}");
            let mut run_dps_test = false;
            let mut tactics_applied = false;
            egui::Window::new(title)
                .id(egui::Id::new(format!("player_editor_{idx}")))
                .open(&mut open)
                .default_size(egui::vec2(560.0, 740.0))
                .resizable(true)
                .show(ctx, |ui| {
                    let id_prefix = if idx == 0 { "p1" } else { "p2" };
                    let fighter_preset_name = &mut self.fighter_preset_names[idx];
                    let damage_plot_iterations = &mut self.damage_plot_iterations[idx];
                    let damage_roll_plot = &mut self.damage_roll_plots[idx];
                    let tactical_draft = &mut self.tactical_drafts[idx];
                    let tactical_preset_index = &mut self.tactical_preset_indices[idx];
                    let tactical_preset_name = &mut self.tactical_preset_names[idx];
                    let tactical_message = &mut self.tactical_messages[idx];
                    let tactical_pending_load = &mut self.tactical_pending_loads[idx];
                    let tactical_confirm_overwrite = &mut self.tactical_confirm_overwrites[idx];
                    let tactical_confirm_delete = &mut self.tactical_confirm_deletes[idx];
                    let (player, opponent) = if idx == 0 {
                        let (left, right) = self.players.split_at_mut(1);
                        (&mut left[0], &right[0])
                    } else {
                        let (left, right) = self.players.split_at_mut(1);
                        (&mut right[0], &left[0])
                    };
                    render_player_editor(
                        ui,
                        id_prefix,
                        player,
                        &mut self.player_colors[idx],
                        opponent,
                        &self.weapon_catalog,
                        &self.armor_catalog,
                        &self.shield_catalog,
                        &self.race_catalog,
                        &self.talent_catalog,
                        &self.npc_presets,
                        &mut self.fighter_presets,
                        fighter_preset_name,
                        &mut self.player_editor_tabs[idx],
                        &mut self.talent_category_tabs[idx],
                        damage_plot_iterations,
                        damage_roll_plot,
                        &player_names,
                        &mut self.dps_attacker_idx,
                        &mut self.dps_defender_idx,
                        &mut self.dps_iterations,
                        &mut self.dps_duration_seconds,
                        &mut self.dps_seed,
                        &self.dps_result,
                        self.dps_sim_duration,
                        &mut run_dps_test,
                        tactical_draft,
                        &mut self.tactical_presets,
                        tactical_preset_index,
                        tactical_preset_name,
                        tactical_message,
                        tactical_pending_load,
                        tactical_confirm_overwrite,
                        tactical_confirm_delete,
                        self.sim.elapsed_seconds > 0,
                        &mut tactics_applied,
                    );
                });
            self.show_player_editor[idx] = open;
            if run_dps_test {
                self.running = false;
                self.run_dps_test();
            }
            if tactics_applied {
                self.running = false;
                self.reset_positions();
            }
        }

        if self.running {
            ctx.request_repaint();
        }
    }
}

fn render_player_editor_tabs(ui: &mut egui::Ui, id_prefix: &str, active_tab: &mut PlayerEditorTab) {
    ui.push_id(format!("{id_prefix}_tabs"), |ui| {
        ui.horizontal_wrapped(|ui| {
            for tab in PLAYER_EDITOR_TABS {
                ui.selectable_value(active_tab, tab, tab.label());
            }
        });
    });
    ui.separator();
}

fn sync_tactical_draft_on_open(
    was_open: bool,
    is_open: bool,
    active: &TacticalPolicy,
    draft: &mut TacticalPolicy,
) -> bool {
    if is_open && !was_open {
        *draft = active.clone();
        true
    } else {
        false
    }
}

fn apply_tactical_enabled_toggle(
    active: &mut TacticalPolicy,
    draft: &TacticalPolicy,
) -> Result<(), Vec<String>> {
    if draft.enabled {
        validate_policy(draft)?;
        *active = draft.clone();
    } else {
        active.enabled = false;
    }
    Ok(())
}

fn apply_tactical_ui_policy(
    active: &mut TacticalPolicy,
    draft: &TacticalPolicy,
) -> Result<bool, Vec<String>> {
    if active == draft {
        return Ok(false);
    }
    validate_policy(draft)?;
    *active = draft.clone();
    Ok(true)
}

fn render_tools_tab(
    ui: &mut egui::Ui,
    active_tool_tab: &mut ToolTab,
    wound_damage: &mut u32,
    days_until_point_healed: &mut f32,
    days: &mut u32,
    tended: &mut bool,
    fast_healer: &mut bool,
    maximum_essence: &mut u32,
    starting_essence: &mut u32,
    essence_change: &mut i64,
) {
    ui.heading("Tools");
    ui.horizontal(|ui| {
        ui.selectable_value(
            active_tool_tab,
            ToolTab::WoundHealing,
            ToolTab::WoundHealing.label(),
        );
        ui.selectable_value(
            active_tool_tab,
            ToolTab::EssenceWounds,
            ToolTab::EssenceWounds.label(),
        );
    });
    ui.separator();
    match *active_tool_tab {
        ToolTab::WoundHealing => {
            ui.heading("Wound Calculator");
            render_wound_calculator(
                ui,
                wound_damage,
                days_until_point_healed,
                days,
                tended,
                fast_healer,
            );
        }
        ToolTab::EssenceWounds => {
            ui.heading("Essence Wound Calculator");
            render_essence_wound_calculator(ui, maximum_essence, starting_essence, essence_change);
        }
    }
}

fn render_detailed_stats_tab(
    ui: &mut egui::Ui,
    players: &[PlayerConfig; 2],
    start_distance: f32,
    bulk_runs: &mut u32,
    bulk_seed: &mut u64,
    bulk_last_seed: Option<u64>,
    bulk_result: &Option<BulkSimResult>,
    bulk_sim_duration: Option<std::time::Duration>,
    run_bulk: &mut bool,
) {
    ui.heading("Detailed simulation statistics");
    ui.horizontal(|ui| {
        ui.label("Runs");
        ui.add(
            egui::DragValue::new(bulk_runs)
                .clamp_range(1..=u32::MAX)
                .speed(100.0),
        );
        ui.label("Seed");
        ui.add(egui::DragValue::new(bulk_seed).clamp_range(1..=u64::MAX));
        if ui.button("Run detailed simulation").clicked() {
            *run_bulk = true;
        }
        ui.separator();
        ui.label(format!("Start distance: {start_distance:.0} ft"));
    });
    ui.small(
        "The seed advances after each run. Enter an earlier seed to reproduce that exact batch.",
    );
    ui.separator();

    let Some(result) = bulk_result else {
        ui.label("No detailed result yet.");
        ui.label("Run the simulation to generate outcome, accuracy, damage, durability, trauma, and shield statistics.");
        return;
    };

    egui::ScrollArea::vertical().show(ui, |ui| {
        let total_runs = result.wins.iter().copied().sum::<u32>() + result.ties;
        ui.horizontal_wrapped(|ui| {
            ui.strong(format!("{total_runs} fights"));
            if let Some(seed) = bulk_last_seed {
                ui.label(format!("Seed {seed}"));
            }
            if let Some(duration) = bulk_sim_duration {
                ui.label(format!("Computed in {:.2}s", duration.as_secs_f64()));
            }
            if result.ties > 0 {
                ui.label(format!(
                    "Ties/timeouts: {} ({:.1}%)",
                    result.ties,
                    percent(u64::from(result.ties), u64::from(total_runs))
                ));
            }
        });
        ui.label(format!(
            "Fight duration — average {:.1}s | p10 {}s | median {}s | p90 {}s | p99 {}s",
            result.avg_duration,
            result.detailed.duration_p10,
            result.detailed.duration_p50,
            result.detailed.duration_p90,
            result.detailed.duration_p99,
        ));
        ui.small("Percentiles are more stable comparison points than shortest/longest-fight extremes.");
        ui.separator();

        ui.columns(2, |columns| {
            for idx in 0..2 {
                let ui = &mut columns[idx];
                ui.heading(players[idx].name.as_str());
                if let Some(stats) = result.detailed.teams.get(idx) {
                    render_detailed_team_stats(ui, stats, total_runs);
                } else {
                    ui.label("No team statistics available.");
                }
            }
        });

        ui.separator();
        egui::CollapsingHeader::new("Rare-event and movement diagnostics")
            .default_open(false)
            .show(ui, |ui| {
                ui.label(format!(
                    "Fights with trauma on first exchange: {} ({:.1}%)",
                    result.fights_with_trauma_first_exchange,
                    percent(
                        u64::from(result.fights_with_trauma_first_exchange),
                        u64::from(total_runs),
                    )
                ));
                ui.label(format!(
                    "Fights with 2+ charges: {} ({:.1}%)",
                    result.fights_with_second_charge,
                    percent(
                        u64::from(result.fights_with_second_charge),
                        u64::from(total_runs),
                    )
                ));
                ui.label(format!(
                    "Fights with a charge begun inside 20 ft: {} ({:.1}%)",
                    result.fights_with_charge_within_20ft,
                    percent(
                        u64::from(result.fights_with_charge_within_20ft),
                        u64::from(total_runs),
                    )
                ));
                ui.label(format!(
                    "Fights with 20+ ft knockback: {} ({:.1}%)",
                    result.fights_with_knockback_20ft,
                    percent(
                        u64::from(result.fights_with_knockback_20ft),
                        u64::from(total_runs),
                    )
                ));
                ui.label(format!(
                    "Average maximum one-side knockback: {:.1} ft | observed maximum: {:.1} ft",
                    result.avg_max_knockback_one_side_ft,
                    result.max_total_knockback_one_side_ft,
                ));
                ui.label(format!("Instant-kill criticals: {}", result.instakills));
            });

        ui.separator();
        ui.small("Definitions: direct hits beat the defence roll; shield blocks are attacks caught by the shield window; HP hits are attacks that remove at least 1 HP through either branch. Combat DPS divides total HP damage by total fight time, including closing time.");
    });
}

fn render_detailed_team_stats(ui: &mut egui::Ui, stats: &sim::DetailedTeamStats, total_runs: u32) {
    ui.strong("Outcome");
    ui.label(format!(
        "Wins: {} / {} ({:.1}%)",
        stats.wins,
        total_runs,
        stats.win_rate * 100.0,
    ));
    ui.label(format!(
        "95% confidence interval: {:.1}% – {:.1}%",
        stats.win_rate_ci_low * 100.0,
        stats.win_rate_ci_high * 100.0,
    ));
    match (stats.avg_winning_hp, stats.median_winning_hp) {
        (Some(average), Some(median)) => {
            ui.label(format!(
                "Winning HP: average {average:.1} | median {median}"
            ));
        }
        _ => {
            ui.label("Winning HP: n/a (no wins)");
        }
    }
    match (
        stats.avg_winning_duration_seconds,
        stats.median_winning_duration_seconds,
    ) {
        (Some(average), Some(median)) => {
            ui.label(format!(
                "Winning duration: average {average:.1}s | median {median}s"
            ));
        }
        _ => {
            ui.label("Winning duration: n/a (no wins)");
        }
    }

    ui.add_space(8.0);
    ui.strong("Offensive funnel");
    ui.label(format!(
        "Attacks: {} ({:.2} / fight)",
        stats.attack_attempts, stats.avg_attacks_per_fight,
    ));
    ui.label(format!(
        "Defence beaten: {} ({:.1}%)",
        stats.direct_hits,
        stats.direct_hit_rate * 100.0,
    ));
    ui.label(format!(
        "Shield blocks: {} ({:.1}%)",
        stats.shield_blocks,
        stats.shield_block_rate * 100.0,
    ));
    ui.label(format!(
        "Contact: {:.1}% | clean misses: {}",
        stats.contact_rate * 100.0,
        stats.misses,
    ));
    ui.label(format!(
        "HP-damaging hits: {} ({:.1}% of attacks)",
        stats.hp_hits,
        stats.hp_hit_rate * 100.0,
    ));
    ui.label(format!(
        "Criticals: {} ({:.2}% / attack, {:.2}% / direct hit)",
        stats.critical_hits,
        stats.critical_rate_per_attack * 100.0,
        stats.critical_rate_per_direct_hit * 100.0,
    ));
    ui.label(format!("Killing blows: {}", stats.kills));
    if stats.eyesmite_available {
        ui.label(format!("Eyes smote: {}", stats.eyes_smote));
    }

    ui.add_space(8.0);
    ui.strong("Damage output");
    ui.label(format!(
        "HP damage: {:.2} / fight | combat DPS: {:.3}",
        stats.avg_hp_damage_per_fight, stats.combat_dps,
    ));
    ui.label(format!(
        "HP damage: {:.2} / attack | {:.2} / damaging hit",
        stats.hp_damage_per_attack, stats.hp_damage_per_hp_hit,
    ));
    ui.label(format!(
        "Damaging-hit distribution: median {} | p90 {} | p99 {}",
        stats.hp_damage_p50, stats.hp_damage_p90, stats.hp_damage_p99,
    ));

    ui.add_space(8.0);
    ui.strong("Timing and control");
    ui.label(format!(
        "First attack: {}",
        optional_seconds(stats.avg_first_attack_seconds),
    ));
    ui.label(format!(
        "Attack interval: {}",
        optional_seconds(stats.avg_attack_interval_seconds),
    ));
    ui.label(format!(
        "Trauma inflicted: {} fights ({:.1}%), {} events",
        stats.fights_with_trauma_inflicted,
        stats.trauma_chance_per_fight * 100.0,
        stats.trauma_events_inflicted,
    ));
    ui.label(format!(
        "Trauma downtime suffered: {:.2}s / fight",
        stats.avg_trauma_downtime_seconds,
    ));

    ui.add_space(8.0);
    ui.strong("Durability pipeline (per fight)");
    ui.label(format!(
        "Incoming raw {:.2} → armour prevented {:.2} → shield prevented {:.2} → HP lost {:.2}",
        stats.incoming_raw_damage_per_fight,
        stats.armor_prevented_per_fight,
        stats.shield_prevented_per_fight,
        stats.hp_damage_taken_per_fight,
    ));
    ui.label(format!(
        "Raw damage prevented: {:.1}%",
        stats.damage_prevention_rate * 100.0,
    ));
    if let Some(break_rate) = stats.shield_break_rate {
        ui.label(format!(
            "Shield broke: {} / {} fights ({:.1}%)",
            stats.fights_with_shield_break,
            stats.shield_fights,
            break_rate * 100.0,
        ));
        if let (Some(average_hits), Some(median_hits)) = (
            stats.avg_hits_shield_survived_before_break,
            stats.median_hits_shield_survived_before_break,
        ) {
            ui.label(format!(
                "Hits survived before break: average {average_hits:.2} | median {median_hits}"
            ));
        }
        if let (Some(average_seconds), Some(median_seconds)) = (
            stats.avg_shield_break_seconds,
            stats.median_shield_break_seconds,
        ) {
            ui.label(format!(
                "Time until break: average {average_seconds:.2}s | median {median_seconds}s"
            ));
        }
    } else {
        ui.label("Shield break chance: n/a (no shield equipped)");
    }
}

fn percent(numerator: u64, denominator: u64) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 * 100.0 / denominator as f32
    }
}

fn optional_seconds(value: Option<f32>) -> String {
    value
        .map(|seconds| format!("{seconds:.2}s"))
        .unwrap_or_else(|| "n/a".to_string())
}

fn render_dps_test_tool(
    ui: &mut egui::Ui,
    player_names: &[String; 2],
    dps_attacker_idx: &mut usize,
    dps_defender_idx: &mut usize,
    dps_iterations: &mut u32,
    dps_duration_seconds: &mut u32,
    dps_seed: &mut u64,
    dps_result: &Option<DpsTestResult>,
    dps_sim_duration: Option<std::time::Duration>,
    run_dps_test: &mut bool,
) {
    ui.heading("DPS Test");
    *dps_attacker_idx = (*dps_attacker_idx).min(1);
    *dps_defender_idx = (*dps_defender_idx).min(1);
    if *dps_defender_idx == *dps_attacker_idx {
        *dps_defender_idx = 1usize.saturating_sub(*dps_attacker_idx);
    }
    let selected_attacker_idx = *dps_attacker_idx;
    let selected_defender_idx = *dps_defender_idx;
    ui.horizontal(|ui| {
        ui.label("Attacker");
        egui::ComboBox::from_id_source("dps_attacker")
            .selected_text(player_names[selected_attacker_idx].as_str())
            .show_ui(ui, |ui| {
                ui.selectable_value(dps_attacker_idx, 0, player_names[0].as_str());
                ui.selectable_value(dps_attacker_idx, 1, player_names[1].as_str());
            });
        if *dps_defender_idx == *dps_attacker_idx {
            *dps_defender_idx = 1usize.saturating_sub(*dps_attacker_idx);
        }
        ui.label("Defender");
        egui::ComboBox::from_id_source("dps_defender")
            .selected_text(player_names[selected_defender_idx].as_str())
            .show_ui(ui, |ui| {
                ui.selectable_value(dps_defender_idx, 0, player_names[0].as_str());
                ui.selectable_value(dps_defender_idx, 1, player_names[1].as_str());
            });
        if *dps_defender_idx == *dps_attacker_idx {
            *dps_attacker_idx = 1usize.saturating_sub(*dps_defender_idx);
        }
    });
    ui.small("Defender uses normal defense rolls, DR, shield blocks, and shield breakage, but does not attack and cannot die.");
    ui.horizontal(|ui| {
        ui.label("Iterations");
        ui.add(
            egui::DragValue::new(dps_iterations)
                .clamp_range(1..=u32::MAX)
                .speed(100.0),
        );
        ui.label("Duration (s)");
        ui.add(
            egui::DragValue::new(dps_duration_seconds)
                .clamp_range(1..=u32::MAX)
                .speed(5.0),
        );
        ui.label("Seed");
        ui.add(egui::DragValue::new(dps_seed).clamp_range(1..=u64::MAX));
    });
    if ui.button("Run DPS test").clicked() {
        *run_dps_test = true;
    }
    if let Some(result) = dps_result {
        let attacker_name = player_names[result.attacker_idx].as_str();
        let defender_name = player_names[result.defender_idx].as_str();
        ui.separator();
        ui.label(format!(
            "{attacker_name} attacking passive infinite-HP {defender_name}"
        ));
        ui.label(format!(
            "DPS: {:.2} over {} x {}s",
            result.dps, result.iterations, result.duration_seconds
        ));
        ui.label(format!(
            "Avg damage/run: {:.1} | Total damage: {}",
            result.avg_damage_per_run, result.total_damage
        ));
        ui.label(format!(
            "Avg attacks/run: {:.1} | Total attacks: {}",
            result.avg_attacks_per_run, result.attacks
        ));
        ui.label(format!(
            "Damage rolls: {} | Rolled avg: {:.1} | Landed avg: {:.1}",
            result.damage_rolls,
            if result.damage_rolls == 0 {
                0.0
            } else {
                result.total_rolled_damage as f64 / result.damage_rolls as f64
            },
            if result.damage_rolls == 0 {
                0.0
            } else {
                result.total_landed_damage as f64 / result.damage_rolls as f64
            }
        ));
        ui.label(format!(
            "Highest crit: {} | Highest non-crit: {} | Highest shield hit: {}",
            result.highest_crit_hit, result.highest_noncrit_hit, result.highest_shield_hit
        ));
        if result.instakills > 0 {
            ui.label(format!(
                "Instant-kill crits rolled: {} (target stayed alive)",
                result.instakills
            ));
        }
        if let Some(duration) = dps_sim_duration {
            ui.label(format!("Sim time: {:.2}s", duration.as_secs_f64()));
        }
    } else {
        ui.label("No DPS result yet.");
    }
}

fn render_wound_calculator(
    ui: &mut egui::Ui,
    wound_damage: &mut u32,
    days_until_point_healed: &mut f32,
    days: &mut u32,
    tended: &mut bool,
    fast_healer: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.label("Wound");
        ui.add(
            egui::DragValue::new(wound_damage)
                .clamp_range(0..=u32::MAX)
                .speed(1.0),
        );
        let current_required_steps = if *wound_damage == 0 {
            0
        } else {
            required_healing_steps(*wound_damage, *fast_healer)
        };
        let steps_per_day = healing_steps_per_day(*tended);
        let current_required_days = current_required_steps as f32 / steps_per_day as f32;
        ui.label("Days until point healed");
        ui.add(
            egui::DragValue::new(days_until_point_healed)
                .clamp_range(0.0..=current_required_days)
                .speed(0.25)
                .fixed_decimals(2),
        );
        ui.label("Additional days");
        ui.add(
            egui::DragValue::new(days)
                .clamp_range(0..=u32::MAX)
                .speed(1.0),
        );
    });
    ui.horizontal(|ui| {
        ui.checkbox(tended, "Being tended");
        ui.checkbox(fast_healer, "Fast Healer");
    });
    ui.small("Being tended/resting gives 4 healing steps per day. Untended activity gives half progress: 2 steps per day.");
    ui.small("Fast Healer reduces the time for each wound point by half a day; the final wound point heals in 1 step.");
    ui.small("For a 9 point wound with 2 tended days left before it drops to 8, enter wound 9 and days until point healed 2.");
    ui.separator();

    let mut wounds = if *wound_damage == 0 {
        Vec::new()
    } else {
        let required_steps = required_healing_steps(*wound_damage, *fast_healer);
        let steps_per_day = healing_steps_per_day(*tended);
        let max_days_until_point_healed = required_steps as f32 / steps_per_day as f32;
        *days_until_point_healed =
            (*days_until_point_healed).clamp(0.0, max_days_until_point_healed);
        let remaining_steps =
            ((*days_until_point_healed * steps_per_day as f32).round() as u32).min(required_steps);
        let healing_progress_steps = required_steps.saturating_sub(remaining_steps);
        vec![Wound {
            damage: *wound_damage,
            healing_progress_steps,
        }]
    };
    heal_wounds(&mut wounds, *days, *fast_healer, *tended);
    let remaining = wounds.first().map(|wound| wound.damage).unwrap_or(0);
    let healed = wound_damage.saturating_sub(remaining);

    ui.label(format!("Healed: {healed}"));
    ui.label(format!("Remaining wound: {remaining}"));
    if let Some(wound) = wounds.first() {
        let required_steps = required_healing_steps(wound.damage, *fast_healer);
        let progress_steps = wound.healing_progress_steps.min(required_steps);
        let next_heal = required_steps.saturating_sub(progress_steps);
        let steps_per_day = healing_steps_per_day(*tended);
        let progress_days = progress_steps as f32 / steps_per_day as f32;
        let required_days = required_steps as f32 / steps_per_day as f32;
        let next_heal_days = next_heal as f32 / steps_per_day as f32;
        ui.label(format!(
            "Current progress: {} / {} days",
            format_healing_days(progress_days),
            format_healing_days(required_days)
        ));
        ui.label(format!(
            "Next point heals in {} days",
            format_healing_days(next_heal_days)
        ));
    } else {
        ui.label("The wound is fully healed.");
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EssenceWoundResult {
    current_essence: u32,
    disorder_wound: u32,
    solar_wound: u32,
    lunar_wound: u32,
}

fn render_essence_wound_calculator(
    ui: &mut egui::Ui,
    maximum_essence: &mut u32,
    starting_essence: &mut u32,
    essence_change: &mut i64,
) {
    ui.horizontal(|ui| {
        ui.label("Maximum Essence");
        ui.add(
            egui::DragValue::new(maximum_essence)
                .clamp_range(0..=u32::MAX)
                .speed(10.0),
        );
        *starting_essence = (*starting_essence).min(*maximum_essence);
        ui.label("Starting Essence");
        ui.add(
            egui::DragValue::new(starting_essence)
                .clamp_range(0..=*maximum_essence)
                .speed(10.0),
        );
        ui.label("Change");
        ui.add(egui::DragValue::new(essence_change).speed(10.0));
    });

    let result = calculate_essence_wounds(*maximum_essence, *starting_essence, *essence_change);
    ui.small(format!(
        "Equilibrium: {}. Disorder Wound increases by 1 for each full 50 Essence from equilibrium.",
        format_equilibrium(*maximum_essence)
    ));
    ui.small("The resulting Current Essence is limited to the range from 0 to Maximum Essence.");
    ui.separator();

    ui.label(format!("Current Essence: {}", result.current_essence));
    ui.label(format!("Current Disorder Wound: {}", result.disorder_wound));
    if result.solar_wound > 0 {
        ui.label(format!("Solar Wound caused: {}", result.solar_wound));
    } else if result.lunar_wound > 0 {
        ui.label(format!("Lunar Wound caused: {}", result.lunar_wound));
    } else {
        ui.label("No Solar or Lunar Wound caused.");
    }
}

fn calculate_essence_wounds(
    maximum_essence: u32,
    starting_essence: u32,
    change: i64,
) -> EssenceWoundResult {
    let starting_essence = starting_essence.min(maximum_essence);
    let current_essence = (i128::from(starting_essence) + i128::from(change))
        .clamp(0, i128::from(maximum_essence)) as u32;
    let starting_disorder = disorder_wound(maximum_essence, starting_essence);
    let current_disorder = disorder_wound(maximum_essence, current_essence);
    let disorder_change = starting_disorder.abs_diff(current_disorder);
    let starts_below_equilibrium = u64::from(starting_essence) * 2 < u64::from(maximum_essence);
    let starts_above_equilibrium = u64::from(starting_essence) * 2 > u64::from(maximum_essence);

    EssenceWoundResult {
        current_essence,
        disorder_wound: current_disorder,
        solar_wound: if starts_below_equilibrium && current_essence > starting_essence {
            disorder_change
        } else {
            0
        },
        lunar_wound: if starts_above_equilibrium && current_essence < starting_essence {
            disorder_change
        } else {
            0
        },
    }
}

fn disorder_wound(maximum_essence: u32, essence: u32) -> u32 {
    let doubled_distance =
        (u64::from(essence.min(maximum_essence)) * 2).abs_diff(u64::from(maximum_essence));
    (doubled_distance / 100) as u32
}

fn format_equilibrium(maximum_essence: u32) -> String {
    if maximum_essence % 2 == 0 {
        (maximum_essence / 2).to_string()
    } else {
        format!("{}.5", maximum_essence / 2)
    }
}

fn healing_steps_per_day(tended: bool) -> u32 {
    if tended { 4 } else { 2 }
}

fn format_healing_days(days: f32) -> String {
    let mut formatted = format!("{days:.2}");
    while formatted.contains('.') && formatted.ends_with('0') {
        formatted.pop();
    }
    if formatted.ends_with('.') {
        formatted.pop();
    }
    formatted
}

fn derived_breakdown_icon(ui: &mut egui::Ui, breakdown: Option<&game_logic::StatBreakdown>) {
    let Some(breakdown) = breakdown else {
        return;
    };
    let (rect, response) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
    let visuals = ui.style().interact(&response);
    ui.painter().circle_stroke(
        rect.center(),
        5.5,
        egui::Stroke::new(1.0, visuals.fg_stroke.color),
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "?",
        egui::FontId::proportional(9.0),
        visuals.fg_stroke.color,
    );
    if response.hovered() {
        egui::show_tooltip_for(
            ui.ctx(),
            response.id.with("__derived_breakdown"),
            &response.rect,
            |ui| {
                ui.set_max_width(420.0);
                ui.label(egui::RichText::new(format!("Result: {}", breakdown.result)).strong());
                if !breakdown.lines.is_empty() {
                    ui.separator();
                    egui::Grid::new(ui.next_auto_id())
                        .num_columns(2)
                        .spacing([10.0, 3.0])
                        .show(ui, |ui| {
                            for line in &breakdown.lines {
                                ui.monospace(&line.value);
                                ui.label(&line.source);
                                ui.end_row();
                            }
                        });
                }
                if !breakdown.notes.is_empty() {
                    ui.separator();
                    for note in &breakdown.notes {
                        ui.label(note);
                    }
                }
            },
        );
    }
}

fn derived_stat_line(
    ui: &mut egui::Ui,
    text: impl Into<egui::WidgetText>,
    breakdown: Option<&game_logic::StatBreakdown>,
) {
    ui.horizontal(|ui| {
        ui.label(text);
        derived_breakdown_icon(ui, breakdown);
    });
}

fn numeric_comparison_editor(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash,
    comparison: &mut NumericComparison,
) {
    egui::ComboBox::from_id_source(id)
        .selected_text(comparison.label())
        .show_ui(ui, |ui| {
            for option in NumericComparison::ALL {
                ui.selectable_value(comparison, option, option.label());
            }
        });
}

fn boolean_condition_editor(ui: &mut egui::Ui, id: impl std::hash::Hash, value: &mut bool) {
    egui::ComboBox::from_id_source(id)
        .selected_text(if *value { "True" } else { "False" })
        .show_ui(ui, |ui| {
            ui.selectable_value(value, true, "True");
            ui.selectable_value(value, false, "False");
        });
}

fn tactical_condition_editor(
    ui: &mut egui::Ui,
    id_prefix: &str,
    condition: &mut TacticalCondition,
    style_options: &[String],
) -> bool {
    let mut remove = false;
    ui.horizontal_wrapped(|ui| {
        let mut kind = match condition {
            TacticalCondition::Always => 0,
            TacticalCondition::MyHpPercent { .. } => 1,
            TacticalCondition::EnemyHpPercent { .. } => 2,
            TacticalCondition::DistanceFt { .. } => 3,
            TacticalCondition::ReachComparedToEnemy { .. } => 4,
            TacticalCondition::RetreatSpaceAvailable { .. } => 5,
            TacticalCondition::MyWeaponCanJab { .. } => 6,
            TacticalCondition::MyHasActiveShield { .. } => 7,
            TacticalCondition::EnemyWeaponGroup { .. } => 8,
            TacticalCondition::EnemyHasActiveShield { .. } => 9,
            TacticalCondition::EnemyArmorType { .. } => 10,
            TacticalCondition::EnemyCharging { .. } => 11,
            TacticalCondition::MyHasAttacked { .. } => 12,
            TacticalCondition::EnemyTimeToReachSeconds { .. } => 13,
            TacticalCondition::MyActiveStyle { .. } => 14,
            TacticalCondition::EnemyActiveStyle { .. } => 15,
            TacticalCondition::EnemyDr { .. } => 16,
            TacticalCondition::EnemyAttackSpeedSeconds { .. } => 17,
            TacticalCondition::EnemyAttackSpeedComparedToMine { .. } => 18,
        };
        let old_kind = kind;
        let labels = [
            "Always",
            "My HP %",
            "Enemy HP %",
            "Distance (ft)",
            "My reach vs enemy",
            "Retreat space",
            "My weapon can Jab",
            "My shield active",
            "Enemy weapon group",
            "Enemy shield active",
            "Enemy armor type",
            "Enemy charging",
            "I have attacked this combat",
            "Enemy time to reach (seconds)",
            "My active style",
            "Enemy active style",
            "Enemy DR",
            "Enemy attack speed (seconds)",
            "Enemy speed vs mine",
        ];
        egui::ComboBox::from_id_source(format!("{id_prefix}_kind"))
            .selected_text(labels[kind])
            .show_ui(ui, |ui| {
                for (idx, label) in labels.iter().enumerate() {
                    ui.selectable_value(&mut kind, idx, *label);
                }
            });
        if kind != old_kind {
            *condition = match kind {
                1 => TacticalCondition::MyHpPercent {
                    comparison: NumericComparison::LessOrEqual,
                    value: 50.0,
                },
                2 => TacticalCondition::EnemyHpPercent {
                    comparison: NumericComparison::LessOrEqual,
                    value: 50.0,
                },
                3 => TacticalCondition::DistanceFt {
                    comparison: NumericComparison::LessOrEqual,
                    value: 10.0,
                },
                4 => TacticalCondition::ReachComparedToEnemy {
                    comparison: RelativeComparison::Greater,
                },
                5 => TacticalCondition::RetreatSpaceAvailable { value: true },
                6 => TacticalCondition::MyWeaponCanJab { value: true },
                7 => TacticalCondition::MyHasActiveShield { value: true },
                8 => TacticalCondition::EnemyWeaponGroup {
                    value: "Polearms".to_string(),
                    negated: false,
                },
                9 => TacticalCondition::EnemyHasActiveShield { value: true },
                10 => TacticalCondition::EnemyArmorType {
                    value: "Heavy".to_string(),
                    negated: false,
                },
                11 => TacticalCondition::EnemyCharging { value: true },
                12 => TacticalCondition::MyHasAttacked { value: true },
                13 => TacticalCondition::EnemyTimeToReachSeconds {
                    comparison: NumericComparison::Greater,
                    value: 3.0,
                },
                14 => TacticalCondition::MyActiveStyle {
                    style_id: String::new(),
                    negated: false,
                },
                15 => TacticalCondition::EnemyActiveStyle {
                    style_id: String::new(),
                    negated: false,
                },
                16 => TacticalCondition::EnemyDr {
                    comparison: NumericComparison::GreaterOrEqual,
                    value: 5.0,
                },
                17 => TacticalCondition::EnemyAttackSpeedSeconds {
                    comparison: NumericComparison::LessOrEqual,
                    value: 8.0,
                },
                18 => TacticalCondition::EnemyAttackSpeedComparedToMine {
                    comparison: SpeedComparison::Faster,
                },
                _ => TacticalCondition::Always,
            };
        }
        match condition {
            TacticalCondition::Always => {}
            TacticalCondition::MyHpPercent { comparison, value }
            | TacticalCondition::EnemyHpPercent { comparison, value } => {
                numeric_comparison_editor(ui, format!("{id_prefix}_cmp"), comparison);
                ui.add(
                    egui::DragValue::new(value)
                        .clamp_range(0.0..=100.0)
                        .suffix("%"),
                );
            }
            TacticalCondition::DistanceFt { comparison, value }
            | TacticalCondition::EnemyDr { comparison, value }
            | TacticalCondition::EnemyAttackSpeedSeconds { comparison, value }
            | TacticalCondition::EnemyTimeToReachSeconds { comparison, value } => {
                numeric_comparison_editor(ui, format!("{id_prefix}_cmp"), comparison);
                ui.add(egui::DragValue::new(value).clamp_range(0.0..=1000.0));
            }
            TacticalCondition::ReachComparedToEnemy { comparison } => {
                egui::ComboBox::from_id_source(format!("{id_prefix}_relative"))
                    .selected_text(match comparison {
                        RelativeComparison::Less => "shorter",
                        RelativeComparison::Equal => "equal",
                        RelativeComparison::Greater => "longer",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(comparison, RelativeComparison::Less, "shorter");
                        ui.selectable_value(comparison, RelativeComparison::Equal, "equal");
                        ui.selectable_value(comparison, RelativeComparison::Greater, "longer");
                    });
            }
            TacticalCondition::RetreatSpaceAvailable { value }
            | TacticalCondition::MyWeaponCanJab { value }
            | TacticalCondition::MyHasActiveShield { value }
            | TacticalCondition::EnemyHasActiveShield { value }
            | TacticalCondition::EnemyCharging { value }
            | TacticalCondition::MyHasAttacked { value } => {
                boolean_condition_editor(ui, format!("{id_prefix}_bool"), value);
            }
            TacticalCondition::EnemyWeaponGroup { value, negated } => {
                ui.checkbox(negated, "not");
                const GROUPS: [&str; 13] = [
                    "Unarmed",
                    "Axes",
                    "Basic",
                    "Blunt",
                    "Bows",
                    "Crossbows",
                    "Double",
                    "Ensnaring",
                    "Lashes",
                    "LargeSwords",
                    "SmallSwords",
                    "Polearms",
                    "Spears",
                ];
                egui::ComboBox::from_id_source(format!("{id_prefix}_weapon_group"))
                    .selected_text(value.as_str())
                    .show_ui(ui, |ui| {
                        for group in GROUPS {
                            ui.selectable_value(value, group.to_string(), group);
                        }
                    });
            }
            TacticalCondition::EnemyArmorType { value, negated } => {
                ui.checkbox(negated, "not");
                egui::ComboBox::from_id_source(format!("{id_prefix}_armor_type"))
                    .selected_text(value.as_str())
                    .show_ui(ui, |ui| {
                        for armor_type in ["None", "Light", "Medium", "Heavy"] {
                            ui.selectable_value(value, armor_type.to_string(), armor_type);
                        }
                    });
            }
            TacticalCondition::MyActiveStyle { style_id, negated }
            | TacticalCondition::EnemyActiveStyle { style_id, negated } => {
                ui.checkbox(negated, "not");
                egui::ComboBox::from_id_source(format!("{id_prefix}_active_style"))
                    .selected_text(if style_id.is_empty() {
                        "Select style"
                    } else {
                        style_id.as_str()
                    })
                    .show_ui(ui, |ui| {
                        for style in style_options {
                            ui.selectable_value(style_id, style.clone(), style);
                        }
                    });
            }
            TacticalCondition::EnemyAttackSpeedComparedToMine { comparison } => {
                egui::ComboBox::from_id_source(format!("{id_prefix}_speed_relative"))
                    .selected_text(match comparison {
                        SpeedComparison::Faster => "faster",
                        SpeedComparison::Equal => "equal",
                        SpeedComparison::Slower => "slower",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(comparison, SpeedComparison::Faster, "faster");
                        ui.selectable_value(comparison, SpeedComparison::Equal, "equal");
                        ui.selectable_value(comparison, SpeedComparison::Slower, "slower");
                    });
            }
        }
        if ui.small_button("Remove condition").clicked() {
            remove = true;
        }
    });
    remove
}

fn tactical_action_editor(
    ui: &mut egui::Ui,
    id_prefix: &str,
    action: &mut TacticalAction,
    learned_styles: &[String],
    compatible_styles: &[String],
    pair_allowed: bool,
) {
    let mut kind = match action {
        TacticalAction::RetainWeaponStyle => 0,
        TacticalAction::NeutralWeaponStyle => 1,
        TacticalAction::UseWeaponStyle { .. } => 2,
        TacticalAction::NormalAttack => 3,
        TacticalAction::Jab => 4,
        TacticalAction::NeutralStance => 5,
        TacticalAction::FightDefensively { .. } => 6,
        TacticalAction::StandGround => 7,
        TacticalAction::GiveGround => 8,
    };
    let old_kind = kind;
    let labels = [
        "Retain style",
        "No weapon style",
        "Use learned style",
        "Normal attack",
        "Jab",
        "Neutral stance",
        "Fight defensively",
        "Stand ground",
        "Give Ground",
    ];
    egui::ComboBox::from_id_source(format!("{id_prefix}_action"))
        .selected_text(labels[kind])
        .show_ui(ui, |ui| {
            for (idx, label) in labels.iter().enumerate() {
                ui.selectable_value(&mut kind, idx, *label);
            }
        });
    if kind != old_kind {
        *action = match kind {
            0 => TacticalAction::RetainWeaponStyle,
            1 => TacticalAction::NeutralWeaponStyle,
            2 => TacticalAction::UseWeaponStyle {
                style_ids: compatible_styles.first().cloned().into_iter().collect(),
            },
            3 => TacticalAction::NormalAttack,
            4 => TacticalAction::Jab,
            5 => TacticalAction::NeutralStance,
            6 => TacticalAction::FightDefensively { penalty: 2 },
            7 => TacticalAction::StandGround,
            _ => TacticalAction::GiveGround,
        };
    }
    match action {
        TacticalAction::FightDefensively { penalty } => {
            egui::ComboBox::from_id_source(format!("{id_prefix}_stance_penalty"))
                .selected_text(format!("-{penalty}/+{}", *penalty / 2))
                .show_ui(ui, |ui| {
                    for option in [2, 4, 6, 8] {
                        ui.selectable_value(penalty, option, format!("-{option}/+{}", option / 2));
                    }
                });
        }
        TacticalAction::UseWeaponStyle { style_ids } => {
            let mut selected = style_ids.join(" + ");
            egui::ComboBox::from_id_source(format!("{id_prefix}_style"))
                .selected_text(if selected.is_empty() {
                    "No compatible learned style"
                } else {
                    selected.as_str()
                })
                .show_ui(ui, |ui| {
                    for style in learned_styles {
                        let compatible = compatible_styles
                            .iter()
                            .any(|available| available.eq_ignore_ascii_case(style));
                        ui.add_enabled_ui(compatible, |ui| {
                            let label = if compatible {
                                style.clone()
                            } else {
                                format!("{style} — unavailable with current equipment")
                            };
                            ui.selectable_value(&mut selected, style.clone(), label);
                        });
                    }
                    if pair_allowed {
                        ui.selectable_value(
                            &mut selected,
                            "shield_of_blades + storm_of_blades".to_string(),
                            "shield_of_blades + storm_of_blades",
                        );
                    }
                });
            *style_ids = if selected.contains(" + ") {
                selected.split(" + ").map(str::to_string).collect()
            } else if selected.is_empty() {
                Vec::new()
            } else {
                vec![selected]
            };
        }
        _ => {}
    }
}

fn tactical_compatibility_warning(
    action: &TacticalAction,
    learned_styles: &[String],
    compatible_styles: &[String],
    pair_allowed: bool,
    weapon_can_jab: bool,
) -> Option<String> {
    match action {
        TacticalAction::Jab if !weapon_can_jab => {
            Some("Current weapon cannot Jab; this rule will be skipped.".to_string())
        }
        TacticalAction::UseWeaponStyle { style_ids }
            if style_ids.is_empty()
                || style_ids.iter().any(|style| {
                    !learned_styles
                        .iter()
                        .any(|learned| learned.eq_ignore_ascii_case(style))
                })
                || style_ids.iter().any(|style| {
                    !compatible_styles
                        .iter()
                        .any(|available| available.eq_ignore_ascii_case(style))
                })
                || (style_ids.len() == 2 && !pair_allowed) =>
        {
            Some(
                "Style is not compatible with this character; the rule is preserved but skipped."
                    .to_string(),
            )
        }
        _ => None,
    }
}

fn weapon_style_display_name(style_id: &str, talent_catalog: &TalentCatalog) -> String {
    talent_catalog
        .entries()
        .iter()
        .find(|talent| talent.id.eq_ignore_ascii_case(style_id))
        .map(|talent| talent.name.clone())
        .unwrap_or_else(|| style_id.to_string())
}

fn weapon_style_selection_label(style_ids: &[String], talent_catalog: &TalentCatalog) -> String {
    if style_ids.is_empty() {
        "No weapon style".to_string()
    } else {
        style_ids
            .iter()
            .map(|style| weapon_style_display_name(style, talent_catalog))
            .collect::<Vec<_>>()
            .join(" + ")
    }
}

fn render_default_weapon_style_selector(
    ui: &mut egui::Ui,
    id_prefix: &str,
    player: &mut PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
    talent_catalog: &TalentCatalog,
) {
    let learned = game_logic::learned_weapon_style_ids(player, talent_catalog);
    let compatible = game_logic::compatible_weapon_style_ids(
        player,
        talent_catalog,
        weapon_catalog,
        armor_catalog,
        shield_catalog,
    );
    let effective = game_logic::effective_default_weapon_style_ids(
        player,
        talent_catalog,
        weapon_catalog,
        armor_catalog,
        shield_catalog,
    );
    let inferred =
        game_logic::inferred_legacy_weapon_style_ids(player, talent_catalog, weapon_catalog);
    let selected = player
        .default_weapon_style_ids
        .clone()
        .map(hackmaster_sim::core::tactics::canonicalize_style_selection)
        .unwrap_or_else(|| inferred.clone());
    let pair_learned = learned
        .iter()
        .any(|style| style.eq_ignore_ascii_case("shield_of_blades"))
        && learned
            .iter()
            .any(|style| style.eq_ignore_ascii_case("storm_of_blades"))
        && game_logic::has_perfect_two_weapon_fighting_effect(player);
    let pair_compatible = game_logic::tactical_style_pair_allowed(player, &compatible);
    let selected_compatible = selected == effective;
    let selector_label = if player.tactical_policy.enabled {
        "Opening style"
    } else {
        "Default style"
    };

    ui.group(|ui| {
        ui.strong("Weapon Style");
        ui.horizontal_wrapped(|ui| {
            ui.label(selector_label);
            egui::ComboBox::from_id_source(format!("{id_prefix}_default_weapon_style"))
                .selected_text(weapon_style_selection_label(&selected, talent_catalog))
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(selected.is_empty(), "No weapon style")
                        .clicked()
                    {
                        player.default_weapon_style_ids = Some(Vec::new());
                    }
                    for style in &learned {
                        let available = compatible
                            .iter()
                            .any(|candidate| candidate.eq_ignore_ascii_case(style));
                        let is_selected = selected.len() == 1
                            && selected[0].eq_ignore_ascii_case(style);
                        let name = weapon_style_display_name(style, talent_catalog);
                        ui.add_enabled_ui(available, |ui| {
                            let label = if available {
                                name
                            } else {
                                format!("{name} — unavailable with current equipment")
                            };
                            if ui.selectable_label(is_selected, label).clicked() {
                                player.default_weapon_style_ids = Some(vec![style.clone()]);
                            }
                        });
                    }
                    if pair_learned {
                        let pair = vec![
                            "shield_of_blades".to_string(),
                            "storm_of_blades".to_string(),
                        ];
                        let is_selected = selected == pair;
                        ui.add_enabled_ui(pair_compatible, |ui| {
                            let label = if pair_compatible {
                                "Shield of Blades + Storm of Blades"
                            } else {
                                "Shield of Blades + Storm of Blades — unavailable"
                            };
                            if ui.selectable_label(is_selected, label).clicked() {
                                player.default_weapon_style_ids = Some(pair);
                            }
                        });
                    }
                });
        });
        if player.tactical_policy.enabled {
            ui.small(
                "This style is active before combat and remains active until a directive changes it at an attack opportunity.",
            );
        } else {
            ui.small("This style remains active throughout combat.");
        }
        if player.default_weapon_style_ids.is_none() && !inferred.is_empty() {
            ui.small(format!(
                "Legacy preset default: {}. Choosing an option will save it explicitly.",
                weapon_style_selection_label(&inferred, talent_catalog)
            ));
        }
        if !selected_compatible {
            ui.colored_label(
                Color32::YELLOW,
                "The selected style is learned but incompatible with the current equipment. Combat falls back to no style.",
            );
        }
        if learned.is_empty() {
            ui.small("Learn weapon styles from the Talents tab to make them available here.");
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn render_tactics_editor(
    ui: &mut egui::Ui,
    id_prefix: &str,
    player: &mut PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
    talent_catalog: &TalentCatalog,
    draft: &mut TacticalPolicy,
    presets: &mut Vec<TacticalPreset>,
    preset_index: &mut usize,
    preset_name: &mut String,
    message: &mut Option<String>,
    pending_load: &mut Option<usize>,
    confirm_overwrite: &mut bool,
    confirm_delete: &mut bool,
    locked: bool,
    applied: &mut bool,
) {
    let learned_styles = game_logic::learned_weapon_style_ids(player, talent_catalog);
    let compatible_styles = game_logic::compatible_weapon_style_ids(
        player,
        talent_catalog,
        weapon_catalog,
        armor_catalog,
        shield_catalog,
    );
    let style_options = talent_catalog
        .entries()
        .iter()
        .filter(|talent| game_logic::is_weapon_style_category(&talent.category))
        .map(|talent| talent.id.clone())
        .collect::<Vec<_>>();
    let pair_allowed = game_logic::tactical_style_pair_allowed(player, &compatible_styles);
    let weapon_can_jab = weapon_catalog
        .get(player.weapon_id)
        .map(|weapon| weapon.jab_speed.is_some())
        .unwrap_or(false);

    ui.heading("Tactical Directives");
    ui.label("Rules are checked in order. The first legal match in each channel wins.");
    ui.small("Attack speed is measured in seconds: lower values are faster.");
    ui.small(
        "Enemy time to reach uses current distance, enemy melee reach, and current movement speed.",
    );
    if locked {
        ui.colored_label(
            Color32::YELLOW,
            "Editing is locked after combat begins. Reset the simulation to edit tactics.",
        );
    }
    if let Some(text) = message.as_ref() {
        ui.colored_label(Color32::LIGHT_RED, text);
    }

    ui.add_enabled_ui(!locked, |ui| {
        let enabled_changed = ui
            .checkbox(&mut draft.enabled, "Enable Tactical Directives")
            .changed();
        if enabled_changed {
            match apply_tactical_enabled_toggle(&mut player.tactical_policy, draft) {
                Ok(()) => {
                    *message = Some(if draft.enabled {
                        "Tactical directives enabled and applied. Existing simulation results were kept."
                            .to_string()
                    } else {
                        "Tactical directives disabled immediately. Existing simulation results were kept."
                            .to_string()
                    });
                    *applied = true;
                }
                Err(errors) => {
                    draft.enabled = player.tactical_policy.enabled;
                    *message = Some(errors.join(" "));
                }
            }
        }
        ui.small(format!(
            "Live combat state: {}",
            if player.tactical_policy.enabled {
                "enabled"
            } else {
                "disabled"
            }
        ));
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.label("Preset");
            if presets.is_empty() {
                ui.label("No presets available");
            } else {
                *preset_index = (*preset_index).min(presets.len() - 1);
                egui::ComboBox::from_id_source(format!("{id_prefix}_tactical_preset"))
                    .selected_text(&presets[*preset_index].name)
                    .show_ui(ui, |ui| {
                        for (idx, preset) in presets.iter().enumerate() {
                            ui.selectable_value(preset_index, idx, &preset.name);
                        }
                    });
                if ui.button("Load").clicked() {
                    if *draft != player.tactical_policy {
                        *pending_load = Some(*preset_index);
                    } else {
                        let preset = &presets[*preset_index];
                        draft.rules = preset.rules.clone();
                        if let Some(opening_style_ids) = &preset.opening_style_ids {
                            player.default_weapon_style_ids = Some(opening_style_ids.clone());
                        }
                        *message = Some(format!(
                            "Loaded preset '{}'. Its tactics apply immediately.",
                            preset.name
                        ));
                        *applied = true;
                    }
                }
            }
        });
        if let Some(load_index) = *pending_load {
            ui.horizontal(|ui| {
                ui.colored_label(Color32::YELLOW, "Discard unsaved draft and load preset?");
                if ui.button("Discard & Load").clicked() {
                    if let Some(preset) = presets.get(load_index) {
                        draft.rules = preset.rules.clone();
                        if let Some(opening_style_ids) = &preset.opening_style_ids {
                            player.default_weapon_style_ids = Some(opening_style_ids.clone());
                        }
                        *message = Some(format!(
                            "Loaded preset '{}'. Its tactics apply immediately.",
                            preset.name
                        ));
                        *applied = true;
                    }
                    *pending_load = None;
                }
                if ui.button("Keep Draft").clicked() {
                    *pending_load = None;
                }
            });
        }

        ui.horizontal_wrapped(|ui| {
            ui.label("Save as");
            ui.text_edit_singleline(preset_name);
            if ui.button("Save preset").clicked() {
                let name = preset_name.trim();
                if let Err(errors) = validate_policy(draft) {
                    *message = Some(errors.join(" "));
                } else if name.is_empty() {
                    *message = Some("Enter a preset name.".to_string());
                } else if presets
                    .iter()
                    .any(|preset| preset.name.eq_ignore_ascii_case(name))
                {
                    *confirm_overwrite = true;
                } else {
                    let previous = presets.clone();
                    presets.push(TacticalPreset {
                        name: name.to_string(),
                        opening_style_ids: Some(
                            player.default_weapon_style_ids.clone().unwrap_or_else(|| {
                                game_logic::inferred_legacy_weapon_style_ids(
                                    player,
                                    talent_catalog,
                                    weapon_catalog,
                                )
                            }),
                        ),
                        rules: draft.rules.clone(),
                    });
                    match data::save_tactical_presets(TACTICAL_PRESETS_PATH, presets) {
                        Ok(()) => {
                            *preset_index = presets.len() - 1;
                            *message = Some(format!("Saved preset '{name}'."));
                        }
                        Err(err) => {
                            *presets = previous;
                            *message = Some(format!("Could not save preset: {err}"));
                        }
                    }
                }
            }
            if !presets.is_empty() && ui.button("Rename selected").clicked() {
                let name = preset_name.trim();
                let duplicate = presets.iter().enumerate().any(|(idx, preset)| {
                    idx != *preset_index && preset.name.eq_ignore_ascii_case(name)
                });
                if name.is_empty() || duplicate {
                    *message = Some("Rename needs a non-empty, unique name.".to_string());
                } else {
                    let previous = presets.clone();
                    presets[*preset_index].name = name.to_string();
                    if let Err(err) = data::save_tactical_presets(TACTICAL_PRESETS_PATH, presets) {
                        *presets = previous;
                        *message = Some(format!("Could not rename preset: {err}"));
                    } else {
                        *message = Some(format!("Renamed preset to '{name}'."));
                    }
                }
            }
            if !presets.is_empty() && ui.button("Delete selected").clicked() {
                *confirm_delete = true;
            }
        });
        if *confirm_overwrite {
            ui.horizontal(|ui| {
                ui.colored_label(Color32::YELLOW, "Overwrite the existing preset?");
                if ui.button("Overwrite").clicked() {
                    let previous = presets.clone();
                    if let Some(existing) = presets
                        .iter_mut()
                        .find(|preset| preset.name.eq_ignore_ascii_case(preset_name.trim()))
                    {
                        existing.opening_style_ids =
                            Some(player.default_weapon_style_ids.clone().unwrap_or_else(|| {
                                game_logic::inferred_legacy_weapon_style_ids(
                                    player,
                                    talent_catalog,
                                    weapon_catalog,
                                )
                            }));
                        existing.rules = draft.rules.clone();
                    }
                    match data::save_tactical_presets(TACTICAL_PRESETS_PATH, presets) {
                        Ok(()) => *message = Some("Preset overwritten.".to_string()),
                        Err(err) => {
                            *presets = previous;
                            *message = Some(format!("Could not overwrite preset: {err}"));
                        }
                    }
                    *confirm_overwrite = false;
                }
                if ui.button("Cancel").clicked() {
                    *confirm_overwrite = false;
                }
            });
        }
        if *confirm_delete && !presets.is_empty() {
            ui.horizontal(|ui| {
                ui.colored_label(Color32::YELLOW, "Delete the selected preset?");
                if ui.button("Delete permanently").clicked() {
                    let previous = presets.clone();
                    let removed_name = presets[*preset_index].name.clone();
                    presets.remove(*preset_index);
                    match data::save_tactical_presets(TACTICAL_PRESETS_PATH, presets) {
                        Ok(()) => {
                            *preset_index = (*preset_index).min(presets.len().saturating_sub(1));
                            *message = Some(format!("Deleted preset '{removed_name}'."));
                        }
                        Err(err) => {
                            *presets = previous;
                            *message = Some(format!("Could not delete preset: {err}"));
                        }
                    }
                    *confirm_delete = false;
                }
                if ui.button("Cancel").clicked() {
                    *confirm_delete = false;
                }
            });
        }

        ui.separator();
        enum RuleEdit {
            MoveUp(usize),
            MoveDown(usize),
            Duplicate(usize),
            Delete(usize),
        }
        let mut edit = None;
        for index in 0..draft.rules.len() {
            ui.push_id(format!("{id_prefix}_tactical_rule_{index}"), |ui| {
                let rule = &mut draft.rules[index];
                ui.group(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(&mut rule.enabled, "");
                        ui.strong(format!(
                            "{}. {} - {}",
                            index + 1,
                            rule.decision.label().to_ascii_uppercase(),
                            rule.action.channel().label().to_ascii_uppercase()
                        ));
                        if ui.small_button("Up").clicked() {
                            edit = Some(RuleEdit::MoveUp(index));
                        }
                        if ui.small_button("Down").clicked() {
                            edit = Some(RuleEdit::MoveDown(index));
                        }
                        if ui.small_button("Duplicate").clicked() {
                            edit = Some(RuleEdit::Duplicate(index));
                        }
                        if ui.small_button("Delete").clicked() {
                            edit = Some(RuleEdit::Delete(index));
                        }
                    });
                    let mut remove_condition = None;
                    for condition_index in 0..rule.conditions.len() {
                        let prefix = format!("{id_prefix}_r{index}_c{condition_index}");
                        if tactical_condition_editor(
                            ui,
                            &prefix,
                            &mut rule.conditions[condition_index],
                            &style_options,
                        ) {
                            remove_condition = Some(condition_index);
                        }
                    }
                    if let Some(condition_index) = remove_condition {
                        rule.conditions.remove(condition_index);
                    }
                    if rule.conditions.len() < MAX_TACTICAL_CONDITIONS
                        && ui.small_button("+ Add AND condition").clicked()
                    {
                        rule.conditions.push(TacticalCondition::Always);
                    }
                    ui.horizontal_wrapped(|ui| {
                        ui.label("THEN");
                        tactical_action_editor(
                            ui,
                            &format!("{id_prefix}_r{index}"),
                            &mut rule.action,
                            &learned_styles,
                            &compatible_styles,
                            pair_allowed,
                        );
                        rule.decision = rule.action.decision_point();
                    });
                    if let Some(warning) = tactical_compatibility_warning(
                        &rule.action,
                        &learned_styles,
                        &compatible_styles,
                        pair_allowed,
                        weapon_can_jab,
                    ) {
                        ui.colored_label(Color32::YELLOW, warning);
                    }
                });
            });
        }
        if let Some(edit) = edit {
            match edit {
                RuleEdit::MoveUp(index) if index > 0 => draft.rules.swap(index, index - 1),
                RuleEdit::MoveDown(index) if index + 1 < draft.rules.len() => {
                    draft.rules.swap(index, index + 1)
                }
                RuleEdit::Duplicate(index) => {
                    let rule = draft.rules[index].clone();
                    draft.rules.insert(index + 1, rule);
                }
                RuleEdit::Delete(index) => {
                    draft.rules.remove(index);
                }
                _ => {}
            }
        }
        if ui.button("+ Add rule").clicked() {
            draft.rules.push(TacticalRule::new(
                TacticalAction::NormalAttack,
                vec![TacticalCondition::Always],
            ));
        }
        ui.separator();
        match apply_tactical_ui_policy(&mut player.tactical_policy, draft) {
            Ok(true) => {
                *message = Some(if draft.enabled {
                    "Tactical changes applied immediately.".to_string()
                } else {
                    "Tactical changes saved; directives are currently disabled.".to_string()
                });
                *applied = true;
            }
            Ok(false) => {}
            Err(errors) => {
                *message = Some(errors.join(" "));
            }
        }
        ui.small("Changes are applied immediately. Presets are only needed to reuse a setup.");
    });
}

#[allow(clippy::too_many_arguments)]
fn render_player_editor(
    ui: &mut egui::Ui,
    id_prefix: &str,
    player: &mut PlayerConfig,
    player_color: &mut Color32,
    opponent: &PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
    race_catalog: &[RaceSpec],
    talent_catalog: &TalentCatalog,
    npc_presets: &NpcPresetCatalog,
    fighter_presets: &mut FighterPresetCatalog,
    fighter_preset_name: &mut String,
    active_tab: &mut PlayerEditorTab,
    talent_category_tab: &mut String,
    damage_plot_iterations: &mut String,
    damage_roll_plot: &mut Option<DamageRollPlotData>,
    player_names: &[String; 2],
    dps_attacker_idx: &mut usize,
    dps_defender_idx: &mut usize,
    dps_iterations: &mut u32,
    dps_duration_seconds: &mut u32,
    dps_seed: &mut u64,
    dps_result: &Option<DpsTestResult>,
    dps_sim_duration: Option<std::time::Duration>,
    run_dps_test: &mut bool,
    tactical_draft: &mut TacticalPolicy,
    tactical_presets: &mut Vec<TacticalPreset>,
    tactical_preset_index: &mut usize,
    tactical_preset_name: &mut String,
    tactical_message: &mut Option<String>,
    tactical_pending_load: &mut Option<usize>,
    tactical_confirm_overwrite: &mut bool,
    tactical_confirm_delete: &mut bool,
    tactics_locked: bool,
    tactics_applied: &mut bool,
) {
    if weapon_catalog.is_empty() {
        ui.label("Weapon catalog is empty.");
        return;
    }
    game_logic::sanitize_player_ids(
        player,
        weapon_catalog,
        armor_catalog,
        shield_catalog,
        talent_catalog,
    );
    if let Some(weapon) = weapon_catalog.get(player.weapon_id) {
        if weapon.jab_speed.is_none() {
            player.use_jab = false;
        }
    }

    render_player_editor_tabs(ui, id_prefix, active_tab);

    match *active_tab {
        PlayerEditorTab::Core => {
            if !fighter_presets.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Fighter preset");
                    let mut selection = player
                        .fighter_preset
                        .map(|id| fighter_presets.index_of(id))
                        .unwrap_or(usize::MAX);
                    let selected_text = player
                        .fighter_preset
                        .and_then(|id| fighter_presets.get(id))
                        .map(|preset| preset.name.clone())
                        .unwrap_or_else(|| "None".to_string());
                    let options = std::iter::once((usize::MAX, "None".to_string(), true)).chain(
                        fighter_presets
                            .entries()
                            .iter()
                            .enumerate()
                            .map(|(idx, preset)| (idx, preset.name.clone(), true)),
                    );
                    searchable_select(
                        ui,
                        format!("{id_prefix}_fighter_preset"),
                        selected_text,
                        &mut selection,
                        options,
                    );
                    let selection = if selection == usize::MAX {
                        None
                    } else {
                        fighter_presets.id_from_index(selection)
                    };
                    if selection != player.fighter_preset {
                        player.fighter_preset = selection;
                        if let Some(id) = selection {
                            if let Some(preset) = fighter_presets.get(id) {
                                apply_fighter_preset(
                                    player,
                                    preset,
                                    weapon_catalog,
                                    armor_catalog,
                                    shield_catalog,
                                    race_catalog,
                                );
                                player.npc_preset = None;
                                fighter_preset_name.clear();
                                fighter_preset_name.push_str(preset.name.as_str());
                            }
                        }
                    }
                });
                let save_enabled =
                    !fighter_preset_name.trim().is_empty() && player.npc_preset.is_none();
                ui.horizontal(|ui| {
                    ui.label("Save as");
                    ui.text_edit_singleline(fighter_preset_name);
                    if ui
                        .add_enabled(save_enabled, egui::Button::new("Save preset"))
                        .clicked()
                    {
                        let name = fighter_preset_name.trim();
                        if !name.is_empty() {
                            let preset = fighter_preset_from_player(
                                player,
                                weapon_catalog,
                                armor_catalog,
                                shield_catalog,
                                name,
                            );
                            if let Some(existing) = fighter_presets
                                .entries()
                                .iter()
                                .position(|entry| entry.name.eq_ignore_ascii_case(name))
                            {
                                if let Some(id) = fighter_presets.id_from_index(existing) {
                                    fighter_presets.replace(id, preset);
                                    player.fighter_preset = Some(id);
                                }
                            } else {
                                let id = fighter_presets.push(preset);
                                player.fighter_preset = Some(id);
                            }
                            if let Err(err) =
                                data::save_fighter_presets(FIGHTER_PRESETS_PATH, fighter_presets)
                            {
                                eprintln!("Failed to save fighter presets: {err}");
                            }
                        }
                    }
                    if player.npc_preset.is_some() {
                        ui.label("Disabled while NPC preset is active.");
                    }
                });
            }

            if !npc_presets.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("NPC preset");
                    let mut selection = player
                        .npc_preset
                        .map(|id| npc_presets.index_of(id))
                        .unwrap_or(usize::MAX);
                    let selected_text = player
                        .npc_preset
                        .and_then(|id| npc_presets.get(id))
                        .map(|preset| preset.name.clone())
                        .unwrap_or_else(|| "None".to_string());
                    let options = std::iter::once((usize::MAX, "None".to_string(), true)).chain(
                        npc_presets
                            .entries()
                            .iter()
                            .enumerate()
                            .map(|(idx, preset)| (idx, preset.name.clone(), true)),
                    );
                    searchable_select(
                        ui,
                        format!("{id_prefix}_npc_preset"),
                        selected_text,
                        &mut selection,
                        options,
                    );
                    player.npc_preset = if selection == usize::MAX {
                        None
                    } else {
                        npc_presets.id_from_index(selection)
                    };
                });
                if let Some(preset) = player.npc_preset.and_then(|id| npc_presets.get(id)) {
                    player.name = preset.name.clone();
                    ui.label(format!(
                        "Preset: HP {} | ATT {} | DEF {} | DR {} | DMG +{} | TOP {}",
                        preset.hp,
                        preset.attack_bonus,
                        preset.defense_mod,
                        preset.armor_dr,
                        preset.damage_bonus,
                        preset.top
                    ));
                }
            }

            let npc_active = player.npc_preset.is_some();
            ui.add_enabled_ui(!npc_active, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut player.name);
                });
                ui.horizontal(|ui| {
                    ui.label("Level");
                    ui.add(egui::Slider::new(&mut player.level, 1..=20).step_by(1.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Base HP");
                    ui.add(egui::Slider::new(&mut player.base_hp, 1..=200).step_by(1.0));
                });
            });
            ui.horizontal(|ui| {
                ui.label("Color");
                ui.color_edit_button_srgba(player_color);
            });
            ui.horizontal(|ui| {
                ui.label("Move speed (ft/s)");
                ui.add(egui::Slider::new(&mut player.move_speed, 0.0..=40.0).step_by(5.0));
            });
            ui.add_enabled_ui(!npc_active, |ui| {
                ui.horizontal(|ui| {
                    tier_combo(
                        ui,
                        format!("{id_prefix}_attack_tier"),
                        "Attack Tier",
                        &mut player.progression.attack,
                        &[
                            ProgressionTier::I,
                            ProgressionTier::II,
                            ProgressionTier::III,
                            ProgressionTier::IV,
                            ProgressionTier::V,
                            ProgressionTier::VI,
                        ],
                    );
                });
                ui.horizontal(|ui| {
                    tier_combo(
                        ui,
                        format!("{id_prefix}_speed_tier"),
                        "Speed Tier",
                        &mut player.progression.speed,
                        &[
                            ProgressionTier::I,
                            ProgressionTier::II,
                            ProgressionTier::III,
                            ProgressionTier::IV,
                            ProgressionTier::V,
                            ProgressionTier::VI,
                        ],
                    );
                });
                ui.horizontal(|ui| {
                    tier_combo(
                        ui,
                        format!("{id_prefix}_initiative_tier"),
                        "Initiative Tier",
                        &mut player.progression.initiative,
                        &[
                            ProgressionTier::I,
                            ProgressionTier::II,
                            ProgressionTier::III,
                            ProgressionTier::IV,
                            ProgressionTier::V,
                        ],
                    );
                });
                ui.horizontal(|ui| {
                    tier_combo(
                        ui,
                        format!("{id_prefix}_health_tier"),
                        "Health Tier",
                        &mut player.progression.health,
                        &[
                            ProgressionTier::I,
                            ProgressionTier::II,
                            ProgressionTier::III,
                            ProgressionTier::IV,
                            ProgressionTier::V,
                        ],
                    );
                });
            });
        }
        PlayerEditorTab::Gear => {
            let mut uses_projectiles = false;
            ui.horizontal(|ui| {
                ui.label("Weapon");
                let mut selection = weapon_catalog.index_of(player.weapon_id);
                let selected_text = weapon_catalog
                    .get(player.weapon_id)
                    .map(|weapon| weapon.name.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                searchable_select(
                    ui,
                    format!("{id_prefix}_weapon"),
                    selected_text,
                    &mut selection,
                    weapon_catalog
                        .entries()
                        .iter()
                        .enumerate()
                        .map(|(idx, weapon)| (idx, weapon.name.clone(), true)),
                );
                if let Some(id) = weapon_catalog.id_from_index(selection) {
                    player.weapon_id = id;
                }
                let weapon = weapon_catalog.get(player.weapon_id).unwrap_or_else(|| {
                    weapon_catalog
                        .entries()
                        .first()
                        .expect("weapon catalog empty")
                });
                game_logic::sanitize_projectile_tier(player, weapon);
                uses_projectiles = game_logic::weapon_uses_projectiles(weapon);
                material_tier_combo(
                    ui,
                    format!("{id_prefix}_weapon_material"),
                    "Weapon material",
                    &mut player.weapon_material_tier,
                );
                if uses_projectiles {
                    material_tier_combo(
                        ui,
                        format!("{id_prefix}_ammo_material"),
                        "Ammo material",
                        &mut player.projectile_material_tier,
                    );
                }
            });

            let weapon = weapon_catalog.get(player.weapon_id).unwrap_or_else(|| {
                weapon_catalog
                    .entries()
                    .first()
                    .expect("weapon catalog empty")
            });
            let is_two_handed = weapon.handedness == WeaponHandedness::TwoHanded;
            let can_two_hand = weapon.handedness == WeaponHandedness::OneHanded
                && (weapon.size == WeaponSize::Medium || weapon.size == WeaponSize::Large);
            if is_two_handed {
                player.two_hand_grip = true;
            } else if !can_two_hand {
                player.two_hand_grip = false;
            }
            let can_dualwield =
                weapon.handedness == WeaponHandedness::OneHanded && !player.two_hand_grip;
            let has_perfect_two_weapon_fighting =
                game_logic::has_perfect_two_weapon_fighting_effect(player);
            let was_offensive_dualwielding = player.offensive_dualwielding;
            let can_defensive_dualwield = can_dualwield
                && (has_perfect_two_weapon_fighting || !player.offensive_dualwielding);
            let can_offensive_dualwield = can_dualwield
                && (has_perfect_two_weapon_fighting || !player.defensive_dualwielding);
            if !can_dualwield {
                player.defensive_dualwielding = false;
                player.offensive_dualwielding = false;
                player.offhand_weapon_id = None;
            }
            if !has_perfect_two_weapon_fighting {
                if player.defensive_dualwielding {
                    player.offensive_dualwielding = false;
                }
                if player.offensive_dualwielding {
                    player.defensive_dualwielding = false;
                }
            }
            let jab_label = weapon
                .jab_speed_label
                .as_ref()
                .map(|jab| format!(" (jab {jab})"))
                .unwrap_or_default();
            ui.label(format!(
                "Speed {}{} | Damage {} | Reach/Range {}",
                weapon.speed_label, jab_label, weapon.damage_expr, weapon.reach_label
            ));
            if player.two_hand_grip && can_two_hand {
                ui.label("Two-hand grip: +3 damage, +2 speed");
            }
            ui.horizontal(|ui| {
                let enabled = can_two_hand && !is_two_handed;
                ui.add_enabled_ui(enabled, |ui| {
                    ui.checkbox(&mut player.two_hand_grip, "Two-hand grip");
                });
                if is_two_handed {
                    ui.label("Required");
                } else if !can_two_hand {
                    ui.label("Unavailable");
                }
            });
            ui.horizontal(|ui| {
                ui.add_enabled_ui(can_defensive_dualwield, |ui| {
                    ui.checkbox(&mut player.defensive_dualwielding, "Defensive dualwielding");
                });
                if !can_defensive_dualwield {
                    ui.label("Unavailable");
                }
            });
            ui.horizontal(|ui| {
                ui.add_enabled_ui(can_offensive_dualwield, |ui| {
                    ui.checkbox(&mut player.offensive_dualwielding, "Offensive dualwielding");
                });
                if !can_offensive_dualwield {
                    ui.label("Unavailable");
                }
            });
            if player.defensive_dualwielding {
                ui.label(
                    "Defensive dualwielding: double defense mastery & weapon defense talent bonus",
                );
            }
            if player.offensive_dualwielding {
                ui.label("Offensive dualwielding: alternate primary/offhand attacks");
            }
            if player.offensive_dualwielding && !was_offensive_dualwielding {
                player.offhand_weapon_id =
                    game_logic::default_offhand_weapon_id(player, weapon, weapon_catalog);
            }

            let npc_active = player.npc_preset.is_some();
            ui.add_enabled_ui(!npc_active, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Armor");
                    let mut selection = armor_catalog.index_of(player.armor_id);
                    searchable_select(
                        ui,
                        format!("{id_prefix}_armor"),
                        armor_display_name(armor_catalog.get(player.armor_id)),
                        &mut selection,
                        armor_catalog
                            .entries()
                            .iter()
                            .enumerate()
                            .map(|(idx, armor)| (idx, armor.label.clone(), true)),
                    );
                    if let Some(id) = armor_catalog.id_from_index(selection) {
                        player.armor_id = id;
                    }
                    material_tier_combo(
                        ui,
                        format!("{id_prefix}_armor_material"),
                        "Material",
                        &mut player.armor_material_tier,
                    );
                });
                let selected_shield_allowed = shield_catalog
                    .get(player.shield_id)
                    .and_then(|entry| entry.shield.as_ref())
                    .map(|shield| {
                        game_logic::shield_option_allowed(
                            player,
                            weapon,
                            Some(shield),
                            talent_catalog,
                            weapon_catalog,
                        )
                    })
                    .unwrap_or(true);
                if !selected_shield_allowed {
                    player.shield_id = ShieldId::new(0);
                    player.shield_material_tier = 0;
                }
                let shield_allowed = shield_catalog.entries().iter().any(|entry| {
                    entry
                        .shield
                        .as_ref()
                        .map(|shield| {
                            game_logic::shield_option_allowed(
                                player,
                                weapon,
                                Some(shield),
                                talent_catalog,
                                weapon_catalog,
                            )
                        })
                        .unwrap_or(false)
                });
                ui.horizontal(|ui| {
                    ui.label("Shield");
                    let mut selection = shield_catalog.index_of(player.shield_id);
                    let selected_name = shield_catalog
                        .get(player.shield_id)
                        .map(|entry| entry.label.as_str())
                        .unwrap_or("None");
                    ui.add_enabled_ui(shield_allowed, |ui| {
                        searchable_select(
                            ui,
                            format!("{id_prefix}_shield"),
                            selected_name,
                            &mut selection,
                            shield_catalog
                                .entries()
                                .iter()
                                .enumerate()
                                .map(|(idx, shield)| {
                                    let option_allowed = shield
                                        .shield
                                        .as_ref()
                                        .map(|shield| {
                                            game_logic::shield_option_allowed(
                                                player,
                                                weapon,
                                                Some(shield),
                                                talent_catalog,
                                                weapon_catalog,
                                            )
                                        })
                                        .unwrap_or(true);
                                    (idx, shield.label.clone(), option_allowed)
                                }),
                        );
                    });
                    if let Some(id) = shield_catalog.id_from_index(selection) {
                        player.shield_id = id;
                    }
                    let shield_selected = player.shield_id.index() > 0;
                    if !shield_selected {
                        player.shield_material_tier = 0;
                    }
                    ui.add_enabled_ui(shield_allowed && shield_selected, |ui| {
                        material_tier_combo(
                            ui,
                            format!("{id_prefix}_shield_material"),
                            "Material",
                            &mut player.shield_material_tier,
                        );
                    });
                    if !shield_allowed {
                        ui.label("Unavailable");
                    } else if !selected_shield_allowed {
                        ui.label("Only bucklers and small shields are allowed.");
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Offhand weapon");
                    let offhand_loadout_possible =
                        weapon.handedness == WeaponHandedness::OneHanded && !player.two_hand_grip;
                    let can_use_offhand = player.offensive_dualwielding && offhand_loadout_possible;
                    if !offhand_loadout_possible {
                        player.offhand_weapon_id = None;
                    }
                    ui.add_enabled_ui(can_use_offhand, |ui| {
                        let mut selection = player
                            .offhand_weapon_id
                            .map(|id| weapon_catalog.index_of(id));
                        let selected_name = selection
                            .and_then(|idx| weapon_catalog.entries().get(idx))
                            .map(|weapon| weapon.name.as_str())
                            .unwrap_or("None");
                        searchable_select(
                            ui,
                            format!("{id_prefix}_offhand"),
                            selected_name,
                            &mut selection,
                            std::iter::once((None, "None".to_string(), true)).chain(
                                weapon_catalog
                                    .entries()
                                    .iter()
                                    .enumerate()
                                    .filter(|(_, weapon)| {
                                        weapon.handedness == WeaponHandedness::OneHanded
                                    })
                                    .map(|(idx, weapon)| (Some(idx), weapon.name.clone(), true)),
                            ),
                        );
                        player.offhand_weapon_id =
                            selection.and_then(|idx| weapon_catalog.id_from_index(idx));
                    });
                    let offhand = player
                        .offhand_weapon_id
                        .and_then(|id| weapon_catalog.get(id));
                    let offhand_enabled = offhand.is_some() && can_use_offhand;
                    ui.add_enabled_ui(offhand_enabled, |ui| {
                        material_tier_combo(
                            ui,
                            format!("{id_prefix}_offhand_material"),
                            "Material",
                            &mut player.offhand_weapon_material_tier,
                        );
                        if offhand
                            .map(game_logic::weapon_uses_projectiles)
                            .unwrap_or(false)
                        {
                            material_tier_combo(
                                ui,
                                format!("{id_prefix}_offhand_ammo_material"),
                                "Ammo material",
                                &mut player.offhand_projectile_material_tier,
                            );
                        } else {
                            player.offhand_projectile_material_tier = 0;
                        }
                    });
                    if !can_use_offhand {
                        ui.label("Unavailable");
                    }
                });
                if let Some(offhand_id) = player.offhand_weapon_id {
                    if let Some(offhand) = weapon_catalog.get(offhand_id) {
                        let jab_label = offhand
                            .jab_speed_label
                            .as_ref()
                            .map(|jab| format!(" (jab {jab})"))
                            .unwrap_or_default();
                        ui.label(format!(
                            "Offhand speed {}{} | Damage {} | Reach/Range {}",
                            offhand.speed_label,
                            jab_label,
                            offhand.damage_expr,
                            offhand.reach_label
                        ));
                    }
                } else {
                    player.offhand_weapon_material_tier = 0;
                    player.offhand_projectile_material_tier = 0;
                }
            });
        }
        PlayerEditorTab::CombatManeuvers => {
            let weapon = weapon_catalog.get(player.weapon_id).unwrap_or_else(|| {
                weapon_catalog
                    .entries()
                    .first()
                    .expect("weapon catalog empty")
            });
            render_default_weapon_style_selector(
                ui,
                id_prefix,
                player,
                weapon_catalog,
                armor_catalog,
                shield_catalog,
                talent_catalog,
            );
            ui.separator();
            let has_jab = weapon.jab_speed.is_some();
            let tactics_controlled = player.tactical_policy.enabled;
            ui.label("Toggle to always attempt maneuvers when eligible.");
            if tactics_controlled {
                ui.colored_label(
                    Color32::LIGHT_BLUE,
                    "Jab, Fight Defensively, and Give Ground are controlled by Tactical Directives.",
                );
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.add_enabled_ui(has_jab && !tactics_controlled, |ui| {
                    ui.checkbox(&mut player.use_jab, "Jab");
                });
                if !has_jab {
                    ui.label("Unavailable");
                } else if tactics_controlled {
                    ui.label("Controlled by Tactical Directives");
                }
            });
            if player.use_jab {
                if let Some(jab_special) = weapon.jab_special_expr.as_ref() {
                    ui.label(format!(
                        "Jab special damage: {jab_special} (non-penetrating)"
                    ));
                } else {
                    ui.label("Jab damage: half, non-penetrating");
                }
            }
            ui.checkbox(&mut player.hold_at_bay, "Hold at bay");
            ui.horizontal(|ui| {
                ui.checkbox(&mut player.called_shot, "Called shot");
                let (called_shot_light_bonus, called_shot_medium_bonus, called_shot_heavy_bonus) =
                    game_logic::called_shot_target_defense_bonuses_for_player(player);
                let called_shot_self_penalty =
                    game_logic::called_shot_self_defense_penalty_for_player(player);
                let called_shot_delay_expr = game_logic::called_shot_delay_expr_for_player(
                    player,
                    game_logic::is_ranged_weapon(weapon),
                );
                ui.label(format!(
                    "Target defense (light/medium/heavy): +{called_shot_light_bonus}/+{called_shot_medium_bonus}/+{called_shot_heavy_bonus}, self -{called_shot_self_penalty} defense, speed +{called_shot_delay_expr}"
                ));
            });
            ui.horizontal(|ui| {
                let can_power_attack =
                    game_logic::power_attack_available_for_player(player, weapon);
                ui.add_enabled_ui(can_power_attack, |ui| {
                    ui.checkbox(&mut player.power_attack, "Power attack");
                });
                if can_power_attack {
                    ui.label("Ignores positive INT/DEX attack bonuses and doubles Strength damage");
                } else {
                    player.power_attack = false;
                    ui.label("Requires Power Attack, STR 13+, and a non-small melee weapon");
                }
            });
            ui.separator();
            ui.add_enabled_ui(false, |ui| {
                ui.checkbox(&mut player.aggressive_attack, "Aggressive attack (NYI)");
            });
            ui.checkbox(&mut player.charge, "Charge");
            ui.add_enabled_ui(false, |ui| {
                ui.checkbox(
                    &mut player.ready_against_charge,
                    "Ready against charge (NYI)",
                );
            });
            ui.add_enabled_ui(false, |ui| {
                ui.checkbox(&mut player.tactical_move, "Tactical move (NYI)");
            });
            ui.horizontal(|ui| {
                ui.add_enabled_ui(!tactics_controlled, |ui| {
                    ui.checkbox(&mut player.fight_defensively, "Fight defensively");
                });
                let mut penalty = game_logic::normalize_fight_defensively_penalty(
                    player.fight_defensively_penalty,
                );
                ui.add_enabled_ui(!tactics_controlled, |ui| {
                    searchable_select(
                        ui,
                        format!("{id_prefix}_fight_defensively_penalty"),
                        format!("-{penalty}/+{}", penalty / 2),
                        &mut penalty,
                        game_logic::FIGHT_DEFENSIVELY_PENALTY_OPTIONS
                            .into_iter()
                            .map(|option| (option, format!("-{option}/+{}", option / 2), true)),
                    );
                });
                player.fight_defensively_penalty = penalty;
            });
            ui.add_enabled_ui(false, |ui| {
                ui.checkbox(&mut player.full_parry, "Full parry (NYI)");
            });
            ui.add_enabled_ui(false, |ui| {
                ui.checkbox(&mut player.give_ground, "Give ground (Tactics only)");
            });
            ui.add_enabled_ui(false, |ui| {
                ui.checkbox(&mut player.scamper_back, "Scamper back (NYI)");
            });
            ui.add_enabled_ui(false, |ui| {
                ui.checkbox(&mut player.fighting_withdrawal, "Fighting withdrawal (NYI)");
            });
            ui.add_enabled_ui(false, |ui| {
                ui.checkbox(&mut player.flee, "Flee (NYI)");
            });
            ui.checkbox(&mut player.mounted, "Mounted");
        }
        PlayerEditorTab::Tactics => {
            egui::ScrollArea::vertical().show(ui, |ui| {
                render_tactics_editor(
                    ui,
                    id_prefix,
                    player,
                    weapon_catalog,
                    armor_catalog,
                    shield_catalog,
                    talent_catalog,
                    tactical_draft,
                    tactical_presets,
                    tactical_preset_index,
                    tactical_preset_name,
                    tactical_message,
                    tactical_pending_load,
                    tactical_confirm_overwrite,
                    tactical_confirm_delete,
                    tactics_locked,
                    tactics_applied,
                );
            });
        }
        PlayerEditorTab::Stats => {
            let npc_active = player.npc_preset.is_some();
            if npc_active {
                ui.label("Disabled while NPC preset is active.");
            }
            if !race_catalog.is_empty() {
                let mut selection = player
                    .race_id
                    .as_ref()
                    .and_then(|id| race_catalog.iter().position(|race| race.id == *id))
                    .unwrap_or(usize::MAX);
                let race_locked =
                    player.race_applied || player.fighter_preset.is_some() || npc_active;
                ui.horizontal(|ui| {
                    ui.label("Race");
                    ui.add_enabled_ui(!race_locked, |ui| {
                        searchable_select(
                            ui,
                            format!("{id_prefix}_race"),
                            race_catalog
                                .get(selection)
                                .map(|race| race.name.as_str())
                                .unwrap_or("None"),
                            &mut selection,
                            std::iter::once((usize::MAX, "None".to_string(), true)).chain(
                                race_catalog
                                    .iter()
                                    .enumerate()
                                    .map(|(idx, race)| (idx, race.name.clone(), true)),
                            ),
                        );
                    });
                    let selected_race = race_catalog.get(selection);
                    let can_apply = selected_race.is_some() && !race_locked;
                    if ui
                        .add_enabled(can_apply, egui::Button::new("Apply race adjustments"))
                        .clicked()
                    {
                        if let Some(race) = selected_race {
                            game_logic::apply_race_adjustments(player, race);
                        }
                    }
                });
                let selected_race = if selection == usize::MAX {
                    None
                } else {
                    race_catalog.get(selection)
                };
                player.race_id = selected_race.map(|race| race.id.clone());
                player.knockback_step = selected_race
                    .map(game_logic::knockback_step_for_race)
                    .unwrap_or(game_logic::DEFAULT_KNOCKBACK_STEP);
                if let Some(race) = player
                    .race_id
                    .as_ref()
                    .and_then(|id| race_catalog.iter().find(|race| race.id == *id))
                {
                    ui.label(format!(
                        "Base HP {} | {}",
                        race.base_hp,
                        race_adjustment_summary(race)
                    ));
                }
                if race_locked {
                    ui.label("Race adjustments apply only when creating a new character.");
                }
                if !npc_active {
                    ui.separator();
                }
            }
            ui.add_enabled_ui(!npc_active, |ui| {
                ui.label("Conditions");
                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut player.environment.natural_surroundings,
                        "Natural surroundings",
                    );
                    ui.checkbox(&mut player.environment.bright_light, "Bright light");
                    ui.label("Temp C");
                    ui.add(egui::DragValue::new(&mut player.environment.temperature_c).speed(1));
                });
                ui.separator();
                ui.label("Misc roll modifiers");
                let mut misc_row = |label: &str, value: &mut i32| {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        ui.add(egui::DragValue::new(value).speed(1));
                    });
                };
                misc_row("All rolls", &mut player.misc_modifiers.all_roll_bonus);
                misc_row("Attack", &mut player.misc_modifiers.attack_bonus);
                misc_row("Defense", &mut player.misc_modifiers.defense_bonus);
                misc_row("Damage", &mut player.misc_modifiers.damage_bonus);
                misc_row("Initiative", &mut player.misc_modifiers.initiative_bonus);
                misc_row("Speed", &mut player.misc_modifiers.speed_mod_bonus);
                misc_row("Armor DR", &mut player.misc_modifiers.armor_dr_bonus);
                misc_row("HP", &mut player.misc_modifiers.hp_bonus);
                misc_row(
                    "Initiative die steps",
                    &mut player.misc_modifiers.initiative_die_bonus,
                );
                ui.separator();
                ui.label("Abilities");
                ability_percentile_editor(
                    ui,
                    &format!("{id_prefix}_str"),
                    "STR",
                    &mut player.strength_base,
                    &mut player.strength_pct,
                );
                ability_percentile_editor(
                    ui,
                    &format!("{id_prefix}_dex"),
                    "DEX",
                    &mut player.dex_base,
                    &mut player.dex_pct,
                );
                ability_slider(ui, "INT", &mut player.intelligence);
                ability_slider(ui, "WIS", &mut player.wisdom);
                ability_slider(ui, "CON", &mut player.constitution);
                ability_slider(ui, "LKS", &mut player.looks);
                ability_slider(ui, "CHA", &mut player.charisma);

                ui.separator();
                let weapon = weapon_catalog.get(player.weapon_id).unwrap_or_else(|| {
                    weapon_catalog
                        .entries()
                        .first()
                        .expect("weapon catalog empty")
                });
                let shield_active = game_logic::shield_equipped_with_catalog(
                    player,
                    weapon,
                    shield_catalog,
                    talent_catalog,
                    weapon_catalog,
                );
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label("Weapon masteries");
                        mastery_slider(ui, "Attack", &mut player.mastery_attack);
                        mastery_slider(ui, "Defense", &mut player.mastery_defense);
                        mastery_slider(ui, "Damage", &mut player.mastery_damage);
                        mastery_slider(ui, "Speed", &mut player.mastery_speed);
                    });
                    ui.separator();
                    ui.add_enabled_ui(shield_active, |ui| {
                        ui.vertical(|ui| {
                            ui.label("Shield masteries");
                            mastery_slider(ui, "Defense", &mut player.shield_mastery_defense);
                            mastery_slider(ui, "Speed", &mut player.shield_mastery_speed);
                        });
                    });
                });
            });
        }
        PlayerEditorTab::Talents => {
            let npc_active = player.npc_preset.is_some();
            if npc_active {
                ui.label("Disabled while NPC preset is active.");
            }
            if !race_catalog.is_empty() {
                let mut selection = player
                    .race_id
                    .as_ref()
                    .and_then(|id| race_catalog.iter().position(|race| race.id == *id))
                    .unwrap_or(usize::MAX);
                let race_locked =
                    player.race_applied || player.fighter_preset.is_some() || npc_active;
                ui.horizontal(|ui| {
                    ui.label("Race");
                    ui.add_enabled_ui(!race_locked, |ui| {
                        searchable_select(
                            ui,
                            format!("{id_prefix}_race_talents"),
                            race_catalog
                                .get(selection)
                                .map(|race| race.name.as_str())
                                .unwrap_or("None"),
                            &mut selection,
                            std::iter::once((usize::MAX, "None".to_string(), true)).chain(
                                race_catalog
                                    .iter()
                                    .enumerate()
                                    .map(|(idx, race)| (idx, race.name.clone(), true)),
                            ),
                        );
                    });
                });
                let selected_race = if selection == usize::MAX {
                    None
                } else {
                    race_catalog.get(selection)
                };
                player.race_id = selected_race.map(|race| race.id.clone());
                player.knockback_step = selected_race
                    .map(game_logic::knockback_step_for_race)
                    .unwrap_or(game_logic::DEFAULT_KNOCKBACK_STEP);
                if race_locked {
                    ui.label("Race selection is locked by the preset.");
                }
                if !npc_active {
                    ui.separator();
                }
            }
            ui.label("Current talents");
            let mut current_talents: Vec<String> = player
                .talents
                .iter()
                .map(|selection| talent_display_label(selection, talent_catalog))
                .collect();
            if current_talents.is_empty() {
                ui.label("None");
            } else {
                current_talents.sort();
                egui::ScrollArea::vertical()
                    .max_height(120.0)
                    .show(ui, |ui| {
                        for label in current_talents {
                            ui.label(label);
                        }
                    });
            }
            if !npc_active {
                ui.separator();
            }
            ui.add_enabled_ui(!npc_active, |ui| {
                ui.label("Talents");
                render_talent_selector(
                    ui,
                    id_prefix,
                    player,
                    weapon_catalog,
                    race_catalog,
                    talent_catalog,
                    talent_category_tab,
                );
            });
        }
        PlayerEditorTab::Derived => {
            let npc_active = player.npc_preset.is_some();
            if npc_active {
                ui.label("Derived stats ignored while NPC preset is active.");
                return;
            }
            let summary = game_logic::player_summary(
                player,
                weapon_catalog,
                armor_catalog,
                shield_catalog,
                talent_catalog,
            );
            let combatant = game_logic::build_combatant(
                player,
                weapon_catalog,
                armor_catalog,
                shield_catalog,
                npc_presets,
                talent_catalog,
            );
            let breakdowns = game_logic::derived_stat_breakdowns(
                player,
                weapon_catalog,
                armor_catalog,
                shield_catalog,
                talent_catalog,
                &summary,
                &combatant,
            );
            let derived = &summary.derived;
            let roll = &summary.roll;
            let defense = &summary.defense;
            ui.label("Derived");
            derived_stat_line(
                ui,
                format!(
                    "Hit points: {} (x{:.1})",
                    derived.hit_points, derived.health_mult
                ),
                breakdowns.get(game_logic::DerivedStatId::HitPoints),
            );
            derived_stat_line(
                ui,
                format!("Drain resistance: {}", derived.drain_resistance),
                breakdowns.get(game_logic::DerivedStatId::DrainResistance),
            );
            derived_stat_line(
                ui,
                format!(
                    "Threshold of Pain: {}",
                    combatant.sheet.vitals.threshold_of_pain
                ),
                breakdowns.get(game_logic::DerivedStatId::ThresholdOfPain),
            );
            derived_stat_line(
                ui,
                format!("Attack bonus: {}", derived.attack_bonus),
                breakdowns.get(game_logic::DerivedStatId::AttackBonus),
            );
            derived_stat_line(
                ui,
                format!("Attack bonus (effective): {}", roll.attack_bonus),
                breakdowns.get(game_logic::DerivedStatId::EffectiveAttackBonus),
            );
            derived_stat_line(
                ui,
                format!("Damage bonus (effective): {}", roll.strength_damage),
                breakdowns.get(game_logic::DerivedStatId::EffectiveDamageBonus),
            );
            derived_stat_line(
                ui,
                format!("Speed mod: {}", derived.speed_mod),
                breakdowns.get(game_logic::DerivedStatId::SpeedModifier),
            );
            derived_stat_line(
                ui,
                format!("Weapon speed: {:.1}", combatant.sheet.offense.weapon.speed),
                breakdowns.get(game_logic::DerivedStatId::MainhandWeaponSpeed),
            );
            if player.called_shot {
                let (called_shot_light_bonus, called_shot_medium_bonus, called_shot_heavy_bonus) =
                    game_logic::called_shot_target_defense_bonuses_for_player(player);
                let called_shot_target_bonus_vs_opponent =
                    game_logic::called_shot_target_defense_bonus_against_target(
                        player,
                        opponent,
                        armor_catalog,
                    );
                let called_shot_self_penalty =
                    game_logic::called_shot_self_defense_penalty_for_player(player);
                let called_shot_delay_expr =
                    game_logic::called_shot_delay_expr_for_player(player, roll.is_ranged_weapon);
                ui.label(format!(
                    "Called shot: target +{called_shot_target_bonus_vs_opponent} defense vs current target (light/medium/heavy +{called_shot_light_bonus}/+{called_shot_medium_bonus}/+{called_shot_heavy_bonus}), self -{called_shot_self_penalty} defense, speed +{called_shot_delay_expr}"
                ));
                if game_logic::called_shot_deceptive_defender_effect_active(opponent) {
                    ui.label(format!(
                        "Vs Deceptive Defender target: speed +{} and target +4 defense",
                        game_logic::CALLED_SHOT_DECEPTIVE_DEFENDER_DELAY_EXPR
                    ));
                }
            }
            if player.power_attack {
                ui.label("Power attack: positive INT/DEX attack bonuses ignored, Strength damage doubled");
            }
            derived_stat_line(
                ui,
                format!("Initiative mod: {}", derived.initiative_mod),
                breakdowns.get(game_logic::DerivedStatId::InitiativeModifier),
            );
            derived_stat_line(
                ui,
                format!("Base DV: {}", derived.base_dv),
                breakdowns.get(game_logic::DerivedStatId::BaseDefense),
            );
            if let Some(dv_with_shield) = defense.melee_with_shield_dv {
                derived_stat_line(
                    ui,
                    format!("DV (melee + shield): {}", dv_with_shield),
                    breakdowns.get(game_logic::DerivedStatId::MeleeDefense),
                );
            }
            derived_stat_line(
                ui,
                format!("Armor DR: {}", derived.armor_dr),
                breakdowns.get(game_logic::DerivedStatId::ArmorDr),
            );
            derived_stat_line(
                ui,
                format!("Carry (none/light/med/heavy): {:?}", derived.carry_capacity),
                breakdowns.get(game_logic::DerivedStatId::CarryCapacity),
            );
            derived_stat_line(
                ui,
                format!("Load: {}", derived.load_category),
                breakdowns.get(game_logic::DerivedStatId::LoadCategory),
            );

            let attack_bonus = roll.attack_bonus;
            let strength_damage = roll.strength_damage;
            let opponent_combatant = game_logic::build_combatant(
                opponent,
                weapon_catalog,
                armor_catalog,
                shield_catalog,
                npc_presets,
                talent_catalog,
            );
            let target_armor_dr = opponent_combatant.sheet.defense.armor_dr.max(0);
            let target_natural_dr = opponent_combatant.sheet.defense.natural_dr.max(0);
            let called_shot_target_bonus_vs_opponent =
                game_logic::called_shot_target_defense_bonus_against_target(
                    player,
                    opponent,
                    armor_catalog,
                );
            let (called_shot_light_bonus, called_shot_medium_bonus, called_shot_heavy_bonus) =
                game_logic::called_shot_target_defense_bonuses_for_player(player);
            let called_shot_mainhand_delay_expr = if player.called_shot {
                if game_logic::called_shot_deceptive_defender_effect_active(opponent) {
                    game_logic::CALLED_SHOT_DECEPTIVE_DEFENDER_DELAY_EXPR
                } else {
                    game_logic::called_shot_delay_expr_for_player(player, roll.is_ranged_weapon)
                }
            } else {
                ""
            };

            ui.separator();
            ui.label("Defense");
            if roll.is_ranged_weapon {
                derived_stat_line(
                    ui,
                    defense.ranged_roll_label.as_str(),
                    breakdowns.get(game_logic::DerivedStatId::RangedDefense),
                );
            } else {
                derived_stat_line(
                    ui,
                    defense.melee_roll_label.as_str(),
                    breakdowns.get(game_logic::DerivedStatId::MeleeDefense),
                );
            }
            ui.separator();
            ui.label("Mainhand");
            let weapon_shield_damage = combatant
                .sheet
                .offense
                .weapon
                .shield_damage_expr
                .as_deref()
                .unwrap_or("-");
            if player.called_shot {
                derived_stat_line(
                    ui,
                    format!(
                        "Weapon speed: {} + {} (called shot)",
                        combatant.sheet.offense.weapon.speed, called_shot_mainhand_delay_expr
                    ),
                    breakdowns.get(game_logic::DerivedStatId::MainhandWeaponSpeed),
                );
            } else {
                derived_stat_line(
                    ui,
                    format!("Weapon speed: {}", combatant.sheet.offense.weapon.speed),
                    breakdowns.get(game_logic::DerivedStatId::MainhandWeaponSpeed),
                );
            }
            derived_stat_line(
                ui,
                format!("Weapon shield damage: {}", weapon_shield_damage),
                breakdowns.get(game_logic::DerivedStatId::MainhandShieldDamage),
            );
            let mainhand_weapon = &combatant.sheet.offense.weapon;
            let effective_damage_expr = mainhand_weapon.damage_expr_for_attack();
            let damage_roll = if mainhand_weapon.halves_damage_for_attack() {
                format!("({effective_damage_expr} + {strength_damage}) / 2")
            } else {
                format!("{effective_damage_expr} + {strength_damage}")
            };
            if player.called_shot {
                derived_stat_line(
                    ui,
                    format!(
                        "Attack roll: d20p + {} (called shot: hit if > defense; precise at defense +{} vs current target armor, light/medium/heavy +{}/+{}/+{})",
                        attack_bonus,
                        called_shot_target_bonus_vs_opponent,
                        called_shot_light_bonus,
                        called_shot_medium_bonus,
                        called_shot_heavy_bonus
                    ),
                    breakdowns.get(game_logic::DerivedStatId::MainhandAttackRoll),
                );
                derived_stat_line(
                    ui,
                    format!(
                        "Damage roll: {} vs target DR {} on precise called shot (near-miss DR {}, AP {})",
                        damage_roll,
                        target_natural_dr,
                        target_armor_dr,
                        mainhand_weapon.armor_penetration
                    ),
                    breakdowns.get(game_logic::DerivedStatId::MainhandDamageRoll),
                );
            } else {
                derived_stat_line(
                    ui,
                    format!("Attack roll: d20p + {}", attack_bonus),
                    breakdowns.get(game_logic::DerivedStatId::MainhandAttackRoll),
                );
                derived_stat_line(
                    ui,
                    format!(
                        "Damage roll: {} vs target DR {} (AP {})",
                        damage_roll, target_armor_dr, mainhand_weapon.armor_penetration
                    ),
                    breakdowns.get(game_logic::DerivedStatId::MainhandDamageRoll),
                );
            }
            if let Some(offhand) = combatant.sheet.offense.offhand.as_ref() {
                ui.separator();
                ui.label("Offhand");
                derived_stat_line(
                    ui,
                    format!("Weapon speed: {}", offhand.weapon.speed),
                    breakdowns.get(game_logic::DerivedStatId::OffhandWeaponSpeed),
                );
                let offhand_shield_damage =
                    offhand.weapon.shield_damage_expr.as_deref().unwrap_or("-");
                derived_stat_line(
                    ui,
                    format!("Weapon shield damage: {}", offhand_shield_damage),
                    breakdowns.get(game_logic::DerivedStatId::OffhandShieldDamage),
                );
                derived_stat_line(
                    ui,
                    format!("Attack roll: d20p + {}", offhand.attack_bonus),
                    breakdowns.get(game_logic::DerivedStatId::OffhandAttackRoll),
                );
                derived_stat_line(
                    ui,
                    format!(
                        "Damage roll: {} + {} {:+} vs target DR {} (AP {})",
                        offhand.weapon.damage_expr,
                        offhand.strength_damage,
                        combatant.sheet.maneuvers.dualwield_offhand_damage_penalty,
                        target_armor_dr,
                        offhand.weapon.armor_penetration
                    ),
                    breakdowns.get(game_logic::DerivedStatId::OffhandDamageRoll),
                );
            }
        }
        PlayerEditorTab::Tools => {
            render_player_tools_tab(
                ui,
                id_prefix,
                player,
                weapon_catalog,
                armor_catalog,
                shield_catalog,
                npc_presets,
                talent_catalog,
                damage_plot_iterations,
                damage_roll_plot,
                player_names,
                dps_attacker_idx,
                dps_defender_idx,
                dps_iterations,
                dps_duration_seconds,
                dps_seed,
                dps_result,
                dps_sim_duration,
                run_dps_test,
            );
        }
    }
}

fn render_player_tools_tab(
    ui: &mut egui::Ui,
    id_prefix: &str,
    player: &PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
    npc_presets: &NpcPresetCatalog,
    talent_catalog: &TalentCatalog,
    damage_plot_iterations: &mut String,
    damage_roll_plot: &mut Option<DamageRollPlotData>,
    player_names: &[String; 2],
    dps_attacker_idx: &mut usize,
    dps_defender_idx: &mut usize,
    dps_iterations: &mut u32,
    dps_duration_seconds: &mut u32,
    dps_seed: &mut u64,
    dps_result: &Option<DpsTestResult>,
    dps_sim_duration: Option<std::time::Duration>,
    run_dps_test: &mut bool,
) {
    ui.heading("Tools");
    ui.separator();
    render_dps_test_tool(
        ui,
        player_names,
        dps_attacker_idx,
        dps_defender_idx,
        dps_iterations,
        dps_duration_seconds,
        dps_seed,
        dps_result,
        dps_sim_duration,
        run_dps_test,
    );
    ui.separator();
    ui.horizontal(|ui| {
        let iterations = parse_damage_plot_iterations(damage_plot_iterations);
        let plot_enabled = iterations.is_some();
        if ui
            .add_enabled(
                plot_enabled,
                egui::Button::new("Plot damage rolls (pre opponent DR)"),
            )
            .clicked()
        {
            if let Some(iterations) = iterations {
                let iterations = iterations.clamp(1, MAX_DAMAGE_PLOT_ITERATIONS);
                *damage_roll_plot = Some(build_damage_roll_plot(
                    player,
                    weapon_catalog,
                    armor_catalog,
                    shield_catalog,
                    npc_presets,
                    talent_catalog,
                    iterations,
                ));
            }
        }
        ui.label("Iterations");
        ui.add(
            egui::TextEdit::singleline(damage_plot_iterations)
                .id_source(format!("{id_prefix}_damage_plot_iterations"))
                .desired_width(90.0),
        );
    });

    match parse_damage_plot_iterations(damage_plot_iterations) {
        Some(iterations) if iterations > MAX_DAMAGE_PLOT_ITERATIONS => {
            ui.small(format!(
                "Iterations will be capped at {}.",
                MAX_DAMAGE_PLOT_ITERATIONS
            ));
        }
        None => {
            ui.small("Enter a positive whole number of iterations.");
        }
        Some(_) => {}
    }

    ui.separator();
    if let Some(plot) = damage_roll_plot {
        show_damage_roll_plot(ui, &format!("{id_prefix}_damage_roll_plot"), plot);
    } else {
        ui.label("No damage roll plot yet.");
    }
}

fn parse_damage_plot_iterations(input: &str) -> Option<usize> {
    let cleaned = input
        .chars()
        .filter(|ch| *ch != ',' && *ch != '_')
        .collect::<String>();
    let value = cleaned.trim().parse::<usize>().ok()?;
    (value > 0).then_some(value)
}

fn build_damage_roll_plot(
    player: &PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
    npc_presets: &NpcPresetCatalog,
    talent_catalog: &TalentCatalog,
    iterations: usize,
) -> DamageRollPlotData {
    let combatant = game_logic::build_combatant(
        player,
        weapon_catalog,
        armor_catalog,
        shield_catalog,
        npc_presets,
        talent_catalog,
    );
    let mut rng = StdRng::from_entropy();
    let mut entries = vec![(
        format!("Mainhand: {}", combatant.sheet.offense.weapon.name),
        combatant.sheet.offense.weapon.clone(),
        combatant.sheet.offense.strength_damage,
        0,
        Color32::from_rgb(214, 93, 69),
    )];
    if let Some(offhand) = combatant.sheet.offense.offhand.as_ref() {
        entries.push((
            format!("Offhand: {}", offhand.weapon.name),
            offhand.weapon.clone(),
            offhand.strength_damage,
            combatant.sheet.maneuvers.dualwield_offhand_damage_penalty,
            Color32::from_rgb(70, 140, 210),
        ));
    }

    let mut lines = Vec::with_capacity(entries.len());
    let mut x_max = 0usize;
    let mut y_max = 0.0f64;

    for (name, weapon, strength_damage, damage_penalty, color) in entries {
        let mut counts = Vec::<usize>::new();
        let mut total = 0i64;
        for _ in 0..iterations {
            let raw =
                roll_weapon_raw_damage(weapon.as_ref(), strength_damage, damage_penalty, &mut rng);
            let raw_idx = raw.max(0) as usize;
            if raw_idx >= counts.len() {
                counts.resize(raw_idx + 1, 0);
            }
            counts[raw_idx] += 1;
            total += i64::from(raw);
        }

        let mut points = Vec::with_capacity(counts.len());
        let mut values = Vec::with_capacity(counts.len());
        let denom = iterations.max(1) as f64;
        for (damage, count) in counts.into_iter().enumerate() {
            let frequency = count as f64 / denom;
            points.push([damage as f64, frequency]);
            values.push(frequency);
            y_max = y_max.max(frequency);
            x_max = x_max.max(damage);
        }

        lines.push(DamageRollLine {
            name,
            color,
            points,
            values,
            average: total as f64 / denom,
        });
    }

    DamageRollPlotData {
        lines,
        iterations,
        x_max: x_max.max(1),
        y_max: y_max.max(0.01),
    }
}

fn roll_weapon_raw_damage(
    weapon: &sim::WeaponProfile,
    strength_damage: i32,
    damage_penalty: i32,
    rng: &mut impl rand::Rng,
) -> i32 {
    let nonpenetrating = if weapon.use_jab {
        true
    } else {
        weapon.force_nonpenetrating_damage
    };
    let rolled_damage = weapon
        .damage_expr_cache_for_attack()
        .roll(rng, nonpenetrating);
    let mut raw = rolled_damage + strength_damage;
    if weapon.halves_damage_for_attack() {
        raw /= 2;
    }
    if weapon.halve_damage {
        raw /= 2;
    }
    raw += damage_penalty;
    raw.max(0)
}

fn show_damage_roll_plot(ui: &mut egui::Ui, plot_id: &str, plot: &DamageRollPlotData) {
    ui.heading(format!("Damage Rolls ({} iterations)", plot.iterations));
    if plot.lines.is_empty() {
        ui.label("No damage profiles available.");
        return;
    }

    let x_max = plot.x_max as f64 + 2.0;
    let y_view = (plot.y_max * 1.2).max(0.05);
    let y_floor = -y_view * 0.12;
    let mut hovered_damage = None;
    let mut hovered_values = Vec::new();

    let response = Plot::new(plot_id)
        .legend(Legend::default())
        .include_x(-1.0)
        .include_x(x_max)
        .include_y(y_floor)
        .include_y(y_view)
        .view_aspect(16.0 / 9.0)
        .allow_scroll(false)
        .allow_boxed_zoom(false)
        .x_grid_spacer(integer_grid_marks)
        .x_axis_label("Damage roll")
        .y_axis_label("Frequency")
        .x_axis_formatter(|mark, _, _| format!("{:.0}", mark.value))
        .show(ui, |plot_space| {
            let pointer = plot_space.pointer_coordinate();
            let snapped = if plot_space.response().hovered() {
                pointer.map(|pos| pos.x.round().clamp(0.0, plot.x_max as f64))
            } else {
                None
            };

            plot_space.hline(HLine::new(0.0).color(Color32::LIGHT_GRAY));
            let tick_step = damage_axis_tick_step(plot.x_max);
            let tick_label_y = y_floor * 0.45;
            for damage in (0..=plot.x_max).step_by(tick_step) {
                plot_space.vline(VLine::new(damage as f64).color(Color32::from_gray(60)));
                plot_space.text(
                    Text::new(
                        PlotPoint::new(damage as f64, tick_label_y),
                        damage.to_string(),
                    )
                    .color(Color32::from_gray(210))
                    .anchor(egui::Align2::CENTER_CENTER),
                );
            }
            if plot.x_max % tick_step != 0 {
                plot_space.vline(VLine::new(plot.x_max as f64).color(Color32::from_gray(60)));
                plot_space.text(
                    Text::new(
                        PlotPoint::new(plot.x_max as f64, tick_label_y),
                        plot.x_max.to_string(),
                    )
                    .color(Color32::from_gray(210))
                    .anchor(egui::Align2::CENTER_CENTER),
                );
            }

            for line in &plot.lines {
                let points = PlotPoints::from_iter(line.points.iter().copied());
                plot_space.line(
                    Line::new(points)
                        .name(line.name.clone())
                        .color(line.color)
                        .highlight(true),
                );
            }

            if let Some(snapped_x) = snapped {
                plot_space.vline(VLine::new(snapped_x).color(Color32::LIGHT_GRAY));
                for line in &plot.lines {
                    if let Some(&value) = line.values.get(snapped_x as usize) {
                        plot_space.points(
                            Points::new(vec![[snapped_x, value]])
                                .radius(4.0)
                                .color(line.color)
                                .name(line.name.clone()),
                        );
                    }
                }
            }

            snapped
        });

    if let Some(damage) = response.inner {
        let damage_idx = damage as usize;
        hovered_damage = Some(damage_idx);
        for line in &plot.lines {
            if let Some(&frequency) = line.values.get(damage_idx) {
                hovered_values.push((line.color, line.name.as_str(), frequency));
            }
        }
    }

    ui.horizontal_wrapped(|ui| {
        for line in &plot.lines {
            ui.colored_label(line.color, format!("{} avg {:.2}", line.name, line.average));
        }
    });

    if let Some(damage) = hovered_damage {
        ui.label(format!("Damage {damage}"));
        for (color, name, frequency) in hovered_values {
            ui.colored_label(color, format!("{name}: {:.2}%", frequency * 100.0));
        }
    } else {
        ui.label("Hover inside the chart to inspect a damage result.");
    }
}

fn damage_axis_tick_step(x_max: usize) -> usize {
    if x_max <= 24 {
        1
    } else {
        x_max.div_ceil(12).max(1)
    }
}

fn integer_grid_marks(input: GridInput) -> Vec<GridMark> {
    let min = input.bounds.0.floor() as i32;
    let max = input.bounds.1.ceil() as i32;
    (min..=max)
        .map(|value| GridMark {
            value: value as f64,
            step_size: 1.0,
        })
        .collect()
}

fn apply_fighter_preset(
    player: &mut PlayerConfig,
    preset: &FighterPreset,
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
    race_catalog: &[RaceSpec],
) {
    let attack = tier_from_label(&preset.progression.attack).unwrap_or(ProgressionTier::I);
    let speed = tier_from_label(&preset.progression.speed).unwrap_or(ProgressionTier::I);
    let initiative = tier_from_label(&preset.progression.initiative).unwrap_or(ProgressionTier::I);
    let health = tier_from_label(&preset.progression.health).unwrap_or(ProgressionTier::I);
    player.name = preset.name.clone();
    player.level = preset.level;
    player.progression = Progression::new(attack, speed, initiative, health);
    player.mastery_attack = game_logic::clamp_mastery(preset.masteries.attack);
    player.mastery_defense = game_logic::clamp_mastery(preset.masteries.defense);
    player.mastery_damage = game_logic::clamp_mastery(preset.masteries.damage);
    player.mastery_speed = game_logic::clamp_mastery(preset.masteries.speed);
    player.shield_mastery_defense = game_logic::clamp_mastery(preset.masteries.shield_defense);
    player.shield_mastery_speed = game_logic::clamp_mastery(preset.masteries.shield_speed);
    player.base_hp = preset.base_hp;
    player.move_speed = preset.move_speed;
    player.strength_base = preset.strength_base;
    player.strength_pct = game_logic::normalize_percentile(preset.strength_pct);
    player.dex_base = preset.dex_base;
    player.dex_pct = game_logic::normalize_percentile(preset.dex_pct);
    player.intelligence = preset.intelligence;
    player.wisdom = preset.wisdom;
    player.constitution = preset.constitution;
    player.looks = preset.looks;
    player.charisma = preset.charisma;
    player.weapon_material_tier = preset.weapon_material_tier;
    player.offhand_weapon_material_tier = preset.offhand_weapon_material_tier;
    player.armor_material_tier = preset.armor_material_tier;
    player.projectile_material_tier = preset.projectile_material_tier;
    player.offhand_projectile_material_tier = preset.offhand_projectile_material_tier;
    player.shield_material_tier = preset.shield_material_tier;
    player.two_hand_grip = preset.two_hand_grip;
    let maneuvers = preset.maneuvers;
    player.use_jab = maneuvers.use_jab;
    player.hold_at_bay = maneuvers.hold_at_bay;
    player.called_shot = maneuvers.called_shot;
    player.power_attack = maneuvers.power_attack;
    player.aggressive_attack = maneuvers.aggressive_attack;
    player.charge = maneuvers.charge;
    player.ready_against_charge = maneuvers.ready_against_charge;
    player.tactical_move = maneuvers.tactical_move;
    player.fight_defensively = maneuvers.fight_defensively;
    player.fight_defensively_penalty = maneuvers.fight_defensively_penalty;
    player.full_parry = maneuvers.full_parry;
    player.give_ground = maneuvers.give_ground;
    player.scamper_back = maneuvers.scamper_back;
    player.fighting_withdrawal = maneuvers.fighting_withdrawal;
    player.flee = maneuvers.flee;
    player.mounted = maneuvers.mounted;
    player.defensive_dualwielding = preset.defensive_dualwielding;
    player.offensive_dualwielding = preset.offensive_dualwielding;
    player.environment = game_logic::EnvironmentConfig::default();
    player.misc_modifiers = game_logic::MiscRollModifiers::default();
    player.proficiencies = preset.proficiencies.clone();
    player.talents = preset.talents.clone();
    player.default_weapon_style_ids = preset.default_weapon_style_ids.clone();
    player.weapon_id = find_weapon_id_by_name(weapon_catalog, &preset.weapon)
        .or_else(|| weapon_catalog.first_id())
        .unwrap_or(WeaponId::new(0));
    player.offhand_weapon_id = preset
        .offhand_weapon
        .as_deref()
        .and_then(|name| find_weapon_id_by_name(weapon_catalog, name));
    player.armor_id = find_armor_id_by_name(armor_catalog, &preset.armor)
        .or_else(|| armor_catalog.first_id())
        .unwrap_or(ArmorId::new(0));
    player.shield_id = find_shield_id_by_name(shield_catalog, &preset.shield)
        .or_else(|| shield_catalog.first_id())
        .unwrap_or(ShieldId::new(0));
    player.race_id = preset.race_id.clone();
    player.race_applied = false;
    player.knockback_step =
        game_logic::knockback_step_for_race_id(player.race_id.as_deref(), race_catalog);
    if let Some(weapon) = weapon_catalog.get(player.weapon_id) {
        game_logic::sanitize_projectile_tier(player, weapon);
    }
}

fn fighter_preset_from_player(
    player: &PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
    name: &str,
) -> FighterPreset {
    let weapon = weapon_catalog
        .get(player.weapon_id)
        .map(|weapon| weapon.name.clone())
        .unwrap_or_else(|| "Fist".to_string());
    let armor = armor_catalog
        .get(player.armor_id)
        .and_then(|entry| entry.armor.as_ref().map(|armor| armor.name.clone()))
        .unwrap_or_else(|| "None".to_string());
    let shield = shield_catalog
        .get(player.shield_id)
        .and_then(|entry| entry.shield.as_ref().map(|shield| shield.name.clone()))
        .unwrap_or_else(|| "None".to_string());
    let offhand_weapon = player
        .offhand_weapon_id
        .and_then(|id| weapon_catalog.get(id))
        .map(|weapon| weapon.name.clone());
    FighterPreset {
        name: name.to_string(),
        level: player.level,
        progression: FighterProgression {
            attack: tier_label(player.progression.attack).to_string(),
            speed: tier_label(player.progression.speed).to_string(),
            initiative: tier_label(player.progression.initiative).to_string(),
            health: tier_label(player.progression.health).to_string(),
        },
        masteries: FighterMasteries {
            attack: game_logic::clamp_mastery(player.mastery_attack),
            defense: game_logic::clamp_mastery(player.mastery_defense),
            damage: game_logic::clamp_mastery(player.mastery_damage),
            speed: game_logic::clamp_mastery(player.mastery_speed),
            shield_defense: game_logic::clamp_mastery(player.shield_mastery_defense),
            shield_speed: game_logic::clamp_mastery(player.shield_mastery_speed),
        },
        base_hp: player.base_hp,
        move_speed: player.move_speed,
        strength_base: player.strength_base,
        strength_pct: player.strength_pct,
        dex_base: player.dex_base,
        dex_pct: player.dex_pct,
        intelligence: player.intelligence,
        wisdom: player.wisdom,
        constitution: player.constitution,
        looks: player.looks,
        charisma: player.charisma,
        weapon,
        offhand_weapon,
        armor,
        shield,
        weapon_material_tier: player.weapon_material_tier,
        offhand_weapon_material_tier: player.offhand_weapon_material_tier,
        armor_material_tier: player.armor_material_tier,
        projectile_material_tier: player.projectile_material_tier,
        offhand_projectile_material_tier: player.offhand_projectile_material_tier,
        shield_material_tier: player.shield_material_tier,
        two_hand_grip: player.two_hand_grip,
        maneuvers: game_logic::CombatManeuverConfig {
            use_jab: player.use_jab,
            hold_at_bay: player.hold_at_bay,
            called_shot: player.called_shot,
            power_attack: player.power_attack,
            aggressive_attack: player.aggressive_attack,
            charge: player.charge,
            ready_against_charge: player.ready_against_charge,
            tactical_move: player.tactical_move,
            fight_defensively: player.fight_defensively,
            fight_defensively_penalty: game_logic::normalize_fight_defensively_penalty(
                player.fight_defensively_penalty,
            ),
            full_parry: player.full_parry,
            give_ground: player.give_ground,
            scamper_back: player.scamper_back,
            fighting_withdrawal: player.fighting_withdrawal,
            flee: player.flee,
            mounted: player.mounted,
        },
        defensive_dualwielding: player.defensive_dualwielding,
        offensive_dualwielding: player.offensive_dualwielding,
        race_id: player.race_id.clone(),
        proficiencies: player.proficiencies.clone(),
        talents: player.talents.clone(),
        default_weapon_style_ids: player.default_weapon_style_ids.clone(),
    }
}

fn tier_from_label(label: &str) -> Option<ProgressionTier> {
    match label.trim() {
        "I" | "1" => Some(ProgressionTier::I),
        "II" | "2" => Some(ProgressionTier::II),
        "III" | "3" => Some(ProgressionTier::III),
        "IV" | "4" => Some(ProgressionTier::IV),
        "V" | "5" => Some(ProgressionTier::V),
        "VI" | "6" => Some(ProgressionTier::VI),
        _ => None,
    }
}

fn find_weapon_id_by_name(catalog: &WeaponCatalog, name: &str) -> Option<WeaponId> {
    catalog
        .entries()
        .iter()
        .position(|weapon| weapon.name.eq_ignore_ascii_case(name))
        .and_then(|idx| catalog.id_from_index(idx))
}

fn find_armor_id_by_name(catalog: &ArmorCatalog, name: &str) -> Option<ArmorId> {
    if name.eq_ignore_ascii_case("None") {
        return catalog.first_id();
    }
    catalog
        .entries()
        .iter()
        .position(|entry| {
            entry
                .armor
                .as_ref()
                .map(|armor| armor.name.eq_ignore_ascii_case(name))
                .unwrap_or(false)
        })
        .and_then(|idx| catalog.id_from_index(idx))
}

fn find_shield_id_by_name(catalog: &ShieldCatalog, name: &str) -> Option<ShieldId> {
    if name.eq_ignore_ascii_case("None") {
        return catalog.first_id();
    }
    catalog
        .entries()
        .iter()
        .position(|entry| {
            entry
                .shield
                .as_ref()
                .map(|shield| shield.name.eq_ignore_ascii_case(name))
                .unwrap_or(false)
        })
        .and_then(|idx| catalog.id_from_index(idx))
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

fn ability_percentile_editor(
    ui: &mut egui::Ui,
    id: &str,
    label: &str,
    base: &mut u8,
    percentile: &mut u8,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::Slider::new(base, 1..=25).step_by(1.0));
        let mut selection = *percentile;
        searchable_select(
            ui,
            id,
            format!("{:02}", percentile),
            &mut selection,
            [
                (1u8, "01".to_string(), true),
                (51u8, "51".to_string(), true),
            ],
        );
        *percentile = selection;
    });
}

fn ability_slider(ui: &mut egui::Ui, label: &str, value: &mut u8) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::Slider::new(value, 1..=25).step_by(1.0));
    });
}

fn mastery_slider(ui: &mut egui::Ui, label: &str, value: &mut i32) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::Slider::new(value, 0..=6).step_by(1.0));
    });
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
        } => format!("Requires {} {required}+ (current {current}).", stat.label()),
        game_logic::TalentRequirementFailure::MinStatPercentile {
            stat,
            required,
            current,
        } => {
            let current_label = current
                .map(|value| value.to_string())
                .unwrap_or_else(|| "N/A".to_string());
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
            format!("Requires {talent_name} rank {required_rank} (current {current_rank}).")
        }
        game_logic::TalentRequirementFailure::MissingSizeLLargeSwordProficiency => {
            "Requires proficiency in at least one type of size L large sword.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingShieldProficiency => {
            "Requires shield proficiency.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingArmerociPoleProficiency => {
            "Requires proficiency with a large sword or polearm with at least 5 feet of reach."
                .to_string()
        }
        game_logic::TalentRequirementFailure::MissingCrescentMoonProficiency => {
            "Requires proficiency in at least one small sword and one size M large sword."
                .to_string()
        }
        game_logic::TalentRequirementFailure::MissingDoomrazorProficiency => {
            "Requires proficiency in at least one piercing melee weapon.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingFallingSunProficiency => {
            "Requires flamberge or two-handed sword proficiency.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingFymblwngerProficiency => {
            "Requires battle axe, executioner's axe, or greataxe proficiency.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingHammererProficiency => {
            "Requires greathammer, hammer, maul, or warhammer proficiency.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingHobblerProficiency => {
            "Requires proficiency in at least one polearm or spear.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingIthicanPrinceProficiency => {
            "Requires shield proficiency and at least one small sword proficiency.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingQuietRiverProficiency => {
            "Requires fist proficiency.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingRegenstatProficiency => {
            "Requires proficiency in at least one size M small or large sword.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingReturnerProficiency => {
            "Requires proficiency in at least one size L large sword.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingRhdwngFlowProficiency => {
            "Requires proficiency in at least one throwing weapon.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingRohavalanBridgeProficiency => {
            "Requires staff proficiency or polearm proficiency.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingScornOfTheDissendriProficiency => {
            "Requires proficiency in at least one size S melee weapon.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingSwordReachStyleProficiency => {
            "Requires proficiency in at least one size S or M sword with at least 2 feet of reach."
                .to_string()
        }
        game_logic::TalentRequirementFailure::MissingSixPathsProficiency => {
            "Requires shield proficiency and at least one size M large sword proficiency."
                .to_string()
        }
        game_logic::TalentRequirementFailure::MissingThreeMountainsProficiency => {
            "Requires proficiency in at least one crushing melee weapon.".to_string()
        }
        game_logic::TalentRequirementFailure::MissingUnbreakableWallProficiency => {
            "Requires shield proficiency.".to_string()
        }
    }
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

fn race_for_player<'a>(
    player: &PlayerConfig,
    race_catalog: &'a [RaceSpec],
) -> Option<&'a RaceSpec> {
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
    if spec.race_ids.iter().any(|race_id| race_id == &race.id) {
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

fn render_talent_selector(
    ui: &mut egui::Ui,
    id_prefix: &str,
    player: &mut PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    race_catalog: &[RaceSpec],
    talent_catalog: &TalentCatalog,
    active_category: &mut String,
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
    categories
        .entry(TALENT_TAB_RACIALS.to_string())
        .or_default();
    let mut categories: Vec<(String, Vec<&TalentSpec>)> = categories.into_iter().collect();
    let total_count: usize = categories.iter().map(|(_, specs)| specs.len()).sum();
    categories.sort_by(|a, b| a.0.cmp(&b.0));

    if active_category != TALENT_TAB_ALL
        && active_category != TALENT_TAB_LEARNED
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
        let learned_label = format!("{TALENT_TAB_LEARNED} ({})", player.talents.len());
        if ui
            .selectable_label(
                active_category.as_str() == TALENT_TAB_LEARNED,
                learned_label,
            )
            .clicked()
        {
            active_category.clear();
            active_category.push_str(TALENT_TAB_LEARNED);
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
    let proficiency_snapshot = player.proficiencies.clone();
    let context = game_logic::TalentContext {
        level: player.level,
        stats: &abilities,
        talents: &talent_snapshot,
        proficiencies: &proficiency_snapshot,
        weapon_catalog: Some(weapon_catalog),
    };
    let mut add_queue: Vec<TalentSelection> = Vec::new();
    let mut remove_queue: Vec<usize> = Vec::new();
    let default_group = weapon_catalog
        .get(player.weapon_id)
        .map(|weapon| weapon_group_label(weapon.group))
        .unwrap_or(WEAPON_GROUP_LABELS[0]);

    egui::ScrollArea::vertical()
        .max_height(320.0)
        .show(ui, |ui| {
            if active_category.as_str() == TALENT_TAB_LEARNED {
                if player.talents.is_empty() {
                    ui.label("No learned talents.");
                } else {
                    let mut learned_talents: Vec<(String, usize)> = player
                        .talents
                        .iter()
                        .enumerate()
                        .map(|(index, selection)| {
                            (talent_display_label(selection, talent_catalog), index)
                        })
                        .collect();
                    learned_talents.sort_by(|left, right| left.0.cmp(&right.0));
                    for (label, index) in learned_talents {
                        ui.horizontal(|ui| {
                            if ui.button("Remove").clicked() {
                                remove_queue.push(index);
                            }
                            ui.label(label);
                        });
                    }
                }
            } else if active_category.as_str() == TALENT_TAB_ALL {
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
    add_queue: &mut Vec<TalentSelection>,
    remove_queue: &mut Vec<usize>,
) {
    let selected_index = player.talents.iter().position(|sel| sel.id == spec.id);
    let requirement_failures = game_logic::evaluate_talent_requirements(spec, context);
    let locked = !requirement_failures.is_empty();
    let is_nyi = !game_logic::talent_is_implemented(spec);
    let requires_group = game_logic::talent_requires_weapon_group(spec);
    let muted_color = ui.visuals().weak_text_color();
    let can_adjust = !locked && (!is_nyi || requires_group);
    let allow_add = !locked && (!is_nyi || requires_group);
    let allow_force_add = locked && (!is_nyi || requires_group);
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
            } else {
                let add_clicked = ui
                    .add_enabled(allow_add, egui::Button::new("Add"))
                    .clicked();
                let force_add_clicked = if locked {
                    ui.add_enabled(allow_force_add, egui::Button::new("Force add"))
                        .clicked()
                } else {
                    false
                };
                if add_clicked || force_add_clicked {
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
            ui.colored_label(Color32::from_rgb(180, 70, 70), "Requirements not met:");
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
            if max_rank > 1 {
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
                        searchable_select(
                            ui,
                            format!("{id_prefix}_talent_group_{}", spec.id),
                            selected_text,
                            &mut selection.weapon,
                            WEAPON_GROUP_LABELS
                                .into_iter()
                                .map(|label| (Some(label.to_string()), label.to_string(), true)),
                        );
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
                        searchable_select(
                            ui,
                            format!("{id_prefix}_talent_weapon_{}", spec.id),
                            selected_text,
                            &mut selection.weapon,
                            weapon_catalog.entries().iter().map(|weapon| {
                                (Some(weapon.name.clone()), weapon.name.clone(), true)
                            }),
                        );
                    });
                });
            }
        }
    });
}

fn tier_label(tier: ProgressionTier) -> &'static str {
    match tier {
        ProgressionTier::I => "I",
        ProgressionTier::II => "II",
        ProgressionTier::III => "III",
        ProgressionTier::IV => "IV",
        ProgressionTier::V => "V",
        ProgressionTier::VI => "VI",
    }
}

fn tier_combo(
    ui: &mut egui::Ui,
    id_source: String,
    label: &str,
    selection: &mut ProgressionTier,
    tiers: &[ProgressionTier],
) {
    ui.label(label);
    let mut value = *selection;
    searchable_select(
        ui,
        id_source,
        tier_label(*selection),
        &mut value,
        tiers
            .iter()
            .map(|tier| (*tier, tier_label(*tier).to_string(), true)),
    );
    *selection = value;
}

fn material_tier_combo(ui: &mut egui::Ui, id_source: String, label: &str, selection: &mut i32) {
    ui.label(label);
    searchable_select(
        ui,
        id_source,
        format!("+{selection}"),
        selection,
        (0..=5).map(|tier| (tier, format!("+{tier}"), true)),
    );
}

fn armor_display_name(entry: Option<&ArmorEntry>) -> String {
    entry
        .map(|armor| armor.label.clone())
        .unwrap_or_else(|| "None".to_string())
}

fn main() -> eframe::Result<()> {
    hackmaster_sim::console::maybe_enable_console();
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([980.0, 560.0])
        .with_min_inner_size([640.0, 360.0]);
    if let Some(icon) = hackmaster_sim::assets::app_icon(hackmaster_sim::assets::AppIcon::SimGui) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "HackMaster Simulator",
        options,
        Box::new(|_cc| Box::new(SimGuiApp::new())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reopening_customize_refreshes_tactical_draft_from_live_policy() {
        let active = TacticalPolicy {
            enabled: true,
            rules: vec![TacticalRule::new(
                TacticalAction::NormalAttack,
                vec![TacticalCondition::Always],
            )],
        };
        let mut draft = TacticalPolicy::default();

        assert!(sync_tactical_draft_on_open(
            false, true, &active, &mut draft
        ));
        assert_eq!(draft, active);
        assert!(!sync_tactical_draft_on_open(
            true, true, &active, &mut draft
        ));
    }

    #[test]
    fn tactical_enable_checkbox_updates_live_policy() {
        let mut active = TacticalPolicy {
            enabled: true,
            rules: vec![TacticalRule::new(
                TacticalAction::Jab,
                vec![TacticalCondition::MyWeaponCanJab { value: true }],
            )],
        };
        let saved_rules = active.rules.clone();
        let mut draft = active.clone();
        draft.enabled = false;

        apply_tactical_enabled_toggle(&mut active, &draft).expect("disable should be valid");
        assert!(!active.enabled);
        assert_eq!(active.rules, saved_rules);

        draft.enabled = true;
        draft.rules = vec![TacticalRule::new(
            TacticalAction::NormalAttack,
            vec![TacticalCondition::Always],
        )];
        apply_tactical_enabled_toggle(&mut active, &draft).expect("enable should be valid");
        assert_eq!(active, draft);
    }

    #[test]
    fn tactical_rule_edits_immediately_replace_live_policy_and_survive_reopen() {
        let mut active = TacticalPolicy {
            enabled: true,
            rules: vec![
                TacticalRule::new(
                    TacticalAction::Jab,
                    vec![TacticalCondition::MyWeaponCanJab { value: true }],
                ),
                TacticalRule::new(
                    TacticalAction::NormalAttack,
                    vec![TacticalCondition::Always],
                ),
            ],
        };
        let mut draft = active.clone();
        draft.rules[0].enabled = false;
        draft.rules.remove(1);

        assert!(
            apply_tactical_ui_policy(&mut active, &draft).expect("edited policy should be valid")
        );
        assert_eq!(active, draft);
        assert!(!active.rules[0].enabled);
        assert_eq!(active.rules.len(), 1);

        let mut reopened_draft = TacticalPolicy::default();
        assert!(sync_tactical_draft_on_open(
            false,
            true,
            &active,
            &mut reopened_draft,
        ));
        assert_eq!(reopened_draft, draft);
    }

    #[test]
    fn deleting_every_tactical_rule_immediately_clears_live_policy() {
        let mut active = TacticalPolicy {
            enabled: true,
            rules: vec![TacticalRule::new(
                TacticalAction::NormalAttack,
                vec![TacticalCondition::Always],
            )],
        };
        let draft = TacticalPolicy {
            enabled: true,
            rules: Vec::new(),
        };

        assert!(
            apply_tactical_ui_policy(&mut active, &draft).expect("empty policy should be valid")
        );
        assert!(active.rules.is_empty());
    }

    #[test]
    fn simulation_rebuild_uses_policy_applied_from_tactical_ui() {
        let mut app = SimGuiApp::new();
        let draft = TacticalPolicy {
            enabled: true,
            rules: vec![TacticalRule::new(
                TacticalAction::NormalAttack,
                vec![TacticalCondition::Always],
            )],
        };

        assert!(
            apply_tactical_ui_policy(&mut app.players[0].tactical_policy, &draft)
                .expect("UI policy should be valid")
        );
        app.reset_positions();

        assert_eq!(app.players[0].tactical_policy, draft);
        assert_eq!(app.sim.combatants[0].tactical_policy, draft);
    }

    #[test]
    fn maximum_start_distance_builds_a_valid_simulation_grid() {
        let config = SimConfig::new(MAX_START_DISTANCE_FT, 1.0);

        assert_eq!(config.start_distance, 4_000.0);
        assert!(config.grid_width > 4_000);
    }

    #[test]
    fn changing_start_distance_resizes_grid_and_respawns_at_requested_distance() {
        let mut app = SimGuiApp::new();

        app.sim.config.set_start_distance(MAX_START_DISTANCE_FT);
        app.reset_positions();

        assert_eq!(app.sim.distance(), MAX_START_DISTANCE_FT);
        assert!(app.sim.config.grid_width > MAX_START_DISTANCE_FT as i32);
    }

    #[test]
    fn every_tactical_policy_change_preserves_cached_winrate() {
        let mut app = SimGuiApp::new();
        app.bulk_runs = 1;
        app.run_bulk_sim();
        assert!(app.bulk_result.is_some());

        let enabled_draft = TacticalPolicy {
            enabled: true,
            rules: vec![TacticalRule::new(
                TacticalAction::NormalAttack,
                vec![TacticalCondition::Always],
            )],
        };
        assert!(
            apply_tactical_ui_policy(&mut app.players[0].tactical_policy, &enabled_draft)
                .expect("UI policy should be valid")
        );
        app.reset_positions();
        assert!(app.bulk_result.is_some());
        assert!(app.bulk_last_seed.is_some());

        let mut disabled_draft = enabled_draft;
        disabled_draft.enabled = false;
        apply_tactical_enabled_toggle(&mut app.players[0].tactical_policy, &disabled_draft)
            .expect("disabling tactics should be valid");
        app.reset_positions();
        assert!(app.bulk_result.is_some());
        assert!(app.bulk_last_seed.is_some());
    }

    #[test]
    fn disorder_wound_counts_complete_fifty_essence_bands() {
        assert_eq!(disorder_wound(1000, 549), 0);
        assert_eq!(disorder_wound(1000, 550), 1);
        assert_eq!(disorder_wound(1000, 450), 1);
        assert_eq!(disorder_wound(501, 300), 0);
        assert_eq!(disorder_wound(501, 301), 1);
    }

    #[test]
    fn upward_movement_from_below_equilibrium_causes_solar_wound() {
        assert_eq!(
            calculate_essence_wounds(1000, 350, 100),
            EssenceWoundResult {
                current_essence: 450,
                disorder_wound: 1,
                solar_wound: 2,
                lunar_wound: 0,
            }
        );
    }

    #[test]
    fn downward_movement_from_above_equilibrium_causes_lunar_wound() {
        assert_eq!(
            calculate_essence_wounds(1000, 650, -100),
            EssenceWoundResult {
                current_essence: 550,
                disorder_wound: 1,
                solar_wound: 0,
                lunar_wound: 2,
            }
        );
    }

    #[test]
    fn direction_or_unchanged_disorder_can_prevent_solar_and_lunar_wounds() {
        let moving_down_from_below = calculate_essence_wounds(1000, 400, -100);
        assert_eq!(moving_down_from_below.solar_wound, 0);
        assert_eq!(moving_down_from_below.lunar_wound, 0);

        let same_disorder_band = calculate_essence_wounds(1000, 420, 10);
        assert_eq!(same_disorder_band.solar_wound, 0);
        assert_eq!(same_disorder_band.lunar_wound, 0);
    }

    #[test]
    fn current_essence_is_clamped_to_the_valid_range() {
        assert_eq!(
            calculate_essence_wounds(1000, 900, i64::MAX).current_essence,
            1000
        );
        assert_eq!(
            calculate_essence_wounds(1000, 100, i64::MIN).current_essence,
            0
        );
    }
}
