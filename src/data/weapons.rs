use crate::character::WeaponGroup;
use crate::data::resolve_data_path;
use crate::game_logic::{
    ShieldCatalog, ShieldEntry, ShieldPreset, WeaponCatalog, WeaponHandedness, WeaponPreset,
    WeaponSize,
};
use serde::Deserialize;
use std::fs;

const EMBEDDED_WEAPONS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/sim/weapons.json"
));

#[derive(Deserialize)]
struct WeaponsFile {
    weapons: Vec<WeaponJson>,
    shields: Vec<ShieldJson>,
}

#[derive(Deserialize)]
struct WeaponJson {
    name: String,
    #[serde(default)]
    price_gp: Option<u32>,
    group: String,
    speed: String,
    jab_speed: Option<String>,
    jab_special: Option<String>,
    damage: Option<String>,
    shield_damage: Option<String>,
    ammunition: Option<String>,
    range_bands_feet: Option<Vec<f32>>,
    armor_penetration: Option<i32>,
    #[serde(rename = "type")]
    damage_type: Option<String>,
    defense_bonus_always: Option<bool>,
    #[serde(rename = "reach_or_range")]
    reach_or_range: Option<String>,
    size: String,
    handedness: String,
}

#[derive(Deserialize)]
struct ShieldJson {
    name: String,
    #[serde(default)]
    price_gp: Option<u32>,
    defense: String,
    damage_reduction: String,
    #[allow(dead_code)]
    arc_of_defense: String,
    cover_value: String,
    breakage_thresholds: Vec<i32>,
    weight_lbs: f32,
}

pub fn load_weapon_catalog(path: &str) -> Result<WeaponCatalog, String> {
    let data = fs::read_to_string(resolve_data_path(path))
        .unwrap_or_else(|_| EMBEDDED_WEAPONS_JSON.to_string());
    let parsed: WeaponsFile = serde_json::from_str(&data).map_err(|err| err.to_string())?;
    let mut catalog = Vec::new();
    for entry in parsed.weapons {
        let group = match weapon_group_from_str(&entry.group) {
            Some(group) => group,
            None => continue,
        };
        let size = match weapon_size_from_str(&entry.size) {
            Some(size) => size,
            None => continue,
        };
        let handedness = match weapon_handedness_from_str(&entry.handedness) {
            Some(handedness) => handedness,
            None => continue,
        };
        let (speed_label, jab_label) = split_speed_label(&entry.speed, entry.jab_speed.as_deref());
        let speed_value = parse_leading_number(&speed_label);
        let jab_speed_value = jab_label
            .as_deref()
            .map(parse_leading_number)
            .filter(|value| *value > 0.0);
        let reach_label = entry
            .reach_or_range
            .clone()
            .unwrap_or_else(|| "-".to_string());
        let reach_ft = parse_reach_ft(&reach_label);
        let damage_expr = entry.damage.unwrap_or_else(|| "-".to_string());
        let range_bands_feet = entry
            .range_bands_feet
            .as_deref()
            .and_then(parse_range_bands_feet);
        catalog.push(WeaponPreset {
            name: entry.name,
            price_gp: entry.price_gp.unwrap_or(0),
            group,
            speed: speed_value,
            speed_label,
            jab_speed: jab_speed_value,
            jab_speed_label: jab_label,
            jab_special_expr: entry.jab_special.clone(),
            damage_expr,
            shield_damage_expr: entry.shield_damage.clone(),
            reach_label,
            reach_ft,
            range_bands_feet,
            armor_pen: entry.armor_penetration.unwrap_or(0),
            hacking_or_piercing: entry
                .damage_type
                .as_deref()
                .map(is_hacking_or_piercing_type)
                .unwrap_or(false),
            defense_bonus_always: entry.defense_bonus_always.unwrap_or(false),
            size,
            handedness,
            ammunition: entry.ammunition.clone(),
        });
    }
    if catalog.is_empty() {
        Err("No weapons loaded from JSON".to_string())
    } else {
        Ok(WeaponCatalog::new(catalog))
    }
}

pub fn load_shield_catalog(path: &str) -> Result<ShieldCatalog, String> {
    let data = fs::read_to_string(resolve_data_path(path))
        .unwrap_or_else(|_| EMBEDDED_WEAPONS_JSON.to_string());
    let parsed: WeaponsFile = serde_json::from_str(&data).map_err(|err| err.to_string())?;
    let mut catalog = Vec::new();
    catalog.push(ShieldEntry {
        label: "None".to_string(),
        shield: None,
    });
    for entry in parsed.shields {
        let defense_bonus = parse_shield_defense_bonus(&entry.defense);
        let dr = parse_leading_number(&entry.damage_reduction) as i32;
        let cover_value = parse_cover_value(&entry.cover_value);
        let breakage_thresholds = parse_breakage_thresholds(&entry.breakage_thresholds)
            .map_err(|err| format!("shield {}: {err}", entry.name))?;
        let shield = ShieldPreset {
            name: entry.name.clone(),
            price_gp: entry.price_gp.unwrap_or(0),
            defense_bonus,
            dr,
            cover_value,
            breakage_thresholds,
            weight_lbs: entry.weight_lbs,
        };
        catalog.push(ShieldEntry {
            label: entry.name,
            shield: Some(shield),
        });
    }
    Ok(ShieldCatalog::new(catalog))
}

fn split_speed_label(speed: &str, jab_speed: Option<&str>) -> (String, Option<String>) {
    let trimmed = speed.trim();
    if let Some(jab_speed) = jab_speed {
        let jab_speed = jab_speed.trim();
        let speed = trimmed
            .split_once(',')
            .map(|pair| pair.0)
            .unwrap_or(trimmed);
        (speed.trim().to_string(), Some(jab_speed.to_string()))
    } else {
        let mut jab = None;
        let speed = if let Some((base, extra)) = trimmed.split_once(',') {
            let extra = extra.trim();
            if !extra.is_empty() {
                jab = Some(extra.to_string());
            }
            base
        } else {
            trimmed
        };
        (speed.trim().to_string(), jab)
    }
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

fn weapon_size_from_str(size: &str) -> Option<WeaponSize> {
    match size {
        "S" => Some(WeaponSize::Small),
        "M" => Some(WeaponSize::Medium),
        "L" => Some(WeaponSize::Large),
        _ => None,
    }
}

fn weapon_handedness_from_str(handedness: &str) -> Option<WeaponHandedness> {
    match handedness {
        "1h" => Some(WeaponHandedness::OneHanded),
        "2h" => Some(WeaponHandedness::TwoHanded),
        _ => None,
    }
}

fn is_hacking_or_piercing_type(damage_type: &str) -> bool {
    let normalized = damage_type.to_ascii_uppercase();
    normalized.contains('H') || normalized.contains('P')
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

fn parse_shield_defense_bonus(value: &str) -> i32 {
    if let Some(idx) = value.rfind('+') {
        return value[idx + 1..].trim().parse::<i32>().unwrap_or(0);
    }
    if let Some(idx) = value.rfind('-') {
        return value[idx..].trim().parse::<i32>().unwrap_or(0);
    }
    0
}

fn parse_cover_value(value: &str) -> i32 {
    parse_leading_number(value) as i32
}

fn parse_breakage_thresholds(values: &[i32]) -> Result<[i32; 4], String> {
    if values.len() != 4 {
        return Err(format!(
            "expected 4 breakage thresholds, got {}",
            values.len()
        ));
    }
    Ok([values[0], values[1], values[2], values[3]])
}

fn parse_reach_ft(value: &str) -> f32 {
    if value.contains('/') {
        return 0.0;
    }
    parse_leading_number(value)
}

fn parse_range_bands_feet(values: &[f32]) -> Option<[f32; 4]> {
    if values.len() != 4 {
        return None;
    }
    Some([values[0], values[1], values[2], values[3]])
}
