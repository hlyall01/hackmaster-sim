use crate::core::gameplay::AutobattlerConfig;
use crate::data::resolve_data_path;
use std::fs;

const EMBEDDED_AUTOBATTLER_CONFIG_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/autobattler_config.json"));

pub fn load_autobattler_config(path: &str) -> Result<AutobattlerConfig, String> {
    let data = fs::read_to_string(resolve_data_path(path))
        .unwrap_or_else(|_| EMBEDDED_AUTOBATTLER_CONFIG_JSON.to_string());
    let parsed: AutobattlerConfig = serde_json::from_str(&data).map_err(|err| err.to_string())?;
    Ok(parsed)
}
