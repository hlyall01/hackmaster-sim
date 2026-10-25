//! Arborite character derivation. Permanent bonuses never mutate saved base scores.
use super::*;
use crate::character::MasteryState;

pub(super) fn arborite_natural_dr(player: &PlayerConfig) -> i32 {
    if player.race_id.as_deref() != Some("arborite") {
        return 0;
    }
    let barkskin = player_has_talent(player, "barkskin");
    if barkskin && player_has_talent(player, "arborhide") {
        8
    } else if player.environment.natural_surroundings {
        if barkskin { 3 } else { 1 }
    } else {
        0
    }
}

pub(super) fn arborite_profile(player: &PlayerConfig) -> sim::ArboriteProfile {
    let arborite = player.race_id.as_deref() == Some("arborite");
    sim::ArboriteProfile {
        iron_dice_multiplier: if !arborite {
            1
        } else if player_has_talent(player, "barkskin") && player_has_talent(player, "arborhide") {
            3
        } else {
            2
        },
        warrior_woad: matches!(player.race_id.as_deref(), Some("arborite" | "armeroci"))
            && player_has_talent(player, "warrior_woad")
            && player.racial_talents.average_artistry_painting,
        warpaint_prepared: player.racial_talents.warpaint_prepared,
        silent_defiance: arborite && player_has_talent(player, "silent_defiance"),
        choose_silent_defiance: player.racial_talents.choose_silent_defiance,
        initially_compelled: player.racial_talents.under_mental_compulsion,
    }
}

fn shillelagh_weapon(weapon: &WeaponPreset) -> bool {
    matches!(
        weapon.name.to_ascii_lowercase().as_str(),
        "stick" | "club" | "great club" | "greatclub" | "staff" | "quarterstaff"
    )
}

pub(super) fn mastery_for_weapon(player: &PlayerConfig, weapon: &WeaponPreset) -> MasteryState {
    let own = player.mastery(weapon.group);
    if player.race_id.as_deref() != Some("arborite")
        || !player_has_talent(player, "shillelagh")
        || !shillelagh_weapon(weapon)
    {
        return own;
    }
    player
        .weapon_masteries
        .iter()
        .filter(|(group, _)| {
            !matches!(
                group,
                WeaponGroup::Bows | WeaponGroup::Crossbows | WeaponGroup::Shields
            )
        })
        .map(|(group, _)| (*group, player.mastery(*group)))
        .max_by_key(|(group, mastery)| {
            (
                i64::from(mastery.attack)
                    + i64::from(mastery.damage)
                    + i64::from(mastery.speed)
                    + i64::from(mastery.defense),
                *group,
            )
        })
        .map_or(own, |(_, mastery)| mastery)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        character::{MasteryState, ProgressionTier},
        data,
    };

    fn fixture(
        weapon_name: &str,
    ) -> (
        PlayerConfig,
        WeaponCatalog,
        ArmorCatalog,
        ShieldCatalog,
        TalentCatalog,
    ) {
        let (w, a, s) = data::load_catalogs().unwrap();
        let id = w
            .id_from_index(
                w.entries()
                    .iter()
                    .position(|x| x.name == weapon_name)
                    .unwrap(),
            )
            .unwrap();
        let mut p = PlayerConfig::new("Arborite", id);
        p.race_id = Some("arborite".into());
        (p, w, a, s, data::load_talents(data::TALENTS_PATH).unwrap())
    }
    fn add(p: &mut PlayerConfig, id: &str) {
        p.talents.push(TalentSelection {
            id: id.into(),
            rank: 1,
            weapon: None,
        });
    }

    #[test]
    fn race_definition_and_ability_adjustments_match_the_supplied_race() {
        let races = data::load_races("data/sim/races.json").unwrap();
        let race = races.iter().find(|r| r.id == "arborite").unwrap();
        let (mut p, w, a, s, t) = fixture("Staff");
        apply_race_adjustments(&mut p, race);
        assert_eq!(
            (
                p.base_hp,
                p.strength_base,
                p.intelligence,
                p.wisdom,
                p.dex_base,
                p.constitution,
                p.charisma,
                p.looks
            ),
            (5, 8, 11, 8, 14, 6, 12, 12)
        );
        assert_eq!(p.knockback_step, 10);
        let c = build_combatant(&p, &w, &a, &s, &NpcPresetCatalog::new(vec![]), &t);
        assert!(c.sheet.defense.is_medium_sized);
        assert_eq!(race.native_languages, ["Arborian"]);
        assert_eq!(race.plural, "arborites");
        assert_eq!(race.other_names, ["arborians"]);
    }

    #[test]
    fn natural_dr_upgrades_replace_the_racial_bonus_and_have_no_false_prerequisite_effects() {
        for surroundings in [false, true] {
            for (talents, expected) in [
                (vec![], if surroundings { 1 } else { 0 }),
                (vec!["barkskin"], if surroundings { 3 } else { 0 }),
                (vec!["arborhide"], if surroundings { 1 } else { 0 }),
                (vec!["barkskin", "arborhide"], 8),
            ] {
                let (mut p, w, a, s, t) = fixture("Staff");
                p.environment.natural_surroundings = surroundings;
                for id in talents {
                    add(&mut p, id);
                }
                let c = build_combatant(&p, &w, &a, &s, &NpcPresetCatalog::new(vec![]), &t);
                assert_eq!(c.sheet.defense.natural_dr, expected);
                assert_eq!(c.sheet.defense.armor_dr, expected);
                assert_eq!(
                    c.arborite.iron_dice_multiplier,
                    if expected == 8 { 3 } else { 2 }
                );
            }
        }
    }

    #[test]
    fn racial_speed_initiative_defense_and_resistances_apply_without_purchases() {
        let (mut p, w, a, s, t) = fixture("Staff");
        let neutral = player_summary(
            &{
                let mut n = p.clone();
                n.race_id = None;
                n
            },
            &w,
            &a,
            &s,
            &t,
        );
        let outdoors = {
            p.environment.natural_surroundings = true;
            player_summary(&p, &w, &a, &s, &t)
        };
        assert_eq!(outdoors.derived.base_dv, neutral.derived.base_dv + 2);
        assert_eq!(outdoors.derived.speed_mod, neutral.derived.speed_mod - 1);
        assert_eq!(
            outdoors.derived.initiative_mod,
            neutral.derived.initiative_mod - 2
        );
        assert_eq!(
            outdoors.derived.initiative_die,
            neutral.derived.initiative_die.improved(1)
        );
        let c = build_combatant(&p, &w, &a, &s, &NpcPresetCatalog::new(vec![]), &t);
        assert!(c.magic.talents.charm_resistant && c.magic.talents.illusion_resistant);
        assert_eq!(p.progression.attack, ProgressionTier::I);
    }

    #[test]
    fn shillelagh_copies_one_whole_melee_group_and_preserves_recorded_masteries() {
        let (mut p, w, a, s, t) = fixture("Staff");
        let best = MasteryState {
            attack: 6,
            damage: 4,
            speed: 3,
            defense: 4,
        };
        p.weapon_masteries.insert(WeaponGroup::Polearms, best);
        p.weapon_masteries.insert(
            WeaponGroup::Axes,
            MasteryState {
                attack: 6,
                damage: 1,
                speed: 1,
                defense: 6,
            },
        );
        p.weapon_masteries.insert(
            WeaponGroup::Bows,
            MasteryState {
                attack: 6,
                damage: 6,
                speed: 6,
                defense: 6,
            },
        );
        p.weapon_masteries.insert(
            WeaponGroup::Shields,
            MasteryState {
                attack: 6,
                damage: 6,
                speed: 6,
                defense: 6,
            },
        );
        let original = p.weapon_masteries.clone();
        let baseline = build_combatant(&p, &w, &a, &s, &NpcPresetCatalog::new(vec![]), &t);
        add(&mut p, "shillelagh");
        assert_eq!(mastery_for_weapon(&p, w.get(p.weapon_id).unwrap()), best);
        let c = build_combatant(&p, &w, &a, &s, &NpcPresetCatalog::new(vec![]), &t);
        assert_eq!(
            c.sheet.offense.attack_bonus - baseline.sheet.offense.attack_bonus,
            6
        );
        assert_eq!(
            c.sheet.defense.defense_mod - baseline.sheet.defense.defense_mod,
            4
        );
        assert_eq!(
            c.sheet.offense.weapon.speed,
            baseline.sheet.offense.weapon.speed - 3.0
        );
        assert_eq!(p.weapon_masteries, original);
        let sword = w
            .entries()
            .iter()
            .find(|w| w.group == WeaponGroup::SmallSwords)
            .unwrap();
        assert_eq!(mastery_for_weapon(&p, sword), p.mastery(sword.group));
    }

    #[test]
    fn naivety_and_painting_prerequisites_are_enforced_without_mutating_saved_scores() {
        let (mut p, w, _, _, t) = fixture("Staff");
        add(&mut p, "overcome_naivety");
        p.racial_talents.age_years = 500;
        assert_eq!(ability_set_from_player(&p).wisdom, 10);
        let stats = ability_set_from_player(&p);
        let context = TalentContext {
            age_years: 500,
            average_artistry_painting: false,
            level: 1,
            stats: &stats,
            talents: &p.talents,
            proficiencies: &p.proficiencies,
            weapon_catalog: Some(&w),
        };
        assert_eq!(
            evaluate_talent_requirements(find_talent(&t, "overcome_naivety").unwrap(), &context),
            vec![TalentRequirementFailure::MinAge {
                required: 501,
                current: 500
            }]
        );
        assert_eq!(
            evaluate_talent_requirements(find_talent(&t, "warrior_woad").unwrap(), &context),
            vec![TalentRequirementFailure::AverageArtistryPainting]
        );
        p.racial_talents.age_years = 501;
        assert_eq!(ability_set_from_player(&p).wisdom, 13);
        assert_eq!(ability_set_from_player(&p).wisdom, 13);
        assert_eq!(p.wisdom, 10);
    }
}
