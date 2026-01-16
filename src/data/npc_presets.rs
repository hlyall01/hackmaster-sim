use crate::core::catalog::Catalog;
use crate::data::resolve_data_path;
use crate::game_logic::{NpcPreset, NpcPresetCatalog};
use serde::Deserialize;
use std::fs;

const EMBEDDED_NPC_PRESETS_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/npc_presets.json"));

#[derive(Deserialize)]
struct NpcPresetsFile {
    presets: Vec<NpcPreset>,
}

pub fn load_npc_presets(path: &str) -> Result<NpcPresetCatalog, String> {
    let data = fs::read_to_string(resolve_data_path(path))
        .unwrap_or_else(|_| EMBEDDED_NPC_PRESETS_JSON.to_string());
    let parsed: NpcPresetsFile = serde_json::from_str(&data).map_err(|err| err.to_string())?;
    Ok(Catalog::new(parsed.presets))
}
