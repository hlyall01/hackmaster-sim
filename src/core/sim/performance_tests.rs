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
    for x in -4..5 {
        for y in -4..5 {
            for tx in -4..5 {
                for ty in -4..5 {
                    let from = GridPos::new(x, y);
                    let target = GridPos::new(tx, ty);
                    for steps in [0, 1, 4, 20] {
                        let mut expected = from;
                        for _ in 0..steps {
                            expected = expected.offset(target.direction_to(expected), 1);
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
                                    expected = expected.offset(expected.direction_to(target), 1);
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
            sim.bulk = Some(BulkContext::new(&sim.combatants).into());
            let trimmed = sim.decide_tactic(0, 1, channel, false);
            assert_eq!(trimmed, full);
            assert_eq!(full.matched_rule_index, Some(0));
        }
    }
}

#[test]
fn bulk_aura_checks_include_alternate_equipment_and_tactical_profiles() {
    use crate::core::sim::{CombatantTacticalProfile, TacticalProfileKey, WeaponSetup, WeaponSetups};
    let mut melee = Combatant::default();
    let mut sheet = melee.sheet.clone();
    sheet.modifiers.add_i32(StatIdI32::FlagIntimidateAdversary, super::super::ModifierOpI32::Set(1));
    melee.tactical_profiles = vec![CombatantTacticalProfile {
        key: TacticalProfileKey { style_ids: vec![], use_jab: false, fight_defensively_penalty: None },
        sheet,
        weapon_group: String::new(),
        armor_type: String::new(),
    }].into();
    melee.magic.loadout.learn_spell(super::super::STREAMLINE_EFFECT_ID);
    let ranged = Combatant::default();
    let mut fighter = ranged.clone();
    let empty = BulkContext::new(std::slice::from_ref(&fighter));
    assert!(empty.intimidation == IntimidationPlan::Absent && !empty.may_average_damage);
    fighter.weapon_setups = Some(std::sync::Arc::new(WeaponSetups {
        melee, ranged, drawing: Combatant::default(), melee_draw_seconds: 1.0,
        start_with: WeaponSetup::Ranged, auto_switch_to_melee: true,
    }));
    let possible = BulkContext::new(&[fighter]);
    assert!(possible.intimidation != IntimidationPlan::Absent && possible.may_average_damage);
}

#[test]
fn indexed_modifiers_keep_clone_isolation_and_float_extension_order() {
    use crate::core::sim::{ModifierOpF32, ModifierOpI32, ModifierStack};
    let mut base = ModifierStack::default();
    base.add_i32(StatIdI32::ArmorDr, ModifierOpI32::Set(6));
    base.add_f32(StatIdF32::MoveSpeed, ModifierOpF32::Add(1e20));
    let mut extra = ModifierStack::default();
    extra.add_i32(StatIdI32::ArmorDr, ModifierOpI32::Set(7));
    extra.add_i32(StatIdI32::ArmorDr, ModifierOpI32::Add(3));
    extra.add_i32(StatIdI32::ArmorDr, ModifierOpI32::Min(9));
    extra.add_i32(StatIdI32::ArmorDr, ModifierOpI32::Max(12));
    extra.add_f32(StatIdF32::MoveSpeed, ModifierOpF32::Add(-1e20));
    extra.add_f32(StatIdF32::MoveSpeed, ModifierOpF32::Add(3.0));
    extra.add_f32(StatIdF32::MoveSpeed, ModifierOpF32::Mul(2.0));
    let original = base.clone();
    base.extend(&extra);
    assert_eq!(base.apply_f32(1.0, StatIdF32::MoveSpeed), 8.0);
    assert_eq!(base.apply_i32(0, StatIdI32::ArmorDr), 12);
    assert_eq!(original.apply_i32(0, StatIdI32::ArmorDr), 6);
    assert_eq!(original.apply_f32(1.0, StatIdF32::MoveSpeed), 1e20);
    assert_eq!(base.apply_f32(9.0, StatIdF32::WeaponSpeed), 9.0);
}

#[test]
fn prepared_combat_flags_follow_sheet_edits_and_temporary_effects() {
    use crate::core::sim::{ModifierOpI32, TemporaryEffect};
    let flag = StatIdI32::FlagHammererStyle;
    let mut fighter = Combatant::default();
    let old_sheet = fighter.sheet.clone();
    fighter.sheet.modifiers.add_i32(flag, ModifierOpI32::Set(2));
    assert!(fighter.combat_flag_active(flag));
    assert!(!old_sheet.modifiers.positive_at_zero(flag));
    let mut suppress = TemporaryEffect::new("suppress_style", 1);
    suppress.modifiers.add_i32(flag, ModifierOpI32::Add(-2));
    fighter.state.add_effect(suppress);
    assert!(!fighter.combat_flag_active(flag));
    fighter.state.tick_effects();
    assert!(fighter.combat_flag_active(flag));
    fighter.sheet.modifiers.add_i32(flag, ModifierOpI32::Min(-1));
    assert!(!fighter.combat_flag_active(flag));
}

#[test]
fn dense_histogram_retains_boundaries_and_sparse_overflow() {
    let mut bins = DenseHistogram::default();
    for value in [256, 0, u64::MAX, 255, 1, u64::MAX, 0, 256] {
        bins.add(value);
    }
    assert_eq!(bins.finish(), BTreeMap::from([(0, 2), (1, 1), (255, 1), (256, 2), (u64::MAX, 2)]));
}

#[test]
fn prepared_ranges_keep_sparse_teams_and_merge_in_any_order() {
    let fighters = [211, 17, 211].map(|team_id| Combatant { team_id, ..Combatant::default() });
    let prepared = PreparedBulkSimulation::new(SimConfig::new(5.0, 1.0), fighters.to_vec());
    let run = |first, runs, limit| prepared.run_range(runs, limit, 551, first, |_| true).unwrap();
    let whole = run(0, 50, 200);
    assert_eq!(whole.detailed_accumulators.iter().map(|t| t.team_id).collect::<Vec<_>>(), [17, 211]);
    assert_eq!(whole.wins.iter().sum::<u32>() + whole.ties, 50);
    assert!(whole.detailed_accumulators.iter().all(|t| t.attack_attempts > 0));
    let mut reversed = run(13, 37, 200);
    reversed.merge(run(0, 13, 200));
    assert_eq!(serde_json::to_value(whole).unwrap(), serde_json::to_value(reversed).unwrap());
    assert_eq!(run(0, 3, 0).ties, 3);
    assert!(prepared.run_range(50, 200, 551, 0, |index| index < 2).is_none());
}

#[test]
fn prepared_called_shot_uses_live_dr_while_a_buff_is_active() {
    use crate::core::sim::{ModifierOpI32, TemporaryEffect};
    use crate::core::tactics::{NumericComparison, TacticalRule};
    let mut attacker = Combatant::default();
    attacker.tactical_policy.enabled = true;
    attacker.tactical_policy.rules = vec![TacticalRule::new(TacticalAction::CalledShot, vec![
        TacticalCondition::EnemyDr { comparison: NumericComparison::Greater, value: 7.0 }
    ])];
    let mut defender = Combatant { team_id: 1, ..Combatant::default() };
    defender.sheet.defense.armor_dr = 7;
    let mut sim = SimState::new(SimConfig::new(1.0, 1.0));
    sim.reset_with_combatants(vec![attacker, defender]);
    let context = BulkContext::new(&sim.combatants);
    assert_eq!(context.fixed_dr_called_shot[0], Some(false));
    sim.bulk = Some(context.into());
    sim.combatants[0].state.set_next_attack_time(WeaponSlot::Primary, Some(10.0));
    let mut buff = TemporaryEffect::new("test_armor", 10);
    buff.modifiers.add_i32(StatIdI32::ArmorDr, ModifierOpI32::Add(1));
    sim.combatants[1].state.active_effects.push(buff);
    assert!(sim.refresh_called_shot_tactic(0, 1, 1.0)[0] > 0.0);
    assert!(sim.combatants[0].sheet.maneuvers.called_shot);
    assert_eq!(sim.refresh_called_shot_tactic(0, 1, 1.0), [0.0; 2]);
    sim.combatants[1].state.active_effects.clear();
    sim.refresh_called_shot_tactic(0, 1, 1.0);
    assert!(!sim.combatants[0].sheet.maneuvers.called_shot);
}

#[test]
fn idle_round_preserves_clocks_and_rng_and_escape_endpoints_match_the_walk() {
    let fighters = vec![Combatant::default(), Combatant::new_with_team(Default::default(), 1)];
    let mut original = SimState::with_rng(SimConfig::new(5.0, 1.0), SimRng::from_seed(9001));
    original.reset_with_combatants(fighters);
    for c in &mut original.combatants { c.state.next_attack_time_primary = Some(12.0); }
    let mut fast = original.clone();
    fast.bulk = Some(BulkContext::new(&fast.combatants).into());
    for sim in [&mut original, &mut fast] { sim.resolve_combat_round_at(0, 1, 1.0, 5.0, 5.0); }
    assert_eq!(original.rng.next_u64(), fast.rng.next_u64());
    assert_eq!(original.combatants[1].state.next_attack_time_primary, fast.combatants[1].state.next_attack_time_primary);
    for size in [1.0, 2.0, 10.0] {
        for p in 0..35 { for q in 0..35 {
            for sim in [&mut original, &mut fast] {
                sim.config.tile_size_ft = size;
                sim.actors[0].position = GridPos::new(p % 7 - 3, p / 7 - 2);
                sim.actors[1].position = GridPos::new(q % 7 - 3, q / 7 - 2);
            }
            assert_eq!(original.precognition_evasion_destination(0, 1), fast.precognition_evasion_destination(0, 1));
        }}
    }
}

#[test]
fn built_range_respects_explicit_edits_and_legacy_name_changes() {
    use crate::core::sim::BuiltWeaponRange;
    let mut w = WeaponProfile::default();
    w.name = "Sword".into();
    w.built_range = Some(BuiltWeaponRange::new(&w.name, None));
    assert_eq!(max_range_for_weapon(&w), None);
    w.name = "Net".into();
    assert_eq!(max_range_for_weapon(&w), Some(15.0));
    w.range_bands_feet = Some([25.0, 30.0, 40.0, 60.0]);
    assert_eq!(max_range_for_weapon(&w), Some(60.0));
    assert_eq!(range_modifier_for_weapon_with_scale(&w, 20.0, 2.0), Some(-6));
}

#[test]
fn first_opponent_tracking_handles_both_sides_of_the_inline_mask() {
    for quiet in [false, true] {
        let mut c = Combatant::default();
        c.state.magic.quiet = quiet;
        for i in [0, 63, 64, 138] {
            assert!(!c.state.has_seen_attacker(i));
            assert!(c.state.remember_attacker(i));
            assert!(!c.state.remember_attacker(i));
        }
        if quiet { assert_eq!(c.state.deceptive_defender_seen_attackers, [64, 138]); }
    }
}

#[test]
fn fixed_duel_intimidation_matches_live_scan_across_range_life_and_effect_changes() {
    use crate::core::sim::{ModifierOpI32, TemporaryEffect};

    let flag = StatIdI32::FlagIntimidateAdversary;
    let mut source = Combatant::new_with_team(Default::default(), 1);
    source.sheet.modifiers.add_i32(flag, ModifierOpI32::Set(1));
    let mut sim = SimState::new(SimConfig::new(10.0, 1.0));
    sim.reset_with_combatants(vec![Combatant::default(), source]);
    sim.bulk = Some(BulkContext::for_grid(&sim.combatants, sim.config.tile_size_ft).into());
    assert_eq!(sim.intimidation_plan(), IntimidationPlan::FixedDuel { sources: [false, true] });

    fn check(sim: &mut SimState, expected: [bool; 2]) {
        let mut live = sim.clone();
        live.bulk = None;
        live.refresh_intimidation();
        for _ in 0..2 {
            sim.refresh_intimidation();
            for (i, expected) in expected.iter().enumerate() {
                assert_eq!(sim.combatants[i].state.intimidated, *expected);
                assert_eq!(sim.combatants[i].state.intimidated, live.combatants[i].state.intimidated);
            }
        }
    }

    sim.actors[0].position = GridPos::new(0, 0);
    sim.actors[1].position = GridPos::new(6, 4);
    check(&mut sim, [true, false]);
    sim.actors[1].position.x += 1;
    check(&mut sim, [false, false]);
    sim.config.tile_size_ft = 0.5;
    check(&mut sim, [true, false]);
    sim.combatants[1].state.hp = 0;
    check(&mut sim, [false, false]);
    sim.combatants[1].state.hp = 1;
    sim.combatants[0].state.hp = 0;
    check(&mut sim, [false, false]);
    sim.combatants[0].state.hp = 1;
    sim.combatants[1].team_id = 0;
    check(&mut sim, [false, false]);
    sim.combatants[1].team_id = 1;
    check(&mut sim, [true, false]);

    let mut suppress = TemporaryEffect::new("test suppression", 1);
    suppress.modifiers.add_i32(flag, ModifierOpI32::Set(0));
    let mut grant = TemporaryEffect::new("test aura", 1);
    grant.modifiers.add_i32(flag, ModifierOpI32::Set(1));
    sim.combatants[0].state.add_effect(grant);
    sim.combatants[1].state.add_effect(suppress);
    check(&mut sim, [false, true]);
    assert!(sim.duel_in_intimidation_range.is_none());
    sim.combatants[1].state.tick_effects();
    check(&mut sim, [true, true]);
    sim.combatants[0].state.tick_effects();
    check(&mut sim, [true, false]);
    sim.reset();
    check(&mut sim, [true, false]);
}

#[test]
fn duel_intimidation_uses_live_flags_when_a_tactical_profile_changes_eligibility() {
    use crate::core::sim::{CombatantTacticalProfile, TacticalProfileKey, ModifierOpI32};
    let mut source = Combatant::default();
    let inactive = source.sheet.clone();
    source.sheet.modifiers.add_i32(StatIdI32::FlagIntimidateAdversary, ModifierOpI32::Set(1));
    source.tactical_profiles = vec![CombatantTacticalProfile {
        key: TacticalProfileKey { style_ids: vec![], use_jab: false, fight_defensively_penalty: None },
        sheet: inactive, weapon_group: String::new(), armor_type: String::new(),
    }].into();
    let mut sim = SimState::new(SimConfig::new(5.0, 1.0));
    sim.reset_with_combatants(vec![Combatant::new_with_team(Default::default(), 1), source.clone()]);
    sim.combatants[1].sheet = source.sheet;
    sim.bulk = Some(BulkContext::new(&sim.combatants).into());
    assert_eq!(sim.intimidation_plan(), IntimidationPlan::Live);
    sim.refresh_intimidation();
    assert!(sim.combatants[0].state.intimidated);
    assert!(sim.combatants[1].activate_tactical_profile(false));
    sim.refresh_intimidation();
    assert!(!sim.combatants[0].state.intimidated);
}


#[test]
fn stationary_spans_match_each_tick_at_attack_timeout_and_cancellation_boundaries() {
    let mut sim = SimState::with_rng(SimConfig::new(1.0, 1.0), SimRng::from_seed(9876));
    let mut pair = vec![Combatant::default(), Combatant::new_with_team(Default::default(), 1)];
    for c in &mut pair {
        c.sheet.maneuvers.charge = false;
        c.magic.loadout.auto_cast = crate::core::magic::AutoCast::SpellPolicies;
    }
    sim.reset_with_combatants(pair);
    sim.bulk = Some(BulkContext::new(&sim.combatants).into());
    for (start, deadline, limit, expected) in [
        (9, 15.5, 100, 16), (9, 15.00005, 100, 15), (9, 15.5, 12, 12),
        (62, 90.0, 100, 64), (64, 90.0, 100, 64),
        ((1 << 25) + 3, 40_000_000.0, u32::MAX, (1 << 25) + 3),
    ] {
        let mut fast = sim.clone();
        fast.elapsed_seconds = start;
        for c in &mut fast.combatants {
            c.state.set_next_attack_time(WeaponSlot::Primary, Some(deadline));
        }
        let mut reference = fast.clone();
        fast.skip_stationary_recovery(limit);
        assert_eq!(fast.elapsed_seconds, expected);
        while reference.elapsed_seconds < expected { reference.tick(); }
        for (a, b) in fast.combatants.iter().zip(&reference.combatants) {
            assert_eq!(a.state.magic.now, b.state.magic.now);
            assert_eq!(a.state.next_attack_time_primary, b.state.next_attack_time_primary);
            assert_eq!((a.state.hp, a.state.moved_last_tick), (b.state.hp, b.state.moved_last_tick));
        }
        assert_eq!(fast.rng.next_u64(), reference.rng.next_u64());
    }
}


#[test]
fn bulk_impact_totals_agree_with_logged_hits_blocks_and_misses() {
    use crate::core::sim::{ModifierOpI32, ShieldBreakageStep, TemporaryEffect};
    let mut attacker = Combatant::default();
    attacker.sheet.offense.strength_damage = 7;
    attacker.sheet.modifiers.add_i32(StatIdI32::FlagLargeSwordShieldStyle, ModifierOpI32::Set(1));
    let mut defender = Combatant::new_with_team(Default::default(), 1);
    defender.sheet.defense.shield_name = Some("Buckler".into());
    defender.sheet.defense.shield_dr = 3;
    defender.sheet.defense.armor_dr = 4;
    defender.sheet.defense.shield_breakage = Some([1, 3, 8, 16].map(|threshold|
        ShieldBreakageStep { threshold, save_mod: Some(2) }));
    let pair = [attacker, defender];
    let mut seen = [false; 3];
    for seed in 0..256 {
        let mut live = pair.clone();
        live[1].state.shield_intact = true;
        live[0].state.shield_intact = seed % 2 == 0;
        live[0].state.intimidated = seed % 3 == 0;
        if seed % 5 == 0 {
            let mut effect = TemporaryEffect::new("test strength", 2);
            effect.modifiers.add_i32(StatIdI32::StrengthDamage, ModifierOpI32::Add(5));
            live[0].state.add_effect(effect);
        }
        let mut bulk = live.clone();
        let plan = BulkContext::new(&bulk);
        for (c, profile) in bulk.iter_mut().zip(plan.prepared) { c.state.prepared = Some(profile); }
        let mut rng = SimRng::from_seed(seed);
        let mut bulk_rng = rng.clone();
        let strike = |actors: &mut [Combatant], rng: &mut SimRng| resolve_attack(
            actors, 0, 1, 0, false, 1.0, AttackMode::Normal, WeaponSlot::Primary, 1.0, None, rng);
        let log = strike(&mut live, &mut rng);
        let fast = strike(&mut bulk, &mut bulk_rng);
        assert_eq!(rng.next_u64(), bulk_rng.next_u64(), "seed {seed}");
        assert_eq!((fast.hit, fast.shield_block, fast.damage, fast.knockback_ft),
            (log.hit, log.shield_block, log.damage, log.knockback_ft));
        assert!(fast.damage_breakdown.is_none() && fast.shield_damage_breakdown.is_none());
        let direct = log.damage_breakdown.as_ref().map(|d|
            (d.raw_damage, (d.raw_damage - d.effective_armor_dr).max(0)));
        assert_eq!(fast.impact.direct, direct);
        let (armor, shield, broken) = if let Some(d) = log.damage_breakdown {
            seen[0] = true;
            (d.raw_damage.max(0).min(d.effective_armor_dr.max(0)), 0, false)
        } else if let Some(d) = log.shield_damage_breakdown {
            seen[1] = true;
            let shield = d.raw_damage.max(0).min(d.shield_dr.max(0));
            ((d.raw_damage.max(0) - shield).min(d.effective_armor_dr.max(0)), shield, d.shield_broken)
        } else {
            seen[2] = true;
            (0, 0, false)
        };
        assert_eq!((fast.impact.armor_prevented, fast.impact.shield_prevented, fast.impact.shield_broken),
            (armor as u32, shield as u32, broken));
    }
    assert_eq!(seen, [true; 3]);
}


#[test]
fn spell_fields_can_read_duel_positions_from_before_the_previous_move() {
    let mut sim = SimState::new(SimConfig::new(60.0, 1.0));
    sim.reset_with_combatants(vec![Combatant::default(), Combatant::new_with_team(Default::default(), 1)]);
    for _ in 0..3 {
        let before = [sim.actors[0].position, sim.actors[1].position];
        sim.tick();
        assert_eq!(sim.previous_positions.as_slice(), &before);
    }
}

#[test]
fn bulk_hook_selection_includes_alternate_weapons_and_tactical_profiles() {
    use crate::core::sim::{
        BowPace, CombatantTacticalProfile, ModifierOpI32, TacticalProfileKey, WeaponSetup, WeaponSetups,
    };
    use std::sync::Arc;

    let mut actor = Combatant::default();
    let baseline = BulkContext::new(std::slice::from_ref(&actor));
    assert!(!baseline.has_running_bow && !baseline.has_precognition);
    assert_eq!(baseline.possible_sheet_flags & StatIdI32::FlagSixPathsStyle.bit(), 0);

    let mut ranged = actor.clone();
    Arc::make_mut(&mut ranged.sheet.offense.weapon).ranged_styles.whistling_wind_pace = Some(BowPace::Run);
    let mut sheet = ranged.sheet.clone();
    sheet.defense.precognition = true;
    sheet.modifiers.add_i32(StatIdI32::FlagSixPathsStyle, ModifierOpI32::Set(1));
    ranged.tactical_profiles = vec![CombatantTacticalProfile {
        key: TacticalProfileKey { style_ids: vec!["six_paths".into()], use_jab: false, fight_defensively_penalty: None },
        sheet, weapon_group: ranged.weapon_group.clone(), armor_type: ranged.armor_type.clone(),
    }].into();
    actor.weapon_setups = Some(WeaponSetups {
        melee: actor.clone(), drawing: actor.clone(), ranged,
        start_with: WeaponSetup::Melee, melee_draw_seconds: 1.0, auto_switch_to_melee: true,
    }.into());
    let batch = BulkContext::new(&[actor]);
    assert!(batch.has_running_bow && batch.has_precognition);
    assert_ne!(batch.possible_sheet_flags & StatIdI32::FlagSixPathsStyle.bit(), 0);
}

#[test]
fn replacing_an_effect_updates_woad_expiry_and_streamline_membership() {
    use crate::core::sim::{STREAMLINE_EFFECT_ID, TemporaryEffect};
    let mut fighter = Combatant::default();
    let effects = &mut fighter.state.active_effects;
    effects.push(TemporaryEffect::new("ordinary", 2));
    assert!(!effects.capabilities().warrior_woad);
    effects[0] = TemporaryEffect::new(crate::core::sim::arborite::WOAD_EFFECT_ID, 1);
    assert!(effects.capabilities().warrior_woad);
    effects[0] = TemporaryEffect::new(STREAMLINE_EFFECT_ID, 1);
    assert!(!effects.capabilities().warrior_woad);
    assert!(effects.capabilities().average_damage);
    fighter.state.tick_effects();
    assert!(!fighter.state.active_effects.capabilities().average_damage);
}
