use crate::core::tactics::{
    TacticalPolicy, TacticalPreset, valid_style_selection_shape, validate_policy,
};
use crate::data::{ensure_parent_dir, resolve_data_path, resolve_writable_data_path};
use serde::{Deserialize, Serialize};
use std::fs;

pub const TACTICAL_PRESET_SCHEMA_VERSION: u32 = 1;

const EMBEDDED_TACTICAL_PRESETS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/sim/tactical_presets.json"
));

#[derive(Clone, Debug, Deserialize, Serialize)]
struct TacticalPresetsFile {
    schema_version: u32,
    presets: Vec<TacticalPreset>,
}

pub fn load_tactical_presets(path: &str) -> Result<Vec<TacticalPreset>, String> {
    let data = fs::read_to_string(resolve_data_path(path))
        .unwrap_or_else(|_| EMBEDDED_TACTICAL_PRESETS_JSON.to_string());
    let parsed: TacticalPresetsFile =
        serde_json::from_str(&data).map_err(|err| format!("Invalid tactical presets: {err}"))?;
    if parsed.schema_version != TACTICAL_PRESET_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported tactical preset schema version {}; expected {}.",
            parsed.schema_version, TACTICAL_PRESET_SCHEMA_VERSION
        ));
    }
    let mut names = std::collections::HashSet::new();
    for preset in &parsed.presets {
        if !names.insert(preset.name.to_ascii_lowercase()) {
            return Err(format!("Duplicate tactical preset name '{}'.", preset.name));
        }
        if let Some(opening_style_ids) = &preset.opening_style_ids
            && !opening_style_ids.is_empty()
            && !valid_style_selection_shape(opening_style_ids)
        {
            return Err(format!(
                "Invalid opening style selection in tactical preset '{}'.",
                preset.name
            ));
        }
        if let Err(errors) = validate_policy(&TacticalPolicy {
            enabled: true,
            rules: preset.rules.clone(),
        }) {
            return Err(format!(
                "Invalid tactical preset '{}': {}",
                preset.name,
                errors.join(" ")
            ));
        }
    }
    Ok(parsed.presets)
}

pub fn save_tactical_presets(path: &str, presets: &[TacticalPreset]) -> Result<(), String> {
    let mut names = std::collections::HashSet::new();
    for preset in presets {
        if preset.name.trim().is_empty() {
            return Err("Tactical preset names cannot be empty.".to_string());
        }
        if !names.insert(preset.name.to_ascii_lowercase()) {
            return Err(format!("Duplicate tactical preset name '{}'.", preset.name));
        }
        if let Some(opening_style_ids) = &preset.opening_style_ids
            && !opening_style_ids.is_empty()
            && !valid_style_selection_shape(opening_style_ids)
        {
            return Err(format!(
                "Invalid opening style selection in tactical preset '{}'.",
                preset.name
            ));
        }
        if let Err(errors) = validate_policy(&TacticalPolicy {
            enabled: true,
            rules: preset.rules.clone(),
        }) {
            return Err(format!(
                "Invalid tactical preset '{}': {}",
                preset.name,
                errors.join(" ")
            ));
        }
    }
    let data = serde_json::to_string_pretty(&TacticalPresetsFile {
        schema_version: TACTICAL_PRESET_SCHEMA_VERSION,
        presets: presets.to_vec(),
    })
    .map_err(|err| err.to_string())?;
    let output_path = resolve_writable_data_path(path);
    ensure_parent_dir(&output_path)?;
    fs::write(output_path, data).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_file_contains_three_presets() {
        let parsed: TacticalPresetsFile =
            serde_json::from_str(EMBEDDED_TACTICAL_PRESETS_JSON).expect("valid presets");
        assert_eq!(parsed.schema_version, TACTICAL_PRESET_SCHEMA_VERSION);
        assert_eq!(parsed.presets.len(), 3);
        let mut names = parsed
            .presets
            .iter()
            .map(|preset| preset.name.to_ascii_lowercase())
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 3, "preset names must be unique");
        let arthur = parsed
            .presets
            .iter()
            .find(|preset| preset.name == "Arthur - Armeroci Bridge")
            .expect("Arthur tactical preset");
        assert_eq!(
            arthur.opening_style_ids,
            Some(vec!["armeroci_pole".to_string()])
        );
        let paths_of_the_sun = parsed
            .presets
            .iter()
            .find(|preset| preset.name == "12 Paths of the Sun")
            .expect("12 Paths of the Sun tactical preset");
        assert_eq!(
            paths_of_the_sun.opening_style_ids,
            Some(vec!["twelve_paths".to_string()])
        );
        let policy = TacticalPolicy {
            enabled: true,
            rules: paths_of_the_sun.rules.clone(),
        };
        let mut context = crate::core::tactics::TacticalContext {
            my_has_active_shield: false,
            my_active_style_ids: vec!["twelve_paths".to_string()],
            available_style_ids: vec!["falling_sun".to_string()],
            ..Default::default()
        };
        let decision = crate::core::tactics::evaluate_channel(
            &policy,
            crate::core::tactics::TacticalDecisionPoint::NextAttackOpportunity,
            crate::core::tactics::TacticalChannel::WeaponStyle,
            &context,
        );
        assert_eq!(
            decision.action,
            crate::core::tactics::TacticalAction::UseWeaponStyle {
                style_ids: vec!["falling_sun".to_string()]
            }
        );

        context.available_style_ids.clear();
        let decision = crate::core::tactics::evaluate_channel(
            &policy,
            crate::core::tactics::TacticalDecisionPoint::NextAttackOpportunity,
            crate::core::tactics::TacticalChannel::WeaponStyle,
            &context,
        );
        assert_eq!(
            decision.action,
            crate::core::tactics::TacticalAction::RetainWeaponStyle
        );
        for preset in parsed.presets {
            crate::core::tactics::validate_policy(&TacticalPolicy {
                enabled: true,
                rules: preset.rules,
            })
            .unwrap_or_else(|errors| panic!("preset '{}' is invalid: {errors:?}", preset.name));
        }
    }
}
