//! High-level squad battler application state.

use crate::core::gameplay::{EncounterTier, XpCurve, apply_xp};
use crate::core::ids::NpcPresetId;
use crate::core::rng::derive_seed;
use crate::core::types::Inventory;
use crate::data;
use crate::game_logic::{
    self, ArmorCatalog, NpcPresetCatalog, PlayerConfig, ShieldCatalog, TalentCatalog,
    WeaponCatalog, WeaponId,
};
use rand::Rng;
use serde::{Deserialize, Serialize};

use super::combat::{
    BattleGrid, BattleUnit, DEFAULT_GRID_HEIGHT, DEFAULT_GRID_WIDTH, SquadCombat, SquadCombatView,
    TILE_SIZE_FT,
};
use super::encounters::{SquadNodeKind, SquadRouteNode, placeholder_route};
use super::rewards::{DEFAULT_RECRUIT_OFFER_SIZE, RecruitDestination, SquadReward};
use super::roster::{
    MAX_ACTIVE_SQUAD, MAX_BENCH, SquadMember, SquadMemberStatus, SquadRoster, SquadView,
    roll_member, roll_starting_squad,
};

const QUICK_STARTS_PATH: &str = "data/autobattler/autobattler_quick_starts.json";
const NPC_PRESETS_PATH: &str = "data/sim/npc_presets.json";

#[derive(Clone)]
pub struct SquadBattlerApp {
    weapon_catalog: WeaponCatalog,
    armor_catalog: ArmorCatalog,
    shield_catalog: ShieldCatalog,
    npc_presets: NpcPresetCatalog,
    talent_catalog: TalentCatalog,
    enemy_weapon_id: WeaponId,
    xp_curve: XpCurve,
    session: Option<SquadRun>,
}

impl SquadBattlerApp {
    pub fn new() -> Result<Self, String> {
        let (weapon_catalog, armor_catalog, shield_catalog) = data::load_catalogs()?;
        let _ = data::load_fighter_presets(QUICK_STARTS_PATH)?;
        let npc_presets = data::load_npc_presets(NPC_PRESETS_PATH)?;
        let talent_catalog = data::load_talents(data::TALENTS_PATH)?;
        let enemy_weapon_id = find_weapon_id_by_name(&weapon_catalog, "Battle Axe")
            .or_else(|| weapon_catalog.first_id())
            .unwrap_or_else(|| WeaponId::new(0));
        Ok(Self {
            weapon_catalog,
            armor_catalog,
            shield_catalog,
            npc_presets,
            talent_catalog,
            enemy_weapon_id,
            xp_curve: XpCurve {
                base: 45,
                per_level: 55,
            },
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
        let roster = SquadRoster::new(active).expect("starting roster exceeds active squad limit");
        self.session = Some(SquadRun {
            seed,
            depth: 0,
            gold: 20,
            inventory: Inventory::default(),
            route: placeholder_route(),
            roster,
            phase: SquadPhase::ChoosingNode,
            pending_fight: None,
            live_fight: None,
            selected_node: None,
            last_reward: None,
            recruit_offer: Vec::new(),
            terminal: None,
            log: vec!["The company assembles at the edge of the first route.".to_string()],
        });
        self.view()
    }

    pub fn choose_node(&mut self, node_id: usize) -> Result<SquadBattlerView, String> {
        let Some(session) = self.session.as_mut() else {
            return Err("Start a run first.".to_string());
        };
        if session.phase != SquadPhase::ChoosingNode {
            return Err("Resolve the current encounter first.".to_string());
        }
        let node = session
            .route
            .iter()
            .find(|node| node.id == node_id && !node.completed)
            .cloned()
            .ok_or_else(|| "Node is not available.".to_string())?;
        let tier = match node.kind {
            SquadNodeKind::Elite => EncounterTier::Elite,
            SquadNodeKind::Boss => EncounterTier::Boss,
            _ => EncounterTier::Normal,
        };
        let enemies = generate_enemy_squad(&self.npc_presets, session.depth, tier);
        session.selected_node = Some(node_id);
        session.pending_fight = Some(PendingSquadFight {
            node_id,
            tier,
            enemies,
        });
        session.phase = SquadPhase::FightPreview;
        session.log =
            vec!["Enemy squad sighted. Review the lineup, then start combat.".to_string()];
        Ok(self.view())
    }

    pub fn start_fight(&mut self) -> Result<SquadBattlerView, String> {
        let Some(session) = self.session.as_mut() else {
            return Err("Start a run first.".to_string());
        };
        if session.phase != SquadPhase::FightPreview {
            return Err("There is no pending fight.".to_string());
        }
        let Some(pending) = session.pending_fight.take() else {
            return Err("There is no pending fight.".to_string());
        };
        let player_units = session
            .roster
            .active()
            .iter()
            .map(|member| {
                let mut combatant = game_logic::build_combatant(
                    &member.config,
                    &self.weapon_catalog,
                    &self.armor_catalog,
                    &self.shield_catalog,
                    &self.npc_presets,
                    &self.talent_catalog,
                );
                combatant.state.hp = member.current_hp.min(combatant.sheet.vitals.max_hp);
                BattleUnit::from_combatant(member.id.clone(), 0, combatant)
            })
            .collect::<Vec<_>>();
        let enemy_units = pending
            .enemies
            .iter()
            .enumerate()
            .map(|(idx, enemy)| {
                let mut config = PlayerConfig::new(&enemy.name, self.enemy_weapon_id);
                config.level = enemy.level;
                config.npc_preset = Some(enemy.preset_id);
                let combatant = game_logic::build_combatant(
                    &config,
                    &self.weapon_catalog,
                    &self.armor_catalog,
                    &self.shield_catalog,
                    &self.npc_presets,
                    &self.talent_catalog,
                );
                BattleUnit::from_combatant(
                    format!("enemy-{}-{}", pending.node_id, idx),
                    1,
                    combatant,
                )
            })
            .collect::<Vec<_>>();
        let combat_seed = derive_seed(session.seed, "squad-combat", pending.node_id as u64);
        session.live_fight = Some(SquadCombat::new_with_seed(
            player_units,
            enemy_units,
            combat_seed,
        ));
        session.phase = SquadPhase::CombatPlayback;
        session.log = vec!["The squads surge onto the marked grid.".to_string()];
        Ok(self.view())
    }

    pub fn fight_command(
        &mut self,
        command: FightCommand,
        seconds: Option<u32>,
    ) -> Result<SquadBattlerView, String> {
        let Some(session) = self.session.as_mut() else {
            return Err("Start a run first.".to_string());
        };
        let Some(fight) = session.live_fight.as_mut() else {
            return Err("There is no live fight.".to_string());
        };
        match command {
            FightCommand::Play => fight.running = true,
            FightCommand::Pause => fight.running = false,
            FightCommand::Step | FightCommand::Tick => {
                let seconds = seconds.unwrap_or(1).clamp(1, 30);
                for _ in 0..seconds {
                    fight.tick();
                    if fight.done {
                        fight.running = false;
                        break;
                    }
                }
            }
            FightCommand::Finish => {
                for _ in 0..fight.max_seconds {
                    fight.tick();
                    if fight.done {
                        fight.running = false;
                        break;
                    }
                }
            }
        }
        if session
            .live_fight
            .as_ref()
            .map(|fight| fight.done)
            .unwrap_or(false)
            && session.phase == SquadPhase::CombatPlayback
        {
            self.finalize_completed_fight();
        }
        Ok(self.view())
    }

    pub fn recruit_choice(
        &mut self,
        candidate_id: String,
        destination: RecruitDestination,
        replace_member_id: Option<String>,
    ) -> Result<SquadBattlerView, String> {
        let Some(session) = self.session.as_mut() else {
            return Err("Start a run first.".to_string());
        };
        let Some(candidate_index) = session
            .recruit_offer
            .iter()
            .position(|candidate| candidate.id == candidate_id)
        else {
            return Err("Recruit candidate not found.".to_string());
        };
        let candidate = session.recruit_offer.remove(candidate_index);
        match destination {
            RecruitDestination::Active => session
                .roster
                .add_active(candidate)
                .map_err(|err| err.to_string())?,
            RecruitDestination::Bench => session
                .roster
                .add_bench(candidate)
                .map_err(|err| err.to_string())?,
            RecruitDestination::Replace => {
                let replace_member_id = replace_member_id
                    .ok_or_else(|| "replace_member_id is required.".to_string())?;
                let replaced = session
                    .roster
                    .replace_active(&replace_member_id, candidate)
                    .map_err(|err| err.to_string())?;
                let _ = session.roster.add_bench(replaced);
            }
            RecruitDestination::Decline => {}
        }
        if session.recruit_offer.is_empty() {
            session.phase = SquadPhase::ChoosingNode;
            session.live_fight = None;
            session.selected_node = None;
            session
                .log
                .push("Recruit decisions complete. Choose the next route.".to_string());
        }
        Ok(self.view())
    }

    fn finalize_completed_fight(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let Some(fight) = session.live_fight.as_ref() else {
            return;
        };
        if !fight.done {
            return;
        }
        let won = fight.winner_team == Some(0);
        let xp = if won { 22 + session.depth * 4 } else { 0 };
        let gold = if won { 12 + session.depth * 3 } else { 0 };
        let mut level_ups = Vec::new();
        for member in session.roster.active_mut() {
            if let Some(unit) = fight.units.iter().find(|unit| unit.id == member.id) {
                if unit.hp <= 0 {
                    member.status = SquadMemberStatus::Dead;
                    member.current_hp = 0;
                } else {
                    member.current_hp = unit.hp.min(member.max_hp);
                    if xp > 0 {
                        let before = member.profile.level;
                        let result = apply_xp(&mut member.profile, &self.xp_curve, xp);
                        member.config.level = member.profile.level;
                        if result.levels_gained > 0 {
                            member.max_hp +=
                                i32::from(member.profile.level.saturating_sub(before)) * 4;
                            member.current_hp = member.max_hp;
                            level_ups.push(member.profile.name.clone());
                        }
                    }
                }
            }
        }
        let dead = session
            .roster
            .remove_dead_active()
            .into_iter()
            .map(|member| member.profile.name)
            .collect::<Vec<_>>();
        if let Some(node_id) = session.selected_node {
            if let Some(node) = session.route.iter_mut().find(|node| node.id == node_id) {
                node.completed = true;
            }
        }
        session.depth = session.depth.saturating_add(1);
        session.gold = session.gold.saturating_add(gold);
        session.inventory.add_gold(gold);
        session.last_reward = Some(SquadReward {
            gold,
            xp_per_survivor: xp,
            deaths: dead.clone(),
            level_ups: level_ups.clone(),
        });
        session.log = vec![if won {
            format!("Victory. Survivors gain {xp} XP and recover {gold} gold.")
        } else {
            "Defeat. The company is broken.".to_string()
        }];
        if !dead.is_empty() {
            session.log.push(format!("Lost: {}.", dead.join(", ")));
        }
        if !level_ups.is_empty() {
            session
                .log
                .push(format!("Level up: {}.", level_ups.join(", ")));
        }
        if session.roster.active().is_empty() {
            session.phase = SquadPhase::RunOver;
            session.terminal = Some("All active heroes are dead.".to_string());
            return;
        }
        if won {
            session.recruit_offer = generate_recruit_offer(
                session.seed,
                session.depth,
                &self.weapon_catalog,
                &self.armor_catalog,
                &self.shield_catalog,
            );
            session.phase = SquadPhase::RewardReview;
        } else {
            session.phase = SquadPhase::ChoosingNode;
            session.live_fight = None;
            session.selected_node = None;
        }
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
                available_nodes: Vec::new(),
                pending_fight: None,
                live_fight: None,
                last_reward: None,
                recruit_offer: Vec::new(),
                terminal: None,
                log: vec!["Roll a squad to begin.".to_string()],
            };
        };
        let available_nodes = if session.phase == SquadPhase::ChoosingNode {
            session
                .route
                .iter()
                .filter(|node| !node.completed)
                .map(|node| node.id)
                .collect()
        } else {
            Vec::new()
        };
        SquadBattlerView {
            has_run: true,
            title: "HackMaster Squad Battler".to_string(),
            phase: session.phase.label().to_string(),
            seed: Some(session.seed),
            depth: session.depth,
            gold: session.gold,
            inventory: InventoryView {
                gold: session.inventory.gold,
                items: session.inventory.items.clone(),
            },
            squad: session.roster.view(),
            grid: GridView {
                width: DEFAULT_GRID_WIDTH,
                height: DEFAULT_GRID_HEIGHT,
                tile_size_ft: TILE_SIZE_FT,
            },
            route: session.route.clone(),
            available_nodes,
            pending_fight: session.pending_fight.as_ref().map(PendingFightView::from),
            live_fight: session.live_fight.as_ref().map(SquadCombat::view),
            last_reward: session.last_reward.clone(),
            recruit_offer: session
                .recruit_offer
                .iter()
                .map(SquadMember::view)
                .collect(),
            terminal: session.terminal.clone(),
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
    roster: SquadRoster,
    phase: SquadPhase,
    pending_fight: Option<PendingSquadFight>,
    live_fight: Option<SquadCombat>,
    selected_node: Option<usize>,
    last_reward: Option<SquadReward>,
    recruit_offer: Vec<SquadMember>,
    terminal: Option<String>,
    log: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SquadPhase {
    ChoosingNode,
    FightPreview,
    CombatPlayback,
    RewardReview,
    RunOver,
}

impl SquadPhase {
    fn label(self) -> &'static str {
        match self {
            SquadPhase::ChoosingNode => "choose_node",
            SquadPhase::FightPreview => "fight_preview",
            SquadPhase::CombatPlayback => "combat_playback",
            SquadPhase::RewardReview => "reward_review",
            SquadPhase::RunOver => "run_over",
        }
    }
}

#[derive(Clone, Debug)]
struct PendingSquadFight {
    node_id: usize,
    tier: EncounterTier,
    enemies: Vec<EnemySquadMember>,
}

#[derive(Clone, Debug)]
struct EnemySquadMember {
    name: String,
    level: u8,
    preset_id: NpcPresetId,
}

#[derive(Clone, Debug, Serialize)]
pub struct PendingFightView {
    pub tier: String,
    pub enemy_count: usize,
    pub enemies: Vec<EnemyView>,
}

impl From<&PendingSquadFight> for PendingFightView {
    fn from(fight: &PendingSquadFight) -> Self {
        Self {
            tier: tier_label(fight.tier).to_string(),
            enemy_count: fight.enemies.len(),
            enemies: fight
                .enemies
                .iter()
                .map(|enemy| EnemyView {
                    name: enemy.name.clone(),
                    level: enemy.level,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct EnemyView {
    pub name: String,
    pub level: u8,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FightCommand {
    Play,
    Pause,
    Step,
    Tick,
    Finish,
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
    pub available_nodes: Vec<usize>,
    pub pending_fight: Option<PendingFightView>,
    pub live_fight: Option<SquadCombatView>,
    pub last_reward: Option<SquadReward>,
    pub recruit_offer: Vec<super::roster::SquadMemberView>,
    pub terminal: Option<String>,
    pub log: Vec<String>,
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

impl From<BattleGrid> for GridView {
    fn from(grid: BattleGrid) -> Self {
        Self {
            width: grid.width,
            height: grid.height,
            tile_size_ft: grid.tile_size_ft,
        }
    }
}

fn generate_enemy_squad(
    npc_presets: &NpcPresetCatalog,
    depth: u32,
    tier: EncounterTier,
) -> Vec<EnemySquadMember> {
    let size = match tier {
        EncounterTier::Normal => 2 + (depth as usize % 3),
        EncounterTier::Elite => 4 + (depth as usize % 3).min(2),
        EncounterTier::Boss => 6,
    }
    .min(6);
    let base_level = 1 + (depth / 2) as u8 + tier_level_bonus(tier);
    let entries = npc_presets.entries();
    if entries.is_empty() {
        return Vec::new();
    }
    (0..size)
        .map(|idx| {
            let preset_index = (idx + depth as usize) % entries.len();
            let preset = &entries[preset_index];
            EnemySquadMember {
                name: format!("{} {}", preset.name, idx + 1),
                level: base_level,
                preset_id: NpcPresetId::new(preset_index),
            }
        })
        .collect()
}

fn generate_recruit_offer(
    seed: u64,
    depth: u32,
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
) -> Vec<SquadMember> {
    (0..DEFAULT_RECRUIT_OFFER_SIZE)
        .map(|idx| {
            roll_member(
                format!("recruit-{}-{}", depth, idx + 1),
                derive_seed(seed, "recruit-offer", depth as u64 * 10 + idx as u64),
                weapon_catalog,
                armor_catalog,
                shield_catalog,
            )
        })
        .collect()
}

fn tier_level_bonus(tier: EncounterTier) -> u8 {
    match tier {
        EncounterTier::Normal => 0,
        EncounterTier::Elite => 1,
        EncounterTier::Boss => 2,
    }
}

fn tier_label(tier: EncounterTier) -> &'static str {
    match tier {
        EncounterTier::Normal => "Normal",
        EncounterTier::Elite => "Elite",
        EncounterTier::Boss => "Boss",
    }
}

fn find_weapon_id_by_name(catalog: &WeaponCatalog, name: &str) -> Option<WeaponId> {
    catalog
        .entries()
        .iter()
        .position(|weapon| weapon.name.eq_ignore_ascii_case(name))
        .map(WeaponId::new)
}
