//! Equipment and profile rules for the four trained weapon styles.
use super::*;

pub(super) fn is_evonia_primary(weapon: &WeaponPreset) -> bool {
    weapon.size == WeaponSize::Large
        && matches!(
            weapon.group,
            WeaponGroup::LargeSwords | WeaponGroup::Polearms
        )
}

pub(super) fn is_evonia_secondary(weapon: &WeaponPreset) -> bool {
    matches!(weapon.size, WeaponSize::Small | WeaponSize::Medium)
        && matches!(
            weapon.group,
            WeaponGroup::SmallSwords | WeaponGroup::LargeSwords
        )
        && weapon.handedness == WeaponHandedness::OneHanded
}

/// Knowing the style permits configuring its loadout before activating it.
pub fn evonia_offhand_allowed(player: &PlayerConfig, weapon: &WeaponPreset) -> bool {
    player_has_talent(player, TALENT_ID_LEFT_HAND_OF_EVONIA) && is_evonia_primary(weapon)
}

pub fn offhand_option_allowed(
    player: &PlayerConfig,
    primary: &WeaponPreset,
    secondary: &WeaponPreset,
) -> bool {
    if evonia_offhand_allowed(player, primary) {
        is_evonia_secondary(secondary)
    } else {
        primary.handedness == WeaponHandedness::OneHanded
            && !player.two_hand_grip
            && secondary.handedness == WeaponHandedness::OneHanded
    }
}

pub(super) fn evonia_active(
    modifiers: &TalentModifiers,
    player: &PlayerConfig,
    primary: &WeaponPreset,
    catalog: &WeaponCatalog,
) -> bool {
    modifiers.left_hand_of_evonia_style
        && is_evonia_primary(primary)
        && player
            .offhand_weapon_id
            .and_then(|id| catalog.get(id))
            .is_some_and(is_evonia_secondary)
}

pub(super) fn evonia_defense_bonus(
    modifiers: &TalentModifiers,
    player: &PlayerConfig,
    primary: &WeaponPreset,
    catalog: &WeaponCatalog,
) -> i32 {
    if !evonia_active(modifiers, player, primary, catalog) {
        return 0;
    }
    let id = player.offhand_weapon_id.expect("validated Evonia offhand");
    let offhand = catalog.get(id).expect("validated Evonia offhand");
    player.mastery(offhand.group).defense
        + modifiers.defense_bonus_for_weapon(id)
        + if offhand.defense_bonus_always { 4 } else { 0 }
}

pub(super) fn is_one_path_weapon(weapon: &WeaponPreset) -> bool {
    weapon.group == WeaponGroup::LargeSwords && weapon.can_hack_and_pierce
}

pub(super) fn one_path_active(
    modifiers: &TalentModifiers,
    player: &PlayerConfig,
    weapon: &WeaponPreset,
) -> bool {
    modifiers.one_path_style
        && is_one_path_weapon(weapon)
        && effective_two_hand_grip(weapon, player.two_hand_grip)
}

pub(super) fn is_reaper_weapon(weapon: &WeaponPreset) -> bool {
    matches!(
        weapon.name.to_ascii_lowercase().as_str(),
        "scythe" | "sickle"
    )
}

pub(super) fn reaper_critical_min(
    modifiers: &TalentModifiers,
    weapon: &WeaponPreset,
    current: i32,
) -> i32 {
    if !modifiers.reaper_of_termon_style || !is_reaper_weapon(weapon) {
        return current;
    }
    match current {
        19 => 18,
        ..=18 => current.min(17),
        _ => current,
    }
}

pub(super) fn new_style_proficiency(id: &str, context: &TalentContext<'_>) -> Option<bool> {
    Some(match id {
        TALENT_ID_LEFT_HAND_OF_EVONIA => {
            has_weapon_matching(context, is_evonia_primary)
                && has_weapon_matching(context, is_evonia_secondary)
        }
        TALENT_ID_ONE_PATH => has_weapon_matching(context, is_one_path_weapon),
        TALENT_ID_PILGRIMS_PATH => has_any_weapon_name_proficiency(context, &["Staff"]),
        TALENT_ID_REAPER_OF_TERMON => {
            has_any_weapon_name_proficiency(context, &["Scythe", "Sickle"])
        }
        _ => return None,
    })
}

/// Reduce every die independently, preserving counts, penetration, and constants.
pub(super) fn reduce_damage_dice(expr: &str) -> String {
    let mut chars = expr.chars().peekable();
    let mut result = String::new();
    while let Some(ch) = chars.next() {
        result.push(ch);
        if ch != 'd' && ch != 'D' {
            continue;
        }
        let mut digits = String::new();
        while chars.peek().is_some_and(char::is_ascii_digit) {
            digits.push(chars.next().unwrap());
        }
        if let Ok(sides) = digits.parse::<u32>() {
            let reduced = [1, 2, 3, 4, 6, 8, 10, 12]
                .into_iter()
                .filter(|step| *step < sides)
                .max()
                .unwrap_or(1);
            result.push_str(&reduced.to_string());
        } else {
            result.push_str(&digits);
        }
    }
    result
}

pub(super) fn new_style_defense_bonus(
    modifiers: &TalentModifiers,
    player: &PlayerConfig,
    weapon: &WeaponPreset,
    catalog: &WeaponCatalog,
) -> i32 {
    evonia_defense_bonus(modifiers, player, weapon, catalog)
        + if modifiers.pilgrims_path_style && weapon.name.eq_ignore_ascii_case("staff") {
            4
        } else {
            0
        }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_path_die_steps_preserve_counts_penetration_and_flat_modifiers() {
        assert_eq!(
            reduce_damage_dice("2d12p+d10p+d8p+d6p+d4p+d3p+d2p+3"),
            "2d10p+d8p+d6p+d4p+d3p+d2p+d1p+3"
        );
        assert_eq!(reduce_damage_dice("lower of 2d6p"), "lower of 2d4p");
        assert_eq!(reduce_damage_dice("(d4p-2)+(d4p-2)"), "(d3p-2)+(d3p-2)");
        assert_eq!(reduce_damage_dice("d1p+3^2"), "d1p+3^2");
        assert_eq!(
            DamageExprCache::new(&reduce_damage_dice("d2p")).expected(false),
            1.0
        );
    }
}
