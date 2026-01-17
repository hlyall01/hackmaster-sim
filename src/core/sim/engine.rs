use crate::core::rng::SimRng;

use super::combat::{resolve_attack, resolve_knock_aside, AttackMode};
use super::modifiers::StatIdF32;
use super::movement::{max_range_for_weapon, range_modifier_for_weapon_with_scale};
use super::types::{
    AttackEvent, CombatEvent, CombatEventKind, Combatant, KnockAsideEvent, SimActor, SimConfig,
    WeaponSlot,
};

#[derive(Clone, Debug)]
pub struct SimState {
    pub config: SimConfig,
    pub actors: [SimActor; 2],
    pub combatants: [Combatant; 2],
    pub elapsed_seconds: u32,
    pub done: bool,
    pub last_event: Option<CombatEvent>,
    pub combat_events: Vec<CombatEvent>,
    pub log_events: bool,
    rng: SimRng,
    tick_accum: f32,
    hold_at_bay: HoldAtBayState,
}

#[derive(Clone, Copy, Debug, Default)]
struct HoldAtBayState {
    pending: bool,
    active: bool,
    holder_idx: usize,
    target_idx: usize,
}

impl SimState {
    pub fn new(config: SimConfig) -> Self {
        Self::with_rng_and_log(config, SimRng::default(), true)
    }

    pub fn with_logging(config: SimConfig, log_events: bool) -> Self {
        Self::with_rng_and_log(config, SimRng::default(), log_events)
    }

    pub fn with_rng(config: SimConfig, rng: SimRng) -> Self {
        Self::with_rng_and_log(config, rng, true)
    }

    fn with_rng_and_log(config: SimConfig, rng: SimRng, log_events: bool) -> Self {
        Self {
            config,
            actors: [
                SimActor { position: 0.0 },
                SimActor {
                    position: config.start_distance,
                },
            ],
            combatants: [Combatant::default(), Combatant::default()],
            elapsed_seconds: 0,
            done: false,
            last_event: None,
            combat_events: Vec::new(),
            log_events,
            rng,
            tick_accum: 0.0,
            hold_at_bay: HoldAtBayState::default(),
        }
    }

    pub fn reset(&mut self) {
        self.actors[0].position = 0.0;
        self.actors[1].position = self.config.start_distance;
        self.elapsed_seconds = 0;
        self.done = false;
        self.last_event = None;
        self.combat_events.clear();
        for combatant in &mut self.combatants {
            combatant.reset_state();
        }
        self.tick_accum = 0.0;
        self.hold_at_bay = HoldAtBayState::default();
    }

    #[allow(dead_code)]
    pub fn reset_preserve_rng(&mut self) {
        self.reset();
    }

    pub fn reset_with_combatants(&mut self, combatants: [Combatant; 2]) {
        self.combatants = combatants;
        self.reset();
    }

    #[allow(dead_code)]
    pub fn reset_with_combatants_preserve_rng(&mut self, combatants: [Combatant; 2]) {
        self.combatants = combatants;
        self.reset_preserve_rng();
    }

    pub fn set_rng(&mut self, rng: SimRng) {
        self.rng = rng;
    }

    pub fn update(&mut self, dt: f32) {
        if self.done {
            return;
        }
        self.tick_accum += dt;
        while self.tick_accum >= 1.0 {
            self.tick_accum -= 1.0;
            self.tick();
            if self.done {
                break;
            }
        }
    }

    pub fn tick(&mut self) {
        if self.done {
            return;
        }
        for combatant in &mut self.combatants {
            combatant.state.knockback_applied_this_tick = false;
            combatant.state.tick_effects();
            if combatant.state.trauma_remaining_seconds > 0 {
                combatant.state.trauma_remaining_seconds -= 1;
            }
            if combatant.state.knockback_immobile_seconds > 0 {
                combatant.state.knockback_immobile_seconds -= 1;
            }
        }
        let old_positions = [self.actors[0].position, self.actors[1].position];
        let distance = self.distance();
        let distance_before_combat = distance;
        let reach_a = self
            .combatants[0]
            .apply_f32(StatIdF32::WeaponReach, self.combatants[0].sheet.offense.weapon.reach_ft)
            .max(1.0);
        let reach_b = self
            .combatants[1]
            .apply_f32(StatIdF32::WeaponReach, self.combatants[1].sheet.offense.weapon.reach_ft)
            .max(1.0);
        let max_reach = self.config.stop_distance.max(1.0);
        let min_reach = reach_a.min(reach_b);
        let weapon_a = self.combatants[0].sheet.offense.weapon.clone();
        let weapon_b = self.combatants[1].sheet.offense.weapon.clone();
        let ranged_projectile_a = weapon_a.uses_projectiles;
        let ranged_projectile_b = weapon_b.uses_projectiles;
        let max_range_a = max_range_cached(
            &mut self.combatants[0].state,
            WeaponSlot::Primary,
            weapon_a.as_ref(),
        );
        let max_range_b = max_range_cached(
            &mut self.combatants[1].state,
            WeaponSlot::Primary,
            weapon_b.as_ref(),
        );
        let ranged_a = max_range_a.is_some();
        let ranged_b = max_range_b.is_some();
        let ranged_projectile_a = ranged_a && ranged_projectile_a;
        let ranged_projectile_b = ranged_b && ranged_projectile_b;
        let any_ranged = ranged_a || ranged_b;

        if distance > max_reach && !any_ranged {
            let step_a = self.move_step(0);
            let step_b = self.move_step(1);
            self.actors[0].position += step_a;
            self.actors[1].position -= step_b;
            for combatant in &mut self.combatants {
                combatant.state.clear_attack_timers();
            }
        } else {
            self.resolve_combat_round();
            let distance = self.distance();
            let step_a = self.move_step(0);
            let step_b = self.move_step(1);
            if any_ranged {
                let backstep_a = step_a;
                let backstep_b = step_b;
                let engaged = distance <= min_reach;
                if !engaged {
                    if ranged_projectile_a {
                        if let Some(max_range) = max_range_a {
                            if distance <= max_range {
                                self.actors[0].position -= backstep_a;
                            } else {
                                self.actors[0].position += step_a;
                            }
                        }
                    } else if distance > reach_a {
                        self.actors[0].position += step_a;
                    }
                    if ranged_projectile_b {
                        if let Some(max_range) = max_range_b {
                            if distance <= max_range {
                                self.actors[1].position += backstep_b;
                            } else {
                                self.actors[1].position -= step_b;
                            }
                        }
                    } else if distance > reach_b {
                        self.actors[1].position -= step_b;
                    }
                }
            } else if distance > min_reach {
                if reach_a < reach_b {
                    if !self.hold_at_bay.blocks_advance(0) {
                        self.actors[0].position += step_a;
                    }
                } else if reach_b < reach_a {
                    if !self.hold_at_bay.blocks_advance(1) {
                        self.actors[1].position -= step_b;
                    }
                }
            }
        }
        let distance_after_combat = self.distance();
        if max_range_a.is_some()
            && !weapon_a.uses_projectiles
            && distance_before_combat > reach_a
            && distance_after_combat <= reach_a
        {
            self.combatants[0].state.clear_attack_timers();
        }
        if max_range_b.is_some()
            && !weapon_b.uses_projectiles
            && distance_before_combat > reach_b
            && distance_after_combat <= reach_b
        {
            self.combatants[1].state.clear_attack_timers();
        }
        self.maybe_start_hold_at_bay(distance_before_combat, distance_after_combat, reach_a, reach_b);
        if self
            .combatants
            .iter()
            .any(|combatant| combatant.state.knockback_applied_this_tick)
            && self.distance() < distance_before_combat
        {
            self.actors[1].position = self.actors[0].position + distance_before_combat;
        }
        if self.actors[0].position > self.actors[1].position {
            let midpoint = (self.actors[0].position + self.actors[1].position) * 0.5;
            self.actors[0].position = midpoint;
            self.actors[1].position = midpoint;
        }
        for (idx, combatant) in self.combatants.iter_mut().enumerate() {
            combatant.state.moved_last_tick =
                (self.actors[idx].position - old_positions[idx]).abs() > f32::EPSILON;
        }
        self.elapsed_seconds += 1;
        let now = self.elapsed_seconds as f32;
        for combatant in &mut self.combatants {
            combatant
                .state
                .refresh_defense_plus_four_ready(&combatant.sheet, now);
        }
    }

    pub fn distance(&self) -> f32 {
        (self.actors[1].position - self.actors[0].position).max(0.0)
    }

    fn move_step(&self, idx: usize) -> f32 {
        let combatant = &self.combatants[idx];
        if combatant.state.trauma_remaining_seconds > 0
            || combatant.state.knockback_immobile_seconds > 0
        {
            0.0
        } else {
            combatant
                .apply_f32(StatIdF32::MoveSpeed, combatant.sheet.mobility.move_speed)
                .max(0.0)
        }
    }

    fn apply_knockback(&mut self, attacker_idx: usize, defender_idx: usize, knockback_ft: f32) {
        if knockback_ft <= 0.0 {
            return;
        }
        if let Some(defender) = self.combatants.get_mut(defender_idx) {
            defender.state.knockback_applied_this_tick = true;
        }
        match (attacker_idx, defender_idx) {
            (0, 1) => {
                self.actors[1].position += knockback_ft;
            }
            (1, 0) => {
                self.actors[0].position = (self.actors[0].position - knockback_ft).max(0.0);
            }
            _ => {}
        }
    }

    fn resolve_combat_round(&mut self) {
        let now = self.elapsed_seconds as f32;
        let distance = self.distance();
        let reach_a = self
            .combatants[0]
            .apply_f32(StatIdF32::WeaponReach, self.combatants[0].sheet.offense.weapon.reach_ft)
            .max(1.0);
        let reach_b = self
            .combatants[1]
            .apply_f32(StatIdF32::WeaponReach, self.combatants[1].sheet.offense.weapon.reach_ft)
            .max(1.0);
        let simultaneous = (reach_a - reach_b).abs() < f32::EPSILON;
        let state_snapshot = if simultaneous {
            Some([self.combatants[0].state.clone(), self.combatants[1].state.clone()])
        } else {
            None
        };
        let alive_start = [
            self.combatants[0].state.hp > 0,
            self.combatants[1].state.hp > 0,
        ];
        for (attacker_idx, defender_idx) in [(0usize, 1usize), (1usize, 0usize)] {
            let snapshot_next_attack_primary = state_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot[attacker_idx].next_attack_time_primary);
            let snapshot_next_attack_secondary = state_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot[attacker_idx].next_attack_time_secondary);
            let use_snapshot_timing = state_snapshot.is_some();
            let attacker_alive = if simultaneous {
                alive_start[attacker_idx]
            } else {
                self.combatants[attacker_idx].state.hp > 0
            };
            let defender_alive = if simultaneous {
                alive_start[defender_idx]
            } else {
                self.combatants[defender_idx].state.hp > 0
            };
            let attacker_trauma = if let Some(snapshot) = state_snapshot.as_ref() {
                snapshot[attacker_idx].trauma_remaining_seconds > 0
            } else {
                self.combatants[attacker_idx].state.trauma_remaining_seconds > 0
            };
            if self.hold_at_bay.active && self.hold_at_bay.holder_idx == attacker_idx {
                if attacker_trauma {
                    self.combatants[attacker_idx].state.clear_attack_timers();
                    continue;
                }
            }
            if !attacker_alive || !defender_alive {
                self.clear_hold_at_bay_if_involves(attacker_idx, defender_idx);
                continue;
            }
            if attacker_trauma {
                self.combatants[attacker_idx].state.clear_attack_timers();
                continue;
            }
            if self.hold_at_bay.active && self.hold_at_bay.target_idx == attacker_idx {
                let weapon_speed = self.combatants[attacker_idx]
                    .apply_f32(
                        StatIdF32::WeaponSpeed,
                        self.combatants[attacker_idx].sheet.offense.weapon.speed,
                    )
                    .max(1.0);
                if snapshot_next_attack_primary.is_none()
                    && self.combatants[attacker_idx]
                        .state
                        .next_attack_time_primary
                        .is_none()
                {
                    self.combatants[attacker_idx]
                        .state
                        .set_next_attack_time(WeaponSlot::Primary, Some(now));
                    self.combatants[attacker_idx]
                        .state
                        .set_next_attack_time(WeaponSlot::Secondary, None);
                }
                let next_attack = if use_snapshot_timing {
                    snapshot_next_attack_primary.unwrap_or(now)
                } else {
                    self.combatants[attacker_idx]
                        .state
                        .next_attack_time_primary
                        .unwrap_or(now)
                };
                if now + 0.0001 >= next_attack {
                    let event = resolve_knock_aside(
                        &mut self.combatants,
                        attacker_idx,
                        defender_idx,
                        now,
                        state_snapshot.as_ref(),
                        &mut self.rng,
                    );
                    if event.success {
                        self.hold_at_bay = HoldAtBayState::default();
                        self.combatants[attacker_idx]
                            .state
                            .set_next_attack_time(WeaponSlot::Primary, Some(now + 1.0));
                    } else {
                        self.combatants[attacker_idx]
                            .state
                            .set_next_attack_time(
                                WeaponSlot::Primary,
                                Some(next_attack + weapon_speed),
                            );
                    }
                    if self.log_events {
                        let event_struct = CombatEvent {
                            time: self.elapsed_seconds,
                            attacker_idx,
                            defender_idx,
                            kind: CombatEventKind::KnockAside(KnockAsideEvent {
                                success: event.success,
                                roll: event.roll,
                            }),
                        };
                        self.last_event = Some(event_struct.clone());
                        self.combat_events.push(event_struct);
                    }
                }
                continue;
            }
            let weapon = &self.combatants[attacker_idx].sheet.offense.weapon;
            let max_range = max_range_cached(
                &mut self.combatants[attacker_idx].state,
                WeaponSlot::Primary,
                weapon.as_ref(),
            );
            let has_range = max_range.is_some();
            let attacker_reach = weapon.reach_ft.max(1.0);
                let mut use_ranged = if has_range && !weapon.uses_projectiles {
                    distance > attacker_reach
                } else {
                    has_range
                };
            let mut attack_mode = AttackMode::Normal;
            if self.hold_at_bay.pending && self.hold_at_bay.holder_idx == attacker_idx {
                let defender_reach =
                    self.combatants[defender_idx].sheet.offense.weapon.reach_ft.max(1.0);
                if attacker_reach > defender_reach && distance <= attacker_reach {
                    attack_mode = AttackMode::HoldAtBay;
                    use_ranged = false;
                } else {
                    self.hold_at_bay = HoldAtBayState::default();
                }
            } else if self.hold_at_bay.active && self.hold_at_bay.holder_idx == attacker_idx {
                attack_mode = AttackMode::HoldAtBay;
                use_ranged = false;
            }
            let ranged_mod = if use_ranged {
                let range_scale = self.combatants[attacker_idx].apply_f32(
                    StatIdF32::RangeDistanceMultiplier,
                    weapon.range_distance_multiplier,
                );
                range_modifier_for_weapon_with_scale(weapon.as_ref(), distance, range_scale)
            } else {
                None
            };
            let primary_speed_base = self.combatants[attacker_idx]
                .apply_f32(StatIdF32::WeaponSpeed, weapon.speed)
                .max(1.0);
            let mut primary_attack_time = None;
            if !use_ranged && distance > attacker_reach {
                continue;
            }
            if use_ranged && ranged_mod.is_none() {
                continue;
            }
            if self.combatants[attacker_idx]
                .state
                .next_attack_time_primary
                .is_none()
            {
                let defender_reach = self.combatants[defender_idx].sheet.offense.weapon.reach_ft;
                let delay = if !use_ranged && attacker_reach < defender_reach {
                    1.0
                } else {
                    0.0
                };
                self.combatants[attacker_idx]
                    .state
                    .set_next_attack_time(WeaponSlot::Primary, Some(now + delay));
            }
            let next_attack = if use_snapshot_timing {
                snapshot_next_attack_primary.unwrap_or(now)
            } else {
                self.combatants[attacker_idx]
                    .state
                    .next_attack_time_primary
                    .unwrap_or(now)
            };
            if now + 0.0001 >= next_attack {
                primary_attack_time = Some(next_attack);
                let mut event = resolve_attack(
                    &mut self.combatants,
                    attacker_idx,
                    defender_idx,
                    ranged_mod.unwrap_or(0),
                    use_ranged,
                    distance,
                    attack_mode,
                    WeaponSlot::Primary,
                    now,
                    state_snapshot.as_ref(),
                    &mut self.rng,
                );
                if attack_mode == AttackMode::HoldAtBay && self.hold_at_bay.pending {
                    if event.hit {
                        self.hold_at_bay.active = true;
                        self.hold_at_bay.pending = false;
                    } else {
                        self.hold_at_bay = HoldAtBayState::default();
                    }
                }
                self.apply_knockback(
                    event.attacker_idx,
                    event.defender_idx,
                    event.knockback_ft,
                );
                if self.log_events {
                    let event_struct = CombatEvent {
                        time: self.elapsed_seconds,
                        attacker_idx: event.attacker_idx,
                        defender_idx: event.defender_idx,
                        kind: CombatEventKind::Attack(AttackEvent {
                            hit: event.hit,
                            shield_block: event.shield_block,
                            damage: event.damage,
                            shield_damage: event.shield_damage,
                            knockback_ft: event.knockback_ft,
                            hold_at_bay: event.hold_at_bay,
                            use_jab: event.use_jab,
                            is_ranged: event.is_ranged,
                            trauma_applied: event.trauma_applied,
                            trauma_seconds: event.trauma_seconds,
                            roll: event.roll,
                            damage_breakdown: event.damage_breakdown,
                            shield_damage_breakdown: event.shield_damage_breakdown,
                            defender_hp_after: event.defender_hp_after,
                            critical: event.critical,
                        }),
                    };
                    self.last_event = Some(event_struct.clone());
                    self.combat_events.push(event_struct);
                }
                if let Some(counter) = event.counter_attack.take() {
                    self.apply_knockback(
                        counter.attacker_idx,
                        counter.defender_idx,
                        counter.knockback_ft,
                    );
                    if self.log_events {
                        let counter_event = CombatEvent {
                            time: self.elapsed_seconds,
                            attacker_idx: counter.attacker_idx,
                            defender_idx: counter.defender_idx,
                            kind: CombatEventKind::Attack(AttackEvent {
                                hit: counter.hit,
                                shield_block: counter.shield_block,
                                damage: counter.damage,
                                shield_damage: counter.shield_damage,
                                knockback_ft: counter.knockback_ft,
                                hold_at_bay: false,
                                use_jab: counter.use_jab,
                                is_ranged: counter.is_ranged,
                                trauma_applied: counter.trauma_applied,
                                trauma_seconds: counter.trauma_seconds,
                                roll: counter.roll,
                                damage_breakdown: counter.damage_breakdown,
                                shield_damage_breakdown: counter.shield_damage_breakdown,
                                defender_hp_after: counter.defender_hp_after,
                                critical: counter.critical,
                            }),
                        };
                        self.last_event = Some(counter_event.clone());
                        self.combat_events.push(counter_event);
                    }
                }
                let mut speed = primary_speed_base;
                if self.combatants[attacker_idx].sheet.maneuvers.offensive_dualwielding {
                    speed += 2.0;
                }
                if self.combatants[defender_idx].state.trauma_remaining_seconds > 0 {
                    speed = (speed / 2.0).ceil().max(1.0);
                }
                self.combatants[attacker_idx]
                    .state
                    .set_next_attack_time(WeaponSlot::Primary, Some(next_attack + speed));
            }

            if self.combatants[attacker_idx]
                .sheet
                .maneuvers
                .offensive_dualwielding
                && self.combatants[attacker_idx].sheet.offense.offhand.is_some()
                && !self.hold_at_bay.pending
                && !self.hold_at_bay.active
                && self.combatants[defender_idx].state.hp > 0
            {
                let offhand_weapon = self.combatants[attacker_idx]
                    .sheet
                    .offense
                    .offhand
                    .as_ref()
                    .map(|offhand| offhand.weapon.clone())
                    .expect("offhand missing");
                let weapon = &offhand_weapon;
                let max_range = max_range_cached(
                    &mut self.combatants[attacker_idx].state,
                    WeaponSlot::Secondary,
                    weapon.as_ref(),
                );
                let has_range = max_range.is_some();
                let attacker_reach = weapon.reach_ft.max(1.0);
                let use_ranged = if has_range && !weapon.uses_projectiles {
                    distance > attacker_reach
                } else {
                    has_range
                };
                if !use_ranged && distance > attacker_reach {
                    continue;
                }
                let ranged_mod = if use_ranged {
                    let range_scale = self.combatants[attacker_idx].apply_f32(
                        StatIdF32::RangeDistanceMultiplier,
                        weapon.range_distance_multiplier,
                    );
                    range_modifier_for_weapon_with_scale(weapon.as_ref(), distance, range_scale)
                } else {
                    None
                };
                if use_ranged && ranged_mod.is_none() {
                    continue;
                }
                if self.combatants[attacker_idx]
                    .state
                    .next_attack_time_secondary
                    .is_none()
                {
                    let primary_anchor = primary_attack_time
                        .or_else(|| self.combatants[attacker_idx].state.next_attack_time_primary)
                        .unwrap_or(now);
                    let offset = 2.0 + (primary_speed_base / 2.0).ceil();
                    self.combatants[attacker_idx]
                        .state
                        .set_next_attack_time(WeaponSlot::Secondary, Some(primary_anchor + offset));
                }
                let next_attack = if use_snapshot_timing {
                    snapshot_next_attack_secondary.unwrap_or_else(|| {
                        self.combatants[attacker_idx]
                            .state
                            .next_attack_time_secondary
                            .unwrap_or(now)
                    })
                } else {
                    self.combatants[attacker_idx]
                        .state
                        .next_attack_time_secondary
                        .unwrap_or(now)
                };
                if now + 0.0001 >= next_attack {
                    let mut event = resolve_attack(
                        &mut self.combatants,
                        attacker_idx,
                        defender_idx,
                        ranged_mod.unwrap_or(0),
                        use_ranged,
                        distance,
                        AttackMode::Normal,
                        WeaponSlot::Secondary,
                        now,
                        state_snapshot.as_ref(),
                        &mut self.rng,
                    );
                    self.apply_knockback(
                        event.attacker_idx,
                        event.defender_idx,
                        event.knockback_ft,
                    );
                    if self.log_events {
                        let event_struct = CombatEvent {
                            time: self.elapsed_seconds,
                            attacker_idx: event.attacker_idx,
                            defender_idx: event.defender_idx,
                            kind: CombatEventKind::Attack(AttackEvent {
                                hit: event.hit,
                                shield_block: event.shield_block,
                                damage: event.damage,
                                shield_damage: event.shield_damage,
                                knockback_ft: event.knockback_ft,
                                hold_at_bay: event.hold_at_bay,
                                use_jab: event.use_jab,
                                is_ranged: event.is_ranged,
                                trauma_applied: event.trauma_applied,
                                trauma_seconds: event.trauma_seconds,
                                roll: event.roll,
                                damage_breakdown: event.damage_breakdown,
                                shield_damage_breakdown: event.shield_damage_breakdown,
                                defender_hp_after: event.defender_hp_after,
                                critical: event.critical,
                            }),
                        };
                        self.last_event = Some(event_struct.clone());
                        self.combat_events.push(event_struct);
                    }
                    if let Some(counter) = event.counter_attack.take() {
                        self.apply_knockback(
                            counter.attacker_idx,
                            counter.defender_idx,
                            counter.knockback_ft,
                        );
                        if self.log_events {
                            let counter_event = CombatEvent {
                                time: self.elapsed_seconds,
                                attacker_idx: counter.attacker_idx,
                                defender_idx: counter.defender_idx,
                                kind: CombatEventKind::Attack(AttackEvent {
                                    hit: counter.hit,
                                    shield_block: counter.shield_block,
                                    damage: counter.damage,
                                    shield_damage: counter.shield_damage,
                                    knockback_ft: counter.knockback_ft,
                                    hold_at_bay: false,
                                    use_jab: counter.use_jab,
                                    is_ranged: counter.is_ranged,
                                    trauma_applied: counter.trauma_applied,
                                    trauma_seconds: counter.trauma_seconds,
                                    roll: counter.roll,
                                    damage_breakdown: counter.damage_breakdown,
                                    shield_damage_breakdown: counter.shield_damage_breakdown,
                                    defender_hp_after: counter.defender_hp_after,
                                    critical: counter.critical,
                                }),
                            };
                            self.last_event = Some(counter_event.clone());
                            self.combat_events.push(counter_event);
                        }
                    }
                    let mut speed = self.combatants[attacker_idx]
                        .apply_f32(StatIdF32::WeaponSpeed, weapon.speed)
                        .max(1.0)
                        + 2.0;
                    if self.combatants[defender_idx].state.trauma_remaining_seconds > 0 {
                        speed = (speed / 2.0).ceil().max(1.0);
                    }
                    self.combatants[attacker_idx]
                        .state
                        .set_next_attack_time(WeaponSlot::Secondary, Some(next_attack + speed));
                }
            }
        }
        if self
            .combatants
            .iter()
            .any(|combatant| combatant.state.hp <= 0)
        {
            self.done = true;
        }
    }
}

fn max_range_cached(
    state: &mut super::types::CombatantState,
    slot: WeaponSlot,
    weapon: &super::types::WeaponProfile,
) -> Option<f32> {
    let cache = state.weapon_cache_mut(slot);
    if let Some(value) = cache.max_range {
        return value;
    }
    let computed = max_range_for_weapon(weapon);
    cache.max_range = Some(computed);
    computed
}

#[derive(Clone, Copy, Debug, Default)]
#[allow(dead_code)]
pub struct BulkSimResult {
    pub wins: [u32; 2],
    pub ties: u32,
    pub avg_duration: f32,
}

#[allow(dead_code)]
pub fn bulk_simulate(
    config: SimConfig,
    combatants: [Combatant; 2],
    runs: u32,
    max_seconds: u32,
) -> BulkSimResult {
    if runs == 0 {
        return BulkSimResult::default();
    }
    let mut sim = SimState::with_logging(config, false);
    sim.reset_with_combatants(combatants);
    let mut wins = [0u32; 2];
    let mut ties = 0u32;
    let mut total_seconds = 0u64;
    for _ in 0..runs {
        sim.reset_preserve_rng();
        while !sim.done && sim.elapsed_seconds < max_seconds {
            sim.update(1.0);
        }
        total_seconds += sim.elapsed_seconds as u64;
        let hp_a = sim.combatants[0].state.hp;
        let hp_b = sim.combatants[1].state.hp;
        if sim.done {
            if hp_a <= 0 && hp_b <= 0 {
                ties += 1;
            } else if hp_a <= 0 {
                wins[1] += 1;
            } else if hp_b <= 0 {
                wins[0] += 1;
            } else {
                ties += 1;
            }
        } else {
            ties += 1;
        }
    }
    let avg_duration = total_seconds as f32 / runs as f32;
    BulkSimResult {
        wins,
        ties,
        avg_duration,
    }
}

impl HoldAtBayState {
    fn blocks_advance(self, idx: usize) -> bool {
        self.active && self.target_idx == idx
    }
}

impl SimState {
    fn clear_hold_at_bay_if_involves(&mut self, attacker_idx: usize, defender_idx: usize) {
        if !self.hold_at_bay.active && !self.hold_at_bay.pending {
            return;
        }
        if self.hold_at_bay.holder_idx == attacker_idx
            || self.hold_at_bay.holder_idx == defender_idx
            || self.hold_at_bay.target_idx == attacker_idx
            || self.hold_at_bay.target_idx == defender_idx
        {
            self.hold_at_bay = HoldAtBayState::default();
        }
    }

    fn maybe_start_hold_at_bay(
        &mut self,
        distance_before: f32,
        distance_after: f32,
        reach_a: f32,
        reach_b: f32,
    ) {
        if self.hold_at_bay.active || self.hold_at_bay.pending {
            return;
        }
        let (holder_idx, target_idx, holder_reach) = if reach_a > reach_b {
            (0usize, 1usize, reach_a)
        } else if reach_b > reach_a {
            (1usize, 0usize, reach_b)
        } else {
            return;
        };
        if !self.combatants[holder_idx].sheet.maneuvers.hold_at_bay {
            return;
        }
        if distance_before > holder_reach && distance_after <= holder_reach {
            self.hold_at_bay = HoldAtBayState {
                pending: true,
                active: false,
                holder_idx,
                target_idx,
            };
        }
    }
}
