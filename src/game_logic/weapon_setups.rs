//! Build both weapon setups once, retaining one set of runtime vitals and effects.
use super::*;

pub fn player_for_ranged_setup(
    player: &PlayerConfig,
    weapons: &WeaponCatalog,
    shields: &ShieldCatalog,
) -> Result<PlayerConfig, String> {
    let config = player
        .ranged_weapon_setup
        .as_ref()
        .ok_or("No ranged setup")?;
    let matches = weapons
        .entries()
        .iter()
        .enumerate()
        .filter(|(_, w)| w.name == config.weapon)
        .collect::<Vec<_>>();
    if matches.len() != 1 || !is_ranged_weapon(matches[0].1) {
        return Err(format!(
            "Missing or invalid ranged weapon: {}",
            config.weapon
        ));
    }
    let mut ranged = player.clone();
    ranged.ranged_weapon_setup = None;
    ranged.weapon_id = weapons.id_from_index(matches[0].0).unwrap();
    ranged.weapon_material_tier = config.material_tier;
    ranged.projectile_material_tier = config.projectile_material_tier;
    ranged.iron_weapon = config.iron;
    ranged.called_shot = config.called_shot.unwrap_or(player.called_shot);
    ranged.offhand_weapon_id = None;
    ranged.shield_id = shields
        .entries()
        .iter()
        .position(|entry| entry.shield.is_none())
        .and_then(|index| shields.id_from_index(index))
        .ok_or("Missing unshielded setup")?;
    ranged.two_hand_grip = matches[0].1.handedness == WeaponHandedness::TwoHanded;
    ranged.defensive_dualwielding = false;
    ranged.offensive_dualwielding = false;
    ranged.charge = false;
    ranged.hold_at_bay = false;
    ranged.power_attack = false;
    ranged.default_weapon_style_ids = config.default_style_ids.clone();
    ranged.active_weapon_style_ids = None;
    Ok(ranged)
}

pub fn resolve_player_stats(
    player: &PlayerConfig,
    weapons: &WeaponCatalog,
    armor: &ArmorCatalog,
    shields: &ShieldCatalog,
    npcs: &NpcPresetCatalog,
    talents: &TalentCatalog,
) -> ResolvedPlayerStats {
    let Some(config) = player
        .ranged_weapon_setup
        .as_ref()
        .filter(|_| player.npc_preset.is_none())
    else {
        return resolve_player_stats_single_setup(player, weapons, armor, shields, npcs, talents);
    };
    let Ok(ranged_player) = player_for_ranged_setup(player, weapons, shields) else {
        return resolve_player_stats_single_setup(player, weapons, armor, shields, npcs, talents);
    };
    let mut melee_player = player.clone();
    melee_player.ranged_weapon_setup = None;
    let melee =
        resolve_player_stats_single_setup(&melee_player, weapons, armor, shields, npcs, talents);
    let ranged =
        resolve_player_stats_single_setup(&ranged_player, weapons, armor, shields, npcs, talents);
    let mut drawing_player = melee_player.clone();
    if let Some((index, _)) = weapons
        .entries()
        .iter()
        .enumerate()
        .find(|(_, w)| w.name == "Fist")
    {
        drawing_player.weapon_id = weapons.id_from_index(index).unwrap();
    }
    drawing_player.offhand_weapon_id = None;
    drawing_player.shield_id = ranged_player.shield_id;
    drawing_player.defensive_dualwielding = false;
    drawing_player.offensive_dualwielding = false;
    drawing_player.tactical_policy.enabled = false;
    drawing_player.default_weapon_style_ids = Some(vec![]);
    drawing_player.active_weapon_style_ids = None;
    let drawing =
        resolve_player_stats_single_setup(&drawing_player, weapons, armor, shields, npcs, talents);
    let draw_seconds = weapons.get(player.weapon_id).map_or(0.0, |w| match w.size {
        WeaponSize::Small => 0.0,
        WeaponSize::Medium => 1.0,
        WeaponSize::Large => 2.0,
    });
    let setups = Arc::new(sim::WeaponSetups {
        melee: melee.combatant.clone(),
        ranged: ranged.combatant.clone(),
        drawing: drawing.combatant,
        melee_draw_seconds: draw_seconds,
        start_with: config.start_with,
        auto_switch_to_melee: config.auto_switch_to_melee,
    });
    let mut selected = match config.start_with {
        sim::WeaponSetup::Melee => melee,
        sim::WeaponSetup::Ranged => ranged,
    };
    selected.combatant.weapon_setups = Some(setups);
    selected.combatant.state.weapon_setup.active = config.start_with;
    selected.combatant.state.weapon_setup.melee_shield_intact = selected
        .combatant
        .weapon_setups
        .as_ref()
        .unwrap()
        .melee
        .sheet
        .defense
        .shield_name
        .is_some();
    selected.config = player.clone();
    selected
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aiming_is_configured_per_setup_and_legacy_saves_keep_their_preference() {
        let (w, a, s) = crate::data::load_catalogs().unwrap();
        let t = crate::data::load_talents(crate::data::TALENTS_PATH).unwrap();
        let id = w.id_from_index(w.entries().iter().position(|w| w.name == "Longsword").unwrap()).unwrap();
        let mut p = PlayerConfig::new("Archer", id);
        p.ranged_weapon_setup = Some(serde_json::from_value(serde_json::json!({
            "weapon":"Longbow", "called_shot":true
        })).unwrap());
        let c = build_combatant(&p, &w, &a, &s, &NpcPresetCatalog::new(vec![]), &t);
        let pair = c.weapon_setups.as_ref().unwrap();
        assert!(pair.ranged.sheet.maneuvers.called_shot);
        assert!(!pair.melee.sheet.maneuvers.called_shot);
        // Old saved setups had one shared checkbox; preserve it until edited.
        p.called_shot = true;
        p.ranged_weapon_setup.as_mut().unwrap().called_shot = None;
        let legacy = player_for_ranged_setup(&p, &w, &s).unwrap();
        assert!(legacy.called_shot);
        p.called_shot = false;
        assert!(!player_for_ranged_setup(&p, &w, &s).unwrap().called_shot);
    }
}
