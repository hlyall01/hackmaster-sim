use crate::character::{Material, MaterialKind};
use crate::data::resolve_data_path;
use serde::Deserialize;
use std::fs;

const EMBEDDED_MATERIALS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/sim/materials.json"
));

#[derive(Deserialize)]
struct MaterialsFile {
    metals: Vec<MaterialJson>,
    fabrics: Vec<MaterialJson>,
    woods: Vec<MaterialJson>,
}

#[derive(Deserialize)]
struct MaterialJson {
    tier: i32,
    name: String,
    weight_multiplier: f32,
}

pub fn load_materials(path: &str) -> Result<Vec<Material>, String> {
    let data = fs::read_to_string(resolve_data_path(path))
        .unwrap_or_else(|_| EMBEDDED_MATERIALS_JSON.to_string());
    let parsed: MaterialsFile = serde_json::from_str(&data).map_err(|err| err.to_string())?;
    let mut materials = Vec::new();
    materials.extend(parsed.metals.into_iter().map(|entry| Material {
        tier: entry.tier,
        name: entry.name,
        weight_mult: entry.weight_multiplier,
        kind: MaterialKind::Metal,
    }));
    materials.extend(parsed.fabrics.into_iter().map(|entry| Material {
        tier: entry.tier,
        name: entry.name,
        weight_mult: entry.weight_multiplier,
        kind: MaterialKind::Fabric,
    }));
    materials.extend(parsed.woods.into_iter().map(|entry| Material {
        tier: entry.tier,
        name: entry.name,
        weight_mult: entry.weight_multiplier,
        kind: MaterialKind::Wood,
    }));
    Ok(materials)
}
