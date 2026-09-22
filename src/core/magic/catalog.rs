//! Typed, immutable spell data shared by every combat host and editor.
//! The JSON is bundled at build time; loading it performs no filesystem I/O.

use super::{MagicLoadout, SpellCastAi, SpellComponents};
use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellKind {
    EchoStrike,
    Chronoblur,
    Streamline,
}

impl SpellKind {
    pub fn id(self) -> &'static str {
        match self {
            Self::EchoStrike => "echo_strike",
            Self::Chronoblur => "spell_chronoblur",
            Self::Streamline => "spell_streamline",
        }
    }

    pub fn catalog_entry(self) -> &'static SpellCatalogEntry {
        spell_catalog()
            .iter()
            .find(|spell| spell.kind == self)
            .expect("validated spell catalog")
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellCastingHelp {
    pub use_once: String,
    pub manual: String,
    pub as_often_as_possible: String,
}

impl SpellCastingHelp {
    pub fn for_mode(&self, mode: SpellCastAi) -> &str {
        match mode {
            SpellCastAi::AtFightStart => &self.use_once,
            SpellCastAi::Manual => &self.manual,
            SpellCastAi::AsOftenAsPossible => &self.as_often_as_possible,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellNumberField {
    EchoDurationSeconds,
    EchoDelaySeconds,
    EchoAdditionalEchoes,
    ChronoblurDurationRanks,
    StreamlineDurationRanks,
    StreamlineRadiusRanks,
}

impl SpellNumberField {
    pub fn value_mut(self, loadout: &mut MagicLoadout) -> &mut u32 {
        match self {
            Self::EchoDurationSeconds => &mut loadout.echo_strike.extra_duration_seconds,
            Self::EchoDelaySeconds => &mut loadout.echo_strike.delay_seconds,
            Self::EchoAdditionalEchoes => &mut loadout.echo_strike.additional_echoes,
            Self::ChronoblurDurationRanks => &mut loadout.chronoblur_duration_ranks,
            Self::StreamlineDurationRanks => &mut loadout.streamline_duration_ranks,
            Self::StreamlineRadiusRanks => &mut loadout.streamline_radius_ranks,
        }
    }

    fn kind(self) -> SpellKind {
        match self {
            Self::EchoDurationSeconds | Self::EchoDelaySeconds | Self::EchoAdditionalEchoes => {
                SpellKind::EchoStrike
            }
            Self::ChronoblurDurationRanks => SpellKind::Chronoblur,
            Self::StreamlineDurationRanks | Self::StreamlineRadiusRanks => SpellKind::Streamline,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellToggleField {
    EchoFullDamage,
}

impl SpellToggleField {
    pub fn value_mut(self, loadout: &mut MagicLoadout) -> &mut bool {
        match self {
            Self::EchoFullDamage => &mut loadout.echo_strike.full_damage,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum SpellEmpowerment {
    Number {
        field: SpellNumberField,
        label: String,
        min: u32,
        max: u32,
        suffix: String,
    },
    Toggle {
        field: SpellToggleField,
        label: String,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EchoStatusText {
    pub armed: String,
    pub pending: String,
    pub dismiss: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellCatalogEntry {
    pub id: String,
    pub kind: SpellKind,
    pub name: String,
    pub level: u8,
    pub base_cost: u32,
    pub casting_seconds: u32,
    pub components: SpellComponents,
    pub description: String,
    pub casting_description: String,
    pub casting_help: SpellCastingHelp,
    pub empowerments: Vec<SpellEmpowerment>,
    pub summary: String,
    #[serde(default)]
    pub half_damage_text: String,
    #[serde(default)]
    pub full_damage_text: String,
    pub echo_status: Option<EchoStatusText>,
}

pub fn parse_spell_catalog(json: &str) -> Result<Vec<SpellCatalogEntry>, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct CatalogFile {
        spells: Vec<SpellCatalogEntry>,
    }
    let file: CatalogFile = serde_json::from_str(json).map_err(|error| error.to_string())?;
    for (index, spell) in file.spells.iter().enumerate() {
        if spell.id != spell.kind.id()
            || file.spells[..index]
                .iter()
                .any(|other| other.id == spell.id)
        {
            return Err(format!("Invalid or duplicate spell ID: {}", spell.id));
        }
        if [
            &spell.name,
            &spell.description,
            &spell.casting_description,
            &spell.summary,
            &spell.casting_help.use_once,
            &spell.casting_help.manual,
            &spell.casting_help.as_often_as_possible,
        ]
        .iter()
        .any(|text| text.trim().is_empty())
        {
            return Err(format!("Missing spell text: {}", spell.id));
        }
        if spell.kind == SpellKind::EchoStrike
            && (spell.half_damage_text.trim().is_empty()
                || spell.full_damage_text.trim().is_empty())
        {
            return Err("Missing Echo Strike damage text".into());
        }
        if spell.level == 0 || spell.base_cost == 0 {
            return Err(format!("Invalid casting metadata: {}", spell.id));
        }
        match (&spell.echo_status, spell.kind) {
            (Some(status), SpellKind::EchoStrike)
                if [&status.armed, &status.pending, &status.dismiss]
                    .iter()
                    .all(|text| !text.trim().is_empty()) => {}
            (None, SpellKind::Chronoblur | SpellKind::Streamline) => {}
            _ => return Err(format!("Invalid echo status text: {}", spell.id)),
        }
        let allowed = match spell.kind {
            SpellKind::EchoStrike => &["{duration}", "{echoes}", "{damage}"][..],
            SpellKind::Chronoblur => &["{duration}"][..],
            SpellKind::Streamline => &["{duration}", "{radius}"][..],
        };
        let mut summary = spell.summary.clone();
        for token in allowed {
            summary = summary.replace(token, "");
        }
        if summary.contains(['{', '}']) {
            return Err(format!("Unknown summary placeholder: {}", spell.id));
        }
        for empowerment in &spell.empowerments {
            let valid = match empowerment {
                SpellEmpowerment::Number {
                    field,
                    label,
                    min,
                    max,
                    ..
                } => field.kind() == spell.kind && !label.trim().is_empty() && min <= max,
                SpellEmpowerment::Toggle { label, .. } => {
                    spell.kind == SpellKind::EchoStrike && !label.trim().is_empty()
                }
            };
            if !valid {
                return Err(format!("Invalid empowerment for {}", spell.id));
            }
        }
    }
    for kind in [
        SpellKind::EchoStrike,
        SpellKind::Chronoblur,
        SpellKind::Streamline,
    ] {
        if !file.spells.iter().any(|spell| spell.kind == kind) {
            return Err(format!("Missing spell: {}", kind.id()));
        }
    }
    Ok(file.spells)
}

pub fn spell_catalog() -> &'static [SpellCatalogEntry] {
    static CATALOG: LazyLock<Vec<SpellCatalogEntry>> = LazyLock::new(|| {
        parse_spell_catalog(include_str!("../../../data/sim/spells.json"))
            .expect("bundled spell catalog must be valid")
    });
    &CATALOG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_owns_each_modes_help_and_spell_specific_editor_fields() {
        let mut json: serde_json::Value =
            serde_json::from_str(include_str!("../../../data/sim/spells.json")).unwrap();
        json["spells"][1]["casting_help"]["as_often_as_possible"] = "Custom Chronoblur help".into();
        let spells = parse_spell_catalog(&json.to_string()).unwrap();
        assert_eq!(
            spells[1]
                .casting_help
                .for_mode(SpellCastAi::AsOftenAsPossible),
            "Custom Chronoblur help"
        );
        assert!(
            spells[0]
                .casting_help
                .for_mode(SpellCastAi::AsOftenAsPossible)
                .contains("Pending echoes")
        );
        for spell in &spells[1..] {
            for mode in SpellCastAi::ALL {
                assert!(
                    !spell
                        .casting_help
                        .for_mode(mode)
                        .to_lowercase()
                        .contains("echo")
                );
            }
        }
    }

    #[test]
    fn rejects_incomplete_mismatched_and_ambiguous_spell_data() {
        let original: serde_json::Value =
            serde_json::from_str(include_str!("../../../data/sim/spells.json")).unwrap();
        let mut invalid = original.clone();
        invalid["spells"][1]["casting_help"]
            .as_object_mut()
            .unwrap()
            .remove("manual");
        assert!(parse_spell_catalog(&invalid.to_string()).is_err());
        invalid = original.clone();
        invalid["spells"][1]["empowerments"][0]["field"] = "echo_delay_seconds".into();
        assert!(parse_spell_catalog(&invalid.to_string()).is_err());
        invalid = original.clone();
        invalid["spells"][1]["summary"] = "Duration: {typo}".into();
        assert!(parse_spell_catalog(&invalid.to_string()).is_err());
        invalid = original.clone();
        invalid["spells"]
            .as_array_mut()
            .unwrap()
            .push(original["spells"][0].clone());
        assert!(parse_spell_catalog(&invalid.to_string()).is_err());
        invalid = original;
        invalid["spells"].as_array_mut().unwrap().remove(0);
        assert!(parse_spell_catalog(&invalid.to_string()).is_err());
    }
}
