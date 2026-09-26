use crate::data::{preserve_invalid_presets, read_presets, recover_presets, write_presets};
use crate::game_logic::{FighterPreset, FighterPresetCatalog};
use serde::{Deserialize, Serialize};

const EMBEDDED_FIGHTER_PRESETS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/sim/fighter_presets.json"
));

#[derive(Deserialize, Serialize)]
struct FighterPresetsFile {
    presets: Vec<FighterPreset>,
}

pub fn load_fighter_presets(path: &str) -> Result<FighterPresetCatalog, String> {
    let data = read_presets(path, EMBEDDED_FIGHTER_PRESETS_JSON)?;
    parse_fighter_presets(&data)
}

pub fn load_fighter_presets_recovering(path: &str) -> (FighterPresetCatalog, Option<String>) {
    recover_presets(path, EMBEDDED_FIGHTER_PRESETS_JSON, parse_fighter_presets)
}

fn parse_fighter_presets(data: &str) -> Result<FighterPresetCatalog, String> {
    let parsed: FighterPresetsFile = serde_json::from_str(data).map_err(|err| err.to_string())?;
    Ok(FighterPresetCatalog::new(parsed.presets))
}

pub fn save_fighter_presets(path: &str, presets: &FighterPresetCatalog) -> Result<(), String> {
    let data = serde_json::to_string_pretty(&FighterPresetsFile {
        presets: presets.entries().to_vec(),
    })
    .map_err(|err| err.to_string())?;
    preserve_invalid_presets(path, parse_fighter_presets)?;
    write_presets(path, &data)
}
