//! Tactical squad combat engine.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

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
    pub initiative_ready_at: f32,
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
            initiative_ready_at: 0.0,
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
}

impl SquadCombat {
    pub fn new(mut player_units: Vec<BattleUnit>, mut enemy_units: Vec<BattleUnit>) -> Self {
        let grid = BattleGrid::default();
        spawn_team(&mut player_units, grid, 0);
        spawn_team(&mut enemy_units, grid, 1);
        let mut units = player_units;
        units.extend(enemy_units);
        Self {
            grid,
            units,
            elapsed_seconds: 0,
            max_seconds: DEFAULT_MAX_SECONDS,
            running: false,
            done: false,
            winner_team: None,
            log: vec!["Squads take their marks on the battle mat.".to_string()],
        }
    }

    pub fn tick(&mut self) {
        if self.done {
            return;
        }
        self.elapsed_seconds = self.elapsed_seconds.saturating_add(1);
        for idx in self.living_indices() {
            if let Some(target_idx) = self.nearest_enemy(idx) {
                let distance = self
                    .grid
                    .distance_ft(self.units[idx].pos, self.units[target_idx].pos);
                self.log.push(format!(
                    "t={}s: {} marks {} at {:.0} ft.",
                    self.elapsed_seconds,
                    self.units[idx].name,
                    self.units[target_idx].name,
                    distance
                ));
            }
        }
        self.refresh_done();
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
            .min_by_key(|(_, other)| unit.pos.manhattan_distance(other.pos))
            .map(|(other_idx, _)| other_idx)
    }

    pub fn occupied_positions(&self) -> HashSet<GridPos> {
        self.units
            .iter()
            .filter(|unit| unit.is_alive())
            .map(|unit| unit.pos)
            .collect()
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
            self.winner_team = None;
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
    pub initiative: f32,
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
            initiative: unit.initiative_ready_at,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct InitiativeView {
    pub combatant_id: String,
    pub name: String,
    pub team_id: u8,
    pub next_action_in_seconds: f32,
    pub ready: bool,
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
