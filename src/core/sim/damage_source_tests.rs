use super::*;
use crate::core::magic::{MagicTalents, SpellCastAi};
use crate::core::sim::{
    CombatantSheet, OffhandProfile, SimConfig, WeaponProfile, bulk_simulate_with_seed,
};
use rand::{SeedableRng, rngs::StdRng};

fn fighters() -> Vec<Combatant> {
    (0..2)
        .map(|team| {
            let mut sheet = CombatantSheet::default();
            sheet.vitals.max_hp = 10_000;
            sheet.vitals.threshold_of_pain = 10_000;
            sheet.defense.knockback_step = 10_000;
            sheet.offense.attack_bonus = 100;
            sheet.offense.attack_bonus_base = 100;
            sheet.offense.strength_damage_base = 10;
            sheet.offense.strength_damage = 10;
            sheet.offense.weapon = Arc::new(WeaponProfile {
                name: "Sword".into(),
                has_weapon: true,
                reach_ft: 5.0,
                speed: 2.0,
                crit_min_roll: 100,
                ..WeaponProfile::default()
            });
            Combatant::new_with_team(sheet, team)
        })
        .collect()
}

fn attack(actors: &mut [Combatant], slot: WeaponSlot, seed: u64) -> AttackOutcome {
    resolve_attack(
        actors,
        0,
        1,
        0,
        false,
        5.0,
        AttackMode::Normal,
        slot,
        10.0,
        None,
        &mut StdRng::seed_from_u64(seed),
    )
}

#[test]
fn damage_source_remembers_weapon_before_switch_and_distinguishes_hands() {
    let mut actors = fighters();
    actors[0].sheet.offense.offhand = Some(OffhandProfile {
        attack_bonus: 100,
        strength_damage: 10,
        weapon: actors[0].sheet.offense.weapon.clone(),
    });
    let first = attack(&mut actors, WeaponSlot::Primary, 7);
    let offhand = attack(&mut actors, WeaponSlot::Secondary, 7);
    Arc::make_mut(&mut actors[0].sheet.offense.weapon).name = "Mace".into();
    actors[0].state.invalidate_weapon_cache(WeaponSlot::Primary);
    let switched = attack(&mut actors, WeaponSlot::Primary, 7);
    assert!(
        first.damage > 0 && offhand.damage > 0 && switched.damage > 0,
        "damage {}, {}, {}",
        first.damage,
        offhand.damage,
        switched.damage
    );
    assert_eq!(
        first.damage_source,
        DamageSource::Weapon {
            name: "Sword".into(),
            slot: WeaponSlot::Primary,
        }
    );
    assert_eq!(
        offhand.damage_source,
        DamageSource::Weapon {
            name: "Sword".into(),
            slot: WeaponSlot::Secondary,
        }
    );
    assert_eq!(
        switched.damage_source,
        DamageSource::Weapon {
            name: "Mace".into(),
            slot: WeaponSlot::Primary,
        }
    );
}

#[test]
fn damage_source_regular_unarmed_attack_is_not_a_weapon_hit() {
    let mut actors = fighters();
    let weapon = Arc::make_mut(&mut actors[0].sheet.offense.weapon);
    weapon.name = "Fist".into();
    weapon.is_unarmed = true;
    let hit = attack(&mut actors, WeaponSlot::Primary, 7);
    assert!(hit.damage > 0);
    assert_eq!(
        hit.damage_source,
        DamageSource::Unarmed {
            name: "Fist".into(),
        }
    );
}

#[test]
fn damage_source_actual_near_perfect_defense_uses_punch_small_weapon_or_eyesmite() {
    for kind in 0..3 {
        let mut found = false;
        for seed in 0..500 {
            let mut actors = fighters();
            actors[0].sheet.offense.attack_bonus = -100;
            actors[1].sheet.defense.defense_mod = 100;
            if kind == 1 {
                let mut weapon = (*actors[1].sheet.offense.weapon).clone();
                weapon.name = "Dagger".into();
                weapon.is_small_weapon = true;
                actors[1].sheet.offense.offhand = Some(OffhandProfile {
                    attack_bonus: 100,
                    strength_damage: 10,
                    weapon: Arc::new(weapon),
                });
                actors[1].sheet.maneuvers.offensive_dualwielding = true;
            }
            actors[1].sheet.defense.eyesmite = kind == 2;
            let outcome = attack(&mut actors, WeaponSlot::Primary, seed);
            if outcome.roll.defense_die != 19 {
                continue;
            }
            let counter = outcome.counter_attack.expect("natural 19 must counter");
            assert!(counter.damage > 0);
            let expected = match kind {
                0 => DamageSource::Unarmed {
                    name: "Near-perfect defense punch".into(),
                },
                1 => DamageSource::Weapon {
                    name: "Dagger".into(),
                    slot: WeaponSlot::Secondary,
                },
                _ => DamageSource::Ability {
                    name: "Eyesmite".into(),
                },
            };
            assert_eq!(counter.damage_source, expected);
            found = true;
            break;
        }
        assert!(found, "no near-perfect defense found for kind {kind}");
    }
}

#[test]
fn damage_source_weapon_counter_combines_with_normal_attack() {
    let mut actors = fighters();
    let counter = resolve_counter_attack(
        &mut actors,
        0,
        1,
        10.0,
        WeaponSlot::Primary,
        true,
        false,
        false,
        false,
        false,
        1,
        &mut StdRng::seed_from_u64(7),
    );
    assert!(counter.damage > 0);
    assert_eq!(
        counter.damage_source,
        DamageSource::Weapon {
            name: "Sword".into(),
            slot: WeaponSlot::Primary,
        }
    );
    assert_eq!(
        counter.damage_source,
        attack(&mut actors, WeaponSlot::Primary, 7).damage_source
    );
}

#[test]
fn damage_source_bulk_echoes_are_separate_and_rows_reconcile_with_team_totals() {
    let mut actors = fighters();
    actors[0].magic.level = 10;
    actors[0].magic.loadout.learn_spell("echo_strike");
    actors[0]
        .magic
        .loadout
        .spell_ai
        .insert("echo_strike".into(), SpellCastAi::AtFightStart);
    actors[0].magic.loadout.echo_strike.delay_seconds = 1;
    actors[0].magic.loadout.echo_strike.additional_echoes = 2;
    actors[0].magic.talents = MagicTalents {
        combat_casting: true,
        diminish_spell_fatigue: 5,
        mitigate_spell_fatigue: true,
        decimate_spell_fatigue: true,
        eliminate_spell_fatigue: true,
        ..Default::default()
    };
    actors[0].sheet.offense.offhand = Some(OffhandProfile {
        attack_bonus: 100,
        strength_damage: 10,
        weapon: Arc::new(WeaponProfile {
            name: "Dagger".into(),
            ..(*actors[0].sheet.offense.weapon).clone()
        }),
    });
    actors[0].sheet.maneuvers.offensive_dualwielding = true;
    // Bulk simulation disables combat logs. Attribution must survive that path.
    let result = bulk_simulate_with_seed(SimConfig::new(20.0, 1.0), actors, 40, 40, 31);
    let sources = &result.detailed.teams[0].damage_by_source;
    for expected in [
        DamageSource::Spell {
            name: "Echo Strike".into(),
        },
        DamageSource::Weapon {
            name: "Sword".into(),
            slot: WeaponSlot::Primary,
        },
        DamageSource::Weapon {
            name: "Dagger".into(),
            slot: WeaponSlot::Secondary,
        },
    ] {
        assert!(
            sources
                .iter()
                .any(|row| row.source == expected && row.total_hp_damage > 0),
            "missing {expected}: {sources:?}"
        );
    }
    assert!(
        !result.detailed.teams[1]
            .damage_by_source
            .iter()
            .any(|row| matches!(row.source, DamageSource::Spell { .. }))
    );
    for (idx, team) in result.detailed.teams.iter().enumerate() {
        let sum = team
            .damage_by_source
            .iter()
            .map(|row| row.total_hp_damage)
            .sum::<u64>();
        assert_eq!(sum as f32 / 40.0, team.avg_hp_damage_per_fight);
        assert_eq!(sum as f32 / 40.0, result.avg_damage_dealt_by_team[idx]);
        assert!(
            (team
                .damage_by_source
                .iter()
                .map(|row| row.damage_share)
                .sum::<f32>()
                - 1.0)
                .abs()
                < 0.0001
        );
        assert!(
            (team
                .damage_by_source
                .iter()
                .map(|row| row.combat_dps)
                .sum::<f32>()
                - team.combat_dps)
                .abs()
                < 0.0001
        );
        assert!(
            team.damage_by_source
                .windows(2)
                .all(|rows| rows[0].total_hp_damage >= rows[1].total_hp_damage)
        );
        for row in &team.damage_by_source {
            assert_eq!(
                row.avg_hp_damage_per_fight,
                row.total_hp_damage as f32 / 40.0
            );
        }
    }
}

#[test]
fn damage_source_misses_are_zero_and_non_attacking_team_has_no_sources() {
    let mut actors = fighters();
    actors[0].sheet.offense.attack_bonus = -10_000;
    actors[1].sheet.defense.defense_mod = 10_000;
    actors[1].sheet.maneuvers.passive = true;
    let result = bulk_simulate_with_seed(SimConfig::new(5.0, 1.0), actors, 10, 10, 1);
    let sources = &result.detailed.teams[0].damage_by_source;
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].total_hp_damage, 0);
    assert_eq!(sources[0].damage_share, 0.0);
    assert_eq!(sources[0].combat_dps, 0.0);
    assert!(result.detailed.teams[1].damage_by_source.is_empty());
}

#[test]
fn damage_source_empty_runs_and_zero_duration_have_no_rows() {
    for (runs, seconds) in [(0, 20), (10, 0)] {
        let result =
            bulk_simulate_with_seed(SimConfig::new(5.0, 1.0), fighters(), runs, seconds, 1);
        assert!(
            result
                .detailed
                .teams
                .iter()
                .all(|team| team.damage_by_source.is_empty())
        );
    }
}
