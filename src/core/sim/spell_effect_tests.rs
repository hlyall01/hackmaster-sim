use super::*;
use crate::core::magic::{MagicLoadout, MagicTalents};
use crate::core::sim::{CombatantSheet, MagicProfile};
use std::sync::Arc;

fn arena(id: &str, distance: f32, ai: SpellCastAi) -> SimState {
    let mut caster = Combatant::new_with_team(CombatantSheet::default(), 0);
    caster.sheet.vitals.max_hp = 1000;
    caster.state.hp = 1000;
    caster.sheet.vitals.threshold_of_pain = 1000;
    caster.sheet.mobility.move_speed = 5.0;
    caster.sheet.offense.attack_bonus = -1000;
    Arc::make_mut(&mut caster.sheet.offense.weapon).reach_ft = 1.0;
    let mut loadout = MagicLoadout::default();
    loadout.learn_spell(id);
    loadout.spell_ai.insert(id.into(), ai);
    caster.configure_magic(MagicProfile {
        level: 10,
        loadout,
        talents: MagicTalents {
            eliminate_spell_fatigue: true,
            ..MagicTalents::default()
        },
        ..MagicProfile::default()
    });
    let mut enemy = caster.clone();
    enemy.team_id = 1;
    enemy.configure_magic(MagicProfile::default());
    enemy.sheet.name = "Enemy".into();
    let mut sim = SimState::with_rng(SimConfig::new(distance, 0.0), SimRng::from_seed(233));
    sim.reset_with_combatants(vec![caster, enemy]);
    sim.collect_detailed_metrics = true;
    sim
}

fn effect(sim: &SimState, id: &str) -> ConfiguredEffect {
    let SpellEffect::Configured(mut effect) =
        SpellRequest::from_loadout(id, &sim.combatants[0].magic.loadout)
            .unwrap()
            .effect
    else {
        panic!("Exertion spell");
    };
    effect.target = Some(1);
    effect
}

fn starts(sim: &SimState) -> usize {
    sim.combat_events.iter().filter(|e| matches!(&e.kind, CombatEventKind::Spell(s) if s.kind == SpellEventKind::CastStarted)).count()
}

#[test]
fn predictive_cast_starts_outside_range_lands_at_completion_and_casts_once() {
    let mut sim = arena("bash_face", 30.0, SpellCastAi::AtFightStart);
    sim.combatants[0].sheet.offense.attack_bonus_base = 1000;
    sim.tick();
    assert_eq!(starts(&sim), 1);
    assert!(sim.combatants[0].state.magic.casting.is_some());
    assert!(sim.distance() > 15.0);
    for _ in 0..3 {
        sim.tick();
    }
    assert!(sim.combatants[1].state.hp < 1000);
    for _ in 0..20 {
        sim.tick();
    }
    assert_eq!(starts(&sim), 1);
}

#[test]
fn predictor_respects_casting_walk_limit_and_immobile_enemies() {
    let mut sim = arena("bash_face", 30.0, SpellCastAi::AtFightStart);
    sim.combatants[0].sheet.mobility.move_speed = 100.0;
    sim.combatants[1].sheet.maneuvers.passive = true;
    sim.prepare_spell_targets(0);
    assert!(sim.combatants[0].state.magic.range_targets.is_empty());
    sim.combatants[1].sheet.maneuvers.passive = false;
    sim.combatants[1].state.trauma_remaining_seconds = 10;
    sim.prepare_spell_targets(0);
    assert!(sim.combatants[0].state.magic.range_targets.is_empty());
}

#[test]
fn predictive_cast_does_not_advance_live_rng_or_positions() {
    let sim = arena("bash_face", 30.0, SpellCastAi::AtFightStart);
    let mut checked = sim.clone();
    let mut baseline = sim.clone();
    checked.prepare_spell_targets(0);
    assert_eq!(checked.actors[0].position, baseline.actors[0].position);
    assert_eq!(checked.elapsed_seconds, baseline.elapsed_seconds);
    assert_eq!(checked.rng.next_u64(), baseline.rng.next_u64());
}

#[test]
fn repeat_predicts_completion_range_then_repeats() {
    let mut sim = arena("bash_face", 30.0, SpellCastAi::AsOftenAsPossible);
    sim.combatants[0].sheet.offense.attack_bonus_base = 1000;
    sim.tick();
    assert_eq!(starts(&sim), 1);
    assert!(sim.distance() > 15.0);
    for _ in 0..14 {
        sim.tick();
    }
    assert!(starts(&sim) >= 2);
}

#[test]
fn completion_rechecks_range_and_does_not_retarget() {
    let mut sim = arena("bash_face", 10.0, SpellCastAi::Manual);
    let request =
        SpellRequest::from_loadout("bash_face", &sim.combatants[0].magic.loadout).unwrap();
    sim.cast_spell(0, request).unwrap();
    sim.actors[1].position.x = sim.actors[0].position.x + 181;
    sim.elapsed_seconds = 2;
    sim.advance_spellcasting();
    assert_eq!(sim.combatants[1].state.hp, 1000);
    assert!(sim.combat_events.iter().any(
        |e| matches!(&e.kind, CombatEventKind::Spell(s) if s.message.contains("out of range"))
    ));
}

#[test]
fn cancelled_once_in_range_is_not_retried() {
    let mut sim = arena("bash_face", 30.0, SpellCastAi::AtFightStart);
    sim.tick();
    sim.cancel_spell(0).unwrap();
    for _ in 0..20 {
        sim.tick();
    }
    assert_eq!(starts(&sim), 1);
}

#[test]
fn empowered_range_changes_ai_start_boundary() {
    let mut sim = arena("bash_face", 30.0, SpellCastAi::AsOftenAsPossible);
    sim.combatants[0]
        .magic
        .loadout
        .spell_parameters
        .insert("bash_face_range".into(), 5);
    sim.tick();
    assert_eq!(starts(&sim), 1);
}

#[test]
fn predictor_waits_if_ranged_retreat_will_take_target_out_of_range() {
    let mut sim = arena("bash_face", 15.0, SpellCastAi::AtFightStart);
    sim.config.grid_width = 200;
    let weapon = Arc::make_mut(&mut sim.combatants[1].sheet.offense.weapon);
    weapon.range_bands_feet = Some([100.0, 200.0, 300.0, 400.0]);
    weapon.uses_projectiles = true;
    sim.combatants[1]
        .state
        .invalidate_weapon_cache(WeaponSlot::Primary);
    sim.combatants[1].sheet.mobility.move_speed = 10.0;
    sim.prepare_spell_targets(0);
    assert!(sim.combatants[0].state.magic.range_targets.is_empty());
    sim.combatants[1].sheet.mobility.move_speed = 0.0;
    sim.prepare_spell_targets(0);
    assert_eq!(
        sim.combatants[0].state.magic.range_targets.get("bash_face"),
        Some(&1)
    );
}

#[test]
fn malformed_exertion_requests_cannot_bypass_casting_metadata() {
    let mut sim = arena("missile", 10.0, SpellCastAi::Manual);
    let mut request =
        SpellRequest::from_loadout("missile", &sim.combatants[0].magic.loadout).unwrap();
    request.definition.casting_seconds = 0;
    assert_eq!(sim.cast_spell(0, request), Err(MagicError::InvalidSpell));
    assert!(sim.combatants[0].state.magic.casting.is_none());
    assert!(sim.combatants[0].state.magic.started_spells.is_empty());
}

#[test]
fn strength_buff_preserves_styles_that_remove_strength_damage() {
    let mut sim = arena("feat_of_strength", 1.0, SpellCastAi::Manual);
    sim.combatants[0].magic.primary_ignores_strength = true;
    sim.resolve_configured_effect(0, effect(&sim, "feat_of_strength"));
    assert_eq!(
        sim.combatants[0].strength_damage_for_slot(WeaponSlot::Primary, 3),
        3
    );
    assert_eq!(
        sim.combatants[0].strength_damage_for_slot(WeaponSlot::Secondary, 3),
        7
    );
}

#[test]
fn all_exertion_spells_construct_and_round_trip_empowerments() {
    let mut loadout = MagicLoadout::default();
    for spell in spell_catalog().iter().skip(3) {
        loadout.learn_spell(&spell.id);
        loadout
            .spell_ai
            .insert(spell.id.clone(), SpellCastAi::AtFightStart);
        for field in &spell.empowerments {
            if let crate::core::magic::SpellEmpowerment::Number { field, .. } = field {
                *field.value_mut(&mut loadout) = 1;
            }
        }
        let request = SpellRequest::from_loadout(&spell.id, &loadout).unwrap();
        assert!(request.total_cost > spell.base_cost);
        assert_eq!(request.definition.casting_seconds, spell.casting_seconds);
        assert_eq!(request.definition.components, spell.components);
    }
    let saved = serde_json::to_string(&loadout).unwrap();
    assert_eq!(
        serde_json::from_str::<MagicLoadout>(&saved).unwrap(),
        loadout
    );
}

#[test]
fn missiles_are_separate_wounds_and_are_attributed_to_the_spell() {
    let mut sim = arena("missile", 10.0, SpellCastAi::Manual);
    sim.combatants[1].state.streamline_averages_incoming_damage = true;
    sim.combatants[1].sheet.defense.armor_dr = 100;
    let mut spell = effect(&sim, "missile");
    spell.projectiles = 3;
    sim.resolve_configured_effect(0, spell);
    let expected = expected_damage_expr("d4p+1").floor() as i32;
    assert_eq!(sim.combatants[1].state.hp, 1000 - 3 * expected);
    assert_eq!(sim.attack_metrics.len(), 3);
    assert!(
        sim.attack_metrics
            .iter()
            .all(|m| m.source_is_spell_for_test("Missile"))
    );
}

#[test]
fn impaling_projectile_loses_energy_and_rejects_targets_behind_it() {
    let mut sim = arena("impaling_missile", 10.0, SpellCastAi::Manual);
    let mut third = sim.combatants[1].clone();
    third.sheet.name = "Second enemy".into();
    sim.combatants.push(third.clone());
    sim.combatants.push(third);
    let origin = sim.actors[0].position;
    sim.actors[1].position = GridPos::new(origin.x + 10, origin.y);
    sim.actors.push(SimActor {
        position: GridPos::new(origin.x + 20, origin.y),
    });
    sim.actors.push(SimActor {
        position: GridPos::new(origin.x + 5, origin.y),
    });
    sim.resolve_configured_effect(0, effect(&sim, "impaling_missile"));
    assert!(sim.combatants[1].state.hp < sim.combatants[2].state.hp);
    assert!(sim.combatants[2].state.hp < 1000);
    assert_eq!(sim.combatants[3].state.hp, 1000);
}

#[test]
fn repel_honors_weight_and_physical_save() {
    let mut sim = arena("repel", 10.0, SpellCastAi::Manual);
    sim.combatants[1].magic.saves.physical = -10000;
    sim.combatants[1].magic.loadout.body_weight_lbs = 100;
    let before = sim.distance();
    sim.resolve_configured_effect(0, effect(&sim, "repel"));
    assert_eq!(sim.distance(), before + 2.0);
    sim.combatants[1].magic.saves.physical = 10000;
    let before = sim.distance();
    sim.resolve_configured_effect(0, effect(&sim, "repel"));
    assert_eq!(sim.distance(), before);
}

#[test]
fn feat_strength_uses_table_and_survives_until_next_attempt() {
    let mut sim = arena("feat_of_strength", 1.0, SpellCastAi::Manual);
    sim.resolve_configured_effect(0, effect(&sim, "feat_of_strength"));
    assert_eq!(
        sim.combatants[0].apply_i32(StatIdI32::StrengthDamageBase, 0),
        4
    );
    assert!(
        sim.combatants[0]
            .state
            .has_active_effect("feat_of_strength")
    );
    let _ = crate::core::sim::test_support::resolve_basic_attack(
        &mut sim.combatants,
        0,
        1,
        0,
        false,
        1.0,
        0.0,
        &mut sim.rng,
    );
    assert!(
        !sim.combatants[0]
            .state
            .has_active_effect("feat_of_strength")
    );
    assert_eq!(
        sim.combatants[0].apply_i32(StatIdI32::StrengthDamageBase, 0),
        0
    );
}

#[test]
fn strength_buffs_do_not_add_two_replacement_scores() {
    let mut sim = arena("boost_strength", 1.0, SpellCastAi::Manual);
    sim.resolve_configured_effect(0, effect(&sim, "boost_strength"));
    let sustained = sim.combatants[0].apply_i32(StatIdI32::StrengthDamageBase, 0);
    sim.resolve_configured_effect(0, effect(&sim, "feat_of_strength"));
    assert_eq!(
        sim.combatants[0].apply_i32(StatIdI32::StrengthDamageBase, 0),
        4
    );
    sim.combatants[0]
        .state
        .active_effects
        .retain(|e| e.id != "feat_of_strength");
    assert_eq!(
        sim.combatants[0].apply_i32(StatIdI32::StrengthDamageBase, 0),
        sustained
    );
}

#[test]
fn aggravate_pain_extends_existing_trauma_only_in_its_area() {
    let mut sim = arena("aggravate_pain", 10.0, SpellCastAi::Manual);
    sim.combatants[1].state.trauma_remaining_seconds = 10;
    sim.combatants[1].magic.saves.mental = -10000;
    sim.resolve_configured_effect(0, effect(&sim, "aggravate_pain"));
    assert_eq!(sim.combatants[1].state.trauma_remaining_seconds, 70);
    assert_eq!(sim.combatants[0].state.trauma_remaining_seconds, 0);
}

#[test]
fn sweeping_agony_causes_no_hp_damage_and_does_not_repeat_on_a_target() {
    let mut sim = arena("sweeping_agony", 5.0, SpellCastAi::Manual);
    sim.combatants[1].sheet.vitals.threshold_of_pain = 0;
    sim.combatants[1].sheet.vitals.constitution = 0;
    sim.resolve_configured_effect(0, effect(&sim, "sweeping_agony"));
    sim.elapsed_seconds = 1;
    sim.advance_effect_fields();
    assert_eq!(sim.combatants[1].state.hp, 1000);
    assert!(sim.combatants[1].state.trauma_remaining_seconds > 0);
    let trauma = sim.combatants[1].state.trauma_remaining_seconds;
    sim.elapsed_seconds = 2;
    sim.advance_effect_fields();
    assert_eq!(sim.combatants[1].state.trauma_remaining_seconds, trauma);
}

#[test]
fn stay_down_adds_a_second_then_requires_a_save_to_stand() {
    let mut sim = arena("stay_down", 10.0, SpellCastAi::Manual);
    sim.combatants[1].state.knockback_immobile_seconds = 1;
    sim.combatants[1].magic.saves.physical = -10000;
    sim.resolve_configured_effect(0, effect(&sim, "stay_down"));
    sim.advance_effect_fields();
    assert_eq!(sim.combatants[1].state.knockback_immobile_seconds, 2);
    sim.combatants[1].state.knockback_immobile_seconds = 1;
    sim.elapsed_seconds = 1;
    sim.advance_effect_fields();
    assert_eq!(sim.combatants[1].state.knockback_immobile_seconds, 2);
    sim.combatants[1].state.knockback_immobile_seconds = 1;
    sim.combatants[1].magic.saves.physical = 10000;
    sim.elapsed_seconds = 2;
    sim.advance_effect_fields();
    assert_eq!(sim.combatants[1].state.knockback_immobile_seconds, 1);
}

#[test]
fn kinetic_fuse_waits_then_explodes_once_and_can_hurt_caster() {
    let mut sim = arena("kinetic_overload", 5.0, SpellCastAi::Manual);
    sim.resolve_configured_effect(0, effect(&sim, "kinetic_overload"));
    sim.elapsed_seconds = 29;
    sim.advance_effect_fields();
    assert_eq!(sim.combatants[0].state.hp, 1000);
    sim.elapsed_seconds = 30;
    sim.advance_effect_fields();
    assert!(sim.combatants[0].state.hp < 1000);
    assert!(sim.combatants[1].state.hp < 1000);
    assert!(sim.combatants[0].state.magic.charged_objects.is_empty());
}

#[test]
fn forceful_charge_moves_caster_without_causing_a_wound() {
    let mut sim = arena("forceful_charge", 40.0, SpellCastAi::Manual);
    let before = sim.distance();
    sim.resolve_configured_effect(0, effect(&sim, "forceful_charge"));
    assert!(sim.distance() < before);
    assert_eq!(sim.combatants[0].state.hp, 1000);
}

#[test]
fn pugilism_channels_attacks_and_stops_cleanly() {
    let mut sim = arena("pugilism", 50.0, SpellCastAi::Manual);
    sim.combatants[0].magic.fist_attack = 1000;
    sim.combatants[0].magic.fist_speed = 2.0;
    let request = SpellRequest::from_loadout("pugilism", &sim.combatants[0].magic.loadout).unwrap();
    sim.cast_spell(0, request).unwrap();
    sim.elapsed_seconds = 2;
    sim.advance_spellcasting();
    assert!(sim.combatants[0].state.magic.channeling.is_some());
    assert!(sim.combatants[1].state.hp < 1000);
    assert_eq!(
        sim.combatants[0].state.magic.fists.as_ref().unwrap().hp,
        [100, 100]
    );
    sim.stop_channeling(0).unwrap();
    assert!(sim.combatants[0].state.magic.fists.is_none());
}

#[test]
fn fist_damage_is_reflected_only_up_to_remaining_hand_hp() {
    let mut sim = arena("pugilism", 50.0, SpellCastAi::Manual);
    let request = SpellRequest::from_loadout("pugilism", &sim.combatants[0].magic.loadout).unwrap();
    sim.cast_spell(0, request).unwrap();
    sim.elapsed_seconds = 2;
    sim.advance_spellcasting();
    sim.combatants[0].state.magic.fists.as_mut().unwrap().hp = vec![3, 100];
    sim.combatants[1].sheet.offense.attack_bonus = 10000;
    sim.combatants[1].sheet.offense.strength_damage = 1000;
    sim.combatants[1]
        .state
        .set_next_attack_time(WeaponSlot::Primary, Some(0.0));
    sim.retaliate_against_fists(0);
    assert_eq!(sim.combatants[0].state.hp, 997);
    assert_eq!(
        sim.combatants[0].state.magic.fists.as_ref().unwrap().hp,
        [0, 100]
    );
}

impl AttackMetricSample {
    fn source_is_spell_for_test(&self, name: &str) -> bool {
        matches!(&self.damage_source, DamageSource::Spell { name: actual } if actual == name)
    }
}

fn custom_spell_json(template: &str) -> serde_json::Value {
    let file: serde_json::Value =
        serde_json::from_str(include_str!("../../../data/sim/spells.json")).unwrap();
    let mut spell = file["spells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["id"] == template)
        .unwrap()
        .clone();
    spell["id"] = format!("custom_{template}").into();
    spell["name"] = format!("Custom {template}").into();
    spell["kind"] = "configured".into();
    for field in spell["empowerments"].as_array_mut().unwrap() {
        field["field"] = format!("custom_{}", field["field"].as_str().unwrap()).into();
    }
    spell
}

fn parse_custom(spell: serde_json::Value) -> SpellCatalogEntry {
    parse_spell_catalog(&serde_json::json!({"spells": [spell]}).to_string())
        .unwrap()
        .remove(0)
}

#[test]
fn every_configured_spell_casts_under_a_new_id_without_runtime_registration() {
    for original in spell_catalog().iter().filter(|s| s.mechanics.is_some()) {
        let entry = parse_custom(custom_spell_json(&original.id));
        let mut sim = arena(&entry.id, 10.0, SpellCastAi::Manual);
        let request = SpellRequest::from_catalog(&entry, &sim.combatants[0].magic.loadout).unwrap();
        sim.cast_spell(0, request).unwrap();
        sim.elapsed_seconds = entry.casting_seconds;
        sim.advance_spellcasting();
        assert!(
            sim.combat_events
                .iter()
                .any(|event| matches!(&event.kind, CombatEventKind::Spell(s)
            if s.kind == SpellEventKind::CastCompleted && s.message.contains(&entry.name))),
            "{}",
            entry.id
        );
    }
}

#[test]
fn new_data_controls_damage_mitigation_amplification_cost_and_editor_summary() {
    let mut json = custom_spell_json("missile");
    json["mechanics"]["values"]["dice"] = 2.into();
    json["mechanics"]["values"]["projectiles"] = 3.into();
    json["mechanics"]["effect"]["damage"] = serde_json::json!({
        "sides": 8, "penetrating": false, "bonus_per_die": 0, "flat_bonus": 3,
        "armor_reduction": true, "phantom": false, "knockback": false
    });
    json["empowerments"] = serde_json::json!([{
        "type": "number", "field": "custom_range", "label": "Extra range", "min": 0,
        "max": 10, "suffix": "", "cost_per_rank": 5,
        "changes": [{"stat": "range", "per_rank": 10}]
    }]);
    json["summary"] = "{dice} dice, {projectiles} wounds, {range} ft".into();
    let entry = parse_custom(json);
    let mut sim = arena(&entry.id, 10.0, SpellCastAi::Manual);
    sim.combatants[0]
        .magic
        .loadout
        .spell_parameters
        .insert("custom_range".into(), 2);
    sim.combatants[1].state.streamline_averages_incoming_damage = true;
    sim.combatants[1].sheet.defense.armor_dr = 2;
    let request = SpellRequest::from_catalog(&entry, &sim.combatants[0].magic.loadout).unwrap();
    assert_eq!(request.total_cost, entry.base_cost + 10);
    assert_eq!(
        crate::game_logic::spell_editor_summary(&entry, &sim.combatants[0].magic.loadout).unwrap(),
        "2 dice, 3 wounds, 200 ft"
    );
    sim.cast_spell(0, request).unwrap();
    sim.elapsed_seconds = entry.casting_seconds;
    sim.advance_spellcasting();
    assert_eq!(sim.combatants[1].state.hp, 970); // Each: floor(2d8) + 3 - 2 DR.
    assert_eq!(sim.attack_metrics.len(), 3);
    assert!(
        sim.attack_metrics
            .iter()
            .all(|m| m.source_is_spell_for_test(&entry.name))
    );
}

#[test]
fn a_data_only_sequence_combines_damage_and_displacement() {
    let mut json = custom_spell_json("missile");
    let damage = json["mechanics"]["effect"].clone();
    json["mechanics"]["values"]["force"] = 400.into();
    json["mechanics"]["effect"] = serde_json::json!({"type": "sequence", "effects": [damage, {
        "type": "displace", "subject": "enemy", "distance": "force_over_weight",
        "direction": "away_from_caster", "stop_distance": 0, "recovery": false, "save": null
    }]});
    let entry = parse_custom(json);
    let mut sim = arena(&entry.id, 10.0, SpellCastAi::Manual);
    let request = SpellRequest::from_catalog(&entry, &sim.combatants[0].magic.loadout).unwrap();
    let before = sim.distance();
    sim.cast_spell(0, request).unwrap();
    sim.elapsed_seconds = entry.casting_seconds;
    sim.advance_spellcasting();
    assert!(sim.combatants[1].state.hp < 1000);
    assert_eq!(sim.distance(), before + 2.0);
}

#[test]
fn renamed_movement_buff_reads_defense_numbers_from_its_own_data() {
    let mut json = custom_spell_json("spell_chronoblur");
    json["mechanics"]["effect"]["movement_defense"]["melee_bonus"] = 7.into();
    json["mechanics"]["effect"]["movement_defense"]["ranged_distance"] = 35.into();
    let entry = parse_custom(json);
    let mut sim = arena(&entry.id, 10.0, SpellCastAi::Manual);
    let request = SpellRequest::from_catalog(&entry, &sim.combatants[0].magic.loadout).unwrap();
    sim.cast_spell(0, request).unwrap();
    sim.elapsed_seconds = entry.casting_seconds;
    sim.advance_spellcasting();
    assert_eq!(sim.combatants[0].state.movement_spell_defense(), (0, 0.0));
    sim.combatants[0].state.moved_last_tick = true;
    assert_eq!(sim.combatants[0].state.movement_spell_defense(), (7, 35.0));
}

#[test]
fn data_only_summon_can_have_a_different_count_and_hp_fraction() {
    let mut json = custom_spell_json("pugilism");
    json["mechanics"]["effect"]["count"] = 3.into();
    json["mechanics"]["effect"]["hp_percent"] = 25.into();
    let entry = parse_custom(json);
    let mut sim = arena(&entry.id, 10.0, SpellCastAi::Manual);
    let request = SpellRequest::from_catalog(&entry, &sim.combatants[0].magic.loadout).unwrap();
    sim.cast_spell(0, request).unwrap();
    sim.elapsed_seconds = entry.casting_seconds;
    sim.advance_spellcasting();
    assert_eq!(
        sim.combatants[0].state.magic.fists.as_ref().unwrap().hp,
        vec![250; 3]
    );
}

#[test]
fn malformed_effect_data_is_rejected_before_it_enters_combat() {
    let original = custom_spell_json("missile");
    let rejects = |spell| {
        assert!(parse_spell_catalog(&serde_json::json!({"spells": [spell]}).to_string()).is_err())
    };
    let mut bad = original.clone();
    bad["mechanics"]["effect"]["damage"]["sides"] = 1.into();
    rejects(bad);
    let mut bad = original.clone();
    bad["mechanics"]["effect"]["damage"]["flat_bonus"] = i32::MIN.into();
    rejects(bad);
    let mut bad = original.clone();
    bad["mechanics"]["values"]["dice"] = 0.into();
    rejects(bad);
    let mut bad = original.clone();
    bad["mechanics"]["effect"]["delivery"] =
        serde_json::json!({"type": "chain", "turn_degrees": 45, "dice_lost_per_target": 0});
    rejects(bad);
    let mut bad = original.clone();
    bad["empowerments"][0]["changes"] = serde_json::json!([{"stat": "dice", "per_rank": 0.5}]);
    rejects(bad);
    let mut bad = original.clone();
    bad["empowerments"][0]["cost_per_rank"] = u32::MAX.into();
    rejects(bad);
    let mut bad = original.clone();
    bad["mechanics"]["effect"]["typo"] = true.into();
    rejects(bad);
    let mut bad = original;
    bad["empowerments"][1]["field"] = bad["empowerments"][0]["field"].clone();
    rejects(bad);
}

#[test]
fn legacy_exertion_parameters_migrate_without_losing_empowerments() {
    let old = serde_json::json!({"exertion_ranks": {"missile_damage": 2}});
    let loadout: MagicLoadout = serde_json::from_value(old).unwrap();
    assert_eq!(loadout.spell_parameters["missile_damage"], 2);
    let saved = serde_json::to_value(&loadout).unwrap();
    assert_eq!(saved["spell_parameters"]["missile_damage"], 2);
    assert!(saved.get("exertion_ranks").is_none());
}

#[test]
fn composite_buffs_survive_together_and_dismiss_together() {
    let mut json = custom_spell_json("spell_chronoblur");
    let movement = json["mechanics"]["effect"].clone();
    json["mechanics"]["values"]["radius"] = 25.into();
    json["mechanics"]["effect"] = serde_json::json!({"type": "sequence", "effects": [movement, {
        "type": "timed_buff", "movement_defense": null, "average_incoming_damage": true
    }]});
    let entry = parse_custom(json);
    let mut sim = arena(&entry.id, 10.0, SpellCastAi::Manual);
    let request = SpellRequest::from_catalog(&entry, &sim.combatants[0].magic.loadout).unwrap();
    sim.cast_spell(0, request).unwrap();
    sim.elapsed_seconds = entry.casting_seconds;
    sim.advance_spellcasting();
    let state = &mut sim.combatants[0].state;
    state.moved_last_tick = true;
    assert_eq!(state.movement_spell_defense(), (4, 20.0));
    assert!(
        state
            .active_effects
            .iter()
            .any(|b| b.average_damage_radius == Some(25.0))
    );
    assert!(state.magic.owned_buffs.contains(&entry.id));
    sim.combatants[0]
        .dismiss_spell(&entry.id, sim.elapsed_seconds)
        .unwrap();
    assert!(!sim.combatants[0].state.has_active_effect(&entry.id));
}

#[test]
fn catalog_rejects_parameter_keys_shared_between_spells() {
    let first = custom_spell_json("missile");
    let mut second = first.clone();
    second["id"] = "different_id".into();
    assert!(
        parse_spell_catalog(&serde_json::json!({"spells": [first, second]}).to_string()).is_err()
    );
}

#[test]
fn both_automatic_modes_wait_for_predicted_completion_range() {
    for ai in [SpellCastAi::AtFightStart, SpellCastAi::AsOftenAsPossible] {
        let mut sim = arena("bash_face", 60.0, ai);
        sim.tick();
        assert_eq!(starts(&sim), 0);
        sim.actors[1].position.x = sim.actors[0].position.x + 30;
        sim.actors[1].position.y = sim.actors[0].position.y;
        sim.tick();
        assert_eq!(starts(&sim), 1);
    }
}

#[test]
fn one_second_spell_resolves_same_second_with_fatigue_starting_next_second() {
    for ai in [
        SpellCastAi::Manual,
        SpellCastAi::AtFightStart,
        SpellCastAi::AsOftenAsPossible,
    ] {
        let mut sim = arena("missile", 10.0, ai);
        sim.combatants[0].magic.talents = MagicTalents::default();
        sim.elapsed_seconds = 10;
        if ai == SpellCastAi::Manual {
            let request =
                SpellRequest::from_loadout("missile", &sim.combatants[0].magic.loadout).unwrap();
            sim.cast_spell(0, request).unwrap();
        } else {
            sim.advance_spellcasting();
        }
        assert!(sim.combatants[1].state.hp < 1000);
        let magic = &sim.combatants[0].state.magic;
        assert!(magic.casting.is_none());
        assert_eq!(magic.fatigue, Some((11, 17)));
        assert!(!magic.fatigued());
        let times: Vec<_> = sim
            .combat_events
            .iter()
            .filter_map(|event| match &event.kind {
                CombatEventKind::Spell(s)
                    if matches!(
                        s.kind,
                        SpellEventKind::CastStarted | SpellEventKind::CastCompleted
                    ) =>
                {
                    Some(s.time)
                }
                _ => None,
            })
            .collect();
        assert_eq!(times, vec![10, 10]);
        sim.elapsed_seconds = 11;
        sim.advance_spellcasting();
        assert!(sim.combatants[0].state.magic.fatigued());
    }
}

#[test]
fn old_range_policy_loads_as_the_single_once_policy() {
    for old in ["when_in_range", "when_useful", "at_fight_start"] {
        let ai: SpellCastAi = serde_json::from_value(serde_json::json!(old)).unwrap();
        assert_eq!(ai, SpellCastAi::AtFightStart);
        assert_eq!(serde_json::to_value(ai).unwrap(), "at_fight_start");
    }
    assert_eq!(SpellCastAi::ALL.len(), 3);
}

#[test]
fn one_second_prediction_does_not_borrow_movement_from_the_following_second() {
    for ai in [SpellCastAi::AtFightStart, SpellCastAi::AsOftenAsPossible] {
        let mut sim = arena("repel", 31.0, ai);
        sim.advance_spellcasting();
        assert_eq!(starts(&sim), 0);
        sim.actors[1].position.x = sim.actors[0].position.x + 30;
        sim.actors[1].position.y = sim.actors[0].position.y;
        sim.advance_spellcasting();
        assert_eq!(starts(&sim), 1);
        assert!(sim.combatants[0].state.magic.casting.is_none());
    }
}

#[test]
fn once_self_buff_still_casts_immediately_without_an_enemy_in_weapon_reach() {
    let mut sim = arena("feat_of_strength", 100.0, SpellCastAi::AtFightStart);
    sim.advance_spellcasting();
    assert_eq!(starts(&sim), 1);
    assert!(
        sim.combatants[0]
            .state
            .has_active_effect("feat_of_strength")
    );
}

#[test]
fn spell_wounds_bypass_worn_armor_but_keep_natural_dr_and_its_modifiers() {
    for id in [
        "missile",
        "impaling_missile",
        "bash_face",
        "kinetic_overload",
    ] {
        for natural in [0, 1, 2, 20] {
            let mut sim = arena(id, 10.0, SpellCastAi::Manual);
            sim.combatants[1].sheet.defense.armor_dr = 100 + natural;
            sim.combatants[1].sheet.defense.natural_dr = natural;
            let mut buff = TemporaryEffect::new("natural_protection", 30);
            buff.modifiers.add_i32(
                StatIdI32::NaturalDr,
                crate::core::sim::ModifierOpI32::Add(1),
            );
            sim.combatants[1].state.add_effect(buff);
            let spell = effect(&sim, id);
            sim.deal_spell_wound(0, 1, &spell, 10);
            assert_eq!(
                sim.combatants[1].state.hp,
                1000 - (10 - natural - 1).max(0),
                "{id}"
            );
        }
    }
}

#[test]
fn armor_sensitive_spell_uses_combined_dr_without_double_counting_natural_dr() {
    let mut sim = arena("pugilism", 10.0, SpellCastAi::Manual);
    sim.combatants[1].sheet.defense.armor_dr = 7; // 5 armor + 2 natural.
    sim.combatants[1].sheet.defense.natural_dr = 2;
    let spell = effect(&sim, "pugilism");
    sim.deal_spell_wound(0, 1, &spell, 10);
    assert_eq!(sim.combatants[1].state.hp, 997);
}

#[test]
fn each_missile_in_a_volley_is_reduced_as_a_separate_wound() {
    let mut sim = arena("missile", 10.0, SpellCastAi::Manual);
    sim.combatants[1].sheet.defense.armor_dr = 102;
    sim.combatants[1].sheet.defense.natural_dr = 2;
    sim.combatants[1].state.streamline_averages_incoming_damage = true;
    let mut spell = effect(&sim, "missile");
    spell.projectiles = 3;
    let raw = SimState::damage_average(&spell, spell.dice, 0);
    sim.resolve_configured_effect(0, spell);
    assert_eq!(sim.combatants[1].state.hp, 1000 - 3 * (raw - 2).max(0));
    assert_eq!(sim.attack_metrics.len(), 3);
}

#[test]
fn zero_delay_explosion_resolves_at_target_immediately_and_only_once() {
    let mut json = custom_spell_json("kinetic_overload");
    json["mechanics"]["target"] = "enemy".into();
    json["mechanics"]["ai_range"] = "spell_range".into();
    json["mechanics"]["values"]["range"] = 60.into();
    json["mechanics"]["values"]["duration"] = 0.into();
    json["empowerments"] = serde_json::json!([]);
    let entry = parse_custom(json);
    let mut sim = arena(&entry.id, 20.0, SpellCastAi::Manual);
    let request = SpellRequest::from_catalog(&entry, &sim.combatants[0].magic.loadout).unwrap();
    sim.cast_spell(0, request).unwrap();
    assert_eq!(sim.combatants[0].state.hp, 1000);
    assert!(sim.combatants[1].state.hp < 1000);
    assert!(sim.combatants[0].state.magic.charged_objects.is_empty());
    let hp = sim.combatants[1].state.hp;
    sim.elapsed_seconds = 30;
    sim.advance_effect_fields();
    assert_eq!(sim.combatants[1].state.hp, hp);
}

#[test]
fn delayed_enemy_explosion_stays_at_impact_position() {
    let mut sim = arena("kinetic_overload", 20.0, SpellCastAi::Manual);
    let mut spell = effect(&sim, "kinetic_overload");
    spell.definition.target = EffectTarget::Enemy;
    spell.duration = 2;
    spell.range = 60.0;
    let center = sim.actors[1].position;
    sim.resolve_configured_effect(0, spell);
    assert_eq!(sim.combatants[1].state.hp, 1000);
    assert_eq!(
        sim.combatants[0].state.magic.charged_objects[0].center,
        Some(center)
    );
    sim.actors[0].position = center;
    sim.actors[1].position.x += 20;
    sim.elapsed_seconds = 2;
    sim.advance_effect_fields();
    assert!(sim.combatants[0].state.hp < 1000);
    assert_eq!(sim.combatants[1].state.hp, 1000);
}

#[test]
fn targeted_explosion_checks_range_before_scheduling_or_detonating() {
    for delay in [0, 2] {
        let mut sim = arena("kinetic_overload", 20.0, SpellCastAi::Manual);
        let mut spell = effect(&sim, "kinetic_overload");
        spell.definition.target = EffectTarget::Enemy;
        spell.duration = delay;
        spell.range = 10.0;
        sim.resolve_configured_effect(0, spell);
        assert_eq!(sim.combatants[1].state.hp, 1000);
        assert!(sim.combatants[0].state.magic.charged_objects.is_empty());
    }
}
