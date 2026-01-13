use eframe::egui::epaint::Hsva;
use eframe::egui::{self, Color32};
use egui_plot::{GridInput, GridMark, Legend, Line, Plot, PlotPoints, Points, VLine};
use hackmaster_sim::character::WeaponGroup;
use hackmaster_sim::core::rules::{effective_armor_value, expected_damage_expr};
use hackmaster_sim::data;
use hackmaster_sim::game_logic::{self, WeaponCatalog, WeaponPreset};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

const ARMOR_MAX: i32 = 15;
const DEFAULT_SIM_DURATION: f64 = 60.0;
const MIN_DURATION: f64 = 1e-3;

#[derive(Clone, Copy)]
struct GlobalAdjustments {
    damage_bonus: f64,
    speed_reduction: f64,
    enable_two_handed: bool,
}

impl GlobalAdjustments {
    const fn new(damage_bonus: f64, speed_reduction: f64, enable_two_handed: bool) -> Self {
        Self {
            damage_bonus,
            speed_reduction,
            enable_two_handed,
        }
    }

    fn adjusted_speed(&self, weapon: &WeaponPreset) -> f64 {
        let min_speed = weapon.size.min_speed() as f64;
        let mut base_speed = weapon.speed as f64;
        if self.enable_two_handed && game_logic::weapon_allows_two_handed_mode(weapon) {
            base_speed += game_logic::TWO_HANDED_SPEED_PENALTY as f64;
        }
        let max_reduction = (base_speed - min_speed).max(0.0);
        let applied_reduction = self.speed_reduction.min(max_reduction);
        (base_speed - applied_reduction).max(min_speed)
    }

    fn two_handed_damage_bonus(&self, weapon: &WeaponPreset) -> f64 {
        if self.enable_two_handed
            && game_logic::weapon_allows_two_handed_mode(weapon)
            && !weapon_has_flat_three_bonus(weapon)
        {
            game_logic::TWO_HANDED_DAMAGE_BONUS as f64
        } else {
            0.0
        }
    }
}

impl Default for GlobalAdjustments {
    fn default() -> Self {
        Self::new(0.0, 0.0, false)
    }
}

const WEAPON_GROUPS: [WeaponGroup; 13] = [
    WeaponGroup::Unarmed,
    WeaponGroup::Axes,
    WeaponGroup::Blunt,
    WeaponGroup::Basic,
    WeaponGroup::Bows,
    WeaponGroup::Crossbows,
    WeaponGroup::Double,
    WeaponGroup::Ensnaring,
    WeaponGroup::Lashes,
    WeaponGroup::LargeSwords,
    WeaponGroup::SmallSwords,
    WeaponGroup::Polearms,
    WeaponGroup::Spears,
];

fn weapon_group_label(group: WeaponGroup) -> &'static str {
    match group {
        WeaponGroup::Unarmed => "Unarmed",
        WeaponGroup::Axes => "Axes",
        WeaponGroup::Blunt => "Blunt Weapons",
        WeaponGroup::Basic => "Basic Weapons",
        WeaponGroup::Bows => "Bows",
        WeaponGroup::Crossbows => "Crossbows",
        WeaponGroup::Double => "Double Weapons",
        WeaponGroup::Ensnaring => "Ensnaring",
        WeaponGroup::Lashes => "Lashes",
        WeaponGroup::LargeSwords => "Large Swords",
        WeaponGroup::SmallSwords => "Small Swords",
        WeaponGroup::Polearms => "Polearms",
        WeaponGroup::Spears => "Spears",
        WeaponGroup::Shields => "Shields",
    }
}

fn weapon_group_slug(group: WeaponGroup) -> &'static str {
    match group {
        WeaponGroup::Unarmed => "unarmed",
        WeaponGroup::Axes => "axes",
        WeaponGroup::Blunt => "blunt",
        WeaponGroup::Basic => "basic",
        WeaponGroup::Bows => "bows",
        WeaponGroup::Crossbows => "crossbows",
        WeaponGroup::Double => "double",
        WeaponGroup::Ensnaring => "ensnaring",
        WeaponGroup::Lashes => "lashes",
        WeaponGroup::LargeSwords => "large_swords",
        WeaponGroup::SmallSwords => "small_swords",
        WeaponGroup::Polearms => "polearms",
        WeaponGroup::Spears => "spears",
        WeaponGroup::Shields => "shields",
    }
}

#[derive(Debug, Clone)]
struct WeaponLine {
    name: String,
    color: Color32,
    points: Vec<[f64; 2]>,
    values: Vec<f64>,
}

#[derive(Debug, Clone)]
struct WeaponPlotData {
    lines: Vec<WeaponLine>,
    y_max: f64,
}

#[derive(Debug, Clone)]
struct HoverEntry {
    color: Color32,
    name: String,
    value: f64,
}

struct HoverDetails {
    has_dataset: bool,
    armor_value: Option<i32>,
    entries: Vec<HoverEntry>,
}

impl Default for HoverDetails {
    fn default() -> Self {
        Self {
            has_dataset: false,
            armor_value: None,
            entries: Vec::new(),
        }
    }
}

struct WeaponPlotApp {
    weapon_catalog: WeaponCatalog,
    datasets: HashMap<WeaponGroup, WeaponPlotData>,
    current_group: WeaponGroup,
    speed_reduction: f64,
    damage_bonus: f64,
    two_handed: bool,
    sim_duration: f64,
}

impl WeaponPlotApp {
    fn with_datasets(
        weapon_catalog: WeaponCatalog,
        datasets: HashMap<WeaponGroup, WeaponPlotData>,
        adjustments: GlobalAdjustments,
        sim_duration: f64,
    ) -> Self {
        Self {
            weapon_catalog,
            datasets,
            current_group: WeaponGroup::Unarmed,
            speed_reduction: adjustments.speed_reduction,
            damage_bonus: adjustments.damage_bonus,
            two_handed: adjustments.enable_two_handed,
            sim_duration,
        }
    }

    fn rebuild_datasets(&mut self) {
        let adjustments =
            GlobalAdjustments::new(self.damage_bonus, self.speed_reduction, self.two_handed);
        self.datasets = build_datasets(&self.weapon_catalog, adjustments, self.sim_duration);
    }
}

fn build_datasets(
    weapon_catalog: &WeaponCatalog,
    adjustments: GlobalAdjustments,
    sim_duration: f64,
) -> HashMap<WeaponGroup, WeaponPlotData> {
    let armor_values: Vec<f64> = (0..=ARMOR_MAX).map(|v| v as f64).collect();
    let mut datasets = HashMap::new();

    for &group in WEAPON_GROUPS.iter() {
        let specs: Vec<&WeaponPreset> = weapon_catalog
            .entries()
            .iter()
            .filter(|weapon| weapon.group == group)
            .collect();

        if specs.is_empty() {
            continue;
        }

        let mut lines = Vec::new();
        let mut y_max = 0.0f64;

        for (idx, weapon) in specs.iter().enumerate() {
            let (points, values, max_val) =
                compute_weapon_curve(weapon, &armor_values, adjustments, sim_duration);
            y_max = y_max.max(max_val);

            let hue = idx as f32 / specs.len() as f32;
            let hsv = Hsva {
                h: hue,
                s: 0.65,
                v: 0.9,
                a: 1.0,
            };
            let color: Color32 = hsv.into();

            lines.push(WeaponLine {
                name: weapon.name.clone(),
                color,
                points,
                values,
            });
            
        }

        datasets.insert(
            group,
            WeaponPlotData {
                lines,
                y_max: y_max.max(0.01),
            },
        );
    }

    datasets
}

impl eframe::App for WeaponPlotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut avg_details = HoverDetails::default();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hackmaster Weapon Damage per Speed");
            ui.label("Pan/zoom inside the chart and hover to inspect values.");

            ui.horizontal(|ui| {
                ui.label("Global adjustments:");
                let mut changed = false;
                changed |= ui
                    .add(
                        egui::Slider::new(&mut self.speed_reduction, 0.0..=10.0)
                            .step_by(1.0)
                            .text("Speed reduction"),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::Slider::new(&mut self.damage_bonus, 0.0..=10.0)
                            .step_by(1.0)
                            .text("Damage bonus"),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::Slider::new(&mut self.sim_duration, 0.0..=60.0)
                            .step_by(1.0)
                            .text("Sim over time (s)"),
                    )
                    .changed();
                changed |= ui.checkbox(&mut self.two_handed, "2h weapons").changed();
                if changed {
                    self.rebuild_datasets();
                }
            });

            ui.horizontal(|ui| {
                ui.label("Weapon group:");
                for group in WEAPON_GROUPS.iter() {
                    ui.selectable_value(&mut self.current_group, *group, weapon_group_label(*group));
                }
            });

            ui.separator();

            if let Some(dataset) = self.datasets.get(&self.current_group) {
                show_weapon_plot(
                    ui,
                    "avg_damage_plot",
                    "Average DPS",
                    &dataset.lines,
                    dataset.y_max,
                    &mut avg_details,
                );
            } else {
                avg_details.has_dataset = false;
                ui.label("No data available for this weapon group.");
            }
        });

        egui::SidePanel::right("value_panel")
            .resizable(false)
            .min_width(260.0)
            .show(ctx, |ui| {
                render_hover_details(ui, "Average DPS", &avg_details);
            });
    }
}

fn weapon_has_flat_three_bonus(weapon: &WeaponPreset) -> bool {
    damage_expr_has_flat_three(&weapon.damage_expr)
}

fn damage_expr_has_flat_three(expr: &str) -> bool {
    let bytes = expr.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        if bytes[idx] == b'+' {
            let mut j = idx + 1;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'3' {
                let next = j + 1;
                if next == bytes.len() || !bytes[next].is_ascii_digit() {
                    return true;
                }
            }
            idx = j;
        } else {
            idx += 1;
        }
        idx += 1;
    }
    false
}

fn show_weapon_plot(
    ui: &mut egui::Ui,
    plot_id: &str,
    heading: &str,
    lines: &[WeaponLine],
    y_max: f64,
    details: &mut HoverDetails,
) {
    ui.heading(heading);
    if lines.is_empty() {
        details.has_dataset = false;
        ui.label("No weapon data available.");
        return;
    }

    details.has_dataset = true;
    details.entries.clear();
    let x_max = ARMOR_MAX as f64 + 2.0;
    let y_view = (y_max * 1.2).max(0.1);

    let plot = Plot::new(plot_id)
        .legend(Legend::default())
        .include_x(-1.0)
        .include_x(x_max)
        .include_y(0.0)
        .include_y(y_view)
        .view_aspect(16.0 / 9.0)
        .allow_scroll(false)
        .allow_boxed_zoom(false)
        .x_grid_spacer(integer_grid_marks)
        .x_axis_formatter(|mark, _| format!("{:.0}", mark.value));

    let response = plot.show(ui, |plot_space| {
        let pointer = plot_space.pointer_coordinate();
        let snapped = if plot_space.response().hovered() {
            pointer.map(|pos| pos.x.round().clamp(0.0, ARMOR_MAX as f64))
        } else {
            None
        };

        for line in lines {
            let points = PlotPoints::from_iter(line.points.iter().copied());
            let plot_line = Line::new(points)
                .name(line.name.clone())
                .color(line.color)
                .highlight(true);
            plot_space.line(plot_line);
        }

        if let Some(snapped_x) = snapped {
            let idx = snapped_x as usize;
            plot_space.vline(VLine::new(snapped_x).color(Color32::LIGHT_GRAY));

            for line in lines {
                if let Some(&value) = line.values.get(idx) {
                    let marker = Points::new(vec![[snapped_x, value]])
                        .radius(4.0)
                        .color(line.color)
                        .name(line.name.clone());
                    plot_space.points(marker);
                }
            }
        }

        snapped
    });

    if let Some(armor_value) = response.inner {
        let idx = armor_value as usize;
        details.armor_value = Some(armor_value as i32);
        for line in lines {
            if let Some(&value) = line.values.get(idx) {
                details.entries.push(HoverEntry {
                    color: line.color,
                    name: line.name.clone(),
                    value,
                });
            }
        }
    } else {
        details.armor_value = None;
    }
}

fn render_hover_details(ui: &mut egui::Ui, label: &str, details: &HoverDetails) {
    ui.heading(label);
    ui.separator();

    if !details.has_dataset {
        ui.label("No data available for this weapon group.");
        return;
    }

    if let Some(armor) = details.armor_value {
        ui.label(format!("Armor: {}", armor));
        ui.add_space(6.0);
        if details.entries.is_empty() {
            ui.label("Hover over lines to view results.");
        } else {
                for entry in &details.entries {
                    ui.colored_label(
                        entry.color,
                        format!("{}: {:.3} dps", entry.name, entry.value),
                    );
                }
            }
    } else {
        ui.label("Hover inside the chart to view exact values.");
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

fn compute_weapon_curve(
    weapon: &WeaponPreset,
    armor_values: &[f64],
    adjustments: GlobalAdjustments,
    sim_duration: f64,
) -> (Vec<[f64; 2]>, Vec<f64>, f64) {
    let avg_damage = expected_damage_expr(&weapon.damage_expr);
    let mut points = Vec::with_capacity(armor_values.len());
    let mut values = Vec::with_capacity(armor_values.len());
    let mut max_val = 0.0f64;
    let adjusted_speed = adjustments.adjusted_speed(weapon);
    let adjusted_damage =
        avg_damage + adjustments.damage_bonus + adjustments.two_handed_damage_bonus(weapon);

    for &armor in armor_values {
        let effective_armor = effective_armor_value(armor, weapon.armor_pen);
        let net = (adjusted_damage - effective_armor).max(0.0);
        let per_second =
            average_damage_per_second(net, adjusted_speed, sim_duration);
        max_val = max_val.max(per_second);
        points.push([armor, per_second]);
        values.push(per_second);
    }

    (points, values, max_val)
}

fn hits_within_duration(speed: f64, duration: f64) -> u32 {
    if duration <= 0.0 {
        1
    } else {
        (duration / speed).floor() as u32 + 1
    }
}

fn average_damage_per_second(net_damage: f64, speed: f64, duration: f64) -> f64 {
    if net_damage <= 0.0 {
        return 0.0;
    }
    if duration <= 0.0 {
        return net_damage / speed;
    }
    let duration_for_avg = duration.max(MIN_DURATION);
    let hits = hits_within_duration(speed, duration) as f64;
    (net_damage * hits) / duration_for_avg
}

fn export_headless_report(
    datasets: &HashMap<WeaponGroup, WeaponPlotData>,
) -> std::io::Result<()> {
    let out_dir = Path::new("headless_output");
    fs::create_dir_all(out_dir)?;

    for group in WEAPON_GROUPS.iter() {
        if let Some(data) = datasets.get(group) {
            let avg_path = out_dir.join(format!("{}_avg.csv", weapon_group_slug(*group)));
            write_dataset_csv(&avg_path, &data.lines)?;
        }
    }

    Ok(())
}

fn write_dataset_csv(path: &Path, lines: &[WeaponLine]) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "armor,weapon,damage_per_speed")?;
    for line in lines {
        for (idx, point) in line.points.iter().enumerate() {
            let armor = point[0] as i32;
            let value = line.values[idx];
            writeln!(writer, "{},{},{}", armor, line.name, value)?;
        }
    }
    Ok(())
}

fn load_weapon_catalog() -> WeaponCatalog {
    data::load_catalogs()
        .map(|(weapons, _, _)| weapons)
        .unwrap_or_else(|_| game_logic::default_weapon_catalog())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_logic::{WeaponHandedness, WeaponSize};

    const EPS: f64 = 1e-6;

    fn make_weapon(
        name: &str,
        damage_expr: &str,
        speed: f64,
        armor_pen: i32,
        size: WeaponSize,
        group: WeaponGroup,
        handedness: WeaponHandedness,
    ) -> WeaponPreset {
        WeaponPreset {
            name: name.to_string(),
            group,
            speed: speed as f32,
            speed_label: speed.to_string(),
            jab_speed: None,
            jab_speed_label: None,
            jab_special_expr: None,
            damage_expr: damage_expr.to_string(),
            shield_damage_expr: None,
            reach_label: "-".to_string(),
            reach_ft: 1.0,
            range_bands_feet: None,
            armor_pen,
            defense_bonus_always: false,
            size,
            handedness,
            ammunition: None,
        }
    }

    #[test]
    fn expected_damage_penetrating_die() {
        let avg = expected_damage_expr("d6p");
        assert!((avg - 4.0).abs() < EPS);
    }
    #[test]
    fn expected_damage_complex_expression() {
        let avg = expected_damage_expr("d8p+2d10p+3");
        assert!((avg - 20.0).abs() < EPS);
    }

    #[test]
    fn expected_damage_with_parentheses() {
        let avg = expected_damage_expr("(d4p-2)+(d4p-2)");
        assert!((avg - 2.0).abs() < EPS);
    }

    #[test]
    fn armor_penetration_reduces_effective_armor() {
        let weapon = make_weapon(
            "Test Warhammer",
            "d8p+d10p",
            12.0,
            1,
            WeaponSize::Medium,
            WeaponGroup::Blunt,
            WeaponHandedness::OneHanded,
        );
        let armor_values = vec![7.0];
        let (_, values, _) = compute_weapon_curve(
            &weapon,
            &armor_values,
            GlobalAdjustments::default(),
            DEFAULT_SIM_DURATION,
        );
        let avg = expected_damage_expr(&weapon.damage_expr);
        let effective = effective_armor_value(armor_values[0], weapon.armor_pen);
        let net = (avg - effective).max(0.0);
        let expected = average_damage_per_second(net, weapon.speed as f64, DEFAULT_SIM_DURATION);
        assert!((values[0] - expected).abs() < EPS);
    }

    #[test]
    fn armor_penetration_does_not_increase_damage_past_zero_armor() {
        let weapon = make_weapon(
            "Piercing Club",
            "d6p",
            10.0,
            3,
            WeaponSize::Medium,
            WeaponGroup::Basic,
            WeaponHandedness::OneHanded,
        );
        let armor_values = vec![1.0];
        let (_, values, _) = compute_weapon_curve(
            &weapon,
            &armor_values,
            GlobalAdjustments::default(),
            DEFAULT_SIM_DURATION,
        );
        let avg = expected_damage_expr(&weapon.damage_expr);
        let net =
            (avg - effective_armor_value(armor_values[0], weapon.armor_pen)).max(0.0);
        let expected = average_damage_per_second(net, weapon.speed as f64, DEFAULT_SIM_DURATION);
        assert!((values[0] - expected).abs() < EPS);
    }

    #[test]
    fn armor_pen_only_affects_dr_above_five() {
        let low = effective_armor_value(4.0, 3);
        assert!((low - 4.0).abs() < EPS);

        let mid = effective_armor_value(7.0, 2);
        assert!((mid - 5.0).abs() < EPS);

        let high = effective_armor_value(10.0, 1);
        assert!((high - 9.0).abs() < EPS);
    }

    #[test]
    fn speed_floor_respected_for_all_weapons() {
        let adjustments = GlobalAdjustments::new(0.0, 10.0, false);
        let weapon_catalog = load_weapon_catalog();
        for weapon in weapon_catalog.entries() {
            let adjusted = adjustments.adjusted_speed(weapon);
            assert!(
                adjusted >= weapon.size.min_speed() as f64 - EPS,
                "Weapon {} dropped below its floor",
                weapon.name
            );
        }
    }

    #[test]
    fn two_handed_eligibility_matches_rules() {
        let weapon_catalog = load_weapon_catalog();
        for weapon in weapon_catalog.entries() {
            let expected = weapon.handedness == WeaponHandedness::OneHanded
                && matches!(weapon.size, WeaponSize::Medium | WeaponSize::Large);
            assert_eq!(
                game_logic::weapon_allows_two_handed_mode(weapon),
                expected,
                "Weapon {} did not match the eligibility rules",
                weapon.name
            );
        }
    }
}

fn main() -> eframe::Result<()> {
    apply_wsl_winit_workaround();
    let adjustments = GlobalAdjustments::default();
    let sim_duration = DEFAULT_SIM_DURATION;
    let weapon_catalog = load_weapon_catalog();
    let datasets = build_datasets(&weapon_catalog, adjustments, sim_duration);
    let lacks_display = cfg!(target_family = "unix")
        && std::env::var("DISPLAY").is_err()
        && std::env::var("WAYLAND_DISPLAY").is_err();

    if lacks_display {
        eprintln!(
            "No GUI display detected (missing DISPLAY/WAYLAND_DISPLAY). \
             Exporting data to headless_output/ for offline review."
        );
        if let Err(err) = export_headless_report(&datasets) {
            eprintln!("Failed to export fallback data: {err}");
        }
        return Ok(());
    }

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(egui::vec2(1100.0, 750.0))
            .with_min_inner_size(egui::vec2(600.0, 400.0)),
        ..Default::default()
    };

    let datasets_for_app = datasets.clone();
    let weapon_catalog_for_app = weapon_catalog.clone();
    let ui_adjustments = adjustments;
    match eframe::run_native(
        "Hackmaster Blunt Weapon Damage per Speed",
        native_options,
        Box::new(move |_| {
            Ok(Box::new(WeaponPlotApp::with_datasets(
                weapon_catalog_for_app,
                datasets_for_app.clone(),
                ui_adjustments,
                sim_duration,
            )))
        }),
    ) {
        Ok(()) => Ok(()),
        Err(err) => {
            eprintln!("Failed to start GUI ({err}). Exporting data to headless_output/.");
            if let Err(export_err) = export_headless_report(&datasets) {
                eprintln!("Also failed to export fallback data: {export_err}");
            }
            Ok(())
        }
    }
}

fn apply_wsl_winit_workaround() {
    let is_wsl = std::env::var("WSL_DISTRO_NAME").is_ok()
        || std::fs::read_to_string("/proc/sys/kernel/osrelease")
            .map(|content| content.to_lowercase().contains("microsoft"))
            .unwrap_or(false);

    if is_wsl && std::env::var("WINIT_UNIX_BACKEND").is_err() {
        // SAFETY: we only touch our own process environment.
        unsafe {
            std::env::set_var("WINIT_UNIX_BACKEND", "x11");
        }
    }
}
