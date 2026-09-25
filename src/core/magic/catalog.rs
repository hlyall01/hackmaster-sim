//! Typed, immutable spell data shared by every combat host and editor.
//! The JSON is bundled at build time; loading it performs no filesystem I/O.

use super::{MagicLoadout, SpellCastAi, SpellComponents};
use serde::Deserialize;
use std::sync::LazyLock;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellKind {
    #[default]
    Configured,
    EchoStrike,
    Chronoblur,
    Streamline,
}

impl SpellKind {
    pub fn id(self) -> &'static str {
        match self {
            Self::Configured => "",
            Self::EchoStrike => "echo_strike",
            Self::Chronoblur => "spell_chronoblur",
            Self::Streamline => "spell_streamline",
        }
    }

    pub fn catalog_entry(self) -> &'static SpellCatalogEntry {
        spell_catalog()
            .iter()
            .find(|spell| spell.id == self.id())
            .expect("validated spell catalog")
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
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

/// Generic catalog key. Only the legacy fields need compatibility adapters.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct SpellNumberField(pub String);

impl SpellNumberField {
    pub fn value_mut<'a>(&self, loadout: &'a mut MagicLoadout) -> &'a mut u32 {
        match self.0.as_str() {
            "echo_duration_seconds" => &mut loadout.echo_strike.extra_duration_seconds,
            "echo_delay_seconds" => &mut loadout.echo_strike.delay_seconds,
            "echo_additional_echoes" => &mut loadout.echo_strike.additional_echoes,
            "chronoblur_duration_ranks" => &mut loadout.chronoblur_duration_ranks,
            "streamline_duration_ranks" => &mut loadout.streamline_duration_ranks,
            "streamline_radius_ranks" => &mut loadout.streamline_radius_ranks,
            _ => loadout.spell_parameters.entry(self.0.clone()).or_default(),
        }
    }
    fn legacy_owner(&self) -> Option<&'static str> {
        match self.0.as_str() {
            "echo_duration_seconds" | "echo_delay_seconds" | "echo_additional_echoes" => {
                Some("echo_strike")
            }
            "chronoblur_duration_ranks" => Some("spell_chronoblur"),
            "streamline_duration_ranks" | "streamline_radius_ranks" => Some("spell_streamline"),
            _ => None,
        }
    }
    pub fn value(&self, loadout: &MagicLoadout) -> u32 {
        match self.0.as_str() {
            "echo_duration_seconds" => loadout.echo_strike.extra_duration_seconds,
            "echo_delay_seconds" => loadout.echo_strike.delay_seconds,
            "echo_additional_echoes" => loadout.echo_strike.additional_echoes,
            "chronoblur_duration_ranks" => loadout.chronoblur_duration_ranks,
            "streamline_duration_ranks" => loadout.streamline_duration_ranks,
            "streamline_radius_ranks" => loadout.streamline_radius_ranks,
            _ => loadout.spell_parameters.get(&self.0).copied().unwrap_or(0),
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

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum SpellEmpowerment {
    Number {
        field: SpellNumberField,
        label: String,
        min: u32,
        max: u32,
        suffix: String,
        #[serde(default)]
        cost_per_rank: u32,
        #[serde(default)]
        changes: Vec<super::EffectScaling>,
    },
    Toggle {
        field: SpellToggleField,
        label: String,
    },
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EchoStatusText {
    pub armed: String,
    pub pending: String,
    pub dismiss: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpellCatalogEntry {
    pub id: String,
    #[serde(default)]
    pub kind: SpellKind,
    pub mechanics: Option<super::EffectDefinition>,
    pub echo_rules: Option<super::EchoRules>,
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

impl SpellCatalogEntry {
    /// Resolve catalog parameters once for both runtime and editor consumers.
    pub fn resolve_values(
        &self,
        loadout: &MagicLoadout,
    ) -> Result<(super::EffectValues, u32), super::MagicError> {
        use super::MagicError;
        let definition = self.mechanics.as_ref().ok_or(MagicError::InvalidSpell)?;
        definition
            .validate()
            .map_err(|_| MagicError::InvalidSpell)?;
        let mut values = definition.values.clone();
        let mut cost = self.base_cost;
        for empowerment in &self.empowerments {
            if let SpellEmpowerment::Number {
                field,
                min,
                max,
                cost_per_rank,
                changes,
                ..
            } = empowerment
            {
                let rank = field.value(loadout);
                if rank < *min || rank > *max {
                    return Err(MagicError::InvalidSpell);
                }
                cost = cost
                    .checked_add(
                        rank.checked_mul(*cost_per_rank)
                            .ok_or(MagicError::CostOverflow)?,
                    )
                    .ok_or(MagicError::CostOverflow)?;
                for scaling in changes {
                    values.apply(scaling, rank)?;
                }
            }
        }
        definition
            .validate_values(&values)
            .map_err(|_| MagicError::InvalidSpell)?;
        Ok((values, cost))
    }
}

pub fn parse_spell_catalog(json: &str) -> Result<Vec<SpellCatalogEntry>, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct CatalogFile {
        spells: Vec<SpellCatalogEntry>,
    }
    let file: CatalogFile = serde_json::from_str(json).map_err(|error| error.to_string())?;
    let mut parameter_keys = std::collections::HashSet::new();
    for (index, spell) in file.spells.iter().enumerate() {
        if spell.id.trim().is_empty()
            || (spell.kind != SpellKind::Configured && spell.id != spell.kind.id())
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
        if spell.kind == SpellKind::EchoStrike {
            let Some(rule) = &spell.echo_rules else {
                return Err("Missing echo rules".into());
            };
            if rule.base_duration == 0
                || rule.minimum_delay == 0
                || rule.default_delay < rule.minimum_delay
                || rule.normal_damage_divisor <= 0
                || rule.defense_per_echo < 0
            {
                return Err("Invalid echo rules".into());
            }
        } else if spell.echo_rules.is_some() {
            return Err("Echo rules require the legacy echo hook".into());
        }
        if spell.level == 0 || spell.base_cost == 0 {
            return Err(format!("Invalid casting metadata: {}", spell.id));
        }
        match (&spell.echo_status, spell.kind) {
            (Some(status), SpellKind::EchoStrike)
                if [&status.armed, &status.pending, &status.dismiss]
                    .iter()
                    .all(|text| !text.trim().is_empty()) => {}
            (None, kind) if kind != SpellKind::EchoStrike => {}
            _ => return Err(format!("Invalid echo status text: {}", spell.id)),
        }
        let allowed = match spell.kind {
            SpellKind::EchoStrike => &["{duration}", "{echoes}", "{damage}"][..],
            _ => &[
                "{duration}",
                "{range}",
                "{radius}",
                "{dice}",
                "{projectiles}",
                "{force}",
                "{width}",
                "{speed}",
            ][..],
        };
        let mut summary = spell.summary.clone();
        for token in allowed {
            summary = summary.replace(token, "");
        }
        if summary.contains(['{', '}']) {
            return Err(format!("Unknown summary placeholder: {}", spell.id));
        }
        if let Some(mechanics) = &spell.mechanics {
            mechanics
                .validate()
                .map_err(|e| format!("{}: {e}", spell.id))?;
            for bound in [false, true] {
                let mut values = mechanics.values.clone();
                let mut cost = spell.base_cost;
                for empowerment in &spell.empowerments {
                    if let SpellEmpowerment::Number {
                        min,
                        max,
                        cost_per_rank,
                        changes,
                        ..
                    } = empowerment
                    {
                        let rank = if bound { *max } else { *min };
                        cost = cost
                            .checked_add(
                                rank.checked_mul(*cost_per_rank)
                                    .ok_or("Empowerment cost overflow")?,
                            )
                            .ok_or("Empowerment cost overflow")?;
                        for scaling in changes {
                            values.apply(scaling, rank).map_err(|e| e.to_string())?;
                        }
                    }
                }
                mechanics.validate_values(&values)?;
            }
        } else if spell.kind != SpellKind::EchoStrike {
            return Err(format!("Missing mechanics for {}", spell.id));
        }
        for empowerment in &spell.empowerments {
            if let SpellEmpowerment::Number { field, changes, .. } = empowerment {
                if !parameter_keys.insert(&field.0)
                    || (spell.mechanics.is_some() && changes.is_empty())
                {
                    return Err(format!("Duplicate or inert empowerment for {}", spell.id));
                }
            }
            let valid = match empowerment {
                SpellEmpowerment::Number {
                    field,
                    label,
                    min,
                    max,
                    ..
                } => {
                    !field.0.trim().is_empty()
                        && !label.trim().is_empty()
                        && min <= max
                        && field.legacy_owner().is_none_or(|id| id == spell.id)
                        && (spell.mechanics.is_none() || *min == 0)
                        && (spell.mechanics.is_some()
                            || match spell.kind {
                                SpellKind::EchoStrike => [
                                    "echo_duration_seconds",
                                    "echo_delay_seconds",
                                    "echo_additional_echoes",
                                ]
                                .contains(&field.0.as_str()),
                                SpellKind::Chronoblur => field.0 == "chronoblur_duration_ranks",
                                SpellKind::Streamline => {
                                    ["streamline_duration_ranks", "streamline_radius_ranks"]
                                        .contains(&field.0.as_str())
                                }
                                _ => false,
                            })
                }
                SpellEmpowerment::Toggle { label, .. } => {
                    spell.kind == SpellKind::EchoStrike && !label.trim().is_empty()
                }
            };
            if !valid {
                return Err(format!("Invalid empowerment for {}", spell.id));
            }
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
        assert!(parse_spell_catalog(&invalid.to_string()).is_ok());
    }
}
