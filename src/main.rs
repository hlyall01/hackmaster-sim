use eframe::egui::epaint::Hsva;
use eframe::egui::{self, Color32};
use egui_plot::{GridInput, GridMark, Legend, Line, Plot, PlotPoints, Points, VLine};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

const ARMOR_MAX: i32 = 15;
const DEFAULT_SIM_DURATION: f64 = 60.0;
const MIN_DURATION: f64 = 1e-3;

const TWO_HANDED_DAMAGE_BONUS: f64 = 3.0;
const TWO_HANDED_SPEED_PENALTY: f64 = 2.0;

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

    fn adjusted_speed(&self, weapon: &WeaponSpec) -> f64 {
        let min_speed = weapon.size.min_speed();
        let mut base_speed = weapon.speed;
        if self.enable_two_handed && weapon_allows_two_handed_mode(weapon) {
            base_speed += TWO_HANDED_SPEED_PENALTY;
        }
        let max_reduction = (base_speed - min_speed).max(0.0);
        let applied_reduction = self.speed_reduction.min(max_reduction);
        (base_speed - applied_reduction).max(min_speed)
    }

    fn two_handed_damage_bonus(&self, weapon: &WeaponSpec) -> f64 {
        if self.enable_two_handed
            && weapon_allows_two_handed_mode(weapon)
            && !weapon_has_flat_three_bonus(weapon)
        {
            TWO_HANDED_DAMAGE_BONUS
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum WeaponCategory {
    Unarmed,
    Axes,
    Blunt,
    Basic,
    Bows,
    Crossbows,
    Double,
    Ensnaring,
    Lashes,
    LargeSwords,
    SmallSwords,
    Polearms,
    Spears,
}

impl WeaponCategory {
    const ALL: [WeaponCategory; 13] = [
        WeaponCategory::Unarmed,
        WeaponCategory::Axes,
        WeaponCategory::Blunt,
        WeaponCategory::Basic,
        WeaponCategory::Bows,
        WeaponCategory::Crossbows,
        WeaponCategory::Double,
        WeaponCategory::Ensnaring,
        WeaponCategory::Lashes,
        WeaponCategory::LargeSwords,
        WeaponCategory::SmallSwords,
        WeaponCategory::Polearms,
        WeaponCategory::Spears,
    ];

    fn label(&self) -> &'static str {
        match self {
            WeaponCategory::Unarmed => "Unarmed",
            WeaponCategory::Axes => "Axes",
            WeaponCategory::Blunt => "Blunt Weapons",
            WeaponCategory::Basic => "Basic Weapons",
            WeaponCategory::Bows => "Bows",
            WeaponCategory::Crossbows => "Crossbows",
            WeaponCategory::Double => "Double Weapons",
            WeaponCategory::Ensnaring => "Ensnaring",
            WeaponCategory::Lashes => "Lashes",
            WeaponCategory::LargeSwords => "Large Swords",
            WeaponCategory::SmallSwords => "Small Swords",
            WeaponCategory::Polearms => "Polearms",
            WeaponCategory::Spears => "Spears",
        }
    }

    fn slug(&self) -> &'static str {
        match self {
            WeaponCategory::Unarmed => "unarmed",
            WeaponCategory::Axes => "axes",
            WeaponCategory::Blunt => "blunt",
            WeaponCategory::Basic => "basic",
            WeaponCategory::Bows => "bows",
            WeaponCategory::Crossbows => "crossbows",
            WeaponCategory::Double => "double",
            WeaponCategory::Ensnaring => "ensnaring",
            WeaponCategory::Lashes => "lashes",
            WeaponCategory::LargeSwords => "large_swords",
            WeaponCategory::SmallSwords => "small_swords",
            WeaponCategory::Polearms => "polearms",
            WeaponCategory::Spears => "spears",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WeaponSize {
    Small,
    Medium,
    Large,
}

impl WeaponSize {
    const fn min_speed(self) -> f64 {
        match self {
            WeaponSize::Small => 2.0,
            WeaponSize::Medium => 3.0,
            WeaponSize::Large => 4.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct WeaponSpec {
    name: &'static str,
    damage_expr: &'static str,
    speed: f64,
    armor_pen: i32,
    size: WeaponSize,
    category: WeaponCategory,
}

fn weapon_allows_two_handed_mode(weapon: &WeaponSpec) -> bool {
    if weapon.name == "Spear" {
        return true;
    }
    weapon.size == WeaponSize::Medium && weapon.category != WeaponCategory::Bows
}

#[derive(Debug, Clone)]
struct WeaponLine {
    name: &'static str,
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
    name: &'static str,
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
    datasets: HashMap<WeaponCategory, WeaponPlotData>,
    current_category: WeaponCategory,
    speed_reduction: f64,
    damage_bonus: f64,
    two_handed: bool,
    sim_duration: f64,
}

impl WeaponPlotApp {
    fn with_datasets(
        datasets: HashMap<WeaponCategory, WeaponPlotData>,
        adjustments: GlobalAdjustments,
        sim_duration: f64,
    ) -> Self {
        Self {
            datasets,
            current_category: WeaponCategory::Unarmed,
            speed_reduction: adjustments.speed_reduction,
            damage_bonus: adjustments.damage_bonus,
            two_handed: adjustments.enable_two_handed,
            sim_duration,
        }
    }

    fn rebuild_datasets(&mut self) {
        let adjustments =
            GlobalAdjustments::new(self.damage_bonus, self.speed_reduction, self.two_handed);
        self.datasets = build_datasets(adjustments, self.sim_duration);
    }
}

fn build_datasets(
    adjustments: GlobalAdjustments,
    sim_duration: f64,
) -> HashMap<WeaponCategory, WeaponPlotData> {
    let armor_values: Vec<f64> = (0..=ARMOR_MAX).map(|v| v as f64).collect();
    let mut datasets = HashMap::new();

    for &category in WeaponCategory::ALL.iter() {
        let specs: Vec<&WeaponSpec> = WEAPONS
            .iter()
            .filter(|weapon| weapon.category == category)
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
                name: weapon.name,
                color,
                points,
                values,
            });
            
        }

        datasets.insert(
            category,
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
                for category in WeaponCategory::ALL.iter() {
                    ui.selectable_value(&mut self.current_category, *category, category.label());
                }
            });

            ui.separator();

            if let Some(dataset) = self.datasets.get(&self.current_category) {
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

fn expected_damage(expr: &str) -> f64 {
    let cleaned = expr.replace(' ', "");
    evaluate_expression(cleaned.as_str())
}

fn weapon_has_flat_three_bonus(weapon: &WeaponSpec) -> bool {
    damage_expr_has_flat_three(weapon.damage_expr)
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

fn evaluate_expression(expr: &str) -> f64 {
    if expr.is_empty() {
        return 0.0;
    }

    let mut total = 0.0;
    let mut idx = 0;
    let chars: Vec<char> = expr.chars().collect();
    while idx < chars.len() {
        let mut sign = 1.0;
        if chars[idx] == '+' {
            idx += 1;
        } else if chars[idx] == '-' {
            sign = -1.0;
            idx += 1;
        }

        let start = idx;
        let mut depth = 0;
        while idx < chars.len() {
            match chars[idx] {
                '(' => {
                    depth += 1;
                    idx += 1;
                }
                ')' => {
                    if depth > 0 {
                        depth -= 1;
                        idx += 1;
                    } else {
                        panic!("Mismatched parentheses in expression: {}", expr);
                    }
                }
                '+' | '-' if depth == 0 => break,
                _ => idx += 1,
            }
        }

        let term = &expr[start..idx];
        if !term.is_empty() {
            total += sign * evaluate_term(term);
        }
    }

    total
}

fn evaluate_term(term: &str) -> f64 {
    let trimmed = strip_outer_parens(term);

    if has_top_level_operator(trimmed) {
        return evaluate_expression(trimmed);
    }

    if let Some(d_pos) = trimmed.find('d') {
        let count = if d_pos == 0 {
            1.0
        } else {
            trimmed[..d_pos].parse::<f64>().unwrap_or(1.0)
        };

        let after_d = &trimmed[d_pos + 1..];
        let mut digits_end = 0;
        for ch in after_d.chars() {
            if ch.is_ascii_digit() {
                digits_end += ch.len_utf8();
            } else {
                break;
            }
        }

        let (sides_str, rest) = after_d.split_at(digits_end);
        let sides = sides_str
            .parse::<f64>()
            .expect("Failed to parse die sides for term");
        let penetrating = rest.starts_with('p');

        let single = if penetrating {
            (sides + 2.0) / 2.0
        } else {
            (sides + 1.0) / 2.0
        };

        count * single
    } else {
        trimmed
            .parse::<f64>()
            .expect("Failed to parse constant damage modifier")
    }
}

fn strip_outer_parens(mut s: &str) -> &str {
    loop {
        let bytes = s.as_bytes();
        if bytes.len() >= 2 && bytes[0] == b'(' && bytes[bytes.len() - 1] == b')' {
            let mut depth = 0;
            let mut balanced = true;
            for (i, ch) in s.chars().enumerate() {
                match ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 && i != s.len() - 1 {
                            balanced = false;
                            break;
                        }
                    }
                    _ => (),
                }
            }
            if balanced && depth == 0 {
                s = &s[1..s.len() - 1];
            } else {
                break;
            }
        } else {
            break;
        }
    }
    s
}

fn has_top_level_operator(s: &str) -> bool {
    let mut depth = 0;
    for (i, ch) in s.chars().enumerate() {
        match ch {
            '(' => depth += 1,
            ')' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            '+' | '-' if depth == 0 && i != 0 => return true,
            _ => {}
        }
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
                .name(line.name)
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
                        .name(line.name);
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
                    name: line.name,
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
    weapon: &WeaponSpec,
    armor_values: &[f64],
    adjustments: GlobalAdjustments,
    sim_duration: f64,
) -> (Vec<[f64; 2]>, Vec<f64>, f64) {
    let avg_damage = expected_damage(weapon.damage_expr);
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

fn effective_armor_value(raw: f64, armor_pen: i32) -> f64 {
    if raw < 5.0 || armor_pen <= 0 {
        return raw;
    }

    let extra = raw - 5.0;
    let reduced_extra = (extra - armor_pen as f64).max(0.0);
    5.0 + reduced_extra
}

fn export_headless_report(
    datasets: &HashMap<WeaponCategory, WeaponPlotData>,
) -> std::io::Result<()> {
    let out_dir = Path::new("headless_output");
    fs::create_dir_all(out_dir)?;

    for category in WeaponCategory::ALL.iter() {
        if let Some(data) = datasets.get(category) {
            let avg_path = out_dir.join(format!("{}_avg.csv", category.slug()));
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

const WEAPONS: &[WeaponSpec] = &[
    // Unarmed
    WeaponSpec {
        name: "Fist",
        damage_expr: "(d4p-2)+(d4p-2)",
        speed: 10.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::Unarmed,
    },
    WeaponSpec {
        name: "Antler",
        damage_expr: "2d6p",
        speed: 10.0,
        armor_pen: 0,
        size: WeaponSize::Medium,
        category: WeaponCategory::Unarmed,
    },
    WeaponSpec {
        name: "Claw",
        damage_expr: "1d8p",
        speed: 5.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::Unarmed,
    },
    WeaponSpec {
        name: "Fang",
        damage_expr: "1d10p",
        speed: 10.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::Unarmed,
    },
    WeaponSpec {
        name: "Cestus",
        damage_expr: "2d4p",
        speed: 10.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::Unarmed,
    },
    WeaponSpec {
        name: "Gauntlet",
        damage_expr: "(d4p-1)+(d4p-1)",
        speed: 10.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::Unarmed,
    },
    WeaponSpec {
        name: "Spiked Gauntlet",
        damage_expr: "1d8p",
        speed: 10.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::Unarmed,
    },
    // Axes
    WeaponSpec {
        name: "Battle Axe",
        damage_expr: "4d3p",
        speed: 12.0,
        armor_pen: 2,
        size: WeaponSize::Medium,
        category: WeaponCategory::Axes,
    },
    WeaponSpec {
        name: "Executioner's Axe",
        damage_expr: "3d8p+3",
        speed: 18.0,
        armor_pen: 2,
        size: WeaponSize::Large,
        category: WeaponCategory::Axes,
    },
    WeaponSpec {
        name: "Greataxe",
        damage_expr: "3d6p+3",
        speed: 14.0,
        armor_pen: 2,
        size: WeaponSize::Large,
        category: WeaponCategory::Axes,
    },
    WeaponSpec {
        name: "Hand Axe",
        damage_expr: "d4p+d6p",
        speed: 8.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::Axes,
    },
    WeaponSpec {
        name: "Khopesh",
        damage_expr: "2d6p",
        speed: 8.0,
        armor_pen: 0,
        size: WeaponSize::Medium,
        category: WeaponCategory::Axes,
    },
    WeaponSpec {
        name: "Military Pick",
        damage_expr: "3d4p",
        speed: 12.0,
        armor_pen: 2,
        size: WeaponSize::Medium,
        category: WeaponCategory::Axes,
    },
    WeaponSpec {
        name: "Horseman's Pick",
        damage_expr: "d4p+d6p",
        speed: 8.0,
        armor_pen: 1,
        size: WeaponSize::Small,
        category: WeaponCategory::Axes,
    },
    WeaponSpec {
        name: "Scythe",
        damage_expr: "2d6p+3",
        speed: 15.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Axes,
    },
    WeaponSpec {
        name: "Sickle",
        damage_expr: "d6p+d3p",
        speed: 8.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::Axes,
    },
    WeaponSpec {
        name: "Throwing Axe",
        damage_expr: "d4p+d6p",
        speed: 7.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::Axes,
    },
    // Blunt
    WeaponSpec {
        name: "Greatclub",
        damage_expr: "d20p+3",
        speed: 16.0,
        armor_pen: 1,
        size: WeaponSize::Large,
        category: WeaponCategory::Blunt,
    },
    WeaponSpec {
        name: "Greathammer",
        damage_expr: "d8p+2d10p+3",
        speed: 20.0,
        armor_pen: 2,
        size: WeaponSize::Large,
        category: WeaponCategory::Blunt,
    },
    WeaponSpec {
        name: "Hammer",
        damage_expr: "2d6p",
        speed: 8.0,
        armor_pen: 1,
        size: WeaponSize::Small,
        category: WeaponCategory::Blunt,
    },
    WeaponSpec {
        name: "Warhammer",
        damage_expr: "d8p+d10p",
        speed: 12.0,
        armor_pen: 1,
        size: WeaponSize::Medium,
        category: WeaponCategory::Blunt,
    },
    WeaponSpec {
        name: "Mace",
        damage_expr: "d6p+d8p",
        speed: 11.0,
        armor_pen: 2,
        size: WeaponSize::Medium,
        category: WeaponCategory::Blunt,
    },
    WeaponSpec {
        name: "Horseman's Mace",
        damage_expr: "2d6p",
        speed: 10.0,
        armor_pen: 1,
        size: WeaponSize::Medium,
        category: WeaponCategory::Blunt,
    },
    WeaponSpec {
        name: "Maul",
        damage_expr: "2d12p+3",
        speed: 15.0,
        armor_pen: 2,
        size: WeaponSize::Large,
        category: WeaponCategory::Blunt,
    },
    WeaponSpec {
        name: "Morning Star",
        damage_expr: "2d8p",
        speed: 11.0,
        armor_pen: 0,
        size: WeaponSize::Medium,
        category: WeaponCategory::Blunt,
    },
    // Basic
    WeaponSpec {
        name: "Club",
        damage_expr: "d6p+d4p",
        speed: 10.0,
        armor_pen: 0,
        size: WeaponSize::Medium,
        category: WeaponCategory::Basic,
    },
    WeaponSpec {
        name: "Dart",
        damage_expr: "d4p",
        speed: 5.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::Basic,
    },
    WeaponSpec {
        name: "Sling",
        damage_expr: "d4p+d6p",
        speed: 10.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::Basic,
    },
    WeaponSpec {
        name: "Staff",
        damage_expr: "2d4p+3",
        speed: 13.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Basic,
    },
    // Bows
    WeaponSpec {
        name: "Longbow",
        damage_expr: "2d8p",
        speed: 12.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Bows,
    },
    WeaponSpec {
        name: "Recurve Bow",
        damage_expr: "3d4p",
        speed: 11.0,
        armor_pen: 0,
        size: WeaponSize::Medium,
        category: WeaponCategory::Bows,
    },
    WeaponSpec {
        name: "Shortbow",
        damage_expr: "2d6p",
        speed: 12.0,
        armor_pen: 0,
        size: WeaponSize::Medium,
        category: WeaponCategory::Bows,
    },
    WeaponSpec {
        name: "Warbow",
        damage_expr: "3d6p",
        speed: 20.0,
        armor_pen: 1,
        size: WeaponSize::Large,
        category: WeaponCategory::Bows,
    },
    // Crossbows
    WeaponSpec {
        name: "Arbalest",
        damage_expr: "3d8p",
        speed: 90.0,
        armor_pen: 1,
        size: WeaponSize::Large,
        category: WeaponCategory::Crossbows,
    },
    WeaponSpec {
        name: "Light Crossbow",
        damage_expr: "2d6p",
        speed: 20.0,
        armor_pen: 0,
        size: WeaponSize::Medium,
        category: WeaponCategory::Crossbows,
    },
    WeaponSpec {
        name: "Hand Crossbow",
        damage_expr: "2d4p",
        speed: 15.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::Crossbows,
    },
    WeaponSpec {
        name: "Heavy Crossbow",
        damage_expr: "2d10p",
        speed: 60.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Crossbows,
    },
    // Double Weapons
    WeaponSpec {
        name: "Double Axe",
        damage_expr: "4d3p",
        speed: 13.0,
        armor_pen: 2,
        size: WeaponSize::Large,
        category: WeaponCategory::Double,
    },
    WeaponSpec {
        name: "Double Scimitar",
        damage_expr: "2d8p",
        speed: 10.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Double,
    },
    WeaponSpec {
        name: "Dual Scythe",
        damage_expr: "2d6p",
        speed: 16.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Double,
    },
    WeaponSpec {
        name: "Hooked Hammer",
        damage_expr: "d8p+d10p",
        speed: 14.0,
        armor_pen: 1,
        size: WeaponSize::Large,
        category: WeaponCategory::Double,
    },
    WeaponSpec {
        name: "Monk's Spade",
        damage_expr: "2d4p",
        speed: 9.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Double,
    },
    WeaponSpec {
        name: "Spear-Axe",
        damage_expr: "2d6p",
        speed: 13.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Double,
    },
    WeaponSpec {
        name: "Two-Bladed Sword",
        damage_expr: "2d8p",
        speed: 11.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Double,
    },
    // Ensnaring (only damaging weapons)
    WeaponSpec {
        name: "Bola",
        damage_expr: "d4p",
        speed: 10.0,
        armor_pen: 0,
        size: WeaponSize::Medium,
        category: WeaponCategory::Ensnaring,
    },
    // Lashes
    WeaponSpec {
        name: "Flail",
        damage_expr: "2d8p",
        speed: 13.0,
        armor_pen: 1,
        size: WeaponSize::Medium,
        category: WeaponCategory::Lashes,
    },
    WeaponSpec {
        name: "Horseman's Flail",
        damage_expr: "d4p+d6p",
        speed: 11.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::Lashes,
    },
    WeaponSpec {
        name: "Scourge",
        damage_expr: "2d4p",
        speed: 9.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::Lashes,
    },
    WeaponSpec {
        name: "Spiked Chain",
        damage_expr: "2d6p+3",
        speed: 14.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Lashes,
    },
    WeaponSpec {
        name: "Whip",
        damage_expr: "1d6p",
        speed: 8.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::Lashes,
    },
    // Large Swords
    WeaponSpec {
        name: "Broadsword",
        damage_expr: "2d6p+d3p",
        speed: 11.0,
        armor_pen: 0,
        size: WeaponSize::Medium,
        category: WeaponCategory::LargeSwords,
    },
    WeaponSpec {
        name: "Bastard Sword",
        damage_expr: "d8p+d10p",
        speed: 12.0,
        armor_pen: 0,
        size: WeaponSize::Medium,
        category: WeaponCategory::LargeSwords,
    },
    WeaponSpec {
        name: "Claymore",
        damage_expr: "2d10p+3",
        speed: 13.0,
        armor_pen: 1,
        size: WeaponSize::Large,
        category: WeaponCategory::LargeSwords,
    },
    WeaponSpec {
        name: "Flamberge",
        damage_expr: "6d3p+3",
        speed: 16.0,
        armor_pen: 2,
        size: WeaponSize::Large,
        category: WeaponCategory::LargeSwords,
    },
    WeaponSpec {
        name: "Greatsword",
        damage_expr: "d10p+d12p+3",
        speed: 14.0,
        armor_pen: 2,
        size: WeaponSize::Large,
        category: WeaponCategory::LargeSwords,
    },
    WeaponSpec {
        name: "Longsword",
        damage_expr: "2d8p",
        speed: 10.0,
        armor_pen: 0,
        size: WeaponSize::Medium,
        category: WeaponCategory::LargeSwords,
    },
    WeaponSpec {
        name: "Greatknife",
        damage_expr: "3d6p+3",
        speed: 12.0,
        armor_pen: 1,
        size: WeaponSize::Large,
        category: WeaponCategory::LargeSwords,
    },
    WeaponSpec {
        name: "Sabre",
        damage_expr: "d6p+d8p",
        speed: 8.0,
        armor_pen: 0,
        size: WeaponSize::Medium,
        category: WeaponCategory::LargeSwords,
    },
    WeaponSpec {
        name: "Scimitar",
        damage_expr: "2d8p",
        speed: 9.0,
        armor_pen: 0,
        size: WeaponSize::Medium,
        category: WeaponCategory::LargeSwords,
    },
    WeaponSpec {
        name: "Spatha",
        damage_expr: "d6p+d8p",
        speed: 9.0,
        armor_pen: 0,
        size: WeaponSize::Medium,
        category: WeaponCategory::LargeSwords,
    },
    WeaponSpec {
        name: "Thrusting Sword",
        damage_expr: "3d4p+3",
        speed: 9.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::LargeSwords,
    },
    WeaponSpec {
        name: "Two-Handed Sword",
        damage_expr: "2d12p+3",
        speed: 16.0,
        armor_pen: 2,
        size: WeaponSize::Large,
        category: WeaponCategory::LargeSwords,
    },
    // Small Swords
    WeaponSpec {
        name: "Dagger",
        damage_expr: "2d4p",
        speed: 7.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::SmallSwords,
    },
    WeaponSpec {
        name: "Dueling Sword",
        damage_expr: "3d4p",
        speed: 7.0,
        armor_pen: 0,
        size: WeaponSize::Medium,
        category: WeaponCategory::SmallSwords,
    },
    WeaponSpec {
        name: "Falx",
        damage_expr: "2d3p+d6p",
        speed: 9.0,
        armor_pen: 0,
        size: WeaponSize::Medium,
        category: WeaponCategory::SmallSwords,
    },
    WeaponSpec {
        name: "Knife",
        damage_expr: "d6p",
        speed: 7.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::SmallSwords,
    },
    WeaponSpec {
        name: "Gladius",
        damage_expr: "d4p+d8p",
        speed: 9.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::SmallSwords,
    },
    WeaponSpec {
        name: "Long Knife",
        damage_expr: "1d10p",
        speed: 6.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::SmallSwords,
    },
    WeaponSpec {
        name: "Short Sword",
        damage_expr: "2d6p",
        speed: 8.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::SmallSwords,
    },
    WeaponSpec {
        name: "Throwing Knife",
        damage_expr: "d6p",
        speed: 6.0,
        armor_pen: 0,
        size: WeaponSize::Small,
        category: WeaponCategory::SmallSwords,
    },
    // Polearms
    WeaponSpec {
        name: "Bardiche",
        damage_expr: "4d4p+3",
        speed: 14.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Polearms,
    },
    WeaponSpec {
        name: "Fauchard",
        damage_expr: "2d6p+3",
        speed: 13.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Polearms,
    },
    WeaponSpec {
        name: "Glaive",
        damage_expr: "5d4p+3",
        speed: 14.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Polearms,
    },
    WeaponSpec {
        name: "Guisarme",
        damage_expr: "2d6p+3",
        speed: 13.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Polearms,
    },
    WeaponSpec {
        name: "Halberd",
        damage_expr: "2d10p+3",
        speed: 14.0,
        armor_pen: 2,
        size: WeaponSize::Large,
        category: WeaponCategory::Polearms,
    },
    WeaponSpec {
        name: "Poleaxe",
        damage_expr: "3d6p+3",
        speed: 13.0,
        armor_pen: 2,
        size: WeaponSize::Large,
        category: WeaponCategory::Polearms,
    },
    WeaponSpec {
        name: "Polehammer",
        damage_expr: "d10p+d12p+3",
        speed: 15.0,
        armor_pen: 2,
        size: WeaponSize::Large,
        category: WeaponCategory::Polearms,
    },
    WeaponSpec {
        name: "Raven's Beak",
        damage_expr: "2d6p+3",
        speed: 14.0,
        armor_pen: 2,
        size: WeaponSize::Large,
        category: WeaponCategory::Polearms,
    },
    WeaponSpec {
        name: "Swordstaff",
        damage_expr: "2d8p+3",
        speed: 11.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Polearms,
    },
    WeaponSpec {
        name: "Voulge",
        damage_expr: "4d4p+3",
        speed: 15.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Polearms,
    },
    // Spears
    WeaponSpec {
        name: "Hasta",
        damage_expr: "2d6p",
        speed: 12.0,
        armor_pen: 0,
        size: WeaponSize::Medium,
        category: WeaponCategory::Spears,
    },
    WeaponSpec {
        name: "Javelin",
        damage_expr: "d12p",
        speed: 7.0,
        armor_pen: 0,
        size: WeaponSize::Medium,
        category: WeaponCategory::Spears,
    },
    WeaponSpec {
        name: "Lance",
        damage_expr: "2d8p",
        speed: 12.0,
        armor_pen: 2,
        size: WeaponSize::Large,
        category: WeaponCategory::Spears,
    },
    WeaponSpec {
        name: "Partisan",
        damage_expr: "2d8p+3",
        speed: 14.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Spears,
    },
    WeaponSpec {
        name: "Pike",
        damage_expr: "2d6p+3",
        speed: 18.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Spears,
    },
    WeaponSpec {
        name: "Pilum",
        damage_expr: "2d6p",
        speed: 8.0,
        armor_pen: 0,
        size: WeaponSize::Medium,
        category: WeaponCategory::Spears,
    },
    WeaponSpec {
        name: "Ranseur",
        damage_expr: "2d6p+3",
        speed: 13.0,
        armor_pen: 3,
        size: WeaponSize::Large,
        category: WeaponCategory::Spears,
    },
    WeaponSpec {
        name: "Short Spear",
        damage_expr: "d4p+d6p",
        speed: 12.0,
        armor_pen: 0,
        size: WeaponSize::Medium,
        category: WeaponCategory::Spears,
    },
    WeaponSpec {
        name: "Spear",
        damage_expr: "2d6p",
        speed: 12.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Spears,
    },
    WeaponSpec {
        name: "Spetum",
        damage_expr: "2d8p+3",
        speed: 13.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Spears,
    },
    WeaponSpec {
        name: "Trident",
        damage_expr: "d6p+d8p+3",
        speed: 12.0,
        armor_pen: 0,
        size: WeaponSize::Large,
        category: WeaponCategory::Spears,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-6;

    #[test]
    fn expected_damage_penetrating_die() {
        let avg = expected_damage("d6p");
        assert!((avg - 4.0).abs() < EPS);
    }
    #[test]
    fn expected_damage_complex_expression() {
        let avg = expected_damage("d8p+2d10p+3");
        assert!((avg - 20.0).abs() < EPS);
    }

    #[test]
    fn expected_damage_with_parentheses() {
        let avg = expected_damage("(d4p-2)+(d4p-2)");
        assert!((avg - 2.0).abs() < EPS);
    }

    #[test]
    fn armor_penetration_reduces_effective_armor() {
        let weapon = WeaponSpec {
            name: "Test Warhammer",
            damage_expr: "d8p+d10p",
            speed: 12.0,
            armor_pen: 1,
            size: WeaponSize::Medium,
            category: WeaponCategory::Blunt,
        };
        let armor_values = vec![7.0];
        let (_, values, _) = compute_weapon_curve(
            &weapon,
            &armor_values,
            GlobalAdjustments::default(),
            DEFAULT_SIM_DURATION,
        );
        let avg = expected_damage(weapon.damage_expr);
        let effective = effective_armor_value(armor_values[0], weapon.armor_pen);
        let net = (avg - effective).max(0.0);
        let expected = average_damage_per_second(net, weapon.speed, DEFAULT_SIM_DURATION);
        assert!((values[0] - expected).abs() < EPS);
    }

    #[test]
    fn armor_penetration_does_not_increase_damage_past_zero_armor() {
        let weapon = WeaponSpec {
            name: "Piercing Club",
            damage_expr: "d6p",
            speed: 10.0,
            armor_pen: 3,
            size: WeaponSize::Medium,
            category: WeaponCategory::Basic,
        };
        let armor_values = vec![1.0];
        let (_, values, _) = compute_weapon_curve(
            &weapon,
            &armor_values,
            GlobalAdjustments::default(),
            DEFAULT_SIM_DURATION,
        );
        let avg = expected_damage(weapon.damage_expr);
        let net =
            (avg - effective_armor_value(armor_values[0], weapon.armor_pen)).max(0.0);
        let expected = average_damage_per_second(net, weapon.speed, DEFAULT_SIM_DURATION);
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
        for weapon in WEAPONS {
            let adjusted = adjustments.adjusted_speed(weapon);
            assert!(
                adjusted >= weapon.size.min_speed() - EPS,
                "Weapon {} dropped below its floor",
                weapon.name
            );
        }
    }

    #[test]
    fn two_handed_eligibility_matches_rules() {
        for weapon in WEAPONS {
            let expected = if weapon.name == "Spear" {
                true
            } else {
                weapon.size == WeaponSize::Medium && weapon.category != WeaponCategory::Bows
            };
            assert_eq!(
                weapon_allows_two_handed_mode(weapon),
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
    let datasets = build_datasets(adjustments, sim_duration);
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
    let ui_adjustments = adjustments;
    match eframe::run_native(
        "Hackmaster Blunt Weapon Damage per Speed",
        native_options,
        Box::new(move |_| {
            Ok(Box::new(WeaponPlotApp::with_datasets(
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
