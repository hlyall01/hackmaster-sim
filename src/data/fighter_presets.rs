use crate::data::{preserve_invalid_presets, read_presets, recover_presets, write_presets};
use crate::game_logic::{FighterPreset, FighterPresetCatalog};
use serde::{Deserialize, Serialize};

const EMBEDDED_FIGHTER_PRESETS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/sim/fighter_presets.json"
));

const LEGACY_ARTHUR_LEVEL9_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/sim/legacy/arthur_level9.json"
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
    let mut parsed: FighterPresetsFile =
        serde_json::from_str(data).map_err(|err| err.to_string())?;
    if parsed
        .presets
        .iter()
        .any(|preset| preset.name == "Arthur Du Randt" && preset.level == 9)
    {
        let legacy: FighterPreset =
            serde_json::from_str(LEGACY_ARTHUR_LEVEL9_JSON).expect("valid legacy Arthur preset");
        let legacy = serde_json::to_value(legacy).expect("serializable legacy Arthur preset");
        let bundled: FighterPresetsFile = serde_json::from_str(EMBEDDED_FIGHTER_PRESETS_JSON)
            .expect("valid bundled fighter presets");
        let current = bundled
            .presets
            .into_iter()
            .find(|preset| preset.name == "Arthur Du Randt")
            .expect("bundled Arthur preset");
        for preset in &mut parsed.presets {
            // Compare every saved field, so personalized level-nine presets remain intact.
            if preset.name == "Arthur Du Randt"
                && serde_json::to_value(&*preset).ok().as_ref() == Some(&legacy)
            {
                *preset = current.clone();
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_saved_arthur_is_upgraded_without_changing_other_or_custom_presets() {
        let legacy: serde_json::Value = serde_json::from_str(LEGACY_ARTHUR_LEVEL9_JSON).unwrap();
        let mut custom_stats = legacy.clone();
        custom_stats["strength_base"] = 19.into();
        let mut custom_talent = legacy.clone();
        custom_talent["talents"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"id":"dodge","rank":1,"weapon":null}));
        let mut renamed = legacy.clone();
        renamed["name"] = "My Arthur".into();
        let input = serde_json::json!({"presets":[legacy, custom_stats, custom_talent, renamed]});
        let loaded = parse_fighter_presets(&input.to_string()).unwrap();
        let first = &loaded.entries()[0];
        assert_eq!(first.level, 10);
        assert_eq!(first.constitution, 16);
        assert_eq!(first.strength_pct, 75);
        assert_eq!(first.progression.health, "V");
        for index in 1..4 {
            let expected: FighterPreset =
                serde_json::from_value(input["presets"][index].clone()).unwrap();
            assert_eq!(
                serde_json::to_value(&loaded.entries()[index]).unwrap(),
                serde_json::to_value(expected).unwrap()
            );
        }
    }
}
