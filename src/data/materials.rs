use serde::Deserialize;
use std::fs;

const EMBEDDED_MATERIALS_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/materials.json"));

#[derive(Deserialize)]
#[allow(dead_code)]
struct MaterialsFile {
    metals: Vec<MaterialJson>,
    fabrics: Vec<MaterialJson>,
    woods: Vec<MaterialJson>,
}

#[derive(Deserialize)]
struct MaterialJson {
    #[allow(dead_code)]
    tier: i32,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    weight_multiplier: f32,
}

pub(super) fn load_materials(path: &str) -> Result<(), String> {
    let data = fs::read_to_string(path).unwrap_or_else(|_| EMBEDDED_MATERIALS_JSON.to_string());
    let _parsed: MaterialsFile = serde_json::from_str(&data).map_err(|err| err.to_string())?;
    Ok(())
}
