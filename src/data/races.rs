use crate::core::types::RaceSpec;
use crate::data::resolve_data_path;
use std::fs;

const EMBEDDED_RACES_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/races.json"));

#[derive(serde::Deserialize)]
struct RacesFile {
    races: Vec<RaceSpec>,
}

pub fn load_races(path: &str) -> Result<Vec<RaceSpec>, String> {
    let data = fs::read_to_string(resolve_data_path(path))
        .unwrap_or_else(|_| EMBEDDED_RACES_JSON.to_string());
    let parsed: RacesFile = serde_json::from_str(&data).map_err(|err| err.to_string())?;
    Ok(parsed.races)
}
