use crate::data::resolve_data_path;
use crate::game_logic::TalentCatalog;
use serde::Deserialize;
use std::fs;

const EMBEDDED_TALENTS_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/sim/talents.json"));

#[derive(Deserialize)]
struct TalentsFile {
    talents: Vec<crate::core::types::TalentSpec>,
}

pub fn load_talents(path: &str) -> Result<TalentCatalog, String> {
    let data = fs::read_to_string(resolve_data_path(path))
        .unwrap_or_else(|_| EMBEDDED_TALENTS_JSON.to_string());
    let parsed: TalentsFile = serde_json::from_str(&data).map_err(|err| err.to_string())?;
    Ok(TalentCatalog::new(parsed.talents))
}
