#[path = "../character.rs"]
mod character;
#[path = "../sim.rs"]
mod sim;

use character::{
    AbilityScore, AbilitySet, Armor, Character, Equipment, Progression, ProgressionTier, Weapon,
    WeaponGroup, WeaponMastery,
};
use eframe::egui::{self, Color32, Pos2, Rect};
use serde::Deserialize;
use sim::{Combatant, SimConfig, SimState};
use std::fs;

#[derive(Clone)]
struct WeaponPreset {
    name: String,
    group: WeaponGroup,
    speed: f32,
    speed_label: String,
    damage_expr: String,
    reach_label: String,
    reach_ft: f32,
    armor_pen: i32,
    defense_bonus_always: bool,
}

#[derive(Clone)]
struct ArmorEntry {
    label: String,
    armor: Option<Armor>,
}

#[derive(Clone)]
struct PlayerConfig {
    name: String,
    color: Color32,
    level: u8,
    progression: Progression,
    base_hp: u32,
    strength_base: u8,
    strength_pct: u8,
    dex_base: u8,
    dex_pct: u8,
    intelligence: u8,
    wisdom: u8,
    constitution: u8,
    looks: u8,
    charisma: u8,
    weapon_index: usize,
    armor_index: usize,
}

impl PlayerConfig {
    fn new(name: &str, color: Color32, weapon_index: usize) -> Self {
        Self {
            name: name.to_string(),
            color,
            level: 1,
            progression: Progression::default(),
            base_hp: 10,
            strength_base: 10,
            strength_pct: 1,
            dex_base: 10,
            dex_pct: 1,
            intelligence: 10,
            wisdom: 10,
            constitution: 10,
            looks: 10,
            charisma: 10,
            weapon_index,
            armor_index: 0,
        }
    }
}

struct SimGuiApp {
    running: bool,
    sim: SimState,
    players: [PlayerConfig; 2],
    weapon_catalog: Vec<WeaponPreset>,
    armor_catalog: Vec<ArmorEntry>,
}

impl SimGuiApp {
    fn new() -> Self {
        let (weapon_catalog, armor_catalog) = match load_catalogs() {
            Ok((weapons, armors)) => (weapons, armors),
            Err(err) => {
                eprintln!("Failed to load JSON catalogs: {err}");
                (default_weapon_catalog(), default_armor_catalog())
            }
        };
        let sim = SimState::new(SimConfig::new(20.0, 5.0, 1.0));
        let mut app = Self {
            running: false,
            sim,
            players: [
                PlayerConfig::new("Fighter A", Color32::from_rgb(214, 93, 69), 1),
                PlayerConfig::new("Fighter B", Color32::from_rgb(70, 140, 210), 2),
            ],
            weapon_catalog,
            armor_catalog,
        };
        app.reset_positions();
        app
    }

    fn reset_positions(&mut self) {
        let combatants = build_combatants(&self.players, &self.weapon_catalog, &self.armor_catalog);
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
        let painter = ui.painter();
        let padding = 20.0;
        let ground_y = rect.center().y + rect.height() * 0.15;
        let left = rect.left() + padding;
        let right = rect.right() - padding;
        let arena_width = (right - left).max(1.0);
        let scale = arena_width / self.sim.config.start_distance.max(1.0);

        painter.line_segment(
            [Pos2::new(left, ground_y), Pos2::new(right, ground_y)],
            (2.0, Color32::from_gray(80)),
        );

        let mut x0 = left + self.sim.actors[0].position * scale;
        let mut x1 = left + self.sim.actors[1].position * scale;
        let gap = (x1 - x0).abs();
        let min_gap = 28.0;
        if gap < min_gap {
            let dir = if x1 >= x0 { 1.0 } else { -1.0 };
            if self.sim.combatants[0].reach_ft >= self.sim.combatants[1].reach_ft {
                x1 = x0 + dir * min_gap;
            } else {
                x0 = x1 - dir * min_gap;
            }
        }

        for (idx, (x, player)) in [(x0, &self.players[0]), (x1, &self.players[1])]
            .into_iter()
            .enumerate()
        {
            let hp = self.sim.combatants[idx].hp.max(0) as f32;
            let max_hp = self.sim.combatants[idx].max_hp.max(1) as f32;
            let hp_ratio = (hp / max_hp).clamp(0.0, 1.0);
            let bar_width = 48.0;
            let bar_height = 6.0;
            let bar_y = ground_y - 56.0;
            let bar_x = x - bar_width * 0.5;
            let bg_rect = Rect::from_min_size(Pos2::new(bar_x, bar_y), egui::vec2(bar_width, bar_height));
            painter.rect_filled(bg_rect, 2.0, Color32::from_gray(40));
            let fill_rect = Rect::from_min_size(
                Pos2::new(bar_x, bar_y),
                egui::vec2(bar_width * hp_ratio, bar_height),
            );
            painter.rect_filled(fill_rect, 2.0, Color32::from_rgb(82, 180, 76));

            let pos = Pos2::new(x, ground_y - 20.0);
            painter.circle_filled(pos, 12.0, player.color);
            painter.text(
                Pos2::new(x, ground_y - 38.0),
                egui::Align2::CENTER_CENTER,
                &player.name,
                egui::TextStyle::Body.resolve(ui.style()),
                Color32::from_gray(220),
            );
        }
    }
}

impl eframe::App for SimGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dt = ctx.input(|i| i.unstable_dt).min(0.05);
        self.sim.config.stop_distance =
            stop_distance_for_players(&self.players, &self.weapon_catalog);
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
                ui.separator();
                ui.label("Start distance (ft)");
                if ui
                    .add(
                        egui::Slider::new(&mut self.sim.config.start_distance, 5.0..=60.0)
                            .step_by(1.0),
                    )
                    .changed()
                {
                    if !self.running {
                        self.reset_positions();
                    }
                }
                ui.label("Walk speed (ft/s)");
                ui.add(
                    egui::Slider::new(&mut self.sim.config.walk_speed, 1.0..=15.0).step_by(0.5),
                );
            });
        });

        egui::SidePanel::left("players")
            .resizable(true)
            .min_width(280.0)
            .show(ctx, |ui| {
                egui::CollapsingHeader::new("Player 1")
                    .default_open(true)
                    .show(ui, |ui| {
                        render_player_editor(
                            ui,
                            "p1",
                            &mut self.players[0],
                            &self.weapon_catalog,
                            &self.armor_catalog,
                        );
                    });
                ui.separator();
                egui::CollapsingHeader::new("Player 2")
                    .default_open(true)
                    .show(ui, |ui| {
                        render_player_editor(
                            ui,
                            "p2",
                            &mut self.players[1],
                            &self.weapon_catalog,
                            &self.armor_catalog,
                        );
                    });
            });

        egui::SidePanel::right("status")
            .resizable(false)
            .min_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Status");
                ui.separator();
                ui.label(format!("Elapsed: {}s", self.sim.elapsed_seconds));
                ui.label(format!("Distance: {:.1} ft", self.sim.distance()));
                ui.label(format!(
                    "Stop distance: {:.1} ft",
                    self.sim.config.stop_distance
                ));
                ui.separator();
                ui.label(format!(
                    "{} HP: {}",
                    self.sim.combatants[0].name, self.sim.combatants[0].hp
                ));
                ui.label(format!(
                    "{} HP: {}",
                    self.sim.combatants[1].name, self.sim.combatants[1].hp
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
            let (rect, _response) =
                ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
            self.draw_arena(ui, rect);
        });

        if self.running {
            ctx.request_repaint();
        }
    }
}

fn render_player_editor(
    ui: &mut egui::Ui,
    id_prefix: &str,
    player: &mut PlayerConfig,
    weapon_catalog: &[WeaponPreset],
    armor_catalog: &[ArmorEntry],
) {
    if weapon_catalog.is_empty() {
        ui.label("Weapon catalog is empty.");
        return;
    }
    player.weapon_index = player.weapon_index.min(weapon_catalog.len() - 1);
    player.armor_index = player.armor_index.min(armor_catalog.len().saturating_sub(1));

    ui.horizontal(|ui| {
        ui.label("Name");
        ui.text_edit_singleline(&mut player.name);
    });
    ui.horizontal(|ui| {
        ui.label("Color");
        ui.color_edit_button_srgba(&mut player.color);
    });
    ui.horizontal(|ui| {
        ui.label("Level");
        ui.add(egui::Slider::new(&mut player.level, 1..=20).step_by(1.0));
    });
    ui.horizontal(|ui| {
        ui.label("Base HP");
        ui.add(egui::Slider::new(&mut player.base_hp, 1..=200).step_by(1.0));
    });
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

    ui.horizontal(|ui| {
        ui.label("Weapon");
        egui::ComboBox::from_id_source(format!("{id_prefix}_weapon"))
            .selected_text(weapon_catalog[player.weapon_index].name.as_str())
            .show_ui(ui, |ui| {
                for (idx, weapon) in weapon_catalog.iter().enumerate() {
                    ui.selectable_value(&mut player.weapon_index, idx, weapon.name.as_str());
                }
            });
    });

    let weapon = &weapon_catalog[player.weapon_index];
    ui.label(format!(
        "Speed {} | Damage {} | Reach/Range {}",
        weapon.speed_label, weapon.damage_expr, weapon.reach_label
    ));

    ui.horizontal(|ui| {
        ui.label("Armor");
        egui::ComboBox::from_id_source(format!("{id_prefix}_armor"))
            .selected_text(armor_display_name(armor_catalog.get(player.armor_index)))
            .show_ui(ui, |ui| {
                for (idx, armor) in armor_catalog.iter().enumerate() {
                    ui.selectable_value(&mut player.armor_index, idx, armor.label.clone());
                }
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

    let character = build_character(player, weapon_catalog, armor_catalog);
    let derived = character.derived();
    ui.separator();
    ui.label("Derived");
    ui.label(format!(
        "Hit points: {} (x{:.1})",
        derived.hit_points, derived.health_mult
    ));
    ui.label(format!("Attack bonus: {}", derived.attack_bonus));
    ui.label(format!("Speed mod: {}", derived.speed_mod));
    ui.label(format!("Initiative mod: {}", derived.initiative_mod));
    ui.label(format!("Base DV: {}", derived.base_dv));
    ui.label(format!("Armor DR: {}", derived.armor_dr));
    ui.label(format!(
        "Carry (none/light/med/heavy): {:?}",
        derived.carry_capacity
    ));
    ui.label(format!("Load: {}", derived.load_category));
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

fn armor_display_name(entry: Option<&ArmorEntry>) -> String {
    entry
        .map(|armor| armor.label.clone())
        .unwrap_or_else(|| "None".to_string())
}

fn base_weapon_threshold(group: WeaponGroup) -> f32 {
    match group {
        WeaponGroup::Bows | WeaponGroup::Crossbows => 150.0,
        WeaponGroup::Shields => 200.0,
        _ => 100.0,
    }
}

fn weapon_preset(
    name: &'static str,
    group: WeaponGroup,
    speed: f32,
    damage_expr: &'static str,
    reach_label: &'static str,
    reach_ft: f32,
) -> WeaponPreset {
    WeaponPreset {
        name: name.to_string(),
        group,
        speed,
        speed_label: format!("{speed:.0}"),
        damage_expr: damage_expr.to_string(),
        reach_label: reach_label.to_string(),
        reach_ft,
        armor_pen: 0,
        defense_bonus_always: false,
    }
}

fn build_character(
    player: &PlayerConfig,
    weapon_catalog: &[WeaponPreset],
    armor_catalog: &[ArmorEntry],
) -> Character {
    let weapon_preset = &weapon_catalog[player.weapon_index];
    let weapon = Weapon {
        name: weapon_preset.name.clone(),
        group: weapon_preset.group,
        speed: weapon_preset.speed,
        damage_expr: weapon_preset.damage_expr.clone(),
        reach_ft: weapon_preset.reach_ft,
        armor_pen: weapon_preset.armor_pen,
        defense_bonus_always: weapon_preset.defense_bonus_always,
    };
    let armor = armor_catalog
        .get(player.armor_index)
        .and_then(|entry| entry.armor.clone());

    let abilities = AbilitySet {
        strength: AbilityScore::new(player.strength_base, player.strength_pct),
        intelligence: player.intelligence,
        wisdom: player.wisdom,
        dexterity: AbilityScore::new(player.dex_base, player.dex_pct),
        constitution: player.constitution,
        looks: player.looks,
        charisma: player.charisma,
    };

    let mastery = WeaponMastery {
        group: weapon_preset.group,
        points: Default::default(),
        base_threshold: base_weapon_threshold(weapon_preset.group),
    };

    let equipment = Equipment {
        weapon: Some(weapon),
        shield: None,
        armor,
        weapon_material: None,
        armor_material: None,
        shield_material: None,
    };

    Character::builder(&player.name)
        .level(player.level, player.progression)
        .base_hp(player.base_hp)
        .abilities(abilities)
        .weapon_mastery(mastery)
        .equipment(equipment)
        .build()
}

fn build_combatants(
    players: &[PlayerConfig; 2],
    weapon_catalog: &[WeaponPreset],
    armor_catalog: &[ArmorEntry],
) -> [Combatant; 2] {
    [
        build_combatant(&players[0], weapon_catalog, armor_catalog),
        build_combatant(&players[1], weapon_catalog, armor_catalog),
    ]
}

fn build_combatant(
    player: &PlayerConfig,
    weapon_catalog: &[WeaponPreset],
    armor_catalog: &[ArmorEntry],
) -> Combatant {
    let character = build_character(player, weapon_catalog, armor_catalog);
    let derived = character.derived();
    let weapon_name = character
        .equipment
        .weapon
        .as_ref()
        .map(|weapon| weapon.name.clone())
        .unwrap_or_else(|| "Unarmed".to_string());
    let weapon_speed = character
        .equipment
        .weapon
        .as_ref()
        .map(|weapon| weapon.speed)
        .unwrap_or(10.0);
    let weapon_reach = character
        .equipment
        .weapon
        .as_ref()
        .map(|weapon| weapon.reach_ft)
        .unwrap_or(1.0);
    let armor_penetration = character
        .equipment
        .weapon
        .as_ref()
        .map(|weapon| weapon.armor_pen)
        .unwrap_or(0);
    let weapon_defense_always = character
        .equipment
        .weapon
        .as_ref()
        .map(|weapon| weapon.defense_bonus_always)
        .unwrap_or(false);
    let has_weapon = character.equipment.weapon.is_some();
    let weapon_damage = character
        .equipment
        .weapon
        .as_ref()
        .map(|weapon| weapon.damage_expr.clone())
        .unwrap_or_else(|| "d4p".to_string());

    Combatant::new(
        character.name,
        weapon_name,
        derived.attack_bonus,
        derived.base_dv,
        derived.armor_dr,
        armor_penetration,
        weapon_damage,
        character.ability_mods.strength.damage,
        weapon_speed,
        weapon_reach,
        has_weapon,
        weapon_defense_always,
        derived.hit_points as i32,
    )
}

fn stop_distance_for_players(players: &[PlayerConfig; 2], weapon_catalog: &[WeaponPreset]) -> f32 {
    let reach_a = weapon_catalog
        .get(players[0].weapon_index)
        .map(|weapon| weapon.reach_ft)
        .unwrap_or(1.0);
    let reach_b = weapon_catalog
        .get(players[1].weapon_index)
        .map(|weapon| weapon.reach_ft)
        .unwrap_or(1.0);
    let reach_a = if reach_a <= 0.0 { 1.0 } else { reach_a };
    let reach_b = if reach_b <= 0.0 { 1.0 } else { reach_b };
    reach_a.max(reach_b)
}

fn default_weapon_catalog() -> Vec<WeaponPreset> {
    vec![
        // Unarmed
        weapon_preset("Fist", WeaponGroup::Unarmed, 10.0, "(d4p-2)+(d4p-2)", "1 foot", 1.0),
        weapon_preset("Antler", WeaponGroup::Unarmed, 10.0, "2d6p", "3 feet", 3.0),
        weapon_preset("Claw", WeaponGroup::Unarmed, 5.0, "1d8p", "1 foot", 1.0),
        weapon_preset("Fang", WeaponGroup::Unarmed, 10.0, "1d10p", "0.5 feet", 0.5),
        weapon_preset("Cestus", WeaponGroup::Unarmed, 10.0, "2d4p", "1 foot", 1.0),
        weapon_preset(
            "Gauntlet",
            WeaponGroup::Unarmed,
            10.0,
            "(d4p-1)+(d4p-1)",
            "1 foot",
            1.0,
        ),
        weapon_preset(
            "Spiked gauntlet",
            WeaponGroup::Unarmed,
            10.0,
            "1d8p",
            "1 foot",
            1.0,
        ),
        // Axes
        weapon_preset("Battle axe", WeaponGroup::Axes, 12.0, "4d3p^2", "3 feet", 3.0),
        weapon_preset(
            "Executioner's axe",
            WeaponGroup::Axes,
            18.0,
            "3d8p+3^2",
            "4 feet",
            4.0,
        ),
        weapon_preset("Greataxe", WeaponGroup::Axes, 14.0, "3d6p+3^2", "3.5 feet", 3.5),
        weapon_preset("Hand axe", WeaponGroup::Axes, 8.0, "d4p+d6p", "1.5 feet", 1.5),
        weapon_preset("Khopesh", WeaponGroup::Axes, 8.0, "2d6p", "2 feet", 2.0),
        weapon_preset("Military pick", WeaponGroup::Axes, 12.0, "3d4p^2", "3 feet", 3.0),
        weapon_preset(
            "Horseman's pick",
            WeaponGroup::Axes,
            8.0,
            "d4p+d6p^1",
            "1.5 feet",
            1.5,
        ),
        weapon_preset("Scythe", WeaponGroup::Axes, 15.0, "2d6p+3", "4.5 feet", 4.5),
        weapon_preset("Sickle", WeaponGroup::Axes, 8.0, "d6p+d3p", "1.5 feet", 1.5),
        weapon_preset(
            "Throwing axe",
            WeaponGroup::Axes,
            7.0,
            "d4p+d6p",
            "1/60 feet",
            0.0,
        ),
        // Basic
        weapon_preset("Club", WeaponGroup::Basic, 10.0, "d6p+d4p", "2.5 feet", 2.5),
        weapon_preset("Dart", WeaponGroup::Basic, 5.0, "d4p", "0.5/40 feet", 0.0),
        weapon_preset("Sling", WeaponGroup::Basic, 10.0, "d4p+d6p", "160 feet", 0.0),
        weapon_preset("Staff", WeaponGroup::Basic, 13.0, "2d4p+3", "8 feet", 8.0),
        // Blunt
        weapon_preset("Greatclub", WeaponGroup::Blunt, 16.0, "d20p+3^1", "5 feet", 5.0),
        weapon_preset(
            "Greathammer",
            WeaponGroup::Blunt,
            20.0,
            "d8p+2d10p+3^2",
            "4.5 feet",
            4.5,
        ),
        weapon_preset("Hammer", WeaponGroup::Blunt, 8.0, "2d6p^1", "1.5 feet", 1.5),
        weapon_preset("Warhammer", WeaponGroup::Blunt, 12.0, "d8p+d10p^1", "2.5 feet", 2.5),
        weapon_preset("Mace", WeaponGroup::Blunt, 11.0, "d6p+d8p^2", "2 feet", 2.0),
        weapon_preset(
            "Horseman's mace",
            WeaponGroup::Blunt,
            10.0,
            "2d6p^1",
            "1.5 feet",
            1.5,
        ),
        weapon_preset("Maul", WeaponGroup::Blunt, 15.0, "2d12p+3^2", "3 feet", 3.0),
        weapon_preset("Morningstar", WeaponGroup::Blunt, 11.0, "2d8p", "3 feet", 3.0),
        // Bows
        weapon_preset("Longbow", WeaponGroup::Bows, 12.0, "2d8p", "210 feet", 0.0),
        weapon_preset("Recurve bow", WeaponGroup::Bows, 11.0, "3d4p", "150 feet", 0.0),
        weapon_preset("Shortbow", WeaponGroup::Bows, 12.0, "2d6p", "150 feet", 0.0),
        weapon_preset("Warbow", WeaponGroup::Bows, 20.0, "3d6p^1", "300 feet", 0.0),
        // Crossbows
        weapon_preset("Arbalest", WeaponGroup::Crossbows, 90.0, "3d8p^1", "400 feet", 0.0),
        weapon_preset(
            "Light crossbow",
            WeaponGroup::Crossbows,
            20.0,
            "2d6p",
            "180 feet",
            0.0,
        ),
        weapon_preset(
            "Hand crossbow",
            WeaponGroup::Crossbows,
            15.0,
            "2d4p",
            "120 feet",
            0.0,
        ),
        weapon_preset(
            "Heavy crossbow",
            WeaponGroup::Crossbows,
            60.0,
            "2d10p",
            "250 feet",
            0.0,
        ),
        // Double weapons
        weapon_preset(
            "Double axe",
            WeaponGroup::Double,
            13.0,
            "4d3p^2 and 4d3p^2",
            "3.5 feet",
            3.5,
        ),
        weapon_preset(
            "Double scimitar",
            WeaponGroup::Double,
            10.0,
            "2d8p and 2d8p",
            "3.5 feet",
            3.5,
        ),
        weapon_preset(
            "Dual scythe",
            WeaponGroup::Double,
            16.0,
            "2d6p and 2d6p",
            "4 feet",
            4.0,
        ),
        weapon_preset(
            "Hooked hammer",
            WeaponGroup::Double,
            14.0,
            "d8p+d10p^1 and 3d4p^2",
            "3 feet",
            3.0,
        ),
        weapon_preset(
            "Monk's Spade",
            WeaponGroup::Double,
            9.0,
            "2d4p and 2d4p",
            "3 feet",
            3.0,
        ),
        weapon_preset(
            "Spear-axe",
            WeaponGroup::Double,
            13.0,
            "2d6p and 4d3p^2",
            "6.5 feet",
            6.5,
        ),
        weapon_preset(
            "Two-bladed sword",
            WeaponGroup::Double,
            11.0,
            "2d8p and 2d8p",
            "4 feet",
            4.0,
        ),
        // Ensnaring
        weapon_preset("Bola", WeaponGroup::Ensnaring, 10.0, "d4p", "50 feet", 0.0),
        weapon_preset("Lasso", WeaponGroup::Ensnaring, 15.0, "-", "50 feet", 0.0),
        weapon_preset("Net", WeaponGroup::Ensnaring, 20.0, "-", "15 feet", 0.0),
        // Lashes
        weapon_preset("Flail", WeaponGroup::Lashes, 13.0, "2d8p^1", "4 feet", 4.0),
        weapon_preset(
            "Horseman's flail",
            WeaponGroup::Lashes,
            11.0,
            "d4p+d6p",
            "2 feet",
            2.0,
        ),
        weapon_preset("Scourge", WeaponGroup::Lashes, 9.0, "2d4p", "1.5 feet", 1.5),
        weapon_preset(
            "Spiked chain",
            WeaponGroup::Lashes,
            14.0,
            "2d6p+3",
            "6 feet",
            6.0,
        ),
        weapon_preset("Whip", WeaponGroup::Lashes, 8.0, "1d6p", "1.5 feet", 1.5),
        // Large swords
        weapon_preset(
            "Broadsword",
            WeaponGroup::LargeSwords,
            11.0,
            "2d6p+d3p",
            "3.25 feet",
            3.25,
        ),
        weapon_preset(
            "Bastard sword",
            WeaponGroup::LargeSwords,
            12.0,
            "d8p+d10p",
            "4.5 feet",
            4.5,
        ),
        weapon_preset(
            "Claymore",
            WeaponGroup::LargeSwords,
            13.0,
            "2d10p+3^1",
            "5 feet",
            5.0,
        ),
        weapon_preset(
            "Flamberge",
            WeaponGroup::LargeSwords,
            16.0,
            "6d3p+3^2",
            "6 feet",
            6.0,
        ),
        weapon_preset(
            "Greatsword",
            WeaponGroup::LargeSwords,
            14.0,
            "d10p+d12p+3^2",
            "5.5 feet",
            5.5,
        ),
        weapon_preset(
            "Longsword",
            WeaponGroup::LargeSwords,
            10.0,
            "2d8p",
            "3.5 feet",
            3.5,
        ),
        weapon_preset(
            "Greatknife",
            WeaponGroup::LargeSwords,
            12.0,
            "3d6p+3^1",
            "4 feet",
            4.0,
        ),
        weapon_preset("Sabre", WeaponGroup::LargeSwords, 8.0, "d6p+d8p", "3 feet", 3.0),
        weapon_preset("Scimitar", WeaponGroup::LargeSwords, 9.0, "2d8p", "3 feet", 3.0),
        weapon_preset("Spatha", WeaponGroup::LargeSwords, 9.0, "d6p+d8p", "3 feet", 3.0),
        weapon_preset(
            "Thrusting sword",
            WeaponGroup::LargeSwords,
            9.0,
            "3d4p+3",
            "4.5 feet",
            4.5,
        ),
        weapon_preset(
            "Two-handed sword",
            WeaponGroup::LargeSwords,
            16.0,
            "2d12p+3^2",
            "6 feet",
            6.0,
        ),
        // Small swords
        weapon_preset("Dagger", WeaponGroup::SmallSwords, 7.0, "2d4p", "1 foot", 1.0),
        weapon_preset(
            "Dueling sword",
            WeaponGroup::SmallSwords,
            7.0,
            "3d4p",
            "3.5 feet",
            3.5,
        ),
        weapon_preset(
            "Falx",
            WeaponGroup::SmallSwords,
            9.0,
            "2d3p+d6p",
            "2.5 feet",
            2.5,
        ),
        weapon_preset("Knife", WeaponGroup::SmallSwords, 7.0, "d6p", "1 foot", 1.0),
        weapon_preset(
            "Gladius",
            WeaponGroup::SmallSwords,
            9.0,
            "d4p+d8p",
            "2 feet",
            2.0,
        ),
        weapon_preset(
            "Long knife",
            WeaponGroup::SmallSwords,
            6.0,
            "1d10p",
            "1.5 feet",
            1.5,
        ),
        weapon_preset(
            "Short sword",
            WeaponGroup::SmallSwords,
            8.0,
            "2d6p",
            "2 feet",
            2.0,
        ),
        weapon_preset(
            "Throwing knife",
            WeaponGroup::SmallSwords,
            6.0,
            "d6p",
            "1/50 feet",
            0.0,
        ),
        // Polearms
        weapon_preset("Bardiche", WeaponGroup::Polearms, 14.0, "4d4p+3", "5 feet", 5.0),
        weapon_preset(
            "Fauchard",
            WeaponGroup::Polearms,
            13.0,
            "2d6p+3",
            "8 feet",
            8.0,
        ),
        weapon_preset("Glaive", WeaponGroup::Polearms, 14.0, "5d4p+3", "8 feet", 8.0),
        weapon_preset(
            "Guisarme",
            WeaponGroup::Polearms,
            13.0,
            "2d6p+3",
            "6 feet",
            6.0,
        ),
        weapon_preset(
            "Halberd",
            WeaponGroup::Polearms,
            14.0,
            "2d10p+3^2",
            "7 feet",
            7.0,
        ),
        weapon_preset(
            "Mancatcher",
            WeaponGroup::Polearms,
            14.0,
            "d4p",
            "8 feet",
            8.0,
        ),
        weapon_preset(
            "Poleaxe",
            WeaponGroup::Polearms,
            13.0,
            "3d6p+3^2",
            "6 feet",
            6.0,
        ),
        weapon_preset(
            "Polehammer",
            WeaponGroup::Polearms,
            15.0,
            "d10p+d12p+3^2",
            "7 feet",
            7.0,
        ),
        weapon_preset(
            "Raven's Beak",
            WeaponGroup::Polearms,
            14.0,
            "2d6p+3^2",
            "6 feet",
            6.0,
        ),
        weapon_preset(
            "Swordstaff",
            WeaponGroup::Polearms,
            11.0,
            "2d8p+3",
            "8 feet",
            8.0,
        ),
        weapon_preset(
            "Voulge",
            WeaponGroup::Polearms,
            15.0,
            "4d4p+3",
            "8 feet",
            8.0,
        ),
        // Spears
        weapon_preset("Hasta", WeaponGroup::Spears, 12.0, "2d6p", "7 feet", 7.0),
        weapon_preset("Javelin", WeaponGroup::Spears, 7.0, "d12p", "5/100 feet", 0.0),
        weapon_preset("Lance", WeaponGroup::Spears, 12.0, "2d8p^2", "10 feet", 10.0),
        weapon_preset(
            "Partisan",
            WeaponGroup::Spears,
            14.0,
            "2d8p+3",
            "7 feet",
            7.0,
        ),
        weapon_preset("Pike", WeaponGroup::Spears, 18.0, "2d6p+3", "18 feet", 18.0),
        weapon_preset("Pilum", WeaponGroup::Spears, 8.0, "2d6p", "5/80 feet", 0.0),
        weapon_preset(
            "Ranseur",
            WeaponGroup::Spears,
            13.0,
            "2d6p+3^3",
            "8 feet",
            8.0,
        ),
        weapon_preset(
            "Short Spear",
            WeaponGroup::Spears,
            12.0,
            "d4p+d6p",
            "5 feet",
            5.0,
        ),
        weapon_preset("Spear", WeaponGroup::Spears, 12.0, "2d6p", "13 feet", 13.0),
        weapon_preset(
            "Spetum",
            WeaponGroup::Spears,
            13.0,
            "2d8p+3",
            "8 feet",
            8.0,
        ),
        weapon_preset(
            "Trident",
            WeaponGroup::Spears,
            12.0,
            "d6p+d8p+3",
            "6 feet",
            6.0,
        ),
    ]
}

fn default_armor_catalog() -> Vec<ArmorEntry> {
    vec![ArmorEntry {
        label: "None".to_string(),
        armor: None,
    }]
}

fn load_catalogs() -> Result<(Vec<WeaponPreset>, Vec<ArmorEntry>), String> {
    let weapons = load_weapon_catalog("data/weapons.json")?;
    let armor = load_armor_catalog("data/armor.json")?;
    let _materials = load_materials("data/materials.json")?;
    Ok((weapons, armor))
}

#[derive(Deserialize)]
struct WeaponsFile {
    weapons: Vec<WeaponJson>,
    #[allow(dead_code)]
    shields: Vec<ShieldJson>,
}

#[derive(Deserialize)]
struct WeaponJson {
    name: String,
    group: String,
    speed: String,
    damage: Option<String>,
    armor_penetration: Option<i32>,
    defense_bonus_always: Option<bool>,
    #[serde(rename = "reach_or_range")]
    reach_or_range: Option<String>,
}

#[derive(Deserialize)]
struct ShieldJson {
    #[allow(dead_code)]
    name: String,
}

#[derive(Deserialize)]
struct ArmorFile {
    armor: Vec<ArmorJson>,
}

#[derive(Deserialize)]
struct ArmorJson {
    name: String,
    region: String,
    damage_reduction: i32,
    defense_adjustment: i32,
    initiative_modifier: i32,
    speed_modifier: i32,
    #[serde(rename = "type")]
    armor_type: String,
    weight_lbs: Option<f32>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct MaterialsFile {
    metals: Vec<MaterialJson>,
    fabrics: Vec<MaterialJson>,
    woods: Vec<MaterialJson>,
}

#[derive(Deserialize)]
struct MaterialJson {
    #[allow(dead_code)]
    tier: i32,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    weight_multiplier: f32,
}

fn load_weapon_catalog(path: &str) -> Result<Vec<WeaponPreset>, String> {
    let data = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let parsed: WeaponsFile = serde_json::from_str(&data).map_err(|err| err.to_string())?;
    let mut catalog = Vec::new();
    for entry in parsed.weapons {
        let group = match weapon_group_from_str(&entry.group) {
            Some(group) => group,
            None => continue,
        };
        let speed_value = parse_leading_number(&entry.speed);
        let reach_label = entry
            .reach_or_range
            .clone()
            .unwrap_or_else(|| "-".to_string());
        let reach_ft = parse_reach_ft(&reach_label);
        let damage_expr = entry.damage.unwrap_or_else(|| "-".to_string());
        catalog.push(WeaponPreset {
            name: entry.name,
            group,
            speed: speed_value,
            speed_label: entry.speed,
            damage_expr,
            reach_label,
            reach_ft,
            armor_pen: entry.armor_penetration.unwrap_or(0),
            defense_bonus_always: entry.defense_bonus_always.unwrap_or(false),
        });
    }
    if catalog.is_empty() {
        Err("No weapons loaded from JSON".to_string())
    } else {
        Ok(catalog)
    }
}

fn load_armor_catalog(path: &str) -> Result<Vec<ArmorEntry>, String> {
    let data = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let parsed: ArmorFile = serde_json::from_str(&data).map_err(|err| err.to_string())?;
    let mut catalog = Vec::new();
    catalog.push(ArmorEntry {
        label: "None".to_string(),
        armor: None,
    });
    for entry in parsed.armor {
        if entry.name == "None" {
            continue;
        }
        let region = match armor_region_from_str(&entry.region) {
            Some(region) => region,
            None => continue,
        };
        let armor_type = match armor_type_from_str(&entry.armor_type) {
            Some(kind) => kind,
            None => continue,
        };
        let label = format!("{} ({})", entry.name, entry.region);
        let armor = Armor {
            name: leak_str(entry.name),
            region,
            damage_reduction: entry.damage_reduction,
            defense_adj: entry.defense_adjustment,
            initiative_mod: entry.initiative_modifier,
            speed_mod: entry.speed_modifier,
            armor_type,
            weight_lbs: entry.weight_lbs.unwrap_or(0.0),
        };
        catalog.push(ArmorEntry {
            label,
            armor: Some(armor),
        });
    }
    Ok(catalog)
}

fn load_materials(path: &str) -> Result<MaterialsFile, String> {
    let data = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&data).map_err(|err| err.to_string())
}

fn weapon_group_from_str(group: &str) -> Option<WeaponGroup> {
    match group {
        "Unarmed" => Some(WeaponGroup::Unarmed),
        "Axes" => Some(WeaponGroup::Axes),
        "Basic" => Some(WeaponGroup::Basic),
        "Blunt" => Some(WeaponGroup::Blunt),
        "Bows" => Some(WeaponGroup::Bows),
        "Crossbows" => Some(WeaponGroup::Crossbows),
        "Double" => Some(WeaponGroup::Double),
        "Ensnaring" => Some(WeaponGroup::Ensnaring),
        "Lashes" => Some(WeaponGroup::Lashes),
        "Large Swords" => Some(WeaponGroup::LargeSwords),
        "Small Swords" => Some(WeaponGroup::SmallSwords),
        "Polearms" => Some(WeaponGroup::Polearms),
        "Spears" => Some(WeaponGroup::Spears),
        "Shields" => Some(WeaponGroup::Shields),
        _ => None,
    }
}

fn armor_region_from_str(region: &str) -> Option<character::ArmorRegion> {
    match region {
        "Northern" => Some(character::ArmorRegion::Northern),
        "Southern" => Some(character::ArmorRegion::Southern),
        _ => None,
    }
}

fn armor_type_from_str(kind: &str) -> Option<character::ArmorType> {
    match kind {
        "None" => Some(character::ArmorType::None),
        "Light" => Some(character::ArmorType::Light),
        "Medium" => Some(character::ArmorType::Medium),
        "Heavy" => Some(character::ArmorType::Heavy),
        _ => None,
    }
}

fn parse_leading_number(value: &str) -> f32 {
    let mut started = false;
    let mut buf = String::new();
    for ch in value.chars() {
        if ch.is_ascii_digit() || (ch == '.' && started) {
            started = true;
            buf.push(ch);
        } else if started {
            break;
        }
    }
    buf.parse::<f32>().unwrap_or(0.0)
}

fn parse_reach_ft(value: &str) -> f32 {
    if value.contains('/') {
        return 0.0;
    }
    parse_leading_number(value)
}

fn leak_str(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([980.0, 560.0]),
        ..Default::default()
    };
    eframe::run_native(
        "HackMaster Simulator",
        options,
        Box::new(|_cc| Ok(Box::new(SimGuiApp::new()))),
    )
}
