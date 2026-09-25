use crate::core::magic::{MagicSaveBonuses, MagicSaveKind, SpellTags};
use crate::core::rng::SimRng;
use crate::core::sim::*;
use crate::core::sim::test_support::resolve_basic_attack;

fn arena() -> SimState {
    let mut knight = Combatant::default();
    knight.sheet.modifiers.add_i32(
        StatIdI32::FlagIntimidateAdversary,
        ModifierOpI32::Set(1),
    );
    let mut enemy = Combatant::default();
    enemy.team_id = 1;
    enemy.magic.saves = MagicSaveBonuses { physical: 7, mental: 5, dodge: 3 };
    let mut sim = SimState::new(SimConfig::new(10.0, 0.0));
    sim.reset_with_combatants(vec![knight.clone(), enemy, knight]);
    sim.actors[0].position = GridPos::new(0, 0);
    sim.actors[1].position = GridPos::new(10, 0);
    sim.actors[2].position = GridPos::new(10, 1);
    sim.refresh_intimidation();
    sim
}

#[test]
fn intimidation_covers_enemies_once_and_leaves_other_rolls_alone() {
    let sim = arena();
    assert!(!sim.combatants[0].state.intimidated);
    assert!(!sim.combatants[2].state.intimidated);
    let enemy = &sim.combatants[1];
    assert!(enemy.state.intimidated);
    assert_eq!(enemy.apply_i32(StatIdI32::AttackBonus, 10), 8);
    assert_eq!(enemy.apply_i32(StatIdI32::AttackBonusBase, 10), 8);
    assert_eq!(enemy.apply_i32(StatIdI32::DefenseMod, 10), 10);
    assert_eq!(enemy.apply_i32(StatIdI32::StrengthDamage, 10), 10);
    assert_eq!(enemy.caster_magic_save_bonus(10), i32::from(enemy.magic.level));
    for (kind, expected) in [(MagicSaveKind::Physical, 5), (MagicSaveKind::Mental, 3), (MagicSaveKind::Dodge, 1)] {
        assert_eq!(enemy.magic_save_bonus(kind, SpellTags::default()), expected);
    }
}

#[test]
fn intimidation_tracks_range_team_death_and_reset() {
    let mut sim = arena();
    sim.combatants[2].state.hp = 0;
    sim.refresh_intimidation();
    assert!(sim.combatants[1].state.intimidated, "exactly ten feet is in range");
    sim.actors[1].position.x = 11;
    sim.refresh_intimidation();
    assert!(!sim.combatants[1].state.intimidated);
    sim.config.tile_size_ft = 0.5;
    sim.refresh_intimidation();
    assert!(sim.combatants[1].state.intimidated, "range is measured in feet");
    sim.combatants[0].team_id = 1;
    sim.refresh_intimidation();
    assert!(!sim.combatants[1].state.intimidated);
    sim.combatants[0].team_id = 0;
    sim.combatants[0].state.hp = 0;
    sim.refresh_intimidation();
    assert!(!sim.combatants[1].state.intimidated);
    sim.combatants[0].reset_state();
    sim.refresh_intimidation();
    assert!(sim.combatants[1].state.intimidated);
    sim.combatants[1].reset_state();
    assert!(!sim.combatants[1].state.intimidated, "new combat state has no stale aura");
}

#[test]
fn intimidation_changes_melee_and_ranged_attack_logs_by_two() {
    for ranged in [false, true] {
        let mut sim = arena();
        let mut plain = sim.combatants.clone();
        plain[1].state.intimidated = false;
        let baseline = resolve_basic_attack(&mut plain, 1, 0, 0, ranged, 10.0, 0.0, &mut SimRng::from_seed(17));
        let affected = resolve_basic_attack(&mut sim.combatants, 1, 0, 0, ranged, 10.0, 0.0, &mut SimRng::from_seed(17));
        assert_eq!(affected.roll.attack_die, baseline.roll.attack_die);
        assert_eq!(affected.roll.attack_bonus, baseline.roll.attack_bonus - 2);
        assert_eq!(affected.roll.attack_total, baseline.roll.attack_total - 2);
    }
}

#[test]
fn intimidation_refreshes_during_ticks_without_retaining_old_coverage() {
    let mut sim = arena();
    for actor in &mut sim.combatants {
        actor.sheet.maneuvers.passive = true;
        actor.sheet.mobility.move_speed = 0.0;
    }
    sim.actors[0].position = GridPos::new(0, 0);
    sim.actors[2].position = GridPos::new(0, 1);
    sim.actors[1].position = GridPos::new(25, 0);
    sim.tick();
    assert!(!sim.combatants[1].state.intimidated);
    sim.actors[1].position = GridPos::new(5, 0);
    sim.tick();
    assert!(sim.combatants[1].state.intimidated);
    sim.combatants[0].state.hp = 0;
    sim.combatants[2].state.hp = 0;
    sim.tick();
    assert!(!sim.combatants[1].state.intimidated);
}
