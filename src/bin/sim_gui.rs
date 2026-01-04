#[path = "../character.rs"]
mod character;
#[path = "../sim.rs"]
mod sim;
#[path = "../game_logic.rs"]
mod game_logic;

use character::{Progression, ProgressionTier, WeaponGroup};
use eframe::egui::{self, Color32, Pos2, Rect};
use sim::{SimConfig, SimState};
use game_logic::{
    ArmorEntry, FighterMasteries, FighterPreset, FighterProgression, NpcPreset, PlayerConfig,
    ShieldEntry, WeaponHandedness, WeaponPreset, WeaponSize,
};

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

const FIGHTER_PRESETS_PATH: &str = "data/fighter_presets.json";

struct SimGuiApp {
    running: bool,
    sim: SimState,
    players: [PlayerConfig; 2],
    weapon_catalog: Vec<WeaponPreset>,
    armor_catalog: Vec<ArmorEntry>,
    shield_catalog: Vec<ShieldEntry>,
    npc_presets: Vec<NpcPreset>,
    fighter_presets: Vec<FighterPreset>,
    fighter_preset_names: [String; 2],
    time_scale: f32,
    show_player_editor: [bool; 2],
    last_screen_size: egui::Vec2,
}

impl SimGuiApp {
    fn new() -> Self {
        let (weapon_catalog, armor_catalog, shield_catalog) = match game_logic::load_catalogs() {
            Ok((weapons, armors, shields)) => (weapons, armors, shields),
            Err(err) => {
                eprintln!("Failed to load JSON catalogs: {err}");
                (
                    game_logic::default_weapon_catalog(),
                    game_logic::default_armor_catalog(),
                    game_logic::default_shield_catalog(),
                )
            }
        };
        let npc_presets = match game_logic::load_npc_presets("data/npc_presets.json") {
            Ok(presets) => presets,
            Err(err) => {
                eprintln!("Failed to load NPC presets: {err}");
                Vec::new()
            }
        };
        let fighter_presets = match game_logic::load_fighter_presets(FIGHTER_PRESETS_PATH) {
            Ok(presets) => presets,
            Err(err) => {
                eprintln!("Failed to load fighter presets: {err}");
                Vec::new()
            }
        };
        let sim = SimState::new(SimConfig::new(200.0, 1.0));
        let mut app = Self {
            running: false,
            sim,
            players: [
                PlayerConfig::new("Fighter A", Color32::from_rgb(214, 93, 69), 1),
                PlayerConfig::new("Fighter B", Color32::from_rgb(70, 140, 210), 2),
            ],
            weapon_catalog,
            armor_catalog,
            shield_catalog,
            npc_presets,
            fighter_presets,
            fighter_preset_names: ["Fighter A".to_string(), "Fighter B".to_string()],
            time_scale: 1.0,
            show_player_editor: [false, false],
            last_screen_size: egui::vec2(0.0, 0.0),
        };
        app.reset_positions();
        app
    }

    fn reset_positions(&mut self) {
        let combatants = game_logic::build_combatants(
            &self.players,
            &self.weapon_catalog,
            &self.armor_catalog,
            &self.shield_catalog,
            &self.npc_presets,
        );
        self.sim.reset_with_combatants(combatants);
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

        let mut x0 = left + self.sim.actors[0].position * scale;
        let mut x1 = left + self.sim.actors[1].position * scale;
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
            let knocked_back = combatant.state.knockback_immobile_seconds > 0;
            let downed =
                combatant.state.hp <= 0 || combatant.state.trauma_remaining_seconds > 0;
            let weapon_icon = self
                .weapon_catalog
                .get(player.weapon_index)
                .map(|weapon| weapon_icon_kind(weapon.group))
                .unwrap_or(WeaponIcon::Other);
            self.draw_person(
                painter,
                Pos2::new(x, ground_y),
                facing,
                player.color,
                downed,
                knocked_back,
                weapon_icon,
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
            let hp = self.sim.combatants[idx].state.hp.max(0) as f32;
            let max_hp = self.sim.combatants[idx].sheet.vitals.max_hp.max(1) as f32;
            let hp_ratio = (hp / max_hp).clamp(0.0, 1.0);
            let bar_x = if idx == 0 {
                left
            } else {
                right - bar_width
            };
            let bg_rect = Rect::from_min_size(Pos2::new(bar_x, y), egui::vec2(bar_width, bar_height));
            painter.rect_filled(bg_rect, 3.0, Color32::from_gray(40));
            let fill_width = bar_width * hp_ratio;
            let fill_x = if idx == 0 { bar_x } else { bar_x + (bar_width - fill_width) };
            let fill_rect = Rect::from_min_size(
                Pos2::new(fill_x, y),
                egui::vec2(fill_width, bar_height),
            );
            painter.rect_filled(fill_rect, 3.0, player.color);
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

        for (idx, player) in self.players.iter().enumerate() {
            if let Some(next) = self.sim.combatants[idx].state.next_attack_time {
                let t = (next - now).max(0.0).min(horizon);
                let x = left + t * scale;
                let pos = Pos2::new(x, y - 14.0);
                painter.circle_filled(pos, 6.0, player.color);
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

        let head = Pos2::new(base.x, base.y - 34.0);
        let neck = Pos2::new(base.x, base.y - 26.0);
        let torso = Pos2::new(base.x, base.y - 14.0);
        painter.circle_filled(head, 6.5, head_color);
        painter.line_segment([neck, torso], stroke);
        painter.line_segment(
            [torso, Pos2::new(base.x - 6.0, base.y - 2.0)],
            stroke,
        );
        painter.line_segment(
            [torso, Pos2::new(base.x + 6.0, base.y - 2.0)],
            stroke,
        );
        let arm_start = Pos2::new(base.x, base.y - 22.0);
        let arm_end = Pos2::new(base.x + facing * 12.0, base.y - 18.0);
        painter.line_segment([arm_start, arm_end], stroke);
        draw_weapon_icon(painter, arm_end, facing, weapon_icon);
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
            painter.circle_filled(
                Pos2::new(pos.x - facing * 1.0, pos.y + 1.0),
                1.5,
                accent,
            );
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
                [blade_back, Pos2::new(blade_back.x - facing * 3.0, blade_back.y - 2.0)],
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
            painter.line_segment(
                [pos, Pos2::new(pos.x + facing * 2.5, pos.y - 4.0)],
                stroke,
            );
            painter.line_segment(
                [end, Pos2::new(end.x - facing * 2.5, end.y + 4.0)],
                stroke,
            );
        }
        WeaponIcon::Ensnaring => {
            let end = Pos2::new(pos.x + facing * 10.0, pos.y - 6.0);
            painter.line_segment([pos, end], stroke);
            let ring = Rect::from_center_size(end, egui::vec2(6.0, 6.0));
            painter.rect_stroke(ring, 3.0, (1.0, accent));
            painter.line_segment(
                [Pos2::new(ring.left(), ring.center().y), Pos2::new(ring.right(), ring.center().y)],
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
        let dt = ctx.input(|i| i.unstable_dt).min(0.05) * self.time_scale;
        let screen_rect = ctx.input(|i| i.screen_rect);
        if screen_rect.size() != self.last_screen_size {
            self.last_screen_size = screen_rect.size();
            ctx.request_repaint();
        }
        self.sim.config.stop_distance =
            game_logic::stop_distance_for_players(&self.players, &self.weapon_catalog);
        self.update_sim(dt);

        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(if self.running { "Pause" } else { "Start" }).clicked() {
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
                if ui
                    .add(
                        egui::Slider::new(&mut self.sim.config.start_distance, 0.0..=400.0)
                            .step_by(5.0),
                    )
                    .changed()
                {
                    if !self.running {
                        self.reset_positions();
                    }
                }
                ui.label("Timescale");
                ui.add(egui::Slider::new(&mut self.time_scale, 0.25..=4.0).step_by(0.25));
            });
        });

        egui::SidePanel::left("players")
            .resizable(true)
            .min_width(220.0)
            .show(ctx, |ui| {
                ui.heading("Characters");
                ui.separator();
                for idx in 0..self.players.len() {
                    let weapon_name = self
                        .weapon_catalog
                        .get(self.players[idx].weapon_index)
                        .map(|weapon| weapon.name.as_str())
                        .unwrap_or("Unarmed");
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "{} ({})",
                            self.players[idx].name, weapon_name
                        ));
                        if ui.button("Customize").clicked() {
                            self.show_player_editor[idx] = true;
                        }
                    });
                    ui.label(format!(
                        "Move: {:.0} ft/s",
                        self.players[idx].move_speed
                    ));
                    if idx == 0 {
                        ui.separator();
                    }
                }
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
                if let Some(event) = &self.sim.last_event {
                    ui.separator();
                    ui.label(event);
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
                    self.weapon_catalog[self.players[0].weapon_index].name
                ));
                ui.label(format!(
                    "{}: {}",
                    self.players[1].name,
                    self.weapon_catalog[self.players[1].weapon_index].name
                ));
                ui.separator();
                ui.label("Combat log");
                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show(ui, |ui| {
                        for line in &self.sim.combat_log {
                            ui.label(line);
                        }
                    });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.max_rect();
            let response = ui.allocate_rect(rect, egui::Sense::hover());
            self.draw_arena(ui, response.rect);
        });

        for idx in 0..self.players.len() {
            let name = self.players[idx].name.clone();
            let mut open = self.show_player_editor[idx];
            let title = format!("Customize {name}");
            egui::Window::new(title)
                .id(egui::Id::new(format!("player_editor_{idx}")))
                .open(&mut open)
                .resizable(true)
                .show(ctx, |ui| {
                    let id_prefix = if idx == 0 { "p1" } else { "p2" };
                    let fighter_preset_name = &mut self.fighter_preset_names[idx];
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
                        opponent,
                        &self.weapon_catalog,
                        &self.armor_catalog,
                        &self.shield_catalog,
                        &self.npc_presets,
                        &mut self.fighter_presets,
                        fighter_preset_name,
                    );
                });
            self.show_player_editor[idx] = open;
        }

        if self.running {
            ctx.request_repaint();
        }
    }
}

fn render_player_editor(
    ui: &mut egui::Ui,
    id_prefix: &str,
    player: &mut PlayerConfig,
    opponent: &PlayerConfig,
    weapon_catalog: &[WeaponPreset],
    armor_catalog: &[ArmorEntry],
    shield_catalog: &[ShieldEntry],
    npc_presets: &[NpcPreset],
    fighter_presets: &mut Vec<FighterPreset>,
    fighter_preset_name: &mut String,
) {
    if weapon_catalog.is_empty() {
        ui.label("Weapon catalog is empty.");
        return;
    }
    player.weapon_index = player.weapon_index.min(weapon_catalog.len() - 1);
    player.armor_index = player.armor_index.min(armor_catalog.len().saturating_sub(1));
    player.shield_index = player.shield_index.min(shield_catalog.len().saturating_sub(1));

    if !fighter_presets.is_empty() {
        ui.horizontal(|ui| {
            ui.label("Fighter preset");
            let mut selection = player.fighter_preset.map_or(usize::MAX, |idx| idx);
            egui::ComboBox::from_id_source(format!("{id_prefix}_fighter_preset"))
                .selected_text(
                    player
                        .fighter_preset
                        .and_then(|idx| fighter_presets.get(idx))
                        .map(|preset| preset.name.as_str())
                        .unwrap_or("None"),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selection, usize::MAX, "None");
                    for (idx, preset) in fighter_presets.iter().enumerate() {
                        ui.selectable_value(&mut selection, idx, preset.name.as_str());
                    }
                });
            let selection = if selection == usize::MAX {
                None
            } else {
                Some(selection)
            };
            if selection != player.fighter_preset {
                player.fighter_preset = selection;
                if let Some(idx) = selection {
                    if let Some(preset) = fighter_presets.get(idx) {
                        apply_fighter_preset(
                            player,
                            preset,
                            weapon_catalog,
                            armor_catalog,
                            shield_catalog,
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
                        .iter()
                        .position(|entry| entry.name.eq_ignore_ascii_case(name))
                    {
                        fighter_presets[existing] = preset;
                        player.fighter_preset = Some(existing);
                    } else {
                        player.fighter_preset = Some(fighter_presets.len());
                        fighter_presets.push(preset);
                    }
                    if let Err(err) =
                        game_logic::save_fighter_presets(FIGHTER_PRESETS_PATH, fighter_presets)
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
            let mut selection = player.npc_preset.map_or(usize::MAX, |idx| idx);
            egui::ComboBox::from_id_source(format!("{id_prefix}_npc_preset"))
                .selected_text(match player.npc_preset.and_then(|idx| npc_presets.get(idx)) {
                    Some(preset) => preset.name.as_str(),
                    None => "None",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selection, usize::MAX, "None");
                    for (idx, preset) in npc_presets.iter().enumerate() {
                        ui.selectable_value(&mut selection, idx, preset.name.as_str());
                    }
                });
            player.npc_preset = if selection == usize::MAX {
                None
            } else {
                Some(selection)
            };
        });
        if let Some(preset) = player.npc_preset.and_then(|idx| npc_presets.get(idx)) {
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
        ui.color_edit_button_srgba(&mut player.color);
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

    let mut uses_projectiles = false;
    ui.horizontal(|ui| {
        ui.label("Weapon");
        egui::ComboBox::from_id_source(format!("{id_prefix}_weapon"))
            .selected_text(weapon_catalog[player.weapon_index].name.as_str())
            .show_ui(ui, |ui| {
                for (idx, weapon) in weapon_catalog.iter().enumerate() {
                    ui.selectable_value(&mut player.weapon_index, idx, weapon.name.as_str());
                }
            });
        let weapon = &weapon_catalog[player.weapon_index];
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

    let weapon = &weapon_catalog[player.weapon_index];
    let shield_bonus = if player.shield_index > 0
        && weapon.handedness == WeaponHandedness::OneHanded
        && !player.two_hand_grip
    {
        shield_catalog
            .get(player.shield_index)
            .and_then(|entry| entry.shield.as_ref())
            .map(|shield| shield.defense_bonus + player.shield_material_tier.clamp(0, 5))
    } else {
        None
    };
    let is_two_handed = weapon.handedness == WeaponHandedness::TwoHanded;
    let can_two_hand = weapon.handedness == WeaponHandedness::OneHanded
        && (weapon.size == WeaponSize::Medium || weapon.size == WeaponSize::Large);
    if is_two_handed {
        player.two_hand_grip = true;
    } else if !can_two_hand {
        player.two_hand_grip = false;
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
    let has_jab = weapon.jab_speed.is_some();
    if !has_jab {
        player.use_jab = false;
    }
    ui.horizontal(|ui| {
        ui.add_enabled_ui(has_jab, |ui| {
            ui.checkbox(&mut player.use_jab, "Jab attack");
        });
        if !has_jab {
            ui.label("Unavailable");
        }
        ui.checkbox(&mut player.hold_at_bay, "Hold at bay");
    });
    if player.use_jab {
        if let Some(jab_special) = weapon.jab_special_expr.as_ref() {
            ui.label(format!("Jab special damage: {jab_special} (non-penetrating)"));
        } else {
            ui.label("Jab damage: half, non-penetrating");
        }
    }
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

    ui.add_enabled_ui(!npc_active, |ui| {
        ui.horizontal(|ui| {
            ui.label("Armor");
            egui::ComboBox::from_id_source(format!("{id_prefix}_armor"))
                .selected_text(armor_display_name(armor_catalog.get(player.armor_index)))
                .show_ui(ui, |ui| {
                    for (idx, armor) in armor_catalog.iter().enumerate() {
                        ui.selectable_value(&mut player.armor_index, idx, armor.label.clone());
                    }
                });
            material_tier_combo(
                ui,
                format!("{id_prefix}_armor_material"),
                "Material",
                &mut player.armor_material_tier,
            );
        });
        ui.horizontal(|ui| {
            ui.label("Shield");
            let can_use_shield =
                weapon.handedness == WeaponHandedness::OneHanded && !player.two_hand_grip;
            if !can_use_shield {
                player.shield_index = 0;
                player.shield_material_tier = 0;
            }
            ui.add_enabled_ui(can_use_shield, |ui| {
                egui::ComboBox::from_id_source(format!("{id_prefix}_shield"))
                    .selected_text(shield_display_name(shield_catalog.get(player.shield_index)))
                    .show_ui(ui, |ui| {
                        for (idx, shield) in shield_catalog.iter().enumerate() {
                            ui.selectable_value(&mut player.shield_index, idx, shield.label.clone());
                        }
                    });
            });
            if !can_use_shield {
                ui.label("Unavailable");
            }
            let shield_enabled = can_use_shield && player.shield_index > 0;
            ui.add_enabled_ui(shield_enabled, |ui| {
                material_tier_combo(
                    ui,
                    format!("{id_prefix}_shield_material"),
                    "Material",
                    &mut player.shield_material_tier,
                );
            });
        });

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
        let shield_active = player.shield_index > 0
            && weapon.handedness == WeaponHandedness::OneHanded
            && !player.two_hand_grip;
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

    let game_logic::PlayerSummary { derived, roll } =
        game_logic::player_summary(player, weapon_catalog, armor_catalog, shield_catalog);
    let defense_mastery = game_logic::effective_defense_mastery(player, weapon);
    ui.separator();
    if npc_active {
        ui.label("Derived stats ignored while NPC preset is active.");
    } else {
        ui.label("Derived");
        ui.label(format!(
            "Hit points: {} (x{:.1})",
            derived.hit_points, derived.health_mult
        ));
        ui.label(format!("Attack bonus: {}", derived.attack_bonus));
        ui.label(format!("Speed mod: {}", derived.speed_mod));
        ui.label(format!("Initiative mod: {}", derived.initiative_mod));
        ui.label(format!("Base DV: {}", derived.base_dv));
        if let Some(shield_bonus) = shield_bonus {
            let weapon_defense = if weapon.defense_bonus_always { 4 } else { 0 };
            let dv_with_shield =
                derived.base_dv + defense_mastery + weapon_defense + 4 + shield_bonus;
            ui.label(format!("DV (melee + shield): {}", dv_with_shield));
        }
        ui.label(format!("Armor DR: {}", derived.armor_dr));
        ui.label(format!(
            "Carry (none/light/med/heavy): {:?}",
            derived.carry_capacity
        ));
        ui.label(format!("Load: {}", derived.load_category));

        let attack_bonus = roll.attack_bonus;
        let strength_damage = roll.strength_damage;

        ui.separator();
        ui.label("Rolls");
        ui.label(format!("Attack roll: d20p + {}", attack_bonus));
        if roll.is_ranged_weapon {
            if let Some(shield_bonus) = shield_bonus {
                ui.label(format!(
                    "Defense roll (ranged): d20p + {} (cover cap applies)",
                    shield_bonus
                ));
            } else {
                ui.label("Defense roll (ranged): d12p if stationary, else d20p");
            }
        } else {
            let weapon_def = if weapon.defense_bonus_always { " (+4 weapon)" } else { "" };
            if let Some(shield_bonus) = shield_bonus {
                ui.label(format!(
                    "Defense roll (melee): d20p + {} + {}{}",
                    derived.base_dv + defense_mastery + 4,
                    shield_bonus,
                    weapon_def
                ));
            } else {
                ui.label(format!(
                    "Defense roll (melee): d20p + {}{}",
                    derived.base_dv + defense_mastery,
                    weapon_def
                ));
            }
        }
        let target_dr = opponent
            .npc_preset
            .and_then(|idx| npc_presets.get(idx))
            .map(|preset| preset.armor_dr)
            .unwrap_or_else(|| {
                game_logic::player_summary(
                    opponent,
                    weapon_catalog,
                    armor_catalog,
                    shield_catalog,
                )
                .derived
                .armor_dr
            });
        ui.label(format!("Your armor DR: {}", derived.armor_dr));
        ui.label(format!(
            "Damage roll: {} + {} vs target DR {} (AP {})",
            weapon.damage_expr, strength_damage, target_dr, weapon.armor_pen
        ));
    }
}

fn apply_fighter_preset(
    player: &mut PlayerConfig,
    preset: &FighterPreset,
    weapon_catalog: &[WeaponPreset],
    armor_catalog: &[ArmorEntry],
    shield_catalog: &[ShieldEntry],
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
    player.armor_material_tier = preset.armor_material_tier;
    player.projectile_material_tier = preset.projectile_material_tier;
    player.shield_material_tier = preset.shield_material_tier;
    player.two_hand_grip = preset.two_hand_grip;
    player.use_jab = preset.use_jab;
    player.hold_at_bay = preset.hold_at_bay;
    player.weapon_index =
        find_weapon_index_by_name(weapon_catalog, &preset.weapon).unwrap_or(0);
    player.armor_index = find_armor_index_by_name(armor_catalog, &preset.armor).unwrap_or(0);
    player.shield_index = find_shield_index_by_name(shield_catalog, &preset.shield).unwrap_or(0);
    if let Some(weapon) = weapon_catalog.get(player.weapon_index) {
        game_logic::sanitize_projectile_tier(player, weapon);
    }
}

fn fighter_preset_from_player(
    player: &PlayerConfig,
    weapon_catalog: &[WeaponPreset],
    armor_catalog: &[ArmorEntry],
    shield_catalog: &[ShieldEntry],
    name: &str,
) -> FighterPreset {
    let weapon = weapon_catalog
        .get(player.weapon_index)
        .map(|weapon| weapon.name.clone())
        .unwrap_or_else(|| "Fist".to_string());
    let armor = armor_catalog
        .get(player.armor_index)
        .and_then(|entry| entry.armor.as_ref().map(|armor| armor.name.to_string()))
        .unwrap_or_else(|| "None".to_string());
    let shield = shield_catalog
        .get(player.shield_index)
        .and_then(|entry| entry.shield.as_ref().map(|shield| shield.name.to_string()))
        .unwrap_or_else(|| "None".to_string());
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
        armor,
        shield,
        weapon_material_tier: player.weapon_material_tier,
        armor_material_tier: player.armor_material_tier,
        projectile_material_tier: player.projectile_material_tier,
        shield_material_tier: player.shield_material_tier,
        two_hand_grip: player.two_hand_grip,
        use_jab: player.use_jab,
        hold_at_bay: player.hold_at_bay,
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

fn find_weapon_index_by_name(catalog: &[WeaponPreset], name: &str) -> Option<usize> {
    catalog
        .iter()
        .position(|weapon| weapon.name.eq_ignore_ascii_case(name))
}

fn find_armor_index_by_name(catalog: &[ArmorEntry], name: &str) -> Option<usize> {
    if name.eq_ignore_ascii_case("None") {
        return Some(0);
    }
    catalog.iter().position(|entry| {
        entry
            .armor
            .as_ref()
            .map(|armor| armor.name.eq_ignore_ascii_case(name))
            .unwrap_or(false)
    })
}

fn find_shield_index_by_name(catalog: &[ShieldEntry], name: &str) -> Option<usize> {
    if name.eq_ignore_ascii_case("None") {
        return Some(0);
    }
    catalog.iter().position(|entry| {
        entry
            .shield
            .as_ref()
            .map(|shield| shield.name.eq_ignore_ascii_case(name))
            .unwrap_or(false)
    })
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
        egui::ComboBox::from_id_source(id)
            .selected_text(format!("{:02}", percentile))
            .show_ui(ui, |ui| {
                ui.selectable_value(percentile, 1, "01");
                ui.selectable_value(percentile, 51, "51");
            });
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
    egui::ComboBox::from_id_source(id_source)
        .selected_text(tier_label(*selection))
        .show_ui(ui, |ui| {
            for tier in tiers {
                ui.selectable_value(selection, *tier, tier_label(*tier));
            }
        });
}

fn material_tier_combo(ui: &mut egui::Ui, id_source: String, label: &str, selection: &mut i32) {
    ui.label(label);
    egui::ComboBox::from_id_source(id_source)
        .selected_text(format!("+{selection}"))
        .show_ui(ui, |ui| {
            for tier in 0..=5 {
                ui.selectable_value(selection, tier, format!("+{tier}"));
            }
        });
}

fn armor_display_name(entry: Option<&ArmorEntry>) -> String {
    entry
        .map(|armor| armor.label.clone())
        .unwrap_or_else(|| "None".to_string())
}

fn shield_display_name(entry: Option<&ShieldEntry>) -> String {
    entry
        .map(|shield| shield.label.clone())
        .unwrap_or_else(|| "None".to_string())
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([980.0, 560.0])
            .with_min_inner_size([640.0, 360.0]),
        ..Default::default()
    };
    eframe::run_native(
        "HackMaster Simulator",
        options,
        Box::new(|_cc| Ok(Box::new(SimGuiApp::new()))),
    )
}
