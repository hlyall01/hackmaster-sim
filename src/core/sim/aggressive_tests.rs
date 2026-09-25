use super::*;

fn pair() -> Vec<Combatant> {
    let make = |name: &str| {
        combatant_basic(
            name.into(),
            "Test Blade".into(),
            100,
            0,
            3,
            false,
            0,
            "12".into(),
            0,
            10.0,
            5.0,
            5.0,
            false,
            false,
            None,
            true,
            false,
            1000,
        )
    };
    let mut pair = vec![make("Attacker"), make("Defender")];
    for c in &mut pair {
        c.sheet.maneuvers.passive = true;
        c.sheet.vitals.infinite_hp = true;
    }
    pair[0].sheet.maneuvers.aggressive_attack = true;
    pair
}
fn strike(c: &mut [Combatant], a: usize, b: usize) -> super::super::combat::AttackOutcome {
    resolve_attack(
        c,
        a,
        b,
        0,
        false,
        5.0,
        AttackMode::Normal,
        WeaponSlot::Primary,
        0.0,
        None,
        &mut FixedRng(0),
    )
}
#[test]
fn aggressive_bonus_and_defence_penalty_apply_once_even_on_a_miss() {
    let mut c = pair();
    c[0].sheet.offense.attack_bonus = -100;
    let first = strike(&mut c, 0, 1);
    assert!(first.is_aggressive);
    assert!(!first.hit);
    assert_eq!(first.roll.attack_bonus, -95);
    assert!(c[0].state.aggressive_defense_pending);
    assert!(c[0].state.aggressive_maneuvers_locked);
    let defence = strike(&mut c, 1, 0);
    assert_eq!(defence.roll.defense_base, -2);
    let defence = strike(&mut c, 1, 0);
    assert_eq!(defence.roll.defense_base, 0);
}
#[test]
fn aggressive_retreat_halves_before_armour_and_project_confidence_only_exempts_give_ground() {
    for confidence in [false, true] {
        for retreat in [None, Some(false), Some(true)] {
            let mut c = pair();
            if confidence {
                c[0].sheet
                    .modifiers
                    .add_i32(StatIdI32::FlagProjectConfidence, ModifierOpI32::Set(1));
            }
            c[1].state.tactical_retreat = retreat;
            // Fymblwnger can suppress the +5 without cancelling the retreat itself.
            c[1].state.tactical_give_ground_defense_bonus = 0;
            let attack = strike(&mut c, 0, 1);
            assert_eq!(attack.roll.attack_bonus, if confidence { 106 } else { 105 });
            let half = retreat == Some(true) || (retreat == Some(false) && !confidence);
            assert_eq!(attack.damage, if half { 3 } else { 9 });
            assert_eq!(
                attack.damage_breakdown.unwrap().raw_damage,
                if half { 6 } else { 12 }
            );
        }
    }
}
#[test]
fn aggressive_shield_damage_is_halved_before_shield_and_armour() {
    let mut c = pair();
    c[1].sheet.defense.defense_mod = 98;
    c[1].sheet.defense.shield_name = Some("Small shield".into());
    c[1].state.shield_intact = true;
    c[1].sheet.defense.shield_defense_bonus = 5;
    c[1].sheet.defense.shield_dr = 1;
    c[1].state.tactical_retreat = Some(true);
    let weapon = Arc::make_mut(&mut c[0].sheet.offense.weapon);
    weapon.shield_damage_expr = Some("12".into());
    weapon.shield_damage_expr_cache = Some(DamageExprCache::new("12"));
    let attack = strike(&mut c, 0, 1);
    assert!(attack.shield_block);
    assert_eq!(attack.shield_damage, 6);
    assert_eq!(attack.damage, 2);
}
#[test]
fn aggressive_does_not_stack_with_charge_jab_ranged_or_defensive_fighting() {
    for variant in 0..5 {
        let mut c = pair();
        let mode = if variant == 0 {
            AttackMode::Charge
        } else {
            AttackMode::Normal
        };
        match variant {
            1 => Arc::make_mut(&mut c[0].sheet.offense.weapon).use_jab = true,
            2 => c[0].sheet.maneuvers.fight_defensively = true,
            4 => c[0].state.tactical_next_attack_penalty = 1,
            _ => {}
        }
        let attack = resolve_attack(
            &mut c,
            0,
            1,
            0,
            variant == 3,
            5.0,
            mode,
            WeaponSlot::Primary,
            0.0,
            None,
            &mut FixedRng(0),
        );
        assert!(!attack.is_aggressive);
        assert!(!c[0].state.aggressive_defense_pending);
    }
}
#[test]
fn aggressive_recovery_prevents_retreat_reactions_until_next_opportunity() {
    let mut c = pair();
    c[0].sheet.maneuvers.passive = false;
    c[1].sheet.maneuvers.passive = false;
    let mut sim = make_state(c.remove(0), c.remove(0));
    sim.combatants[0].sheet.maneuvers.give_ground = true;
    sim.combatants[0].state.aggressive_maneuvers_locked = true;
    assert!(!sim.apply_incoming_attack_tactics(1, 0, AttackMode::Normal));
    assert_eq!(sim.combatants[0].state.tactical_retreat, None);
}
