use super::*;

#[derive(Clone, Deserialize)]
pub struct NpcLoadout {
    pub weapon: String,
    #[serde(default)]
    pub weapon_material_tier: i32,
    pub armor: String,
    #[serde(default)]
    pub armor_material_tier: i32,
    #[serde(default)]
    pub shield: Option<String>,
    #[serde(default)]
    pub shield_material_tier: i32,
    #[serde(default)]
    pub offhand_weapon: Option<String>,
    #[serde(default)]
    pub offhand_weapon_material_tier: i32,
    #[serde(default)]
    pub defensive_dualwielding: bool,
    #[serde(default)]
    pub talents: Vec<TalentSelection>,
}

/// Select a stat block and, when provided, replace the previous fighter's loadout.
/// Resolve every reference before changing the player so a bad preset is atomic.
pub fn apply_npc_preset(
    player: &mut PlayerConfig,
    id: NpcPresetId,
    presets: &NpcPresetCatalog,
    weapons: &WeaponCatalog,
    armor: &ArmorCatalog,
    shields: &ShieldCatalog,
) -> Result<(), String> {
    let preset = presets.get(id).ok_or("Unknown NPC preset")?;
    if let Some(loadout) = &preset.loadout {
        let weapon_id = |name: &str| {
            weapons
                .entries()
                .iter()
                .position(|weapon| weapon.name.eq_ignore_ascii_case(name))
                .and_then(|index| weapons.id_from_index(index))
                .ok_or_else(|| format!("Unknown NPC weapon: {name}"))
        };
        let mut configured = PlayerConfig::new(&preset.name, weapon_id(&loadout.weapon)?);
        configured.two_hand_grip = weapons
            .get(configured.weapon_id)
            .is_some_and(|weapon| weapon.handedness == WeaponHandedness::TwoHanded);
        configured.armor_id = armor
            .entries()
            .iter()
            .position(|entry| {
                entry
                    .armor
                    .as_ref()
                    .map_or("None", |armor| armor.name.as_str())
                    .eq_ignore_ascii_case(&loadout.armor)
            })
            .and_then(|index| armor.id_from_index(index))
            .ok_or_else(|| format!("Unknown NPC armor: {}", loadout.armor))?;
        configured.shield_id = shields
            .entries()
            .iter()
            .position(|entry| {
                entry
                    .label
                    .eq_ignore_ascii_case(loadout.shield.as_deref().unwrap_or("None"))
            })
            .and_then(|index| shields.id_from_index(index))
            .ok_or_else(|| format!("Unknown NPC shield: {:?}", loadout.shield))?;
        configured.offhand_weapon_id = loadout
            .offhand_weapon
            .as_deref()
            .map(weapon_id)
            .transpose()?;
        configured.weapon_material_tier = loadout.weapon_material_tier;
        configured.armor_material_tier = loadout.armor_material_tier;
        configured.shield_material_tier = loadout.shield_material_tier;
        configured.offhand_weapon_material_tier = loadout.offhand_weapon_material_tier;
        configured.defensive_dualwielding = loadout.defensive_dualwielding;
        configured.talents = loadout.talents.clone();
        *player = configured;
    }
    player.name = preset.name.clone();
    player.npc_preset = Some(id);
    player.fighter_preset = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalogs() -> (
        WeaponCatalog,
        ArmorCatalog,
        ShieldCatalog,
        NpcPresetCatalog,
        TalentCatalog,
    ) {
        let path = |name: &str| format!("{}/data/sim/{name}.json", env!("CARGO_MANIFEST_DIR"));
        (
            crate::data::load_weapon_catalog(&path("weapons")).unwrap(),
            crate::data::load_armor_catalog(&path("armor")).unwrap(),
            crate::data::load_shield_catalog(&path("weapons")).unwrap(),
            crate::data::load_npc_presets(&path("npc_presets")).unwrap(),
            crate::data::load_talents(&path("talents")).unwrap(),
        )
    }

    #[test]
    fn equipped_npc_combat_totals_match_supplied_stat_blocks() {
        let (weapons, armor, shields, presets, talents) = catalogs();
        let expected = [
            ("Mace Guard", 25, 3, 0, 1, 3, 10, -1, 10.0, "Mace", 0),
            (
                "Scimitar Raider",
                30,
                5,
                1,
                1,
                3,
                12,
                -1,
                8.0,
                "Scimitar",
                0,
            ),
            (
                "Hide Swordsman",
                35,
                6,
                4,
                4,
                4,
                14,
                -3,
                8.0,
                "Broadsword",
                0,
            ),
            ("Axe Sentinel", 40, 8, 9, 4, 7, 16, -4, 8.0, "Battle axe", 7),
            ("Twin Sabres", 45, 9, 10, 7, 9, 18, -5, 3.0, "Sabre", 0),
            ("Glaive Striker", 50, 19, 14, 5, 0, 20, -8, 6.0, "Glaive", 0),
            (
                "Plate Warden",
                55,
                12,
                15,
                6,
                11,
                22,
                -7,
                5.0,
                "Bastard sword",
                8,
            ),
            (
                "Greatsword Knight",
                60,
                14,
                8,
                5,
                11,
                24,
                -6,
                8.0,
                "Greatsword",
                0,
            ),
        ];
        for (name, hp, att, def, dam, dr, top, sp, speed, weapon, sdr) in expected {
            let index = presets
                .entries()
                .iter()
                .position(|preset| preset.name == name)
                .unwrap();
            let mut player = PlayerConfig::new("Previous fighter", weapons.first_id().unwrap());
            player.level = 20;
            player.misc_modifiers.all_roll_bonus = 50;
            player.magic.learn_spell("echo_strike");
            apply_npc_preset(
                &mut player,
                NpcPresetId::new(index),
                &presets,
                &weapons,
                &armor,
                &shields,
            )
            .unwrap();
            let resolved =
                resolve_player_stats(&player, &weapons, &armor, &shields, &presets, &talents);
            let combatant = &resolved.combatant;
            assert_eq!(combatant.sheet.vitals.max_hp, hp, "{name}: HP");
            assert_eq!(combatant.sheet.vitals.threshold_of_pain, top, "{name}: TOP");
            assert_eq!(resolved.summary.roll.attack_bonus, att, "{name}: ATT");
            assert_eq!(resolved.summary.roll.strength_damage, dam, "{name}: DAM");
            assert_eq!(combatant.sheet.defense.armor_dr, dr, "{name}: DR");
            assert_eq!(combatant.sheet.defense.shield_dr, sdr, "{name}: SDR");
            assert_eq!(resolved.summary.derived.speed_mod, sp, "{name}: SP");
            assert_eq!(combatant.sheet.offense.weapon.speed, speed, "{name}: speed");
            assert_eq!(
                combatant.sheet.offense.weapon.two_hand_grip,
                matches!(name, "Glaive Striker" | "Greatsword Knight"),
                "{name}: two-handed grip"
            );
            assert_eq!(
                combatant.sheet.offense.weapon.name, weapon,
                "{name}: weapon"
            );
            use rand::SeedableRng;
            let mut rng = rand::rngs::StdRng::seed_from_u64(1);
            let mut pair = vec![combatant.clone(), combatant.clone()];
            let attack = crate::core::sim::test_support::resolve_basic_attack(
                &mut pair, 0, 1, 0, false, 1.0, 0.0, &mut rng,
            );
            assert_eq!(
                attack.roll.defense_total - attack.roll.defense_die,
                def,
                "{name}: ready DEF"
            );
            assert_eq!(
                combatant.sheet.maneuvers.defensive_dualwielding,
                name == "Twin Sabres"
            );
            assert_eq!(
                combatant.sheet.defense.natural_dr,
                if name == "Greatsword Knight" { 1 } else { 0 }
            );
            assert_eq!(
                combatant.apply_i32(sim::StatIdI32::FlagIgnoreAncillaryCritEffects, 0) > 0,
                matches!(name, "Twin Sabres" | "Plate Warden"),
                "{name}: HAO"
            );
            assert!(
                !player.magic.knows_spell("echo_strike"),
                "previous fighter's spells must not carry over"
            );
        }
    }

    #[test]
    fn npc_damage_rolls_include_weapon_constants_and_listed_damage_once() {
        use rand::{SeedableRng, rngs::StdRng};

        let (weapons, armor, shields, presets, talents) = catalogs();
        // These expressions include both the listed DAM and any flat bonus
        // built into the weapon dice. In particular, the two-handed weapons
        // already have +3 before the NPC's +5 DAM is added.
        let expected = [
            ("Mace Guard", "d6p+d8p+1"),
            ("Scimitar Raider", "2d8p+1"),
            ("Hide Swordsman", "2d6p+d3p+4"),
            ("Axe Sentinel", "4d3p+4"),
            ("Twin Sabres", "d6p+d8p+7"),
            ("Glaive Striker", "5d4p+8"),
            ("Plate Warden", "d8p+d10p+6"),
            ("Greatsword Knight", "d10p+d12p+8"),
        ];
        for (name, expression) in expected {
            let index = presets
                .entries()
                .iter()
                .position(|preset| preset.name == name)
                .unwrap();
            let mut player = PlayerConfig::new("NPC", weapons.first_id().unwrap());
            apply_npc_preset(
                &mut player,
                NpcPresetId::new(index),
                &presets,
                &weapons,
                &armor,
                &shields,
            )
            .unwrap();
            let combatant =
                build_combatant(&player, &weapons, &armor, &shields, &presets, &talents);
            let expected_damage = DamageExprCache::new(expression);
            let mut actual_rng = StdRng::seed_from_u64(17);
            let mut expected_rng = actual_rng.clone();
            for _ in 0..1_000 {
                let actual = simulation_jobs::roll_weapon_raw_damage(
                    &combatant.sheet.offense.weapon,
                    combatant.sheet.offense.strength_damage,
                    0,
                    &mut actual_rng,
                );
                assert_eq!(
                    actual,
                    expected_damage.roll(&mut expected_rng, false),
                    "{name}"
                );
            }
        }
    }

    #[test]
    fn legacy_npc_presets_keep_existing_behavior() {
        let (weapons, armor, shields, presets, talents) = catalogs();
        let mut player = PlayerConfig::new("Original", weapons.first_id().unwrap());
        player.shield_id = ShieldId::new(1);
        player.defensive_dualwielding = true;
        apply_npc_preset(
            &mut player,
            NpcPresetId::new(0),
            &presets,
            &weapons,
            &armor,
            &shields,
        )
        .unwrap();
        let result = build_combatant(&player, &weapons, &armor, &shields, &presets, &talents);
        assert_eq!(result.sheet.vitals.max_hp, 27);
        assert_eq!(result.sheet.defense.defense_mod, -2);
        assert_eq!(result.sheet.offense.attack_bonus, 5);
        assert!(result.sheet.defense.shield_name.is_none());
        assert!(!result.sheet.maneuvers.defensive_dualwielding);
    }

    #[test]
    fn invalid_npc_equipment_does_not_partially_apply() {
        let (weapons, armor, shields, mut presets, _) = catalogs();
        let mut player = PlayerConfig::new("Original", weapons.first_id().unwrap());
        player.level = 12;
        let original = player.clone();
        let index = presets
            .entries()
            .iter()
            .position(|preset| preset.loadout.is_some())
            .unwrap();
        let id = NpcPresetId::new(index);
        presets.get_mut(id).unwrap().loadout.as_mut().unwrap().armor = "Missing armor".into();
        assert!(apply_npc_preset(&mut player, id, &presets, &weapons, &armor, &shields).is_err());
        assert!(player == original);
    }
}
