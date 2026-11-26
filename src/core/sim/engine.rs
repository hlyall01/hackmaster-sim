#[cfg(test)]
use crate::core::tactics::{evaluate_channel, TacticalCondition};
use super::damage_sources::{DamageSource, DamageSourceStats, ImpactSource};
use crate::core::rng::SimRng;
use crate::core::rules::roll_damage_expr;
use crate::core::tactics::{
    TacticalAction, TacticalChannel, TacticalContext,
    TacticalDecision,
};
use tactical_context::ChannelPlan;
#[path = "tactical_context.rs"]
mod tactical_context;
use rand::RngCore;

use super::combat::{
    AttackMode, AttackOutcome, CounterAttackOutcome, resolve_attack, resolve_knock_aside,
};
use super::modifiers::{StatIdF32, StatIdI32};
use super::movement::{max_range_for_weapon, range_modifier_for_weapon_with_scale};
use super::types::{
    AttackEvent, CalledShotDelayProfile, CombatEvent, CombatEventKind, Combatant,
    GridPos, KnockAsideEvent, SimActor, SimConfig, TacticalEvent,
    WeaponProfile, WeaponSlot,
};
use std::collections::{BTreeMap, HashMap};

#[path = "spell_effect_engine.rs"]
mod spell_effect_engine;
#[path = "weapon_setup_engine.rs"]
mod weapon_setup_engine;

#[path = "bulk_metrics.rs"]
mod bulk_metrics;
use bulk_metrics::{BulkMetrics, DenseHistogram};
use super::intimidation::IntimidationPlan;

#[path = "bulk_parallel.rs"]
mod bulk_parallel;
pub use bulk_parallel::{BulkSimBatch, bulk_simulate_batch};

const CHARGE_MIN_DISTANCE_FT: f32 = 20.0;
const SHIELD_STRIKE_SPEEDUP_SECONDS: f32 = 2.0;
const SIX_PATHS_FOLLOWUP_SECONDS: f32 = 1.0;

fn unarmed_opponent(defender: &Combatant) -> bool {
    let armed_weapon = |weapon: &WeaponProfile| {
        weapon.has_weapon && !weapon.is_unarmed
    };
    !armed_weapon(&defender.sheet.offense.weapon)
        && !defender.sheet.offense.offhand.as_ref()
            .is_some_and(|offhand| armed_weapon(&offhand.weapon))
}

fn faster_against_unarmed(weapon: &WeaponProfile, defender: &Combatant, is_ranged: bool) -> bool {
    !is_ranged && weapon.has_weapon && !weapon.is_unarmed && unarmed_opponent(defender)
}

fn recovery_against_opponent(
    speed: f32, weapon: &WeaponProfile, defender: &Combatant, is_ranged: bool,
) -> f32 {
    if faster_against_unarmed(weapon, defender, is_ranged) {
        // Quiet River remains unarmed. Vulnerability speed bonuses do not stack.
        (speed / 2.0).ceil().max(1.0)
    } else if defender.state.trauma_remaining_seconds > 0 {
        (speed / 2.0).ceil().max(1.0)
    } else {
        speed
    }
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
    use crate::core::rules::DamageExprCache;
    use std::sync::LazyLock;

    static D4: LazyLock<DamageExprCache> = LazyLock::new(|| DamageExprCache::new("1d4p"));
    static TWO_D4: LazyLock<DamageExprCache> = LazyLock::new(|| DamageExprCache::new("2d4p"));
    static FOUR_D4: LazyLock<DamageExprCache> = LazyLock::new(|| DamageExprCache::new("4d4p"));
    static D2: LazyLock<DamageExprCache> = LazyLock::new(|| DamageExprCache::new("1d2"));

    let dice = if defender.sheet.maneuvers.called_shot_deceptive_defender {
        &FOUR_D4
    } else {
        match attacker.sheet.maneuvers.called_shot_delay_profile {
            CalledShotDelayProfile::Standard if !is_ranged => &TWO_D4,
            CalledShotDelayProfile::Standard | CalledShotDelayProfile::PrecisionCombatant => &D4,
            CalledShotDelayProfile::PrecisionAiming => &D2,
        }
    };
    dice.roll(rng, false).max(0) as f32
}

#[derive(Clone, Debug)]
enum PreviousPositions {
    Duel([GridPos; 2]),
    Roster(Vec<GridPos>),
}

impl Default for PreviousPositions {
    fn default() -> Self { Self::Roster(Vec::new()) }
}

impl PreviousPositions {
    fn capture(&mut self, actors: &[SimActor]) {
        if let [a, b] = actors {
            *self = Self::Duel([a.position, b.position]);
        } else {
            if !matches!(self, Self::Roster(_)) { *self = Self::default(); }
            if let Self::Roster(v) = self {
                v.clear();
                v.extend(actors.iter().map(|a| a.position));
            }
        }
    }

    fn as_slice(&self) -> &[GridPos] {
        match self { Self::Duel(pair) => pair, Self::Roster(v) => v }
    }
}

#[derive(Clone, Copy)]
enum EscapeDestination {
    Known(Option<GridPos>),
    Duel(super::movement::DuelEscape),
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
    pub collect_detailed_metrics: bool,
    pub first_attack_time: Option<u32>,
    pub trauma_first_exchange: bool,
    pub charges_started_within_20ft: u32,
    attack_metrics: Vec<AttackMetricSample>,
    bulk_metrics: Option<Box<BulkMetrics>>,
    rng: SimRng,
    tick_accum: f32,
    hold_at_bay: HoldAtBayState,
    initial_positions: Option<std::sync::Arc<[SimActor]>>,
    active_field_owners: Vec<usize>,
    fist_targets: Vec<bool>,
    streamline_inputs: Vec<(GridPos, Option<u32>)>,
    streamline_tile_size: u32,
    pub(super) duel_in_intimidation_range: Option<bool>,
    previous_positions: PreviousPositions,
    predicting_movement: bool,
    // Bulk jobs own a fixed roster and policies for the life of the batch.
    bulk: Option<std::sync::Arc<BulkContext>>,
}

#[derive(Clone, Debug)]
struct BulkContext {
    prepared: Vec<std::sync::Arc<super::prepared_fighter::PreparedFighter>>,
    tactics: Vec<[ChannelPlan; 5]>,
    fixed_dr_called_shot: Vec<Option<bool>>,
    fixed_move_tiles: Vec<Option<i32>>,
    moving_reaction: Vec<bool>,
    spell_plans: Vec<super::magic::PreparedSpellPolicies>,
    has_weapon_setups: bool,
    charge_indices: Vec<usize>,
    spell_free: bool,
    simple_recovery: bool,
    intimidation: IntimidationPlan,
    may_average_damage: bool,
    possible_sheet_flags: u64,
    has_running_bow: bool,
    has_precognition: bool,
}

impl BulkContext {
    #[cfg(test)]
    fn new(combatants: &[Combatant]) -> Self {
        Self::for_grid(combatants, SimConfig::new(0.0, 0.0).tile_size_ft)
    }

    fn for_grid(combatants: &[Combatant], tile_size: f32) -> Self {
        let prepared = combatants.iter().enumerate().map(|(i, actor)| {
            let start = actor.weapon_setups.as_ref().map_or(actor, |s| match s.start_with {
                super::WeaponSetup::Melee => &s.melee, super::WeaponSetup::Ranged => &s.ranged,
            });
            let adjustment = if combatants.len() == 2 {
                combatants[1-i].sheet.offense.weapon.defender_knockback_step_adjustment
            } else { 0 };
            super::prepared_fighter::PreparedFighter::new(&start.sheet, adjustment).into()
        }).collect();
        let tactics: Vec<_> = combatants.iter().map(|c|
            TacticalChannel::ALL.map(|channel| ChannelPlan::new(&c.tactical_policy, channel))
        ).collect();
        // Equipment changes can replace both the loadout and available style sheets.
        // Only a private bulk batch has a closed roster. Interactive spells and edits
        // always take the normal refresh path.
        let has_weapon_setups = combatants.iter().any(|c| c.weapon_setups.is_some());
        let mut charge_indices = Vec::new();
        let mut spell_free = true;
        let mut intimidation = IntimidationPlan::new(combatants);
        let mut may_average_damage = false;
        let mut possible_sheet_flags = 0;
        let mut has_running_bow = false;
        let mut has_precognition = false;
        for (idx, actor) in combatants.iter().enumerate() {
            let alternatives = actor.weapon_setups.iter()
                .flat_map(|s| [&s.melee, &s.ranged, &s.drawing]);
            let mut can_charge = false;
            for setup in std::iter::once(actor).chain(alternatives) {
                let loadout = &setup.magic.loadout;
                spell_free &= loadout.auto_cast == crate::core::magic::AutoCast::SpellPolicies
                    && !loadout.knows_echo_strike && loadout.known_spells.is_empty();
                let sheets = std::iter::once(&setup.sheet)
                    .chain(setup.tactical_profiles.iter().map(|p| &p.sheet));
                for sheet in sheets {
                    can_charge |= sheet.maneuvers.charge;
                    intimidation.include_profile(idx, sheet);
                    possible_sheet_flags |= sheet.modifiers.positive_bits();
                    has_running_bow |= sheet.offense.weapon.ranged_styles.whistling_wind_pace == Some(super::BowPace::Run);
                    has_precognition |= sheet.defense.precognition;
                    may_average_damage |= sheet.modifiers.apply_i32(0, StatIdI32::FlagStreamlineSpell) > 0;
                }
                may_average_damage |= crate::core::magic::spell_catalog().iter().any(|spell|
                    setup.magic.loadout.knows_spell(&spell.id) && spell.mechanics.as_ref().is_some_and(|m|
                        matches!(m.effect, crate::core::magic::EffectRule::TimedBuff { average_incoming_damage: true, .. })
                    )
                );
            }
            if can_charge { charge_indices.push(idx); }
        }
        let simple_recovery = combatants.len() == 2 && spell_free && !has_weapon_setups
            && combatants.iter().all(|c| !c.tactical_policy.enabled && !c.arborite.warrior_woad
                && !c.sheet.maneuvers.passive && !c.sheet.maneuvers.offensive_dualwielding);
        let spell_plans = combatants.iter().map(|c| {
            let start = c.weapon_setups.as_ref().map_or(c, |sets| match sets.start_with {
                super::WeaponSetup::Melee => &sets.melee, super::WeaponSetup::Ranged => &sets.ranged,
            });
            super::magic::PreparedSpellPolicies::new(&start.magic.loadout)
        }).collect();
        let fixed_dr_called_shot = combatants.iter().enumerate().map(|(idx, actor)| {
            if combatants.len() != 2 { return None; }
            let (rule, tactical_context::SingleCondition::EnemyDr(compare, threshold)) =
                tactics[idx][TacticalChannel::AttackMode as usize].single_rule? else { return None };
            if actor.tactical_policy.rules[rule].action != TacticalAction::CalledShot { return None; }
            let enemy = &combatants[1 - idx];
            let options = enemy.weapon_setups.iter().flat_map(|s| [&s.melee, &s.ranged, &s.drawing]);
            let mut results = std::iter::once(enemy).chain(options).flat_map(|c|
                std::iter::once(&c.sheet).chain(c.tactical_profiles.iter().map(|p| &p.sheet))
            ).map(|sheet| {
                // Ideal Self changes the target's DR as it takes wounds and heals.
                if sheet.modifiers.positive_at_zero(StatIdI32::FlagIdealSelf) { return None; }
                Some(compare.matches(sheet.modifiers.apply_i32(sheet.defense.armor_dr, StatIdI32::ArmorDr).max(0) as f32, threshold))
            });
            let first = results.next()??;
            results.all(|value| value == Some(first)).then_some(first)
        }).collect();
        let moving_reaction = combatants.iter().enumerate().map(|(idx, c)|
            tactics[idx][TacticalChannel::Reaction as usize].rules.iter().any(|&i|
                matches!(c.tactical_policy.rules[i].action, TacticalAction::GiveGround | TacticalAction::ScamperBack))
        ).collect();
        let fixed_move_tiles = combatants.iter().map(|actor| {
            let setups = actor.weapon_setups.iter().flat_map(|s| [&s.melee, &s.ranged, &s.drawing]);
            let mut speeds = std::iter::once(actor).chain(setups).flat_map(|c|
                std::iter::once(&c.sheet).chain(c.tactical_profiles.iter().map(|p| &p.sheet))
            ).map(|s| {
                if s.offense.weapon.uses_projectiles { return None; }
                let speed = s.modifiers.apply_f32(s.mobility.move_speed, StatIdF32::MoveSpeed).max(0.0);
                Some(if speed == 0.0 { 0 } else { ((speed / tile_size.max(0.01)).round() as i32).max(1) })
            });
            let first = speeds.next()??;
            speeds.all(|other| other == Some(first)).then_some(first)
        }).collect();
        Self { prepared, tactics, fixed_dr_called_shot, fixed_move_tiles, moving_reaction, spell_plans, has_weapon_setups, charge_indices, spell_free, simple_recovery, intimidation, may_average_damage,
            possible_sheet_flags, has_running_bow, has_precognition }
    }
}

#[derive(Clone, Debug)]
struct AttackMetricSample {
    damage_source: ImpactSource,
    time: u32,
    attacker_idx: usize,
    defender_idx: usize,
    direct_hit: bool,
    shield_block: bool,
    hp_damage: u32,
    knockback_ft: f32,
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
    damage_source: ImpactSource,
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
        Self {
            damage_source: event.damage_source.clone(),
            attacker_idx: event.attacker_idx,
            defender_idx: event.defender_idx,
            hp_damage: event.damage,
            damage_rolled: event.impact.direct.map(|(rolled, _)| rolled),
            damage_landed: event.impact.direct.map(|(_, landed)| landed),
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
            shield_broken: event.impact.shield_broken,
            shield_damage: event.shield_damage,
            knockback_ft: event.knockback_ft,
            attempted: event.roll.attack_die > 0,
            direct_hit: event.hit,
            critical: event.critical.is_some(),
            trauma_applied: event.trauma_applied,
            defender_hp_after: event.defender_hp_after,
            armor_prevented: event.impact.armor_prevented,
            shield_prevented: event.impact.shield_prevented,
        }
    }

    fn from_counter(event: &CounterAttackOutcome) -> Self {
        Self {
            damage_source: event.damage_source.clone(),
            attacker_idx: event.attacker_idx,
            defender_idx: event.defender_idx,
            hp_damage: event.damage,
            damage_rolled: event.impact.direct.map(|(rolled, _)| rolled),
            damage_landed: event.impact.direct.map(|(_, landed)| landed),
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
            shield_broken: event.impact.shield_broken,
            shield_damage: event.shield_damage,
            knockback_ft: event.knockback_ft,
            attempted: event.roll.attack_die > 0,
            direct_hit: event.hit,
            critical: event.critical.is_some(),
            trauma_applied: event.trauma_applied,
            defender_hp_after: event.defender_hp_after,
            armor_prevented: event.impact.armor_prevented,
            shield_prevented: event.impact.shield_prevented,
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
            collect_detailed_metrics: false,
            first_attack_time: None,
            trauma_first_exchange: false,
            charges_started_within_20ft: 0,
            attack_metrics: Vec::new(),
            bulk_metrics: None,
            rng,
            tick_accum: 0.0,
            hold_at_bay: HoldAtBayState::default(),
            initial_positions: None,
            active_field_owners: Vec::new(),
            fist_targets: Vec::new(),
            streamline_inputs: Vec::new(),
            streamline_tile_size: 0,
            duel_in_intimidation_range: None,
            previous_positions: PreviousPositions::default(),
            predicting_movement: false,
            bulk: None,
        }
    }

    pub(super) fn intimidation_plan(&self) -> IntimidationPlan {
        self.bulk.as_ref().map_or(IntimidationPlan::Live, |b| b.intimidation)
    }

    pub fn reset(&mut self) {
        if let Some(positions) = &self.initial_positions {
            self.actors.clear();
            self.actors.extend_from_slice(positions);
        } else {
            self.actors = self.spawn_positions();
        }
        self.elapsed_seconds = 0;
        self.done = false;
        self.last_event = None;
        self.combat_events.clear();
        self.first_attack_time = None;
        self.trauma_first_exchange = false;
        self.charges_started_within_20ft = 0;
        self.attack_metrics.clear();
        self.active_field_owners.clear();
        self.streamline_inputs.clear();
        self.duel_in_intimidation_range = None;
        for (index, combatant) in self.combatants.iter_mut().enumerate() {
            combatant.reset_state();
            if let Some(bulk) = &self.bulk {
                combatant.state.magic.quiet = true;
                combatant.install_spell_plan(bulk.spell_plans[index].clone());
            }
        }
        self.refresh_intimidation();
        self.tick_accum = 0.0;
        self.hold_at_bay = HoldAtBayState::default();
    }

    #[allow(dead_code)]
    pub fn reset_preserve_rng(&mut self) {
        self.reset();
    }

    fn spawn_positions(&self) -> Vec<SimActor> {
        Self::layout(self.config, &self.combatants)
    }

    fn layout(config: SimConfig, combatants: &[Combatant]) -> Vec<SimActor> {
        let count = combatants.len();
        if count == 0 {
            return Vec::new();
        }
        let grid_width = config.grid_width.max(1);
        let grid_height = config.grid_height.max(1);
        let center_x = grid_width / 2;
        let center_y = grid_height / 2;
        let tile_size_ft = config.tile_size_ft.max(0.01);
        let start_tiles = (config.start_distance / tile_size_ft).ceil() as i32;

        // GUI duels have one actor per team; their placement needs no team maps.
        if count == 2 && combatants[0].team_id != combatants[1].team_id {
            let left = ((grid_width - 1 - start_tiles) / 2).max(0);
            let right = (left + start_tiles).min(grid_width - 1);
            let mut positions = [left, right];
            if combatants[0].team_id > combatants[1].team_id {
                positions.swap(0, 1);
            }
            return positions
                .into_iter()
                .map(|x| SimActor {
                    position: GridPos::new(x, center_y).clamp(grid_width, grid_height),
                })
                .collect();
        }

        let mut teams: Vec<u8> = combatants.iter().map(|c| c.team_id).collect();
        teams.sort_unstable();
        teams.dedup();
        if teams.is_empty() {
            teams.push(0);
        }

        let mut team_sizes: HashMap<u8, usize> = HashMap::new();
        for combatant in combatants {
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
        for combatant in combatants {
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

    #[inline(always)]
    pub(crate) fn apply_shield_strike_speedup(&mut self, defender_idx: usize, now: f32) {
        if !self.combatants[defender_idx].combat_flag_active(StatIdI32::FlagLargeSwordShieldStyle) { return; }
        let Some(next_attack) = self.combatants[defender_idx].state.next_attack_time_primary else {
            return;
        };
        let adjusted = (next_attack - SHIELD_STRIKE_SPEEDUP_SECONDS).max(now);
        self.combatants[defender_idx]
            .state
            .set_next_attack_time(WeaponSlot::Primary, Some(adjusted));
    }

    #[inline(always)]
    pub(crate) fn apply_six_paths_followup(&mut self, attacker_idx: usize, now: f32) {
        if !self.combatants[attacker_idx].combat_flag_active(StatIdI32::FlagSixPathsStyle) { return; }
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
            damage_source,
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

        self.combatants[attacker_idx].state.attack_events += 1;
        if attempted && !direct_hit && !shield_block && !critical && !trauma_applied && !instant_kill
            && defender_hp_after > 0 && hp_damage == 0 && shield_damage == 0 && knockback_ft == 0.0
            && damage_rolled.is_none() && damage_landed.is_none() && !shield_broken
            && armor_prevented == 0 && shield_prevented == 0
        {
            if let Some(bulk) = &mut self.bulk_metrics {
                if self.collect_detailed_metrics { bulk.record_attempt(attacker_idx, damage_source, self.elapsed_seconds, 0); }
                return;
            }
        }
        if attempted && self.collect_detailed_metrics {
            let shield_hits_survived_before_break = if shield_broken {
                self.combatants[defender_idx].state.shield_blocks_taken
            } else {
                0
            };
            let sample = AttackMetricSample {
                damage_source,
                time: self.elapsed_seconds,
                attacker_idx,
                defender_idx,
                direct_hit,
                shield_block,
                hp_damage: hp_damage_u32,
                knockback_ft: knockback,
                critical,
                killing_blow: defender_hp_after <= 0,
                raw_damage: damage_rolled.unwrap_or(shield_damage.max(0)).max(0) as u32,
                armor_prevented,
                shield_prevented,
                trauma_applied,
                shield_broken,
                shield_hits_survived_before_break,
            };
            if let Some(bulk) = &mut self.bulk_metrics {
                bulk.record(sample);
            } else {
                self.attack_metrics.push(sample);
            }
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
        self.refresh_intimidation();
        // Only private bulk jobs start every fight with reset magic state and a closed loadout.
        let spell_free = self.bulk.as_ref().is_some_and(|b| b.spell_free);
        if spell_free {
            for c in &mut self.combatants { c.state.magic.now = self.elapsed_seconds; }
        } else if self.predicting_movement {
            for actor in &mut self.combatants { actor.advance_magic(self.elapsed_seconds, &mut self.rng); }
        } else { self.advance_spellcasting(); }
        for combatant in &mut self.combatants {
            combatant.state.knockback_applied_this_tick = false;
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
        let track_streamline = self.bulk.as_ref().is_none_or(|b| b.may_average_damage);
        if track_streamline { self.refresh_streamline_coverage(); }
        let mut old_positions = std::mem::take(&mut self.previous_positions);
        old_positions.capture(&self.actors);
        let positions = old_positions.as_slice();
        let active_pair = self.active_pair();
        if let Some((a_idx, b_idx)) = active_pair {
            if self.bulk.as_ref().is_none_or(|b| b.has_weapon_setups) {
                self.advance_weapon_setups(a_idx, b_idx);
            }
            if (self.hold_at_bay.active || self.hold_at_bay.pending)
                && self.hold_at_bay.holder_idx != a_idx
                && self.hold_at_bay.holder_idx != b_idx
                && self.hold_at_bay.target_idx != a_idx
                && self.hold_at_bay.target_idx != b_idx
            {
                self.hold_at_bay = HoldAtBayState::default();
            }

            let distance_before_combat = self.distance_between(a_idx, b_idx).unwrap_or(0.0);
            let reach_a = self.combatants[a_idx].effective_reach();
            let reach_b = self.combatants[b_idx].effective_reach();
            let max_reach = reach_a.max(reach_b);
            let min_reach = reach_a.min(reach_b);
            let ranged_projectile_a = self.combatants[a_idx].sheet.offense.weapon.uses_projectiles;
            let ranged_projectile_b = self.combatants[b_idx].sheet.offense.weapon.uses_projectiles;
            let max_range_a = {
                let combatant = &mut self.combatants[a_idx];
                max_range_cached(
                    &mut combatant.state,
                    WeaponSlot::Primary,
                    &combatant.sheet.offense.weapon,
                )
            };
            let max_range_b = {
                let combatant = &mut self.combatants[b_idx];
                max_range_cached(
                    &mut combatant.state,
                    WeaponSlot::Primary,
                    &combatant.sheet.offense.weapon,
                )
            };
            let ranged_a = max_range_a.is_some();
            let ranged_b = max_range_b.is_some();
            let ranged_projectile_a = ranged_a && ranged_projectile_a;
            let ranged_projectile_b = ranged_b && ranged_projectile_b;
            let any_ranged = ranged_a || ranged_b;
            let eyesmite_a = self.combatants[a_idx].sheet.defense.eyesmite;
            let eyesmite_b = self.combatants[b_idx].sheet.defense.eyesmite;

            let distance_after_combat;
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
                if self.bulk.is_some() && self.active_field_owners.is_empty() {
                    for c in &mut self.combatants { c.state.clear_attack_timers(); }
                } else {
                    self.fist_targets.clear();
                    self.fist_targets.resize(self.combatants.len(), false);
                    for caster in &self.combatants {
                        if let Some(target) = caster.state.magic.fists.as_ref().and_then(|f| f.spell.target)
                            .and_then(|i| self.fist_targets.get_mut(i)) { *target = true; }
                    }
                    for (c, targeted) in self.combatants.iter_mut().zip(&self.fist_targets) {
                        if !targeted { c.state.clear_attack_timers(); }
                    }
                }
                distance_after_combat = self.distance_between(a_idx, b_idx).unwrap_or(0.0);
            } else {
                if !self.predicting_movement {
                    self.resolve_combat_round_at(a_idx, b_idx, distance_before_combat, reach_a, reach_b);
                }
                let mut distance = self.distance_between(a_idx, b_idx).unwrap_or(0.0);
                if any_ranged {
                    if eyesmite_a && distance > 5.0 && !self.hold_at_bay.blocks_advance(a_idx) {
                        distance = self.move_for_separation(a_idx, b_idx, Some(5.0), distance);
                    }
                    if eyesmite_b && distance > 5.0 && !self.hold_at_bay.blocks_advance(b_idx) {
                        distance = self.move_for_separation(b_idx, a_idx, Some(5.0), distance);
                    }
                    let engaged = distance <= min_reach;
                    if !engaged {
                        // Resolve the archer's movement before pursuit on either side.
                        // Otherwise a pursuer can reach melee and lose contact again
                        // before it gets an attack opportunity at the next tick.
                        for (mover, target, range) in [
                            (a_idx, b_idx, max_range_a.filter(|_| ranged_projectile_a && !eyesmite_a)),
                            (b_idx, a_idx, max_range_b.filter(|_| ranged_projectile_b && !eyesmite_b)),
                        ] {
                            if let Some(range) = range {
                                let stop = (distance > range).then_some(max_reach);
                                distance = self.move_for_separation(mover, target, stop, distance);
                            }
                        }
                        if !eyesmite_a && !ranged_projectile_a && distance > reach_a {
                            distance = self.move_for_separation(a_idx, b_idx, Some(reach_a), distance);
                        }
                        if !eyesmite_b && !ranged_projectile_b && distance > reach_b {
                            distance = self.move_for_separation(b_idx, a_idx, Some(reach_b), distance);
                        }
                    }
                } else if distance > min_reach || (distance > 5.0 && (eyesmite_a || eyesmite_b)) {
                    // Eyesmite changes its user's preferred distance, but must not
                    // prevent the opponent from closing to their own weapon reach.
                    if (eyesmite_a || reach_a < reach_b) && !self.hold_at_bay.blocks_advance(a_idx)
                    {
                        let stop_distance = if eyesmite_a {
                            reach_a.min(5.0)
                        } else {
                            reach_a
                        };
                        distance = self.move_for_separation(a_idx, b_idx, Some(stop_distance), distance);
                    }
                    if (eyesmite_b || reach_b < reach_a) && !self.hold_at_bay.blocks_advance(b_idx)
                    {
                        let stop_distance = if eyesmite_b {
                            reach_b.min(5.0)
                        } else {
                            reach_b
                        };
                        distance = self.move_for_separation(b_idx, a_idx, Some(stop_distance), distance);
                    }
                }
                distance_after_combat = distance;
            }
            if max_range_a.is_some()
                && !ranged_projectile_a
                && distance_before_combat > reach_a
                && distance_after_combat <= reach_a
            {
                self.combatants[a_idx].state.clear_attack_timers();
            }
            if max_range_b.is_some()
                && !ranged_projectile_b
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
        let duel_moved = match self.actors.as_slice() {
            [a, b] => Some([a.position != positions[0], b.position != positions[1]]),
            _ => None,
        };
        self.update_charge_progress(active_pair, positions, duel_moved);
        let track_bow_running = self.bulk.as_ref().is_none_or(|batch| batch.has_running_bow);
        for (idx, combatant) in self.combatants.iter_mut().enumerate() {
            let moved = duel_moved.map_or_else(
                || self.actors[idx].position != positions[idx], |m| m[idx]);
            if track_bow_running {
                let running = moved && super::ranged_styles::bow_running(combatant);
                combatant.state.whistling_wind_running = running;
                combatant.state.whistling_wind_next_attack_penalty |= running;
            }
            combatant.state.moved_last_tick = moved;
        }
        self.previous_positions = old_positions;
        self.refresh_intimidation();
        for combatant in &mut self.combatants {
            if combatant.state.active_effects.capabilities().warrior_woad {
                combatant.tick_racial_effects(&mut self.rng);
            } else if combatant.state.arborite.unconscious_seconds > 0 {
                combatant.state.arborite.unconscious_seconds -= 1;
            }
            if !combatant.state.active_effects.is_empty() {
                combatant.state.tick_effects();
            }
        }
        if track_streamline { self.refresh_streamline_coverage(); }
        self.elapsed_seconds += 1;
        if self.bulk.is_none() {
            for combatant in &mut self.combatants {
                combatant
                    .state
                    .refresh_defense_plus_four_ready(&combatant.sheet);
            }
        }
        if !spell_free { self.flush_spell_events(); }
        self.done =
            !self.multiple_teams_remain() && !super::magic::has_pending_echoes(&self.combatants)
                && !self.combatants.iter().any(|c| !c.state.magic.charged_objects.is_empty() || !c.state.magic.sweeping_fields.is_empty());
    }

    pub fn cast_spell(
        &mut self,
        caster: usize,
        mut request: super::SpellRequest,
    ) -> Result<(), crate::core::magic::MagicError> {
        if self.done {
            return Err(crate::core::magic::MagicError::Incapacitated);
        }
        self.target_manual_spell(caster, &mut request)?;
        self.combatants
            .get_mut(caster)
            .ok_or(crate::core::magic::MagicError::Incapacitated)?
            .cast_spell(request, self.elapsed_seconds, &mut self.rng)?;
        self.resolve_completed_spell_effects(caster);
        self.flush_spell_events();
        Ok(())
    }

    pub fn stop_channeling(&mut self, caster: usize) -> Result<(), crate::core::magic::MagicError> {
        self.combatants
            .get_mut(caster)
            .ok_or(crate::core::magic::MagicError::Incapacitated)?
            .stop_channeling(self.elapsed_seconds)?;
        self.flush_spell_events();
        Ok(())
    }

    pub fn cast_echo_strike(
        &mut self,
        caster: usize,
    ) -> Result<(), crate::core::magic::MagicError> {
        if self.done {
            return Err(crate::core::magic::MagicError::Incapacitated);
        }
        self.combatants
            .get_mut(caster)
            .ok_or(crate::core::magic::MagicError::Incapacitated)?
            .cast_echo_strike(self.elapsed_seconds)?;
        self.combatants[caster].advance_magic(self.elapsed_seconds, &mut self.rng);
        self.flush_spell_events();
        Ok(())
    }

    pub fn cancel_spell(&mut self, caster: usize) -> Result<(), crate::core::magic::MagicError> {
        self.combatants
            .get_mut(caster)
            .ok_or(crate::core::magic::MagicError::Incapacitated)?
            .cancel_spell(self.elapsed_seconds)?;
        self.flush_spell_events();
        Ok(())
    }

    pub fn dismiss_echo_strike(
        &mut self,
        caster: usize,
    ) -> Result<(), crate::core::magic::MagicError> {
        self.combatants
            .get_mut(caster)
            .ok_or(crate::core::magic::MagicError::Incapacitated)?
            .dismiss_echo_strike(self.elapsed_seconds)?;
        self.flush_spell_events();
        Ok(())
    }

    fn flush_spell_events(&mut self) {
        if self.bulk.is_some() { return; }
        for (caster, combatant) in self.combatants.iter_mut().enumerate() {
            for spell in combatant.state.magic.events.drain(..) {
                if self.log_events {
                    let event = CombatEvent {
                        time: spell.time,
                        attacker_idx: caster,
                        defender_idx: caster,
                        kind: CombatEventKind::Spell(spell),
                    };
                    self.last_event = Some(event.clone());
                    self.combat_events.push(event);
                }
            }
        }
    }

    fn resolve_completed_spell_effects(&mut self, caster: usize) {
        for effect in std::mem::take(&mut self.combatants[caster].state.magic.resolved_effects) {
            self.resolve_configured_effect(caster, effect);
        }
    }

    fn advance_spellcasting(&mut self) {
        let now = self.elapsed_seconds;
        for caster in 0..self.combatants.len() {
            let actor = &mut self.combatants[caster];
            actor.state.magic.now = now;
            let magic = &actor.state.magic;
            if actor.magic.loadout.auto_cast == crate::core::magic::AutoCast::SpellPolicies
                && magic.casting.is_none() && magic.channeling.is_none()
                && magic.owned_buffs.is_empty() && magic.resolved_effects.is_empty()
                && !magic.fatigue.is_some_and(|(_, end)| now >= end)
                && !magic.armed_echo.as_ref().is_some_and(|echo| now >= echo.expires_at)
                && !actor.has_eligible_spell_policy()
            {
                // Area effects and discharged echoes advance at arena scope below.
                actor.state.magic.range_targets.clear();
                continue;
            }
            self.combatants[caster].advance_magic(now, &mut self.rng);
            self.resolve_completed_spell_effects(caster);
            self.prepare_spell_targets(caster);
            let in_reach = self.combatants[caster].magic.loadout.auto_cast
                == crate::core::magic::AutoCast::InWeaponReach
                && super::SpellRequest::from_loadout("echo_strike", &self.combatants[caster].magic.loadout)
                    .ok()
                    .and_then(|request| self.predict_spell_target(caster, &request,
                        self.combatants[caster].sheet.offense.weapon.reach_ft))
                    .is_some();
            self.combatants[caster].try_auto_cast(now, in_reach);
            // One-second casts complete in this same arena second.
            self.combatants[caster].advance_magic(now, &mut self.rng);
            self.resolve_completed_spell_effects(caster);
        }
        self.flush_spell_events();
        self.advance_effect_fields();
        for (caster, echo) in super::magic::take_due_echoes(&mut self.combatants, now) {
            self.refresh_intimidation();
            let distance = self
                .distance_between(caster, echo.target)
                .unwrap_or(f32::INFINITY);
            let evasion = self.prepare_escape(echo.target, caster);
            if let Some((attack, evaded)) = super::combat::resolve_echo(
                &mut self.combatants,
                caster,
                &echo,
                distance,
                now,
                &mut self.rng,
            ) {
                if evaded {
                    self.apply_precognition_evasion(echo.target, evasion);
                }
                self.record_attack_metrics(RecordedAttackMetrics {
                    damage_source: ImpactSource::Echo,
                    attacker_idx: caster,
                    defender_idx: echo.target,
                    hp_damage: attack.damage,
                    damage_rolled: None,
                    damage_landed: Some(attack.damage),
                    highest_hit_bucket: Some(false),
                    instant_kill: false,
                    shield_block: false,
                    shield_broken: false,
                    shield_damage: 0,
                    knockback_ft: 0.0,
                    attempted: true,
                    direct_hit: attack.hit,
                    critical: false,
                    trauma_applied: attack.trauma_applied,
                    defender_hp_after: attack.defender_hp_after,
                    armor_prevented: 0,
                    shield_prevented: 0,
                });
                if self.log_events {
                    let event = CombatEvent {
                        time: now,
                        attacker_idx: caster,
                        defender_idx: echo.target,
                        kind: CombatEventKind::Attack(attack),
                    };
                    self.last_event = Some(event.clone());
                    self.combat_events.push(event);
                }
            }
        }
        self.flush_spell_events();
    }

    pub fn distance(&self) -> f32 {
        self.distance_between(0, 1).unwrap_or(0.0)
    }

    pub fn distance_between(&self, a_idx: usize, b_idx: usize) -> Option<f32> {
        let tiles = self.grid_distance_tiles(a_idx, b_idx)?;
        Some(tiles as f32 * self.config.tile_size_ft)
    }

    #[inline(always)]
    fn refresh_streamline_coverage(&mut self) {
        if self.bulk.as_ref().is_some_and(|batch| !batch.may_average_damage) {
            return;
        }
        if self.bulk.is_none() && self.streamline_inputs.is_empty()
            && !self.combatants.iter().any(|c| c.state.active_effects.capabilities().average_damage) {
            return;
        }
        self.refresh_enabled_streamline_coverage();
    }

    #[inline(never)]
    fn refresh_enabled_streamline_coverage(&mut self) {
        let tile_size_ft = self.config.tile_size_ft.max(0.01);
        let mut changed = self.streamline_tile_size != tile_size_ft.to_bits()
            || self.streamline_inputs.len() != self.actors.len();
        self.streamline_tile_size = tile_size_ft.to_bits();
        self.streamline_inputs.resize(self.actors.len(), (GridPos::new(0, 0), None));
        for ((slot, actor), caster) in self.streamline_inputs.iter_mut().zip(&self.actors).zip(&self.combatants) {
            let radius = caster.state.active_effects.iter().filter(|e| e.remaining_seconds > 0)
                .filter_map(|e| e.average_damage_radius).max_by(f32::total_cmp)
                .map(|r| caster.apply_f32(StatIdF32::StreamlineRadius, r).to_bits());
            let input = (actor.position, radius);
            changed |= *slot != input;
            *slot = input;
        }
        if self.bulk.is_some() && !changed { return; }
        for c in &mut self.combatants { c.state.streamline_averages_incoming_damage = false; }
        if self.bulk.is_none() && self.streamline_inputs.iter().all(|(_, radius)| radius.is_none()) {
            self.streamline_inputs.clear();
            return;
        }
        for &(position, radius) in self.streamline_inputs.iter().take(self.combatants.len()) {
            let Some(radius) = radius.map(f32::from_bits) else { continue };
            for (target, combatant) in self.actors.iter().zip(&mut self.combatants) {
                if position.manhattan_distance(target.position) as f32 * tile_size_ft
                    <= radius
                {
                    combatant.state.streamline_averages_incoming_damage = true;
                }
            }
        }
    }

    fn decide_tactic(
        &self, actor: usize, enemy: usize, channel: TacticalChannel, incoming_charge: bool,
    ) -> TacticalDecision {
        let policy = &self.combatants[actor].tactical_policy;
        let live_plan;
        let plan = if let Some(bulk) = &self.bulk {
            &bulk.tactics[actor][channel as usize]
        } else {
            // Interactive callers may edit the public policy between ticks.
            live_plan = ChannelPlan::new(policy, channel);
            &live_plan
        };
        if plan.rules.is_empty() {
            return TacticalDecision::fallback(channel);
        }
        if let Some((index, condition)) = plan.single_rule {
            let matches = match condition {
                tactical_context::SingleCondition::Always => true,
                tactical_context::SingleCondition::EnemyDr(comparison, value) =>
                    comparison.matches(self.combatants[enemy].tactical_enemy_dr(), value),
            };
            return if matches {
                TacticalDecision { action: policy.rules[index].action.clone(), matched_rule_index: Some(index) }
            } else {
                TacticalDecision::fallback(channel)
            };
        }
        let mut context = self.tactical_context_with_needs(actor, enemy, plan.needs);
        if incoming_charge {
            context.enemy_charging = true;
            context.give_ground_legal = false;
        }
        crate::core::tactics::first_matching_rule(
            plan.rules.iter().map(|&i| (i, &policy.rules[i])), channel, &context,
        )
    }

    #[cfg(test)]
    fn tactical_context(&self, mine: usize, enemy: usize) -> TacticalContext {
        self.tactical_context_with_needs(mine, enemy, tactical_context::ContextNeeds::all())
    }

    fn tactical_context_with_needs(
        &self, my_idx: usize, enemy_idx: usize, needs: tactical_context::ContextNeeds,
    ) -> TacticalContext {
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
            combatant.effective_reach()
        };
        let attack_speed = |combatant: &Combatant, opponent: &Combatant| {
            let weapon = &combatant.sheet.offense.weapon;
            let speed = combatant
                .apply_f32(StatIdF32::WeaponSpeed, combatant.sheet.offense.weapon.speed)
                .max(1.0);
            let is_ranged = max_range_for_weapon(weapon).is_some()
                && (weapon.uses_projectiles
                    || self.distance_between(my_idx, enemy_idx).unwrap_or(0.0)
                        > weapon.reach_ft.max(1.0));
            recovery_against_opponent(speed, weapon, opponent, is_ranged)
        };
        let move_speed = |combatant: &Combatant| {
            combatant
                .apply_f32(StatIdF32::MoveSpeed, combatant.sheet.mobility.move_speed)
                .max(0.0)
        };
        let retreat_space_available = needs.retreat
            && self
                .actors
                .get(my_idx)
                .zip(self.actors.get(enemy_idx))
                .is_some_and(|(mine, enemy)| mine.position != enemy.position);
        let enemy_charging = (needs.reaction
            || needs.enemy_charging)
            && enemy.sheet.maneuvers.charge
            && enemy.state.charge_target_idx == Some(my_idx)
            && enemy.state.charge_distance_ft > 0.0;
        let give_ground_legal = needs.reaction
            && retreat_space_available
            && !enemy_charging
            && move_speed(enemy) <= move_speed(mine)
            && self.move_tiles(my_idx) > 0;
        let distance_ft = if needs.distance {
            self.distance_between(my_idx, enemy_idx).unwrap_or(0.0)
        } else {
            0.0
        };
        let enemy_reach_ft = if needs.reach { reach(enemy) } else { 0.0 };
        let enemy_distance_to_reach_ft = (distance_ft - enemy_reach_ft).max(0.0);
        let enemy_closure_per_second = if needs.time {
            self.move_tiles(enemy_idx) as f32 * self.config.tile_size_ft.max(0.01)
        } else {
            0.0
        };
        let enemy_time_to_reach_seconds = if enemy_distance_to_reach_ft <= 0.0 {
            0.0
        } else if enemy_closure_per_second <= 0.0 {
            f32::INFINITY
        } else {
            (enemy_distance_to_reach_ft / enemy_closure_per_second).ceil()
        };

        TacticalContext {
            engaged_in_melee: needs.engaged && enemy.state.weapon_setup.pending.is_none()
                && !enemy.sheet.offense.weapon.uses_projectiles
                && distance_ft <= enemy.apply_f32(StatIdF32::WeaponReach, enemy.sheet.offense.weapon.reach_ft).max(1.0),
            can_equip_melee: mine.weapon_setups.is_some(),
            can_equip_ranged: mine.weapon_setups.is_some(),
            my_hp_percent: if needs.my_hp {
                hp_percent(mine)
            } else {
                0.0
            },
            enemy_hp_percent: if needs.enemy_hp {
                hp_percent(enemy)
            } else {
                0.0
            },
            distance_ft,
            my_reach_ft: if needs.reach { reach(mine) } else { 0.0 },
            enemy_reach_ft,
            retreat_space_available,
            my_weapon_can_jab: needs.jab && mine.tactical_jab_available(),
            my_has_active_shield: mine.sheet.defense.shield_name.is_some()
                && mine.state.shield_intact,
            enemy_weapon_group: if needs.weapon_group {
                enemy.weapon_group.clone()
            } else {
                String::new()
            },
            enemy_has_active_shield: enemy.sheet.defense.shield_name.is_some()
                && enemy.state.shield_intact,
            enemy_armor_type: if needs.armor {
                enemy.armor_type.clone()
            } else {
                String::new()
            },
            enemy_charging,
            my_has_attacked: mine.state.has_attacked,
            enemy_time_to_reach_seconds,
            my_active_style_ids: if needs.my_styles {
                mine.active_style_ids.clone()
            } else {
                Vec::new()
            },
            enemy_active_style_ids: if needs.enemy_styles {
                enemy.active_style_ids.clone()
            } else {
                Vec::new()
            },
            available_style_ids: if needs.styles {
                mine.available_tactical_style_ids()
            } else {
                Vec::new()
            },
            style_pair_allowed: needs.styles && mine.tactical_style_pair_allowed(),
            enemy_dr: if needs.enemy_dr {
                enemy.tactical_enemy_dr()
            } else {
                0.0
            },
            my_attack_speed_seconds: if needs.attack_speed {
                attack_speed(mine, enemy)
            } else {
                0.0
            },
            enemy_attack_speed_seconds: if needs.attack_speed {
                attack_speed(enemy, mine)
            } else {
                0.0
            },
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
        if self.bulk.is_some() {
            return;
        }
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

    /// Select aiming before scheduling an attack, so directives pay the normal
    /// Called Shot delay even when switching on during existing recovery.
    fn refresh_called_shot_tactic(
        &mut self,
        attacker_idx: usize,
        defender_idx: usize,
        distance: f32,
    ) -> [f32; 2] {
        if !self.combatants[attacker_idx].tactical_policy.enabled
            || self.combatants[attacker_idx].state.aggressive_maneuvers_locked {
            return [0.0; 2];
        }
        let cached = self.bulk.as_ref().and_then(|b| b.fixed_dr_called_shot[attacker_idx])
            .filter(|_| self.combatants[defender_idx].state.active_effects.is_empty());
        let called_shot = cached.unwrap_or_else(|| matches!(
            self.decide_tactic(attacker_idx, defender_idx, TacticalChannel::AttackMode, false).action,
            TacticalAction::CalledShot
        ));
        let was_called_shot = self.combatants[attacker_idx].sheet.maneuvers.called_shot;
        self.combatants[attacker_idx].sheet.maneuvers.called_shot = called_shot;
        let mut added = [0.0; 2];
        if called_shot && !was_called_shot {
            let actor = &self.combatants[attacker_idx];
            let pending = [
                (
                    WeaponSlot::Primary,
                    actor.state.next_attack_time_primary,
                    Some(actor.sheet.offense.weapon.clone()),
                ),
                (
                    WeaponSlot::Secondary,
                    actor.state.next_attack_time_secondary,
                    actor
                        .sheet
                        .offense
                        .offhand
                        .as_ref()
                        .map(|offhand| offhand.weapon.clone()),
                ),
            ];
            for (index, (slot, scheduled, weapon)) in pending.into_iter().enumerate() {
                let (Some(scheduled), Some(weapon)) = (scheduled, weapon) else {
                    continue;
                };
                let is_ranged = max_range_for_weapon(&weapon).is_some()
                    && (weapon.uses_projectiles || distance > weapon.reach_ft.max(1.0));
                let delay = called_shot_delay_seconds(
                    &self.combatants[attacker_idx],
                    &self.combatants[defender_idx],
                    is_ranged,
                    &mut self.rng,
                );
                let next = scheduled.max(self.elapsed_seconds as f32) + delay;
                added[index] = next - scheduled;
                self.combatants[attacker_idx]
                    .state
                    .set_next_attack_time(slot, Some(next));
            }
        }
        added
    }

    fn restore_attack_profile(&mut self, idx: usize) {
        let actor = &mut self.combatants[idx];
        if self.bulk.is_none() || !actor.state.normal_tactical_profile_installed {
            actor.activate_tactical_profile(false);
        }
    }

    fn apply_next_attack_tactics(&mut self, attacker_idx: usize, defender_idx: usize) -> bool {
        self.combatants[attacker_idx].state.aggressive_maneuvers_locked = false;
        if !self.combatants[attacker_idx].tactical_policy.enabled {
            return false;
        }
        let style = self.decide_tactic(attacker_idx, defender_idx, TacticalChannel::WeaponStyle, false);
        let mut installed = false;
        match &style.action {
            TacticalAction::RetainWeaponStyle => {}
            TacticalAction::NeutralWeaponStyle => {
                installed |= self.combatants[attacker_idx].switch_tactical_style(Vec::new());
            }
            TacticalAction::UseWeaponStyle { style_ids } => {
                installed |= self.combatants[attacker_idx].switch_tactical_style(style_ids.clone());
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

        let stance = self.decide_tactic(attacker_idx, defender_idx, TacticalChannel::Stance, false);
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
        let actor = &mut self.combatants[attacker_idx];
        actor.active_fight_defensively_penalty = new_stance;
        if self.bulk.is_none() || old_stance != new_stance || !actor.state.normal_tactical_profile_installed {
            installed |= actor.activate_tactical_profile(false);
        }
        if stance.matched_rule_index.is_some() {
            self.record_tactical_directive(
                attacker_idx,
                defender_idx,
                &stance.action,
                stance.matched_rule_index,
                None,
            );
        }

        let attack = self.decide_tactic(attacker_idx, defender_idx, TacticalChannel::AttackMode, false);
        let use_jab = matches!(attack.action, TacticalAction::Jab);
        // The stance step already installed the non-jab profile for this attack.
        if use_jab {
            installed |= self.combatants[attacker_idx].activate_tactical_profile(true);
        }
        if attack.matched_rule_index.is_some() {
            self.record_tactical_directive(
                attacker_idx,
                defender_idx,
                &attack.action,
                attack.matched_rule_index,
                None,
            );
        }
        installed
    }

    fn reaction_possible(&self, defender: usize) -> bool {
        let c = &self.combatants[defender];
        if c.tactical_policy.enabled {
            self.bulk.as_ref().is_none_or(|b| b.moving_reaction[defender])
        } else { c.sheet.maneuvers.give_ground || c.sheet.maneuvers.scamper_back }
    }

    pub(super) fn apply_incoming_attack_tactics(
        &mut self,
        attacker_idx: usize,
        defender_idx: usize,
        attack_mode: AttackMode,
    ) -> bool {
        let defender = &self.combatants[defender_idx];
        if defender.tactical_policy.enabled && self.bulk.as_ref().is_some_and(|b| !b.moving_reaction[defender_idx]) {
            return false;
        }
        if !defender.tactical_policy.enabled
            && !defender.sheet.maneuvers.give_ground
            && !defender.sheet.maneuvers.scamper_back
        {
            return false;
        }
        if defender.state.aggressive_maneuvers_locked
            || defender.state.hp <= 0
            || defender.state.trauma_remaining_seconds > 0
            || defender.sheet.maneuvers.passive
        {
            return false;
        }
        let reaction = if defender.tactical_policy.enabled {
            self.decide_tactic(defender_idx, attacker_idx, TacticalChannel::Reaction, attack_mode == AttackMode::Charge)
        } else {
            let mut context = self.tactical_context_with_needs(
                defender_idx, attacker_idx, tactical_context::ContextNeeds::reaction(),
            );
            context.give_ground_legal &= attack_mode != AttackMode::Charge;
            let maneuvers = &self.combatants[defender_idx].sheet.maneuvers;
            let action = if context.give_ground_legal && maneuvers.scamper_back {
                TacticalAction::ScamperBack
            } else if context.give_ground_legal && maneuvers.give_ground {
                TacticalAction::GiveGround
            } else {
                TacticalAction::StandGround
            };
            crate::core::tactics::TacticalDecision {
                action,
                matched_rule_index: None,
            }
        };
        let scamper = matches!(reaction.action, TacticalAction::ScamperBack);
        if !matches!(
            reaction.action,
            TacticalAction::GiveGround | TacticalAction::ScamperBack
        ) {
            if reaction.matched_rule_index.is_some() {
                self.record_tactical_directive(
                    defender_idx,
                    attacker_idx,
                    &reaction.action,
                    reaction.matched_rule_index,
                    None,
                );
            }
            return false;
        }
        let before = self.actors[defender_idx].position;
        let original_distance = self
            .distance_between(attacker_idx, defender_idx)
            .unwrap_or(0.0);
        let steps = self.move_tiles(defender_idx) * if scamper { 2 } else { 1 };
        self.move_away(defender_idx, attacker_idx, steps);
        let moved_tiles = before.manhattan_distance(self.actors[defender_idx].position);
        if moved_tiles <= 0 {
            return false;
        }
        self.combatants[defender_idx].state.moved_last_tick = true;
        self.combatants[defender_idx]
            .state
            .tactical_give_ground_defense_bonus = 5;
        self.combatants[defender_idx].state.tactical_retreat = Some(scamper);
        self.combatants[defender_idx]
            .state
            .tactical_next_attack_penalty += if scamper { 4 } else { 1 };
        let pursuer_before = self.actors[attacker_idx].position;
        if self.combatants[attacker_idx].apply_i32(StatIdI32::FlagDeclinePursuit, 0) == 0
            && self.combatants[attacker_idx].apply_f32(
                StatIdF32::MoveSpeed,
                self.combatants[attacker_idx].sheet.mobility.move_speed,
            ) > 0.0
        {
            self.move_toward(attacker_idx, defender_idx, moved_tiles, original_distance);
        }
        let pursued = self.actors[attacker_idx].position != pursuer_before;
        if pursued {
            self.combatants[attacker_idx].state.moved_last_tick = true;
        }
        if self.bulk.is_none() {
            let moved_ft = moved_tiles as f32 * self.config.tile_size_ft;
            self.record_tactical_directive(
                defender_idx,
                attacker_idx,
                &reaction.action,
                reaction.matched_rule_index,
                Some(format!(
                    "{}: {} {:.0}ft (+5 Defense, -{} next Attack)",
                    self.combatants[defender_idx].sheet.name,
                    reaction.action.label(),
                    moved_ft,
                    if scamper { 4 } else { 1 }
                )),
            );
        }
        if pursued
            && self.combatants[defender_idx].apply_i32(StatIdI32::FlagPilgrimsPathStyle, 0) > 0
        {
            let distance = self
                .distance_between(attacker_idx, defender_idx)
                .unwrap_or(f32::INFINITY);
            let attacker_evasion = self.prepare_escape(attacker_idx, defender_idx);
            let defender_evasion = self.prepare_escape(defender_idx, attacker_idx);
            self.refresh_intimidation();
            for counter in super::combat::resolve_style_strike_chain(
                &mut self.combatants,
                defender_idx,
                attacker_idx,
                WeaponSlot::Primary,
                distance,
                self.elapsed_seconds as f32,
                &mut self.rng,
            ) {
                let evasion = if counter.defender_idx == attacker_idx {
                    attacker_evasion
                } else {
                    defender_evasion
                };
                let defender_idx = counter.attacker_idx;
                let attacker_idx = counter.defender_idx;
                self.record_attack_metrics(RecordedAttackMetrics::from_counter(&counter));
                if counter.precognition_triggered {
                    self.apply_precognition_evasion(attacker_idx, evasion);
                }
                if counter.hit && !counter.is_ranged {
                    self.apply_six_paths_followup(defender_idx, self.elapsed_seconds as f32);
                }
                if counter.shield_block {
                    self.apply_shield_strike_speedup(attacker_idx, self.elapsed_seconds as f32);
                }
                self.apply_knockback(defender_idx, attacker_idx, counter.knockback_ft);
                self.first_attack_time.get_or_insert(self.elapsed_seconds);
                if counter.trauma_applied {
                    self.combatants[attacker_idx].state.saw_trauma = true;
                    if self.first_attack_time == Some(self.elapsed_seconds) {
                        self.trauma_first_exchange = true;
                    }
                }
                if self.log_events {
                    let event = CombatEvent {
                        time: self.elapsed_seconds,
                        attacker_idx: defender_idx,
                        defender_idx: attacker_idx,
                        kind: CombatEventKind::Attack(super::counter_event(counter)),
                    };
                    self.last_event = Some(event.clone());
                    self.combat_events.push(event);
                }
            }
        }
        true
    }

    fn incoming_attack_cancelled(
        &mut self,
        attacker: usize,
        defender: usize,
        slot: WeaponSlot,
        ranged: bool,
    ) -> bool {
        if !self.combatants[attacker].magic_can_attack(slot, self.elapsed_seconds as f32)
            || self.combatants[attacker].state.hp <= 0
            || self.combatants[defender].state.hp <= 0
            || self.combatants[attacker].state.trauma_remaining_seconds > 0
        {
            self.combatants[defender]
                .state
                .tactical_give_ground_defense_bonus = 0;
            self.combatants[defender].state.tactical_retreat = None;
            return true;
        }
        let weapon = match slot {
            WeaponSlot::Primary => &self.combatants[attacker].sheet.offense.weapon,
            WeaponSlot::Secondary => {
                &self.combatants[attacker]
                    .sheet
                    .offense
                    .offhand
                    .as_ref()
                    .unwrap()
                    .weapon
            }
        };
        if !ranged
            && self
                .distance_between(attacker, defender)
                .unwrap_or(f32::INFINITY)
                > self.combatants[attacker]
                    .apply_f32(StatIdF32::WeaponReach, weapon.reach_ft)
                    .max(0.5)
        {
            let next = self.elapsed_seconds as f32 + weapon.speed;
            self.combatants[attacker]
                .state
                .set_next_attack_time(slot, Some(next));
            self.combatants[defender]
                .state
                .tactical_give_ground_defense_bonus = 0;
            self.combatants[defender].state.tactical_retreat = None;
            return true;
        }
        false
    }

    fn grid_distance_tiles(&self, a_idx: usize, b_idx: usize) -> Option<i32> {
        let pos_a = self.actors.get(a_idx)?.position;
        let pos_b = self.actors.get(b_idx)?.position;
        Some(pos_a.manhattan_distance(pos_b))
    }

    fn move_tiles(&self, idx: usize) -> i32 {
        self.movement_tiles(&self.combatants[idx], self.bulk.as_ref().and_then(|b| b.fixed_move_tiles[idx]))
    }

    fn movement_tiles(&self, combatant: &Combatant, ordinary: Option<i32>) -> i32 {
        if combatant.sheet.maneuvers.passive || combatant.racial_actions_blocked()
            || combatant.state.weapon_setup.pending.is_some() {
            return 0;
        }
        if combatant.state.trauma_remaining_seconds > 0
            || combatant.state.knockback_immobile_seconds > 0
        {
            return 0;
        }
        let fractional = combatant.state.magic.casting.is_some() || combatant.state.magic.channeling.is_some()
            || combatant.state.magic.fatigued();
        if !fractional && combatant.state.active_effects.is_empty() {
            if let Some(tiles) = ordinary { return tiles; }
        }
        let speed_ft = combatant
            .apply_f32(StatIdF32::MoveSpeed, combatant.sheet.mobility.move_speed)
            .max(0.0);
        let speed_ft = super::ranged_styles::shooting_move_speed(combatant, speed_ft);
        if speed_ft <= 0.0 {
            return 0;
        }
        let tile_size_ft = self.config.tile_size_ft.max(0.01);
        if fractional {
            let rate = speed_ft / tile_size_ft;
            return (((self.elapsed_seconds + 1) as f32 * rate).floor()
                - (self.elapsed_seconds as f32 * rate).floor()) as i32;
        }
        let tiles = (speed_ft / tile_size_ft).round();
        if tiles <= 0.0 { 1 } else { tiles as i32 }
    }

    fn skip_stationary_recovery(&mut self, limit: u32) {
        if self.done || !self.bulk.as_ref().is_some_and(|b| b.simple_recovery)
            || self.hold_at_bay.active || self.hold_at_bay.pending { return; }
        let now = self.elapsed_seconds;
        // The caller polls cancellation before ticking at exact 64-second boundaries.
        // Beyond this point consecutive integer seconds can share one f32 attack time.
        if now.is_multiple_of(64) || now >= (1 << 24) { return; }
        let mut end = limit.min(now.saturating_add(64 - now % 64));
        let distance = self.distance_between(0, 1).unwrap_or(f32::INFINITY);
        for c in &self.combatants {
            let s = &c.state;
            if s.moved_last_tick || s.knockback_applied_this_tick || s.trauma_remaining_seconds > 0
                || s.knockback_immobile_seconds > 0 || !s.active_effects.is_empty() || c.racial_actions_blocked()
                || c.sheet.maneuvers.offensive_dualwielding || c.sheet.maneuvers.passive
                || distance > c.effective_reach() || (c.sheet.defense.eyesmite && distance > 5.0) { return; }
            let Some(next) = s.next_attack_time_primary else { return };
            let mut deadline = next.ceil() as u32;
            if deadline > 0 && (deadline - 1) as f32 + 0.0001 >= next { deadline -= 1; }
            end = end.min(deadline);
        }
        if end > now {
            for _ in now..end { self.rng.next_u32(); }
            for c in &mut self.combatants { c.state.magic.now = end - 1; }
            self.elapsed_seconds = end;
        }
    }

    fn move_for_separation(&mut self, mover: usize, target: usize, stop: Option<f32>, distance: f32) -> f32 {
        let before = self.actors[mover].position;
        let steps = self.move_tiles(mover);
        if let Some(feet) = stop { self.move_toward(mover, target, steps, feet); }
        else { self.move_away(mover, target, steps); }
        if self.actors[mover].position == before { distance }
        else { self.distance_between(mover, target).unwrap_or(distance) }
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
        let from = self.actors[mover_idx].position;
        let to = self.actors[target_idx].position;
        // An aligned path never changes axis. Find its stopping tile using the
        // same distance comparison as the stepping loop, including float rounding.
        if (from.x == to.x || from.y == to.y)
            && self.config.tile_size_ft > 0.0
            && self.config.tile_size_ft.is_finite()
            && stop_distance_ft.is_finite()
        {
            let distance = from.manhattan_distance(to);
            let mut low = 0;
            let mut high = steps.min(distance);
            // All integer tile distances in this range are exactly representable.
            if self.config.tile_size_ft == 1.0 && distance <= (1 << 24) && stop_distance_ft >= 0.0 {
                low = high.min(distance.saturating_sub(stop_distance_ft.floor() as i32).max(0));
                high = low;
            }
            while low < high {
                let middle = low + (high - low) / 2;
                if (distance - middle) as f32 * self.config.tile_size_ft <= stop_distance_ft {
                    high = middle;
                } else {
                    low = middle + 1;
                }
            }
            self.actors[mover_idx].position = from.offset(from.direction_to(to), low);
            return;
        }
        for _ in 0..steps {
            let distance = self.distance_between(mover_idx, target_idx).unwrap_or(0.0);
            if distance <= stop_distance_ft {
                break;
            }
            let from = self.actors[mover_idx].position;
            let to = self.actors[target_idx].position;
            let next = from.offset(from.direction_to(to), 1);
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
        let from = self.actors[mover_idx].position;
        // Every step away stays on the same axis.
        let away = self.actors[target_idx].position.direction_to(from);
        self.actors[mover_idx].position = from.offset(away, steps);
    }

    fn update_charge_progress(
        &mut self,
        active_pair: Option<(usize, usize)>,
        old_positions: &[GridPos],
        duel_moved: Option<[bool; 2]>,
    ) {
        let tile_size_ft = self.config.tile_size_ft.max(0.01);
        let (pair_a, pair_b) = active_pair.unwrap_or((usize::MAX, usize::MAX));
        let indices = self.bulk.as_ref().map(|b| b.charge_indices.as_slice());
        for i in 0..indices.map_or(self.combatants.len(), <[usize]>::len) {
            let idx = indices.map_or(i, |v| v[i]);
            let target_idx = if idx == pair_a {
                Some(pair_b)
            } else if idx == pair_b {
                Some(pair_a)
            } else {
                None
            };
            let combatant = &self.combatants[idx];
            if !combatant.sheet.maneuvers.charge
                || combatant.magic_blocks_running()
                || target_idx.is_none()
            {
                let state = &mut self.combatants[idx].state;
                if state.charge_target_idx.is_some() || state.charge_distance_ft != 0.0 {
                    state.charge_distance_ft = 0.0;
                    state.charge_target_idx = None;
                }
                continue;
            }
            let reach = combatant.effective_reach();
            let current_target = combatant.state.charge_target_idx;
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
            let state = &mut self.combatants[idx].state;
            if !duel_moved.map_or_else(|| old_pos != new_pos, |m| m[idx]) {
                if distance_before > reach {
                    state.charge_distance_ft = 0.0;
                }
                continue;
            }
            let distance_after = new_pos.manhattan_distance(old_target_pos) as f32 * tile_size_ft;
            if distance_after >= distance_before {
                state.charge_distance_ft = 0.0;
                continue;
            }
            if distance_before > reach {
                let moved_ft = old_pos.manhattan_distance(new_pos) as f32 * tile_size_ft;
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

    #[inline(always)]
    fn prepare_escape(&mut self, defender: usize, attacker: usize) -> EscapeDestination {
        if !self.combatants.get(defender).is_some_and(|c| c.sheet.defense.precognition) {
            return EscapeDestination::Known(None);
        }
        self.prepare_enabled_escape(defender, attacker)
    }

    #[inline(never)]
    fn prepare_enabled_escape(&mut self, defender: usize, attacker: usize) -> EscapeDestination {
        let pending = if self.bulk.is_some() && self.actors.len() == 2 && attacker < 2 && defender < 2 {
            EscapeDestination::Duel(super::movement::DuelEscape {
                from: self.actors[defender].position, attacker: self.actors[attacker].position, tile_size_ft: self.config.tile_size_ft,
            })
        } else { EscapeDestination::Known(self.precognition_evasion_destination(defender, attacker)) };
        let state = &mut self.combatants[defender].state;
        state.duel_escape = match pending { EscapeDestination::Duel(d) => Some(d), _ => None };
        state.precognition_space_available = matches!(pending, EscapeDestination::Known(Some(_)));
        pending
    }

    fn precognition_evasion_destination(
        &self,
        defender_idx: usize,
        attacker_idx: usize,
    ) -> Option<GridPos> {
        let from = self.actors.get(defender_idx)?.position;
        let attacker = self.actors.get(attacker_idx)?.position;
        let geometry = super::movement::DuelEscape { from, attacker, tile_size_ft: self.config.tile_size_ft };
        if self.bulk.is_some() && self.actors.len() == 2 { return geometry.destination(); }
        let directions = geometry.directions()?;
        let steps = (5.0 / self.config.tile_size_ft.max(0.01)).ceil() as i32;
        for (step_x, step_y) in directions {
            let mut current = from;
            let mut valid = true;
            for _ in 0..steps.max(1) {
                let next = current.offset((step_x, step_y), 1);
                if next == current || self.actors.iter().enumerate().any(|(idx, actor)| {
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

    fn apply_precognition_evasion(&mut self, defender_idx: usize, escape: EscapeDestination) {
        let destination = match escape { EscapeDestination::Known(p) => p, EscapeDestination::Duel(d) => d.destination() };
        if let (Some(actor), Some(destination)) = (self.actors.get_mut(defender_idx), destination) {
            actor.position = destination;
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
        self.move_away(mover_idx, other_idx, min_tiles - current_tiles);
    }

    fn multiple_teams_remain(&self) -> bool {
        let mut living = self
            .combatants
            .iter()
            .filter(|combatant| combatant.state.hp > 0);
        living
            .next()
            .is_some_and(|first| living.any(|other| other.team_id != first.team_id))
    }

    fn active_pair(&self) -> Option<(usize, usize)> {
        if let [first, second] = self.combatants.as_slice() {
            return (first.state.hp > 0 && second.state.hp > 0 && first.team_id != second.team_id)
                .then_some((0, 1));
        }
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
        let distance = self.distance_between(a_idx, b_idx).unwrap_or(0.0);
        let reach_a = self.combatants[a_idx].effective_reach();
        let reach_b = self.combatants[b_idx].effective_reach();
        self.resolve_combat_round_at(a_idx, b_idx, distance, reach_a, reach_b);
    }

    fn resolve_combat_round_at(&mut self, a_idx: usize, b_idx: usize, distance: f32, reach_a: f32, reach_b: f32) {
        let track_streamline = self.bulk.as_ref().is_none_or(|b| b.may_average_damage);
        if track_streamline { self.refresh_streamline_coverage(); }
        let now = self.elapsed_seconds as f32;
        if let Some(batch) = &self.bulk {
            if self.combatants.len() == 2 && !self.hold_at_bay.active && !self.hold_at_bay.pending
                && [(a_idx, b_idx), (b_idx, a_idx)].into_iter().all(|(idx, enemy)| {
                    let c = &self.combatants[idx];
                    let future = |time: Option<f32>| time.is_some_and(|t| t > now + 0.0001);
                    c.state.hp > 0 && c.state.trauma_remaining_seconds == 0 && !c.sheet.maneuvers.passive
                        && future(c.state.next_attack_time_primary)
                        && (!c.sheet.maneuvers.offensive_dualwielding || c.sheet.offense.offhand.is_none()
                            || future(c.state.next_attack_time_secondary))
                        && (!c.tactical_policy.enabled || c.state.aggressive_maneuvers_locked
                            || (self.combatants[enemy].state.active_effects.is_empty()
                                && batch.fixed_dr_called_shot[idx] == Some(c.sheet.maneuvers.called_shot))
                            || (batch.tactics[idx][TacticalChannel::AttackMode as usize].rules.is_empty()
                                && !c.sheet.maneuvers.called_shot))
                })
            {
                self.rng.next_u32(); // Every combat round randomises the two attack slots.
                return;
            }
        }
        // Tactical decisions may change either sheet inside this round. The batch
        // includes every reachable sheet. Live effects may additionally grant a style.
        let followups = self.bulk.as_ref().map_or(u64::MAX, |batch| batch.possible_sheet_flags)
            | self.combatants[a_idx].state.active_effects.capabilities().integers
            | self.combatants[b_idx].state.active_effects.capabilities().integers;
        let six_paths = followups & StatIdI32::FlagSixPathsStyle.bit() != 0;
        let twelve_paths = followups & StatIdI32::FlagLargeSwordShieldStyle.bit() != 0;
        let precognition = self.bulk.as_ref().is_none_or(|batch| batch.has_precognition);
        let simultaneous = (reach_a - reach_b).abs() < f32::EPSILON;
        let state_snapshot = simultaneous
            .then(|| super::combat::CombatRoundSnapshot::capture(&self.combatants, a_idx, b_idx));
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
            if !self.combatants[attacker_idx].magic_can_attack(WeaponSlot::Primary, now)
                && (self.combatants[attacker_idx]
                    .sheet
                    .offense
                    .offhand
                    .is_none()
                    || !self.combatants[attacker_idx].magic_can_attack(WeaponSlot::Secondary, now))
            {
                continue;
            }
            if track_streamline { self.refresh_streamline_coverage(); }
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
                    self.refresh_intimidation();
                    let event = resolve_knock_aside(
                        &mut self.combatants,
                        attacker_idx,
                        defender_idx,
                        state_snapshot.as_ref(),
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
            let max_range = {
                let combatant = &mut self.combatants[attacker_idx];
                max_range_cached(
                    &mut combatant.state,
                    WeaponSlot::Primary,
                    &combatant.sheet.offense.weapon,
                )
            };
            let weapon = &self.combatants[attacker_idx].sheet.offense.weapon;
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
                && !self.combatants[attacker_idx].magic_blocks_running()
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
            let mut scheduled_primary_speed = None;
            let mut primary_attack_time = None;
            let mut scheduled_primary_attack_time = None;
            if !use_ranged && distance > attacker_reach {
                continue;
            }
            if use_ranged && ranged_mod.is_none() {
                continue;
            }
            let mut called_shot_delays =
                if self.combatants[attacker_idx].tactical_policy.enabled {
                    self.refresh_called_shot_tactic(attacker_idx, defender_idx, distance)
                } else { [0.0; 2] };
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
                let scheduled_time = now + delay + called_shot_delay
                    + if use_ranged { self.combatants[attacker_idx].sheet.offense.weapon.ranged_styles.aim_seconds() } else { 0.0 };
                self.combatants[attacker_idx]
                    .state
                    .set_next_attack_time(WeaponSlot::Primary, Some(scheduled_time));
                scheduled_primary_attack_time = Some(scheduled_time);
            }
            let next_attack = if use_snapshot_timing {
                snapshot_next_attack_primary
                    .map(|time| time + called_shot_delays[0])
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
                let previous_aim = self.combatants[attacker_idx].sheet.offense.weapon.ranged_styles.aim_seconds();
                let previous_offhand_aim = self.combatants[attacker_idx].sheet.offense.offhand.as_ref()
                    .map_or(0.0, |o| o.weapon.ranged_styles.aim_seconds());
                let profile_changed = self.apply_next_attack_tactics(attacker_idx, defender_idx);
                if profile_changed {
                    let new_offhand_aim = self.combatants[attacker_idx].sheet.offense.offhand.as_ref()
                        .map_or(0.0, |o| o.weapon.ranged_styles.aim_seconds());
                    let offhand_added = (new_offhand_aim - previous_offhand_aim).max(0.0);
                    if offhand_added > 0.0 {
                        if let Some(scheduled) = self.combatants[attacker_idx].state.next_attack_time_secondary {
                            let next = scheduled.max(now) + offhand_added;
                            called_shot_delays[1] += next - scheduled;
                            self.combatants[attacker_idx].state.set_next_attack_time(WeaponSlot::Secondary, Some(next));
                        }
                    }
                    let aim_added = self.combatants[attacker_idx].sheet.offense.weapon.ranged_styles.aim_seconds() - previous_aim;
                    if aim_added > 0.0 {
                        self.combatants[attacker_idx].state.set_next_attack_time(WeaponSlot::Primary, Some(now + aim_added));
                        continue;
                    }
                }

                // Style/stance rules can change attack-mode conditions. Recheck
                // before striking, and wait if that decision newly requires aim.
                let added = if self.combatants[attacker_idx].tactical_policy.enabled {
                    self.refresh_called_shot_tactic(attacker_idx, defender_idx, distance)
                } else { [0.0; 2] };
                if added[0] > 0.0 {
                    continue;
                }

                // The due time is intentionally unchanged. The newly selected
                // profile controls this attack and the recovery scheduled below.
                if profile_changed {
                    let c = &mut self.combatants[attacker_idx];
                    has_range = max_range_cached(&mut c.state, WeaponSlot::Primary, &c.sheet.offense.weapon).is_some();
                }
                let c = &self.combatants[attacker_idx];
                let weapon = &c.sheet.offense.weapon;
                attacker_reach = c.apply_f32(StatIdF32::WeaponReach, weapon.reach_ft).max(1.0);
                let updated_ranged = has_range && (weapon.uses_projectiles || distance > attacker_reach);
                if profile_changed || updated_ranged != use_ranged {
                    ranged_mod = if updated_ranged {
                        let scale = c.apply_f32(StatIdF32::RangeDistanceMultiplier, weapon.range_distance_multiplier);
                        range_modifier_for_weapon_with_scale(weapon, distance, scale)
                    } else { None };
                }
                use_ranged = updated_ranged;
                let primary_speed_base = self.combatants[attacker_idx]
                    .apply_f32(StatIdF32::WeaponSpeed, weapon.speed)
                    .max(1.0);
                scheduled_primary_speed = Some(primary_speed_base);
                if (!use_ranged && distance > attacker_reach)
                    || (use_ranged && ranged_mod.is_none())
                {
                    self.restore_attack_profile(attacker_idx);
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
                if self.reaction_possible(defender_idx) && self.apply_incoming_attack_tactics(attacker_idx, defender_idx, attack_mode)
                    && self.incoming_attack_cancelled(
                        attacker_idx,
                        defender_idx,
                        WeaponSlot::Primary,
                        use_ranged,
                    )
                {
                    continue;
                }
                let (defender_evasion, attacker_evasion) = if precognition {
                    (self.prepare_escape(defender_idx, attacker_idx), self.prepare_escape(attacker_idx, defender_idx))
                } else {
                    (EscapeDestination::Known(None), EscapeDestination::Known(None))
                };
                let attack_distance = self
                    .distance_between(attacker_idx, defender_idx)
                    .unwrap_or(distance);
                self.refresh_intimidation();
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
                    state_snapshot.as_ref(),
                    &mut self.rng,
                );
                self.combatants[defender_idx]
                    .state
                    .tactical_give_ground_defense_bonus = 0;
                self.combatants[defender_idx].state.tactical_retreat = None;
                let six_paths_followup = six_paths && event.hit && !event.is_ranged;
                if attack_mode == AttackMode::HoldAtBay && self.hold_at_bay.pending {
                    if event.hit {
                        self.hold_at_bay.active = true;
                        self.hold_at_bay.pending = false;
                    } else {
                        self.hold_at_bay = HoldAtBayState::default();
                    }
                }
                if twelve_paths && event.shield_block {
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
                            source: crate::core::sim::AttackSource::Weapon,
                            damage_source: event.damage_source.describe(),
                            called_shot: self.combatants[attacker_idx].sheet.maneuvers.called_shot,
                            hit: event.hit,
                            shield_block: event.shield_block,
                            damage: event.damage,
                            shield_damage: event.shield_damage,
                            knockback_ft: event.knockback_ft,
                            hold_at_bay: event.hold_at_bay,
                            is_charge: attack_mode == AttackMode::Charge,
                            is_aggressive: event.is_aggressive,
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
                for counter in event
                    .counter_attack
                    .take()
                    .into_iter()
                    .chain(event.additional_counters.drain(..))
                {
                    if twelve_paths && counter.shield_block {
                        self.apply_shield_strike_speedup(counter.defender_idx, now);
                    }
                    if six_paths && counter.hit && !counter.is_ranged {
                        self.apply_six_paths_followup(counter.attacker_idx, now);
                    }
                    self.record_attack_metrics(RecordedAttackMetrics::from_counter(&counter));
                    if counter.precognition_triggered {
                        self.apply_precognition_evasion(
                            counter.defender_idx,
                            if counter.defender_idx == attacker_idx {
                                attacker_evasion
                            } else {
                                defender_evasion
                            },
                        );
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
                                source: crate::core::sim::AttackSource::Weapon,
                                damage_source: counter.damage_source.describe(),
                                called_shot: false,
                                hit: counter.hit,
                                shield_block: counter.shield_block,
                                damage: counter.damage,
                                shield_damage: counter.shield_damage,
                                knockback_ft: counter.knockback_ft,
                                hold_at_bay: false,
                                is_charge: false,
                                is_aggressive: false,
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
                speed = recovery_against_opponent(
                    speed, &self.combatants[attacker_idx].sheet.offense.weapon,
                    &self.combatants[defender_idx], event.is_ranged,
                );
                speed += called_shot_delay_seconds(
                    &self.combatants[attacker_idx],
                    &self.combatants[event.defender_idx],
                    event.is_ranged,
                    &mut self.rng,
                );
                if event.is_ranged { speed += self.combatants[attacker_idx].sheet.offense.weapon.ranged_styles.aim_seconds(); }
                self.combatants[attacker_idx]
                    .state
                    .set_next_attack_time(WeaponSlot::Primary, Some(next_attack + speed));
                self.combatants[attacker_idx]
                    .state
                    .tactical_next_attack_penalty = 0;
                self.restore_attack_profile(attacker_idx);
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
                        let speed = scheduled_primary_speed.unwrap_or_else(|| {
                            let actor = &self.combatants[attacker_idx];
                            actor.apply_f32(StatIdF32::WeaponSpeed, actor.sheet.offense.weapon.speed).max(1.0)
                        });
                        2.0 + (speed / 2.0).ceil()
                    };
                    let offset = if faster_against_unarmed(
                        weapon, &self.combatants[defender_idx], use_ranged,
                    ) {
                        (offset / 2.0).ceil().max(1.0)
                    } else {
                        offset
                    };
                    let called_shot_delay = called_shot_delay_seconds(
                        &self.combatants[attacker_idx],
                        &self.combatants[defender_idx],
                        use_ranged,
                        &mut self.rng,
                    );
                    let scheduled_time = primary_anchor + offset + called_shot_delay
                        + if use_ranged { weapon.ranged_styles.aim_seconds() } else { 0.0 };
                    self.combatants[attacker_idx]
                        .state
                        .set_next_attack_time(WeaponSlot::Secondary, Some(scheduled_time));
                    scheduled_secondary_attack_time = Some(scheduled_time);
                }
                let next_attack = if use_snapshot_timing {
                    snapshot_next_attack_secondary
                        .map(|time| time + called_shot_delays[1])
                        .or(scheduled_secondary_attack_time)
                        .unwrap_or(now)
                } else {
                    self.combatants[attacker_idx]
                        .state
                        .next_attack_time_secondary
                        .unwrap_or(now)
                };
                if now + 0.0001 >= next_attack {
                    self.combatants[attacker_idx].state.aggressive_maneuvers_locked = false;
                    if self.reaction_possible(defender_idx) && self.apply_incoming_attack_tactics(
                        attacker_idx,
                        defender_idx,
                        AttackMode::Normal,
                    ) && self.incoming_attack_cancelled(
                        attacker_idx,
                        defender_idx,
                        WeaponSlot::Secondary,
                        use_ranged,
                    ) {
                        continue;
                    }
                    let (defender_evasion, attacker_evasion) = if precognition {
                        (self.prepare_escape(defender_idx, attacker_idx), self.prepare_escape(attacker_idx, defender_idx))
                    } else {
                        (EscapeDestination::Known(None), EscapeDestination::Known(None))
                    };
                    let attack_distance = self
                        .distance_between(attacker_idx, defender_idx)
                        .unwrap_or(distance);
                    self.refresh_intimidation();
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
                        state_snapshot.as_ref(),
                        &mut self.rng,
                    );
                    self.combatants[defender_idx]
                        .state
                        .tactical_give_ground_defense_bonus = 0;
                    self.combatants[defender_idx].state.tactical_retreat = None;
                    let six_paths_followup = six_paths && event.hit && !event.is_ranged;
                    if twelve_paths && event.shield_block {
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
                                source: crate::core::sim::AttackSource::Weapon,
                                damage_source: event.damage_source.describe(),
                                called_shot: self.combatants[attacker_idx].sheet.maneuvers.called_shot,
                                hit: event.hit,
                                shield_block: event.shield_block,
                                damage: event.damage,
                                shield_damage: event.shield_damage,
                                knockback_ft: event.knockback_ft,
                                hold_at_bay: event.hold_at_bay,
                                is_charge: false,
                                is_aggressive: event.is_aggressive,
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
                    for counter in event
                        .counter_attack
                        .take()
                        .into_iter()
                        .chain(event.additional_counters.drain(..))
                    {
                        if twelve_paths && counter.shield_block {
                            self.apply_shield_strike_speedup(counter.defender_idx, now);
                        }
                        if six_paths && counter.hit && !counter.is_ranged {
                            self.apply_six_paths_followup(counter.attacker_idx, now);
                        }
                        self.record_attack_metrics(RecordedAttackMetrics::from_counter(&counter));
                        if counter.precognition_triggered {
                            self.apply_precognition_evasion(
                                counter.defender_idx,
                                if counter.defender_idx == attacker_idx {
                                    attacker_evasion
                                } else {
                                    defender_evasion
                                },
                            );
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
                                    source: crate::core::sim::AttackSource::Weapon,
                                    damage_source: counter.damage_source.describe(),
                                    called_shot: false,
                                    hit: counter.hit,
                                    shield_block: counter.shield_block,
                                    damage: counter.damage,
                                    shield_damage: counter.shield_damage,
                                    knockback_ft: counter.knockback_ft,
                                    hold_at_bay: false,
                                    is_charge: false,
                                    is_aggressive: false,
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
                    speed = recovery_against_opponent(
                        speed, weapon, &self.combatants[defender_idx], event.is_ranged,
                    );
                    speed += called_shot_delay_seconds(
                        &self.combatants[attacker_idx],
                        &self.combatants[event.defender_idx],
                        event.is_ranged,
                        &mut self.rng,
                    );
                    if event.is_ranged { speed += weapon.ranged_styles.aim_seconds(); }
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

    #[test]
    fn unarmed_opponent_checks_both_hands_and_keeps_quiet_river_unarmed() {
        use crate::core::sim::OffhandProfile;
        use std::sync::Arc;

        let mut defender = Combatant::default();
        assert!(unarmed_opponent(&defender));
        let weapon = Arc::make_mut(&mut defender.sheet.offense.weapon);
        weapon.has_weapon = true;
        weapon.is_unarmed = true;
        defender.sheet.modifiers.add_i32(StatIdI32::FlagQuietRiverStyle, ModifierOpI32::Set(1));
        assert!(unarmed_opponent(&defender));
        defender.sheet.defense.shield_name = Some("Shield".into());
        defender.state.shield_intact = true;
        assert!(unarmed_opponent(&defender));

        let mut offhand = (*defender.sheet.offense.weapon).clone();
        offhand.is_unarmed = false;
        defender.sheet.offense.offhand = Some(OffhandProfile {
            attack_bonus: 0,
            strength_damage: 0,
            weapon: Arc::new(offhand),
        });
        assert!(!unarmed_opponent(&defender));
        defender.sheet.offense.offhand = None;
        Arc::make_mut(&mut defender.sheet.offense.weapon).is_unarmed = false;
        assert!(!unarmed_opponent(&defender));
    }

    fn speed_duel(attacker_unarmed: bool, defender_unarmed: bool, ranged: bool) -> SimState {
        use std::sync::Arc;
        let mut attacker = Combatant::default();
        attacker.team_id = 0;
        attacker.sheet.vitals.max_hp = 10_000;
        attacker.sheet.vitals.threshold_of_pain = 10_000;
        let weapon = Arc::make_mut(&mut attacker.sheet.offense.weapon);
        weapon.has_weapon = true;
        weapon.is_unarmed = attacker_unarmed;
        weapon.speed = 7.0;
        weapon.crit_min_roll = 21;
        if ranged {
            weapon.range_bands_feet = Some([30.0, 60.0, 90.0, 120.0]);
            weapon.uses_projectiles = true;
        }
        let mut defender = Combatant::default();
        defender.team_id = 1;
        defender.sheet.vitals.max_hp = 10_000;
        defender.sheet.vitals.threshold_of_pain = 10_000;
        defender.sheet.defense.knockback_step = 10_000;
        defender.sheet.maneuvers.passive = true;
        let weapon = Arc::make_mut(&mut defender.sheet.offense.weapon);
        weapon.has_weapon = true;
        weapon.is_unarmed = defender_unarmed;
        let mut sim = SimState::with_rng(SimConfig::new(1.0, 1.0), SimRng::from_seed(77));
        sim.reset_with_combatants(vec![attacker, defender]);
        sim
    }

    #[test]
    fn only_armed_melee_attacks_halve_recovery_rounding_up_against_unarmed_opponents() {
        for (attacker_unarmed, defender_unarmed, ranged, expected) in [
            (false, true, false, 4.0),
            (true, true, false, 7.0),
            (false, false, false, 7.0),
            (false, true, true, 7.0),
        ] {
            let mut sim = speed_duel(attacker_unarmed, defender_unarmed, ranged);
            sim.tick();
            assert!(sim.combatants[0].state.has_attacked);
            assert_eq!(sim.combatants[0].state.next_attack_time_primary, Some(expected));
            if expected == 4.0 {
                for _ in 0..4 { sim.tick(); }
                assert_eq!(sim.combatants[0].state.next_attack_time_primary, Some(8.0),
                    "Each halved recovery rounds up to a whole second");
            }
        }
    }

    #[test]
    fn unarmed_speed_bonus_updates_after_equipment_changes_without_restarting_recovery() {
        let mut sim = speed_duel(false, true, false);
        sim.tick();
        assert_eq!(sim.combatants[0].state.next_attack_time_primary, Some(4.0));
        std::sync::Arc::make_mut(&mut sim.combatants[1].sheet.offense.weapon).is_unarmed = false;
        for _ in 0..3 { sim.tick(); }
        assert_eq!(sim.combatants[0].state.next_attack_time_primary, Some(4.0));
        sim.tick();
        assert_eq!(sim.combatants[0].state.next_attack_time_primary, Some(11.0));
    }

    #[test]
    fn unarmed_speed_bonus_applies_to_both_dualwielding_clocks() {
        use crate::core::sim::OffhandProfile;
        use std::sync::Arc;
        let mut sim = speed_duel(false, true, false);
        let attacker = &mut sim.combatants[0];
        Arc::make_mut(&mut attacker.sheet.offense.weapon).speed = 10.0;
        let mut offhand = (*attacker.sheet.offense.weapon).clone();
        offhand.speed = 6.0;
        attacker.sheet.offense.offhand = Some(OffhandProfile {
            attack_bonus: 0, strength_damage: 0, weapon: Arc::new(offhand),
        });
        attacker.sheet.maneuvers.offensive_dualwielding = true;
        sim.tick();
        assert_eq!(sim.combatants[0].state.next_attack_time_primary, Some(6.0));
        assert_eq!(sim.combatants[0].state.next_attack_time_secondary, Some(4.0));
        for _ in 0..4 { sim.tick(); }
        assert_eq!(sim.combatants[0].state.next_attack_time_secondary, Some(8.0));
    }

    #[test]
    fn unarmed_speed_bonus_keeps_called_shot_delays_and_one_second_minimum() {
        let run = |unarmed| {
            let mut sim = speed_duel(false, unarmed, false);
            sim.combatants[0].sheet.maneuvers.called_shot = true;
            for _ in 0..100 {
                sim.tick();
                if sim.combatants[0].state.has_attacked { break; }
            }
            sim.combatants[0].state.next_attack_time_primary.unwrap()
        };
        assert_eq!(run(false) - run(true), 3.0, "Only weapon recovery is halved and rounded up");
        let mut sim = speed_duel(false, true, false);
        std::sync::Arc::make_mut(&mut sim.combatants[0].sheet.offense.weapon).speed = 1.0;
        sim.tick();
        assert_eq!(sim.combatants[0].state.next_attack_time_primary, Some(1.0));
    }

    #[test]
    fn tactical_speed_comparisons_include_the_unarmed_opponent_bonus_and_temporary_modifiers() {
        let mut sim = speed_duel(false, true, false);
        let context = sim.tactical_context(0, 1);
        assert_eq!(context.my_attack_speed_seconds, 4.0);
        assert_eq!(context.enemy_attack_speed_seconds, 10.0);
        let mut effect = TemporaryEffect::new("speed_buff", 10);
        effect.modifiers.add_f32(StatIdF32::WeaponSpeed, ModifierOpF32::Add(-2.0));
        sim.combatants[0].state.active_effects.push(effect);
        assert_eq!(sim.tactical_context(0, 1).my_attack_speed_seconds, 3.0);
        sim.tick();
        assert_eq!(sim.combatants[0].state.next_attack_time_primary, Some(3.0));
    }

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
    fn melee_fighter_closes_on_traumatized_eyesmite_opponent() {
        for attacker_idx in 0..2 {
            let defender_idx = 1 - attacker_idx;
            let mut attacker = Combatant::default();
            attacker.team_id = 0;
            attacker.sheet.mobility.move_speed = 20.0;
            attacker.sheet.maneuvers.called_shot = true;
            attacker.sheet.maneuvers.called_shot_delay_profile =
                CalledShotDelayProfile::PrecisionCombatant;
            std::sync::Arc::make_mut(&mut attacker.sheet.offense.weapon).reach_ft = 2.0;

            let mut defender = Combatant::default();
            defender.team_id = 1;
            defender.sheet.defense.eyesmite = true;
            defender.sheet.vitals.infinite_hp = true;
            std::sync::Arc::make_mut(&mut defender.sheet.offense.weapon).reach_ft = 6.0;

            let mut fighters = vec![attacker, defender];
            if attacker_idx == 1 {
                fighters.swap(0, 1);
            }
            // A five-foot knockback from short-sword range leaves a seven-foot gap.
            let mut sim = SimState::with_rng(SimConfig::new(7.0, 1.0), SimRng::from_seed(1));
            sim.reset_with_combatants(fighters);
            sim.combatants[defender_idx].state.trauma_remaining_seconds = 1_440;
            let defender_position = sim.actors[defender_idx].position;

            sim.tick();
            assert_eq!(sim.distance(), 6.0);
            sim.tick();
            assert_eq!(
                sim.distance(),
                2.0,
                "the conscious fighter must close to sword reach"
            );
            assert_eq!(sim.actors[defender_idx].position, defender_position);

            for _ in 0..30 {
                sim.tick();
                if sim.combatants[attacker_idx].state.has_attacked {
                    break;
                }
            }
            assert!(sim.combatants[attacker_idx].state.has_attacked);
            assert!(sim.combatants[defender_idx].state.trauma_remaining_seconds > 1_400);
            assert!(sim.combatants[defender_idx].state.hp > 0);
        }
    }

    #[test]
    fn eyesmite_melee_approach_uses_one_movement_budget() {
        let mut attacker = Combatant::default();
        attacker.team_id = 0;
        attacker.sheet.defense.eyesmite = true;
        attacker.sheet.mobility.move_speed = 2.0;
        std::sync::Arc::make_mut(&mut attacker.sheet.offense.weapon).reach_ft = 2.0;
        let mut defender = Combatant::default();
        defender.team_id = 1;
        defender.sheet.maneuvers.passive = true;
        std::sync::Arc::make_mut(&mut defender.sheet.offense.weapon).reach_ft = 6.0;
        let mut sim = SimState::with_rng(SimConfig::new(6.0, 1.0), SimRng::from_seed(1));
        sim.reset_with_combatants(vec![attacker, defender]);

        sim.tick();
        assert_eq!(sim.distance(), 4.0);
        sim.tick();
        assert_eq!(sim.distance(), 2.0);
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
    fn called_shot_directive_pays_opening_delay_and_tracks_enemy_dr() {
        use crate::core::tactics::NumericComparison;
        for dr in [7, 8] {
            let mut attacker = Combatant::default();
            attacker.team_id = 0;
            attacker.sheet.vitals.max_hp = 1_000;
            attacker.sheet.maneuvers.called_shot_delay_profile =
                CalledShotDelayProfile::PrecisionCombatant;
            attacker.reset_state();
            let profiles = vec![tactical_profile(&attacker, Vec::new(), false, None, 9.0)];
            attacker.configure_tactical_profiles(
                TacticalPolicy {
                    enabled: true,
                    rules: vec![TacticalRule::new(
                        TacticalAction::CalledShot,
                        vec![TacticalCondition::EnemyDr {
                            comparison: NumericComparison::Greater,
                            value: 7.0,
                        }],
                    )],
                },
                profiles,
                vec![],
            );
            let mut defender = Combatant::default();
            defender.team_id = 1;
            defender.sheet.vitals.max_hp = 1_000;
            defender.sheet.defense.armor_dr = dr;
            defender.sheet.maneuvers.passive = true;
            defender.reset_state();
            let mut sim = SimState::with_rng(SimConfig::new(1.0, 1.0), SimRng::from_seed(77));
            sim.reset_with_combatants(vec![attacker, defender]);
            sim.tick();
            let attacked = |sim: &SimState| {
                sim.combat_events.iter().any(|event| {
                    event.attacker_idx == 0 && matches!(event.kind, CombatEventKind::Attack(_))
                })
            };
            assert_eq!(sim.combatants[0].sheet.maneuvers.called_shot, dr > 7);
            assert_eq!(
                attacked(&sim),
                dr <= 7,
                "Called Shot must pay its opening delay"
            );
            if dr > 7 {
                for _ in 0..40 {
                    if attacked(&sim) {
                        break;
                    }
                    sim.tick();
                }
                assert!(attacked(&sim));
                assert!(
                    sim.combatants[0].sheet.maneuvers.called_shot,
                    "profile restoration must preserve aiming mode"
                );
                sim.combatants[1].sheet.defense.armor_dr = 7;
                sim.tick();
                assert!(!sim.combatants[0].sheet.maneuvers.called_shot);
            }
        }
    }

    #[test]
    fn enabling_called_shot_during_recovery_adds_delay_only_once() {
        let mut attacker = Combatant::default();
        attacker.team_id = 0;
        attacker.tactical_policy = always_policy(TacticalAction::CalledShot);
        let mut defender = Combatant::default();
        defender.team_id = 1;
        let mut sim = SimState::with_rng(SimConfig::new(1.0, 1.0), SimRng::from_seed(77));
        sim.reset_with_combatants(vec![attacker, defender]);
        sim.combatants[0]
            .state
            .set_next_attack_time(WeaponSlot::Primary, Some(10.0));
        let added = sim.refresh_called_shot_tactic(0, 1, 1.0);
        assert!(added[0] >= 2.0);
        assert_eq!(
            sim.combatants[0].state.next_attack_time_primary,
            Some(10.0 + added[0])
        );
        assert_eq!(sim.refresh_called_shot_tactic(0, 1, 1.0), [0.0, 0.0]);
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
                    CombatEventKind::Spell(spell) => {
                        format!("{}:spell:{:?}:{}", event.time, spell.kind, spell.message)
                    }
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
#[derive(serde::Serialize, serde::Deserialize)]
pub struct DetailedSimStats {
    pub duration_p10: u32,
    pub duration_p50: u32,
    pub duration_p90: u32,
    pub duration_p99: u32,
    pub teams: Vec<DetailedTeamStats>,
}

#[derive(Clone, Debug, Default)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct DetailedTeamStats {
    pub damage_by_source: Vec<DamageSourceStats>,
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
    /// Individual attacks (including counters) that inflict at least 10 ft of knockback.
    pub knockbacks_10ft: u64,
    pub critical_hits: u64,
    pub kills: u64,
    pub eyesmite_available: bool,
    pub eyes_smote: u64,
    pub avg_attacks_per_fight: f32,
    pub direct_hit_rate: f32,
    pub contact_rate: f32,
    pub shield_block_rate: f32,
    pub hp_hit_rate: f32,
    pub knockback_10ft_rate_per_attack: f32,
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

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
struct DetailedTeamAccumulator {
    #[serde(with = "bulk_parallel::source_totals")]
    damage_by_source: BTreeMap<DamageSource, u64>,
    team_id: u8,
    wins: u32,
    attack_attempts: u64,
    direct_hits: u64,
    shield_blocks: u64,
    hp_hits: u64,
    knockbacks_10ft: u64,
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
        let mut damage_by_source: Vec<_> = self
            .damage_by_source
            .into_iter()
            .map(|(source, total_hp_damage)| DamageSourceStats {
                source,
                total_hp_damage,
                avg_hp_damage_per_fight: rate(total_hp_damage, u64::from(runs)),
                damage_share: rate(total_hp_damage, self.total_hp_damage),
                combat_dps: rate(total_hp_damage, total_seconds),
            })
            .collect();
        damage_by_source.sort_by(|a, b| {
            b.total_hp_damage
                .cmp(&a.total_hp_damage)
                .then_with(|| a.source.cmp(&b.source))
        });
        DetailedTeamStats {
            damage_by_source,
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
            knockbacks_10ft: self.knockbacks_10ft,
            critical_hits: self.critical_hits,
            kills: self.kills,
            eyesmite_available: self.eyesmite_available,
            eyes_smote: self.eyes_smote,
            avg_attacks_per_fight: rate(attempts, u64::from(runs)),
            direct_hit_rate: rate(self.direct_hits, attempts),
            contact_rate: rate(self.direct_hits + self.shield_blocks, attempts),
            shield_block_rate: rate(self.shield_blocks, attempts),
            hp_hit_rate: rate(self.hp_hits, attempts),
            knockback_10ft_rate_per_attack: rate(self.knockbacks_10ft, attempts),
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
#[derive(serde::Serialize, serde::Deserialize)]
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
    bulk_simulate_with_seed_controlled(config, combatants, runs, max_seconds, seed, |_| true)
        .expect("uninterrupted simulation")
}

/// Returning false discards the partial batch; no incomplete stats are published.
pub fn bulk_simulate_with_seed_controlled(
    config: SimConfig,
    combatants: Vec<Combatant>,
    runs: u32,
    max_seconds: u32,
    seed: u64,
    keep_running: impl FnMut(u32) -> bool,
) -> Option<BulkSimResult> {
    bulk_simulate_batch_inner(config, combatants, runs, max_seconds, seed, None, keep_running)
        .map(BulkSimBatch::finish)
}

/// Immutable inputs shared by all ranges belonging to one simulation request.
#[derive(Clone, Debug)]
pub struct PreparedBulkSimulation {
    config: SimConfig,
    fighters: Vec<Combatant>,
    context: std::sync::Arc<BulkContext>,
    positions: std::sync::Arc<[SimActor]>,
    fighter_teams: Vec<usize>,
    team_ids: Vec<u8>,
    team_has_shield: Vec<bool>,
    team_has_eyesmite: Vec<bool>,
    shields_present: bool,
}

impl PreparedBulkSimulation {
    pub fn new(config: SimConfig, combatants: Vec<Combatant>) -> Self {
        let shields_present = combatants
            .iter()
            .any(|combatant| combatant.sheet.defense.shield_name.is_some());
        let mut team_ids: Vec<u8> = combatants
            .iter()
            .map(|combatant| combatant.team_id)
            .collect();
        team_ids.sort_unstable();
        team_ids.dedup();
        let fighter_teams = combatants.iter().map(|c| team_ids.binary_search(&c.team_id).unwrap()).collect();
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
        let positions = SimState::layout(config, &combatants).into();
        let context = BulkContext::for_grid(&combatants, config.tile_size_ft).into();
        Self { config, fighters: combatants, context, positions, fighter_teams,
            team_ids, team_has_shield, team_has_eyesmite, shields_present }
    }

    pub fn run_range(&self, runs: u32, max_seconds: u32, seed: u64, first_run: u32,
        keep_running: impl FnMut(u32) -> bool) -> Option<BulkSimBatch> {
        simulate_prepared(self, runs, max_seconds, seed, Some(first_run), keep_running)
    }
}

fn bulk_simulate_batch_inner(
    config: SimConfig,
    combatants: Vec<Combatant>,
    runs: u32,
    max_seconds: u32,
    seed: u64,
    first_run: Option<u32>,
    keep_running: impl FnMut(u32) -> bool,
) -> Option<BulkSimBatch> {
    if runs == 0 {
        return Some(BulkSimBatch::default());
    }
    let prepared = PreparedBulkSimulation::new(config, combatants);
    simulate_prepared(&prepared, runs, max_seconds, seed, first_run, keep_running)
}

fn simulate_prepared(
    prepared: &PreparedBulkSimulation, runs: u32, max_seconds: u32, seed: u64,
    first_run: Option<u32>, mut keep_running: impl FnMut(u32) -> bool,
) -> Option<BulkSimBatch> {
    if runs == 0 { return Some(BulkSimBatch::default()); }
    let team_ids = &prepared.team_ids;
    let team_has_shield = &prepared.team_has_shield;
    let team_has_eyesmite = &prepared.team_has_eyesmite;
    let shields_present = prepared.shields_present;
    let mut sim = SimState::with_rng(prepared.config, SimRng::from_seed(seed));
    sim.log_events = false;
    sim.collect_detailed_metrics = true;
    sim.bulk = Some(prepared.context.clone());
    sim.initial_positions = Some(prepared.positions.clone());
    sim.combatants = prepared.fighters.clone();
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
    let mut total_max_knockback_one_side_ft = 0.0f64;
    let mut shield_breaks = 0u32;
    let mut total_hits_shield_survived = 0u64;
    let mut total_seconds = 0u64;
    let mut duration_histogram = DenseHistogram::default();
    let detailed_accumulators: Vec<DetailedTeamAccumulator> = team_ids
        .iter()
        .enumerate()
        .map(|(idx, team_id)| DetailedTeamAccumulator {
            team_id: *team_id,
            shield_fights: if team_has_shield[idx] { runs } else { 0 },
            eyesmite_available: team_has_eyesmite[idx],
            ..DetailedTeamAccumulator::default()
        })
        .collect();
    sim.bulk_metrics = Some(Box::new(BulkMetrics::new(detailed_accumulators, prepared.fighter_teams.clone())));
    let mut remaining_hp_by_team = vec![0u64; team_ids.len()];
    let mut shield_broke_by_team = vec![false; team_ids.len()];
    for run_idx in 0..runs {
        if !keep_running(run_idx) {
            return None;
        }
        if let Some(first_run) = first_run {
            sim.rng = SimRng::from_seed(bulk_parallel::fight_seed(seed, first_run.wrapping_add(run_idx)));
            // Tactics and weapon changes may leave a different sheet selected.
            // Restore the same inputs before every independently seeded fight.
            for (actor, initial) in sim.combatants.iter_mut().zip(&prepared.fighters) {
                // Alternate equipment restores its full starting profile in reset_state.
                if actor.weapon_setups.is_none() {
                    if !actor.tactical_profiles.is_empty() {
                        actor.sheet.clone_from(&initial.sheet);
                        actor.active_style_ids.clone_from(&initial.active_style_ids);
                        actor.weapon_group.clone_from(&initial.weapon_group);
                        actor.armor_type.clone_from(&initial.armor_type);
                    }
                    actor.active_fight_defensively_penalty = initial.active_fight_defensively_penalty;
                    actor.sheet.maneuvers.called_shot = initial.sheet.maneuvers.called_shot;
                }
            }
        }
        sim.reset_preserve_rng();
        for (i, actor) in sim.combatants.iter_mut().enumerate() {
            let profile = &prepared.context.prepared[i];
            actor.state.prepared = Some(if first_run.is_none() && actor.weapon_setups.is_none() && !actor.tactical_profiles.is_empty() {
                super::prepared_fighter::PreparedFighter::new(&actor.sheet, profile.knockback.weapon_adjustment).into()
            } else { profile.clone() });
        }
        sim.bulk_metrics.as_mut().expect("bulk metrics").begin_fight();
        while !sim.done && sim.elapsed_seconds < max_seconds {
            if sim.elapsed_seconds.is_multiple_of(64) && !keep_running(run_idx) {
                return None;
            }
            sim.tick();
            sim.skip_stationary_recovery(max_seconds);
        }
        let duration = sim.elapsed_seconds;
        duration_histogram.add(u64::from(duration));
        total_seconds += duration as u64;
        shortest_duration = shortest_duration.min(duration);
        longest_duration = longest_duration.max(duration);
        let mut living_team = None;
        let mut multiple_teams = false;
        let mut second_charge = false;
        let mut trauma = false;
        let mut long_knockback = false;
        let mut close_charge = false;
        let metrics = sim.bulk_metrics.as_mut().expect("bulk metrics");
        let detailed_accumulators = &mut metrics.teams;
        for team_idx in 0..team_ids.len() {
            if let Some(first_attack) = metrics.first_attack[team_idx] {
                detailed_accumulators[team_idx].first_attack_seconds_total = detailed_accumulators
                    [team_idx]
                    .first_attack_seconds_total
                    .saturating_add(u64::from(first_attack));
                detailed_accumulators[team_idx].fights_with_attack = detailed_accumulators
                    [team_idx]
                    .fights_with_attack
                    .saturating_add(1);
            }
            if metrics.trauma_inflicted[team_idx] {
                detailed_accumulators[team_idx].fights_with_trauma_inflicted =
                    detailed_accumulators[team_idx]
                        .fights_with_trauma_inflicted
                        .saturating_add(1);
            }
        }
        let mut fight_max_knockback_side = 0.0f32;
        remaining_hp_by_team.fill(0);
        shield_broke_by_team.fill(false);
        for (fighter_idx, combatant) in sim.combatants.iter().enumerate() {
            let team_idx = prepared.fighter_teams[fighter_idx];
            let state = &combatant.state;
            if state.hp > 0 {
                multiple_teams |= living_team.is_some_and(|team| team != team_idx);
                living_team = Some(team_idx);
            }
            second_charge |= state.charge_attacks >= 2;
            trauma |= state.saw_trauma;
            long_knockback |= state.max_knockback_ft >= 20.0;
            close_charge |= state.charge_started_within_20ft;
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
        for (team_idx, &shield_broke) in shield_broke_by_team.iter().enumerate() {
            if shield_broke {
                detailed_accumulators[team_idx].fights_with_shield_break = detailed_accumulators
                    [team_idx]
                    .fights_with_shield_break
                    .saturating_add(1);
            }
        }
        fights_with_second_charge += u32::from(second_charge);
        fights_with_trauma += u32::from(trauma);
        fights_with_trauma_first_exchange += u32::from(sim.trauma_first_exchange);
        fights_with_knockback_20ft += u32::from(long_knockback);
        fights_with_charge_within_20ft += u32::from(close_charge);
        if let Some(team_idx) = living_team.filter(|_| sim.done && !multiple_teams) {
            wins[team_idx] += 1;
            detailed_accumulators[team_idx].wins += 1;
            let remaining_hp = remaining_hp_by_team[team_idx];
            detailed_accumulators[team_idx].winning_hp_total = detailed_accumulators[team_idx]
                .winning_hp_total
                .saturating_add(remaining_hp);
            metrics.winning_hp[team_idx].add(remaining_hp);
            detailed_accumulators[team_idx].winning_duration_seconds_total = detailed_accumulators
                [team_idx]
                .winning_duration_seconds_total
                .saturating_add(u64::from(duration));
            metrics.winning_time[team_idx].add(u64::from(duration));
        } else {
            ties += 1;
        }
        max_total_knockback_one_side_ft =
            max_total_knockback_one_side_ft.max(fight_max_knockback_side);
        if first_run.is_some() {
            total_max_knockback_one_side_ft += f64::from(fight_max_knockback_side);
        } else {
            // Preserve the legacy API's original f32 accumulation.
            total_max_knockback_one_side_ft =
                f64::from(total_max_knockback_one_side_ft as f32 + fight_max_knockback_side);
        }
    }
    if !keep_running(runs) {
        return None;
    }
    Some(BulkSimBatch {
        runs,
        wins,
        ties,
        shields_present,
        shield_breaks,
        total_hits_shield_survived,
        total_seconds,
        shortest_duration,
        longest_duration,
        fights_with_second_charge,
        fights_with_trauma,
        fights_with_trauma_first_exchange,
        fights_with_knockback_20ft,
        fights_with_charge_within_20ft,
        instakills,
        highest_single_crit_hit,
        highest_single_noncrit_hit,
        highest_single_shield_hit,
        highest_single_crit_hit_by_team,
        highest_single_noncrit_hit_by_team,
        highest_single_shield_hit_by_team,
        instakills_by_team,
        total_damage_dealt_by_team,
        total_damage_taken_by_team,
        total_damage_rolled_dealt_by_team,
        total_damage_landed_dealt_by_team,
        damage_rolls_dealt_by_team,
        total_remaining_hp_by_team,
        max_total_knockback_one_side_ft,
        total_max_knockback_one_side_ft,
        duration_histogram: duration_histogram.finish(),
        detailed_accumulators: sim.bulk_metrics.take().expect("bulk metrics").finish(),
    })
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

#[cfg(test)]
#[path = "performance_tests.rs"]
mod performance_tests;
