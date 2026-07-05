//! Tactical squad combat engine.

use crate::core::rng::SimRng;
use crate::core::sim::{Combatant, resolve_basic_attack};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

use super::ai::{
    AiGrid, AiPosition, AiUnitInput, TieBreakMode, assign_default_role, choose_movement_intent,
    choose_target,
};

pub const DEFAULT_GRID_WIDTH: i32 = 12;
pub const DEFAULT_GRID_HEIGHT: i32 = 8;
pub const TILE_SIZE_FT: f32 = 5.0;
pub const DEFAULT_MAX_SECONDS: u32 = 180;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
}

impl GridPos {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn manhattan_distance(self, other: GridPos) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    pub fn clamp(self, grid: BattleGrid) -> Self {
        Self {
            x: self.x.clamp(0, grid.width.saturating_sub(1)),
            y: self.y.clamp(0, grid.height.saturating_sub(1)),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct BattleGrid {
    pub width: i32,
    pub height: i32,
    pub tile_size_ft: f32,
}

impl Default for BattleGrid {
    fn default() -> Self {
        Self {
            width: DEFAULT_GRID_WIDTH,
            height: DEFAULT_GRID_HEIGHT,
            tile_size_ft: TILE_SIZE_FT,
        }
    }
}

impl BattleGrid {
    pub fn distance_ft(self, a: GridPos, b: GridPos) -> f32 {
        a.manhattan_distance(b) as f32 * self.tile_size_ft
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleUnitStatus {
    Alive,
    Downed,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SquadCombatEventKind {
    Move,
    Attack,
    Miss,
    Hit,
    Death,
    Knockback,
    Skip,
    Timeout,
}

#[derive(Clone, Debug, Serialize)]
pub struct SquadCombatEvent {
    pub time: u32,
    pub kind: SquadCombatEventKind,
    pub actor_id: String,
    pub actor_name: String,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub from: Option<GridPos>,
    pub to: Option<GridPos>,
    pub damage: Option<i32>,
    pub hit: Option<bool>,
    pub remaining_hp: Option<i32>,
    pub knockback_ft: Option<f32>,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct BattleUnit {
    pub id: String,
    pub name: String,
    pub team_id: u8,
    pub pos: GridPos,
    pub hp: i32,
    pub max_hp: i32,
    pub weapon: String,
    pub reach_ft: f32,
    pub max_range_ft: Option<f32>,
    pub move_tiles: i32,
    pub current_speed_tiles: i32,
    pub initiative_ready_at: f32,
    pub intent: String,
    pub intent_target_id: Option<String>,
    pub combatant: Option<Combatant>,
}

impl BattleUnit {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        team_id: u8,
        hp: i32,
        weapon: impl Into<String>,
        reach_ft: f32,
    ) -> Self {
        let hp = hp.max(1);
        Self {
            id: id.into(),
            name: name.into(),
            team_id,
            pos: GridPos::new(0, 0),
            hp,
            max_hp: hp,
            weapon: weapon.into(),
            reach_ft,
            max_range_ft: None,
            move_tiles: 4,
            current_speed_tiles: 1,
            initiative_ready_at: 0.0,
            intent: "Forming up".to_string(),
            intent_target_id: None,
            combatant: None,
        }
    }

    pub fn from_combatant(id: impl Into<String>, team_id: u8, mut combatant: Combatant) -> Self {
        combatant.team_id = team_id;
        let weapon = combatant.sheet.offense.weapon.clone();
        let max_range_ft = weapon.range_bands_feet.map(|bands| bands[3]);
        let move_tiles = (combatant.sheet.mobility.move_speed / TILE_SIZE_FT)
            .round()
            .max(1.0) as i32;
        Self {
            id: id.into(),
            name: combatant.sheet.name.clone(),
            team_id,
            pos: GridPos::new(0, 0),
            hp: combatant.state.hp,
            max_hp: combatant.sheet.vitals.max_hp,
            weapon: weapon.name.clone(),
            reach_ft: weapon.reach_ft.max(1.0),
            max_range_ft,
            move_tiles,
            current_speed_tiles: 1,
            initiative_ready_at: 0.0,
            intent: "Forming up".to_string(),
            intent_target_id: None,
            combatant: Some(combatant),
        }
    }

    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }
}

#[derive(Clone, Debug)]
pub struct SquadCombat {
    pub grid: BattleGrid,
    pub units: Vec<BattleUnit>,
    pub elapsed_seconds: u32,
    pub max_seconds: u32,
    pub running: bool,
    pub done: bool,
    pub winner_team: Option<u8>,
    pub log: Vec<String>,
    events: Vec<SquadCombatEvent>,
    tick_start_positions: HashMap<String, GridPos>,
    moved_unit_ids: HashSet<String>,
    movement_budgets: HashMap<String, i32>,
    rng: SimRng,
}

#[derive(Clone, Copy, Debug)]
struct AttackIntent {
    attacker_idx: usize,
    defender_idx: usize,
    distance_ft: f32,
    move_after_attack: bool,
}

#[derive(Clone, Debug)]
struct TacticalSecondPlan {
    ready_indices: Vec<usize>,
    waiting_indices: Vec<usize>,
}

impl SquadCombat {
    pub fn new(player_units: Vec<BattleUnit>, enemy_units: Vec<BattleUnit>) -> Self {
        Self::new_with_seed(player_units, enemy_units, 1)
    }

    pub fn new_with_seed(
        mut player_units: Vec<BattleUnit>,
        mut enemy_units: Vec<BattleUnit>,
        seed: u64,
    ) -> Self {
        let grid = BattleGrid::default();
        spawn_team(&mut player_units, grid, 0);
        spawn_team(&mut enemy_units, grid, 1);
        Self::from_spawned_units(player_units, enemy_units, grid, seed)
    }

    pub fn new_with_seed_and_player_positions(
        mut player_units: Vec<BattleUnit>,
        mut enemy_units: Vec<BattleUnit>,
        seed: u64,
        player_positions: &[(String, GridPos)],
    ) -> Self {
        let grid = BattleGrid::default();
        spawn_team(&mut player_units, grid, 0);
        apply_team_positions(&mut player_units, grid, 0, player_positions);
        spawn_team(&mut enemy_units, grid, 1);
        Self::from_spawned_units(player_units, enemy_units, grid, seed)
    }

    fn from_spawned_units(
        player_units: Vec<BattleUnit>,
        enemy_units: Vec<BattleUnit>,
        grid: BattleGrid,
        seed: u64,
    ) -> Self {
        let mut units = player_units;
        units.extend(enemy_units);
        let mut rng = SimRng::from_seed(seed);
        roll_initial_initiative(&mut units, &mut rng);
        Self {
            grid,
            units,
            elapsed_seconds: 0,
            max_seconds: DEFAULT_MAX_SECONDS,
            running: false,
            done: false,
            winner_team: None,
            log: vec!["Squads take their marks on the battle mat.".to_string()],
            events: Vec::new(),
            tick_start_positions: HashMap::new(),
            moved_unit_ids: HashSet::new(),
            movement_budgets: HashMap::new(),
            rng,
        }
    }

    pub fn tick(&mut self) {
        if self.done {
            return;
        }
        self.advance_clock();
        self.begin_tactical_second();
        let skipped = self.tick_incapacitation();
        let plan = self.plan_tactical_second(&skipped);
        let attack_intents = self.resolve_ready_unit_actions(&plan.ready_indices);
        self.resolve_attack_phase(attack_intents);
        self.resolve_waiting_unit_movement(&plan.waiting_indices);
        self.refresh_done();
        self.end_tactical_second();
    }

    fn advance_clock(&mut self) {
        self.elapsed_seconds = self.elapsed_seconds.saturating_add(1);
    }

    fn begin_tactical_second(&mut self) {
        self.capture_tactical_start_positions();
        self.clear_tactical_movement_flags();
        self.refresh_movement_budgets();
    }

    fn capture_tactical_start_positions(&mut self) {
        self.tick_start_positions = self
            .units
            .iter()
            .map(|unit| (unit.id.clone(), unit.pos))
            .collect();
    }

    fn clear_tactical_movement_flags(&mut self) {
        self.moved_unit_ids.clear();
        for unit in &mut self.units {
            if let Some(combatant) = unit.combatant.as_mut() {
                combatant.state.moved_last_tick = false;
            }
        }
    }

    fn refresh_movement_budgets(&mut self) {
        self.movement_budgets.clear();
        for unit in &mut self.units {
            let desired = unit.move_tiles.clamp(0, 4);
            let current = unit.current_speed_tiles.clamp(1, 4);
            let next = if desired <= 0 {
                0
            } else if desired > current {
                (current + 2).min(desired).min(4)
            } else if desired < current {
                (current - 2).max(desired).max(0)
            } else {
                current
            };
            unit.current_speed_tiles = next;
            self.movement_budgets.insert(unit.id.clone(), next);
        }
    }

    fn end_tactical_second(&mut self) {
        self.tick_start_positions.clear();
        self.moved_unit_ids.clear();
        self.movement_budgets.clear();
    }

    fn plan_tactical_second(&self, skipped: &HashSet<usize>) -> TacticalSecondPlan {
        let now = self.elapsed_seconds as f32;
        let order = self.action_order();
        let ready_indices = order
            .iter()
            .copied()
            .filter(|idx| !skipped.contains(idx) && self.units[*idx].initiative_ready_at <= now)
            .collect::<Vec<_>>();
        let waiting_indices = order
            .iter()
            .copied()
            .filter(|idx| !skipped.contains(idx) && self.units[*idx].initiative_ready_at > now)
            .collect::<Vec<_>>();
        TacticalSecondPlan {
            ready_indices,
            waiting_indices,
        }
    }

    fn resolve_ready_unit_actions(&mut self, ready_indices: &[usize]) -> Vec<AttackIntent> {
        let mut intents = Vec::new();
        for &idx in ready_indices {
            if let Some(target_idx) = self.select_target(idx) {
                if let Some(intent) = self.resolve_unit_action(idx, target_idx, true) {
                    intents.push(intent);
                }
            }
        }
        intents
    }

    fn resolve_attack_phase(&mut self, attack_intents: Vec<AttackIntent>) {
        for intent in attack_intents {
            self.resolve_attack_intent(intent);
        }
    }

    fn resolve_waiting_unit_movement(&mut self, waiting_indices: &[usize]) {
        for &idx in waiting_indices {
            if let Some(target_idx) = self.select_target(idx) {
                self.resolve_unit_action(idx, target_idx, false);
            }
        }
    }

    fn action_order(&self) -> Vec<usize> {
        let now = self.elapsed_seconds as f32;
        let mut indices = self.living_indices();
        indices.sort_by(|&a, &b| {
            let a_ready = self.units[a].initiative_ready_at <= now;
            let b_ready = self.units[b].initiative_ready_at <= now;
            b_ready
                .cmp(&a_ready)
                .then_with(|| {
                    self.units[a]
                        .initiative_ready_at
                        .total_cmp(&self.units[b].initiative_ready_at)
                })
                .then_with(|| self.units[a].team_id.cmp(&self.units[b].team_id))
                .then_with(|| self.units[a].id.cmp(&self.units[b].id))
                .then_with(|| a.cmp(&b))
        });
        indices
    }

    fn tick_incapacitation(&mut self) -> HashSet<usize> {
        let mut skipped = HashSet::new();
        for idx in self.living_indices() {
            let Some(combatant) = self.units[idx].combatant.as_ref() else {
                continue;
            };
            let trauma = combatant.state.trauma_remaining_seconds;
            let knockback = combatant.state.knockback_immobile_seconds;
            if trauma > 0 || knockback > 0 {
                skipped.insert(idx);
                let reason = if trauma > 0 { "trauma" } else { "stun" };
                self.emit_simple_event(
                    SquadCombatEventKind::Skip,
                    idx,
                    None,
                    format!(
                        "t={}s: {} loses the moment to {}.",
                        self.elapsed_seconds, self.units[idx].name, reason
                    ),
                );
            }
            if let Some(combatant) = self.units[idx].combatant.as_mut() {
                if combatant.state.trauma_remaining_seconds > 0 {
                    combatant.state.trauma_remaining_seconds -= 1;
                }
                if combatant.state.knockback_immobile_seconds > 0 {
                    combatant.state.knockback_immobile_seconds -= 1;
                }
            }
        }
        skipped
    }

    pub fn living_indices(&self) -> Vec<usize> {
        self.units
            .iter()
            .enumerate()
            .filter_map(|(idx, unit)| unit.is_alive().then_some(idx))
            .collect()
    }

    pub fn nearest_enemy(&self, idx: usize) -> Option<usize> {
        let unit = self.units.get(idx)?;
        self.units
            .iter()
            .enumerate()
            .filter(|(_, other)| other.is_alive() && other.team_id != unit.team_id)
            .min_by_key(|(other_idx, other)| {
                (
                    unit.pos.manhattan_distance(other.pos),
                    other.team_id,
                    other.id.as_str(),
                    *other_idx,
                )
            })
            .map(|(other_idx, _)| other_idx)
    }

    fn select_target(&self, idx: usize) -> Option<usize> {
        self.engaged_enemy(idx)
            .or_else(|| self.ai_target(idx))
            .or_else(|| self.nearest_enemy(idx))
    }

    fn engaged_enemy(&self, idx: usize) -> Option<usize> {
        let unit = self.units.get(idx)?;
        self.units
            .iter()
            .enumerate()
            .filter(|(_, other)| other.is_alive() && other.team_id != unit.team_id)
            .filter(|(other_idx, _)| {
                self.attack_distance_between_units(idx, *other_idx)
                    .is_some()
            })
            .min_by_key(|(other_idx, other)| {
                (
                    unit.pos.manhattan_distance(other.pos),
                    other.team_id,
                    other.id.as_str(),
                    *other_idx,
                )
            })
            .map(|(other_idx, _)| other_idx)
    }

    pub fn occupied_positions(&self) -> HashSet<GridPos> {
        self.units
            .iter()
            .filter(|unit| unit.is_alive())
            .map(|unit| unit.pos)
            .collect()
    }

    fn threat_positions_for(&self, idx: usize) -> Vec<GridPos> {
        let Some(unit) = self.units.get(idx) else {
            return Vec::new();
        };
        let mut positions = vec![unit.pos];
        if self.moved_unit_ids.contains(&unit.id) {
            if let Some(start) = self.tick_start_positions.get(&unit.id).copied() {
                if start != unit.pos {
                    positions.push(start);
                }
            }
        }
        positions
    }

    fn attack_distance_between_units(
        &self,
        attacker_idx: usize,
        defender_idx: usize,
    ) -> Option<f32> {
        let attacker = self.units.get(attacker_idx)?;
        let defender = self.units.get(defender_idx)?;
        if !attacker.is_alive() || !defender.is_alive() || attacker.team_id == defender.team_id {
            return None;
        }
        self.threat_positions_for(attacker_idx)
            .into_iter()
            .flat_map(|attacker_pos| {
                self.threat_positions_for(defender_idx)
                    .into_iter()
                    .map(move |target_pos| (attacker_pos, target_pos))
            })
            .filter_map(|(attacker_pos, target_pos)| {
                self.can_attack_from(attacker, attacker_pos, target_pos)
                    .then_some(self.grid.distance_ft(attacker_pos, target_pos))
            })
            .min_by(|a, b| a.total_cmp(b))
    }

    fn resolve_unit_action(
        &mut self,
        idx: usize,
        target_idx: usize,
        allow_attack: bool,
    ) -> Option<AttackIntent> {
        if !self.units[idx].is_alive() || !self.units[target_idx].is_alive() {
            return None;
        }
        let current_distance = self
            .grid
            .distance_ft(self.units[idx].pos, self.units[target_idx].pos);
        let mut attack_distance = self.attack_distance_between_units(idx, target_idx);
        let desired_range = self.units[idx]
            .max_range_ft
            .unwrap_or_else(|| self.melee_reach_ft(&self.units[idx]))
            .max(self.melee_reach_ft(&self.units[idx]));
        let path_blocked = attack_distance.is_none()
            && self
                .path_toward_range(idx, target_idx, desired_range)
                .is_none();
        self.update_ai_intent(idx, Some(target_idx), path_blocked);

        if self.units[idx].max_range_ft.is_some()
            && current_distance <= self.melee_reach_ft(&self.units[idx])
        {
            if allow_attack {
                if let Some(distance_ft) = attack_distance {
                    self.units[idx].intent = "Take ranged shot".to_string();
                    self.units[idx].intent_target_id = Some(self.units[target_idx].id.clone());
                    return Some(AttackIntent {
                        attacker_idx: idx,
                        defender_idx: target_idx,
                        distance_ft,
                        move_after_attack: true,
                    });
                }
            }
            if self.try_move_away(idx, target_idx) {
                self.log.push(format!(
                    "t={}s: {} keeps distance from {}.",
                    self.elapsed_seconds, self.units[idx].name, self.units[target_idx].name
                ));
                attack_distance = self.attack_distance_between_units(idx, target_idx);
            }
            if allow_attack {
                let Some(distance_ft) = attack_distance else {
                    return None;
                };
                self.units[idx].intent = "Take ranged shot".to_string();
                self.units[idx].intent_target_id = Some(self.units[target_idx].id.clone());
                return Some(AttackIntent {
                    attacker_idx: idx,
                    defender_idx: target_idx,
                    distance_ft,
                    move_after_attack: false,
                });
            }
            return None;
        }

        if attack_distance.is_none() {
            self.move_toward(idx, target_idx, desired_range);
            attack_distance = self.attack_distance_between_units(idx, target_idx);
        }

        if allow_attack {
            let Some(distance_ft) = attack_distance else {
                return None;
            };
            self.update_ai_intent(idx, Some(target_idx), false);
            return Some(AttackIntent {
                attacker_idx: idx,
                defender_idx: target_idx,
                distance_ft,
                move_after_attack: false,
            });
        }
        None
    }

    fn move_toward(&mut self, mover_idx: usize, target_idx: usize, stop_distance_ft: f32) {
        let before = self.units[mover_idx].pos;
        let steps = self.movement_budget_for(mover_idx).max(0) as usize;
        let Some(path) = self.path_toward_range(mover_idx, target_idx, stop_distance_ft) else {
            return;
        };
        for next in path.into_iter().take(steps) {
            self.units[mover_idx].pos = next;
        }
        if self.units[mover_idx].pos != before {
            let after_distance = self
                .grid
                .distance_ft(self.units[mover_idx].pos, self.units[target_idx].pos);
            let message = format!(
                "t={}s: {} advances on {} ({:.0} ft).",
                self.elapsed_seconds,
                self.units[mover_idx].name,
                self.units[target_idx].name,
                after_distance
            );
            self.emit_move_event(
                mover_idx,
                Some(target_idx),
                before,
                self.units[mover_idx].pos,
                message,
            );
        }
    }

    fn try_move_away(&mut self, mover_idx: usize, target_idx: usize) -> bool {
        if self.movement_budget_for(mover_idx) <= 0 {
            return false;
        }
        let before = self.units[mover_idx].pos;
        let Some(next) = self.best_step_away(mover_idx, self.units[target_idx].pos) else {
            return false;
        };
        self.units[mover_idx].pos = next;
        self.emit_move_event(
            mover_idx,
            Some(target_idx),
            before,
            next,
            format!(
                "t={}s: {} shifts away from {}.",
                self.elapsed_seconds, self.units[mover_idx].name, self.units[target_idx].name
            ),
        );
        true
    }

    fn movement_budget_for(&self, mover_idx: usize) -> i32 {
        self.units
            .get(mover_idx)
            .and_then(|unit| self.movement_budgets.get(&unit.id).copied())
            .unwrap_or_else(|| self.units[mover_idx].move_tiles)
    }

    fn path_toward_range(
        &self,
        mover_idx: usize,
        target_idx: usize,
        stop_distance_ft: f32,
    ) -> Option<Vec<GridPos>> {
        let mover = self.units.get(mover_idx)?;
        let target = self.units.get(target_idx)?;
        let stop_tiles = self.tiles_for_distance(stop_distance_ft).max(1);
        let start = mover.pos;
        let target_pos = target.pos;
        if start.manhattan_distance(target_pos) <= stop_tiles {
            return Some(Vec::new());
        }

        let mut occupied = self.occupied_positions();
        occupied.remove(&start);
        let mut frontier = VecDeque::from([start]);
        let mut came_from: HashMap<GridPos, GridPos> = HashMap::new();
        let mut seen = HashSet::from([start]);

        while let Some(current) = frontier.pop_front() {
            let mut neighbors = self.legal_neighbors_from(current, &occupied);
            neighbors.sort_by_key(|pos| (pos.manhattan_distance(target_pos), pos.y, pos.x));
            for next in neighbors {
                if !seen.insert(next) {
                    continue;
                }
                came_from.insert(next, current);
                if next.manhattan_distance(target_pos) <= stop_tiles {
                    return Some(reconstruct_path(start, next, &came_from));
                }
                frontier.push_back(next);
            }
        }
        None
    }

    fn best_step_away(&self, mover_idx: usize, target: GridPos) -> Option<GridPos> {
        let from = self.units.get(mover_idx)?.pos;
        self.legal_neighbors(mover_idx)
            .into_iter()
            .max_by_key(|pos| pos.manhattan_distance(target))
            .filter(|pos| pos.manhattan_distance(target) > from.manhattan_distance(target))
    }

    fn legal_neighbors(&self, mover_idx: usize) -> Vec<GridPos> {
        let Some(unit) = self.units.get(mover_idx) else {
            return Vec::new();
        };
        let occupied = self.occupied_positions();
        self.legal_neighbors_from(unit.pos, &occupied)
    }

    fn legal_neighbors_from(&self, from: GridPos, occupied: &HashSet<GridPos>) -> Vec<GridPos> {
        [
            GridPos::new(from.x + 1, from.y),
            GridPos::new(from.x, from.y + 1),
            GridPos::new(from.x, from.y - 1),
            GridPos::new(from.x - 1, from.y),
        ]
        .into_iter()
        .filter(|pos| {
            pos.x >= 0
                && pos.y >= 0
                && pos.x < self.grid.width
                && pos.y < self.grid.height
                && !occupied.contains(pos)
        })
        .collect()
    }

    fn can_attack_from(
        &self,
        attacker: &BattleUnit,
        attacker_pos: GridPos,
        target_pos: GridPos,
    ) -> bool {
        let distance = self.grid.distance_ft(attacker_pos, target_pos);
        if distance <= self.melee_reach_ft(attacker) {
            return true;
        }
        attacker
            .max_range_ft
            .is_some_and(|max_range| distance <= max_range)
    }

    fn ai_target(&self, idx: usize) -> Option<usize> {
        let actor = self.ai_unit_input(idx)?;
        let role = assign_default_role(actor.role_input());
        let enemies = self
            .units
            .iter()
            .enumerate()
            .filter(|(_, unit)| unit.is_alive() && unit.team_id != actor.team_id)
            .map(|(unit_idx, _)| self.ai_unit_input(unit_idx))
            .collect::<Option<Vec<_>>>()?;
        let allies = self
            .units
            .iter()
            .enumerate()
            .filter(|(unit_idx, unit)| {
                *unit_idx != idx && unit.is_alive() && unit.team_id == actor.team_id
            })
            .map(|(unit_idx, _)| self.ai_unit_input(unit_idx))
            .collect::<Option<Vec<_>>>()?;
        let ally_target_ids = self
            .units
            .iter()
            .filter(|unit| unit.is_alive() && unit.team_id == actor.team_id)
            .filter_map(|unit| unit.intent_target_id.clone())
            .collect::<Vec<_>>();
        let score = choose_target(
            &actor,
            &enemies,
            &allies,
            &ally_target_ids,
            role,
            TieBreakMode::StableId,
            self.grid.tile_size_ft,
        )?;
        self.units
            .iter()
            .position(|unit| unit.id == score.target_id && unit.is_alive())
    }

    fn update_ai_intent(&mut self, idx: usize, target_idx: Option<usize>, path_blocked: bool) {
        let Some(actor) = self.ai_unit_input(idx) else {
            return;
        };
        let role = assign_default_role(actor.role_input());
        let allies = self
            .units
            .iter()
            .enumerate()
            .filter(|(unit_idx, unit)| {
                *unit_idx != idx && unit.is_alive() && unit.team_id == actor.team_id
            })
            .filter_map(|(unit_idx, _)| self.ai_unit_input(unit_idx))
            .collect::<Vec<_>>();
        let enemies = self
            .units
            .iter()
            .enumerate()
            .filter(|(_, unit)| unit.is_alive() && unit.team_id != actor.team_id)
            .filter_map(|(unit_idx, _)| self.ai_unit_input(unit_idx))
            .collect::<Vec<_>>();
        let target = target_idx
            .and_then(|target_idx| self.ai_unit_input(target_idx))
            .or_else(|| {
                choose_target(
                    &actor,
                    &enemies,
                    &allies,
                    &[],
                    role,
                    TieBreakMode::StableId,
                    self.grid.tile_size_ft,
                )
                .map(|score| score.target_id)
                .and_then(|target_id| enemies.iter().find(|unit| unit.id == target_id).cloned())
            });
        let intent = choose_movement_intent(
            &actor,
            target.as_ref(),
            &allies,
            &enemies,
            role,
            path_blocked,
            AiGrid::new(self.grid.width, self.grid.height),
            self.grid.tile_size_ft,
        );
        self.units[idx].intent = intent.label;
        self.units[idx].intent_target_id = intent.target_id;
    }

    fn ai_unit_input(&self, idx: usize) -> Option<AiUnitInput> {
        let unit = self.units.get(idx)?;
        Some(AiUnitInput {
            id: unit.id.clone(),
            team_id: unit.team_id,
            pos: AiPosition::new(unit.pos.x, unit.pos.y),
            hp: unit.hp,
            max_hp: unit.max_hp,
            reach_ft: self.melee_reach_ft(unit),
            max_range_ft: unit.max_range_ft,
            armor_score: armor_score(unit),
            move_tiles: unit.move_tiles,
            claimed_target_id: unit.intent_target_id.clone(),
        })
    }

    fn melee_reach_ft(&self, unit: &BattleUnit) -> f32 {
        unit.reach_ft.max(self.grid.tile_size_ft)
    }

    fn tiles_for_distance(&self, distance_ft: f32) -> i32 {
        (distance_ft / self.grid.tile_size_ft.max(0.01)).ceil() as i32
    }

    fn resolve_attack_intent(&mut self, intent: AttackIntent) {
        let attacker_idx = intent.attacker_idx;
        let defender_idx = intent.defender_idx;
        let distance_ft = intent.distance_ft;
        let now = self.elapsed_seconds as f32;
        if !self.units[defender_idx].is_alive() {
            return;
        }
        let is_ranged = self.units[attacker_idx].max_range_ft.is_some()
            && distance_ft > self.melee_reach_ft(&self.units[attacker_idx]);
        let speed = self.recovery_speed_for(attacker_idx);

        if self.units.iter().all(|unit| unit.combatant.is_some()) {
            let mut combatants = self
                .units
                .iter()
                .map(|unit| unit.combatant.clone().expect("checked combatant"))
                .collect::<Vec<_>>();
            let result = resolve_basic_attack(
                &mut combatants,
                attacker_idx,
                defender_idx,
                0,
                is_ranged,
                distance_ft,
                now,
                &mut self.rng,
            );
            for (unit, combatant) in self.units.iter_mut().zip(combatants) {
                unit.hp = combatant.state.hp;
                unit.combatant = Some(combatant);
            }
            self.units[attacker_idx].initiative_ready_at = now + speed;
            self.emit_attack_event(
                attacker_idx,
                defender_idx,
                result.event.damage,
                result.event.hit,
                result.event.knockback_ft,
                result.event.trauma_seconds,
            );
            self.apply_knockback(attacker_idx, defender_idx, result.event.knockback_ft);
            self.emit_death_if_needed(defender_idx, Some(attacker_idx));
            if let Some(counter) = result.counter_attack {
                self.emit_attack_event(
                    defender_idx,
                    attacker_idx,
                    counter.damage,
                    counter.hit,
                    counter.knockback_ft,
                    counter.trauma_seconds,
                );
                self.apply_knockback(defender_idx, attacker_idx, counter.knockback_ft);
                self.emit_death_if_needed(attacker_idx, Some(defender_idx));
            }
        } else {
            let damage = 2;
            self.units[defender_idx].hp = self.units[defender_idx].hp.saturating_sub(damage);
            self.units[attacker_idx].initiative_ready_at = now + speed;
            self.emit_attack_event(attacker_idx, defender_idx, damage, true, 0.0, None);
            self.emit_death_if_needed(defender_idx, Some(attacker_idx));
        }
        if intent.move_after_attack
            && self.units[attacker_idx].is_alive()
            && self.units[defender_idx].is_alive()
        {
            self.try_move_away(attacker_idx, defender_idx);
        }
    }

    fn recovery_speed_for(&self, unit_idx: usize) -> f32 {
        self.units
            .get(unit_idx)
            .and_then(|unit| unit.combatant.as_ref())
            .map(|combatant| combatant.sheet.offense.weapon.speed.max(1.0))
            .unwrap_or(6.0)
    }

    fn apply_knockback(&mut self, attacker_idx: usize, defender_idx: usize, knockback_ft: f32) {
        if knockback_ft <= 0.0 || !self.units[defender_idx].is_alive() {
            return;
        }
        if knockback_ft >= TILE_SIZE_FT * 2.0 {
            let reset_to = self.elapsed_seconds as f32 + self.recovery_speed_for(defender_idx);
            self.units[defender_idx].initiative_ready_at =
                self.units[defender_idx].initiative_ready_at.max(reset_to);
        }
        let tiles = self.tiles_for_distance(knockback_ft).max(0);
        let before = self.units[defender_idx].pos;
        for _ in 0..tiles {
            let next = self.step_away(self.units[defender_idx].pos, self.units[attacker_idx].pos);
            if next == self.units[defender_idx].pos || !self.is_open_for(defender_idx, next) {
                break;
            }
            self.units[defender_idx].pos = next;
        }
        let after = self.units[defender_idx].pos;
        if after != before {
            self.mark_unit_moved(defender_idx);
            let message = format!(
                "t={}s: {} is knocked back {:.0} ft by {}.",
                self.elapsed_seconds,
                self.units[defender_idx].name,
                self.grid.distance_ft(before, after),
                self.units[attacker_idx].name
            );
            self.log.push(message.clone());
            self.events.push(SquadCombatEvent {
                time: self.elapsed_seconds,
                kind: SquadCombatEventKind::Knockback,
                actor_id: self.units[defender_idx].id.clone(),
                actor_name: self.units[defender_idx].name.clone(),
                target_id: Some(self.units[attacker_idx].id.clone()),
                target_name: Some(self.units[attacker_idx].name.clone()),
                from: Some(before),
                to: Some(after),
                damage: None,
                hit: None,
                remaining_hp: Some(self.units[defender_idx].hp),
                knockback_ft: Some(self.grid.distance_ft(before, after)),
                message,
            });
        }
    }

    fn step_away(&self, from: GridPos, away_from: GridPos) -> GridPos {
        let dx = from.x - away_from.x;
        let dy = from.y - away_from.y;
        let next = if dx.abs() >= dy.abs() && dx != 0 {
            GridPos::new(from.x + dx.signum(), from.y)
        } else if dy != 0 {
            GridPos::new(from.x, from.y + dy.signum())
        } else {
            from
        };
        next.clamp(self.grid)
    }

    fn is_open_for(&self, mover_idx: usize, pos: GridPos) -> bool {
        pos.x >= 0
            && pos.y >= 0
            && pos.x < self.grid.width
            && pos.y < self.grid.height
            && self
                .units
                .iter()
                .enumerate()
                .all(|(idx, unit)| idx == mover_idx || !unit.is_alive() || unit.pos != pos)
    }

    fn emit_attack_event(
        &mut self,
        attacker_idx: usize,
        defender_idx: usize,
        damage: i32,
        hit: bool,
        knockback_ft: f32,
        trauma_seconds: Option<i32>,
    ) {
        let message = if hit {
            format!(
                "t={}s: {} hits {} for {}.",
                self.elapsed_seconds,
                self.units[attacker_idx].name,
                self.units[defender_idx].name,
                damage.max(0)
            )
        } else {
            format!(
                "t={}s: {} misses {}.",
                self.elapsed_seconds, self.units[attacker_idx].name, self.units[defender_idx].name
            )
        };
        self.log.push(message.clone());
        if let Some(seconds) = trauma_seconds {
            self.log.push(format!(
                "t={}s: {} is staggered for {}s.",
                self.elapsed_seconds, self.units[defender_idx].name, seconds
            ));
        }
        self.events.push(SquadCombatEvent {
            time: self.elapsed_seconds,
            kind: if hit {
                SquadCombatEventKind::Hit
            } else {
                SquadCombatEventKind::Miss
            },
            actor_id: self.units[attacker_idx].id.clone(),
            actor_name: self.units[attacker_idx].name.clone(),
            target_id: Some(self.units[defender_idx].id.clone()),
            target_name: Some(self.units[defender_idx].name.clone()),
            from: None,
            to: None,
            damage: Some(damage.max(0)),
            hit: Some(hit),
            remaining_hp: Some(self.units[defender_idx].hp.max(0)),
            knockback_ft: (knockback_ft > 0.0).then_some(knockback_ft),
            message,
        });
    }

    fn emit_move_event(
        &mut self,
        actor_idx: usize,
        target_idx: Option<usize>,
        from: GridPos,
        to: GridPos,
        message: String,
    ) {
        self.mark_unit_moved(actor_idx);
        self.log.push(message.clone());
        self.events.push(SquadCombatEvent {
            time: self.elapsed_seconds,
            kind: SquadCombatEventKind::Move,
            actor_id: self.units[actor_idx].id.clone(),
            actor_name: self.units[actor_idx].name.clone(),
            target_id: target_idx.map(|idx| self.units[idx].id.clone()),
            target_name: target_idx.map(|idx| self.units[idx].name.clone()),
            from: Some(from),
            to: Some(to),
            damage: None,
            hit: None,
            remaining_hp: Some(self.units[actor_idx].hp.max(0)),
            knockback_ft: None,
            message,
        });
    }

    fn mark_unit_moved(&mut self, unit_idx: usize) {
        let Some(unit) = self.units.get_mut(unit_idx) else {
            return;
        };
        self.moved_unit_ids.insert(unit.id.clone());
        if let Some(combatant) = unit.combatant.as_mut() {
            combatant.state.moved_last_tick = true;
        }
    }

    fn emit_simple_event(
        &mut self,
        kind: SquadCombatEventKind,
        actor_idx: usize,
        target_idx: Option<usize>,
        message: String,
    ) {
        self.log.push(message.clone());
        self.events.push(SquadCombatEvent {
            time: self.elapsed_seconds,
            kind,
            actor_id: self.units[actor_idx].id.clone(),
            actor_name: self.units[actor_idx].name.clone(),
            target_id: target_idx.map(|idx| self.units[idx].id.clone()),
            target_name: target_idx.map(|idx| self.units[idx].name.clone()),
            from: None,
            to: None,
            damage: None,
            hit: None,
            remaining_hp: Some(self.units[actor_idx].hp.max(0)),
            knockback_ft: None,
            message,
        });
    }

    fn emit_death_if_needed(&mut self, unit_idx: usize, source_idx: Option<usize>) {
        if self.units[unit_idx].hp > 0 {
            return;
        }
        let already_logged = self.events.iter().any(|event| {
            event.time == self.elapsed_seconds
                && matches!(event.kind, SquadCombatEventKind::Death)
                && event.actor_id == self.units[unit_idx].id
        });
        if already_logged {
            return;
        }
        self.emit_simple_event(
            SquadCombatEventKind::Death,
            unit_idx,
            source_idx,
            format!(
                "t={}s: {} drops.",
                self.elapsed_seconds, self.units[unit_idx].name
            ),
        );
    }

    pub fn view(&self) -> SquadCombatView {
        SquadCombatView {
            grid: self.grid,
            elapsed_seconds: self.elapsed_seconds,
            max_seconds: self.max_seconds,
            running: self.running,
            done: self.done,
            winner_team: self.winner_team,
            combatants: self.units.iter().map(BattleUnitView::from).collect(),
            initiative: self
                .units
                .iter()
                .filter(|unit| unit.is_alive())
                .map(|unit| InitiativeView {
                    combatant_id: unit.id.clone(),
                    name: unit.name.clone(),
                    team_id: unit.team_id,
                    next_action_in_seconds: (unit.initiative_ready_at
                        - self.elapsed_seconds as f32)
                        .max(0.0),
                    ready: unit.initiative_ready_at <= self.elapsed_seconds as f32,
                })
                .collect(),
            log_tail: self.log.iter().rev().take(20).cloned().collect(),
            events_tail: self.events.iter().rev().take(20).cloned().collect(),
        }
    }

    fn refresh_done(&mut self) {
        let mut living_teams = self
            .units
            .iter()
            .filter(|unit| unit.is_alive())
            .map(|unit| unit.team_id)
            .collect::<Vec<_>>();
        living_teams.sort_unstable();
        living_teams.dedup();
        if living_teams.len() <= 1 {
            self.done = true;
            self.winner_team = living_teams.first().copied();
        } else if self.elapsed_seconds >= self.max_seconds {
            self.done = true;
            self.winner_team = self.timeout_winner();
            let message = match self.winner_team {
                Some(team) => format!(
                    "t={}s: time expires; team {} holds the stronger field.",
                    self.elapsed_seconds, team
                ),
                None => format!(
                    "t={}s: time expires with neither squad ahead.",
                    self.elapsed_seconds
                ),
            };
            if let Some(actor_idx) = self.living_indices().first().copied() {
                self.emit_simple_event(SquadCombatEventKind::Timeout, actor_idx, None, message);
            } else {
                self.log.push(message);
            }
        }
    }

    fn timeout_winner(&self) -> Option<u8> {
        let mut strengths: HashMap<u8, (i32, i32, i32)> = HashMap::new();
        for unit in self.units.iter().filter(|unit| unit.is_alive()) {
            let entry = strengths.entry(unit.team_id).or_default();
            entry.0 += 1;
            entry.1 += unit.hp.max(0);
            entry.2 += unit.max_hp.max(0);
        }
        let mut ranked = strengths.into_iter().collect::<Vec<_>>();
        ranked.sort_by(|(team_a, score_a), (team_b, score_b)| {
            score_b.cmp(score_a).then_with(|| team_a.cmp(team_b))
        });
        match ranked.as_slice() {
            [(_, best), (_, second), ..] if best == second => None,
            [(team, _), ..] => Some(*team),
            [] => None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SquadCombatView {
    pub grid: BattleGrid,
    pub elapsed_seconds: u32,
    pub max_seconds: u32,
    pub running: bool,
    pub done: bool,
    pub winner_team: Option<u8>,
    pub combatants: Vec<BattleUnitView>,
    pub initiative: Vec<InitiativeView>,
    pub log_tail: Vec<String>,
    pub events_tail: Vec<SquadCombatEvent>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BattleUnitView {
    pub id: String,
    pub name: String,
    pub team_id: u8,
    pub x: i32,
    pub y: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub status: BattleUnitStatus,
    pub weapon: String,
    pub reach_ft: f32,
    pub max_range_ft: Option<f32>,
    pub move_tiles: i32,
    pub initiative: f32,
    pub intent: String,
}

impl From<&BattleUnit> for BattleUnitView {
    fn from(unit: &BattleUnit) -> Self {
        Self {
            id: unit.id.clone(),
            name: unit.name.clone(),
            team_id: unit.team_id,
            x: unit.pos.x,
            y: unit.pos.y,
            hp: unit.hp,
            max_hp: unit.max_hp,
            status: if unit.is_alive() {
                BattleUnitStatus::Alive
            } else {
                BattleUnitStatus::Downed
            },
            weapon: unit.weapon.clone(),
            reach_ft: unit.reach_ft,
            max_range_ft: unit.max_range_ft,
            move_tiles: unit.move_tiles,
            initiative: unit.initiative_ready_at,
            intent: unit.intent.clone(),
        }
    }
}

fn armor_score(unit: &BattleUnit) -> i32 {
    unit.combatant
        .as_ref()
        .map(|combatant| {
            combatant
                .sheet
                .defense
                .armor_dr
                .saturating_add(combatant.sheet.defense.shield_defense_bonus)
        })
        .unwrap_or(0)
}

#[derive(Clone, Debug, Serialize)]
pub struct InitiativeView {
    pub combatant_id: String,
    pub name: String,
    pub team_id: u8,
    pub next_action_in_seconds: f32,
    pub ready: bool,
}

fn reconstruct_path(
    start: GridPos,
    goal: GridPos,
    came_from: &HashMap<GridPos, GridPos>,
) -> Vec<GridPos> {
    let mut path = vec![goal];
    let mut current = goal;
    while current != start {
        let Some(previous) = came_from.get(&current).copied() else {
            return Vec::new();
        };
        current = previous;
        if current != start {
            path.push(current);
        }
    }
    path.reverse();
    path
}

fn roll_initial_initiative(units: &mut [BattleUnit], rng: &mut SimRng) {
    for unit in units {
        if unit.combatant.is_some() && unit.initiative_ready_at <= 0.0 {
            unit.initiative_ready_at = rng.gen_range(1..=10) as f32;
        }
    }
}

fn spawn_team(units: &mut [BattleUnit], grid: BattleGrid, team_id: u8) {
    let center_y = grid.height / 2;
    let start_x = if team_id == 0 { 1 } else { grid.width - 2 };
    let count = units.len();
    for (idx, unit) in units.iter_mut().enumerate() {
        unit.team_id = team_id;
        let offset = idx as i32 - (count as i32 - 1) / 2;
        unit.pos = GridPos::new(start_x, center_y + offset).clamp(grid);
    }
}

fn apply_team_positions(
    units: &mut [BattleUnit],
    grid: BattleGrid,
    team_id: u8,
    positions: &[(String, GridPos)],
) {
    let mut occupied = HashSet::new();
    for unit in units.iter_mut() {
        unit.team_id = team_id;
        let Some((_, pos)) = positions.iter().find(|(id, _)| id == &unit.id) else {
            occupied.insert(unit.pos);
            continue;
        };
        let pos = pos.clamp(grid);
        if occupied.insert(pos) {
            unit.pos = pos;
        } else {
            occupied.insert(unit.pos);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::sim::{
        Combatant, CombatantSheet, MobilityProfile, OffenseProfile, Vitals, WeaponProfile,
    };
    use std::sync::Arc;

    fn unit(id: &str, team_id: u8) -> BattleUnit {
        BattleUnit::new(id, id, team_id, 12, "Test blade", TILE_SIZE_FT)
    }

    fn tough_unit(id: &str, team_id: u8, hp: i32) -> BattleUnit {
        let mut unit = unit(id, team_id);
        unit.hp = hp;
        unit.max_hp = hp;
        unit
    }

    fn archer(id: &str, team_id: u8) -> BattleUnit {
        let mut unit = unit(id, team_id);
        unit.weapon = "Test bow".to_string();
        unit.max_range_ft = Some(TILE_SIZE_FT * 8.0);
        unit
    }

    fn combatant_unit(id: &str, team_id: u8, move_speed: f32) -> BattleUnit {
        let mut weapon = WeaponProfile::default();
        weapon.name = "Test blade".to_string();
        weapon.speed = 6.0;
        weapon.reach_ft = TILE_SIZE_FT;
        weapon.has_weapon = true;
        let sheet = CombatantSheet {
            name: id.to_string(),
            offense: OffenseProfile {
                weapon: Arc::new(weapon),
                ..OffenseProfile::default()
            },
            mobility: MobilityProfile { move_speed },
            vitals: Vitals {
                infinite_hp: false,
                max_hp: 12,
                ..Vitals::default()
            },
            ..CombatantSheet::default()
        };
        BattleUnit::from_combatant(id, team_id, Combatant::new_with_team(sheet, team_id))
    }

    fn hit_actor_ids(combat: &SquadCombat) -> Vec<&str> {
        combat
            .events
            .iter()
            .filter(|event| matches!(event.kind, SquadCombatEventKind::Hit))
            .map(|event| event.actor_id.as_str())
            .collect()
    }

    fn hit_target_ids(combat: &SquadCombat) -> Vec<&str> {
        combat
            .events
            .iter()
            .filter(|event| matches!(event.kind, SquadCombatEventKind::Hit))
            .filter_map(|event| event.target_id.as_deref())
            .collect()
    }

    fn move_distance_tiles(event: &SquadCombatEvent) -> i32 {
        let from = event.from.expect("move event should have a source square");
        let to = event
            .to
            .expect("move event should have a destination square");
        from.manhattan_distance(to)
    }

    #[test]
    fn five_foot_grid_distance_is_manhattan() {
        let grid = BattleGrid::default();
        assert_eq!(
            grid.distance_ft(GridPos::new(1, 1), GridPos::new(2, 1)),
            5.0
        );
        assert_eq!(
            grid.distance_ft(GridPos::new(1, 1), GridPos::new(4, 3)),
            25.0
        );
    }

    #[test]
    fn spawn_positions_do_not_overlap() {
        let combat = SquadCombat::new(
            vec![unit("a", 0), unit("b", 0), unit("c", 0)],
            vec![unit("x", 1), unit("y", 1), unit("z", 1)],
        );
        assert_eq!(combat.occupied_positions().len(), combat.units.len());
    }

    #[test]
    fn player_spawn_positions_can_be_overridden() {
        let combat = SquadCombat::new_with_seed_and_player_positions(
            vec![unit("a", 0), unit("b", 0)],
            vec![unit("x", 1)],
            7,
            &[
                ("a".to_string(), GridPos::new(0, 0)),
                ("b".to_string(), GridPos::new(3, 7)),
            ],
        );
        assert_eq!(combat.units[0].pos, GridPos::new(0, 0));
        assert_eq!(combat.units[1].pos, GridPos::new(3, 7));
        assert_eq!(combat.units[2].pos, GridPos::new(10, 4));
        assert_eq!(combat.occupied_positions().len(), combat.units.len());
    }

    #[test]
    fn units_move_toward_nearest_enemy_on_grid() {
        let mut combat = SquadCombat::new(vec![unit("a", 0)], vec![unit("x", 1)]);
        let start = combat.units[0].pos;
        combat.tick();
        assert!(
            combat.units[0].pos.manhattan_distance(combat.units[1].pos)
                < start.manhattan_distance(combat.units[1].pos)
        );
    }

    #[test]
    fn blocked_movement_never_overlaps_units() {
        let mut combat = SquadCombat::new(
            vec![unit("a", 0), unit("b", 0), unit("c", 0)],
            vec![unit("x", 1), unit("y", 1), unit("z", 1)],
        );
        combat.tick();
        assert_eq!(combat.occupied_positions().len(), combat.units.len());
    }

    #[test]
    fn nearest_enemy_targeting_uses_grid_distance() {
        let mut combat = SquadCombat::new(vec![unit("a", 0)], vec![unit("x", 1), unit("y", 1)]);
        combat.units[0].pos = GridPos::new(2, 2);
        combat.units[1].pos = GridPos::new(8, 2);
        combat.units[2].pos = GridPos::new(3, 2);
        assert_eq!(combat.nearest_enemy(0), Some(2));
    }

    #[test]
    fn multiple_units_can_attack_in_one_tick() {
        let mut combat = SquadCombat::new(vec![unit("a", 0), unit("b", 0)], vec![unit("x", 1)]);
        combat.units[0].pos = GridPos::new(4, 4);
        combat.units[1].pos = GridPos::new(5, 5);
        combat.units[2].pos = GridPos::new(5, 4);
        combat.tick();
        assert!(combat.units[2].hp <= 8, "expected two fallback hits");
    }

    #[test]
    fn movement_paths_around_blocking_units() {
        let mut combat = SquadCombat::new(vec![unit("a", 0), unit("b", 0)], vec![unit("x", 1)]);
        combat.units[0].pos = GridPos::new(1, 1);
        combat.units[1].pos = GridPos::new(2, 1);
        combat.units[1].move_tiles = 0;
        combat.units[2].pos = GridPos::new(4, 1);

        combat.move_toward(0, 2, TILE_SIZE_FT);

        assert_eq!(combat.units[0].pos, GridPos::new(4, 0));
        assert_eq!(combat.occupied_positions().len(), combat.units.len());
    }

    #[test]
    fn reach_weapon_attacks_without_adjacent_step() {
        let mut reach = unit("reach", 0);
        reach.reach_ft = TILE_SIZE_FT * 2.0;
        let mut enemy = unit("x", 1);
        enemy.initiative_ready_at = 99.0;
        let mut combat = SquadCombat::new(vec![reach], vec![enemy]);
        combat.units[0].pos = GridPos::new(2, 2);
        combat.units[1].pos = GridPos::new(4, 2);
        let start = combat.units[0].pos;

        combat.tick();

        assert_eq!(combat.units[0].pos, start);
        assert_eq!(combat.units[1].hp, 10);
    }

    #[test]
    fn knockback_stops_before_occupied_cells() {
        let mut combat = SquadCombat::new(
            vec![unit("attacker", 0)],
            vec![unit("defender", 1), unit("blocker", 1)],
        );
        combat.units[0].pos = GridPos::new(2, 2);
        combat.units[1].pos = GridPos::new(3, 2);
        combat.units[2].pos = GridPos::new(5, 2);

        combat.apply_knockback(0, 1, TILE_SIZE_FT * 3.0);

        assert_eq!(combat.units[1].pos, GridPos::new(4, 2));
        assert_eq!(combat.occupied_positions().len(), combat.units.len());
    }

    #[test]
    fn ready_units_act_in_deterministic_id_order() {
        let mut unit_b = unit("b", 0);
        let unit_a = unit("a", 0);
        unit_b.hp = 20;
        unit_b.max_hp = 20;
        let mut enemy = unit("x", 1);
        enemy.hp = 20;
        enemy.max_hp = 20;
        enemy.initiative_ready_at = 99.0;
        let mut combat = SquadCombat::new(vec![unit_b, unit_a], vec![enemy]);
        combat.units[0].pos = GridPos::new(4, 4);
        combat.units[1].pos = GridPos::new(5, 5);
        combat.units[2].pos = GridPos::new(5, 4);

        combat.tick();

        let attackers = combat
            .events
            .iter()
            .filter(|event| matches!(event.kind, SquadCombatEventKind::Hit))
            .map(|event| event.actor_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(attackers, vec!["a", "b"]);
    }

    #[test]
    fn timeout_awards_stronger_surviving_team() {
        let mut strong = unit("strong", 0);
        strong.hp = 12;
        strong.max_hp = 12;
        strong.move_tiles = 0;
        strong.initiative_ready_at = 99.0;
        let mut weak = unit("weak", 1);
        weak.hp = 6;
        weak.max_hp = 12;
        weak.move_tiles = 0;
        weak.initiative_ready_at = 99.0;
        let mut combat = SquadCombat::new(vec![strong], vec![weak]);
        combat.max_seconds = 1;

        combat.tick();

        assert!(combat.done);
        assert_eq!(combat.winner_team, Some(0));
    }

    #[test]
    fn tick_advances_exactly_one_second() {
        let mut player = unit("a", 0);
        let mut enemy = unit("x", 1);
        player.initiative_ready_at = 99.0;
        enemy.initiative_ready_at = 99.0;
        let mut combat = SquadCombat::new(vec![player], vec![enemy]);

        combat.tick();

        assert_eq!(combat.elapsed_seconds, 1);
    }

    #[test]
    fn simultaneous_lethal_attacks_both_land() {
        let mut player = tough_unit("a", 0, 2);
        let mut enemy = tough_unit("x", 1, 2);
        player.pos = GridPos::new(4, 4);
        enemy.pos = GridPos::new(5, 4);
        let mut combat = SquadCombat::new(vec![player], vec![enemy]);
        combat.units[0].pos = GridPos::new(4, 4);
        combat.units[1].pos = GridPos::new(5, 4);

        combat.tick();

        assert_eq!(hit_actor_ids(&combat), vec!["a", "x"]);
        assert!(
            combat.units[0].hp <= 0,
            "player attack should not be cancelled by death"
        );
        assert!(
            combat.units[1].hp <= 0,
            "enemy attack should not be cancelled by death"
        );
    }

    #[test]
    fn moving_unit_can_be_attacked_at_starting_square_until_tick_ends() {
        let mut melee = tough_unit("a", 0, 20);
        melee.move_tiles = 0;
        melee.initiative_ready_at = 1.0;
        let mut skirmisher = archer("x", 1);
        skirmisher.hp = 20;
        skirmisher.max_hp = 20;
        skirmisher.initiative_ready_at = 0.0;
        let mut combat = SquadCombat::new(vec![melee], vec![skirmisher]);
        combat.units[0].pos = GridPos::new(2, 2);
        combat.units[1].pos = GridPos::new(3, 2);

        combat.tick();

        assert!(
            combat.events.iter().any(|event| {
                matches!(event.kind, SquadCombatEventKind::Hit)
                    && event.actor_id == "a"
                    && event.target_id.as_deref() == Some("x")
            }),
            "a melee unit should be able to attack the target's old square during the same second"
        );
    }

    #[test]
    fn old_square_threat_expires_after_one_tick() {
        let mut melee = tough_unit("a", 0, 20);
        melee.move_tiles = 0;
        melee.initiative_ready_at = 1.0;
        let mut skirmisher = archer("x", 1);
        skirmisher.hp = 20;
        skirmisher.max_hp = 20;
        skirmisher.initiative_ready_at = 0.0;
        let mut combat = SquadCombat::new(vec![melee], vec![skirmisher]);
        combat.units[0].pos = GridPos::new(2, 2);
        combat.units[1].pos = GridPos::new(3, 2);

        combat.tick();
        combat.tick();

        let melee_hits = combat
            .events
            .iter()
            .filter(|event| {
                matches!(event.kind, SquadCombatEventKind::Hit)
                    && event.actor_id == "a"
                    && event.target_id.as_deref() == Some("x")
            })
            .count();
        assert_eq!(
            melee_hits, 1,
            "the target's old square should only be attackable during the second it moved"
        );
    }

    #[test]
    fn moving_unit_only_blocks_its_destination_for_movement() {
        let mut lead = unit("a", 0);
        let mut follower = unit("b", 0);
        let mut enemy = tough_unit("x", 1, 40);
        lead.move_tiles = 1;
        follower.move_tiles = 1;
        enemy.initiative_ready_at = 99.0;
        let mut combat = SquadCombat::new(vec![lead, follower], vec![enemy]);
        combat.units[0].pos = GridPos::new(2, 2);
        combat.units[1].pos = GridPos::new(1, 2);
        combat.units[2].pos = GridPos::new(7, 2);

        combat.tick();

        assert_eq!(combat.units[0].pos, GridPos::new(3, 2));
        assert_eq!(
            combat.units[1].pos,
            GridPos::new(2, 2),
            "the follower should be allowed to step into the leader's vacated square"
        );
        assert_eq!(combat.occupied_positions().len(), combat.units.len());
    }

    #[test]
    fn movement_cannot_path_through_allied_squares() {
        let mut mover = unit("a", 0);
        let mut ally = unit("b", 0);
        let enemy = unit("x", 1);
        mover.move_tiles = 4;
        ally.move_tiles = 0;
        let mut combat = SquadCombat::new(vec![mover, ally], vec![enemy]);
        combat.units[0].pos = GridPos::new(1, 1);
        combat.units[1].pos = GridPos::new(2, 1);
        combat.units[2].pos = GridPos::new(4, 1);

        combat.move_toward(0, 2, TILE_SIZE_FT);

        assert_ne!(combat.units[0].pos, GridPos::new(2, 1));
        assert_eq!(combat.units[1].pos, GridPos::new(2, 1));
    }

    #[test]
    fn movement_cannot_path_through_enemy_squares() {
        let mover = unit("a", 0);
        let blocker = unit("x", 1);
        let target = unit("y", 1);
        let mut combat = SquadCombat::new(vec![mover], vec![blocker, target]);
        combat.units[0].pos = GridPos::new(1, 1);
        combat.units[1].pos = GridPos::new(2, 1);
        combat.units[2].pos = GridPos::new(4, 1);

        combat.move_toward(0, 2, TILE_SIZE_FT);

        assert_ne!(combat.units[0].pos, GridPos::new(2, 1));
        assert_eq!(combat.units[1].pos, GridPos::new(2, 1));
    }

    #[test]
    fn contested_destination_is_awarded_by_initiative_order() {
        let mut earlier = unit("b", 0);
        let mut later = unit("a", 0);
        let mut enemy = tough_unit("x", 1, 40);
        earlier.initiative_ready_at = 0.0;
        later.initiative_ready_at = 1.0;
        earlier.move_tiles = 1;
        later.move_tiles = 1;
        enemy.initiative_ready_at = 99.0;
        let mut combat = SquadCombat::new(vec![later, earlier], vec![enemy]);
        combat.units[0].pos = GridPos::new(2, 3);
        combat.units[1].pos = GridPos::new(2, 2);
        combat.units[2].pos = GridPos::new(4, 2);

        combat.tick();

        let earlier = combat
            .units
            .iter()
            .find(|unit| unit.id == "b")
            .expect("earlier unit should exist");
        assert_eq!(earlier.pos, GridPos::new(3, 2));
        assert_eq!(combat.occupied_positions().len(), combat.units.len());
    }

    #[test]
    fn movement_does_not_trigger_opportunity_attacks() {
        let mut guard = tough_unit("a", 0, 20);
        guard.initiative_ready_at = 99.0;
        let mut skirmisher = archer("x", 1);
        skirmisher.hp = 20;
        skirmisher.max_hp = 20;
        let mut combat = SquadCombat::new(vec![guard], vec![skirmisher]);
        combat.units[0].pos = GridPos::new(2, 2);
        combat.units[1].pos = GridPos::new(3, 2);

        combat.tick();

        assert!(
            !combat.events.iter().any(|event| {
                matches!(event.kind, SquadCombatEventKind::Hit) && event.actor_id == "a"
            }),
            "leaving melee should not grant a free attack outside normal initiative"
        );
    }

    #[test]
    fn ready_unit_can_move_and_attack_in_the_same_second() {
        let mut attacker = unit("a", 0);
        let mut enemy = tough_unit("x", 1, 20);
        attacker.move_tiles = 1;
        enemy.initiative_ready_at = 99.0;
        let mut combat = SquadCombat::new(vec![attacker], vec![enemy]);
        combat.units[0].pos = GridPos::new(2, 2);
        combat.units[1].pos = GridPos::new(4, 2);

        combat.tick();

        assert!(
            combat
                .events
                .iter()
                .any(|event| matches!(event.kind, SquadCombatEventKind::Move)
                    && event.actor_id == "a")
        );
        assert!(
            combat
                .events
                .iter()
                .any(|event| matches!(event.kind, SquadCombatEventKind::Hit)
                    && event.actor_id == "a")
        );
    }

    #[test]
    fn ready_unit_can_attack_then_move_in_the_same_second() {
        let skirmisher = archer("a", 0);
        let mut enemy = tough_unit("x", 1, 20);
        enemy.initiative_ready_at = 99.0;
        let mut combat = SquadCombat::new(vec![skirmisher], vec![enemy]);
        combat.units[0].pos = GridPos::new(2, 2);
        combat.units[1].pos = GridPos::new(3, 2);

        combat.tick();

        let hit_index = combat
            .events
            .iter()
            .position(|event| {
                matches!(event.kind, SquadCombatEventKind::Hit) && event.actor_id == "a"
            })
            .expect("skirmisher should attack");
        let move_index = combat
            .events
            .iter()
            .position(|event| {
                matches!(event.kind, SquadCombatEventKind::Move) && event.actor_id == "a"
            })
            .expect("skirmisher should move");
        assert!(
            hit_index < move_index,
            "attack and movement are independent actions; this test locks the attack-then-move case"
        );
    }

    #[test]
    fn ranged_unit_uses_target_starting_square_for_same_second_range() {
        let mut archer = archer("a", 0);
        archer.move_tiles = 0;
        archer.max_range_ft = Some(TILE_SIZE_FT * 3.0);
        archer.initiative_ready_at = 1.0;
        let mut target = unit("x", 1);
        target.move_tiles = 1;
        target.initiative_ready_at = 0.0;
        let mut combat = SquadCombat::new(vec![archer], vec![target]);
        combat.units[0].pos = GridPos::new(1, 1);
        combat.units[1].pos = GridPos::new(4, 1);

        combat.tick();

        assert!(
            combat.events.iter().any(|event| {
                matches!(event.kind, SquadCombatEventKind::Hit)
                    && event.actor_id == "a"
                    && event.target_id.as_deref() == Some("x")
            }),
            "ranged attacks should be able to use the target's starting square during the movement second"
        );
    }

    #[test]
    fn moved_combatant_sets_core_moved_last_tick_for_ranged_defense() {
        let player = combatant_unit("a", 0, TILE_SIZE_FT * 4.0);
        let mut enemy = combatant_unit("x", 1, TILE_SIZE_FT * 4.0);
        enemy.initiative_ready_at = 99.0;
        let mut combat = SquadCombat::new(vec![player], vec![enemy]);
        combat.units[0].pos = GridPos::new(1, 1);
        combat.units[1].pos = GridPos::new(8, 1);

        combat.tick();

        assert!(
            combat.units[0]
                .combatant
                .as_ref()
                .expect("unit should carry core combatant")
                .state
                .moved_last_tick,
            "squad movement must update the core moved flag used by ranged defense"
        );
    }

    #[test]
    fn new_fight_rolls_initial_initiative_before_first_action() {
        let combat = SquadCombat::new_with_seed(
            vec![
                combatant_unit("a", 0, TILE_SIZE_FT),
                combatant_unit("b", 0, TILE_SIZE_FT),
            ],
            vec![
                combatant_unit("x", 1, TILE_SIZE_FT),
                combatant_unit("y", 1, TILE_SIZE_FT),
            ],
            42,
        );

        assert!(
            combat
                .units
                .iter()
                .any(|unit| unit.initiative_ready_at > 0.0),
            "initial initiative should stagger first actions instead of every unit acting at t=1"
        );
    }

    #[test]
    fn attack_recovery_uses_weapon_speed() {
        let mut attacker = unit("a", 0);
        let mut enemy = tough_unit("x", 1, 20);
        attacker.pos = GridPos::new(4, 4);
        enemy.pos = GridPos::new(5, 4);
        enemy.initiative_ready_at = 99.0;
        let mut combat = SquadCombat::new(vec![attacker], vec![enemy]);
        combat.units[0].pos = GridPos::new(4, 4);
        combat.units[1].pos = GridPos::new(5, 4);

        combat.tick();

        assert_eq!(combat.units[0].initiative_ready_at, 7.0);
    }

    #[test]
    fn movement_does_not_reset_attack_recovery_timer() {
        let mut mover = unit("a", 0);
        let enemy = unit("x", 1);
        mover.initiative_ready_at = 10.0;
        let mut combat = SquadCombat::new(vec![mover], vec![enemy]);
        combat.units[0].pos = GridPos::new(1, 1);
        combat.units[1].pos = GridPos::new(8, 1);

        combat.tick();

        assert_eq!(combat.units[0].initiative_ready_at, 10.0);
    }

    #[test]
    fn first_second_movement_cannot_jump_from_walk_to_sprint() {
        let mut mover = unit("a", 0);
        mover.move_tiles = 4;
        mover.initiative_ready_at = 99.0;
        let mut enemy = unit("x", 1);
        enemy.initiative_ready_at = 99.0;
        let mut combat = SquadCombat::new(vec![mover], vec![enemy]);
        combat.units[0].pos = GridPos::new(1, 1);
        combat.units[1].pos = GridPos::new(10, 1);

        combat.tick();

        let movement = combat
            .events
            .iter()
            .find(|event| matches!(event.kind, SquadCombatEventKind::Move) && event.actor_id == "a")
            .expect("unit should move toward the enemy");
        assert!(
            move_distance_tiles(movement) <= 3,
            "walk/jog/run/sprint acceleration allows walk -> run, not immediate sprint"
        );
    }

    #[test]
    fn ranged_role_kites_when_engaged() {
        let skirmisher = archer("a", 0);
        let mut enemy = tough_unit("x", 1, 20);
        enemy.initiative_ready_at = 99.0;
        enemy.move_tiles = 0;
        let mut combat = SquadCombat::new(vec![skirmisher], vec![enemy]);
        combat.units[0].pos = GridPos::new(2, 2);
        combat.units[1].pos = GridPos::new(3, 2);
        let start_distance = combat.units[0].pos.manhattan_distance(combat.units[1].pos);

        combat.tick();

        assert!(
            combat.units[0].pos.manhattan_distance(combat.units[1].pos) > start_distance,
            "ranged roles should open distance when engaged"
        );
    }

    #[test]
    fn ai_focuses_low_hp_targets_for_ranged_roles() {
        let actor = archer("a", 0);
        let healthy = tough_unit("x", 1, 20);
        let wounded = tough_unit("y", 1, 4);
        let mut combat = SquadCombat::new(vec![actor], vec![healthy, wounded]);
        combat.units[0].pos = GridPos::new(1, 1);
        combat.units[1].pos = GridPos::new(3, 1);
        combat.units[2].pos = GridPos::new(4, 1);
        combat.units[2].max_hp = 20;

        assert_eq!(combat.ai_target(0), Some(2));
    }

    #[test]
    fn zero_hp_units_render_as_downed() {
        let mut combat = SquadCombat::new(vec![unit("a", 0)], vec![unit("x", 1)]);
        combat.units[1].hp = 0;

        let view = combat.view();

        let enemy = view
            .combatants
            .iter()
            .find(|unit| unit.id == "x")
            .expect("enemy should be visible in combat view");
        assert_eq!(enemy.status, BattleUnitStatus::Downed);
    }

    #[test]
    fn downed_units_do_not_block_movement() {
        let mover = unit("a", 0);
        let mut downed = unit("b", 0);
        let enemy = unit("x", 1);
        downed.hp = 0;
        let mut combat = SquadCombat::new(vec![mover, downed], vec![enemy]);
        combat.units[0].pos = GridPos::new(1, 1);
        combat.units[1].pos = GridPos::new(2, 1);
        combat.units[2].pos = GridPos::new(4, 1);

        combat.move_toward(0, 2, TILE_SIZE_FT);

        assert!(
            combat.units[0].pos.x > 2,
            "movement should be able to pass through a downed unit's square"
        );
    }

    #[test]
    fn downed_units_are_not_valid_attack_targets() {
        let mut attacker = unit("a", 0);
        let mut downed = unit("x", 1);
        let living = tough_unit("y", 1, 20);
        attacker.move_tiles = 0;
        downed.hp = 0;
        let mut combat = SquadCombat::new(vec![attacker], vec![downed, living]);
        combat.units[0].pos = GridPos::new(2, 2);
        combat.units[1].pos = GridPos::new(3, 2);
        combat.units[2].pos = GridPos::new(2, 3);

        combat.tick();

        assert!(!hit_target_ids(&combat).contains(&"x"));
        assert!(hit_target_ids(&combat).contains(&"y"));
    }

    #[test]
    fn large_knockback_resets_squad_initiative_timer() {
        let attacker = unit("a", 0);
        let mut defender = unit("x", 1);
        defender.initiative_ready_at = 1.0;
        let mut combat = SquadCombat::new(vec![attacker], vec![defender]);
        combat.elapsed_seconds = 1;
        combat.units[0].pos = GridPos::new(2, 2);
        combat.units[1].pos = GridPos::new(3, 2);

        combat.apply_knockback(0, 1, TILE_SIZE_FT * 2.0);

        assert!(
            combat.units[1].initiative_ready_at > combat.elapsed_seconds as f32,
            "large knockback should reset the defender's next attack timing in the squad layer"
        );
    }
}
