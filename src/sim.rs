pub use crate::core::sim::*;

pub fn format_combat_event(event: &CombatEvent, combatants: &[Combatant]) -> String {
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
                .map(|combatant| match attack.weapon_slot {
                    WeaponSlot::Primary => combatant.sheet.offense.weapon.name.as_str(),
                    WeaponSlot::Secondary => combatant
                        .sheet
                        .offense
                        .offhand
                        .as_ref()
                        .map(|offhand| offhand.weapon.name.as_str())
                        .unwrap_or(combatant.sheet.offense.weapon.name.as_str()),
                })
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
                    let verb = if attack.is_charge { "charges" } else { "hits" };
                    format!(
                        "{attacker_name} {verb} {defender_name} with {weapon_name} for {} dmg",
                        attack.damage
                    )
                }
            } else if attack.shield_block {
                if attack.is_charge {
                    format!(
                        "{defender_name} blocks {attacker_name}'s charge with shield for {} shield dmg",
                        attack.shield_damage
                    )
                } else {
                    format!(
                        "{defender_name} blocks {attacker_name} with shield for {} shield dmg",
                        attack.shield_damage
                    )
                }
            } else if attack.hold_at_bay {
                format!("{attacker_name} fails to hold {defender_name} at bay with {weapon_name}")
            } else {
                if attack.is_charge {
                    format!("{attacker_name} charges {defender_name} with {weapon_name} but misses")
                } else {
                    format!("{attacker_name} misses {defender_name} with {weapon_name}")
                }
            };

            let mut details = Vec::new();
            details.push(format_attack_roll(&attack.roll));
            if let Some(breakdown) = attack.damage_breakdown.as_ref() {
                details.push(format_damage_breakdown(breakdown));
            }
            if let Some(breakdown) = attack.shield_damage_breakdown.as_ref() {
                details.push(format_shield_damage_breakdown(breakdown));
            }
            details.push(format!("hp {}", attack.defender_hp_after.max(0)));
            if attack.is_ranged {
                details.push("ranged".to_string());
            }
            if attack.is_charge {
                details.push("charge".to_string());
            }
            if attack.use_jab {
                details.push("jab".to_string());
            }
            if attack.knockback_ft > 0.0 {
                details.push(format!("knockback {:.0}ft", attack.knockback_ft));
            }
            if let Some(seconds) = attack.trauma_seconds {
                details.push(format!("trauma {}s", seconds));
            }
            if let Some(crit) = attack.critical.as_ref() {
                let mut crit_parts = vec![format!("sev {}", crit.severity)];
                if crit.instant_kill {
                    crit_parts.push("kill".to_string());
                } else {
                    if crit.extra_dice > 0 {
                        crit_parts.push(format!("+{} dice", crit.extra_dice));
                    }
                    if crit.extra_damage > 0 {
                        crit_parts.push(format!("+{} dmg", crit.extra_damage));
                    }
                    if crit.speed_reset {
                        crit_parts.push("speed reset".to_string());
                    }
                    if let Some(seconds) = crit.trauma_seconds {
                        crit_parts.push(format!("ToP {}s", seconds));
                    }
                }
                details.push(format!("crit {}", crit_parts.join(", ")));
            }

            if details.is_empty() {
                base
            } else {
                format!("{base} [{}]", details.join(" | "))
            }
        }
        CombatEventKind::KnockAside(knock) => {
            let base = if knock.success {
                format!("{attacker_name} knocks aside {defender_name}'s weapon")
            } else {
                format!("{attacker_name} fails to knock aside {defender_name}'s weapon")
            };
            let details = format_knock_aside_roll(&knock.roll);
            format!("{base} [{details}]")
        }
    }
}

pub fn format_combat_event_line(event: &CombatEvent, combatants: &[Combatant]) -> String {
    format!(
        "t={}s | {}",
        event.time,
        format_combat_event(event, combatants)
    )
}

fn format_attack_roll(roll: &AttackRollBreakdown) -> String {
    let mut atk_parts = vec![
        format!("d20 {}", roll.attack_die),
        format!("bonus {}", roll.attack_bonus),
    ];
    if roll.range_mod != 0 {
        atk_parts.push(format!("range {}", roll.range_mod));
    }
    let atk = format!("atk {} ({})", roll.attack_total, atk_parts.join(" + "));

    let mut def_parts = vec![
        format!("d20 {}", roll.defense_die),
        format!("base {}", roll.defense_base),
    ];
    if roll.weapon_defense_bonus != 0 {
        def_parts.push(format!("weapon {}", roll.weapon_defense_bonus));
    }
    if roll.shield_defense_bonus != 0 {
        def_parts.push(format!("shield {}", roll.shield_defense_bonus));
    }
    let def = format!("def {} ({})", roll.defense_total, def_parts.join(" + "));

    format!("{atk} vs {def}")
}

fn format_knock_aside_roll(roll: &KnockAsideRollBreakdown) -> String {
    let atk_parts = vec![
        format!("d20 {}", roll.attack_die),
        format!("bonus {}", roll.attack_bonus),
    ];
    let atk = format!("atk {} ({})", roll.attack_total, atk_parts.join(" + "));

    let mut def_parts = vec![
        format!("d20 {}", roll.defense_die),
        format!("base {}", roll.defense_base),
    ];
    if roll.weapon_defense_bonus != 0 {
        def_parts.push(format!("weapon {}", roll.weapon_defense_bonus));
    }
    let def = format!("def {} ({})", roll.defense_total, def_parts.join(" + "));

    format!("{atk} vs {def}")
}

fn format_damage_breakdown(breakdown: &DamageBreakdown) -> String {
    let mut raw_terms = format!("roll {}", breakdown.rolled_damage);
    let mods = breakdown.raw_damage - breakdown.rolled_damage;
    if mods > 0 {
        raw_terms.push_str(&format!(" + mods {}", mods));
    } else if mods < 0 {
        raw_terms.push_str(&format!(" - mods {}", -mods));
    }
    format!(
        "raw {} ({}) - dr {} = {}",
        breakdown.raw_damage, raw_terms, breakdown.effective_armor_dr, breakdown.final_damage
    )
}

fn format_shield_damage_breakdown(breakdown: &ShieldDamageBreakdown) -> String {
    let mut raw_terms = format!("roll {}", breakdown.rolled_damage);
    let mods = breakdown.raw_damage - breakdown.rolled_damage;
    if mods > 0 {
        raw_terms.push_str(&format!(" + mods {}", mods));
    } else if mods < 0 {
        raw_terms.push_str(&format!(" - mods {}", -mods));
    }
    let mut text = format!(
        "shield raw {} ({}) - sdr {} - dr {} = hp {}",
        breakdown.raw_damage,
        raw_terms,
        breakdown.shield_dr,
        breakdown.effective_armor_dr,
        breakdown.hp_damage
    );
    if breakdown.shield_broken {
        text.push_str(" (shield broken)");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_breakdown_shows_hidden_modifiers() {
        let breakdown = DamageBreakdown {
            rolled_damage: 5,
            strength_damage: 2,
            raw_damage: 5,
            armor_dr: 7,
            armor_penetration: 0,
            effective_armor_dr: 7,
            final_damage: 0,
        };
        assert_eq!(
            format_damage_breakdown(&breakdown),
            "raw 5 (roll 5) - dr 7 = 0"
        );
    }

    #[test]
    fn shield_breakdown_shows_hidden_modifiers() {
        let breakdown = ShieldDamageBreakdown {
            rolled_damage: 4,
            strength_damage: 3,
            raw_damage: 10,
            shield_dr: 2,
            armor_dr: 5,
            armor_penetration: 0,
            effective_armor_dr: 5,
            hp_damage: 3,
            shield_broken: false,
        };
        assert_eq!(
            format_shield_damage_breakdown(&breakdown),
            "shield raw 10 (roll 4 + mods 6) - sdr 2 - dr 5 = hp 3"
        );
    }
}
