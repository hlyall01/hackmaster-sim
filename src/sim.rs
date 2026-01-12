pub use crate::core::sim::*;

pub fn format_combat_event(event: &CombatEvent, combatants: &[Combatant; 2]) -> String {
    let attacker_name = combatants
        .get(event.attacker_idx)
        .map(|combatant| combatant.sheet.name.as_str())
        .unwrap_or("Attacker");
    let defender_name = combatants
        .get(event.defender_idx)
        .map(|combatant| combatant.sheet.name.as_str())
        .unwrap_or("Defender");

    match &event.kind {
        CombatEventKind::Attack(attack) => {
            let weapon_name = combatants
                .get(event.attacker_idx)
                .map(|combatant| combatant.sheet.offense.weapon.name.as_str())
                .unwrap_or("Weapon");
            let base = if attack.hit {
                if attack.hold_at_bay {
                    if attack.damage > 0 {
                        format!(
                            "{attacker_name} holds {defender_name} at bay with {weapon_name} for {} dmg",
                            attack.damage
                        )
                    } else {
                        format!(
                            "{attacker_name} holds {defender_name} at bay with {weapon_name} (no damage)"
                        )
                    }
                } else {
                    format!(
                        "{attacker_name} hits {defender_name} with {weapon_name} for {} dmg",
                        attack.damage
                    )
                }
            } else if attack.shield_block {
                format!(
                    "{defender_name} blocks {attacker_name} with shield for {} shield dmg",
                    attack.shield_damage
                )
            } else if attack.hold_at_bay {
                format!(
                    "{attacker_name} fails to hold {defender_name} at bay with {weapon_name}"
                )
            } else {
                format!("{attacker_name} misses {defender_name} with {weapon_name}")
            };

            let mut extras = Vec::new();
            if attack.is_ranged {
                extras.push("ranged".to_string());
            }
            if attack.use_jab {
                extras.push("jab".to_string());
            }
            if attack.knockback_ft > 0.0 {
                extras.push(format!("knockback {:.0}ft", attack.knockback_ft));
            }
            if attack.trauma_applied {
                extras.push("trauma".to_string());
            }

            if extras.is_empty() {
                base
            } else {
                format!("{base} ({})", extras.join(", "))
            }
        }
        CombatEventKind::KnockAside(knock) => {
            if knock.success {
                format!("{attacker_name} knocks aside {defender_name}'s weapon")
            } else {
                format!("{attacker_name} fails to knock aside {defender_name}'s weapon")
            }
        }
    }
}

pub fn format_combat_event_line(event: &CombatEvent, combatants: &[Combatant; 2]) -> String {
    format!(
        "t={}s | {}",
        event.time,
        format_combat_event(event, combatants)
    )
}
