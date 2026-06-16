use crate::character::WeaponGroup;
use crate::data::resolve_data_path;
use crate::game_logic::{
    ShieldCatalog, ShieldEntry, ShieldPreset, WeaponCatalog, WeaponHandedness, WeaponPreset,
    WeaponSize,
};
use serde::Deserialize;
use std::fs;

const EMBEDDED_WEAPONS_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/weapons.json"));

#[derive(Deserialize)]
struct WeaponsFile {
    weapons: Vec<WeaponJson>,
    shields: Vec<ShieldJson>,
}

#[derive(Deserialize)]
struct WeaponJson {
    name: String,
    group: String,
    str_required: Option<i32>,
    skill_level: String,
    speed: String,
    jab_speed: Option<String>,
    jab_special: Option<String>,
    damage: Option<String>,
    shield_damage: Option<String>,
    ammunition: Option<String>,
    range_bands_feet: Option<Vec<f32>>,
    armor_penetration: Option<i32>,
    defense_bonus_always: Option<bool>,
    defense: Option<String>,
    #[serde(rename = "reach_or_range")]
    reach_or_range: Option<String>,
    size: String,
    handedness: String,
    #[serde(rename = "type")]
    damage_type: String,
    weight_lbs: Option<f32>,
    dismount: Option<bool>,
    set_for_charge: Option<bool>,
    phalanx_rank: Option<String>,
}

#[derive(Deserialize)]
struct ShieldJson {
    name: String,
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
            group,
            str_required: entry.str_required,
            skill_level: entry.skill_level,
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
            defense_bonus_always: entry.defense_bonus_always.unwrap_or(false),
            defense: entry.defense.clone(),
            size,
            handedness,
            damage_type: entry.damage_type,
            ammunition: entry.ammunition.clone(),
            weight_lbs: entry.weight_lbs,
            dismount: entry.dismount,
            set_for_charge: entry.set_for_charge,
            phalanx_rank: entry.phalanx_rank.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn weapon<'a>(catalog: &'a WeaponCatalog, name: &str) -> &'a WeaponPreset {
        catalog
            .entries()
            .iter()
            .find(|weapon| weapon.name == name)
            .unwrap_or_else(|| panic!("missing weapon {name}"))
    }

    fn shield<'a>(catalog: &'a ShieldCatalog, name: &str) -> &'a ShieldPreset {
        catalog
            .entries()
            .iter()
            .find_map(|entry| entry.shield.as_ref().filter(|shield| shield.name == name))
            .unwrap_or_else(|| panic!("missing shield {name}"))
    }

    #[test]
    fn weapon_catalog_loads_updated_table_metadata() {
        let catalog = load_weapon_catalog("data/weapons.json").expect("weapon catalog");
        assert_eq!(catalog.len(), 94);

        let battle_axe = weapon(&catalog, "Battle axe");
        assert_eq!(battle_axe.str_required, Some(10));
        assert_eq!(battle_axe.skill_level, "low");
        assert_eq!(battle_axe.damage_expr, "4d3p");
        assert_eq!(battle_axe.shield_damage_expr.as_deref(), Some("3d3p"));
        assert_eq!(battle_axe.armor_pen, 2);
        assert_eq!(battle_axe.damage_type, "H");
        assert_eq!(battle_axe.weight_lbs, Some(3.5));

        let arbalest = weapon(&catalog, "Arbalest");
        assert_eq!(arbalest.speed_label, "40R");
        assert_eq!(arbalest.speed, 40.0);
        assert_eq!(arbalest.ammunition.as_deref(), Some("Heavy quarrel"));

        let club = weapon(&catalog, "Club");
        assert_eq!(club.defense.as_deref(), Some("d20p-4"));

        let staff = weapon(&catalog, "Staff");
        assert_eq!(staff.defense.as_deref(), Some("d20p"));
    }

    #[test]
    fn polearm_and_spear_rows_match_updated_table_values() {
        let catalog = load_weapon_catalog("data/weapons.json").expect("weapon catalog");

        let bardiche = weapon(&catalog, "Bardiche");
        assert_eq!(bardiche.damage_expr, "4d6p+3");
        assert_eq!(bardiche.shield_damage_expr.as_deref(), Some("2d6p+3"));
        assert_eq!(bardiche.dismount, None);
        assert_eq!(bardiche.set_for_charge, None);

        let hasta = weapon(&catalog, "Hasta");
        assert_eq!(hasta.speed_label, "11");
        assert_eq!(hasta.jab_speed_label.as_deref(), Some("8"));
        assert_eq!(hasta.reach_label, "8 feet");
        assert_eq!(hasta.phalanx_rank.as_deref(), Some("2nd"));

        let pike = weapon(&catalog, "Pike");
        assert_eq!(pike.phalanx_rank.as_deref(), Some("4th"));

        let trident = weapon(&catalog, "Trident");
        assert_eq!(trident.damage_expr, "d4p+d6p+d8p+3");
        assert_eq!(trident.shield_damage_expr.as_deref(), Some("d8p"));
        assert_eq!(trident.jab_special_expr.as_deref(), Some("d4+d6+d8+3"));

        let spear_axe = weapon(&catalog, "Spear-axe");
        assert_eq!(spear_axe.damage_expr, "2d6p and 4d3p");
        assert_eq!(
            spear_axe.shield_damage_expr.as_deref(),
            Some("d6p+3 and 3d3p")
        );
        // The current simulator models a single active damage head, so this field
        // tracks the first listed head's armor penetration.
        assert_eq!(spear_axe.armor_pen, 0);
        assert_eq!(spear_axe.jab_special_expr, None);
    }

    #[test]
    fn shield_catalog_parses_updated_table_values() {
        let catalog = load_shield_catalog("data/weapons.json").expect("shield catalog");

        let buckler = shield(&catalog, "Buckler");
        assert_eq!(buckler.defense_bonus, 2);
        assert_eq!(buckler.dr, 6);
        assert_eq!(buckler.cover_value, 20);
        assert_eq!(buckler.weight_lbs, 2.0);

        let medium_wood = shield(&catalog, "Medium wooden shield");
        assert_eq!(medium_wood.defense_bonus, 6);
        assert_eq!(medium_wood.dr, 4);
        assert_eq!(medium_wood.cover_value, 16);
        assert_eq!(medium_wood.weight_lbs, 6.0);

        let tower = shield(&catalog, "Tower shield");
        assert_eq!(tower.defense_bonus, 6);
        assert_eq!(tower.dr, 6);
        assert_eq!(tower.cover_value, 6);
        assert_eq!(tower.weight_lbs, 35.0);
    }
}
