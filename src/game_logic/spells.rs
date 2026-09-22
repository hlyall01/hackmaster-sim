//! Spell presentation values and actions; the GUI only renders catalog fields.

use crate::core::magic::{MagicError, MagicLoadout, SpellCatalogEntry, SpellKind};
use crate::sim::{SimState, SpellEffect, SpellRequest};

#[derive(Clone, Debug)]
pub enum SpellAction {
    Cast(String),
    Cancel,
    DismissEcho,
    StopChanneling,
}

pub fn apply_spell_action(
    sim: &mut SimState,
    caster: usize,
    action: SpellAction,
) -> Result<(), MagicError> {
    match action {
        SpellAction::Cast(id) => {
            if id == SpellKind::EchoStrike.id() {
                return sim.cast_echo_strike(caster);
            }
            let actor = sim
                .combatants
                .get(caster)
                .ok_or(MagicError::Incapacitated)?;
            let request = SpellRequest::from_loadout(&id, &actor.magic.loadout)?;
            sim.cast_spell(caster, request)
        }
        SpellAction::Cancel => sim.cancel_spell(caster),
        SpellAction::DismissEcho => sim.dismiss_echo_strike(caster),
        SpellAction::StopChanneling => sim.stop_channeling(caster),
    }
}

/// Use the actual runtime request so the displayed duration matches the effect.
pub fn spell_editor_summary(
    spell: &SpellCatalogEntry,
    loadout: &MagicLoadout,
) -> Result<String, MagicError> {
    let request = SpellRequest::from_loadout(&spell.id, loadout)?;
    let duration = match request.effect {
        SpellEffect::EchoStrike(options) => options.duration_seconds()?.to_string(),
        SpellEffect::TimedBuff(buff) => buff.remaining_seconds.to_string(),
    };
    Ok(spell
        .summary
        .replace("{duration}", &duration)
        .replace("{radius}", &loadout.streamline_radius_feet().to_string())
        .replace(
            "{echoes}",
            &(u64::from(loadout.echo_strike.additional_echoes) + 1).to_string(),
        )
        .replace(
            "{damage}",
            if loadout.echo_strike.full_damage {
                &spell.full_damage_text
            } else {
                &spell.half_damage_text
            },
        ))
}

pub fn echo_status_lines(state: &crate::sim::MagicState, now: u32) -> Vec<String> {
    let text = SpellKind::EchoStrike
        .catalog_entry()
        .echo_status
        .as_ref()
        .expect("validated echo status");
    let mut lines = Vec::new();
    if let Some(buff) = &state.armed_echo {
        lines.push(text.armed.replace(
            "{seconds}",
            &buff.expires_at.saturating_sub(now).to_string(),
        ));
    }
    for echo in &state.echoes {
        lines.push(
            text.pending
                .replace("{ordinal}", &(u64::from(echo.ordinal) + 1).to_string())
                .replace("{seconds}", &echo.due_at.saturating_sub(now).to_string())
                .replace("{damage}", &echo.damage.to_string()),
        );
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_summaries_follow_empowered_runtime_values_and_json_templates() {
        let mut loadout = MagicLoadout::default();
        loadout.echo_strike.extra_duration_seconds = 5;
        loadout.echo_strike.additional_echoes = 2;
        loadout.echo_strike.full_damage = true;
        loadout.chronoblur_duration_ranks = 2;
        loadout.streamline_duration_ranks = 3;
        loadout.streamline_radius_ranks = 4;
        for (kind, expected) in [
            (
                SpellKind::EchoStrike,
                "Armed for 20s · 3 echo(s) · full damage",
            ),
            (SpellKind::Chronoblur, "Duration: 120s"),
            (SpellKind::Streamline, "Duration: 480s · Radius: 70 ft"),
        ] {
            assert_eq!(
                spell_editor_summary(kind.catalog_entry(), &loadout).unwrap(),
                expected
            );
        }
        let mut spell = SpellKind::Chronoblur.catalog_entry().clone();
        spell.summary = "Custom duration {duration}".into();
        assert_eq!(
            spell_editor_summary(&spell, &loadout).unwrap(),
            "Custom duration 120"
        );
        loadout.echo_strike.delay_seconds = 0;
        assert_eq!(
            spell_editor_summary(SpellKind::EchoStrike.catalog_entry(), &loadout),
            Err(MagicError::InvalidEchoDelay)
        );
    }
}
