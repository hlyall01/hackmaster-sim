use super::*;

pub fn mounted_defense_bonus(player: &PlayerConfig) -> i32 {
    player.mounted_combat.defense_bonus(player.mounted)
}

/// Add target-dependent weapon dice to Derived tooltips while retaining flat modifier totals.
pub fn apply_target_damage_breakdowns(
    breakdowns: &mut DerivedStatBreakdowns,
    attacker: &Combatant,
    defender: &Combatant,
    is_ranged: bool,
) {
    let primary = &attacker.sheet.offense.weapon;
    let mut weapons = vec![(primary.as_ref(), is_ranged,
        DerivedStatId::MainhandDamageRoll, DerivedStatId::MainhandShieldDamage)];
    if let Some(offhand) = &attacker.sheet.offense.offhand {
        weapons.push((offhand.weapon.as_ref(), false,
            DerivedStatId::OffhandDamageRoll, DerivedStatId::OffhandShieldDamage));
    }
    for (weapon, ranged, damage_id, shield_id) in weapons {
        let expression = sim::weapon_damage_expression(attacker, defender, weapon, ranged, false);
        if let Some(breakdown) = breakdowns.entries.get_mut(&damage_id) {
            breakdown.notes.retain(|note| !note.starts_with("Add weapon damage dice"));
            breakdown.note(format!("Add weapon damage dice {expression} against the current target."));
        }
        if let Some(breakdown) = breakdowns.entries.get_mut(&shield_id) {
            let expression = sim::weapon_damage_expression(attacker, defender, weapon, ranged, true);
            breakdown.result = expression.clone();
            breakdown.note(format!("Effective weapon shield damage against the current target: {expression}."));
        }
    }
}

pub fn mounted_attack_bonus(player: &PlayerConfig, weapon: &WeaponPreset) -> i32 {
    if !player.mounted || weapon.group == WeaponGroup::Unarmed {
        return 0;
    }
    player
        .mounted_combat
        .attack_bonus(&weapon.name, is_ranged_weapon(weapon))
}

pub fn mounted_damage_summary(player: &PlayerConfig, weapon: &WeaponPreset) -> String {
    if !player.mounted || is_ranged_weapon(weapon) || weapon.group == WeaponGroup::Unarmed {
        return "No mounted bonus damage dice for this attack.".into();
    }
    let settings = player.mounted_combat;
    let medium = settings.damage_plan(&weapon.name, true);
    let other = settings.damage_plan(&weapon.name, false);
    format!(
        "Mounted: {}weapon dice; +{} smallest dice vs Medium, +{} vs other sizes. Flat damage modifiers are unchanged.",
        if medium.double_base_dice {
            "double "
        } else {
            "normal "
        },
        medium.extra_smallest,
        other.extra_smallest,
    )
}

pub const MOUNT_TYPE_OPTIONS: [(MountType, &str); 4] = [
    (MountType::RidingHorse, "Riding horse"),
    (MountType::Rounsey, "Rounsey (light warhorse)"),
    (MountType::Courser, "Courser (+1 die at trot+)"),
    (MountType::Destrier, "Destrier (+2 dice at trot+)"),
];
pub const RIDING_MASTERY_OPTIONS: [(RidingMastery, &str); 4] = [
    (RidingMastery::Average, "Average (-2 melee / -6 ranged)"),
    (RidingMastery::Advanced, "Advanced (0 melee / -4 ranged)"),
    (RidingMastery::Expert, "Expert (0 melee / -2 ranged)"),
    (RidingMastery::Master, "Master (no Riding penalty)"),
];
pub const MOUNTED_TARGET_SIZE_OPTIONS: [(MountedTargetSize, &str); 3] = [
    (MountedTargetSize::FromDefender, "Use defender's body size"),
    (MountedTargetSize::Medium, "Treat targets as Medium"),
    (MountedTargetSize::Other, "Treat targets as non-Medium"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data;

    #[test]
    fn mounted_defense_updates_derived_rolls_and_breakdowns_without_changing_base_dv() {
        let (weapons, armor, shields) = data::load_catalogs().unwrap();
        let talents = TalentCatalog::new(vec![]);
        let npcs = NpcPresetCatalog::new(vec![]);
        let id = weapons.id_from_index(weapons.entries().iter().position(|w| w.name == "Lance").unwrap()).unwrap();
        let mut player = PlayerConfig::new("Rider", id);
        let baseline = player_summary(&player, &weapons, &armor, &shields, &talents);
        for (mounted, moving, bonus) in [(true, false, 2), (true, true, 6), (false, true, 0)] {
            player.mounted = mounted;
            player.mounted_combat.trot_or_faster = moving;
            let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
            let combatant = build_combatant(&player, &weapons, &armor, &shields, &npcs, &talents);
            let breakdowns = derived_stat_breakdowns(&player, &weapons, &armor, &shields,
                &talents, &summary, &combatant);
            assert_eq!(summary.derived.base_dv, baseline.derived.base_dv);
            let melee = breakdowns.get(DerivedStatId::MeleeDefense).unwrap();
            assert_eq!(summary.defense.melee_roll_label,
                format!("Defense roll (melee): d20p + {}", melee.additive_total() as i32));
            for id in [DerivedStatId::MeleeDefense, DerivedStatId::RangedDefense] {
                let total: f64 = breakdowns.get(id).unwrap().lines.iter()
                    .filter(|line| line.source.starts_with("Mounted:"))
                    .filter_map(|line| line.numeric_amount).sum();
                assert_eq!(total, bonus as f64);
            }
            assert_eq!(summary.defense.ranged_roll_label.contains("(mounted)"), mounted);
            if !mounted {
                assert_eq!(summary.defense.melee_roll_label, baseline.defense.melee_roll_label);
            }
        }
    }

    #[test]
    fn derived_tooltips_use_current_target_dice_for_both_hands_and_shields() {
        let mut attacker = Combatant::new(sim::CombatantSheet::default());
        attacker.sheet.maneuvers.mounted = true;
        attacker.sheet.maneuvers.mounted_combat = MountedCombatConfig {
            mount: MountType::Courser,
            trot_or_faster: true,
            ..Default::default()
        };
        attacker.sheet.offense.weapon = std::sync::Arc::new(sim::WeaponProfile {
            name: "Lance".into(),
            damage_expr: "2d8p".into(),
            damage_expr_cache: DamageExprCache::new("2d8p"),
            shield_damage_expr: Some("2d8p+3".into()),
            shield_damage_expr_cache: Some(DamageExprCache::new("2d8p+3")),
            has_weapon: true,
            ..Default::default()
        });
        attacker.sheet.offense.offhand = Some(sim::OffhandProfile {
            weapon: attacker.sheet.offense.weapon.clone(),
            attack_bonus: 0,
            strength_damage: 5,
        });
        let mut defender = Combatant::new(sim::CombatantSheet::default());
        for (medium, dice) in [(true, 7), (false, 5)] {
            defender.sheet.defense.is_medium_sized = medium;
            let mut breakdowns = DerivedStatBreakdowns::default();
            for id in [DerivedStatId::MainhandDamageRoll, DerivedStatId::OffhandDamageRoll,
                DerivedStatId::MainhandShieldDamage, DerivedStatId::OffhandShieldDamage] {
                let mut breakdown = StatBreakdown::new("5");
                breakdown.add_i32(5, "Flat damage modifier");
                breakdown.note("Add weapon damage dice 2d8p.");
                breakdowns.insert(id, breakdown);
            }
            apply_target_damage_breakdowns(&mut breakdowns, &attacker, &defender, false);
            for id in [DerivedStatId::MainhandDamageRoll, DerivedStatId::OffhandDamageRoll] {
                let breakdown = breakdowns.get(id).unwrap();
                assert_eq!(breakdown.additive_total(), 5.0);
                assert_eq!(breakdown.notes, vec![format!(
                    "Add weapon damage dice {dice}d8p against the current target.")]);
            }
            for id in [DerivedStatId::MainhandShieldDamage, DerivedStatId::OffhandShieldDamage] {
                assert_eq!(breakdowns.get(id).unwrap().result, format!("{dice}d8p + 3"));
            }
        }
    }

    #[test]
    fn mounted_settings_are_backward_compatible_and_round_trip() {
        let old: CombatManeuverConfig = serde_json::from_str(r#"{"mounted":true}"#).unwrap();
        assert_eq!(old.mounted_combat, MountedCombatConfig::default());
        let mut config = old;
        config.mounted_combat = MountedCombatConfig {
            mount: MountType::Destrier,
            riding: RidingMastery::Expert,
            trot_or_faster: true,
            target_size: MountedTargetSize::Other,
        };
        let saved = serde_json::to_string(&config).unwrap();
        assert_eq!(
            serde_json::from_str::<CombatManeuverConfig>(&saved).unwrap(),
            config
        );
    }

    #[test]
    fn mounted_attack_bonus_summary_and_combat_profile_agree() {
        let (weapons, armor, shields) = data::load_catalogs().unwrap();
        let talents = TalentCatalog::new(vec![]);
        let id = weapons
            .id_from_index(
                weapons
                    .entries()
                    .iter()
                    .position(|w| w.name == "Lance")
                    .unwrap(),
            )
            .unwrap();
        let mut player = PlayerConfig::new("Rider", id);
        let npcs = NpcPresetCatalog::new(vec![]);
        let build =
            |p: &PlayerConfig| build_combatant(p, &weapons, &armor, &shields, &npcs, &talents);
        let baseline = build(&player).sheet.offense.attack_bonus;
        player.mounted = true;
        assert_eq!(build(&player).sheet.offense.attack_bonus, baseline); // Average: +2 -2
        player.mounted_combat.riding = RidingMastery::Advanced;
        assert_eq!(build(&player).sheet.offense.attack_bonus, baseline + 2);
        player.mounted_combat.trot_or_faster = true;
        let built = build(&player);
        assert_eq!(built.sheet.offense.attack_bonus, baseline + 6);
        assert_eq!(built.sheet.maneuvers.mounted_combat, player.mounted_combat);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        assert_eq!(summary.roll.attack_bonus, built.sheet.offense.attack_bonus);
        player.mounted = false;
        assert_eq!(build(&player).sheet.offense.attack_bonus, baseline);
    }
}
