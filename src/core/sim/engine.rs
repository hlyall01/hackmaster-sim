use crate::core::rng::SimRng;
use crate::core::rules::roll_damage_expr;
use rand::RngCore;

use super::combat::{AttackMode, resolve_attack, resolve_knock_aside};
use super::modifiers::{StatIdF32, StatIdI32};
use super::movement::{max_range_for_weapon, range_modifier_for_weapon_with_scale};
use super::types::{
    AttackEvent, CalledShotDelayProfile, CombatEvent, CombatEventKind, Combatant, GridPos,
    KnockAsideEvent, SimActor, SimConfig, WeaponSlot,
};
use std::collections::{HashMap, HashSet};

const CHARGE_MIN_DISTANCE_FT: f32 = 20.0;
const SHIELD_STRIKE_SPEEDUP_SECONDS: f32 = 2.0;
const SIX_PATHS_FOLLOWUP_SECONDS: f32 = 1.0;

fn called_shot_delay_seconds(
    attacker: &Combatant,
    defender: &Combatant,
    is_ranged: bool,
    rng: &mut SimRng,
) -> f32 {
    if !attacker.sheet.maneuvers.called_shot {
        return 0.0;
    }
    if defender.sheet.maneuvers.called_shot_deceptive_defender {
        return roll_damage_expr("4d4p", rng, false).max(0) as f32;
    }
    match attacker.sheet.maneuvers.called_shot_delay_profile {
        CalledShotDelayProfile::Standard => {
            let delay_expr = if is_ranged { "1d4p" } else { "2d4p" };
            roll_damage_expr(delay_expr, rng, false).max(0) as f32
        }
        CalledShotDelayProfile::PrecisionCombatant => {
            roll_damage_expr("1d4p", rng, false).max(0) as f32
        }
        CalledShotDelayProfile::PrecisionAiming => {
            roll_damage_expr("1d2", rng, false).max(0) as f32
        }
    }
}

#[derive(Clone, Debug)]
pub struct SimState {
    pub config: SimConfig,
    pub actors: Vec<SimActor>,
    pub combatants: Vec<Combatant>,
    pub elapsed_seconds: u32,
    pub done: bool,
    pub last_event: Option<CombatEvent>,
    pub combat_events: Vec<CombatEvent>,
    pub log_events: bool,
    pub first_attack_time: Option<u32>,
    pub trauma_first_exchange: bool,
    pub charges_started_within_20ft: u32,
    rng: SimRng,
    tick_accum: f32,
    hold_at_bay: HoldAtBayState,
}

#[derive(Clone, Debug, Default)]
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
            actors: Vec::new(),
            combatants: Vec::new(),
            elapsed_seconds: 0,
            done: false,
            last_event: None,
            combat_events: Vec::new(),
            log_events,
            first_attack_time: None,
            trauma_first_exchange: false,
            charges_started_within_20ft: 0,
            rng,
            tick_accum: 0.0,
            hold_at_bay: HoldAtBayState::default(),
        }
    }

    pub fn reset(&mut self) {
        self.actors = self.spawn_positions();
        self.elapsed_seconds = 0;
        self.done = false;
        self.last_event = None;
        self.combat_events.clear();
        self.first_attack_time = None;
        self.trauma_first_exchange = false;
        self.charges_started_within_20ft = 0;
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

    fn spawn_positions(&self) -> Vec<SimActor> {
        let count = self.combatants.len();
        if count == 0 {
            return Vec::new();
        }
        let grid_width = self.config.grid_width.max(1);
        let grid_height = self.config.grid_height.max(1);
        let center_x = grid_width / 2;
        let center_y = grid_height / 2;
        let tile_size_ft = self.config.tile_size_ft.max(0.01);
        let start_tiles = (self.config.start_distance / tile_size_ft).ceil() as i32;

        let mut teams: Vec<u8> = self.combatants.iter().map(|c| c.team_id).collect();
        teams.sort_unstable();
        teams.dedup();
        if teams.is_empty() {
            teams.push(0);
        }

        let mut team_sizes: HashMap<u8, usize> = HashMap::new();
        for combatant in &self.combatants {
            *team_sizes.entry(combatant.team_id).or_insert(0) += 1;
        }

        let mut base_x_by_team: HashMap<u8, i32> = HashMap::new();
        match teams.len() {
            1 => {
                base_x_by_team.insert(teams[0], center_x);
            }
            2 => {
                let padding = ((grid_width - 1 - start_tiles) / 2).max(0);
                let left_x = padding;
                let right_x = (padding + start_tiles).min(grid_width - 1);
                base_x_by_team.insert(teams[0], left_x);
                base_x_by_team.insert(teams[1], right_x);
            }
            _ => {
                let span = (grid_width - 1).max(1);
                let spacing = span / (teams.len() as i32 - 1);
                for (idx, team_id) in teams.iter().enumerate() {
                    let x = (idx as i32 * spacing).clamp(0, grid_width - 1);
                    base_x_by_team.insert(*team_id, x);
                }
            }
        }

        let mut team_offsets: HashMap<u8, usize> = HashMap::new();
        let mut actors = Vec::with_capacity(count);
        for combatant in &self.combatants {
            let team_id = combatant.team_id;
            let team_size = *team_sizes.get(&team_id).unwrap_or(&1);
            let slot = team_offsets.entry(team_id).or_insert(0);
            let offset = *slot as i32 - (team_size as i32 - 1) / 2;
            *slot += 1;
            let base_x = *base_x_by_team.get(&team_id).unwrap_or(&center_x);
            let pos = GridPos::new(base_x, center_y + offset).clamp(grid_width, grid_height);
            actors.push(SimActor { position: pos });
        }
        actors
    }

    pub fn reset_with_combatants(&mut self, combatants: Vec<Combatant>) {
        self.combatants = combatants;
        self.reset();
    }

    #[allow(dead_code)]
    pub fn reset_with_combatants_preserve_rng(&mut self, combatants: Vec<Combatant>) {
        self.combatants = combatants;
        self.reset_preserve_rng();
    }

    pub fn set_rng(&mut self, rng: SimRng) {
        self.rng = rng;
    }

    pub(crate) fn apply_shield_strike_speedup(&mut self, defender_idx: usize, now: f32) {
        let has_style =
            self.combatants[defender_idx].apply_i32(StatIdI32::FlagLargeSwordShieldStyle, 0) > 0;
        if !has_style {
            return;
        }
        let Some(next_attack) = self.combatants[defender_idx].state.next_attack_time_primary else {
            return;
        };
        let adjusted = (next_attack - SHIELD_STRIKE_SPEEDUP_SECONDS).max(now);
        self.combatants[defender_idx]
            .state
            .set_next_attack_time(WeaponSlot::Primary, Some(adjusted));
    }

    pub(crate) fn apply_six_paths_followup(&mut self, attacker_idx: usize, now: f32) {
        let has_style =
            self.combatants[attacker_idx].apply_i32(StatIdI32::FlagSixPathsStyle, 0) > 0;
        if !has_style {
            return;
        }
        let Some(next_attack) = self.combatants[attacker_idx].state.next_attack_time_primary else {
            return;
        };
        let followup_time = now + SIX_PATHS_FOLLOWUP_SECONDS;
        if followup_time < next_attack {
            self.combatants[attacker_idx]
                .state
                .set_next_attack_time(WeaponSlot::Primary, Some(followup_time));
        }
    }

    fn record_attack_metrics(
        &mut self,
        attacker_idx: usize,
        defender_idx: usize,
        hp_damage: i32,
        shield_damage: i32,
        knockback_ft: f32,
    ) {
        let hp_damage_u32 = hp_damage.max(0) as u32;
        let shield_damage_u32 = shield_damage.max(0) as u32;
        let knockback = knockback_ft.max(0.0);

        {
            let attacker = &mut self.combatants[attacker_idx].state;
            attacker.max_hit_dealt = attacker.max_hit_dealt.max(hp_damage.max(0));
            attacker.max_shield_hit_dealt = attacker.max_shield_hit_dealt.max(shield_damage.max(0));
            attacker.total_hp_damage_dealt =
                attacker.total_hp_damage_dealt.saturating_add(hp_damage_u32);
            attacker.total_shield_damage_dealt = attacker
                .total_shield_damage_dealt
                .saturating_add(shield_damage_u32);
            attacker.total_knockback_inflicted_ft += knockback;
        }
        {
            let defender = &mut self.combatants[defender_idx].state;
            defender.total_hp_damage_taken =
                defender.total_hp_damage_taken.saturating_add(hp_damage_u32);
            defender.total_shield_damage_taken = defender
                .total_shield_damage_taken
                .saturating_add(shield_damage_u32);
            defender.total_knockback_taken_ft += knockback;
        }
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
        if self.actors.len() != self.combatants.len() {
            self.actors = self.spawn_positions();
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
        let old_positions: Vec<GridPos> = self.actors.iter().map(|actor| actor.position).collect();
        let active_pair = self.active_pair();
        if let Some((a_idx, b_idx)) = active_pair {
            if (self.hold_at_bay.active || self.hold_at_bay.pending)
                && self.hold_at_bay.holder_idx != a_idx
                && self.hold_at_bay.holder_idx != b_idx
                && self.hold_at_bay.target_idx != a_idx
                && self.hold_at_bay.target_idx != b_idx
            {
                self.hold_at_bay = HoldAtBayState::default();
            }

            let distance_before_combat = self.distance_between(a_idx, b_idx).unwrap_or(0.0);
            let reach_a = self.combatants[a_idx]
                .apply_f32(
                    StatIdF32::WeaponReach,
                    self.combatants[a_idx].sheet.offense.weapon.reach_ft,
                )
                .max(1.0);
            let reach_b = self.combatants[b_idx]
                .apply_f32(
                    StatIdF32::WeaponReach,
                    self.combatants[b_idx].sheet.offense.weapon.reach_ft,
                )
                .max(1.0);
            let max_reach = self.config.stop_distance.max(1.0);
            let min_reach = reach_a.min(reach_b);
            let weapon_a = self.combatants[a_idx].sheet.offense.weapon.clone();
            let weapon_b = self.combatants[b_idx].sheet.offense.weapon.clone();
            let ranged_projectile_a = weapon_a.uses_projectiles;
            let ranged_projectile_b = weapon_b.uses_projectiles;
            let max_range_a = max_range_cached(
                &mut self.combatants[a_idx].state,
                WeaponSlot::Primary,
                weapon_a.as_ref(),
            );
            let max_range_b = max_range_cached(
                &mut self.combatants[b_idx].state,
                WeaponSlot::Primary,
                weapon_b.as_ref(),
            );
            let ranged_a = max_range_a.is_some();
            let ranged_b = max_range_b.is_some();
            let ranged_projectile_a = ranged_a && ranged_projectile_a;
            let ranged_projectile_b = ranged_b && ranged_projectile_b;
            let any_ranged = ranged_a || ranged_b;

            if distance_before_combat > max_reach && !any_ranged {
                let step_a = self.move_tiles(a_idx);
                let step_b = self.move_tiles(b_idx);
                let tile_size_ft = self.config.tile_size_ft.max(0.01);
                let closure_ft = (distance_before_combat - max_reach).max(0.0);
                let closure_tiles = (closure_ft / tile_size_ft).ceil() as i32;
                let total_steps = step_a + step_b;
                if total_steps > 0 && closure_tiles > 0 {
                    let alloc_a = ((step_a as f32 / total_steps as f32) * closure_tiles as f32)
                        .round() as i32;
                    let mut steps_a = alloc_a.clamp(0, step_a).min(closure_tiles);
                    let mut steps_b = (closure_tiles - steps_a).clamp(0, step_b);
                    if steps_a + steps_b < closure_tiles {
                        let remaining = closure_tiles - steps_a - steps_b;
                        let add_a = remaining.min(step_a - steps_a);
                        steps_a += add_a;
                        let add_b = remaining - add_a;
                        steps_b += add_b.min(step_b - steps_b);
                    }
                    self.move_toward(a_idx, b_idx, steps_a, max_reach);
                    self.move_toward(b_idx, a_idx, steps_b, max_reach);
                }
                for combatant in &mut self.combatants {
                    combatant.state.clear_attack_timers();
                }
            } else {
                self.resolve_combat_round(a_idx, b_idx);
                let distance = self.distance_between(a_idx, b_idx).unwrap_or(0.0);
                let step_a = self.move_tiles(a_idx);
                let step_b = self.move_tiles(b_idx);
                if any_ranged {
                    let engaged = distance <= min_reach;
                    if !engaged {
                        if ranged_projectile_a {
                            if let Some(max_range) = max_range_a {
                                if distance <= max_range {
                                    self.move_away(a_idx, b_idx, step_a);
                                } else {
                                    self.move_toward(a_idx, b_idx, step_a, max_reach);
                                }
                            }
                        } else if distance > reach_a {
                            self.move_toward(a_idx, b_idx, step_a, max_reach);
                        }
                        if ranged_projectile_b {
                            if let Some(max_range) = max_range_b {
                                if distance <= max_range {
                                    self.move_away(b_idx, a_idx, step_b);
                                } else {
                                    self.move_toward(b_idx, a_idx, step_b, max_reach);
                                }
                            }
                        } else if distance > reach_b {
                            self.move_toward(b_idx, a_idx, step_b, max_reach);
                        }
                    }
                } else if distance > min_reach {
                    if reach_a < reach_b {
                        if !self.hold_at_bay.blocks_advance(a_idx) {
                            self.move_toward(a_idx, b_idx, step_a, reach_a);
                        }
                    } else if reach_b < reach_a {
                        if !self.hold_at_bay.blocks_advance(b_idx) {
                            self.move_toward(b_idx, a_idx, step_b, reach_b);
                        }
                    }
                }
            }
            let distance_after_combat = self.distance_between(a_idx, b_idx).unwrap_or(0.0);
            if max_range_a.is_some()
                && !weapon_a.uses_projectiles
                && distance_before_combat > reach_a
                && distance_after_combat <= reach_a
            {
                self.combatants[a_idx].state.clear_attack_timers();
            }
            if max_range_b.is_some()
                && !weapon_b.uses_projectiles
                && distance_before_combat > reach_b
                && distance_after_combat <= reach_b
            {
                self.combatants[b_idx].state.clear_attack_timers();
            }
            self.maybe_start_hold_at_bay(
                a_idx,
                b_idx,
                distance_before_combat,
                distance_after_combat,
                reach_a,
                reach_b,
            );
            if (self.combatants[a_idx].state.knockback_applied_this_tick
                || self.combatants[b_idx].state.knockback_applied_this_tick)
                && distance_after_combat < distance_before_combat
            {
                self.enforce_min_distance(a_idx, b_idx, distance_before_combat);
            }
        }
        self.update_charge_progress(active_pair, &old_positions);
        for (idx, combatant) in self.combatants.iter_mut().enumerate() {
            let moved = self
                .actors
                .get(idx)
                .zip(old_positions.get(idx))
                .map(|(actor, old)| actor.position.x != old.x || actor.position.y != old.y)
                .unwrap_or(false);
            combatant.state.moved_last_tick = moved;
        }
        self.elapsed_seconds += 1;
        let now = self.elapsed_seconds as f32;
        for combatant in &mut self.combatants {
            combatant
                .state
                .refresh_defense_plus_four_ready(&combatant.sheet, now);
        }
        self.done = self.remaining_team_count() <= 1;
    }

    pub fn distance(&self) -> f32 {
        self.distance_between(0, 1).unwrap_or(0.0)
    }

    pub fn distance_between(&self, a_idx: usize, b_idx: usize) -> Option<f32> {
        let tiles = self.grid_distance_tiles(a_idx, b_idx)?;
        Some(tiles as f32 * self.config.tile_size_ft)
    }

    fn grid_distance_tiles(&self, a_idx: usize, b_idx: usize) -> Option<i32> {
        let pos_a = self.actors.get(a_idx)?.position;
        let pos_b = self.actors.get(b_idx)?.position;
        Some(pos_a.manhattan_distance(pos_b))
    }

    fn move_tiles(&self, idx: usize) -> i32 {
        let combatant = &self.combatants[idx];
        if combatant.state.trauma_remaining_seconds > 0
            || combatant.state.knockback_immobile_seconds > 0
        {
            return 0;
        }
        let speed_ft = combatant
            .apply_f32(StatIdF32::MoveSpeed, combatant.sheet.mobility.move_speed)
            .max(0.0);
        if speed_ft <= 0.0 {
            return 0;
        }
        let tile_size_ft = self.config.tile_size_ft.max(0.01);
        let tiles = (speed_ft / tile_size_ft).round();
        if tiles <= 0.0 { 1 } else { tiles as i32 }
    }

    fn move_toward(
        &mut self,
        mover_idx: usize,
        target_idx: usize,
        steps: i32,
        stop_distance_ft: f32,
    ) {
        if steps <= 0 {
            return;
        }
        for _ in 0..steps {
            let distance = self.distance_between(mover_idx, target_idx).unwrap_or(0.0);
            if distance <= stop_distance_ft {
                break;
            }
            let from = self.actors[mover_idx].position;
            let to = self.actors[target_idx].position;
            let next =
                Self::step_toward(from, to).clamp(self.config.grid_width, self.config.grid_height);
            if next.x == from.x && next.y == from.y {
                break;
            }
            self.actors[mover_idx].position = next;
        }
    }

    fn move_away(&mut self, mover_idx: usize, target_idx: usize, steps: i32) {
        if steps <= 0 {
            return;
        }
        for _ in 0..steps {
            let from = self.actors[mover_idx].position;
            let away_from = self.actors[target_idx].position;
            let next = Self::step_away(from, away_from)
                .clamp(self.config.grid_width, self.config.grid_height);
            if next.x == from.x && next.y == from.y {
                break;
            }
            self.actors[mover_idx].position = next;
        }
    }

    fn update_charge_progress(
        &mut self,
        active_pair: Option<(usize, usize)>,
        old_positions: &[GridPos],
    ) {
        let tile_size_ft = self.config.tile_size_ft.max(0.01);
        let (pair_a, pair_b) = active_pair.unwrap_or((usize::MAX, usize::MAX));
        for idx in 0..self.combatants.len() {
            let target_idx = if idx == pair_a {
                Some(pair_b)
            } else if idx == pair_b {
                Some(pair_a)
            } else {
                None
            };
            let (charge_enabled, reach, current_target) = {
                let combatant = &self.combatants[idx];
                (
                    combatant.sheet.maneuvers.charge,
                    combatant
                        .apply_f32(
                            StatIdF32::WeaponReach,
                            combatant.sheet.offense.weapon.reach_ft,
                        )
                        .max(1.0),
                    combatant.state.charge_target_idx,
                )
            };
            if !charge_enabled || target_idx.is_none() {
                let state = &mut self.combatants[idx].state;
                state.charge_distance_ft = 0.0;
                state.charge_target_idx = None;
                continue;
            }
            let target_idx = target_idx.expect("target index missing");
            if current_target != Some(target_idx) {
                let state = &mut self.combatants[idx].state;
                state.charge_distance_ft = 0.0;
                state.charge_target_idx = Some(target_idx);
            }
            let (Some(old_pos), Some(old_target_pos), Some(new_pos)) = (
                old_positions.get(idx).copied(),
                old_positions.get(target_idx).copied(),
                self.actors.get(idx).map(|actor| actor.position),
            ) else {
                continue;
            };
            let distance_before = old_pos.manhattan_distance(old_target_pos) as f32 * tile_size_ft;
            let distance_after = new_pos.manhattan_distance(old_target_pos) as f32 * tile_size_ft;
            let moved_tiles = old_pos.manhattan_distance(new_pos);
            let state = &mut self.combatants[idx].state;
            if moved_tiles == 0 {
                if distance_before > reach {
                    state.charge_distance_ft = 0.0;
                }
                continue;
            }
            if distance_after >= distance_before {
                state.charge_distance_ft = 0.0;
                continue;
            }
            if distance_before > reach {
                let moved_ft = moved_tiles as f32 * tile_size_ft;
                let max_closure_ft = (distance_before - reach).max(0.0);
                let delta = moved_ft.min(max_closure_ft);
                let before = state.charge_distance_ft;
                state.charge_distance_ft += delta;
                if before < 20.0 && state.charge_distance_ft >= 20.0 && distance_before < 20.0 {
                    state.charge_threshold_started_within_20ft = true;
                }
            }
        }
    }

    fn apply_knockback(&mut self, attacker_idx: usize, defender_idx: usize, knockback_ft: f32) {
        if knockback_ft <= 0.0 {
            return;
        }
        if let Some(defender) = self.combatants.get_mut(defender_idx) {
            defender.state.knockback_applied_this_tick = true;
        }
        let tile_size_ft = self.config.tile_size_ft.max(0.01);
        let tiles = (knockback_ft / tile_size_ft).ceil() as i32;
        self.move_away(defender_idx, attacker_idx, tiles);
    }

    fn step_toward(from: GridPos, to: GridPos) -> GridPos {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        if dx.abs() >= dy.abs() && dx != 0 {
            GridPos::new(from.x + dx.signum(), from.y)
        } else if dy != 0 {
            GridPos::new(from.x, from.y + dy.signum())
        } else {
            from
        }
    }

    fn step_away(from: GridPos, away_from: GridPos) -> GridPos {
        let dx = from.x - away_from.x;
        let dy = from.y - away_from.y;
        if dx.abs() >= dy.abs() && dx != 0 {
            GridPos::new(from.x + dx.signum(), from.y)
        } else if dy != 0 {
            GridPos::new(from.x, from.y + dy.signum())
        } else {
            from
        }
    }

    fn enforce_min_distance(&mut self, a_idx: usize, b_idx: usize, distance_ft: f32) {
        let tile_size_ft = self.config.tile_size_ft.max(0.01);
        let min_tiles = (distance_ft / tile_size_ft).round() as i32;
        let current_tiles = self.grid_distance_tiles(a_idx, b_idx).unwrap_or(0);
        if current_tiles >= min_tiles {
            return;
        }
        let mover_idx = if self.combatants[a_idx].state.knockback_applied_this_tick {
            a_idx
        } else if self.combatants[b_idx].state.knockback_applied_this_tick {
            b_idx
        } else {
            b_idx
        };
        let other_idx = if mover_idx == a_idx { b_idx } else { a_idx };
        let mut remaining = min_tiles - current_tiles;
        while remaining > 0 {
            let from = self.actors[mover_idx].position;
            let next = Self::step_away(from, self.actors[other_idx].position)
                .clamp(self.config.grid_width, self.config.grid_height);
            if next.x == from.x && next.y == from.y {
                break;
            }
            self.actors[mover_idx].position = next;
            remaining -= 1;
        }
    }

    fn remaining_team_count(&self) -> usize {
        let mut teams = HashSet::new();
        for combatant in &self.combatants {
            if combatant.state.hp > 0 {
                teams.insert(combatant.team_id);
            }
        }
        teams.len()
    }

    fn active_pair(&self) -> Option<(usize, usize)> {
        let mut best: Option<(usize, usize, i32)> = None;
        for i in 0..self.combatants.len() {
            if self.combatants[i].state.hp <= 0 {
                continue;
            }
            for j in (i + 1)..self.combatants.len() {
                if self.combatants[j].state.hp <= 0 {
                    continue;
                }
                if self.combatants[i].team_id == self.combatants[j].team_id {
                    continue;
                }
                let distance = self.grid_distance_tiles(i, j).unwrap_or(i32::MAX);
                let replace = match best {
                    None => true,
                    Some((_, _, best_distance)) => distance < best_distance,
                };
                if replace {
                    best = Some((i, j, distance));
                }
            }
        }
        best.map(|(i, j, _)| (i, j))
    }

    fn resolve_combat_round(&mut self, a_idx: usize, b_idx: usize) {
        let now = self.elapsed_seconds as f32;
        let distance = self.distance_between(a_idx, b_idx).unwrap_or(0.0);
        let reach_a = self.combatants[a_idx]
            .apply_f32(
                StatIdF32::WeaponReach,
                self.combatants[a_idx].sheet.offense.weapon.reach_ft,
            )
            .max(1.0);
        let reach_b = self.combatants[b_idx]
            .apply_f32(
                StatIdF32::WeaponReach,
                self.combatants[b_idx].sheet.offense.weapon.reach_ft,
            )
            .max(1.0);
        let simultaneous = (reach_a - reach_b).abs() < f32::EPSILON;
        let state_snapshot = if simultaneous {
            Some(
                self.combatants
                    .iter()
                    .map(|combatant| combatant.state.clone())
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        };
        let alive_start_a = self.combatants[a_idx].state.hp > 0;
        let alive_start_b = self.combatants[b_idx].state.hp > 0;
        let alive_start = |idx: usize| {
            if idx == a_idx {
                alive_start_a
            } else if idx == b_idx {
                alive_start_b
            } else {
                false
            }
        };
        let mut order = [(a_idx, b_idx), (b_idx, a_idx)];
        if self.rng.next_u32() & 1 == 1 {
            order.swap(0, 1);
        }
        for (attacker_idx, defender_idx) in order {
            let snapshot_next_attack_primary = state_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.get(attacker_idx))
                .and_then(|state| state.next_attack_time_primary);
            let snapshot_next_attack_secondary = state_snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.get(attacker_idx))
                .and_then(|state| state.next_attack_time_secondary);
            let use_snapshot_timing = state_snapshot.is_some();
            let attacker_alive = if simultaneous {
                alive_start(attacker_idx)
            } else {
                self.combatants[attacker_idx].state.hp > 0
            };
            let defender_alive = if simultaneous {
                alive_start(defender_idx)
            } else {
                self.combatants[defender_idx].state.hp > 0
            };
            let attacker_trauma = if let Some(snapshot) = state_snapshot.as_ref() {
                snapshot
                    .get(attacker_idx)
                    .map(|state| state.trauma_remaining_seconds > 0)
                    .unwrap_or(false)
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
                    snapshot_next_attack_primary.unwrap_or_else(|| {
                        self.combatants[attacker_idx]
                            .state
                            .next_attack_time_primary
                            .unwrap_or(now)
                    })
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
                        state_snapshot.as_deref(),
                        &mut self.rng,
                    );
                    if event.success {
                        self.hold_at_bay = HoldAtBayState::default();
                        self.combatants[attacker_idx]
                            .state
                            .set_next_attack_time(WeaponSlot::Primary, Some(now + 1.0));
                    } else {
                        self.combatants[attacker_idx].state.set_next_attack_time(
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
            let weapon = self.combatants[attacker_idx].sheet.offense.weapon.clone();
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
                let defender_reach = self.combatants[defender_idx]
                    .sheet
                    .offense
                    .weapon
                    .reach_ft
                    .max(1.0);
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
            if attack_mode == AttackMode::Normal
                && self.combatants[attacker_idx].sheet.maneuvers.charge
                && !use_ranged
                && self.combatants[attacker_idx].state.charge_target_idx == Some(defender_idx)
                && self.combatants[attacker_idx].state.charge_distance_ft >= CHARGE_MIN_DISTANCE_FT
            {
                attack_mode = AttackMode::Charge;
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
                let called_shot_delay = called_shot_delay_seconds(
                    &self.combatants[attacker_idx],
                    &self.combatants[defender_idx],
                    use_ranged,
                    &mut self.rng,
                );
                self.combatants[attacker_idx].state.set_next_attack_time(
                    WeaponSlot::Primary,
                    Some(now + delay + called_shot_delay),
                );
            }
            let next_attack = if use_snapshot_timing {
                snapshot_next_attack_primary.unwrap_or_else(|| {
                    self.combatants[attacker_idx]
                        .state
                        .next_attack_time_primary
                        .unwrap_or(now)
                })
            } else {
                self.combatants[attacker_idx]
                    .state
                    .next_attack_time_primary
                    .unwrap_or(now)
            };
            if now + 0.0001 >= next_attack {
                primary_attack_time = Some(next_attack);
                if attack_mode == AttackMode::Charge {
                    self.combatants[attacker_idx].state.charge_attacks = self.combatants
                        [attacker_idx]
                        .state
                        .charge_attacks
                        .saturating_add(1);
                    if self.combatants[attacker_idx]
                        .state
                        .charge_threshold_started_within_20ft
                    {
                        self.combatants[attacker_idx]
                            .state
                            .charge_started_within_20ft = true;
                        self.charges_started_within_20ft =
                            self.charges_started_within_20ft.saturating_add(1);
                    }
                }
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
                    state_snapshot.as_deref(),
                    &mut self.rng,
                );
                let six_paths_followup = event.hit && !event.is_ranged;
                if attack_mode == AttackMode::HoldAtBay && self.hold_at_bay.pending {
                    if event.hit {
                        self.hold_at_bay.active = true;
                        self.hold_at_bay.pending = false;
                    } else {
                        self.hold_at_bay = HoldAtBayState::default();
                    }
                }
                if event.shield_block {
                    self.apply_shield_strike_speedup(event.defender_idx, now);
                }
                self.record_attack_metrics(
                    event.attacker_idx,
                    event.defender_idx,
                    event.damage,
                    event.shield_damage,
                    event.knockback_ft,
                );
                self.apply_knockback(event.attacker_idx, event.defender_idx, event.knockback_ft);
                if self.first_attack_time.is_none() {
                    self.first_attack_time = Some(self.elapsed_seconds);
                }
                if event.trauma_applied {
                    self.combatants[event.defender_idx].state.saw_trauma = true;
                    if self.first_attack_time == Some(self.elapsed_seconds) {
                        self.trauma_first_exchange = true;
                    }
                }
                if event.knockback_ft > 0.0 {
                    let state = &mut self.combatants[event.defender_idx].state;
                    state.max_knockback_ft = state.max_knockback_ft.max(event.knockback_ft);
                }
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
                            is_charge: attack_mode == AttackMode::Charge,
                            weapon_slot: event.weapon_slot,
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
                    if counter.shield_block {
                        self.apply_shield_strike_speedup(counter.defender_idx, now);
                    }
                    if counter.hit && !counter.is_ranged {
                        self.apply_six_paths_followup(counter.attacker_idx, now);
                    }
                    self.record_attack_metrics(
                        counter.attacker_idx,
                        counter.defender_idx,
                        counter.damage,
                        counter.shield_damage,
                        counter.knockback_ft,
                    );
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
                                is_charge: false,
                                weapon_slot: counter.weapon_slot,
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
                        if self.first_attack_time.is_none() {
                            self.first_attack_time = Some(self.elapsed_seconds);
                        }
                        if counter.trauma_applied {
                            self.combatants[counter.defender_idx].state.saw_trauma = true;
                            if self.first_attack_time == Some(self.elapsed_seconds) {
                                self.trauma_first_exchange = true;
                            }
                        }
                        if counter.knockback_ft > 0.0 {
                            let state = &mut self.combatants[counter.defender_idx].state;
                            state.max_knockback_ft =
                                state.max_knockback_ft.max(counter.knockback_ft);
                        }
                        self.last_event = Some(counter_event.clone());
                        self.combat_events.push(counter_event);
                    }
                }
                let mut speed = primary_speed_base;
                if self.combatants[attacker_idx]
                    .sheet
                    .maneuvers
                    .offensive_dualwielding
                {
                    speed += 2.0;
                }
                if self.combatants[defender_idx].state.trauma_remaining_seconds > 0 {
                    speed = (speed / 2.0).ceil().max(1.0);
                }
                speed += called_shot_delay_seconds(
                    &self.combatants[attacker_idx],
                    &self.combatants[event.defender_idx],
                    event.is_ranged,
                    &mut self.rng,
                );
                self.combatants[attacker_idx]
                    .state
                    .set_next_attack_time(WeaponSlot::Primary, Some(next_attack + speed));
                if six_paths_followup {
                    self.apply_six_paths_followup(attacker_idx, now);
                }
            }

            if self.combatants[attacker_idx]
                .sheet
                .maneuvers
                .offensive_dualwielding
                && self.combatants[attacker_idx]
                    .sheet
                    .offense
                    .offhand
                    .is_some()
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
                    let called_shot_delay = called_shot_delay_seconds(
                        &self.combatants[attacker_idx],
                        &self.combatants[defender_idx],
                        use_ranged,
                        &mut self.rng,
                    );
                    self.combatants[attacker_idx].state.set_next_attack_time(
                        WeaponSlot::Secondary,
                        Some(primary_anchor + offset + called_shot_delay),
                    );
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
                        state_snapshot.as_deref(),
                        &mut self.rng,
                    );
                    let six_paths_followup = event.hit && !event.is_ranged;
                    if event.shield_block {
                        self.apply_shield_strike_speedup(event.defender_idx, now);
                    }
                    self.record_attack_metrics(
                        event.attacker_idx,
                        event.defender_idx,
                        event.damage,
                        event.shield_damage,
                        event.knockback_ft,
                    );
                    self.apply_knockback(
                        event.attacker_idx,
                        event.defender_idx,
                        event.knockback_ft,
                    );
                    if self.first_attack_time.is_none() {
                        self.first_attack_time = Some(self.elapsed_seconds);
                    }
                    if event.trauma_applied {
                        self.combatants[event.defender_idx].state.saw_trauma = true;
                        if self.first_attack_time == Some(self.elapsed_seconds) {
                            self.trauma_first_exchange = true;
                        }
                    }
                    if event.knockback_ft > 0.0 {
                        let state = &mut self.combatants[event.defender_idx].state;
                        state.max_knockback_ft = state.max_knockback_ft.max(event.knockback_ft);
                    }
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
                                is_charge: false,
                                weapon_slot: event.weapon_slot,
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
                        if counter.shield_block {
                            self.apply_shield_strike_speedup(counter.defender_idx, now);
                        }
                        if counter.hit && !counter.is_ranged {
                            self.apply_six_paths_followup(counter.attacker_idx, now);
                        }
                        self.record_attack_metrics(
                            counter.attacker_idx,
                            counter.defender_idx,
                            counter.damage,
                            counter.shield_damage,
                            counter.knockback_ft,
                        );
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
                                    is_charge: false,
                                    weapon_slot: counter.weapon_slot,
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
                            if self.first_attack_time.is_none() {
                                self.first_attack_time = Some(self.elapsed_seconds);
                            }
                            if counter.trauma_applied {
                                self.combatants[counter.defender_idx].state.saw_trauma = true;
                                if self.first_attack_time == Some(self.elapsed_seconds) {
                                    self.trauma_first_exchange = true;
                                }
                            }
                            if counter.knockback_ft > 0.0 {
                                let state = &mut self.combatants[counter.defender_idx].state;
                                state.max_knockback_ft =
                                    state.max_knockback_ft.max(counter.knockback_ft);
                            }
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
                    speed += called_shot_delay_seconds(
                        &self.combatants[attacker_idx],
                        &self.combatants[event.defender_idx],
                        event.is_ranged,
                        &mut self.rng,
                    );
                    self.combatants[attacker_idx]
                        .state
                        .set_next_attack_time(WeaponSlot::Secondary, Some(next_attack + speed));
                    if six_paths_followup {
                        self.apply_six_paths_followup(attacker_idx, now);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn called_shot_delay_requires_called_shot_toggle() {
        let attacker = Combatant::default();
        let defender = Combatant::default();
        let mut rng = SimRng::from_seed(1);
        let delay = called_shot_delay_seconds(&attacker, &defender, false, &mut rng);
        assert_eq!(delay, 0.0);
    }

    #[test]
    fn called_shot_delay_standard_melee_is_at_least_two_seconds() {
        let mut attacker = Combatant::default();
        attacker.sheet.maneuvers.called_shot = true;
        let defender = Combatant::default();
        let mut rng = SimRng::from_seed(2);
        let delay = called_shot_delay_seconds(&attacker, &defender, false, &mut rng);
        assert!(delay >= 2.0, "expected 2d4p delay >= 2, got {delay}");
    }

    #[test]
    fn called_shot_delay_precision_combatant_is_at_least_one_second() {
        let mut attacker = Combatant::default();
        attacker.sheet.maneuvers.called_shot = true;
        attacker.sheet.maneuvers.called_shot_delay_profile =
            CalledShotDelayProfile::PrecisionCombatant;
        let defender = Combatant::default();
        let mut rng = SimRng::from_seed(3);
        let delay = called_shot_delay_seconds(&attacker, &defender, false, &mut rng);
        assert!(delay >= 1.0, "expected 1d4p delay >= 1, got {delay}");
    }

    #[test]
    fn called_shot_delay_precision_aiming_is_between_one_and_two_seconds() {
        let mut attacker = Combatant::default();
        attacker.sheet.maneuvers.called_shot = true;
        attacker.sheet.maneuvers.called_shot_delay_profile =
            CalledShotDelayProfile::PrecisionAiming;
        let defender = Combatant::default();
        let mut rng = SimRng::from_seed(4);
        let delay = called_shot_delay_seconds(&attacker, &defender, false, &mut rng);
        assert!(
            (1.0..=2.0).contains(&delay),
            "expected precision aiming delay in [1, 2], got {delay}"
        );
    }

    #[test]
    fn called_shot_delay_deceptive_defender_forces_four_d4p() {
        let mut attacker = Combatant::default();
        attacker.sheet.maneuvers.called_shot = true;
        attacker.sheet.maneuvers.called_shot_delay_profile =
            CalledShotDelayProfile::PrecisionAiming;
        let mut defender = Combatant::default();
        defender.sheet.maneuvers.called_shot_deceptive_defender = true;
        let mut rng = SimRng::from_seed(5);
        let delay = called_shot_delay_seconds(&attacker, &defender, false, &mut rng);
        assert!(
            delay >= 4.0,
            "expected deceptive defender delay >= 4 (4d4p), got {delay}"
        );
    }

    #[test]
    fn called_shot_delay_applies_before_opening_melee_attack() {
        let mut called_shot_attacker = Combatant::default();
        called_shot_attacker.team_id = 0;
        called_shot_attacker.sheet.maneuvers.called_shot = true;
        called_shot_attacker.sheet.vitals.max_hp = 100;
        called_shot_attacker.sheet.defense.defense_mod = 100;
        called_shot_attacker.reset_state();

        let mut defender = Combatant::default();
        defender.team_id = 1;
        defender.sheet.vitals.max_hp = 100;
        defender.reset_state();

        let mut sim = SimState::with_rng(SimConfig::new(1.0, 1.0), SimRng::from_seed(9));
        sim.reset_with_combatants(vec![called_shot_attacker, defender]);
        sim.tick();

        assert!(
            !sim.combat_events
                .iter()
                .any(|event| event.time == 0 && event.attacker_idx == 0),
            "called-shot attacker should not attack on opening tick before delay elapses"
        );
        let next_attack = sim.combatants[0]
            .state
            .next_attack_time_primary
            .unwrap_or(0.0);
        assert!(
            next_attack >= 2.0,
            "expected opening called-shot melee delay >= 2s (2d4p), got {next_attack}"
        );
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

#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub struct BulkSimResult {
    pub wins: Vec<u32>,
    pub ties: u32,
    pub avg_duration: f32,
    pub shortest_duration: u32,
    pub longest_duration: u32,
    pub fights_with_second_charge: u32,
    pub fights_with_trauma: u32,
    pub fights_with_trauma_first_exchange: u32,
    pub fights_with_knockback_20ft: u32,
    pub fights_with_charge_within_20ft: u32,
    pub highest_single_hit: i32,
    pub highest_single_shield_hit: i32,
    pub highest_single_hit_by_team: Vec<i32>,
    pub highest_single_shield_hit_by_team: Vec<i32>,
    pub avg_damage_dealt_by_team: Vec<f32>,
    pub avg_damage_taken_by_team: Vec<f32>,
    pub avg_remaining_hp_by_team: Vec<f32>,
    pub max_total_knockback_one_side_ft: f32,
    pub avg_max_knockback_one_side_ft: f32,
}

#[allow(dead_code)]
pub fn bulk_simulate(
    config: SimConfig,
    combatants: Vec<Combatant>,
    runs: u32,
    max_seconds: u32,
) -> BulkSimResult {
    if runs == 0 {
        return BulkSimResult::default();
    }
    let mut team_ids: Vec<u8> = combatants
        .iter()
        .map(|combatant| combatant.team_id)
        .collect();
    team_ids.sort_unstable();
    team_ids.dedup();
    let mut team_index = HashMap::new();
    for (idx, team_id) in team_ids.iter().enumerate() {
        team_index.insert(*team_id, idx);
    }
    let mut sim = SimState::with_logging(config, false);
    sim.reset_with_combatants(combatants);
    let mut wins = vec![0u32; team_ids.len()];
    let mut ties = 0u32;
    let mut fights_with_second_charge = 0u32;
    let mut fights_with_trauma = 0u32;
    let mut fights_with_trauma_first_exchange = 0u32;
    let mut fights_with_knockback_20ft = 0u32;
    let mut fights_with_charge_within_20ft = 0u32;
    let mut shortest_duration = u32::MAX;
    let mut longest_duration = 0u32;
    let mut highest_single_hit = 0i32;
    let mut highest_single_shield_hit = 0i32;
    let mut highest_single_hit_by_team = vec![0i32; team_ids.len()];
    let mut highest_single_shield_hit_by_team = vec![0i32; team_ids.len()];
    let mut total_damage_dealt_by_team = vec![0u64; team_ids.len()];
    let mut total_damage_taken_by_team = vec![0u64; team_ids.len()];
    let mut total_remaining_hp_by_team = vec![0u64; team_ids.len()];
    let mut max_total_knockback_one_side_ft = 0.0f32;
    let mut total_max_knockback_one_side_ft = 0.0f32;
    let mut total_seconds = 0u64;
    for _ in 0..runs {
        sim.reset_preserve_rng();
        while !sim.done && sim.elapsed_seconds < max_seconds {
            sim.update(1.0);
        }
        let duration = sim.elapsed_seconds;
        total_seconds += duration as u64;
        shortest_duration = shortest_duration.min(duration);
        longest_duration = longest_duration.max(duration);
        if sim.done {
            let mut alive_teams = HashSet::new();
            for combatant in &sim.combatants {
                if combatant.state.hp > 0 {
                    alive_teams.insert(combatant.team_id);
                }
            }
            if alive_teams.len() == 1 {
                if let Some(team_id) = alive_teams.iter().next() {
                    if let Some(&idx) = team_index.get(team_id) {
                        wins[idx] += 1;
                    } else {
                        ties += 1;
                    }
                } else {
                    ties += 1;
                }
            } else {
                ties += 1;
            }
        } else {
            ties += 1;
        }
        if sim
            .combatants
            .iter()
            .any(|combatant| combatant.state.charge_attacks >= 2)
        {
            fights_with_second_charge += 1;
        }
        if sim
            .combatants
            .iter()
            .any(|combatant| combatant.state.saw_trauma)
        {
            fights_with_trauma += 1;
        }
        if sim.trauma_first_exchange {
            fights_with_trauma_first_exchange += 1;
        }
        if sim
            .combatants
            .iter()
            .any(|combatant| combatant.state.max_knockback_ft >= 20.0)
        {
            fights_with_knockback_20ft += 1;
        }
        if sim
            .combatants
            .iter()
            .any(|combatant| combatant.state.charge_started_within_20ft)
        {
            fights_with_charge_within_20ft += 1;
        }
        let mut fight_max_knockback_side = 0.0f32;
        for combatant in &sim.combatants {
            let Some(&team_idx) = team_index.get(&combatant.team_id) else {
                continue;
            };
            let state = &combatant.state;
            highest_single_hit = highest_single_hit.max(state.max_hit_dealt);
            highest_single_shield_hit = highest_single_shield_hit.max(state.max_shield_hit_dealt);
            highest_single_hit_by_team[team_idx] =
                highest_single_hit_by_team[team_idx].max(state.max_hit_dealt);
            highest_single_shield_hit_by_team[team_idx] =
                highest_single_shield_hit_by_team[team_idx].max(state.max_shield_hit_dealt);
            total_damage_dealt_by_team[team_idx] = total_damage_dealt_by_team[team_idx]
                .saturating_add(u64::from(state.total_hp_damage_dealt));
            total_damage_taken_by_team[team_idx] = total_damage_taken_by_team[team_idx]
                .saturating_add(u64::from(state.total_hp_damage_taken));
            total_remaining_hp_by_team[team_idx] =
                total_remaining_hp_by_team[team_idx].saturating_add(state.hp.max(0) as u64);
            fight_max_knockback_side = fight_max_knockback_side.max(state.total_knockback_taken_ft);
        }
        max_total_knockback_one_side_ft =
            max_total_knockback_one_side_ft.max(fight_max_knockback_side);
        total_max_knockback_one_side_ft += fight_max_knockback_side;
    }
    let avg_duration = total_seconds as f32 / runs as f32;
    let avg_damage_dealt_by_team = total_damage_dealt_by_team
        .into_iter()
        .map(|value| value as f32 / runs as f32)
        .collect();
    let avg_damage_taken_by_team = total_damage_taken_by_team
        .into_iter()
        .map(|value| value as f32 / runs as f32)
        .collect();
    let avg_remaining_hp_by_team = total_remaining_hp_by_team
        .into_iter()
        .map(|value| value as f32 / runs as f32)
        .collect();
    BulkSimResult {
        wins,
        ties,
        avg_duration,
        shortest_duration: if shortest_duration == u32::MAX {
            0
        } else {
            shortest_duration
        },
        longest_duration,
        fights_with_second_charge,
        fights_with_trauma,
        fights_with_trauma_first_exchange,
        fights_with_knockback_20ft,
        fights_with_charge_within_20ft,
        highest_single_hit,
        highest_single_shield_hit,
        highest_single_hit_by_team,
        highest_single_shield_hit_by_team,
        avg_damage_dealt_by_team,
        avg_damage_taken_by_team,
        avg_remaining_hp_by_team,
        max_total_knockback_one_side_ft,
        avg_max_knockback_one_side_ft: total_max_knockback_one_side_ft / runs as f32,
    }
}

impl HoldAtBayState {
    fn blocks_advance(&self, idx: usize) -> bool {
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
        a_idx: usize,
        b_idx: usize,
        distance_before: f32,
        distance_after: f32,
        reach_a: f32,
        reach_b: f32,
    ) {
        if self.hold_at_bay.active || self.hold_at_bay.pending {
            return;
        }
        let (holder_idx, target_idx, holder_reach) = if reach_a > reach_b {
            (a_idx, b_idx, reach_a)
        } else if reach_b > reach_a {
            (b_idx, a_idx, reach_b)
        } else {
            return;
        };
        if !self.combatants[holder_idx].sheet.maneuvers.hold_at_bay {
            return;
        }
        let holder_weapon = self.combatants[holder_idx].sheet.offense.weapon.clone();
        let holder_ranged = max_range_cached(
            &mut self.combatants[holder_idx].state,
            WeaponSlot::Primary,
            holder_weapon.as_ref(),
        )
        .is_some();
        if holder_ranged {
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
