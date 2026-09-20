use super::magic::{on_attack_resolved, take_due_echoes};
use super::*;
use crate::core::magic::*;
use crate::core::rng::SimRng;
use std::sync::Arc;

fn caster() -> Combatant {
    let mut sheet = CombatantSheet::default();
    sheet.vitals.max_hp = 1000;
    sheet.vitals.threshold_of_pain = 1000;
    sheet.offense.attack_bonus = 100;
    Arc::make_mut(&mut sheet.offense.weapon).speed = 5.0;
    Arc::make_mut(&mut sheet.offense.weapon).reach_ft = 5.0;
    let mut actor = Combatant::new_with_team(sheet, 0);
    actor.configure_magic(MagicProfile {
        level: 10,
        loadout: MagicLoadout {
            knows_echo_strike: true,
            use_essence_costs: true,
            auto_cast: AutoCast::Manual,
            essences: vec![EssencePool {
                essence_id: "test".into(),
                proficiency: EssenceProficiency::V,
                capacity: 1000,
                current: 1000,
            }],
            ..MagicLoadout::default()
        },
        ..MagicProfile::default()
    });
    actor
}

fn target() -> Combatant {
    let mut actor = caster();
    actor.configure_magic(MagicProfile::default());
    actor.team_id = 1;
    actor.sheet.offense.attack_bonus = -100;
    actor.sheet.defense.defense_mod = -100;
    actor
}

fn rng() -> SimRng {
    SimRng::from_seed(250918)
}

fn test_spell(casting_seconds: u32, channelled: bool) -> SpellRequest {
    let mut buff = TemporaryEffect::new("test_spell_buff", 20);
    buff.modifiers
        .add_i32(StatIdI32::AttackBonus, ModifierOpI32::Add(3));
    SpellRequest {
        definition: SpellDefinition {
            id: "test_spell".into(),
            name: "Test spell".into(),
            level: 1,
            base_cost: 10,
            casting_seconds,
            components: SpellComponents {
                somatic: true,
                verbal: true,
            },
            channelled,
        },
        effect: SpellEffect::TimedBuff(buff),
        total_cost: 10,
        essence_index: 0,
    }
}

fn generic_caster() -> Combatant {
    let mut actor = caster();
    actor.magic.loadout.known_spells.push("test_spell".into());
    actor
}

#[test]
fn instant_spells_discharge_immediately_and_cannot_repeat_in_the_same_second() {
    let mut actor = generic_caster();
    actor.magic.talents.eliminate_spell_fatigue = true;
    actor
        .cast_spell(test_spell(0, false), 0, &mut rng())
        .unwrap();
    assert!(actor.state.magic.casting.is_none());
    assert!(actor.state.has_active_effect("test_spell_buff"));
    assert_eq!(actor.state.magic.essences[0].current, 990);
    assert_eq!(actor.cancel_spell(0), Err(MagicError::NoActiveCast));
    assert_eq!(
        actor.cast_spell(test_spell(0, false), 0, &mut rng()),
        Err(MagicError::InstantCastLimit)
    );
    actor
        .cast_spell(test_spell(0, false), 1, &mut rng())
        .unwrap();
    assert_eq!(actor.state.magic.essences[0].current, 980);
    assert_eq!(actor.state.active_effects.len(), 1); // refresh, never accidental stacking
}

#[test]
fn instant_spells_without_fatigue_talents_still_cause_five_second_fatigue() {
    let mut actor = generic_caster();
    actor
        .cast_spell(test_spell(0, false), 0, &mut rng())
        .unwrap();
    assert_eq!(actor.state.magic.fatigue, Some((1, 6)));
    assert_eq!(
        actor.cast_spell(test_spell(0, false), 1, &mut rng()),
        Err(MagicError::Busy)
    );
}

#[test]
fn channelling_delays_fatigue_until_stopped_and_spends_essence_only_once() {
    let mut actor = generic_caster();
    actor
        .cast_spell(test_spell(2, true), 0, &mut rng())
        .unwrap();
    actor.advance_magic(2, &mut rng());
    assert!(actor.state.magic.channeling.is_some());
    assert!(actor.state.magic.fatigue.is_none());
    assert!(!actor.magic_can_attack(WeaponSlot::Primary, 2.0));
    assert!(actor.state.magic.restricted_defense(actor.magic.talents));
    for now in 3..30 {
        actor.state.tick_effects();
        actor.advance_magic(now, &mut rng());
        assert!(actor.state.has_active_effect("test_spell_buff"));
    }
    actor.stop_channeling(30).unwrap();
    assert!(!actor.state.has_active_effect("test_spell_buff"));
    assert_eq!(actor.state.magic.essences[0].current, 990);
    assert_eq!(actor.state.magic.fatigue, Some((31, 38)));
}

#[test]
fn disrupted_channel_has_no_second_essence_cost() {
    let mut actor = generic_caster();
    actor
        .cast_spell(test_spell(1, true), 0, &mut rng())
        .unwrap();
    actor.advance_magic(1, &mut rng());
    actor.interrupt_spell(3);
    assert!(actor.state.magic.channeling.is_none());
    assert_eq!(actor.state.magic.essences[0].current, 990);
    assert!(!actor.state.has_active_effect("test_spell_buff"));
    assert_eq!(actor.state.magic.fatigue, Some((4, 10)));
}

#[test]
fn verbal_component_is_enforced_at_completion_and_silent_casting_removes_it() {
    for silent in [false, true] {
        let mut actor = generic_caster();
        actor.magic.loadout.silenced = true;
        actor.magic.talents.silent_casting = silent;
        actor
            .cast_spell(test_spell(3, false), 0, &mut rng())
            .unwrap();
        actor.advance_magic(2, &mut rng());
        assert!(actor.state.magic.casting.is_some());
        actor.advance_magic(3, &mut rng());
        assert_eq!(actor.state.has_active_effect("test_spell_buff"), silent);
        assert_eq!(actor.state.magic.essences[0].current, 990);
    }
}

#[test]
fn generic_buff_dismissal_cannot_remove_an_unowned_effect() {
    let mut actor = generic_caster();
    actor
        .state
        .add_effect(TemporaryEffect::new("other_casters_buff", 30));
    actor
        .cast_spell(test_spell(0, false), 0, &mut rng())
        .unwrap();
    assert_eq!(
        actor.dismiss_spell("other_casters_buff", 1),
        Err(MagicError::NoActiveSpell)
    );
    actor.dismiss_spell("test_spell_buff", 1).unwrap();
    assert!(!actor.state.has_active_effect("test_spell_buff"));
    assert!(actor.state.has_active_effect("other_casters_buff"));
}

#[test]
fn malformed_spell_requests_cannot_override_echo_level_cost_or_components() {
    let mut actor = caster();
    let mut request = SpellRequest::echo_strike(EchoStrikeOptions::default(), 0).unwrap();
    request.definition.casting_seconds = 0;
    assert_eq!(
        actor.cast_spell(request, 0, &mut rng()),
        Err(MagicError::InvalidSpell)
    );
    let mut request = SpellRequest::echo_strike(EchoStrikeOptions::default(), 0).unwrap();
    request.total_cost = 1;
    assert_eq!(
        actor.cast_spell(request, 0, &mut rng()),
        Err(MagicError::InvalidSpell)
    );
    assert_eq!(actor.state.magic.essences[0].current, 1000);
}

#[test]
fn fatigue_skill_and_action_penalties_are_removed_by_decimate() {
    let mut actor = caster();
    arm(&mut actor);
    actor.advance_magic(2, &mut rng());
    assert_eq!(actor.spell_fatigue_skill_penalty_percent(), 30);
    assert_eq!(actor.spell_fatigue_action_seconds(3), 6);
    actor.magic.talents.decimate_spell_fatigue = true;
    assert_eq!(actor.spell_fatigue_skill_penalty_percent(), 0);
    assert_eq!(actor.spell_fatigue_action_seconds(3), 3);
}

#[test]
fn casting_and_fatigue_movement_respect_walking_and_running_limits() {
    let mut actor = generic_caster();
    actor.sheet.mobility.move_speed = 20.0;
    actor
        .cast_spell(test_spell(2, true), 0, &mut rng())
        .unwrap();
    assert_eq!(actor.apply_f32(StatIdF32::MoveSpeed, 20.0), 5.0);
    actor.advance_magic(2, &mut rng());
    assert_eq!(actor.apply_f32(StatIdF32::MoveSpeed, 20.0), 5.0);
    actor.stop_channeling(3).unwrap();
    actor.advance_magic(4, &mut rng());
    assert_eq!(actor.apply_f32(StatIdF32::MoveSpeed, 20.0), 5.0); // half jog; no running
    assert!(actor.magic_blocks_running());
    actor.magic.talents.mitigate_spell_fatigue = true;
    assert_eq!(actor.apply_f32(StatIdF32::MoveSpeed, 20.0), 10.0);
    actor.magic.talents.decimate_spell_fatigue = true;
    assert_eq!(actor.apply_f32(StatIdF32::MoveSpeed, 20.0), 20.0);
    assert!(!actor.magic_blocks_running());
}

#[test]
fn an_echo_does_not_discharge_a_newly_armed_echo_buff() {
    let mut actors = vec![caster(), target()];
    actors[0].magic.talents.eliminate_spell_fatigue = true;
    arm(&mut actors[0]);
    strike(&mut actors, 2, true, 20);
    actors[0].cast_echo_strike(3).unwrap();
    actors[0].advance_magic(4, &mut rng());
    let echo = take_due_echoes(&mut actors, 12).remove(0).1;
    super::combat::resolve_echo(&mut actors, 0, &echo, 5.0, 12, &mut rng())
        .unwrap()
        .0;
    assert!(actors[0].state.magic.armed_echo.is_some());
    assert!(actors[0].state.magic.echoes.is_empty());
}

#[test]
fn magic_focus_and_resistances_are_applied_to_the_correct_participant() {
    let mut actor = caster();
    actor.magic.talents.focus_tier = 3;
    actor.magic.talents.charm_resistant = true;
    actor.magic.saves.mental = 4;
    assert_eq!(actor.caster_magic_save_bonus(19), 14);
    assert_eq!(
        actor.magic_save_bonus(
            MagicSaveKind::Mental,
            SpellTags {
                charm: true,
                ..SpellTags::default()
            }
        ),
        16
    );
    assert_eq!(
        actor.magic_save_bonus(MagicSaveKind::Mental, SpellTags::default()),
        4
    );
}

#[test]
fn echoes_keep_existing_magical_precognition_mitigation() {
    let mut actors = vec![caster(), target()];
    arm(&mut actors[0]);
    strike(&mut actors, 2, true, 19);
    let echo = take_due_echoes(&mut actors, 12).remove(0).1;
    actors[1].sheet.defense.precognition = true;
    actors[1].sheet.defense.feat_of_agility = 10000;
    actors[1].state.precognition_space_available = true;
    let (attack, evaded) =
        super::combat::resolve_echo(&mut actors, 0, &echo, 5.0, 12, &mut rng()).unwrap();
    assert!(evaded);
    assert_eq!(attack.damage, 4); // 19 / 2 echo, then magical evasion halves 9.
}

#[test]
fn manual_cast_satisfies_the_once_per_fight_auto_cast_rule() {
    let mut actor = caster();
    actor.magic.loadout.auto_cast = AutoCast::AtStart;
    actor.cast_echo_strike(0).unwrap();
    actor.try_auto_cast(0, true);
    assert_eq!(actor.state.magic.events.len(), 1);
}

#[test]
fn in_reach_auto_cast_waits_for_its_condition_and_uses_only_one_attempt() {
    let mut actor = caster();
    actor.magic.loadout.auto_cast = AutoCast::InWeaponReach;
    actor.try_auto_cast(0, false);
    assert!(actor.state.magic.casting.is_none());
    actor.try_auto_cast(1, true);
    assert!(actor.state.magic.casting.is_some());
    actor.advance_magic(2, &mut rng());
    actor.advance_magic(20, &mut rng());
    actor.try_auto_cast(20, true);
    assert!(actor.state.magic.casting.is_none());
    assert_eq!(actor.state.magic.essences[0].current, 850);
}

#[test]
fn due_echo_order_is_stable_and_is_sorted_by_time_before_caster() {
    let mut actors = vec![caster(), caster(), target()];
    arm(&mut actors[0]);
    arm(&mut actors[1]);
    strike(&mut actors, 4, true, 20);
    let mut earlier = actors[0].state.magic.echoes[0].clone();
    earlier.due_at = 12;
    earlier.target = 2;
    actors[1].state.magic.echoes.push(earlier);
    let due = take_due_echoes(&mut actors, 14);
    assert_eq!(
        due.iter()
            .map(|(caster, echo)| (*caster, echo.due_at))
            .collect::<Vec<_>>(),
        vec![(1, 12), (0, 14)]
    );
}

fn arm(actor: &mut Combatant) {
    actor.cast_echo_strike(0).unwrap();
    actor.advance_magic(1, &mut rng());
    assert!(actor.state.magic.armed_echo.is_some());
}

fn strike(actors: &mut [Combatant], time: u32, hit: bool, damage: i32) {
    let roll = AttackRollBreakdown {
        attack_die: 10,
        defense_die: 10,
        attack_bonus: 100,
        range_mod: 0,
        defense_base: -100,
        weapon_defense_bonus: 0,
        shield_defense_bonus: 0,
        attack_total: 110,
        defense_total: -90,
    };
    on_attack_resolved(
        actors,
        0,
        1,
        time as f32,
        hit,
        damage,
        WeaponSlot::Primary,
        false,
        &roll,
    );
}

#[test]
fn casting_reserves_essence_then_spends_once_and_arms_at_completion() {
    let mut actor = caster();
    actor.cast_echo_strike(0).unwrap();
    assert_eq!(actor.state.magic.essences[0].current, 1000);
    actor.advance_magic(0, &mut rng());
    assert!(actor.state.magic.armed_echo.is_none());
    actor.advance_magic(1, &mut rng());
    assert_eq!(actor.state.magic.essences[0].current, 850);
    assert_eq!(
        actor.state.magic.armed_echo.as_ref().unwrap().expires_at,
        16
    );
    actor.advance_magic(1, &mut rng());
    assert_eq!(actor.state.magic.essences[0].current, 850);
    assert!(actor.state.magic.casting.is_none());
}

#[test]
fn rejected_casts_do_not_spend_essence_or_change_combat_state() {
    let mut actor = caster();
    actor.magic.level = 9;
    assert_eq!(
        actor.cast_echo_strike(0),
        Err(MagicError::SpellLevelTooHigh)
    );
    actor.magic.level = 10;
    actor.state.magic.essences[0].current = 149;
    assert_eq!(
        actor.cast_echo_strike(0),
        Err(MagicError::InsufficientEssence)
    );
    assert_eq!(actor.state.magic.essences[0].current, 149);
    assert!(actor.state.magic.casting.is_none());
    assert!(actor.state.magic.fatigue.is_none());
    assert!(actor.state.magic.events.is_empty());
}

#[test]
fn dead_incapacitated_and_passive_characters_cannot_cast() {
    for condition in 0..3 {
        let mut actor = caster();
        match condition {
            0 => actor.state.hp = 0,
            1 => actor.state.trauma_remaining_seconds = 1,
            _ => actor.sheet.maneuvers.passive = true,
        }
        assert_eq!(actor.cast_echo_strike(0), Err(MagicError::Incapacitated));
    }
}

#[test]
fn fatigue_starts_next_second_and_weapon_recovery_survives_timer_clearing() {
    let mut actor = caster();
    arm(&mut actor);
    assert!(!actor.state.magic.fatigued());
    assert_eq!(actor.state.magic.fatigue, Some((2, 8)));
    assert_eq!(actor.cast_echo_strike(1), Err(MagicError::Busy));
    actor.advance_magic(2, &mut rng());
    assert!(actor.state.magic.fatigued());
    actor.advance_magic(8, &mut rng());
    assert!(!actor.state.magic.fatigued());
    actor.state.clear_attack_timers();
    assert!(!actor.magic_can_attack(WeaponSlot::Primary, 12.0));
    assert!(actor.magic_can_attack(WeaponSlot::Primary, 13.0));
}

#[test]
fn diminish_five_leaves_only_casting_time_fatigue() {
    let mut actor = caster();
    actor.magic.talents.diminish_spell_fatigue = 5;
    arm(&mut actor);
    assert_eq!(actor.state.magic.fatigue, Some((2, 3)));
    assert_eq!(actor.state.magic.primary_recovery_until, 8.0);
}

#[test]
fn decimate_allows_recovered_weapon_attacks_but_not_casting_during_fatigue() {
    let mut actor = caster();
    actor.magic.talents.decimate_spell_fatigue = true;
    arm(&mut actor);
    actor.advance_magic(6, &mut rng());
    assert!(actor.state.magic.fatigued());
    assert_eq!(actor.state.magic.defense_penalty(actor.magic.talents), 0);
    assert!(actor.magic_can_attack(WeaponSlot::Primary, 6.0));
    assert_eq!(actor.cast_echo_strike(6), Err(MagicError::Busy));
}

#[test]
fn eliminate_allows_back_to_back_casts_but_still_resets_weapon_speed() {
    let mut actor = caster();
    actor.magic.talents.eliminate_spell_fatigue = true;
    arm(&mut actor);
    assert!(actor.state.magic.fatigue.is_none());
    assert!(!actor.magic_can_attack(WeaponSlot::Primary, 1.0));
    actor.cast_echo_strike(1).unwrap();
    actor.advance_magic(2, &mut rng());
    assert_eq!(actor.state.magic.essences[0].current, 700);
    assert_eq!(actor.state.magic.primary_recovery_until, 7.0);
}

#[test]
fn cancellation_after_time_advances_preserves_essence_and_gives_five_second_fatigue() {
    let mut actor = caster();
    actor.magic.loadout.known_spells.push("test_spell".into());
    actor
        .cast_spell(test_spell(5, false), 0, &mut rng())
        .unwrap();
    actor.advance_magic(1, &mut rng());
    actor.cancel_spell(1).unwrap();
    assert_eq!(actor.state.magic.essences[0].current, 1000);
    assert_eq!(actor.state.magic.fatigue, Some((2, 7)));
    actor.advance_magic(2, &mut rng());
    assert!(!actor.state.has_active_effect("test_spell_buff"));
    assert_eq!(actor.cancel_spell(2), Err(MagicError::NoActiveCast));
}

#[test]
fn same_second_cancellation_preserves_existing_weapon_recovery_without_fatigue() {
    for now in [0, 20] {
        let mut actor = caster();
        actor.state.magic.primary_recovery_until = 40.0;
        actor.state.magic.secondary_recovery_until = 45.0;
        actor.state.next_attack_time_primary = Some(42.0);
        actor.state.next_attack_time_secondary = Some(47.0);
        actor.cast_echo_strike(now).unwrap();
        actor.cancel_spell(now).unwrap();
        assert!(actor.state.magic.casting.is_none());
        assert!(actor.state.magic.fatigue.is_none());
        assert_eq!(actor.state.magic.essences[0].current, 1000);
        assert_eq!(actor.state.magic.primary_recovery_until, 40.0);
        assert_eq!(actor.state.magic.secondary_recovery_until, 45.0);
        assert_eq!(actor.state.next_attack_time_primary, Some(42.0));
        assert_eq!(actor.state.next_attack_time_secondary, Some(47.0));
        actor.advance_magic(now + 1, &mut rng());
        assert!(actor.state.magic.fatigue.is_none());
        assert!(actor.state.magic.armed_echo.is_none());
        assert!(actor.cast_echo_strike(now + 1).is_ok());
    }
}

#[test]
fn interruption_spends_essence_once_and_never_arms_the_spell() {
    let mut actor = caster();
    actor.cast_echo_strike(0).unwrap();
    actor.interrupt_spell(0);
    actor.interrupt_spell(0);
    actor.advance_magic(1, &mut rng());
    assert_eq!(actor.state.magic.essences[0].current, 850);
    assert!(actor.state.magic.armed_echo.is_none());
    assert!(
        actor
            .state
            .magic
            .events
            .iter()
            .any(|event| event.kind == SpellEventKind::CastInterrupted)
    );
}

#[test]
fn somatic_restriction_during_cast_interrupts_unless_still_casting() {
    for still in [false, true] {
        let mut actor = caster();
        actor.magic.talents.still_casting = still;
        actor.cast_echo_strike(0).unwrap();
        actor.magic.loadout.arms_restricted = true;
        actor.advance_magic(1, &mut rng());
        assert_eq!(actor.state.magic.armed_echo.is_some(), still);
        assert_eq!(actor.state.magic.essences[0].current, 850);
    }
}

#[test]
fn equipment_failure_spends_essence_and_causes_fatigue() {
    let mut actor = caster();
    actor.magic.loadout.encumbrance = Encumbrance::Heavy;
    actor.sheet.defense.armor_is_heavy = true;
    actor.cast_echo_strike(0).unwrap();
    actor.advance_magic(1, &mut rng());
    assert_eq!(actor.state.magic.essences[0].current, 850);
    assert!(actor.state.magic.armed_echo.is_none());
    assert!(actor.state.magic.fatigue.is_some());
    assert!(
        actor
            .state
            .magic
            .events
            .iter()
            .any(|event| event.kind == SpellEventKind::CastFailed)
    );
}

#[test]
fn combat_casting_restores_normal_defense_die_in_actual_attack_resolution() {
    let mut ordinary = vec![target(), caster()];
    ordinary[0].sheet.offense.attack_bonus = -1000;
    let mut casting = ordinary.clone();
    casting[1].cast_echo_strike(0).unwrap();
    let mut talented = casting.clone();
    talented[1].magic.talents.combat_casting = true;
    for seed in 0..32 {
        let run = |actors: &mut Vec<Combatant>| {
            resolve_basic_attack(
                actors,
                0,
                1,
                0,
                false,
                5.0,
                0.0,
                &mut SimRng::from_seed(seed),
            )
            .event
            .roll
        };
        let normal_roll = run(&mut ordinary.clone());
        let talented_roll = run(&mut talented.clone());
        assert_eq!(normal_roll.defense_die, talented_roll.defense_die);
        let mut expected = SimRng::from_seed(seed);
        crate::core::rules::penetrating_roll(20, &mut expected);
        let expected_defense = crate::core::rules::penetrating_roll(8, &mut expected);
        assert_eq!(run(&mut casting.clone()).defense_die, expected_defense);
    }
}

#[test]
fn fatigue_penalty_applies_to_ranged_shield_defense() {
    let mut plain = vec![target(), caster()];
    plain[1].sheet.defense.shield_name = Some("Shield".into());
    plain[1].state.shield_intact = true;
    plain[1].sheet.defense.shield_defense_bonus = 10;
    plain[1].sheet.defense.ranged_defense_mod = 0;
    let mut fatigued = plain.clone();
    fatigued[1].state.magic.fatigue = Some((0, 6));
    let base = resolve_basic_attack(&mut plain, 0, 1, 0, true, 10.0, 0.0, &mut rng()).event;
    let penalty = resolve_basic_attack(&mut fatigued, 0, 1, 0, true, 10.0, 0.0, &mut rng()).event;
    assert_eq!(penalty.roll.defense_total, base.roll.defense_total - 6);
}

#[test]
fn misses_do_not_consume_buff_and_first_hit_records_rounded_down_damage() {
    let mut actors = vec![caster(), target()];
    arm(&mut actors[0]);
    strike(&mut actors, 3, false, 0);
    assert!(actors[0].state.magic.armed_echo.is_some());
    strike(&mut actors, 4, true, 19);
    strike(&mut actors, 5, true, 100);
    let echoes = &actors[0].state.magic.echoes;
    assert_eq!(echoes.len(), 1);
    assert_eq!(echoes[0].due_at, 14);
    assert_eq!(echoes[0].damage, 9);
    assert!(actors[0].state.magic.armed_echo.is_none());
}

#[test]
fn a_successful_zero_damage_attack_still_discharges_the_spell() {
    let mut actors = vec![caster(), target()];
    arm(&mut actors[0]);
    strike(&mut actors, 4, true, 0);
    assert!(actors[0].state.magic.armed_echo.is_none());
    assert_eq!(actors[0].state.magic.echoes[0].damage, 0);
}

#[test]
fn expiry_boundary_excludes_hits_at_expiration_but_pending_echoes_survive_it() {
    let mut actors = vec![caster(), target()];
    arm(&mut actors[0]);
    let mut expired = actors.clone();
    expired[0].advance_magic(16, &mut rng());
    strike(&mut expired, 16, true, 20);
    assert!(expired[0].state.magic.echoes.is_empty());
    strike(&mut actors, 15, true, 20);
    actors[0].advance_magic(16, &mut rng());
    assert!(take_due_echoes(&mut actors, 24).is_empty());
    assert_eq!(take_due_echoes(&mut actors, 25).len(), 1);
}

#[test]
fn additional_echoes_use_intervals_and_cumulative_defense_even_after_misses() {
    let mut actors = vec![caster(), target()];
    actors[0].magic.loadout.echo_strike.additional_echoes = 2;
    arm(&mut actors[0]);
    strike(&mut actors, 4, true, 19);
    assert_eq!(actors[0].state.magic.essences[0].current, 550);
    assert_eq!(
        actors[0]
            .state
            .magic
            .echoes
            .iter()
            .map(|echo| echo.due_at)
            .collect::<Vec<_>>(),
        vec![14, 24, 34]
    );
    let echoes = take_due_echoes(&mut actors, 34);
    let mut defense_totals = Vec::new();
    actors[0].sheet.offense.attack_bonus = -1000; // all three miss
    for (_, echo) in echoes {
        let result =
            super::combat::resolve_echo(&mut actors, 0, &echo, 5.0, echo.due_at, &mut rng())
                .unwrap()
                .0;
        assert!(!result.hit);
        defense_totals.push(result.roll.defense_total);
    }
    assert_eq!(defense_totals[1], defense_totals[0] + 2);
    assert_eq!(defense_totals[2], defense_totals[0] + 4);
}

#[test]
fn echo_damage_is_fixed_and_bypasses_mundane_mitigation_without_retriggering() {
    let mut actors = vec![caster(), target()];
    arm(&mut actors[0]);
    strike(&mut actors, 4, true, 19);
    let echo = take_due_echoes(&mut actors, 14).remove(0).1;
    actors[1].sheet.defense.armor_dr = 100;
    actors[1].sheet.defense.natural_dr = 100;
    actors[1].sheet.defense.shield_dr = 100;
    actors[1].sheet.defense.shield_name = Some("Shield".into());
    actors[1].state.shield_intact = true;
    let timers = (
        actors[0].state.next_attack_time_primary,
        actors[0].state.next_attack_time_secondary,
    );
    let result = super::combat::resolve_echo(&mut actors, 0, &echo, 5.0, 14, &mut rng())
        .unwrap()
        .0;
    assert!(result.hit);
    assert_eq!(result.damage, 9);
    assert_eq!(actors[1].state.hp, 991);
    assert_eq!(result.knockback_ft, 0.0);
    assert!(result.critical.is_none());
    assert_eq!(result.shield_damage, 0);
    assert_eq!(result.source, AttackSource::EchoStrike);
    assert_eq!(
        timers,
        (
            actors[0].state.next_attack_time_primary,
            actors[0].state.next_attack_time_secondary
        )
    );
    assert!(actors[0].state.magic.echoes.is_empty());
}

#[test]
fn full_damage_amplification_preserves_the_recorded_wound() {
    let mut actors = vec![caster(), target()];
    actors[0].magic.loadout.echo_strike.full_damage = true;
    arm(&mut actors[0]);
    strike(&mut actors, 4, true, 19);
    assert_eq!(actors[0].state.magic.echoes[0].damage, 19);
    assert_eq!(actors[0].state.magic.essences[0].current, 650);
}

#[test]
fn each_cast_starts_its_defense_counter_at_zero_and_does_not_replace_pending_echoes() {
    let mut actors = vec![caster(), target()];
    actors[0].magic.talents.eliminate_spell_fatigue = true;
    arm(&mut actors[0]);
    strike(&mut actors, 2, true, 20);
    actors[0].cast_echo_strike(3).unwrap();
    actors[0].advance_magic(4, &mut rng());
    strike(&mut actors, 5, true, 30);
    let echoes = &actors[0].state.magic.echoes;
    assert_eq!(echoes.len(), 2);
    assert_eq!((echoes[0].ordinal, echoes[1].ordinal), (0, 0));
    assert_ne!(echoes[0].cast_id, echoes[1].cast_id);
}

#[test]
fn echoes_follow_configured_death_and_range_rules_and_never_retarget() {
    let mut actors = vec![caster(), target()];
    arm(&mut actors[0]);
    strike(&mut actors, 2, true, 20);
    let echo = take_due_echoes(&mut actors, 12).remove(0).1;
    actors[0].state.hp = 0;
    assert!(super::combat::resolve_echo(&mut actors, 0, &echo, 500.0, 12, &mut rng()).is_some());
    actors[0].magic.loadout.echo_requires_living_caster = true;
    assert!(super::combat::resolve_echo(&mut actors, 0, &echo, 5.0, 12, &mut rng()).is_none());
    actors[0].state.hp = 100;
    actors[0].magic.loadout.echo_requires_weapon_range = true;
    assert!(super::combat::resolve_echo(&mut actors, 0, &echo, 500.0, 12, &mut rng()).is_none());
    actors[1].state.hp = 0;
    actors.push(target());
    assert!(super::combat::resolve_echo(&mut actors, 0, &echo, 5.0, 12, &mut rng()).is_none());
    assert_eq!(actors[2].state.hp, 1000);
}

#[test]
fn dismiss_clears_active_buff_and_scheduled_echoes_but_requires_a_gesture() {
    let mut actors = vec![caster(), target()];
    arm(&mut actors[0]);
    strike(&mut actors, 2, true, 20);
    actors[0].magic.loadout.arms_restricted = true;
    assert_eq!(
        actors[0].dismiss_echo_strike(3),
        Err(MagicError::SomaticComponentBlocked)
    );
    actors[0].magic.loadout.arms_restricted = false;
    actors[0].dismiss_echo_strike(3).unwrap();
    assert!(actors[0].state.magic.echoes.is_empty());
    assert_eq!(actors[0].state.magic.essences[0].current, 850);
}

#[test]
fn resetting_combat_restores_initial_essence_and_clears_magic_state() {
    let mut actor = caster();
    arm(&mut actor);
    actor.reset_state();
    assert_eq!(actor.state.magic.essences[0].current, 1000);
    assert!(actor.state.magic.armed_echo.is_none());
    assert!(actor.state.magic.fatigue.is_none());
    assert!(actor.state.magic.events.is_empty());
}

fn duel(mut actor: Combatant) -> SimState {
    actor.sheet.mobility.move_speed = 0.0;
    let mut enemy = target();
    enemy.sheet.maneuvers.passive = true;
    let mut sim = SimState::with_rng(
        SimConfig {
            start_distance: 5.0,
            tile_size_ft: 1.0,
            ..SimConfig::new(5.0, 5.0)
        },
        rng(),
    );
    sim.reset_with_combatants(vec![actor, enemy]);
    sim
}

#[test]
fn duel_autocast_produces_real_spell_and_echo_events_and_replays_deterministically() {
    let run = || {
        let mut actor = caster();
        actor.magic.loadout.auto_cast = AutoCast::AtStart;
        let mut sim = duel(actor);
        for _ in 0..40 {
            sim.tick();
        }
        let events = sim
            .combat_events
            .iter()
            .map(|event| crate::sim::format_combat_event_line(event, &sim.combatants))
            .collect::<Vec<_>>();
        assert!(sim.combat_events.iter().any(|event| matches!(&event.kind, CombatEventKind::Spell(spell) if spell.kind == SpellEventKind::CastCompleted)));
        assert!(sim.combat_events.iter().any(|event| matches!(&event.kind, CombatEventKind::Attack(attack) if attack.source == AttackSource::EchoStrike)));
        assert_eq!(sim.combatants[0].state.magic.essences[0].current, 850);
        events
    };
    assert_eq!(run(), run());
}

#[test]
fn failed_autocast_logs_once_and_does_not_loop_or_stop_normal_combat() {
    let mut actor = caster();
    actor.magic.loadout.auto_cast = AutoCast::AtStart;
    actor.magic.loadout.essences[0].current = 0;
    let mut sim = duel(actor);
    for _ in 0..20 {
        sim.tick();
    }
    assert_eq!(sim.combat_events.iter().filter(|event| matches!(&event.kind, CombatEventKind::Spell(spell) if spell.kind == SpellEventKind::CastRejected)).count(), 1);
    assert!(
        sim.combat_events
            .iter()
            .any(|event| matches!(&event.kind, CombatEventKind::Attack(_)))
    );
}

#[test]
fn combat_waits_for_pending_echo_after_caster_death() {
    let mut sim = duel(caster());
    arm(&mut sim.combatants[0]);
    strike(&mut sim.combatants, 2, true, 20);
    sim.combatants[0].state.hp = 0;
    for _ in 0..12 {
        sim.tick();
        assert!(!sim.done);
    }
    sim.tick();
    assert!(sim.done);
    assert_eq!(sim.combatants[1].state.hp, 990);
}

#[test]
fn squad_and_duel_apply_the_same_cast_and_fatigue_timeline() {
    use crate::squad_battler::combat::{BattleUnit, SquadCombat};
    let mut sim = duel(caster());
    let mut enemy = target();
    enemy.sheet.maneuvers.passive = true;
    let mut squad = SquadCombat::new_with_seed(
        vec![BattleUnit::from_combatant("caster", 0, caster())],
        vec![BattleUnit::from_combatant("enemy", 1, enemy)],
        123,
    );
    sim.cast_echo_strike(0).unwrap();
    squad.cast_echo_strike(0).unwrap();
    sim.tick(); // Duel processes t=0; squad begins processing at t=1.
    for _ in 0..10 {
        sim.tick();
        squad.tick();
        let left = &sim.combatants[0].state.magic;
        let right = &squad.units[0].combatant.as_ref().unwrap().state.magic;
        assert_eq!(left.essences, right.essences);
        assert_eq!(left.fatigue, right.fatigue);
        assert_eq!(left.primary_recovery_until, right.primary_recovery_until);
    }
}

#[test]
fn squad_echoes_damage_the_original_target_and_do_not_retarget_or_reset_initiative() {
    use crate::squad_battler::combat::{BattleUnit, SquadCombat};
    let mut actors = vec![caster(), target()];
    arm(&mut actors[0]);
    strike(&mut actors, 2, true, 19);
    actors[0].sheet.maneuvers.passive = true;
    actors[1].sheet.maneuvers.passive = true;
    let mut squad = SquadCombat::new_with_seed(
        vec![BattleUnit::from_combatant("caster", 0, actors.remove(0))],
        vec![BattleUnit::from_combatant("enemy", 1, actors.remove(0))],
        123,
    );
    squad.units[0].initiative_ready_at = 50.0;
    for _ in 0..12 {
        squad.tick();
    }
    assert_eq!(squad.units[1].hp, 991);
    assert_eq!(squad.units[0].initiative_ready_at, 50.0);
    assert!(squad.log.iter().any(|line| line.contains("Echo Strike vs")));
}

fn free_caster(spells: &[&str]) -> Combatant {
    let mut actor = caster();
    let mut loadout = MagicLoadout::default();
    for id in spells {
        loadout.learn_spell(id);
    }
    actor.configure_magic(MagicProfile {
        level: 10,
        loadout,
        ..MagicProfile::default()
    });
    actor
}

#[test]
fn free_empowered_echo_needs_no_pool_and_ignores_ep_caps() {
    let mut actor = free_caster(&["echo_strike"]);
    actor.magic.loadout.echo_strike = EchoStrikeOptions {
        extra_duration_seconds: 60,
        delay_seconds: 1,
        additional_echoes: 4,
        full_damage: true,
    };
    assert_eq!(actor.validate_echo_cast(), Ok(0));
    actor.cast_known_spell("echo_strike", 0).unwrap();
    actor.advance_magic(1, &mut rng());
    let buff = actor.state.magic.armed_echo.as_ref().unwrap();
    assert_eq!(buff.options.additional_echoes, 4);
    assert_eq!(buff.expires_at, 76);
    assert!(buff.options.full_damage);
    assert!(actor.state.magic.essences.is_empty());
    assert!(
        actor
            .state
            .magic
            .events
            .iter()
            .all(|event| event.essence_spent == 0)
    );
}

#[test]
fn free_cast_does_not_modify_saved_essence_even_on_interruption() {
    let mut actor = caster();
    actor.magic.loadout.use_essence_costs = false;
    actor.state.magic.essences[0].current = 0;
    actor.cast_echo_strike(0).unwrap();
    actor.interrupt_spell(0);
    assert_eq!(actor.state.magic.essences[0].current, 0);
    assert_eq!(actor.state.magic.events.last().unwrap().essence_spent, 0);
    assert!(actor.state.magic.fatigue.is_some());
}

#[test]
fn assigned_spells_ignore_level_but_still_require_components_and_knowledge() {
    let mut actor = free_caster(&["spell_chronoblur"]);
    actor.magic.level = 3;
    let mut allowed = actor.clone();
    assert!(allowed.cast_known_spell("spell_chronoblur", 0).is_ok());
    actor.magic.loadout.arms_restricted = true;
    assert_eq!(
        actor.cast_known_spell("spell_chronoblur", 0),
        Err(MagicError::SomaticComponentBlocked)
    );
    actor.magic.loadout.arms_restricted = false;
    actor.magic.loadout.forget_spell("spell_chronoblur");
    assert_eq!(
        actor.cast_known_spell("spell_chronoblur", 0),
        Err(MagicError::UnknownSpell)
    );
}

#[test]
fn chronoblur_uses_two_second_cast_empowered_duration_and_fatigue() {
    let mut actor = free_caster(&["spell_chronoblur"]);
    actor.magic.loadout.chronoblur_duration_ranks = 3;
    actor.cast_known_spell("spell_chronoblur", 10).unwrap();
    actor.advance_magic(11, &mut rng());
    assert!(!actor.state.has_active_effect(CHRONOBLUR_EFFECT_ID));
    actor.advance_magic(12, &mut rng());
    assert_eq!(
        actor
            .state
            .active_effects
            .iter()
            .find(|e| e.id == CHRONOBLUR_EFFECT_ID)
            .unwrap()
            .remaining_seconds,
        150
    );
    assert_eq!(actor.state.magic.fatigue, Some((13, 20)));
}

#[test]
fn streamline_uses_five_second_cast_and_empowers_duration_and_radius() {
    let mut actor = free_caster(&["spell_streamline"]);
    actor.magic.loadout.streamline_duration_ranks = 2;
    actor.magic.loadout.streamline_radius_ranks = 3;
    actor.cast_known_spell("spell_streamline", 0).unwrap();
    actor.advance_magic(4, &mut rng());
    assert!(!actor.state.has_active_effect(STREAMLINE_EFFECT_ID));
    actor.advance_magic(5, &mut rng());
    assert_eq!(
        actor
            .state
            .active_effects
            .iter()
            .find(|e| e.id == STREAMLINE_EFFECT_ID)
            .unwrap()
            .remaining_seconds,
        420
    );
    assert_eq!(actor.apply_f32(StatIdF32::StreamlineRadius, 30.0), 60.0);
    assert_eq!(actor.state.magic.fatigue, Some((6, 16)));
}

#[test]
fn new_buff_spells_check_verbal_components_at_completion() {
    for (id, completes) in [("spell_chronoblur", 2), ("spell_streamline", 5)] {
        let mut actor = free_caster(&[id]);
        actor.cast_known_spell(id, 0).unwrap();
        actor.magic.loadout.silenced = true;
        actor.advance_magic(completes, &mut rng());
        assert!(!actor.state.has_active_effect(id));
        assert!(
            actor
                .state
                .magic
                .events
                .iter()
                .any(|e| e.kind == SpellEventKind::CastInterrupted)
        );
    }
}

#[test]
fn fatigue_talents_apply_to_new_spell_catalog_casts() {
    let mut actor = free_caster(&["spell_chronoblur"]);
    actor.magic.talents.eliminate_spell_fatigue = true;
    actor.cast_known_spell("spell_chronoblur", 0).unwrap();
    actor.advance_magic(2, &mut rng());
    assert!(actor.state.magic.fatigue.is_none());
    assert_eq!(actor.state.magic.primary_recovery_until, 7.0);
}

#[test]
fn spell_ai_manual_does_not_cast_and_fight_start_casts_without_reach() {
    let mut actor = free_caster(&["echo_strike"]);
    actor
        .magic
        .loadout
        .spell_ai
        .insert("echo_strike".into(), SpellCastAi::Manual);
    actor.try_auto_cast(0, true);
    assert!(actor.state.magic.casting.is_none());
    actor
        .magic
        .loadout
        .spell_ai
        .insert("echo_strike".into(), SpellCastAi::AtFightStart);
    actor.try_auto_cast(1, false);
    assert!(actor.state.magic.casting.is_some());
    actor.advance_magic(2, &mut rng());
    actor.advance_magic(30, &mut rng());
    actor.try_auto_cast(30, true);
    assert!(actor.state.magic.casting.is_none());
}

#[test]
fn spell_ai_repeat_waits_for_buff_expiry_and_recasts_without_ep() {
    let mut actor = free_caster(&["echo_strike"]);
    actor
        .magic
        .loadout
        .spell_ai
        .insert("echo_strike".into(), SpellCastAi::AsOftenAsPossible);
    actor.try_auto_cast(0, false);
    actor.advance_magic(1, &mut rng());
    actor.advance_magic(8, &mut rng());
    actor.try_auto_cast(8, false);
    assert!(actor.state.magic.casting.is_none());
    actor.advance_magic(16, &mut rng());
    actor.try_auto_cast(16, false);
    assert!(actor.state.magic.casting.is_some());
    assert!(actor.state.magic.essences.is_empty());
}

#[test]
fn spell_ai_reconsiders_temporary_restrictions_without_logging_failures() {
    let mut actor = free_caster(&["spell_chronoblur"]);
    actor.magic.loadout.arms_restricted = true;
    for now in 0..5 {
        actor.try_auto_cast(now, true);
    }
    assert!(actor.state.magic.events.is_empty());
    actor.magic.loadout.arms_restricted = false;
    actor.try_auto_cast(5, true);
    assert!(actor.state.magic.casting.is_some());
}

#[test]
fn recommended_echo_timing_requires_reach_and_time_to_use_the_buff() {
    let mut actor = free_caster(&["echo_strike"]);
    actor.try_auto_cast(0, false);
    assert!(actor.state.magic.casting.is_none());
    Arc::make_mut(&mut actor.sheet.offense.weapon).speed = 10.0;
    actor.try_auto_cast(1, true);
    assert!(actor.state.magic.casting.is_none());
    actor.magic.loadout.echo_strike.extra_duration_seconds = 10;
    actor.try_auto_cast(2, true);
    assert!(actor.state.magic.casting.is_some());
}

#[test]
fn opening_spells_queue_through_fatigue_and_each_get_a_turn() {
    let mut actor = free_caster(&["spell_chronoblur", "spell_streamline"]);
    actor.try_auto_cast(0, false);
    assert_eq!(
        actor.state.magic.casting.as_ref().unwrap().definition.id,
        "spell_chronoblur"
    );
    actor.advance_magic(2, &mut rng());
    actor.try_auto_cast(2, false);
    assert!(actor.state.magic.casting.is_none());
    actor.advance_magic(10, &mut rng());
    actor.try_auto_cast(10, false);
    assert_eq!(
        actor.state.magic.casting.as_ref().unwrap().definition.id,
        "spell_streamline"
    );
    actor.advance_magic(15, &mut rng());
    assert!(actor.state.has_active_effect(CHRONOBLUR_EFFECT_ID));
    assert!(actor.state.has_active_effect(STREAMLINE_EFFECT_ID));
}

#[test]
fn resetting_fight_resets_per_spell_ai_usage() {
    let mut actor = free_caster(&["spell_chronoblur"]);
    actor.try_auto_cast(0, false);
    assert!(!actor.state.magic.started_spells.is_empty());
    actor.reset_state();
    assert!(actor.state.magic.started_spells.is_empty());
    actor.try_auto_cast(0, false);
    assert!(actor.state.magic.casting.is_some());
}

#[test]
fn empowered_streamline_radius_covers_targets_in_both_combat_hosts() {
    use super::types::GridPos;
    use crate::squad_battler::combat::{BattleUnit, SquadCombat};
    for (ranks, covered) in [(0, false), (1, true)] {
        let mut actor = free_caster(&["spell_streamline"]);
        actor.magic.loadout.streamline_radius_ranks = ranks;
        actor.sheet.mobility.move_speed = 0.0;
        let mut sim = duel(actor.clone());
        sim.actors[0].position = GridPos::new(0, 0);
        sim.actors[1].position = GridPos::new(35, 0);
        let mut enemy = target();
        enemy.sheet.maneuvers.passive = true;
        let mut squad = SquadCombat::new_with_seed(
            vec![BattleUnit::from_combatant("caster", 0, actor)],
            vec![BattleUnit::from_combatant("target", 1, enemy)],
            123,
        );
        for unit in &mut squad.units {
            unit.move_tiles = 0;
        }
        squad.units[0].pos = crate::squad_battler::combat::GridPos::new(0, 0);
        squad.units[1].pos = crate::squad_battler::combat::GridPos::new(7, 0);
        for _ in 0..7 {
            sim.tick();
            squad.tick();
        }
        assert!(
            sim.combatants[0]
                .state
                .has_active_effect(STREAMLINE_EFFECT_ID)
        );
        assert!(
            squad.units[0]
                .combatant
                .as_ref()
                .unwrap()
                .state
                .has_active_effect(STREAMLINE_EFFECT_ID)
        );
        assert_eq!(
            sim.combatants[1].state.streamline_averages_incoming_damage,
            covered
        );
        assert_eq!(
            squad.units[1]
                .combatant
                .as_ref()
                .unwrap()
                .state
                .streamline_averages_incoming_damage,
            covered
        );
    }
}

#[test]
fn casting_limits_actual_movement_to_five_feet_in_both_hosts_even_with_casting_talents() {
    use crate::squad_battler::combat::{BattleUnit, SquadCombat};
    for combat_casting in [false, true] {
        let mut actor = free_caster(&["spell_streamline"]);
        actor.sheet.mobility.move_speed = 20.0;
        actor.magic.talents.combat_casting = combat_casting;
        actor.magic.talents.diminish_spell_fatigue = 5;
        actor.magic.talents.mitigate_spell_fatigue = true;
        actor.magic.talents.decimate_spell_fatigue = true;
        actor.magic.talents.eliminate_spell_fatigue = true;
        let mut enemy = target();
        enemy.sheet.maneuvers.passive = true;
        let mut sim = SimState::with_rng(SimConfig::new(200.0, 1.0), rng());
        sim.reset_with_combatants(vec![actor.clone(), enemy.clone()]);
        let mut squad = SquadCombat::new_with_seed(
            vec![BattleUnit::from_combatant("caster", 0, actor)],
            vec![BattleUnit::from_combatant("enemy", 1, enemy)],
            123,
        );
        squad.units[0].pos = crate::squad_battler::combat::GridPos::new(0, 0);
        squad.units[1].pos =
            crate::squad_battler::combat::GridPos::new(squad.grid.width - 1, squad.grid.height - 1);
        squad.units[1].move_tiles = 0;
        let mut duel_movement = 0.0;
        let mut squad_movement = 0.0;
        for _ in 0..3 {
            let duel_before = sim.actors[0].position;
            let squad_before = squad.units[0].pos;
            sim.tick();
            squad.tick();
            assert!(sim.combatants[0].state.magic.casting.is_some());
            assert!(
                squad.units[0]
                    .combatant
                    .as_ref()
                    .unwrap()
                    .state
                    .magic
                    .casting
                    .is_some()
            );
            let duel_feet = duel_before.manhattan_distance(sim.actors[0].position) as f32
                * sim.config.tile_size_ft;
            let squad_feet = squad.grid.distance_ft(squad_before, squad.units[0].pos);
            assert!(duel_feet <= 5.0, "duel caster moved {duel_feet} feet");
            assert!(squad_feet <= 5.0, "squad caster moved {squad_feet} feet");
            duel_movement += duel_feet;
            squad_movement += squad_feet;
        }
        assert!(duel_movement > 0.0);
        assert!(squad_movement > 0.0);
    }
}
