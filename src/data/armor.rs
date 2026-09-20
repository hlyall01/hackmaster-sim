use crate::character::{Armor, ArmorRegion, ArmorType};
use crate::data::resolve_data_path;
use crate::game_logic::{ArmorCatalog, ArmorEntry};
use serde::Deserialize;
use std::fs;

const EMBEDDED_ARMOR_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/sim/armor.json"));

#[derive(Deserialize)]
struct ArmorFile {
    armor: Vec<ArmorJson>,
}

#[derive(Deserialize)]
struct ArmorJson {
    name: String,
    #[serde(default)]
    price_gp: Option<u32>,
    region: String,
    damage_reduction: i32,
    defense_adjustment: i32,
    initiative_modifier: i32,
    speed_modifier: i32,
    #[serde(rename = "type")]
    armor_type: String,
    weight_lbs: Option<f32>,
}

pub fn load_armor_catalog(path: &str) -> Result<ArmorCatalog, String> {
    let data = fs::read_to_string(resolve_data_path(path))
        .unwrap_or_else(|_| EMBEDDED_ARMOR_JSON.to_string());
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
        let label = format!("{} ({})", entry.name.as_str(), entry.region);
        let armor = Armor {
            name: entry.name,
            price_gp: entry.price_gp.unwrap_or(0),
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
    Ok(ArmorCatalog::new(catalog))
}

fn armor_region_from_str(region: &str) -> Option<ArmorRegion> {
    match region {
        "Northern" => Some(ArmorRegion::Northern),
        "Southern" => Some(ArmorRegion::Southern),
        _ => None,
    }
}

fn armor_type_from_str(kind: &str) -> Option<ArmorType> {
    match kind {
        "None" => Some(ArmorType::None),
        "Light" => Some(ArmorType::Light),
        "Medium" => Some(ArmorType::Medium),
        "Heavy" => Some(ArmorType::Heavy),
        _ => None,
    }
}
