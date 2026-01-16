use crate::data::{ensure_parent_dir, resolve_data_path, resolve_writable_data_path};
use crate::game_logic::{FighterPreset, FighterPresetCatalog};
use serde::{Deserialize, Serialize};
use std::fs;

const EMBEDDED_FIGHTER_PRESETS_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/fighter_presets.json"));

#[derive(Deserialize, Serialize)]
struct FighterPresetsFile {
    presets: Vec<FighterPreset>,
}

pub fn load_fighter_presets(path: &str) -> Result<FighterPresetCatalog, String> {
    let data = fs::read_to_string(resolve_data_path(path))
        .unwrap_or_else(|_| EMBEDDED_FIGHTER_PRESETS_JSON.to_string());
    let parsed: FighterPresetsFile = serde_json::from_str(&data).map_err(|err| err.to_string())?;
    Ok(FighterPresetCatalog::new(parsed.presets))
}

pub fn save_fighter_presets(path: &str, presets: &FighterPresetCatalog) -> Result<(), String> {
    let data = serde_json::to_string_pretty(&FighterPresetsFile {
        presets: presets.entries().to_vec(),
    })
    .map_err(|err| err.to_string())?;
    let output_path = resolve_writable_data_path(path);
    ensure_parent_dir(&output_path)?;
    fs::write(output_path, data).map_err(|err| err.to_string())
}
