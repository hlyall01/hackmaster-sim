//! High-level squad battler application state.

use crate::core::gameplay::{EncounterTier, XpCurve};
use crate::core::ids::NpcPresetId;
use crate::core::rng::{SimRng, derive_seed};
use crate::core::types::Inventory;
use crate::data;
use crate::game_logic::{
    self, ArmorCatalog, NpcPresetCatalog, PlayerConfig, ShieldCatalog, TalentCatalog,
    WeaponCatalog, WeaponId,
};
use rand::Rng;
use serde::{Deserialize, Serialize};

use super::combat::{
    BattleGrid, BattleUnit, DEFAULT_GRID_HEIGHT, DEFAULT_GRID_WIDTH, GridPos, SquadCombat,
    SquadCombatView, TILE_SIZE_FT,
};
use super::encounters::{
    SquadEncounterTier, SquadEventOutcomeKind, SquadNodeKind, SquadRouteNode, available_node_ids,
    generate_route, placeholder_route, route_status,
};
use super::rewards::{DEFAULT_RECRUIT_OFFER_SIZE, RecruitDestination, SquadReward};
use super::roster::{
    MAX_ACTIVE_SQUAD, MAX_BENCH, SquadMember, SquadMemberStatus, SquadRoster, SquadView,
    roll_recruit_member, roll_starting_squad,
};

const QUICK_STARTS_PATH: &str = "data/autobattler/autobattler_quick_starts.json";
const NPC_PRESETS_PATH: &str = "data/sim/npc_presets.json";
const PLAYER_DEPLOYMENT_COLUMNS: i32 = 4;

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
        let mut active = roll_starting_squad(
            seed,
            &self.weapon_catalog,
            &self.armor_catalog,
            &self.shield_catalog,
        );
        sync_members_to_combat_vitals(
            &mut active,
            &self.weapon_catalog,
            &self.armor_catalog,
            &self.shield_catalog,
            &self.npc_presets,
            &self.talent_catalog,
        );
        let roster = SquadRoster::new(active).expect("starting roster exceeds active squad limit");
        let formation = default_formation_for_members(roster.active());
        self.session = Some(SquadRun {
            seed,
            depth: 0,
            gold: 20,
            inventory: Inventory {
                gold: 20,
                ..Inventory::default()
            },
            route: generate_route(seed),
            roster,
            formation,
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
        let available = available_node_ids(&session.route, session.depth);
        if !available.contains(&node_id) {
            return Err("Node is not available on the current floor.".to_string());
        }
        let node = session
            .route
            .iter()
            .find(|node| node.id == node_id && !node.completed)
            .cloned()
            .ok_or_else(|| "Node is not available.".to_string())?;
        match node.kind {
            SquadNodeKind::Recruit => {
                complete_route_node(session, node_id);
                session.selected_node = Some(node_id);
                session.recruit_offer = generate_recruit_offer(
                    session.seed,
                    session.depth,
                    encounter_tier(node.tier),
                    node_id as u64,
                    &self.weapon_catalog,
                    &self.armor_catalog,
                    &self.shield_catalog,
                    &self.npc_presets,
                    &self.talent_catalog,
                );
                session.phase = SquadPhase::RewardReview;
                session.log = vec!["A band of mercenaries offers to join the company.".to_string()];
            }
            SquadNodeKind::Rest => {
                let recovered = resolve_rest_node(session, &node);
                complete_route_node(session, node_id);
                session.selected_node = None;
                session.phase = next_route_phase(session);
                session.log = vec![format!(
                    "Camp secured. {recovered} company members recover."
                )];
            }
            SquadNodeKind::Event => {
                let offers_recruits = resolve_event_node(session, &node, &self.xp_curve);
                complete_route_node(session, node_id);
                if offers_recruits {
                    session.recruit_offer = generate_recruit_offer(
                        session.seed,
                        session.depth,
                        EncounterTier::Normal,
                        node_id as u64,
                        &self.weapon_catalog,
                        &self.armor_catalog,
                        &self.shield_catalog,
                        &self.npc_presets,
                        &self.talent_catalog,
                    );
                }
                session.selected_node = Some(node_id);
                session.phase = if session.recruit_offer.is_empty() {
                    next_route_phase(session)
                } else {
                    SquadPhase::RewardReview
                };
            }
            SquadNodeKind::Fight | SquadNodeKind::Elite | SquadNodeKind::Boss => {
                let tier = encounter_tier(node.tier);
                let enemies = generate_enemy_squad(&self.npc_presets, &node);
                session.selected_node = Some(node_id);
                session.pending_fight = Some(PendingSquadFight {
                    node_id,
                    tier,
                    enemies,
                });
                session.phase = SquadPhase::FightPreview;
                session.log =
                    vec!["Enemy squad sighted. Review the lineup, then start combat.".to_string()];
            }
        }
        Ok(self.view())
    }

    pub fn start_fight(&mut self) -> Result<SquadBattlerView, String> {
        let Some(session) = self.session.as_mut() else {
            return Err("Start a run first.".to_string());
        };
        if session.phase != SquadPhase::FightPreview {
            return Err("There is no pending fight.".to_string());
        }
        sync_formation_for_active(session);
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
        let player_positions = session
            .formation
            .iter()
            .map(|slot| (slot.member_id.clone(), slot.pos))
            .collect::<Vec<_>>();
        session.live_fight = Some(SquadCombat::new_with_seed_and_player_positions(
            player_units,
            enemy_units,
            combat_seed,
            &player_positions,
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
            FightCommand::SkipToNextInitiative => {
                let seconds = fight_seconds_to_next_initiative(fight).clamp(1, 30);
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
                session
                    .roster
                    .replace_active_and_bench_replaced(&replace_member_id, candidate)
                    .map_err(|err| err.to_string())?;
            }
            RecruitDestination::Decline => {}
        }
        if session.recruit_offer.is_empty() {
            session.phase = next_route_phase(session);
            session.live_fight = None;
            session.selected_node = None;
            session
                .log
                .push("Recruit decisions complete. Choose the next route.".to_string());
        }
        sync_formation_for_active(session);
        Ok(self.view())
    }

    pub fn set_formation_position(
        &mut self,
        member_id: String,
        x: i32,
        y: i32,
    ) -> Result<SquadBattlerView, String> {
        let Some(session) = self.session.as_mut() else {
            return Err("Start a run first.".to_string());
        };
        if session.phase != SquadPhase::FightPreview {
            return Err("Formation can only be changed while scouting a fight.".to_string());
        }
        let pos = validate_player_deployment_pos(GridPos::new(x, y))?;
        if !session
            .roster
            .active()
            .iter()
            .any(|member| member.id == member_id)
        {
            return Err("Formation member is not in the active squad.".to_string());
        }
        sync_formation_for_active(session);
        move_formation_member(session, &member_id, pos)?;
        session.log = vec!["Formation updated.".to_string()];
        Ok(self.view())
    }

    pub fn roster_swap(
        &mut self,
        active_member_id: String,
        bench_member_id: String,
    ) -> Result<SquadBattlerView, String> {
        let Some(session) = self.session.as_mut() else {
            return Err("Start a run first.".to_string());
        };
        if session.phase == SquadPhase::CombatPlayback || session.phase == SquadPhase::FightPreview
        {
            return Err("Roster changes are locked during combat.".to_string());
        }
        session
            .roster
            .swap_bench_to_active(&active_member_id, &bench_member_id)
            .map_err(|err| err.to_string())?;
        sync_formation_for_active(session);
        session.log.push("Roster positions swapped.".to_string());
        Ok(self.view())
    }

    pub fn roster_promote(&mut self, bench_member_id: String) -> Result<SquadBattlerView, String> {
        let Some(session) = self.session.as_mut() else {
            return Err("Start a run first.".to_string());
        };
        if session.phase == SquadPhase::CombatPlayback || session.phase == SquadPhase::FightPreview
        {
            return Err("Roster changes are locked during combat.".to_string());
        }
        session
            .roster
            .promote_bench_to_active(&bench_member_id)
            .map_err(|err| err.to_string())?;
        sync_formation_for_active(session);
        session
            .log
            .push("Bench member promoted to active squad.".to_string());
        Ok(self.view())
    }

    pub fn roster_dismiss(&mut self, bench_member_id: String) -> Result<SquadBattlerView, String> {
        let Some(session) = self.session.as_mut() else {
            return Err("Start a run first.".to_string());
        };
        if session.phase == SquadPhase::CombatPlayback || session.phase == SquadPhase::FightPreview
        {
            return Err("Roster changes are locked during combat.".to_string());
        }
        let dismissed = session
            .roster
            .dismiss_bench(&bench_member_id)
            .map_err(|err| err.to_string())?;
        session
            .log
            .push(format!("{} leaves the company.", dismissed.profile.name));
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
        let selected_node = session
            .selected_node
            .and_then(|node_id| session.route.iter().find(|node| node.id == node_id))
            .cloned();
        let node_reward = selected_node.as_ref().map(|node| node.reward.clone());
        let xp = if won {
            node_reward
                .as_ref()
                .map(|reward| reward.xp_per_survivor)
                .unwrap_or_else(|| 22 + session.depth * 4)
        } else {
            0
        };
        let gold = if won {
            selected_node
                .as_ref()
                .map(|node| roll_gold_for_node(session.seed, node))
                .unwrap_or_else(|| 12 + session.depth * 3)
        } else {
            0
        };
        let mut level_ups = Vec::new();
        for member in session.roster.active_mut() {
            if let Some(unit) = fight.units.iter().find(|unit| unit.id == member.id) {
                if unit.hp <= 0 {
                    member.status = SquadMemberStatus::Dead;
                    member.current_hp = 0;
                } else {
                    let _ = member.apply_post_fight_injury(unit.hp.min(member.max_hp));
                    if xp > 0 {
                        let result = member.award_xp(&self.xp_curve, xp);
                        if result.levels_gained > 0 {
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
        sync_formation_for_active(session);
        if let Some(node_id) = session.selected_node {
            complete_route_node(session, node_id);
        }
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
            let route_complete = selected_node
                .as_ref()
                .is_some_and(|node| node.kind == SquadNodeKind::Boss);
            let recruit_chance = node_reward
                .as_ref()
                .map(|reward| reward.recruit_chance_percent)
                .unwrap_or(20);
            let recruit_roll = selected_node
                .as_ref()
                .map(|node| roll_percent(session.seed, "post-fight-recruit", node.id as u64))
                .unwrap_or(100);
            if !route_complete && recruit_roll < recruit_chance {
                let tier = selected_node
                    .as_ref()
                    .map(|node| encounter_tier(node.tier))
                    .unwrap_or(EncounterTier::Normal);
                session.recruit_offer = generate_recruit_offer(
                    session.seed,
                    session.depth,
                    tier,
                    selected_node
                        .as_ref()
                        .map(|node| node.id as u64)
                        .unwrap_or(0),
                    &self.weapon_catalog,
                    &self.armor_catalog,
                    &self.shield_catalog,
                    &self.npc_presets,
                    &self.talent_catalog,
                );
            } else {
                session.recruit_offer.clear();
            }
            session.phase = if route_complete {
                session.terminal = Some("The boss is beaten. The route is complete.".to_string());
                SquadPhase::RunOver
            } else if session.recruit_offer.is_empty() {
                session.live_fight = None;
                session.selected_node = None;
                next_route_phase(session)
            } else {
                SquadPhase::RewardReview
            };
        } else {
            session.phase = SquadPhase::RunOver;
            session.terminal = Some("The company breaks after defeat.".to_string());
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
                formation: SquadFormationView {
                    deployment_columns: PLAYER_DEPLOYMENT_COLUMNS,
                    slots: Vec::new(),
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
            route_status(&session.route, session.depth).available_nodes
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
            formation: formation_view(&session.formation),
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
    formation: Vec<FormationSlot>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct FormationSlot {
    member_id: String,
    pos: GridPos,
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
    SkipToNextInitiative,
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
    pub formation: SquadFormationView,
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

#[derive(Clone, Debug, Serialize)]
pub struct SquadFormationView {
    pub deployment_columns: i32,
    pub slots: Vec<FormationSlotView>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FormationSlotView {
    pub member_id: String,
    pub x: i32,
    pub y: i32,
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

fn formation_view(formation: &[FormationSlot]) -> SquadFormationView {
    SquadFormationView {
        deployment_columns: PLAYER_DEPLOYMENT_COLUMNS,
        slots: formation
            .iter()
            .map(|slot| FormationSlotView {
                member_id: slot.member_id.clone(),
                x: slot.pos.x,
                y: slot.pos.y,
            })
            .collect(),
    }
}

fn default_formation_for_members(members: &[SquadMember]) -> Vec<FormationSlot> {
    let grid = BattleGrid::default();
    let count = members.len();
    let center_y = grid.height / 2;
    members
        .iter()
        .enumerate()
        .map(|(idx, member)| {
            let offset = idx as i32 - (count as i32 - 1) / 2;
            FormationSlot {
                member_id: member.id.clone(),
                pos: GridPos::new(1, center_y + offset)
                    .clamp(grid)
                    .clamp(BattleGrid {
                        width: PLAYER_DEPLOYMENT_COLUMNS,
                        ..grid
                    }),
            }
        })
        .collect()
}

fn sync_formation_for_active(session: &mut SquadRun) {
    let active = session
        .roster
        .active()
        .iter()
        .map(|member| member.id.clone())
        .collect::<Vec<_>>();
    session
        .formation
        .retain(|slot| active.iter().any(|member_id| member_id == &slot.member_id));

    let mut occupied = session
        .formation
        .iter()
        .map(|slot| slot.pos)
        .collect::<std::collections::HashSet<_>>();
    for member_id in active {
        if session
            .formation
            .iter()
            .any(|slot| slot.member_id == member_id)
        {
            continue;
        }
        let pos = first_open_formation_pos(&occupied);
        occupied.insert(pos);
        session.formation.push(FormationSlot { member_id, pos });
    }
}

fn validate_player_deployment_pos(pos: GridPos) -> Result<GridPos, String> {
    if pos.x < 0 || pos.x >= PLAYER_DEPLOYMENT_COLUMNS || pos.y < 0 || pos.y >= DEFAULT_GRID_HEIGHT
    {
        return Err(format!(
            "Formation position must be inside the first {} columns of the grid.",
            PLAYER_DEPLOYMENT_COLUMNS
        ));
    }
    Ok(pos)
}

fn move_formation_member(
    session: &mut SquadRun,
    member_id: &str,
    pos: GridPos,
) -> Result<(), String> {
    let Some(index) = session
        .formation
        .iter()
        .position(|slot| slot.member_id == member_id)
    else {
        return Err("Formation member is not in the active squad.".to_string());
    };
    let old_pos = session.formation[index].pos;
    if let Some(other_index) = session
        .formation
        .iter()
        .position(|slot| slot.member_id != member_id && slot.pos == pos)
    {
        session.formation[other_index].pos = old_pos;
    }
    session.formation[index].pos = pos;
    Ok(())
}

fn first_open_formation_pos(occupied: &std::collections::HashSet<GridPos>) -> GridPos {
    let grid = BattleGrid::default();
    let center_y = grid.height / 2;
    let mut candidates = (0..grid.height)
        .flat_map(|y| (0..PLAYER_DEPLOYMENT_COLUMNS).map(move |x| GridPos::new(x, y)))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|pos| ((pos.y - center_y).abs(), (pos.x - 1).abs(), pos.y, pos.x));
    candidates
        .into_iter()
        .find(|pos| !occupied.contains(pos))
        .unwrap_or_else(|| GridPos::new(0, 0))
}

fn complete_route_node(session: &mut SquadRun, node_id: usize) {
    if let Some(node) = session.route.iter_mut().find(|node| node.id == node_id) {
        node.completed = true;
    }
    session.depth = session.depth.saturating_add(1);
}

fn next_route_phase(session: &mut SquadRun) -> SquadPhase {
    if session.roster.active().is_empty() {
        session.terminal = Some("All active heroes are dead.".to_string());
        return SquadPhase::RunOver;
    }
    if route_status(&session.route, session.depth).kind
        == super::encounters::SquadRouteStatusKind::Complete
    {
        session.terminal = Some("The route is complete.".to_string());
        return SquadPhase::RunOver;
    }
    SquadPhase::ChoosingNode
}

fn resolve_rest_node(session: &mut SquadRun, node: &SquadRouteNode) -> usize {
    let heal_percent = i32::from(
        node.rest
            .as_ref()
            .map(|rest| rest.heal_percent)
            .unwrap_or(30),
    );
    let wound_days = if node
        .rest
        .as_ref()
        .map(|rest| rest.clears_downed)
        .unwrap_or(false)
    {
        2
    } else {
        1
    };
    let reports = session.roster.recover_after_fight(
        (10 + i32::try_from(session.depth).unwrap_or(0)).max(1) * heal_percent / 30,
        wound_days,
    );
    reports
        .into_iter()
        .filter(|report| report.hp_recovered > 0 || !report.wounds_healed.is_empty())
        .count()
}

fn resolve_event_node(session: &mut SquadRun, node: &SquadRouteNode, xp_curve: &XpCurve) -> bool {
    let Some(event) = node.event.as_ref() else {
        session.log = vec!["The company finds nothing of consequence.".to_string()];
        return false;
    };
    let outcome = &event.outcome;
    if outcome.gold_delta >= 0 {
        let gold = outcome.gold_delta as u32;
        session.gold = session.gold.saturating_add(gold);
        session.inventory.add_gold(gold);
    } else {
        let loss = outcome.gold_delta.unsigned_abs();
        session.gold = session.gold.saturating_sub(loss);
        session.inventory.gold = session.inventory.gold.saturating_sub(loss);
    }

    let mut level_ups = Vec::new();
    if outcome.xp_bonus > 0 {
        for member in session.roster.active_mut() {
            if member.status == SquadMemberStatus::Ready {
                let report = member.award_xp(xp_curve, outcome.xp_bonus);
                if report.levels_gained > 0 {
                    level_ups.push(member.profile.name.clone());
                }
            }
        }
    }

    session.last_reward = Some(SquadReward {
        gold: outcome.gold_delta.max(0) as u32,
        xp_per_survivor: outcome.xp_bonus,
        deaths: Vec::new(),
        level_ups,
    });
    session.log = vec![format!("{}: {}", event.title, outcome.label)];
    outcome.kind == SquadEventOutcomeKind::RecruitLead || outcome.recruit_bonus_percent >= 25
}

fn generate_enemy_squad(
    npc_presets: &NpcPresetCatalog,
    node: &SquadRouteNode,
) -> Vec<EnemySquadMember> {
    let template = node
        .enemy_template
        .clone()
        .unwrap_or_else(|| super::encounters::enemy_squad_template(node.tier, node.required_depth));
    let size = template.squad_size.min(6);
    let base_level = template.base_level;
    let entries = npc_presets.entries();
    if entries.is_empty() {
        return Vec::new();
    }
    (0..size)
        .map(|idx| {
            let preset_index = (template.preset_offset + idx) % entries.len();
            let preset = &entries[preset_index];
            EnemySquadMember {
                name: format!("{} {}", preset.name, idx + 1),
                level: base_level.saturating_add((idx as u8) % template.level_spread.max(1)),
                preset_id: NpcPresetId::new(preset_index),
            }
        })
        .collect()
}

fn generate_recruit_offer(
    seed: u64,
    depth: u32,
    tier: EncounterTier,
    key: u64,
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
    npc_presets: &NpcPresetCatalog,
    talent_catalog: &TalentCatalog,
) -> Vec<SquadMember> {
    let scaling = super::rewards::recruit_offer_scaling(depth, tier);
    let mut recruits = (0..scaling.offer_size.max(DEFAULT_RECRUIT_OFFER_SIZE))
        .map(|idx| {
            roll_recruit_member(
                format!("recruit-{}-{}", depth, idx + 1),
                derive_seed(seed, "recruit-offer", key.saturating_mul(10) + idx as u64),
                depth,
                tier,
                weapon_catalog,
                armor_catalog,
                shield_catalog,
            )
        })
        .collect::<Vec<_>>();
    sync_members_to_combat_vitals(
        &mut recruits,
        weapon_catalog,
        armor_catalog,
        shield_catalog,
        npc_presets,
        talent_catalog,
    );
    recruits
}

fn sync_members_to_combat_vitals(
    members: &mut [SquadMember],
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
    npc_presets: &NpcPresetCatalog,
    talent_catalog: &TalentCatalog,
) {
    for member in members {
        sync_member_to_combat_vitals(
            member,
            weapon_catalog,
            armor_catalog,
            shield_catalog,
            npc_presets,
            talent_catalog,
        );
    }
}

fn sync_member_to_combat_vitals(
    member: &mut SquadMember,
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
    npc_presets: &NpcPresetCatalog,
    talent_catalog: &TalentCatalog,
) {
    let combatant = game_logic::build_combatant(
        &member.config,
        weapon_catalog,
        armor_catalog,
        shield_catalog,
        npc_presets,
        talent_catalog,
    );
    let combat_max_hp = combatant.sheet.vitals.max_hp.max(1);
    if combat_max_hp == member.max_hp {
        return;
    }

    let was_full = member.current_hp >= member.max_hp;
    let hp_ratio = if member.max_hp <= 0 {
        1.0
    } else {
        (member.current_hp.max(0) as f32 / member.max_hp as f32).clamp(0.0, 1.0)
    };
    member.max_hp = combat_max_hp;
    member.current_hp = if was_full {
        combat_max_hp
    } else {
        ((combat_max_hp as f32) * hp_ratio)
            .round()
            .clamp(0.0, combat_max_hp as f32) as i32
    };
}

fn fight_seconds_to_next_initiative(fight: &SquadCombat) -> u32 {
    let now = fight.elapsed_seconds as f32;
    fight
        .units
        .iter()
        .filter(|unit| unit.is_alive())
        .map(|unit| (unit.initiative_ready_at - now).ceil().max(1.0) as u32)
        .min()
        .unwrap_or(1)
}

fn encounter_tier(tier: SquadEncounterTier) -> EncounterTier {
    match tier {
        SquadEncounterTier::Normal => EncounterTier::Normal,
        SquadEncounterTier::Elite => EncounterTier::Elite,
        SquadEncounterTier::Boss => EncounterTier::Boss,
    }
}

fn tier_label(tier: EncounterTier) -> &'static str {
    match tier {
        EncounterTier::Normal => "Normal",
        EncounterTier::Elite => "Elite",
        EncounterTier::Boss => "Boss",
    }
}

fn roll_gold_for_node(seed: u64, node: &SquadRouteNode) -> u32 {
    if node.reward.gold_max <= node.reward.gold_min {
        return node.reward.gold_min;
    }
    let mut rng = SimRng::from_seed(derive_seed(seed, "squad-node-gold", node.id as u64));
    rng.gen_range(node.reward.gold_min..=node.reward.gold_max)
}

fn roll_percent(seed: u64, label: &'static str, key: u64) -> u8 {
    let mut rng = SimRng::from_seed(derive_seed(seed, label, key));
    rng.gen_range(0..100)
}

fn find_weapon_id_by_name(catalog: &WeaponCatalog, name: &str) -> Option<WeaponId> {
    catalog
        .entries()
        .iter()
        .position(|weapon| weapon.name.eq_ignore_ascii_case(name))
        .map(WeaponId::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formation_move_is_used_for_fight_spawn() {
        let mut app = SquadBattlerApp::new().expect("app should load");
        let view = app.new_run(Some(0x5155_4144_4256_0001));
        let node_id = view
            .available_nodes
            .iter()
            .copied()
            .find(|node_id| {
                view.route
                    .iter()
                    .find(|node| node.id == *node_id)
                    .is_some_and(|node| {
                        matches!(
                            node.kind,
                            SquadNodeKind::Fight | SquadNodeKind::Elite | SquadNodeKind::Boss
                        )
                    })
            })
            .expect("first floor should include a fight");
        let view = app.choose_node(node_id).expect("fight node should open");
        let member_id = view.squad.active[0].id.clone();

        app.set_formation_position(member_id.clone(), 3, 7)
            .expect("formation move should be accepted");
        let view = app.start_fight().expect("fight should start");
        let fight = view.live_fight.expect("fight should be live");
        let unit = fight
            .combatants
            .iter()
            .find(|unit| unit.id == member_id)
            .expect("moved member should exist in combat");

        assert_eq!((unit.x, unit.y), (3, 7));
    }

    #[test]
    fn fight_starts_players_at_full_combat_sheet_hp() {
        let mut app = SquadBattlerApp::new().expect("app should load");
        let view = app.new_run(Some(0x5155_4144_4256_0001));
        assert!(
            view.squad
                .active
                .iter()
                .all(|member| member.hp == member.max_hp),
            "freshly rolled heroes should start full"
        );
        let expected_hp = view
            .squad
            .active
            .iter()
            .map(|member| (member.id.clone(), member.max_hp))
            .collect::<std::collections::HashMap<_, _>>();
        let node_id = view
            .available_nodes
            .iter()
            .copied()
            .find(|node_id| {
                view.route
                    .iter()
                    .find(|node| node.id == *node_id)
                    .is_some_and(|node| matches!(node.kind, SquadNodeKind::Fight))
            })
            .expect("first floor should include a fight");

        app.choose_node(node_id).expect("fight node should open");
        let view = app.start_fight().expect("fight should start");
        let fight = view.live_fight.expect("fight should be live");

        for unit in fight.combatants.iter().filter(|unit| unit.team_id == 0) {
            assert_eq!(unit.hp, unit.max_hp);
            assert_eq!(Some(&unit.max_hp), expected_hp.get(&unit.id));
        }
    }

    #[test]
    fn formation_swap_prevents_overlapping_slots() {
        let mut app = SquadBattlerApp::new().expect("app should load");
        let view = app.new_run(Some(0x5155_4144_4256_0001));
        let node_id = view
            .available_nodes
            .iter()
            .copied()
            .find(|node_id| {
                view.route
                    .iter()
                    .find(|node| node.id == *node_id)
                    .is_some_and(|node| matches!(node.kind, SquadNodeKind::Fight))
            })
            .expect("first floor should include a fight");
        let view = app.choose_node(node_id).expect("fight node should open");
        let first = view.squad.active[0].id.clone();
        let second_slot = view
            .formation
            .slots
            .iter()
            .find(|slot| slot.member_id == view.squad.active[1].id)
            .expect("second member should have a slot")
            .clone();

        let view = app
            .set_formation_position(first, second_slot.x, second_slot.y)
            .expect("occupied formation move should swap");
        let occupied = view
            .formation
            .slots
            .iter()
            .map(|slot| (slot.x, slot.y))
            .collect::<std::collections::HashSet<_>>();

        assert_eq!(occupied.len(), view.formation.slots.len());
    }
}
