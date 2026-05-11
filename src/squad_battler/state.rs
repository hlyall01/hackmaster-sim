//! High-level squad battler application state.

use serde::Serialize;

use super::combat::{DEFAULT_GRID_HEIGHT, DEFAULT_GRID_WIDTH, TILE_SIZE_FT};
use super::encounters::{SquadRouteNode, placeholder_route};
use super::roster::{MAX_ACTIVE_SQUAD, MAX_BENCH};

#[derive(Clone, Debug, Serialize)]
pub struct SquadBattlerApp {
    route: Vec<SquadRouteNode>,
}

impl SquadBattlerApp {
    pub fn new() -> Self {
        Self {
            route: placeholder_route(),
        }
    }

    pub fn view(&self) -> SquadBattlerView {
        SquadBattlerView {
            title: "HackMaster Squad Battler".to_string(),
            phase: "start".to_string(),
            max_active: MAX_ACTIVE_SQUAD,
            max_bench: MAX_BENCH,
            grid: GridView {
                width: DEFAULT_GRID_WIDTH,
                height: DEFAULT_GRID_HEIGHT,
                tile_size_ft: TILE_SIZE_FT,
            },
            route: self.route.clone(),
            log: vec!["Roll a squad to begin.".to_string()],
        }
    }
}

impl Default for SquadBattlerApp {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SquadBattlerView {
    pub title: String,
    pub phase: String,
    pub max_active: usize,
    pub max_bench: usize,
    pub grid: GridView,
    pub route: Vec<SquadRouteNode>,
    pub log: Vec<String>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct GridView {
    pub width: i32,
    pub height: i32,
    pub tile_size_ft: f32,
}
