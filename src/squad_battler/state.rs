//! High-level squad battler application state.

use crate::core::types::Inventory;
use crate::data;
use crate::game_logic::{ArmorCatalog, ShieldCatalog, WeaponCatalog};
use rand::Rng;
use serde::Serialize;

use super::combat::{DEFAULT_GRID_HEIGHT, DEFAULT_GRID_WIDTH, TILE_SIZE_FT};
use super::encounters::{SquadRouteNode, placeholder_route};
use super::roster::{
    MAX_ACTIVE_SQUAD, MAX_BENCH, SquadMember, SquadMemberView, roll_starting_squad,
};

const QUICK_STARTS_PATH: &str = "data/autobattler/autobattler_quick_starts.json";
const NPC_PRESETS_PATH: &str = "data/sim/npc_presets.json";

#[derive(Clone)]
pub struct SquadBattlerApp {
    weapon_catalog: WeaponCatalog,
    armor_catalog: ArmorCatalog,
    shield_catalog: ShieldCatalog,
    session: Option<SquadRun>,
}

impl SquadBattlerApp {
    pub fn new() -> Result<Self, String> {
        let (weapon_catalog, armor_catalog, shield_catalog) = data::load_catalogs()?;
        let _ = data::load_fighter_presets(QUICK_STARTS_PATH)?;
        let _ = data::load_npc_presets(NPC_PRESETS_PATH)?;
        Ok(Self {
            weapon_catalog,
            armor_catalog,
            shield_catalog,
            session: None,
        })
    }

    pub fn new_run(&mut self, seed: Option<u64>) -> SquadBattlerView {
        let seed = seed.unwrap_or_else(|| {
            let mut rng = rand::thread_rng();
            rng.gen_range(1..=u64::MAX)
        });
        let active = roll_starting_squad(
            seed,
            &self.weapon_catalog,
            &self.armor_catalog,
            &self.shield_catalog,
        );
        self.session = Some(SquadRun {
            seed,
            depth: 0,
            gold: 20,
            inventory: Inventory::default(),
            route: placeholder_route(),
            active,
            bench: Vec::new(),
            log: vec!["The company assembles at the edge of the first route.".to_string()],
        });
        self.view()
    }

    pub fn view(&self) -> SquadBattlerView {
        let Some(session) = self.session.as_ref() else {
            return SquadBattlerView {
                has_run: false,
                title: "HackMaster Squad Battler".to_string(),
                phase: "start".to_string(),
                seed: None,
                depth: 0,
                gold: 0,
                inventory: InventoryView {
                    gold: 0,
                    items: Vec::new(),
                },
                squad: SquadView {
                    active: Vec::new(),
                    bench: Vec::new(),
                    max_active: MAX_ACTIVE_SQUAD,
                    max_bench: MAX_BENCH,
                },
                grid: GridView {
                    width: DEFAULT_GRID_WIDTH,
                    height: DEFAULT_GRID_HEIGHT,
                    tile_size_ft: TILE_SIZE_FT,
                },
                route: placeholder_route(),
                log: vec!["Roll a squad to begin.".to_string()],
            };
        };
        SquadBattlerView {
            has_run: true,
            title: "HackMaster Squad Battler".to_string(),
            phase: "choose_node".to_string(),
            seed: Some(session.seed),
            depth: session.depth,
            gold: session.gold,
            inventory: InventoryView {
                gold: session.inventory.gold,
                items: session.inventory.items.clone(),
            },
            squad: SquadView {
                active: session.active.iter().map(SquadMember::view).collect(),
                bench: session.bench.iter().map(SquadMember::view).collect(),
                max_active: MAX_ACTIVE_SQUAD,
                max_bench: MAX_BENCH,
            },
            grid: GridView {
                width: DEFAULT_GRID_WIDTH,
                height: DEFAULT_GRID_HEIGHT,
                tile_size_ft: TILE_SIZE_FT,
            },
            route: session.route.clone(),
            log: session.log.clone(),
        }
    }
}

#[derive(Clone)]
struct SquadRun {
    seed: u64,
    depth: u32,
    gold: u32,
    inventory: Inventory,
    route: Vec<SquadRouteNode>,
    active: Vec<SquadMember>,
    bench: Vec<SquadMember>,
    log: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SquadBattlerView {
    pub has_run: bool,
    pub title: String,
    pub phase: String,
    pub seed: Option<u64>,
    pub depth: u32,
    pub gold: u32,
    pub inventory: InventoryView,
    pub squad: SquadView,
    pub grid: GridView,
    pub route: Vec<SquadRouteNode>,
    pub log: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SquadView {
    pub active: Vec<SquadMemberView>,
    pub bench: Vec<SquadMemberView>,
    pub max_active: usize,
    pub max_bench: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct InventoryView {
    pub gold: u32,
    pub items: Vec<String>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct GridView {
    pub width: i32,
    pub height: i32,
    pub tile_size_ft: f32,
}
