//! Equivalence checks for shortcuts in the simulation loop.
use super::*;

#[test]
fn tactical_profiles_preserve_only_caches_for_unchanged_weapons() {
    use crate::core::sim::{OffhandProfile, CombatantTacticalProfile, TacticalProfileKey};
    use std::sync::Arc;
    let mut fighter = Combatant::default();
    fighter.sheet.offense.offhand = Some(OffhandProfile {
        attack_bonus: 0,
        strength_damage: 0,
        weapon: fighter.sheet.offense.weapon.clone(),
    });
    fighter.tactical_policy.enabled = true;
    let profile = CombatantTacticalProfile {
        key: TacticalProfileKey {
            style_ids: fighter.active_style_ids.clone(),
            use_jab: false,
            fight_defensively_penalty: None,
        },
        sheet: fighter.sheet.clone(),
        weapon_group: fighter.weapon_group.clone(),
        armor_type: fighter.armor_type.clone(),
    };
    fighter.tactical_profiles = vec![profile.clone()].into();
    fighter.state.cache.primary.max_range = Some(Some(120.0));
    fighter.state.cache.secondary.max_range = Some(Some(60.0));
    fighter.sheet.offense.attack_bonus += 20;
    fighter.sheet.maneuvers.called_shot = true;
    assert!(fighter.activate_tactical_profile(false));
    assert_eq!(fighter.sheet.offense.attack_bonus, profile.sheet.offense.attack_bonus);
    assert!(fighter.sheet.maneuvers.called_shot);
    assert_eq!(fighter.state.cache.primary.max_range, Some(Some(120.0)));
    assert_eq!(fighter.state.cache.secondary.max_range, Some(Some(60.0)));

    let mut changed = profile;
    Arc::make_mut(&mut changed.sheet.offense.weapon).name = "Replacement weapon".into();
    fighter.tactical_profiles = vec![changed.clone()].into();
    assert!(fighter.activate_tactical_profile(false));
    assert_eq!(fighter.state.cache.primary.max_range, None);
    assert_eq!(fighter.state.cache.secondary.max_range, Some(Some(60.0)));
    changed.sheet.offense.offhand = None;
    fighter.tactical_profiles = vec![changed].into();
    assert!(fighter.activate_tactical_profile(false));
    assert_eq!(fighter.state.cache.secondary.max_range, None);
}

#[test]
fn batched_movement_matches_individual_steps() {
    let mut sim = SimState::new(SimConfig::new(5.0, 1.0));
    let mut enemy = Combatant::default();
    enemy.team_id = 1;
    sim.reset_with_combatants(vec![Combatant::default(), enemy]);
    sim.config.grid_width = 9;
    sim.config.grid_height = 9;
    for x in 0..9 {
        for y in 0..9 {
            for tx in 0..9 {
                for ty in 0..9 {
                    let from = GridPos::new(x, y);
                    let target = GridPos::new(tx, ty);
                    for steps in [0, 1, 4, 20] {
                        let mut expected = from;
                        for _ in 0..steps {
                            expected = SimState::step_away(expected, target).clamp(9, 9);
                        }
                        sim.actors[0].position = from;
                        sim.actors[1].position = target;
                        sim.move_away(0, 1, steps);
                        assert_eq!(sim.actors[0].position, expected);
                        for tile_size in [0.01, 0.3, 1.0, 5.0] {
                            sim.config.tile_size_ft = tile_size;
                            for stop in [-1.0, 0.0, 0.9, 2.0, 20.0] {
                                let mut expected = from;
                                for _ in 0..steps {
                                    if expected.manhattan_distance(target) as f32 * tile_size
                                        <= stop
                                    {
                                        break;
                                    }
                                    expected = SimState::step_toward(expected, target).clamp(9, 9);
                                }
                                sim.actors[0].position = from;
                                sim.move_toward(0, 1, steps, stop);
                                assert_eq!(
                                    sim.actors[0].position, expected,
                                    "{from:?} -> {target:?}, {steps} steps, tile {tile_size}, stop {stop}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn channel_context_matches_full_context_for_each_rule() {
    use crate::core::tactics::{
        NumericComparison, RelativeComparison, SpeedComparison, TacticalRule,
    };
    let mut sim = SimState::new(SimConfig::new(5.0, 1.0));
    let mut enemy = Combatant::default();
    enemy.team_id = 1;
    enemy.weapon_group = "bows".into();
    enemy.armor_type = "heavy".into();
    enemy.active_style_ids = vec!["enemy_style".into()];
    let mut mine = Combatant::default();
    mine.active_style_ids = vec!["my_style".into()];
    sim.reset_with_combatants(vec![mine, enemy]);
    for condition in [
        TacticalCondition::MyHpPercent {
            comparison: NumericComparison::GreaterOrEqual,
            value: 0.0,
        },
        TacticalCondition::EnemyHpPercent {
            comparison: NumericComparison::GreaterOrEqual,
            value: 0.0,
        },
        TacticalCondition::DistanceFt {
            comparison: NumericComparison::Greater,
            value: 0.0,
        },
        TacticalCondition::ReachComparedToEnemy {
            comparison: RelativeComparison::Equal,
        },
        TacticalCondition::EnemyTimeToReachSeconds {
            comparison: NumericComparison::GreaterOrEqual,
            value: 0.0,
        },
        TacticalCondition::EnemyDr {
            comparison: NumericComparison::GreaterOrEqual,
            value: 0.0,
        },
        TacticalCondition::EnemyAttackSpeedSeconds {
            comparison: NumericComparison::Greater,
            value: 0.0,
        },
        TacticalCondition::EnemyAttackSpeedComparedToMine {
            comparison: SpeedComparison::Equal,
        },
        TacticalCondition::MyWeaponCanJab { value: false },
        TacticalCondition::EnemyWeaponGroup {
            value: "bows".into(),
            negated: false,
        },
        TacticalCondition::EnemyArmorType {
            value: "heavy".into(),
            negated: false,
        },
        TacticalCondition::MyActiveStyle {
            style_id: "my_style".into(),
            negated: false,
        },
        TacticalCondition::EnemyActiveStyle {
            style_id: "enemy_style".into(),
            negated: false,
        },
    ] {
        for action in [
            TacticalAction::NeutralWeaponStyle,
            TacticalAction::CalledShot,
            TacticalAction::FightDefensively { penalty: 4 },
            TacticalAction::StandGround,
        ] {
            let channel = action.channel();
            let decision = action.decision_point();
            sim.combatants[0].tactical_policy.enabled = true;
            sim.combatants[0].tactical_policy.rules =
                vec![TacticalRule::new(action, vec![condition.clone()])];
            let policy = &sim.combatants[0].tactical_policy;
            let full = evaluate_channel(policy, decision, channel, &sim.tactical_context(0, 1));
            let trimmed = evaluate_channel(
                policy,
                decision,
                channel,
                &sim.tactical_context_for_channel(0, 1, Some(channel)),
            );
            assert_eq!(trimmed, full);
            assert_eq!(full.matched_rule_index, Some(0));
        }
    }
}
