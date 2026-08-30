use crate::core::rng::SimRng;
use crate::core::rules::roll_damage_expr;
use crate::core::tactics::{
    TacticalAction, TacticalChannel, TacticalContext, TacticalDecisionPoint, evaluate_channel,
};
use rand::RngCore;

use super::combat::{
    AttackMode, AttackOutcome, CounterAttackOutcome, resolve_attack, resolve_knock_aside,
};
use super::modifiers::{StatIdF32, StatIdI32};
use super::movement::{max_range_for_weapon, range_modifier_for_weapon_with_scale};
use super::types::{
    AttackEvent, CalledShotDelayProfile, CombatEvent, CombatEventKind, Combatant, DamageBreakdown,
    GridPos, KnockAsideEvent, ShieldDamageBreakdown, SimActor, SimConfig, TacticalEvent,
    WeaponSlot,
};
use std::collections::{BTreeMap, HashMap, HashSet};

const CHARGE_MIN_DISTANCE_FT: f32 = 20.0;
const SHIELD_STRIKE_SPEEDUP_SECONDS: f32 = 2.0;
const SIX_PATHS_FOLLOWUP_SECONDS: f32 = 1.0;

fn damage_prevented(
    damage: Option<&DamageBreakdown>,
    shield_damage: Option<&ShieldDamageBreakdown>,
) -> (u32, u32) {
    if let Some(damage) = damage {
        let armor = damage
            .raw_damage
            .max(0)
            .min(damage.effective_armor_dr.max(0)) as u32;
        return (armor, 0);
    }
    if let Some(damage) = shield_damage {
        let raw = damage.raw_damage.max(0);
        let shield = raw.min(damage.shield_dr.max(0));
        let after_shield = (raw - shield).max(0);
        let armor = after_shield.min(damage.effective_armor_dr.max(0));
        return (armor as u32, shield as u32);
    }
    (0, 0)
}

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
    attack_metrics: Vec<AttackMetricSample>,
    rng: SimRng,
    tick_accum: f32,
    hold_at_bay: HoldAtBayState,
}

#[derive(Clone, Debug)]
struct AttackMetricSample {
    time: u32,
    attacker_idx: usize,
    defender_idx: usize,
    direct_hit: bool,
    shield_block: bool,
    hp_damage: u32,
    critical: bool,
    killing_blow: bool,
    raw_damage: u32,
    armor_prevented: u32,
    shield_prevented: u32,
    trauma_applied: bool,
    shield_broken: bool,
    shield_hits_survived_before_break: u32,
}

struct RecordedAttackMetrics {
    attacker_idx: usize,
    defender_idx: usize,
    hp_damage: i32,
    damage_rolled: Option<i32>,
    damage_landed: Option<i32>,
    highest_hit_bucket: Option<bool>,
    instant_kill: bool,
    shield_block: bool,
    shield_broken: bool,
    shield_damage: i32,
    knockback_ft: f32,
    attempted: bool,
    direct_hit: bool,
    critical: bool,
    trauma_applied: bool,
    defender_hp_after: i32,
    armor_prevented: u32,
    shield_prevented: u32,
}

impl RecordedAttackMetrics {
    fn from_attack(event: &AttackOutcome) -> Self {
        let (armor_prevented, shield_prevented) = damage_prevented(
            event.damage_breakdown.as_ref(),
            event.shield_damage_breakdown.as_ref(),
        );
        Self {
            attacker_idx: event.attacker_idx,
            defender_idx: event.defender_idx,
            hp_damage: event.damage,
            damage_rolled: event
                .damage_breakdown
                .as_ref()
                .map(|breakdown| breakdown.raw_damage),
            damage_landed: event
                .damage_breakdown
                .as_ref()
                .map(|breakdown| (breakdown.raw_damage - breakdown.effective_armor_dr).max(0)),
            highest_hit_bucket: match event.critical.as_ref() {
                Some(crit) if crit.instant_kill => None,
                Some(_) => Some(true),
                None => Some(false),
            },
            instant_kill: event
                .critical
                .as_ref()
                .map(|crit| crit.instant_kill)
                .unwrap_or(false),
            shield_block: event.shield_block,
            shield_broken: event
                .shield_damage_breakdown
                .as_ref()
                .map(|breakdown| breakdown.shield_broken)
                .unwrap_or(false),
            shield_damage: event.shield_damage,
            knockback_ft: event.knockback_ft,
            attempted: event.roll.attack_die > 0,
            direct_hit: event.hit,
            critical: event.critical.is_some(),
            trauma_applied: event.trauma_applied,
            defender_hp_after: event.defender_hp_after,
            armor_prevented,
            shield_prevented,
        }
    }

    fn from_counter(event: &CounterAttackOutcome) -> Self {
        let (armor_prevented, shield_prevented) = damage_prevented(
            event.damage_breakdown.as_ref(),
            event.shield_damage_breakdown.as_ref(),
        );
        Self {
            attacker_idx: event.attacker_idx,
            defender_idx: event.defender_idx,
            hp_damage: event.damage,
            damage_rolled: event
                .damage_breakdown
                .as_ref()
                .map(|breakdown| breakdown.raw_damage),
            damage_landed: event
                .damage_breakdown
                .as_ref()
                .map(|breakdown| (breakdown.raw_damage - breakdown.effective_armor_dr).max(0)),
            highest_hit_bucket: match event.critical.as_ref() {
                Some(crit) if crit.instant_kill => None,
                Some(_) => Some(true),
                None => Some(false),
            },
            instant_kill: event
                .critical
                .as_ref()
                .map(|crit| crit.instant_kill)
                .unwrap_or(false),
            shield_block: event.shield_block,
            shield_broken: event
                .shield_damage_breakdown
                .as_ref()
                .map(|breakdown| breakdown.shield_broken)
                .unwrap_or(false),
            shield_damage: event.shield_damage,
            knockback_ft: event.knockback_ft,
            attempted: event.roll.attack_die > 0,
            direct_hit: event.hit,
            critical: event.critical.is_some(),
            trauma_applied: event.trauma_applied,
            defender_hp_after: event.defender_hp_after,
            armor_prevented,
            shield_prevented,
        }
    }
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
            attack_metrics: Vec::new(),
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
        self.attack_metrics.clear();
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

    fn record_attack_metrics(&mut self, metrics: RecordedAttackMetrics) {
        let RecordedAttackMetrics {
            attacker_idx,
            defender_idx,
            hp_damage,
            damage_rolled,
            damage_landed,
            highest_hit_bucket,
            instant_kill,
            shield_block,
            shield_broken,
            shield_damage,
            knockback_ft,
            attempted,
            direct_hit,
            critical,
            trauma_applied,
            defender_hp_after,
            armor_prevented,
            shield_prevented,
        } = metrics;
        let hp_damage_u32 = hp_damage.max(0) as u32;
        let shield_damage_u32 = shield_damage.max(0) as u32;
        let knockback = knockback_ft.max(0.0);

        if attempted {
            let shield_hits_survived_before_break = if shield_broken {
                self.combatants[defender_idx].state.shield_blocks_taken
            } else {
                0
            };
            self.attack_metrics.push(AttackMetricSample {
                time: self.elapsed_seconds,
                attacker_idx,
                defender_idx,
                direct_hit,
                shield_block,
                hp_damage: hp_damage_u32,
                critical,
                killing_blow: defender_hp_after <= 0,
                raw_damage: damage_rolled.unwrap_or(shield_damage.max(0)).max(0) as u32,
                armor_prevented,
                shield_prevented,
                trauma_applied,
                shield_broken,
                shield_hits_survived_before_break,
            });
        }

        {
            let attacker = &mut self.combatants[attacker_idx].state;
            if let Some(critical_hit) = highest_hit_bucket {
                if critical_hit {
                    attacker.max_crit_hit_dealt = attacker.max_crit_hit_dealt.max(hp_damage.max(0));
                } else {
                    attacker.max_noncrit_hit_dealt =
                        attacker.max_noncrit_hit_dealt.max(hp_damage.max(0));
                }
            }
            attacker.max_shield_hit_dealt = attacker.max_shield_hit_dealt.max(shield_damage.max(0));
            attacker.total_hp_damage_dealt =
                attacker.total_hp_damage_dealt.saturating_add(hp_damage_u32);
            if let Some(damage_rolled) = damage_rolled {
                attacker.total_damage_rolled_dealt += i64::from(damage_rolled);
                attacker.damage_rolls_dealt = attacker.damage_rolls_dealt.saturating_add(1);
            }
            if let Some(damage_landed) = damage_landed {
                attacker.total_damage_landed_dealt += i64::from(damage_landed);
            }
            attacker.total_shield_damage_dealt = attacker
                .total_shield_damage_dealt
                .saturating_add(shield_damage_u32);
            if instant_kill {
                attacker.total_instakills_dealt = attacker.total_instakills_dealt.saturating_add(1);
            }
            attacker.total_knockback_inflicted_ft += knockback;
        }
        {
            let defender = &mut self.combatants[defender_idx].state;
            defender.total_hp_damage_taken =
                defender.total_hp_damage_taken.saturating_add(hp_damage_u32);
            defender.total_shield_damage_taken = defender
                .total_shield_damage_taken
                .saturating_add(shield_damage_u32);
            if shield_block {
                defender.shield_blocks_taken = defender.shield_blocks_taken.saturating_add(1);
                if shield_broken {
                    defender.total_shield_breaks_taken =
                        defender.total_shield_breaks_taken.saturating_add(1);
                    defender.total_shield_hits_survived_before_break = defender
                        .total_shield_hits_survived_before_break
                        .saturating_add(defender.shield_blocks_taken.saturating_sub(1));
                }
            }
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
                combatant.state.total_trauma_seconds_suffered = combatant
                    .state
                    .total_trauma_seconds_suffered
                    .saturating_add(1);
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
            let max_reach = reach_a.max(reach_b);
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
            let eyesmite_a = self.combatants[a_idx].sheet.defense.eyesmite;
            let eyesmite_b = self.combatants[b_idx].sheet.defense.eyesmite;

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
                    if eyesmite_a && distance > 5.0 && !self.hold_at_bay.blocks_advance(a_idx) {
                        self.move_toward(a_idx, b_idx, step_a, 5.0);
                    }
                    let distance = self.distance_between(a_idx, b_idx).unwrap_or(0.0);
                    if eyesmite_b && distance > 5.0 && !self.hold_at_bay.blocks_advance(b_idx) {
                        self.move_toward(b_idx, a_idx, step_b, 5.0);
                    }
                    let distance = self.distance_between(a_idx, b_idx).unwrap_or(0.0);
                    let engaged = distance <= min_reach;
                    if !engaged {
                        if !eyesmite_a && ranged_projectile_a {
                            if let Some(max_range) = max_range_a {
                                if distance <= max_range {
                                    self.move_away(a_idx, b_idx, step_a);
                                } else {
                                    self.move_toward(a_idx, b_idx, step_a, max_reach);
                                }
                            }
                        } else if !eyesmite_a && distance > reach_a {
                            self.move_toward(a_idx, b_idx, step_a, max_reach);
                        }
                        let distance = self.distance_between(a_idx, b_idx).unwrap_or(0.0);
                        if !eyesmite_b && ranged_projectile_b {
                            if let Some(max_range) = max_range_b {
                                if distance <= max_range {
                                    self.move_away(b_idx, a_idx, step_b);
                                } else {
                                    self.move_toward(b_idx, a_idx, step_b, max_reach);
                                }
                            }
                        } else if !eyesmite_b && distance > reach_b {
                            self.move_toward(b_idx, a_idx, step_b, max_reach);
                        }
                    }
                } else if distance > 5.0 && (eyesmite_a || eyesmite_b) {
                    if eyesmite_a && !self.hold_at_bay.blocks_advance(a_idx) {
                        self.move_toward(a_idx, b_idx, step_a, 5.0);
                    }
                    let distance = self.distance_between(a_idx, b_idx).unwrap_or(0.0);
                    if eyesmite_b && distance > 5.0 && !self.hold_at_bay.blocks_advance(b_idx) {
                        self.move_toward(b_idx, a_idx, step_b, 5.0);
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

    fn tactical_context(&self, my_idx: usize, enemy_idx: usize) -> TacticalContext {
        let mine = &self.combatants[my_idx];
        let enemy = &self.combatants[enemy_idx];
        let hp_percent = |combatant: &Combatant| {
            if combatant.sheet.vitals.max_hp <= 0 {
                0.0
            } else {
                combatant.state.hp.max(0) as f32 / combatant.sheet.vitals.max_hp as f32 * 100.0
            }
        };
        let reach = |combatant: &Combatant| {
            combatant
                .apply_f32(
                    StatIdF32::WeaponReach,
                    combatant.sheet.offense.weapon.reach_ft,
                )
                .max(1.0)
        };
        let attack_speed = |combatant: &Combatant| {
            combatant
                .apply_f32(StatIdF32::WeaponSpeed, combatant.sheet.offense.weapon.speed)
                .max(1.0)
        };
        let move_speed = |combatant: &Combatant| {
            combatant
                .apply_f32(StatIdF32::MoveSpeed, combatant.sheet.mobility.move_speed)
                .max(0.0)
        };
        let retreat_space_available = self
            .actors
            .get(my_idx)
            .zip(self.actors.get(enemy_idx))
            .map(|(mine, enemy)| {
                let next = Self::step_away(mine.position, enemy.position)
                    .clamp(self.config.grid_width, self.config.grid_height);
                next != mine.position
            })
            .unwrap_or(false);
        let enemy_charging = enemy.sheet.maneuvers.charge
            && enemy.state.charge_target_idx == Some(my_idx)
            && enemy.state.charge_distance_ft > 0.0;
        let give_ground_legal = retreat_space_available
            && !enemy_charging
            && move_speed(enemy) <= move_speed(mine)
            && self.move_tiles(my_idx) > 0;
        let distance_ft = self.distance_between(my_idx, enemy_idx).unwrap_or(0.0);
        let enemy_reach_ft = reach(enemy);
        let enemy_distance_to_reach_ft = (distance_ft - enemy_reach_ft).max(0.0);
        let enemy_closure_per_second =
            self.move_tiles(enemy_idx) as f32 * self.config.tile_size_ft.max(0.01);
        let enemy_time_to_reach_seconds = if enemy_distance_to_reach_ft <= 0.0 {
            0.0
        } else if enemy_closure_per_second <= 0.0 {
            f32::INFINITY
        } else {
            (enemy_distance_to_reach_ft / enemy_closure_per_second).ceil()
        };

        TacticalContext {
            my_hp_percent: hp_percent(mine),
            enemy_hp_percent: hp_percent(enemy),
            distance_ft,
            my_reach_ft: reach(mine),
            enemy_reach_ft,
            retreat_space_available,
            my_weapon_can_jab: mine.tactical_jab_available(),
            my_has_active_shield: mine.sheet.defense.shield_name.is_some()
                && mine.state.shield_intact,
            enemy_weapon_group: enemy.weapon_group.clone(),
            enemy_has_active_shield: enemy.sheet.defense.shield_name.is_some()
                && enemy.state.shield_intact,
            enemy_armor_type: enemy.armor_type.clone(),
            enemy_charging,
            my_has_attacked: mine.state.has_attacked,
            enemy_time_to_reach_seconds,
            my_active_style_ids: mine.active_style_ids.clone(),
            enemy_active_style_ids: enemy.active_style_ids.clone(),
            available_style_ids: mine.available_tactical_style_ids(),
            style_pair_allowed: mine.tactical_style_pair_allowed(),
            enemy_dr: enemy
                .apply_i32(StatIdI32::ArmorDr, enemy.sheet.defense.armor_dr)
                .max(0) as f32,
            my_attack_speed_seconds: attack_speed(mine),
            enemy_attack_speed_seconds: attack_speed(enemy),
            give_ground_legal,
        }
    }

    fn record_tactical_directive(
        &mut self,
        actor_idx: usize,
        target_idx: usize,
        action: &TacticalAction,
        rule_index: Option<usize>,
        detail: Option<String>,
    ) {
        let rule_label = rule_index
            .map(|index| format!("rule {}", index + 1))
            .unwrap_or_else(|| "fallback".to_string());
        let action_label = action.label();
        let actor_name = self.combatants[actor_idx].sheet.name.clone();
        let message = detail
            .unwrap_or_else(|| format!("{actor_name} uses tactical {rule_label}: {action_label}"));
        self.combatants[actor_idx].last_tactical_directive = Some(message.clone());
        if self.log_events {
            let event = CombatEvent {
                time: self.elapsed_seconds,
                attacker_idx: actor_idx,
                defender_idx: target_idx,
                kind: CombatEventKind::Tactical(TacticalEvent {
                    rule_index,
                    action: action_label,
                    message,
                }),
            };
            self.last_event = Some(event.clone());
            self.combat_events.push(event);
        }
    }

    fn apply_next_attack_tactics(&mut self, attacker_idx: usize, defender_idx: usize) {
        if !self.combatants[attacker_idx].tactical_policy.enabled {
            return;
        }
        let context = self.tactical_context(attacker_idx, defender_idx);
        let policy = self.combatants[attacker_idx].tactical_policy.clone();

        let style = evaluate_channel(
            &policy,
            TacticalDecisionPoint::NextAttackOpportunity,
            TacticalChannel::WeaponStyle,
            &context,
        );
        match &style.action {
            TacticalAction::RetainWeaponStyle => {}
            TacticalAction::NeutralWeaponStyle => {
                self.combatants[attacker_idx].switch_tactical_style(Vec::new());
            }
            TacticalAction::UseWeaponStyle { style_ids } => {
                self.combatants[attacker_idx].switch_tactical_style(style_ids.clone());
            }
            _ => {}
        }
        if style.matched_rule_index.is_some() {
            self.record_tactical_directive(
                attacker_idx,
                defender_idx,
                &style.action,
                style.matched_rule_index,
                None,
            );
        }

        let stance_context = self.tactical_context(attacker_idx, defender_idx);
        let stance = evaluate_channel(
            &policy,
            TacticalDecisionPoint::NextAttackOpportunity,
            TacticalChannel::Stance,
            &stance_context,
        );
        let old_stance = self.combatants[attacker_idx].active_fight_defensively_penalty;
        let new_stance = match stance.action {
            TacticalAction::FightDefensively { penalty } => Some(penalty),
            _ => None,
        };
        if old_stance.is_some() && old_stance != new_stance {
            let lingering = self.combatants[attacker_idx]
                .sheet
                .maneuvers
                .fight_defensively_attack_penalty
                .max(0);
            self.combatants[attacker_idx]
                .state
                .tactical_next_attack_penalty += lingering;
        }
        self.combatants[attacker_idx].active_fight_defensively_penalty = new_stance;
        self.combatants[attacker_idx].activate_tactical_profile(false);
        if stance.matched_rule_index.is_some() {
            self.record_tactical_directive(
                attacker_idx,
                defender_idx,
                &stance.action,
                stance.matched_rule_index,
                None,
            );
        }

        let attack_context = self.tactical_context(attacker_idx, defender_idx);
        let attack = evaluate_channel(
            &policy,
            TacticalDecisionPoint::NextAttackOpportunity,
            TacticalChannel::AttackMode,
            &attack_context,
        );
        let use_jab = matches!(attack.action, TacticalAction::Jab);
        self.combatants[attacker_idx].activate_tactical_profile(use_jab);
        if attack.matched_rule_index.is_some() {
            self.record_tactical_directive(
                attacker_idx,
                defender_idx,
                &attack.action,
                attack.matched_rule_index,
                None,
            );
        }
    }

    fn apply_incoming_attack_tactics(
        &mut self,
        attacker_idx: usize,
        defender_idx: usize,
        attack_mode: AttackMode,
    ) {
        if !self.combatants[defender_idx].tactical_policy.enabled {
            return;
        }
        let mut context = self.tactical_context(defender_idx, attacker_idx);
        if attack_mode == AttackMode::Charge {
            context.give_ground_legal = false;
            context.enemy_charging = true;
        }
        let policy = self.combatants[defender_idx].tactical_policy.clone();
        let reaction = evaluate_channel(
            &policy,
            TacticalDecisionPoint::IncomingAttackReaction,
            TacticalChannel::Reaction,
            &context,
        );
        if !matches!(reaction.action, TacticalAction::GiveGround) {
            if reaction.matched_rule_index.is_some() {
                self.record_tactical_directive(
                    defender_idx,
                    attacker_idx,
                    &reaction.action,
                    reaction.matched_rule_index,
                    None,
                );
            }
            return;
        }

        let before = self.actors[defender_idx].position;
        let original_distance = self
            .distance_between(attacker_idx, defender_idx)
            .unwrap_or(0.0);
        self.move_away(defender_idx, attacker_idx, self.move_tiles(defender_idx));
        let after = self.actors[defender_idx].position;
        let moved_tiles = before.manhattan_distance(after);
        if moved_tiles <= 0 {
            return;
        }
        self.move_toward(attacker_idx, defender_idx, moved_tiles, original_distance);
        self.combatants[defender_idx]
            .state
            .tactical_give_ground_defense_bonus = 5;
        self.combatants[defender_idx]
            .state
            .tactical_next_attack_penalty += 1;
        let moved_ft = moved_tiles as f32 * self.config.tile_size_ft;
        self.record_tactical_directive(
            defender_idx,
            attacker_idx,
            &reaction.action,
            reaction.matched_rule_index,
            Some(format!(
                "{} gives ground {:.0}ft (+5 Defense, -1 next Attack)",
                self.combatants[defender_idx].sheet.name, moved_ft
            )),
        );
    }

    fn grid_distance_tiles(&self, a_idx: usize, b_idx: usize) -> Option<i32> {
        let pos_a = self.actors.get(a_idx)?.position;
        let pos_b = self.actors.get(b_idx)?.position;
        Some(pos_a.manhattan_distance(pos_b))
    }

    fn move_tiles(&self, idx: usize) -> i32 {
        let combatant = &self.combatants[idx];
        if combatant.sheet.maneuvers.passive {
            return 0;
        }
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

    fn precognition_evasion_destination(
        &self,
        defender_idx: usize,
        attacker_idx: usize,
    ) -> Option<GridPos> {
        let from = self.actors.get(defender_idx)?.position;
        let attacker = self.actors.get(attacker_idx)?.position;
        let dx = from.x - attacker.x;
        let dy = from.y - attacker.y;
        let away = if dx.abs() >= dy.abs() && dx != 0 {
            (dx.signum(), 0)
        } else if dy != 0 {
            (0, dy.signum())
        } else {
            return None;
        };
        let directions = [away, (-away.1, away.0), (away.1, -away.0)];
        let steps = (5.0 / self.config.tile_size_ft.max(0.01)).ceil() as i32;

        for (step_x, step_y) in directions {
            let mut current = from;
            let mut valid = true;
            for _ in 0..steps.max(1) {
                let next = GridPos::new(current.x + step_x, current.y + step_y);
                if next.x < 0
                    || next.y < 0
                    || next.x >= self.config.grid_width
                    || next.y >= self.config.grid_height
                    || self.actors.iter().enumerate().any(|(idx, actor)| {
                        idx != defender_idx
                            && self
                                .combatants
                                .get(idx)
                                .map(|combatant| combatant.state.hp > 0)
                                .unwrap_or(false)
                            && actor.position == next
                    })
                {
                    valid = false;
                    break;
                }
                current = next;
            }
            if valid {
                return Some(current);
            }
        }
        None
    }

    fn apply_precognition_evasion(&mut self, defender_idx: usize, destination: Option<GridPos>) {
        if let (Some(actor), Some(destination)) = (self.actors.get_mut(defender_idx), destination) {
            actor.position = destination;
        }
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
            if self.combatants[attacker_idx].sheet.maneuvers.passive {
                self.combatants[attacker_idx].state.clear_attack_timers();
                continue;
            }
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
            let mut weapon = self.combatants[attacker_idx].sheet.offense.weapon.clone();
            let max_range = max_range_cached(
                &mut self.combatants[attacker_idx].state,
                WeaponSlot::Primary,
                weapon.as_ref(),
            );
            let mut has_range = max_range.is_some();
            let mut attacker_reach = weapon.reach_ft.max(1.0);
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
            let mut ranged_mod = if use_ranged {
                let range_scale = self.combatants[attacker_idx].apply_f32(
                    StatIdF32::RangeDistanceMultiplier,
                    weapon.range_distance_multiplier,
                );
                range_modifier_for_weapon_with_scale(weapon.as_ref(), distance, range_scale)
            } else {
                None
            };
            let mut primary_speed_base = self.combatants[attacker_idx]
                .apply_f32(StatIdF32::WeaponSpeed, weapon.speed)
                .max(1.0);
            let mut primary_attack_time = None;
            let mut scheduled_primary_attack_time = None;
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
                let scheduled_time = now + delay + called_shot_delay;
                self.combatants[attacker_idx]
                    .state
                    .set_next_attack_time(WeaponSlot::Primary, Some(scheduled_time));
                scheduled_primary_attack_time = Some(scheduled_time);
            }
            let next_attack = if use_snapshot_timing {
                snapshot_next_attack_primary
                    .or(scheduled_primary_attack_time)
                    .unwrap_or(now)
            } else {
                self.combatants[attacker_idx]
                    .state
                    .next_attack_time_primary
                    .unwrap_or(now)
            };
            if now + 0.0001 >= next_attack {
                primary_attack_time = Some(next_attack);
                self.apply_next_attack_tactics(attacker_idx, defender_idx);

                // The due time is intentionally unchanged. The newly selected
                // profile controls this attack and the recovery scheduled below.
                weapon = self.combatants[attacker_idx].sheet.offense.weapon.clone();
                let updated_max_range = max_range_cached(
                    &mut self.combatants[attacker_idx].state,
                    WeaponSlot::Primary,
                    weapon.as_ref(),
                );
                has_range = updated_max_range.is_some();
                attacker_reach = self.combatants[attacker_idx]
                    .apply_f32(StatIdF32::WeaponReach, weapon.reach_ft)
                    .max(1.0);
                use_ranged = if has_range && !weapon.uses_projectiles {
                    distance > attacker_reach
                } else {
                    has_range
                };
                ranged_mod = if use_ranged {
                    let range_scale = self.combatants[attacker_idx].apply_f32(
                        StatIdF32::RangeDistanceMultiplier,
                        weapon.range_distance_multiplier,
                    );
                    range_modifier_for_weapon_with_scale(weapon.as_ref(), distance, range_scale)
                } else {
                    None
                };
                primary_speed_base = self.combatants[attacker_idx]
                    .apply_f32(StatIdF32::WeaponSpeed, weapon.speed)
                    .max(1.0);
                if (!use_ranged && distance > attacker_reach)
                    || (use_ranged && ranged_mod.is_none())
                {
                    self.combatants[attacker_idx].activate_tactical_profile(false);
                    continue;
                }
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
                self.apply_incoming_attack_tactics(attacker_idx, defender_idx, attack_mode);
                let defender_evasion =
                    self.precognition_evasion_destination(defender_idx, attacker_idx);
                let attacker_evasion =
                    self.precognition_evasion_destination(attacker_idx, defender_idx);
                self.combatants[defender_idx]
                    .state
                    .precognition_space_available = defender_evasion.is_some();
                self.combatants[attacker_idx]
                    .state
                    .precognition_space_available = attacker_evasion.is_some();
                let attack_distance = self
                    .distance_between(attacker_idx, defender_idx)
                    .unwrap_or(distance);
                let mut event = resolve_attack(
                    &mut self.combatants,
                    attacker_idx,
                    defender_idx,
                    ranged_mod.unwrap_or(0),
                    use_ranged,
                    attack_distance,
                    attack_mode,
                    WeaponSlot::Primary,
                    now,
                    state_snapshot.as_deref(),
                    &mut self.rng,
                );
                self.combatants[defender_idx]
                    .state
                    .tactical_give_ground_defense_bonus = 0;
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
                self.record_attack_metrics(RecordedAttackMetrics::from_attack(&event));
                if event.precognition_triggered {
                    self.apply_precognition_evasion(event.defender_idx, defender_evasion);
                }
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
                    self.record_attack_metrics(RecordedAttackMetrics::from_counter(&counter));
                    if counter.precognition_triggered {
                        self.apply_precognition_evasion(counter.defender_idx, attacker_evasion);
                    }
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
                    let mut recovery_penalty = self.combatants[attacker_idx]
                        .sheet
                        .maneuvers
                        .dualwield_primary_recovery_penalty
                        .max(0.0);
                    if self.combatants[attacker_idx]
                        .sheet
                        .maneuvers
                        .storm_of_blades
                    {
                        recovery_penalty = (recovery_penalty - 1.0).max(0.0);
                    }
                    speed += recovery_penalty;
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
                self.combatants[attacker_idx]
                    .state
                    .tactical_next_attack_penalty = 0;
                self.combatants[attacker_idx].activate_tactical_profile(false);
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
                let mut scheduled_secondary_attack_time = None;
                if self.combatants[attacker_idx]
                    .state
                    .next_attack_time_secondary
                    .is_none()
                {
                    let primary_anchor = primary_attack_time
                        .or_else(|| self.combatants[attacker_idx].state.next_attack_time_primary)
                        .unwrap_or(now);
                    let offset = if self.combatants[attacker_idx]
                        .sheet
                        .maneuvers
                        .storm_of_blades
                    {
                        2.0
                    } else {
                        2.0 + (primary_speed_base / 2.0).ceil()
                    };
                    let called_shot_delay = called_shot_delay_seconds(
                        &self.combatants[attacker_idx],
                        &self.combatants[defender_idx],
                        use_ranged,
                        &mut self.rng,
                    );
                    let scheduled_time = primary_anchor + offset + called_shot_delay;
                    self.combatants[attacker_idx]
                        .state
                        .set_next_attack_time(WeaponSlot::Secondary, Some(scheduled_time));
                    scheduled_secondary_attack_time = Some(scheduled_time);
                }
                let next_attack = if use_snapshot_timing {
                    snapshot_next_attack_secondary
                        .or(scheduled_secondary_attack_time)
                        .unwrap_or(now)
                } else {
                    self.combatants[attacker_idx]
                        .state
                        .next_attack_time_secondary
                        .unwrap_or(now)
                };
                if now + 0.0001 >= next_attack {
                    self.apply_incoming_attack_tactics(
                        attacker_idx,
                        defender_idx,
                        AttackMode::Normal,
                    );
                    let defender_evasion =
                        self.precognition_evasion_destination(defender_idx, attacker_idx);
                    let attacker_evasion =
                        self.precognition_evasion_destination(attacker_idx, defender_idx);
                    self.combatants[defender_idx]
                        .state
                        .precognition_space_available = defender_evasion.is_some();
                    self.combatants[attacker_idx]
                        .state
                        .precognition_space_available = attacker_evasion.is_some();
                    let attack_distance = self
                        .distance_between(attacker_idx, defender_idx)
                        .unwrap_or(distance);
                    let mut event = resolve_attack(
                        &mut self.combatants,
                        attacker_idx,
                        defender_idx,
                        ranged_mod.unwrap_or(0),
                        use_ranged,
                        attack_distance,
                        AttackMode::Normal,
                        WeaponSlot::Secondary,
                        now,
                        state_snapshot.as_deref(),
                        &mut self.rng,
                    );
                    self.combatants[defender_idx]
                        .state
                        .tactical_give_ground_defense_bonus = 0;
                    let six_paths_followup = event.hit && !event.is_ranged;
                    if event.shield_block {
                        self.apply_shield_strike_speedup(event.defender_idx, now);
                    }
                    self.record_attack_metrics(RecordedAttackMetrics::from_attack(&event));
                    if event.precognition_triggered {
                        self.apply_precognition_evasion(event.defender_idx, defender_evasion);
                    }
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
                        self.record_attack_metrics(RecordedAttackMetrics::from_counter(&counter));
                        if counter.precognition_triggered {
                            self.apply_precognition_evasion(counter.defender_idx, attacker_evasion);
                        }
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
                    let mut recovery_penalty = self.combatants[attacker_idx]
                        .sheet
                        .maneuvers
                        .dualwield_secondary_recovery_penalty
                        .max(0.0);
                    if self.combatants[attacker_idx]
                        .sheet
                        .maneuvers
                        .storm_of_blades
                    {
                        recovery_penalty = (recovery_penalty - 1.0).max(0.0);
                    }
                    let mut speed = self.combatants[attacker_idx]
                        .apply_f32(StatIdF32::WeaponSpeed, weapon.speed)
                        .max(1.0)
                        + recovery_penalty;
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
    use crate::core::sim::{CombatantTacticalProfile, TacticalProfileKey};
    use crate::core::sim::{ModifierOpF32, ModifierOpI32, TemporaryEffect};
    use crate::core::tactics::{TacticalCondition, TacticalPolicy, TacticalRule};

    fn tactical_profile(
        base: &Combatant,
        style_ids: Vec<String>,
        use_jab: bool,
        stance: Option<i32>,
        speed: f32,
    ) -> CombatantTacticalProfile {
        let mut sheet = base.sheet.clone();
        let mut weapon = (*sheet.offense.weapon).clone();
        weapon.speed = speed;
        weapon.use_jab = use_jab;
        if use_jab {
            weapon.force_nonpenetrating_damage = true;
            weapon.halve_damage = true;
        }
        sheet.offense.weapon = std::sync::Arc::new(weapon);
        sheet.maneuvers.fight_defensively = stance.is_some();
        sheet.maneuvers.fight_defensively_attack_penalty = stance.unwrap_or(0);
        sheet.maneuvers.fight_defensively_defense_bonus = stance.unwrap_or(0) / 2;
        CombatantTacticalProfile {
            key: TacticalProfileKey {
                style_ids,
                use_jab,
                fight_defensively_penalty: stance,
            },
            sheet,
            weapon_group: "Test".to_string(),
            armor_type: "None".to_string(),
        }
    }

    fn always_policy(action: TacticalAction) -> TacticalPolicy {
        TacticalPolicy {
            enabled: true,
            rules: vec![TacticalRule::new(action, vec![TacticalCondition::Always])],
        }
    }

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
    fn jab_directive_uses_jab_profile_for_attack_and_recovery() {
        let mut attacker = Combatant::default();
        attacker.team_id = 0;
        attacker.sheet.vitals.max_hp = 1_000;
        attacker.reset_state();
        let profiles = vec![
            tactical_profile(&attacker, Vec::new(), false, None, 9.0),
            tactical_profile(&attacker, Vec::new(), true, None, 3.0),
        ];
        attacker.configure_tactical_profiles(always_policy(TacticalAction::Jab), profiles, vec![]);

        let mut defender = Combatant::default();
        defender.team_id = 1;
        defender.sheet.vitals.max_hp = 1_000;
        defender.sheet.maneuvers.passive = true;
        defender.reset_state();

        let mut sim = SimState::with_rng(SimConfig::new(1.0, 1.0), SimRng::from_seed(77));
        sim.reset_with_combatants(vec![attacker, defender]);
        sim.tick();

        let attack = sim
            .combat_events
            .iter()
            .find_map(|event| match &event.kind {
                CombatEventKind::Attack(attack) if event.attacker_idx == 0 => Some(attack),
                _ => None,
            })
            .expect("tactical attacker should attack");
        assert!(attack.use_jab);
        let next = sim.combatants[0]
            .state
            .next_attack_time_primary
            .expect("recovery timer");
        assert!(
            (next - 3.0).abs() < 0.001,
            "expected Jab recovery at 3s, got {next}"
        );
        assert!(!sim.combatants[0].sheet.offense.weapon.use_jab);
    }

    #[test]
    fn style_switch_waits_for_already_scheduled_attack_opportunity() {
        let mut attacker = Combatant::default();
        attacker.team_id = 0;
        attacker.sheet.vitals.max_hp = 1_000;
        attacker.reset_state();
        let profiles = vec![
            tactical_profile(&attacker, Vec::new(), false, None, 9.0),
            tactical_profile(&attacker, vec!["fast_style".to_string()], false, None, 2.0),
        ];
        attacker.configure_tactical_profiles(
            always_policy(TacticalAction::UseWeaponStyle {
                style_ids: vec!["fast_style".to_string()],
            }),
            profiles,
            vec![],
        );

        let mut defender = Combatant::default();
        defender.team_id = 1;
        defender.sheet.vitals.max_hp = 1_000;
        defender.sheet.maneuvers.passive = true;
        defender.reset_state();
        let mut sim = SimState::with_rng(SimConfig::new(1.0, 1.0), SimRng::from_seed(78));
        sim.reset_with_combatants(vec![attacker, defender]);
        sim.combatants[0]
            .state
            .set_next_attack_time(WeaponSlot::Primary, Some(5.0));

        for _ in 0..4 {
            sim.tick();
        }
        assert!(sim.combatants[0].active_style_ids.is_empty());
        sim.tick();
        assert!(sim.combatants[0].active_style_ids.is_empty());
        sim.tick();
        assert_eq!(
            sim.combatants[0].active_style_ids,
            vec!["fast_style".to_string()]
        );
        let next = sim.combatants[0]
            .state
            .next_attack_time_primary
            .expect("recovery timer");
        assert!(
            (next - 7.0).abs() < 0.001,
            "new style should set 2s recovery"
        );
    }

    #[test]
    fn style_switch_to_shorter_equal_reach_closes_to_live_engagement_distance() {
        let mut attacker = Combatant::default();
        attacker.team_id = 0;
        attacker.sheet.vitals.max_hp = 1_000;
        attacker.sheet.mobility.move_speed = 20.0;
        let mut opening_weapon = (*attacker.sheet.offense.weapon).clone();
        opening_weapon.reach_ft = 8.0;
        attacker.sheet.offense.weapon = std::sync::Arc::new(opening_weapon);
        attacker.reset_state();

        let opening_profile =
            tactical_profile(&attacker, vec!["long_style".to_string()], false, None, 9.0);
        let mut short_profile =
            tactical_profile(&attacker, vec!["short_style".to_string()], false, None, 2.0);
        let mut short_weapon = (*short_profile.sheet.offense.weapon).clone();
        short_weapon.reach_ft = 4.0;
        short_profile.sheet.offense.weapon = std::sync::Arc::new(short_weapon);
        attacker.configure_tactical_profiles(
            TacticalPolicy::default(),
            vec![opening_profile, short_profile],
            vec!["long_style".to_string()],
        );

        let mut defender = Combatant::default();
        defender.team_id = 1;
        defender.sheet.vitals.max_hp = 1_000;
        defender.sheet.mobility.move_speed = 20.0;
        defender.sheet.maneuvers.passive = true;
        let mut defender_weapon = (*defender.sheet.offense.weapon).clone();
        defender_weapon.reach_ft = 4.0;
        defender.sheet.offense.weapon = std::sync::Arc::new(defender_weapon);
        defender.reset_state();

        let mut sim = SimState::with_rng(SimConfig::new(8.0, 8.0), SimRng::from_seed(79));
        sim.reset_with_combatants(vec![attacker, defender]);
        assert!((sim.distance() - 8.0).abs() < 0.001);
        assert!(sim.combatants[0].switch_tactical_style(vec!["short_style".to_string()]));

        sim.tick();

        assert!(
            sim.distance() <= 4.0,
            "fighters should close to their live 4ft reach after the style switch, got {}ft",
            sim.distance()
        );
    }

    #[test]
    fn armeroci_opening_attack_precedes_rohavalan_switch() {
        let mut attacker = Combatant::default();
        attacker.team_id = 0;
        attacker.sheet.vitals.max_hp = 1_000;
        attacker.reset_state();
        let mut armeroci_profile = tactical_profile(
            &attacker,
            vec!["armeroci_pole".to_string()],
            false,
            None,
            9.0,
        );
        armeroci_profile
            .sheet
            .modifiers
            .add_i32(StatIdI32::FlagArmerociPoleStyle, ModifierOpI32::Set(1));
        let rohavalan_profile = tactical_profile(
            &attacker,
            vec!["rohavalan_bridge".to_string()],
            false,
            None,
            2.0,
        );
        let rohavalan_jab_profile = tactical_profile(
            &attacker,
            vec!["rohavalan_bridge".to_string()],
            true,
            None,
            1.0,
        );
        let policy = TacticalPolicy {
            enabled: true,
            rules: vec![
                TacticalRule::new(
                    TacticalAction::UseWeaponStyle {
                        style_ids: vec!["rohavalan_bridge".to_string()],
                    },
                    vec![
                        TacticalCondition::MyHasAttacked { value: true },
                        TacticalCondition::MyActiveStyle {
                            style_id: "armeroci_pole".to_string(),
                            negated: false,
                        },
                    ],
                ),
                TacticalRule::new(
                    TacticalAction::Jab,
                    vec![
                        TacticalCondition::MyActiveStyle {
                            style_id: "rohavalan_bridge".to_string(),
                            negated: false,
                        },
                        TacticalCondition::MyWeaponCanJab { value: true },
                    ],
                ),
            ],
        };
        attacker.configure_tactical_profiles(
            policy,
            vec![armeroci_profile, rohavalan_profile, rohavalan_jab_profile],
            vec!["armeroci_pole".to_string()],
        );
        attacker.state.armeroci_opening_strike_available = true;

        let mut defender = Combatant::default();
        defender.team_id = 1;
        defender.sheet.vitals.max_hp = 1_000;
        defender.sheet.maneuvers.passive = true;
        defender.reset_state();

        let mut sim = SimState::with_rng(SimConfig::new(1.0, 1.0), SimRng::from_seed(781));
        sim.reset_with_combatants(vec![attacker, defender]);
        sim.combatants[0].state.armeroci_opening_strike_available = true;

        sim.tick();
        assert_eq!(
            sim.combatants[0].active_style_ids,
            vec!["armeroci_pole".to_string()]
        );
        assert!(sim.combatants[0].state.has_attacked);
        assert!(!sim.combatants[0].state.armeroci_opening_strike_available);

        for _ in 0..12 {
            sim.tick();
            if sim.combatants[0].active_style_ids == ["rohavalan_bridge".to_string()] {
                break;
            }
        }
        assert_eq!(
            sim.combatants[0].active_style_ids,
            vec!["rohavalan_bridge".to_string()]
        );
        let switched_attack = sim
            .combat_events
            .iter()
            .rev()
            .find_map(|event| match &event.kind {
                CombatEventKind::Attack(attack) if event.attacker_idx == 0 && event.time > 0 => {
                    Some(attack)
                }
                _ => None,
            })
            .expect("expected an attack after switching to Rohavalan");
        assert!(switched_attack.use_jab);

        sim.combatants[0].activate_tactical_profile(false);
        assert!(!sim.combatants[0].sheet.offense.weapon.use_jab);
        sim.apply_next_attack_tactics(0, 1);
        assert!(sim.combatants[0].sheet.offense.weapon.use_jab);
        assert!((sim.combatants[0].sheet.offense.weapon.speed - 1.0).abs() < 0.001);
    }

    #[test]
    fn tactical_context_reports_enemy_time_to_reach() {
        let mut mine = Combatant::default();
        mine.team_id = 0;
        let mut enemy = Combatant::default();
        enemy.team_id = 1;
        enemy.sheet.mobility.move_speed = 5.0;
        enemy.sheet.offense.weapon = {
            let mut weapon = (*enemy.sheet.offense.weapon).clone();
            weapon.reach_ft = 1.0;
            std::sync::Arc::new(weapon)
        };
        let mut sim = SimState::with_rng(SimConfig::new(25.0, 1.0), SimRng::from_seed(782));
        sim.reset_with_combatants(vec![mine, enemy]);

        let context = sim.tactical_context(0, 1);
        assert_eq!(context.enemy_time_to_reach_seconds, 5.0);

        sim.combatants[1].sheet.maneuvers.passive = true;
        assert!(
            sim.tactical_context(0, 1)
                .enemy_time_to_reach_seconds
                .is_infinite()
        );
    }

    #[test]
    fn tactical_context_reports_when_my_shield_breaks() {
        let mut mine = Combatant::default();
        mine.team_id = 0;
        mine.sheet.defense.shield_name = Some("Buckler".to_string());
        mine.state.shield_intact = true;
        let mut enemy = Combatant::default();
        enemy.team_id = 1;
        let mut sim = SimState::new(SimConfig::new(1.0, 1.0));
        sim.reset_with_combatants(vec![mine, enemy]);

        assert!(sim.tactical_context(0, 1).my_has_active_shield);
        sim.combatants[0].state.shield_intact = false;
        assert!(!sim.tactical_context(0, 1).my_has_active_shield);
    }

    #[test]
    fn give_ground_moves_both_combatants_and_sets_roll_modifiers() {
        let mut attacker = Combatant::default();
        attacker.team_id = 0;
        attacker.sheet.mobility.move_speed = 5.0;
        let mut defender = Combatant::default();
        defender.team_id = 1;
        defender.sheet.mobility.move_speed = 10.0;
        defender.tactical_policy = always_policy(TacticalAction::GiveGround);
        let mut sim = SimState::with_rng(SimConfig::new(5.0, 1.0), SimRng::from_seed(79));
        sim.reset_with_combatants(vec![attacker, defender]);
        let attacker_before = sim.actors[0].position;
        let defender_before = sim.actors[1].position;

        sim.apply_incoming_attack_tactics(0, 1, AttackMode::Normal);

        assert_ne!(sim.actors[1].position, defender_before);
        assert_ne!(sim.actors[0].position, attacker_before);
        assert_eq!(
            sim.combatants[1].state.tactical_give_ground_defense_bonus,
            5
        );
        assert_eq!(sim.combatants[1].state.tactical_next_attack_penalty, 1);
        assert!(
            sim.combat_events
                .iter()
                .any(|event| matches!(event.kind, CombatEventKind::Tactical(_)))
        );
    }

    #[test]
    fn dropping_defensive_stance_preserves_penalty_for_next_attack() {
        let mut attacker = Combatant::default();
        attacker.team_id = 0;
        let profiles = vec![
            tactical_profile(&attacker, Vec::new(), false, None, 8.0),
            tactical_profile(&attacker, Vec::new(), false, Some(4), 8.0),
        ];
        attacker.configure_tactical_profiles(
            always_policy(TacticalAction::FightDefensively { penalty: 4 }),
            profiles,
            vec![],
        );
        let mut defender = Combatant::default();
        defender.team_id = 1;
        let mut sim = SimState::new(SimConfig::new(1.0, 1.0));
        sim.reset_with_combatants(vec![attacker, defender]);
        sim.apply_next_attack_tactics(0, 1);
        assert_eq!(sim.combatants[0].active_fight_defensively_penalty, Some(4));

        sim.combatants[0].tactical_policy = always_policy(TacticalAction::NeutralStance);
        sim.apply_next_attack_tactics(0, 1);
        assert_eq!(sim.combatants[0].active_fight_defensively_penalty, None);
        assert_eq!(sim.combatants[0].state.tactical_next_attack_penalty, 4);
    }

    #[test]
    fn tactical_context_uses_temporary_dr_and_speed_modifiers() {
        let mut mine = Combatant::default();
        mine.team_id = 0;
        let mut enemy = Combatant::default();
        enemy.team_id = 1;
        enemy.sheet.defense.armor_dr = 4;
        let mut enemy_weapon = (*enemy.sheet.offense.weapon).clone();
        enemy_weapon.speed = 10.0;
        enemy.sheet.offense.weapon = std::sync::Arc::new(enemy_weapon);
        let mut sim = SimState::new(SimConfig::new(1.0, 1.0));
        sim.reset_with_combatants(vec![mine, enemy]);

        let mut effect = TemporaryEffect::new("tactical_context_effect", 10);
        effect
            .modifiers
            .add_i32(StatIdI32::ArmorDr, ModifierOpI32::Add(3));
        effect
            .modifiers
            .add_f32(StatIdF32::WeaponSpeed, ModifierOpF32::Add(-4.0));
        sim.combatants[1].state.active_effects.push(effect);
        let context = sim.tactical_context(0, 1);
        assert_eq!(context.enemy_dr, 7.0);
        assert_eq!(
            context.enemy_attack_speed_seconds,
            sim.combatants[1].sheet.offense.weapon.speed - 4.0
        );
    }

    #[test]
    fn seeded_tactical_simulation_is_deterministic() {
        let mut attacker = Combatant::default();
        attacker.team_id = 0;
        attacker.sheet.vitals.max_hp = 500;
        attacker.reset_state();
        let profiles = vec![
            tactical_profile(&attacker, Vec::new(), false, None, 8.0),
            tactical_profile(&attacker, Vec::new(), true, None, 3.0),
        ];
        attacker.configure_tactical_profiles(always_policy(TacticalAction::Jab), profiles, vec![]);
        let mut defender = Combatant::default();
        defender.team_id = 1;
        defender.sheet.vitals.max_hp = 500;
        defender.reset_state();

        let run = |seed| {
            let mut sim = SimState::with_rng(SimConfig::new(1.0, 1.0), SimRng::from_seed(seed));
            sim.reset_with_combatants(vec![attacker.clone(), defender.clone()]);
            for _ in 0..20 {
                sim.tick();
            }
            let events = sim
                .combat_events
                .iter()
                .map(|event| match &event.kind {
                    CombatEventKind::Attack(attack) => format!(
                        "{}:{}:{}:{}:{}",
                        event.time, event.attacker_idx, attack.use_jab, attack.hit, attack.damage
                    ),
                    CombatEventKind::KnockAside(knock) => {
                        format!("{}:knock:{}", event.time, knock.success)
                    }
                    CombatEventKind::Tactical(tactical) => {
                        format!("{}:tactic:{}", event.time, tactical.action)
                    }
                })
                .collect::<Vec<_>>();
            (
                sim.combatants
                    .iter()
                    .map(|combatant| combatant.state.hp)
                    .collect::<Vec<_>>(),
                events,
            )
        };
        assert_eq!(run(4242), run(4242));
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
pub struct DetailedSimStats {
    pub duration_p10: u32,
    pub duration_p50: u32,
    pub duration_p90: u32,
    pub duration_p99: u32,
    pub teams: Vec<DetailedTeamStats>,
}

#[derive(Clone, Debug, Default)]
pub struct DetailedTeamStats {
    pub team_id: u8,
    pub wins: u32,
    pub win_rate: f32,
    pub win_rate_ci_low: f32,
    pub win_rate_ci_high: f32,
    pub avg_winning_hp: Option<f32>,
    pub median_winning_hp: Option<u32>,
    pub avg_winning_duration_seconds: Option<f32>,
    pub median_winning_duration_seconds: Option<u32>,
    pub attack_attempts: u64,
    pub direct_hits: u64,
    pub shield_blocks: u64,
    pub misses: u64,
    pub hp_hits: u64,
    pub critical_hits: u64,
    pub kills: u64,
    pub eyesmite_available: bool,
    pub eyes_smote: u64,
    pub avg_attacks_per_fight: f32,
    pub direct_hit_rate: f32,
    pub contact_rate: f32,
    pub shield_block_rate: f32,
    pub hp_hit_rate: f32,
    pub critical_rate_per_attack: f32,
    pub critical_rate_per_direct_hit: f32,
    pub avg_hp_damage_per_fight: f32,
    pub combat_dps: f32,
    pub hp_damage_per_attack: f32,
    pub hp_damage_per_hp_hit: f32,
    pub hp_damage_p50: u32,
    pub hp_damage_p90: u32,
    pub hp_damage_p99: u32,
    pub avg_first_attack_seconds: Option<f32>,
    pub avg_attack_interval_seconds: Option<f32>,
    pub incoming_raw_damage_per_fight: f32,
    pub armor_prevented_per_fight: f32,
    pub shield_prevented_per_fight: f32,
    pub hp_damage_taken_per_fight: f32,
    pub damage_prevention_rate: f32,
    pub fights_with_trauma_inflicted: u32,
    pub trauma_events_inflicted: u64,
    pub trauma_chance_per_fight: f32,
    pub avg_trauma_downtime_seconds: f32,
    pub shield_fights: u32,
    pub fights_with_shield_break: u32,
    pub shield_break_rate: Option<f32>,
    pub avg_hits_shield_survived_before_break: Option<f32>,
    pub median_hits_shield_survived_before_break: Option<u32>,
    pub avg_shield_break_seconds: Option<f32>,
    pub median_shield_break_seconds: Option<u32>,
}

#[derive(Default)]
struct DetailedTeamAccumulator {
    team_id: u8,
    wins: u32,
    attack_attempts: u64,
    direct_hits: u64,
    shield_blocks: u64,
    hp_hits: u64,
    critical_hits: u64,
    kills: u64,
    eyesmite_available: bool,
    eyes_smote: u64,
    total_hp_damage: u64,
    total_incoming_raw_damage: u64,
    total_armor_prevented: u64,
    total_shield_prevented: u64,
    total_hp_damage_taken: u64,
    first_attack_seconds_total: u64,
    fights_with_attack: u32,
    attack_interval_seconds_total: u64,
    attack_intervals: u64,
    fights_with_trauma_inflicted: u32,
    trauma_events_inflicted: u64,
    total_trauma_downtime_seconds: u64,
    shield_fights: u32,
    fights_with_shield_break: u32,
    total_hits_shield_survived_before_break: u64,
    shield_breaks: u64,
    shield_hits_before_break_histogram: BTreeMap<u64, u64>,
    shield_break_seconds_total: u64,
    shield_break_time_histogram: BTreeMap<u64, u64>,
    winning_hp_total: u64,
    winning_hp_histogram: BTreeMap<u64, u64>,
    winning_duration_seconds_total: u64,
    winning_duration_seconds_histogram: BTreeMap<u64, u64>,
    hp_damage_histogram: BTreeMap<u64, u64>,
}

fn add_histogram_sample(histogram: &mut BTreeMap<u64, u64>, value: u64) {
    *histogram.entry(value).or_insert(0) += 1;
}

fn histogram_count(histogram: &BTreeMap<u64, u64>) -> u64 {
    histogram.values().copied().sum()
}

fn histogram_percentile(histogram: &BTreeMap<u64, u64>, percentile: f64) -> u64 {
    let count = histogram_count(histogram);
    if count == 0 {
        return 0;
    }
    let target = ((count as f64 * percentile.clamp(0.0, 1.0)).ceil() as u64).max(1);
    let mut seen = 0u64;
    for (value, occurrences) in histogram {
        seen = seen.saturating_add(*occurrences);
        if seen >= target {
            return *value;
        }
    }
    histogram.keys().next_back().copied().unwrap_or(0)
}

fn rate(numerator: u64, denominator: u64) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

fn wilson_interval(successes: u32, trials: u32) -> (f32, f32) {
    if trials == 0 {
        return (0.0, 0.0);
    }
    let n = f64::from(trials);
    let p = f64::from(successes) / n;
    let z = 1.96f64;
    let z2 = z * z;
    let denominator = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denominator;
    let margin = z * ((p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt()) / denominator;
    (
        (center - margin).max(0.0) as f32,
        (center + margin).min(1.0) as f32,
    )
}

impl DetailedTeamAccumulator {
    fn finish(self, runs: u32, total_seconds: u64) -> DetailedTeamStats {
        let attempts = self.attack_attempts;
        let misses = attempts.saturating_sub(self.direct_hits + self.shield_blocks);
        let (win_rate_ci_low, win_rate_ci_high) = wilson_interval(self.wins, runs);
        let winning_hp_count = histogram_count(&self.winning_hp_histogram);
        let winning_duration_count = histogram_count(&self.winning_duration_seconds_histogram);
        let prevented = self.total_armor_prevented + self.total_shield_prevented;
        DetailedTeamStats {
            team_id: self.team_id,
            wins: self.wins,
            win_rate: rate(u64::from(self.wins), u64::from(runs)),
            win_rate_ci_low,
            win_rate_ci_high,
            avg_winning_hp: (winning_hp_count > 0)
                .then_some(self.winning_hp_total as f32 / winning_hp_count as f32),
            median_winning_hp: (winning_hp_count > 0)
                .then_some(histogram_percentile(&self.winning_hp_histogram, 0.50) as u32),
            avg_winning_duration_seconds: (winning_duration_count > 0).then_some(
                self.winning_duration_seconds_total as f32 / winning_duration_count as f32,
            ),
            median_winning_duration_seconds: (winning_duration_count > 0).then_some(
                histogram_percentile(&self.winning_duration_seconds_histogram, 0.50) as u32,
            ),
            attack_attempts: attempts,
            direct_hits: self.direct_hits,
            shield_blocks: self.shield_blocks,
            misses,
            hp_hits: self.hp_hits,
            critical_hits: self.critical_hits,
            kills: self.kills,
            eyesmite_available: self.eyesmite_available,
            eyes_smote: self.eyes_smote,
            avg_attacks_per_fight: rate(attempts, u64::from(runs)),
            direct_hit_rate: rate(self.direct_hits, attempts),
            contact_rate: rate(self.direct_hits + self.shield_blocks, attempts),
            shield_block_rate: rate(self.shield_blocks, attempts),
            hp_hit_rate: rate(self.hp_hits, attempts),
            critical_rate_per_attack: rate(self.critical_hits, attempts),
            critical_rate_per_direct_hit: rate(self.critical_hits, self.direct_hits),
            avg_hp_damage_per_fight: rate(self.total_hp_damage, u64::from(runs)),
            combat_dps: rate(self.total_hp_damage, total_seconds),
            hp_damage_per_attack: rate(self.total_hp_damage, attempts),
            hp_damage_per_hp_hit: rate(self.total_hp_damage, self.hp_hits),
            hp_damage_p50: histogram_percentile(&self.hp_damage_histogram, 0.50) as u32,
            hp_damage_p90: histogram_percentile(&self.hp_damage_histogram, 0.90) as u32,
            hp_damage_p99: histogram_percentile(&self.hp_damage_histogram, 0.99) as u32,
            avg_first_attack_seconds: (self.fights_with_attack > 0)
                .then_some(self.first_attack_seconds_total as f32 / self.fights_with_attack as f32),
            avg_attack_interval_seconds: (self.attack_intervals > 0).then_some(
                self.attack_interval_seconds_total as f32 / self.attack_intervals as f32,
            ),
            incoming_raw_damage_per_fight: rate(self.total_incoming_raw_damage, u64::from(runs)),
            armor_prevented_per_fight: rate(self.total_armor_prevented, u64::from(runs)),
            shield_prevented_per_fight: rate(self.total_shield_prevented, u64::from(runs)),
            hp_damage_taken_per_fight: rate(self.total_hp_damage_taken, u64::from(runs)),
            damage_prevention_rate: rate(prevented, self.total_incoming_raw_damage),
            fights_with_trauma_inflicted: self.fights_with_trauma_inflicted,
            trauma_events_inflicted: self.trauma_events_inflicted,
            trauma_chance_per_fight: rate(
                u64::from(self.fights_with_trauma_inflicted),
                u64::from(runs),
            ),
            avg_trauma_downtime_seconds: rate(self.total_trauma_downtime_seconds, u64::from(runs)),
            shield_fights: self.shield_fights,
            fights_with_shield_break: self.fights_with_shield_break,
            shield_break_rate: (self.shield_fights > 0).then_some(rate(
                u64::from(self.fights_with_shield_break),
                u64::from(self.shield_fights),
            )),
            avg_hits_shield_survived_before_break: (self.shield_breaks > 0).then_some(rate(
                self.total_hits_shield_survived_before_break,
                self.shield_breaks,
            )),
            median_hits_shield_survived_before_break: (self.shield_breaks > 0).then_some(
                histogram_percentile(&self.shield_hits_before_break_histogram, 0.50) as u32,
            ),
            avg_shield_break_seconds: (self.shield_breaks > 0)
                .then_some(rate(self.shield_break_seconds_total, self.shield_breaks)),
            median_shield_break_seconds: (self.shield_breaks > 0)
                .then_some(histogram_percentile(&self.shield_break_time_histogram, 0.50) as u32),
        }
    }
}

#[derive(Clone, Debug, Default)]
#[allow(dead_code)]
pub struct BulkSimResult {
    pub wins: Vec<u32>,
    pub ties: u32,
    pub shields_present: bool,
    pub shield_breaks: u32,
    pub avg_hits_shield_survived: f32,
    pub avg_duration: f32,
    pub shortest_duration: u32,
    pub longest_duration: u32,
    pub fights_with_second_charge: u32,
    pub fights_with_trauma: u32,
    pub fights_with_trauma_first_exchange: u32,
    pub fights_with_knockback_20ft: u32,
    pub fights_with_charge_within_20ft: u32,
    pub instakills: u32,
    pub instakills_by_team: Vec<u32>,
    pub highest_single_crit_hit: i32,
    pub highest_single_noncrit_hit: i32,
    pub highest_single_shield_hit: i32,
    pub highest_single_crit_hit_by_team: Vec<i32>,
    pub highest_single_noncrit_hit_by_team: Vec<i32>,
    pub highest_single_shield_hit_by_team: Vec<i32>,
    pub avg_damage_dealt_by_team: Vec<f32>,
    pub avg_damage_taken_by_team: Vec<f32>,
    pub avg_damage_rolled_by_team: Vec<f32>,
    pub avg_damage_landed_by_team: Vec<f32>,
    pub avg_remaining_hp_by_team: Vec<f32>,
    pub max_total_knockback_one_side_ft: f32,
    pub avg_max_knockback_one_side_ft: f32,
    pub detailed: DetailedSimStats,
}

#[allow(dead_code)]
pub fn bulk_simulate(
    config: SimConfig,
    combatants: Vec<Combatant>,
    runs: u32,
    max_seconds: u32,
) -> BulkSimResult {
    bulk_simulate_with_seed(config, combatants, runs, max_seconds, 1)
}

#[allow(dead_code)]
pub fn bulk_simulate_with_seed(
    config: SimConfig,
    combatants: Vec<Combatant>,
    runs: u32,
    max_seconds: u32,
    seed: u64,
) -> BulkSimResult {
    if runs == 0 {
        return BulkSimResult::default();
    }
    let shields_present = combatants
        .iter()
        .any(|combatant| combatant.sheet.defense.shield_name.is_some());
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
    let team_has_shield: Vec<bool> = team_ids
        .iter()
        .map(|team_id| {
            combatants.iter().any(|combatant| {
                combatant.team_id == *team_id && combatant.sheet.defense.shield_name.is_some()
            })
        })
        .collect();
    let team_has_eyesmite: Vec<bool> = team_ids
        .iter()
        .map(|team_id| {
            combatants
                .iter()
                .any(|combatant| combatant.team_id == *team_id && combatant.sheet.defense.eyesmite)
        })
        .collect();
    let mut sim = SimState::with_rng(config, SimRng::from_seed(seed));
    sim.log_events = false;
    sim.reset_with_combatants(combatants);
    let mut wins = vec![0u32; team_ids.len()];
    let mut ties = 0u32;
    let mut fights_with_second_charge = 0u32;
    let mut fights_with_trauma = 0u32;
    let mut fights_with_trauma_first_exchange = 0u32;
    let mut fights_with_knockback_20ft = 0u32;
    let mut fights_with_charge_within_20ft = 0u32;
    let mut instakills = 0u32;
    let mut shortest_duration = u32::MAX;
    let mut longest_duration = 0u32;
    let mut highest_single_crit_hit = 0i32;
    let mut highest_single_noncrit_hit = 0i32;
    let mut highest_single_shield_hit = 0i32;
    let mut highest_single_crit_hit_by_team = vec![0i32; team_ids.len()];
    let mut highest_single_noncrit_hit_by_team = vec![0i32; team_ids.len()];
    let mut highest_single_shield_hit_by_team = vec![0i32; team_ids.len()];
    let mut instakills_by_team = vec![0u32; team_ids.len()];
    let mut total_damage_dealt_by_team = vec![0u64; team_ids.len()];
    let mut total_damage_taken_by_team = vec![0u64; team_ids.len()];
    let mut total_damage_rolled_dealt_by_team = vec![0i64; team_ids.len()];
    let mut total_damage_landed_dealt_by_team = vec![0i64; team_ids.len()];
    let mut damage_rolls_dealt_by_team = vec![0u64; team_ids.len()];
    let mut total_remaining_hp_by_team = vec![0u64; team_ids.len()];
    let mut max_total_knockback_one_side_ft = 0.0f32;
    let mut total_max_knockback_one_side_ft = 0.0f32;
    let mut shield_breaks = 0u32;
    let mut total_hits_shield_survived = 0u64;
    let mut total_seconds = 0u64;
    let mut duration_histogram = BTreeMap::new();
    let mut detailed_accumulators: Vec<DetailedTeamAccumulator> = team_ids
        .iter()
        .enumerate()
        .map(|(idx, team_id)| DetailedTeamAccumulator {
            team_id: *team_id,
            shield_fights: if team_has_shield[idx] { runs } else { 0 },
            eyesmite_available: team_has_eyesmite[idx],
            ..DetailedTeamAccumulator::default()
        })
        .collect();
    for _ in 0..runs {
        sim.reset_preserve_rng();
        while !sim.done && sim.elapsed_seconds < max_seconds {
            sim.update(1.0);
        }
        let duration = sim.elapsed_seconds;
        add_histogram_sample(&mut duration_histogram, u64::from(duration));
        total_seconds += duration as u64;
        shortest_duration = shortest_duration.min(duration);
        longest_duration = longest_duration.max(duration);
        let mut winning_team_idx = None;
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
                        detailed_accumulators[idx].wins += 1;
                        winning_team_idx = Some(idx);
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
        let mut first_attack_by_team = vec![None::<u32>; team_ids.len()];
        let mut last_attack_by_combatant = vec![None::<u32>; sim.combatants.len()];
        let mut trauma_inflicted_by_team = vec![false; team_ids.len()];
        for sample in &sim.attack_metrics {
            let Some(attacker) = sim.combatants.get(sample.attacker_idx) else {
                continue;
            };
            let Some(defender) = sim.combatants.get(sample.defender_idx) else {
                continue;
            };
            let Some(&attacker_team_idx) = team_index.get(&attacker.team_id) else {
                continue;
            };
            let Some(&defender_team_idx) = team_index.get(&defender.team_id) else {
                continue;
            };
            let attacker_stats = &mut detailed_accumulators[attacker_team_idx];
            attacker_stats.attack_attempts = attacker_stats.attack_attempts.saturating_add(1);
            if sample.direct_hit {
                attacker_stats.direct_hits = attacker_stats.direct_hits.saturating_add(1);
            }
            if sample.shield_block {
                attacker_stats.shield_blocks = attacker_stats.shield_blocks.saturating_add(1);
            }
            if sample.hp_damage > 0 {
                attacker_stats.hp_hits = attacker_stats.hp_hits.saturating_add(1);
                add_histogram_sample(
                    &mut attacker_stats.hp_damage_histogram,
                    u64::from(sample.hp_damage),
                );
            }
            if sample.critical {
                attacker_stats.critical_hits = attacker_stats.critical_hits.saturating_add(1);
            }
            if sample.killing_blow {
                attacker_stats.kills = attacker_stats.kills.saturating_add(1);
            }
            attacker_stats.total_hp_damage = attacker_stats
                .total_hp_damage
                .saturating_add(u64::from(sample.hp_damage));
            if sample.trauma_applied {
                attacker_stats.trauma_events_inflicted =
                    attacker_stats.trauma_events_inflicted.saturating_add(1);
                trauma_inflicted_by_team[attacker_team_idx] = true;
            }
            first_attack_by_team[attacker_team_idx] = Some(
                first_attack_by_team[attacker_team_idx]
                    .map_or(sample.time, |current| current.min(sample.time)),
            );
            if let Some(last_time) = last_attack_by_combatant[sample.attacker_idx] {
                attacker_stats.attack_interval_seconds_total = attacker_stats
                    .attack_interval_seconds_total
                    .saturating_add(u64::from(sample.time.saturating_sub(last_time)));
                attacker_stats.attack_intervals = attacker_stats.attack_intervals.saturating_add(1);
            }
            last_attack_by_combatant[sample.attacker_idx] = Some(sample.time);

            let defender_stats = &mut detailed_accumulators[defender_team_idx];
            defender_stats.total_incoming_raw_damage = defender_stats
                .total_incoming_raw_damage
                .saturating_add(u64::from(sample.raw_damage));
            defender_stats.total_armor_prevented = defender_stats
                .total_armor_prevented
                .saturating_add(u64::from(sample.armor_prevented));
            defender_stats.total_shield_prevented = defender_stats
                .total_shield_prevented
                .saturating_add(u64::from(sample.shield_prevented));
            defender_stats.total_hp_damage_taken = defender_stats
                .total_hp_damage_taken
                .saturating_add(u64::from(sample.hp_damage));
            if sample.shield_broken {
                defender_stats.shield_breaks = defender_stats.shield_breaks.saturating_add(1);
                defender_stats.total_hits_shield_survived_before_break = defender_stats
                    .total_hits_shield_survived_before_break
                    .saturating_add(u64::from(sample.shield_hits_survived_before_break));
                add_histogram_sample(
                    &mut defender_stats.shield_hits_before_break_histogram,
                    u64::from(sample.shield_hits_survived_before_break),
                );
                defender_stats.shield_break_seconds_total = defender_stats
                    .shield_break_seconds_total
                    .saturating_add(u64::from(sample.time));
                add_histogram_sample(
                    &mut defender_stats.shield_break_time_histogram,
                    u64::from(sample.time),
                );
            }
        }
        for team_idx in 0..team_ids.len() {
            if let Some(first_attack) = first_attack_by_team[team_idx] {
                detailed_accumulators[team_idx].first_attack_seconds_total = detailed_accumulators
                    [team_idx]
                    .first_attack_seconds_total
                    .saturating_add(u64::from(first_attack));
                detailed_accumulators[team_idx].fights_with_attack = detailed_accumulators
                    [team_idx]
                    .fights_with_attack
                    .saturating_add(1);
            }
            if trauma_inflicted_by_team[team_idx] {
                detailed_accumulators[team_idx].fights_with_trauma_inflicted =
                    detailed_accumulators[team_idx]
                        .fights_with_trauma_inflicted
                        .saturating_add(1);
            }
        }
        let mut fight_max_knockback_side = 0.0f32;
        let mut remaining_hp_by_team = vec![0u64; team_ids.len()];
        let mut shield_broke_by_team = vec![false; team_ids.len()];
        for combatant in &sim.combatants {
            let Some(&team_idx) = team_index.get(&combatant.team_id) else {
                continue;
            };
            let state = &combatant.state;
            highest_single_crit_hit = highest_single_crit_hit.max(state.max_crit_hit_dealt);
            highest_single_noncrit_hit =
                highest_single_noncrit_hit.max(state.max_noncrit_hit_dealt);
            highest_single_shield_hit = highest_single_shield_hit.max(state.max_shield_hit_dealt);
            highest_single_crit_hit_by_team[team_idx] =
                highest_single_crit_hit_by_team[team_idx].max(state.max_crit_hit_dealt);
            highest_single_noncrit_hit_by_team[team_idx] =
                highest_single_noncrit_hit_by_team[team_idx].max(state.max_noncrit_hit_dealt);
            highest_single_shield_hit_by_team[team_idx] =
                highest_single_shield_hit_by_team[team_idx].max(state.max_shield_hit_dealt);
            instakills = instakills.saturating_add(state.total_instakills_dealt);
            instakills_by_team[team_idx] =
                instakills_by_team[team_idx].saturating_add(state.total_instakills_dealt);
            detailed_accumulators[team_idx].eyes_smote = detailed_accumulators[team_idx]
                .eyes_smote
                .saturating_add(u64::from(state.total_eyes_smote));
            shield_breaks = shield_breaks.saturating_add(state.total_shield_breaks_taken);
            total_hits_shield_survived = total_hits_shield_survived
                .saturating_add(u64::from(state.total_shield_hits_survived_before_break));
            total_damage_dealt_by_team[team_idx] = total_damage_dealt_by_team[team_idx]
                .saturating_add(u64::from(state.total_hp_damage_dealt));
            total_damage_taken_by_team[team_idx] = total_damage_taken_by_team[team_idx]
                .saturating_add(u64::from(state.total_hp_damage_taken));
            total_damage_rolled_dealt_by_team[team_idx] += state.total_damage_rolled_dealt;
            total_damage_landed_dealt_by_team[team_idx] += state.total_damage_landed_dealt;
            damage_rolls_dealt_by_team[team_idx] = damage_rolls_dealt_by_team[team_idx]
                .saturating_add(u64::from(state.damage_rolls_dealt));
            total_remaining_hp_by_team[team_idx] =
                total_remaining_hp_by_team[team_idx].saturating_add(state.hp.max(0) as u64);
            remaining_hp_by_team[team_idx] =
                remaining_hp_by_team[team_idx].saturating_add(state.hp.max(0) as u64);
            detailed_accumulators[team_idx].total_trauma_downtime_seconds = detailed_accumulators
                [team_idx]
                .total_trauma_downtime_seconds
                .saturating_add(u64::from(state.total_trauma_seconds_suffered));
            if state.total_shield_breaks_taken > 0 {
                shield_broke_by_team[team_idx] = true;
            }
            fight_max_knockback_side = fight_max_knockback_side.max(state.total_knockback_taken_ft);
        }
        for (team_idx, shield_broke) in shield_broke_by_team.into_iter().enumerate() {
            if shield_broke {
                detailed_accumulators[team_idx].fights_with_shield_break = detailed_accumulators
                    [team_idx]
                    .fights_with_shield_break
                    .saturating_add(1);
            }
        }
        if let Some(team_idx) = winning_team_idx {
            let remaining_hp = remaining_hp_by_team[team_idx];
            detailed_accumulators[team_idx].winning_hp_total = detailed_accumulators[team_idx]
                .winning_hp_total
                .saturating_add(remaining_hp);
            add_histogram_sample(
                &mut detailed_accumulators[team_idx].winning_hp_histogram,
                remaining_hp,
            );
            detailed_accumulators[team_idx].winning_duration_seconds_total = detailed_accumulators
                [team_idx]
                .winning_duration_seconds_total
                .saturating_add(u64::from(duration));
            add_histogram_sample(
                &mut detailed_accumulators[team_idx].winning_duration_seconds_histogram,
                u64::from(duration),
            );
        }
        max_total_knockback_one_side_ft =
            max_total_knockback_one_side_ft.max(fight_max_knockback_side);
        total_max_knockback_one_side_ft += fight_max_knockback_side;
    }
    let avg_duration = total_seconds as f32 / runs as f32;
    let avg_damage_dealt_by_team = total_damage_dealt_by_team
        .iter()
        .copied()
        .map(|value| value as f32 / runs as f32)
        .collect();
    let avg_damage_taken_by_team = total_damage_taken_by_team
        .into_iter()
        .map(|value| value as f32 / runs as f32)
        .collect();
    let avg_damage_landed_by_team = total_damage_landed_dealt_by_team
        .into_iter()
        .zip(damage_rolls_dealt_by_team.iter().copied())
        .map(|(total, count)| {
            if count > 0 {
                total as f32 / count as f32
            } else {
                0.0
            }
        })
        .collect();
    let avg_damage_rolled_by_team = total_damage_rolled_dealt_by_team
        .into_iter()
        .zip(damage_rolls_dealt_by_team)
        .map(|(total, count)| {
            if count > 0 {
                total as f32 / count as f32
            } else {
                0.0
            }
        })
        .collect();
    let avg_remaining_hp_by_team = total_remaining_hp_by_team
        .into_iter()
        .map(|value| value as f32 / runs as f32)
        .collect();
    let avg_hits_shield_survived = if shield_breaks > 0 {
        total_hits_shield_survived as f32 / shield_breaks as f32
    } else {
        0.0
    };
    let detailed = DetailedSimStats {
        duration_p10: histogram_percentile(&duration_histogram, 0.10) as u32,
        duration_p50: histogram_percentile(&duration_histogram, 0.50) as u32,
        duration_p90: histogram_percentile(&duration_histogram, 0.90) as u32,
        duration_p99: histogram_percentile(&duration_histogram, 0.99) as u32,
        teams: detailed_accumulators
            .into_iter()
            .map(|accumulator| accumulator.finish(runs, total_seconds))
            .collect(),
    };
    BulkSimResult {
        wins,
        ties,
        shields_present,
        shield_breaks,
        avg_hits_shield_survived,
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
        instakills,
        instakills_by_team,
        highest_single_crit_hit,
        highest_single_noncrit_hit,
        highest_single_shield_hit,
        highest_single_crit_hit_by_team,
        highest_single_noncrit_hit_by_team,
        highest_single_shield_hit_by_team,
        avg_damage_dealt_by_team,
        avg_damage_taken_by_team,
        avg_damage_rolled_by_team,
        avg_damage_landed_by_team,
        avg_remaining_hp_by_team,
        max_total_knockback_one_side_ft,
        avg_max_knockback_one_side_ft: total_max_knockback_one_side_ft / runs as f32,
        detailed,
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
