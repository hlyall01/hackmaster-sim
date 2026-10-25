//! Equipment decisions run before combat each second, including during ranged aiming.
use super::*;
use crate::core::sim::{PendingWeaponSetup, WeaponSetup};

impl SimState {
    fn setup_approach_speed(&self, idx: usize, target: usize, distance: f32) -> f32 {
        let c = &self.combatants[idx];
        let speed = self.move_tiles(idx) as f32 * self.config.tile_size_ft.max(0.01);
        if c.sheet.defense.eyesmite {
            return if distance > 5.0 { speed } else { 0.0 };
        }
        let weapon = &c.sheet.offense.weapon;
        if weapon.uses_projectiles {
            if max_range_for_weapon(weapon).is_some_and(|range| distance <= range) {
                let from = self.actors[idx].position;
                let step = Self::step_away(from, self.actors[target].position);
                let tiles = (speed / self.config.tile_size_ft.max(0.01)).round() as i32;
                let dest = super::super::GridPos::new(
                    from.x + (step.x - from.x) * tiles,
                    from.y + (step.y - from.y) * tiles,
                )
                .clamp(self.config.grid_width, self.config.grid_height);
                return -(from.manhattan_distance(dest) as f32
                    * self.config.tile_size_ft.max(0.01));
            }
        }
        if distance > weapon.reach_ft.max(1.0) {
            speed
        } else {
            0.0
        }
    }

    /// Latest whole-second switch that still finishes drawing by first contact.
    fn should_draw_melee_for_engagement(&self, idx: usize, enemy: usize) -> bool {
        let c = &self.combatants[idx];
        let Some(setups) = c.weapon_setups.as_ref() else {
            return false;
        };
        if !setups.auto_switch_to_melee
            || c.state.weapon_setup.active != WeaponSetup::Ranged
            || c.state.weapon_setup.pending.is_some()
            || c.state.hp <= 0
            || c.racial_actions_blocked()
            || c.state.trauma_remaining_seconds > 0
            || c.state.magic.casting.is_some()
            || c.state.magic.channeling.is_some()
        {
            return false;
        }
        let opponent = &self.combatants[enemy];
        let distance = self.distance_between(idx, enemy).unwrap_or(f32::INFINITY);
        let mine = setups.melee.sheet.offense.weapon.reach_ft.max(1.0);
        let enemy_reach = opponent
            .apply_f32(
                StatIdF32::WeaponReach,
                opponent.sheet.offense.weapon.reach_ft,
            )
            .max(1.0);
        let contact = mine.max(enemy_reach);
        if distance <= contact {
            return true;
        }
        let enemy_approach = self.setup_approach_speed(enemy, idx, distance);
        let current_approach = self.setup_approach_speed(idx, enemy, distance);
        let next_distance = (distance - enemy_approach - current_approach).max(0.0);
        // Even a small, instant draw must start before entering melee; otherwise
        // the rule for drawing while engaged imposes a full weapon-speed wait.
        if enemy_approach > 0.0 && next_distance <= enemy_reach {
            return true;
        }
        // Drawing is a timed action. The actor holds position while drawing;
        // only the opponent closes until the weapon is ready.
        let closure_after_switch = enemy_approach;
        if closure_after_switch <= 0.0 {
            return false;
        }
        let contact_after_waiting =
            ((next_distance - contact).max(0.0) / closure_after_switch).ceil();
        contact_after_waiting < setups.melee_draw_seconds
    }

    fn request_weapon_setup(
        &mut self,
        idx: usize,
        enemy: usize,
        target: WeaponSetup,
        rule: Option<usize>,
    ) -> bool {
        let now = self.elapsed_seconds as f32;
        let c = &self.combatants[idx];
        let Some(setups) = c.weapon_setups.clone() else {
            return false;
        };
        if c.state.weapon_setup.pending.is_some()
            || c.state.weapon_setup.active == target
            || c.state.hp <= 0
            || c.state.trauma_remaining_seconds > 0
            || c.racial_actions_blocked()
            || c.state.magic.casting.is_some()
            || c.state.magic.channeling.is_some()
        {
            return false;
        }
        let enemy_weapon = &self.combatants[enemy].sheet.offense.weapon;
        let engaged = !enemy_weapon.uses_projectiles
            && self.distance_between(idx, enemy).unwrap_or(f32::INFINITY)
                <= enemy_weapon.reach_ft.max(1.0);
        let drawing_seconds = match target {
            WeaponSetup::Melee => setups.melee_draw_seconds,
            WeaponSetup::Ranged if c.state.weapon_setup.ranged_dropped => 1.0,
            WeaponSetup::Ranged => roll_damage_expr("d4p+2", &mut self.rng, false).max(0) as f32,
        };
        let profile = match target {
            WeaponSetup::Melee => &setups.melee,
            WeaponSetup::Ranged => &setups.ranged,
        };
        let ready = now + drawing_seconds;
        let attack_ready = (ready
            + if engaged && profile.sheet.offense.weapon.has_weapon {
                profile.sheet.offense.weapon.speed
            } else {
                0.0
            })
        .max(self.combatants[idx].state.weapon_setup.recoveries[target.index()].unwrap_or(0.0));
        let c = &mut self.combatants[idx];
        let old = c.state.weapon_setup.active;
        if old == WeaponSetup::Melee {
            c.state.weapon_setup.melee_shield_intact = c.state.shield_intact;
            if c.state.weapon_setup.has_attacked[old.index()] {
                let recovery = c
                    .state
                    .next_attack_time_primary
                    .into_iter()
                    .chain(c.state.next_attack_time_secondary)
                    .fold(
                        c.state.weapon_setup.recoveries[old.index()].unwrap_or(0.0),
                        f32::max,
                    );
                c.state.weapon_setup.recoveries[old.index()] = Some(recovery);
            }
        }
        if old == WeaponSetup::Ranged {
            c.state.weapon_setup.ranged_dropped = true;
        }
        c.state.clear_attack_timers();
        c.state.armeroci_opening_strike_available = false;
        c.state.shield_intact = false;
        c.install_weapon_setup_profile(&setups.drawing);
        // Drawing uses the actual unarmed reach and defense until the weapon
        // is ready; it cannot hold an enemy away with an undrawn polearm.
        c.state.weapon_setup.pending = Some(PendingWeaponSetup {
            target,
            equipment_ready_at: ready,
            attack_ready_at: attack_ready,
        });
        c.finish_weapon_setup(now);
        let action = match target {
            WeaponSetup::Melee => TacticalAction::EquipMelee,
            WeaponSetup::Ranged => TacticalAction::EquipRanged,
        };
        let message = if target == WeaponSetup::Melee {
            format!(
                "{} drops {} instantly and draws {}; ready at {:.0}s",
                c.sheet.name,
                setups.ranged.sheet.offense.weapon.name,
                setups.melee.sheet.offense.weapon.name,
                attack_ready
            )
        } else {
            format!(
                "{} retrieves {}; ready at {:.0}s",
                c.sheet.name, setups.ranged.sheet.offense.weapon.name, attack_ready
            )
        };
        self.record_tactical_directive(idx, enemy, &action, rule, Some(message));
        true
    }

    pub(super) fn advance_weapon_setups(&mut self, a: usize, b: usize) {
        let now = self.elapsed_seconds as f32;
        self.combatants[a].finish_weapon_setup(now);
        self.combatants[b].finish_weapon_setup(now);
        for (idx, enemy) in [(a, b), (b, a)] {
            if self.combatants[idx].weapon_setups.is_none() {
                continue;
            }
            let decision = evaluate_channel(
                &self.combatants[idx].tactical_policy,
                TacticalDecisionPoint::EverySecond,
                TacticalChannel::WeaponSetup,
                &self.tactical_context_for_channel(idx, enemy, Some(TacticalChannel::WeaponSetup)),
            );
            match decision.action {
                TacticalAction::EquipMelee => {
                    self.request_weapon_setup(
                        idx,
                        enemy,
                        WeaponSetup::Melee,
                        decision.matched_rule_index,
                    );
                }
                TacticalAction::EquipRanged => {
                    self.request_weapon_setup(
                        idx,
                        enemy,
                        WeaponSetup::Ranged,
                        decision.matched_rule_index,
                    );
                }
                TacticalAction::RetainWeaponSetup if decision.matched_rule_index.is_some() => {}
                _ => {
                    if self.should_draw_melee_for_engagement(idx, enemy) {
                        // Firing a readied shot and dropping the bow are both at
                        // this second; do not throw away a free final shot.
                        let distance = self.distance_between(idx, enemy).unwrap_or(0.0);
                        let opponent = &self.combatants[enemy];
                        let safely_outside_melee =
                            distance > opponent.sheet.offense.weapon.reach_ft.max(1.0);
                        let actor = &self.combatants[idx];
                        let readied = actor
                            .state
                            .next_attack_time_primary
                            .is_some_and(|t| t <= now)
                            || (actor.state.next_attack_time_primary.is_none()
                                && actor.sheet.offense.weapon.ranged_styles.aim_seconds() == 0.0
                                && !actor.sheet.maneuvers.called_shot);
                        if readied && safely_outside_melee && !self.predicting_movement {
                            self.resolve_combat_round(a, b);
                        }
                        if self.should_draw_melee_for_engagement(idx, enemy) {
                            self.request_weapon_setup(idx, enemy, WeaponSetup::Melee, None);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::sim::{CombatEventKind, Combatant, GridPos, RangedWeaponSetup, WeaponSetup};
    use crate::{
        core::{
            tactics::{TacticalCondition, TacticalPolicy, TacticalRule},
            types::TalentSelection,
        },
        data, game_logic as logic,
    };
    use std::sync::Arc;

    pub(super) fn fixture(auto: bool, grave: bool) -> Combatant {
        let (w, a, s) = data::load_catalogs().unwrap();
        let t = data::load_talents(data::TALENTS_PATH).unwrap();
        let id = w
            .id_from_index(
                w.entries()
                    .iter()
                    .position(|w| w.name == "Halberd")
                    .unwrap(),
            )
            .unwrap();
        let mut p = logic::PlayerConfig::new("Archer", id);
        p.level = 10;
        p.strength_base = 16;
        p.dex_base = 16;
        p.intelligence = 12;
        p.wisdom = 13;
        p.constitution = 16;
        p.move_speed = 20.0;
        p.proficiencies = vec!["Halberd".into(), "Shortbow".into()];
        p.talents = vec![TalentSelection {
            id: "armeroci_pole".into(),
            rank: 1,
            weapon: None,
        }];
        p.default_weapon_style_ids = Some(vec!["armeroci_pole".into()]);
        if grave {
            p.talents.push(TalentSelection {
                id: "gravesend".into(),
                rank: 1,
                weapon: None,
            });
            p.ranged_styles.gravesend_aim = crate::core::sim::GravesendAim::ForceTrauma;
        }
        p.ranged_weapon_setup = Some(RangedWeaponSetup {
            weapon: "Shortbow".into(),
            material_tier: 2,
            projectile_material_tier: 1,
            iron: true,
            default_style_ids: Some(if grave {
                vec!["gravesend".into()]
            } else {
                vec![]
            }),
            start_with: WeaponSetup::Ranged,
            auto_switch_to_melee: auto,
        });
        let mut c =
            logic::build_combatant(&p, &w, &a, &s, &logic::NpcPresetCatalog::new(vec![]), &t);
        // Avoid unrelated critical/instant-death noise in scheduling assertions.
        let setups = Arc::make_mut(c.weapon_setups.as_mut().unwrap());
        for profile in [&mut setups.melee, &mut setups.ranged] {
            Arc::make_mut(&mut profile.sheet.offense.weapon).crit_min_roll = 21;
        }
        Arc::make_mut(&mut c.sheet.offense.weapon).crit_min_roll = 21;
        c
    }
    pub(super) fn arena(c: Combatant, distance: i32, at_edge: bool) -> SimState {
        let mut enemy = Combatant::default();
        enemy.team_id = 1;
        enemy.sheet.mobility.move_speed = 20.0;
        enemy.sheet.vitals.max_hp = 10000;
        enemy.sheet.vitals.threshold_of_pain = 10000;
        enemy.sheet.offense.attack_bonus = -1000;
        enemy.sheet.defense.knockback_step = 10000;
        let w = Arc::make_mut(&mut enemy.sheet.offense.weapon);
        w.has_weapon = true;
        w.is_unarmed = false;
        w.reach_ft = 3.0;
        w.speed = 50.0;
        let mut config = SimConfig::new(distance as f32, 1.0);
        config.grid_width = 1000;
        let mut sim = SimState::with_rng(config, SimRng::from_seed(151));
        sim.log_events = true;
        sim.reset_with_combatants(vec![c, enemy]);
        let x = if at_edge { 0 } else { 400 };
        sim.actors[0].position = GridPos::new(x, 5);
        sim.actors[1].position = GridPos::new(x + distance, 5);
        sim
    }

    #[test]
    fn automatic_switch_is_ready_for_first_melee_exchange_and_preserves_opening_attack() {
        for edge in [false, true] {
            for distance in [30, 55, 100, 150] {
                let mut sim = arena(fixture(true, false), distance, edge);
                let mut first_melee = None;
                for _ in 0..60 {
                    sim.tick();
                    if let Some(event) = sim.combat_events.iter().find(|e| {
                        e.attacker_idx == 0
                            && matches!(&e.kind,CombatEventKind::Attack(a) if !a.is_ranged)
                    }) {
                        first_melee = Some(event.time);
                        break;
                    }
                }
                let time = first_melee.expect("automatic draw must reach melee");
                let swap=sim.combat_events.iter().find(|e|e.attacker_idx==0 && matches!(&e.kind,CombatEventKind::Tactical(a) if a.action=="Equip melee setup")).unwrap();
                let draw = sim.combatants[0]
                    .weapon_setups
                    .as_ref()
                    .unwrap()
                    .melee_draw_seconds;
                assert_eq!(
                    time,
                    swap.time + draw as u32,
                    "edge {edge}, distance {distance}"
                );
                let first_enemy = sim.combat_events.iter().find(|e| {
                    e.attacker_idx == 1
                        && matches!(&e.kind,CombatEventKind::Attack(a) if !a.is_ranged)
                });
                assert!(
                    first_enemy.is_none_or(|e| e.time >= time),
                    "edge {edge}, distance {distance}, melee at {time}, enemy {:?}",
                    first_enemy.map(|e| e.time)
                );
                assert_eq!(
                    sim.combatants[0].state.weapon_setup.active,
                    WeaponSetup::Melee
                );
                assert!(sim.combatants[0].state.weapon_setup.ranged_dropped);
                assert!(
                    sim.combatants[0].state.weapon_setup.has_attacked[WeaponSetup::Ranged.index()]
                );
                assert!(
                    sim.combatants[0].state.weapon_setup.has_attacked[WeaponSetup::Melee.index()]
                );
                assert!(
                    !sim.combatants[0].state.armeroci_opening_strike_available,
                    "the engagement attack consumed the opening bonus"
                );
            }
        }
    }

    #[test]
    fn dropping_is_instant_and_draw_starts_at_the_last_safe_second() {
        let mut sim = arena(fixture(true, false), 100, false);
        while !sim.should_draw_melee_for_engagement(0, 1) {
            sim.tick();
            assert!(sim.elapsed_seconds < 30);
        }
        let now = sim.elapsed_seconds as f32;
        let draw = sim.combatants[0]
            .weapon_setups
            .as_ref()
            .unwrap()
            .melee_draw_seconds;
        sim.tick();
        let pending = sim.combatants[0]
            .state
            .weapon_setup
            .pending
            .as_ref()
            .unwrap();
        assert_eq!(
            pending.equipment_ready_at,
            now + draw,
            "no time is added for dropping the bow"
        );
        assert_eq!(
            pending.attack_ready_at, pending.equipment_ready_at,
            "drawing before engagement avoids a weapon-speed wait"
        );
    }

    #[test]
    fn disabled_automatic_swap_keeps_using_the_ranged_weapon() {
        let mut sim = arena(fixture(false, false), 15, false);
        for _ in 0..20 {
            sim.tick();
        }
        assert_eq!(
            sim.combatants[0].state.weapon_setup.active,
            WeaponSetup::Ranged
        );
        assert!(!sim.combatants[0].state.weapon_setup.ranged_dropped);
        assert!(!sim.combatants[0].state.weapon_setup.has_attacked[WeaponSetup::Melee.index()]);
    }

    #[test]
    fn equipment_tactics_cancel_gravesend_aim_without_waiting_for_the_shot() {
        let mut sim = arena(fixture(false, true), 120, false);
        sim.combatants[1].sheet.maneuvers.passive = true;
        sim.tick();
        assert_eq!(sim.combatants[0].state.next_attack_time_primary, Some(20.0));
        sim.combatants[0].tactical_policy = TacticalPolicy {
            enabled: true,
            rules: vec![TacticalRule::new(
                TacticalAction::EquipMelee,
                vec![TacticalCondition::Always],
            )],
        };
        sim.tick();
        assert_eq!(
            sim.combatants[0]
                .state
                .weapon_setup
                .pending
                .as_ref()
                .unwrap()
                .target,
            WeaponSetup::Melee
        );
        assert_eq!(
            sim.combatants[0]
                .state
                .weapon_setup
                .pending
                .as_ref()
                .unwrap()
                .equipment_ready_at,
            3.0
        );
        assert!(!sim.combatants[0].state.weapon_setup.has_attacked[WeaponSetup::Ranged.index()]);
        assert!(sim.combatants[0].state.next_attack_time_primary.is_none());
    }

    #[test]
    fn explicit_keep_ranged_tactic_overrides_automatic_switch() {
        let mut c = fixture(true, false);
        c.tactical_policy = TacticalPolicy {
            enabled: true,
            rules: vec![TacticalRule::new(
                TacticalAction::EquipRanged,
                vec![TacticalCondition::Always],
            )],
        };
        let mut sim = arena(c, 15, false);
        for _ in 0..10 {
            sim.tick();
        }
        assert_eq!(
            sim.combatants[0].state.weapon_setup.active,
            WeaponSetup::Ranged
        );
        assert!(sim.combatants[0].state.weapon_setup.pending.is_none());
    }

    #[test]
    fn swapping_does_not_heal_or_erase_effects_and_reset_restores_the_starting_setup() {
        let mut sim = arena(fixture(false, false), 100, false);
        sim.combatants[0].state.hp = 23;
        sim.combatants[0]
            .state
            .add_effect(crate::core::sim::TemporaryEffect::new("test", 100));
        assert!(sim.request_weapon_setup(0, 1, WeaponSetup::Melee, None));
        let pending = sim.combatants[0]
            .state
            .weapon_setup
            .pending
            .clone()
            .unwrap();
        assert!(sim.combatants[0].finish_weapon_setup(pending.equipment_ready_at));
        assert_eq!(sim.combatants[0].state.hp, 23);
        assert!(sim.combatants[0].state.has_active_effect("test"));
        let recovery = sim.combatants[0].state.next_attack_time_primary;
        assert!(!sim.request_weapon_setup(0, 1, WeaponSetup::Melee, None));
        assert_eq!(sim.combatants[0].state.next_attack_time_primary, recovery);
        sim.reset();
        assert_eq!(
            sim.combatants[0].state.weapon_setup.active,
            WeaponSetup::Ranged
        );
        assert_eq!(sim.combatants[0].sheet.offense.weapon.name, "Shortbow");
        assert!(!sim.combatants[0].state.weapon_setup.ranged_dropped);
    }

    #[test]
    fn dropped_bow_retrieval_has_no_location_requirement_and_preserves_its_recovery() {
        let mut sim = arena(fixture(false, false), 100, false);
        sim.tick();
        let recovery =
            sim.combatants[0].state.weapon_setup.recoveries[WeaponSetup::Ranged.index()].unwrap();
        assert!(sim.request_weapon_setup(0, 1, WeaponSetup::Melee, None));
        let ready = sim.combatants[0]
            .state
            .weapon_setup
            .pending
            .as_ref()
            .unwrap()
            .equipment_ready_at;
        sim.combatants[0].finish_weapon_setup(ready);
        sim.actors[0].position.x += 100;
        assert!(sim.request_weapon_setup(0, 1, WeaponSetup::Ranged, None));
        let pending = sim.combatants[0]
            .state
            .weapon_setup
            .pending
            .as_ref()
            .unwrap();
        assert_eq!(pending.equipment_ready_at, sim.elapsed_seconds as f32 + 1.0);
        assert!(pending.attack_ready_at >= recovery);
    }

    #[test]
    fn already_caught_in_melee_pays_readiness_without_getting_an_early_attack() {
        let mut sim = arena(fixture(true, false), 2, false);
        let speed = sim.combatants[0]
            .weapon_setups
            .as_ref()
            .unwrap()
            .melee
            .sheet
            .offense
            .weapon
            .speed;
        sim.tick();
        assert_eq!(
            sim.combatants[0]
                .state
                .weapon_setup
                .pending
                .as_ref()
                .unwrap()
                .attack_ready_at,
            2.0 + speed
        );
        for _ in 0..(2.0 + speed).ceil() as usize - 1 {
            sim.tick();
        }
        assert!(!sim.combatants[0].state.weapon_setup.has_attacked[WeaponSetup::Melee.index()]);
    }
}

#[cfg(test)]
mod opening_and_shield_tests {
    use super::tests::{arena, fixture};
    use super::*;
    use crate::core::sim::WeaponSetup;
    use std::sync::Arc;

    fn finish_draw(sim: &mut SimState) {
        let ready = sim.combatants[0]
            .state
            .weapon_setup
            .pending
            .as_ref()
            .unwrap()
            .equipment_ready_at;
        sim.elapsed_seconds = ready.ceil() as u32;
        assert!(sim.combatants[0].finish_weapon_setup(ready));
    }

    #[test]
    fn earlier_bow_shots_do_not_consume_the_first_melee_opening_bonus() {
        let mut sim = arena(fixture(false, false), 100, false);
        sim.tick();
        assert!(sim.combatants[0].state.has_attacked);
        assert!(sim.request_weapon_setup(0, 1, WeaponSetup::Melee, None));
        finish_draw(&mut sim);
        assert!(
            sim.combatants[0]
                .sheet
                .modifiers
                .apply_i32(0, StatIdI32::FlagArmerociPoleStyle)
                > 0
        );
        assert!(sim.combatants[0].state.armeroci_opening_strike_available);
        assert!(!sim.combatants[0].state.weapon_setup.has_attacked[WeaponSetup::Melee.index()]);
    }

    #[test]
    fn an_inactive_shield_gives_no_bow_defense_and_a_broken_shield_is_not_repaired_by_switching() {
        let mut c = fixture(false, false);
        let setup = Arc::make_mut(c.weapon_setups.as_mut().unwrap());
        setup.melee.sheet.defense.shield_name = Some("Shield".into());
        setup.melee.sheet.defense.shield_defense_bonus = 4;
        let mut sim = arena(c, 100, false);
        assert!(!sim.combatants[0].state.shield_intact);
        assert!(sim.request_weapon_setup(0, 1, WeaponSetup::Melee, None));
        finish_draw(&mut sim);
        assert!(sim.combatants[0].state.shield_intact);
        sim.combatants[0].state.shield_intact = false;
        assert!(sim.request_weapon_setup(0, 1, WeaponSetup::Ranged, None));
        finish_draw(&mut sim);
        assert!(!sim.combatants[0].state.shield_intact);
        assert!(sim.request_weapon_setup(0, 1, WeaponSetup::Melee, None));
        finish_draw(&mut sim);
        assert!(!sim.combatants[0].state.shield_intact);
    }

    #[test]
    fn retrieving_a_bow_restarts_gravesend_aim_instead_of_reusing_the_abandoned_shot() {
        let mut sim = arena(fixture(false, true), 120, false);
        sim.combatants[1].sheet.maneuvers.passive = true;
        sim.tick();
        assert_eq!(sim.combatants[0].state.next_attack_time_primary, Some(20.0));
        assert!(sim.request_weapon_setup(0, 1, WeaponSetup::Melee, None));
        finish_draw(&mut sim);
        assert!(sim.request_weapon_setup(0, 1, WeaponSetup::Ranged, None));
        finish_draw(&mut sim);
        let expected = sim.elapsed_seconds as f32 + 20.0;
        sim.tick();
        assert_eq!(
            sim.combatants[0].state.next_attack_time_primary,
            Some(expected)
        );
    }
}
