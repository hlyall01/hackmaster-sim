//! Fixed combat scenarios for tests, independent of user-editable preset files.
//! Change these only when deliberately changing a test scenario or a game rule.

use crate::core::tactics::TacticalPreset;
use crate::game_logic::{FighterPreset, FighterPresetCatalog};

pub(crate) fn fighter_presets() -> FighterPresetCatalog {
    let fighters: Vec<FighterPreset> =
        serde_json::from_str(include_str!("test_support/fighters.json"))
            .expect("valid test fighter fixtures");
    FighterPresetCatalog::new(fighters)
}

pub(crate) fn fighter(name: &str) -> FighterPreset {
    fighter_presets()
        .entries()
        .iter()
        .find(|fighter| fighter.name == name)
        .unwrap_or_else(|| panic!("missing test fighter {name}"))
        .clone()
}

pub(crate) fn tactical_presets() -> Vec<TacticalPreset> {
    #[derive(serde::Deserialize)]
    struct File {
        presets: Vec<TacticalPreset>,
    }
    serde_json::from_str::<File>(include_str!("test_support/tactics.json"))
        .expect("valid test tactical fixtures")
        .presets
}
