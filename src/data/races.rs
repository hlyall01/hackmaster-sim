use crate::core::types::RaceSpec;

#[derive(serde::Deserialize)]
struct RacesFile {
    races: Vec<RaceSpec>,
}

pub fn load_races(path: &str) -> Result<Vec<RaceSpec>, String> {
    let data = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
    let parsed: RacesFile = serde_json::from_str(&data).map_err(|err| err.to_string())?;
    Ok(parsed.races)
}
