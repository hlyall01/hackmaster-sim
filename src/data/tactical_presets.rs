use crate::core::tactics::{
    TacticalPolicy, TacticalPreset, valid_style_selection_shape, validate_policy,
};
use crate::data::{preserve_invalid_presets, read_presets, recover_presets, write_presets};
use serde::{Deserialize, Serialize};

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
    let data = read_presets(path, EMBEDDED_TACTICAL_PRESETS_JSON)?;
    parse_tactical_presets(&data)
}

pub fn load_tactical_presets_recovering(path: &str) -> (Vec<TacticalPreset>, Option<String>) {
    recover_presets(path, EMBEDDED_TACTICAL_PRESETS_JSON, parse_tactical_presets)
}

fn parse_tactical_presets(data: &str) -> Result<Vec<TacticalPreset>, String> {
    let parsed: TacticalPresetsFile =
        serde_json::from_str(data).map_err(|err| format!("Invalid tactical presets: {err}"))?;
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
    preserve_invalid_presets(path, parse_tactical_presets)?;
    write_presets(path, &data)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tactical_fixture_round_trip_preserves_conditional_style_switching() {
        let file = TacticalPresetsFile {
            schema_version: TACTICAL_PRESET_SCHEMA_VERSION,
            presets: crate::test_support::tactical_presets(),
        };
        let json = serde_json::to_string(&file).unwrap();
        let parsed: TacticalPresetsFile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.schema_version, TACTICAL_PRESET_SCHEMA_VERSION);
        assert_eq!(parsed.presets, file.presets);
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
