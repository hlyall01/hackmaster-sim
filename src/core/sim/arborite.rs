//! Arborite racial configuration and timed combat effects.
use super::{Combatant, ModifierOpI32, StatIdI32, TemporaryEffect};
use crate::core::rules::roll_damage_expr;
use rand::Rng;
use serde::{Deserialize, Serialize};

pub const WOAD_EFFECT_ID: &str = "racial_warrior_woad";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RacialTalentConfig {
    pub age_years: u16,
    pub average_artistry_painting: bool,
    pub warpaint_prepared: bool,
    pub under_mental_compulsion: bool,
    pub choose_silent_defiance: bool,
}

impl Default for RacialTalentConfig {
    fn default() -> Self {
        Self {
            age_years: 0,
            average_artistry_painting: false,
            warpaint_prepared: false,
            under_mental_compulsion: false,
            choose_silent_defiance: true,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ArboriteProfile {
    pub iron_dice_multiplier: i32,
    pub warrior_woad: bool,
    pub warpaint_prepared: bool,
    pub silent_defiance: bool,
    pub choose_silent_defiance: bool,
    pub initially_compelled: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ArboriteState {
    pub warpaint_used: bool,
    pub compelled: bool,
    pub unconscious_seconds: u32,
    pub woad_strength_roll: Option<u8>,
    pub woad_expiry_saved: Option<(bool, bool)>,
}

impl Combatant {
    pub fn current_strength(&self) -> crate::character::AbilityScore {
        let base = self
            .state
            .active_effects
            .iter()
            .filter(|e| e.strength_override.is_some())
            .max_by_key(|e| e.strength_override.unwrap().1)
            .and_then(|e| e.strength_score_override)
            .unwrap_or(self.magic.strength);
        let increase = self
            .state
            .active_effects
            .iter()
            .map(|e| u32::from(e.strength_increase))
            .sum::<u32>();
        crate::character::AbilityScore::new(
            (u32::from(base.base) + increase).min(25) as u8,
            base.percentile,
        )
    }

    pub fn set_mental_compulsion(&mut self, active: bool) {
        self.state.arborite.compelled = active;
        if self.silent_defiance_holding_still() {
            self.state.clear_attack_timers();
            self.state.magic.casting = None;
            self.state.magic.channeling = None;
        }
    }

    pub fn silent_defiance_holding_still(&self) -> bool {
        self.arborite.silent_defiance
            && self.arborite.choose_silent_defiance
            && self.state.arborite.compelled
    }

    pub(crate) fn racial_actions_blocked(&self) -> bool {
        self.state.arborite.unconscious_seconds > 0 || self.silent_defiance_holding_still()
    }

    pub(crate) fn iron_dice_multiplier(&self, iron_weapon: bool) -> i32 {
        if iron_weapon {
            self.arborite.iron_dice_multiplier.max(1)
        } else {
            1
        }
    }

    pub(crate) fn trigger_warrior_woad(
        &mut self,
        damage: i32,
        hacking_or_piercing: bool,
        rng: &mut impl Rng,
    ) {
        if damage < 1
            || !hacking_or_piercing
            || self.state.hp <= 0
            || !self.arborite.warrior_woad
            || !self.arborite.warpaint_prepared
            || self.state.arborite.warpaint_used
        {
            return;
        }
        self.state.arborite.warpaint_used = true;
        let strength_roll = rng.gen_range(1..=6);
        self.state.arborite.woad_strength_roll = Some(strength_roll);
        let duration = roll_damage_expr("5d12p", rng, false).max(1);
        let mut buff = TemporaryEffect::new(WOAD_EFFECT_ID, duration);
        buff.strength_increase = strength_roll;
        buff.modifiers
            .add_i32(StatIdI32::AllDefenseBonus, ModifierOpI32::Add(6));
        self.state.add_effect(buff);
    }

    pub(crate) fn tick_racial_effects(&mut self, rng: &mut impl Rng) {
        self.state.arborite.unconscious_seconds =
            self.state.arborite.unconscious_seconds.saturating_sub(1);
        if self
            .state
            .active_effects
            .iter()
            .any(|effect| effect.id == WOAD_EFFECT_ID && effect.remaining_seconds == 1)
        {
            use crate::core::magic::{MagicSaveKind, SpellTags, roll_magic_save};
            let physical = roll_magic_save(
                11,
                i32::from(self.magic.level)
                    + self.magic_save_bonus(MagicSaveKind::Physical, SpellTags::default()),
                rng,
            )
            .saved;
            let mental = roll_magic_save(
                11,
                i32::from(self.magic.level)
                    + self.magic_save_bonus(MagicSaveKind::Mental, SpellTags::default()),
                rng,
            )
            .saved;
            self.state.arborite.woad_expiry_saved = Some((physical, mental));
            if !(physical && mental) {
                self.state.arborite.unconscious_seconds = 3600;
                self.state.trauma_remaining_seconds = self.state.trauma_remaining_seconds.max(3600);
                self.state.clear_attack_timers();
                self.state.magic.casting = None;
                self.state.magic.channeling = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::rules::DamageExprCache;
    use crate::core::sim::{
        combat::{AttackMode, resolve_attack, resolve_style_strike},
        *,
    };
    use crate::core::types::TalentSelection;
    use crate::{data, game_logic as logic};
    use rand::{SeedableRng, rngs::StdRng};
    use std::sync::Arc;

    fn arborite(talents: &[&str], paint: bool) -> Combatant {
        let (w, a, s) = data::load_catalogs().unwrap();
        let t = data::load_talents(data::TALENTS_PATH).unwrap();
        let mut p = logic::PlayerConfig::new("Arborite", w.first_id().unwrap());
        p.race_id = Some("arborite".into());
        p.level = 10;
        p.racial_talents.average_artistry_painting = paint;
        p.racial_talents.warpaint_prepared = paint;
        p.talents = talents
            .iter()
            .map(|id| TalentSelection {
                id: (*id).into(),
                rank: 1,
                weapon: None,
            })
            .collect();
        let mut c =
            logic::build_combatant(&p, &w, &a, &s, &logic::NpcPresetCatalog::new(vec![]), &t);
        c.sheet.vitals.max_hp = 10000;
        c.sheet.vitals.threshold_of_pain = 10000;
        c.sheet.defense.defense_mod = -1000;
        c.sheet.defense.ranged_defense_mod = -1000;
        c.sheet.defense.knockback_step = 10000;
        c.reset_state();
        c
    }
    fn weapon(iron: bool, hacking: bool) -> Combatant {
        let mut c = Combatant::default();
        c.sheet.offense.attack_bonus = 1000;
        let w = Arc::make_mut(&mut c.sheet.offense.weapon);
        w.has_weapon = true;
        w.is_unarmed = false;
        w.hacking_or_piercing = hacking;
        w.is_iron = iron;
        w.damage_expr = "2d1+7".into();
        w.damage_expr_cache = DamageExprCache::new("2d1+7");
        w.crit_min_roll = 21;
        c
    }
    fn strike(
        attacker: Combatant,
        defender: Combatant,
        slot: WeaponSlot,
    ) -> (super::super::combat::AttackOutcome, Vec<Combatant>) {
        let mut actors = vec![attacker, defender];
        let hit = resolve_attack(
            &mut actors,
            0,
            1,
            0,
            false,
            1.0,
            AttackMode::Normal,
            slot,
            0.0,
            None,
            &mut StdRng::seed_from_u64(4),
        );
        (hit, actors)
    }

    #[test]
    fn iron_anathema_adds_dice_without_multiplying_flat_bonuses_and_arborhide_replaces_it() {
        for (talents, mult) in [(vec![], 2), (vec!["barkskin", "arborhide"], 3)] {
            for iron in [false, true] {
                let (hit, _) = strike(
                    weapon(iron, true),
                    arborite(&talents, false),
                    WeaponSlot::Primary,
                );
                assert!(hit.hit);
                assert_eq!(
                    hit.damage_breakdown.unwrap().rolled_damage,
                    2 * if iron { mult } else { 1 } + 7
                );
                let mut actors = vec![weapon(iron, true), arborite(&talents, false)];
                let counter = resolve_style_strike(
                    &mut actors,
                    0,
                    1,
                    WeaponSlot::Primary,
                    1.0,
                    0.0,
                    &mut StdRng::seed_from_u64(4),
                )
                .unwrap();
                assert_eq!(
                    counter.damage_breakdown.unwrap().rolled_damage,
                    2 * if iron { mult } else { 1 } + 7
                );
            }
        }
    }

    #[test]
    fn iron_checkboxes_apply_independently_to_the_primary_and_offhand_strike() {
        let mut c = weapon(false, true);
        let offhand = weapon(true, true).sheet.offense.weapon.clone();
        c.sheet.offense.offhand = Some(OffhandProfile {
            weapon: offhand,
            attack_bonus: 1000,
            strength_damage: 0,
        });
        for (slot, expected) in [(WeaponSlot::Primary, 9), (WeaponSlot::Secondary, 13)] {
            let (hit, _) = strike(c.clone(), arborite(&["barkskin", "arborhide"], false), slot);
            assert_eq!(hit.damage_breakdown.unwrap().rolled_damage, expected);
        }
    }

    #[test]
    fn woad_requires_preparation_painting_and_a_positive_hacking_or_piercing_wound() {
        for (paint, hacking, blocked, expected) in [
            (false, true, false, false),
            (true, false, false, false),
            (true, true, true, false),
            (true, true, false, true),
        ] {
            let mut defender = arborite(&["warrior_woad"], paint);
            if blocked {
                defender.sheet.defense.armor_dr = 100;
            }
            let (_, actors) = strike(weapon(false, hacking), defender, WeaponSlot::Primary);
            assert_eq!(actors[1].state.has_active_effect(WOAD_EFFECT_ID), expected);
        }
    }

    #[test]
    fn woad_increases_the_strength_score_and_all_defenses_once_and_never_refreshes_from_more_wounds()
     {
        let mut c = arborite(&["warrior_woad"], true);
        let before = combat::preview_defense(&c);
        let base = c.magic.strength;
        let mut rng = StdRng::seed_from_u64(5);
        c.trigger_warrior_woad(1, true, &mut rng);
        let roll = c.state.arborite.woad_strength_roll.unwrap();
        assert!((1..=6).contains(&roll));
        let boosted =
            crate::character::AbilityScore::new((base.base + roll).min(25), base.percentile);
        let expected = crate::character::lookup_strength(&boosted).damage;
        assert_eq!(
            c.apply_i32(StatIdI32::StrengthDamageBase, c.magic.strength_damage),
            expected
        );
        let after = combat::preview_defense(&c);
        assert_eq!(after.melee_bonus - before.melee_bonus, 6);
        assert_eq!(after.ranged_bonus - before.ranged_bonus, 6);
        let duration = c
            .state
            .active_effects
            .iter()
            .find(|e| e.id == WOAD_EFFECT_ID)
            .unwrap()
            .remaining_seconds;
        assert!(duration >= 5);
        c.trigger_warrior_woad(1, true, &mut rng);
        assert_eq!(
            c.state
                .active_effects
                .iter()
                .filter(|e| e.id == WOAD_EFFECT_ID)
                .count(),
            1
        );
        assert_eq!(
            c.state
                .active_effects
                .iter()
                .find(|e| e.id == WOAD_EFFECT_ID)
                .unwrap()
                .remaining_seconds,
            duration
        );
        c.sheet.defense.shield_name = Some("Shield".into());
        c.sheet.defense.shield_defense_bonus = 20;
        let shielded = combat::preview_defense(&c);
        c.state.active_effects.clear();
        let without = combat::preview_defense(&c);
        assert_eq!(shielded.ranged_bonus - without.ranged_bonus, 6);
    }

    #[test]
    fn woad_expiry_rolls_both_saves_and_either_failure_causes_one_hour_of_unconsciousness() {
        let mut seen = std::collections::HashSet::new();
        for seed in 0..150 {
            let mut c = arborite(&["warrior_woad"], true);
            let mut rng = StdRng::seed_from_u64(seed);
            c.trigger_warrior_woad(1, true, &mut rng);
            c.state
                .active_effects
                .iter_mut()
                .find(|e| e.id == WOAD_EFFECT_ID)
                .unwrap()
                .remaining_seconds = 1;
            c.tick_racial_effects(&mut rng);
            c.state.tick_effects();
            let saves = c.state.arborite.woad_expiry_saved.unwrap();
            seen.insert(saves);
            assert_eq!(
                c.state.arborite.unconscious_seconds,
                if saves.0 && saves.1 { 0 } else { 3600 }
            );
            assert!(!c.state.has_active_effect(WOAD_EFFECT_ID));
            if !(saves.0 && saves.1) {
                assert!(!c.magic_can_attack(WeaponSlot::Primary, 100.0));
            }
        }
        assert_eq!(
            seen.len(),
            4,
            "both saves are independent and both must succeed"
        );
    }

    #[test]
    fn silent_defiance_is_optional_stops_actions_under_mental_compulsion_and_ends_when_released() {
        let mut c = arborite(&["silent_defiance"], false);
        c.set_mental_compulsion(true);
        assert!(c.silent_defiance_holding_still());
        assert!(!c.magic_can_attack(WeaponSlot::Primary, 0.0));
        c.arborite.choose_silent_defiance = false;
        assert!(!c.silent_defiance_holding_still());
        assert!(c.magic_can_attack(WeaponSlot::Primary, 0.0));
        c.arborite.choose_silent_defiance = true;
        c.set_mental_compulsion(false);
        assert!(c.magic_can_attack(WeaponSlot::Primary, 0.0));
        let mut without = arborite(&[], false);
        without.set_mental_compulsion(true);
        assert!(!without.silent_defiance_holding_still());
    }
}

#[cfg(test)]
mod additional_tests {
    use crate::core::rules::DamageExprCache;
    use crate::core::sim::{Combatant, WeaponSlot, combat};
    use std::sync::Arc;

    #[test]
    fn iron_target_damage_macros_match_the_dice_only_combat_multiplier() {
        let mut attacker = Combatant::default();
        let weapon = Arc::make_mut(&mut attacker.sheet.offense.weapon);
        weapon.has_weapon = true;
        weapon.is_iron = true;
        weapon.damage_expr = "2d1+7".into();
        weapon.damage_expr_cache = DamageExprCache::new("2d1+7");
        let mut target = Combatant::default();
        target.arborite.iron_dice_multiplier = 3;
        let macro_expr = combat::weapon_roll20_damage_expression(
            &attacker,
            &target,
            WeaponSlot::Primary,
            false,
            false,
        )
        .unwrap();
        assert_eq!(macro_expr, "6d1+7");
        assert_eq!(
            combat::weapon_damage_expression(
                &attacker,
                &target,
                &attacker.sheet.offense.weapon,
                false,
                false
            )
            .replace(' ', ""),
            "6d1+7"
        );
        for multiplier in [1, 2, 3] {
            let pool = DamageExprCache::new("2d4p+7");
            let triple = pool.with_multiplied_dice(multiplier);
            assert!(
                (triple.expected(false)
                    - (pool.expected(false) - 7.0) * f64::from(multiplier)
                    - 7.0)
                    .abs()
                    < 1e-8
            );
        }
    }

    #[test]
    fn silent_defiance_suppresses_arena_movement_and_attacks_without_freezing_the_opponent() {
        use crate::core::sim::{SimConfig, SimState};
        let mut held = Combatant::default();
        held.team_id = 0;
        held.arborite.silent_defiance = true;
        held.arborite.choose_silent_defiance = true;
        held.arborite.initially_compelled = true;
        held.sheet.mobility.move_speed = 20.0;
        let mut target = Combatant::default();
        target.team_id = 1;
        target.sheet.maneuvers.passive = true;
        let mut sim = SimState::new(SimConfig::new(30.0, 1.0));
        sim.reset_with_combatants(vec![held, target]);
        let start = sim.actors[0].position;
        sim.tick();
        assert_eq!(sim.actors[0].position, start);
        assert!(!sim.combatants[0].state.has_attacked);
        sim.combatants[0].set_mental_compulsion(false);
        sim.tick();
        assert_ne!(sim.actors[0].position, start);
    }
}
