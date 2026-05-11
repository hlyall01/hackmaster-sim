#[path = "server.rs"]
mod server;
#[path = "web_assets.rs"]
mod web_assets;

use hackmaster_sim::character::{
    AbilityScore, AbilitySet, AbilitySetFull, Progression, ProgressionTier,
};
use hackmaster_sim::core::gameplay::{
    CombatantBuilder, EncounterTier, EnemySpawnEntry, EnemySpawner, EventCatalog, EventSpec,
    LootItemEntry, LootTable, RunState, Wound, XpCurve, apply_downtime, apply_fight_result,
    apply_xp, choose_event, resolve_event_choice,
};
use hackmaster_sim::core::ids::NpcPresetId;
use hackmaster_sim::core::rng::{SimRng, derive_seed};
use hackmaster_sim::core::sim::{CombatEvent, CombatEventKind, SimConfig, SimState};
use hackmaster_sim::core::types::{EnemyProfile, Inventory, PlayerProfile, PointPools, RaceSpec};
use hackmaster_sim::data;
use hackmaster_sim::game_logic::{
    self, ArmorCatalog, ArmorId, FighterPreset, FighterPresetCatalog, NpcPresetCatalog,
    PlayerConfig, ShieldCatalog, ShieldId, TalentCatalog, WeaponCatalog, WeaponId,
};
use hackmaster_sim::sim;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

const QUICK_STARTS_PATH: &str = "data/autobattler/autobattler_quick_starts.json";
const NPC_PRESETS_PATH: &str = "data/sim/npc_presets.json";
const EVENTS_PATH: &str = "data/autobattler/events_v1_handcrafted.json";
const DEFAULT_PORT: u16 = 8787;
const DEMO_START_DISTANCE_FT: f32 = 20.0;
const DEMO_STOP_DISTANCE_FT: f32 = 1.0;
const DEMO_FIGHT_MAX_SECONDS: u32 = 120;
const SIM_STEP_SECONDS: f32 = 1.0;

pub(crate) fn run() {
    hackmaster_sim::console::maybe_enable_console();
    let port = std::env::args()
        .skip_while(|arg| arg != "--port")
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_PORT);

    let demo = Arc::new(Mutex::new(
        DemoApp::new().unwrap_or_else(|err| panic!("Failed to start v2 demo: {err}")),
    ));
    let listener = TcpListener::bind(("127.0.0.1", port))
        .unwrap_or_else(|err| panic!("Failed to bind 127.0.0.1:{port}: {err}"));

    println!("HackMaster Autobattler v2 demo running at http://127.0.0.1:{port}");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let demo = Arc::clone(&demo);
                std::thread::spawn(move || server::handle_connection(stream, demo));
            }
            Err(err) => eprintln!("Connection failed: {err}"),
        }
    }
}

#[derive(Clone)]
pub(crate) struct DemoApp {
    weapon_catalog: WeaponCatalog,
    armor_catalog: ArmorCatalog,
    shield_catalog: ShieldCatalog,
    npc_presets: NpcPresetCatalog,
    quick_starts: FighterPresetCatalog,
    race_catalog: Vec<RaceSpec>,
    talent_catalog: TalentCatalog,
    event_catalog: EventCatalog,
    spawner: EnemySpawner,
    loot_table: LootTable,
    xp_curve: XpCurve,
    enemy_weapon_id: WeaponId,
    session: Option<DemoSession>,
}

impl DemoApp {
    pub(crate) fn new() -> Result<Self, String> {
        let (weapon_catalog, armor_catalog, shield_catalog) = data::load_catalogs()?;
        let npc_presets = data::load_npc_presets(NPC_PRESETS_PATH)?;
        let quick_starts = data::load_fighter_presets(QUICK_STARTS_PATH)?;
        let race_catalog = data::load_races("data/sim/races.json")?;
        let talent_catalog = data::load_talents(data::TALENTS_PATH)?;
        let event_catalog = data::load_autobattler_events(EVENTS_PATH)?;
        let enemy_weapon_id = find_weapon_id_by_name(&weapon_catalog, "Battle axe")
            .or_else(|| weapon_catalog.first_id())
            .unwrap_or_else(|| WeaponId::new(0));
        let spawner = hobgoblin_spawner(&npc_presets);
        let loot_table = LootTable {
            gold_range: 10..=24,
            xp_per_level: 22,
            item_table: vec![
                LootItemEntry {
                    name: "field dressing".to_string(),
                    weight: 4,
                },
                LootItemEntry {
                    name: "sharpening stone".to_string(),
                    weight: 3,
                },
                LootItemEntry {
                    name: "minor trade good".to_string(),
                    weight: 2,
                },
                LootItemEntry {
                    name: "lucky charm".to_string(),
                    weight: 1,
                },
            ],
        };
        Ok(Self {
            weapon_catalog,
            armor_catalog,
            shield_catalog,
            npc_presets,
            quick_starts,
            race_catalog,
            talent_catalog,
            event_catalog,
            spawner,
            loot_table,
            xp_curve: XpCurve {
                base: 45,
                per_level: 55,
            },
            enemy_weapon_id,
            session: None,
        })
    }

    pub(crate) fn new_run(&mut self, request: NewRunRequest) -> Result<DemoView, String> {
        let seed = request.seed.unwrap_or_else(|| {
            let mut rng = rand::thread_rng();
            rng.gen_range(1..=u64::MAX)
        });
        let preset = self
            .find_preset(request.preset.as_deref())
            .ok_or_else(|| "No quick-start presets found.".to_string())?
            .clone();
        let mut player_config = player_config_from_preset(
            &preset,
            &self.weapon_catalog,
            &self.armor_catalog,
            &self.shield_catalog,
            &self.race_catalog,
        );
        player_config.name = request
            .name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| preset.name.clone());
        let player_profile = player_profile_from_config(&player_config);
        let mut run_state = RunState::new(player_profile, Inventory::default(), seed);
        run_state.inventory.gold = 20;

        let session = DemoSession {
            run_state,
            player_config,
            map: generate_map(seed),
            current_floor: 0,
            phase: DemoPhase::ChoosingNode,
            pending_event: None,
            pending_fight: None,
            live_fight: None,
            selected_node: None,
            last_log: vec!["Run started. Pick a route.".to_string()],
            last_reward: None,
            last_fight: None,
            terminal: None,
        };
        self.session = Some(session);
        Ok(self.view())
    }

    pub(crate) fn choose_node(&mut self, node_id: usize) -> Result<DemoView, String> {
        let spawner = self.spawner.clone();
        let npc_presets = self.npc_presets.clone();
        let Some(session) = self.session.as_mut() else {
            return Err("Start a run first.".to_string());
        };
        if session.phase != DemoPhase::ChoosingNode {
            return Err("Resolve the current node before choosing another.".to_string());
        }
        let Some(node) = session.map.iter().find(|node| node.id == node_id).cloned() else {
            return Err("Node does not exist.".to_string());
        };
        if node.floor != session.current_floor || node.completed {
            return Err("That node is not currently available.".to_string());
        }
        session.selected_node = Some(node.id);
        session.last_reward = None;
        session.last_fight = None;

        match node.kind {
            NodeKind::Fight | NodeKind::Elite | NodeKind::Boss => {
                let tier = encounter_tier_for_node(node.kind);
                let enemy_name =
                    preview_enemy_name(&spawner, &npc_presets, &session.run_state, tier);
                session.pending_fight = Some(PendingDemoFight {
                    kind: node.kind,
                    enemy_name,
                    tier,
                });
                session.phase = DemoPhase::FightPreview;
                session.last_log = vec![format!("Fight scouted: {}.", node.kind.label())];
            }
            NodeKind::Rest => {
                apply_downtime(&mut session.run_state, 8, true);
                session.last_log = vec![
                    "You spend eight days in guarded rest.".to_string(),
                    format_wound_line(&session.run_state.wounds),
                ];
                session.last_reward = Some(DemoReward {
                    gold: 0,
                    xp: 0,
                    items: Vec::new(),
                    level_gained: false,
                });
                self.complete_selected_node();
            }
            NodeKind::Event => {
                let encounter_index = session.run_state.encounter_index as u64;
                let event_seed =
                    derive_seed(session.run_state.run_seed, "v2-event-kind", encounter_index);
                let mut rng = SimRng::from_seed(event_seed);
                let event = choose_event(
                    &self.event_catalog,
                    &session.run_state,
                    EncounterTier::Normal,
                    &mut rng,
                )
                .or_else(|| self.event_catalog.events.first().cloned())
                .ok_or_else(|| "No events available.".to_string())?;
                session.last_log = vec![format!("Event discovered: {}", event.name)];
                session.pending_event = Some(PendingDemoEvent {
                    event,
                    resolve_seed: derive_seed(
                        session.run_state.run_seed,
                        "v2-event-resolve",
                        encounter_index,
                    ),
                });
                session.phase = DemoPhase::ResolvingEvent;
            }
        }
        Ok(self.view())
    }

    pub(crate) fn resolve_event_choice(&mut self, choice_id: String) -> Result<DemoView, String> {
        let spawner = self.spawner.clone();
        let npc_presets = self.npc_presets.clone();
        let Some(session) = self.session.as_mut() else {
            return Err("Start a run first.".to_string());
        };
        if session.phase != DemoPhase::ResolvingEvent {
            return Err("There is no pending event choice.".to_string());
        }
        let Some(pending) = session.pending_event.take() else {
            return Err("There is no pending event.".to_string());
        };
        let previous_level = session.run_state.player.level;
        let mut rng = SimRng::from_seed(pending.resolve_seed);
        let resolution = resolve_event_choice(
            &mut session.run_state,
            &pending.event,
            Some(&choice_id),
            &mut rng,
        );
        let _ = apply_xp(&mut session.run_state.player, &self.xp_curve, 0);
        let level_gained = session.run_state.player.level > previous_level;
        if level_gained {
            grant_demo_level_points(&mut session.run_state.player);
        }
        session.run_state.encounter_index = session.run_state.encounter_index.saturating_add(1);
        session.last_log = resolution.lines.clone();
        session.last_reward = Some(DemoReward {
            gold: 0,
            xp: 0,
            items: Vec::new(),
            level_gained,
        });
        if resolution.trigger_fight {
            session
                .last_log
                .push("The event spills into a fight.".to_string());
            let kind = NodeKind::Elite;
            let tier = EncounterTier::Elite;
            let enemy_name = preview_enemy_name(&spawner, &npc_presets, &session.run_state, tier);
            session.pending_fight = Some(PendingDemoFight {
                kind,
                enemy_name,
                tier,
            });
            session.phase = DemoPhase::FightPreview;
        } else {
            self.complete_selected_node();
        }
        Ok(self.view())
    }

    pub(crate) fn start_pending_fight(&mut self) -> Result<DemoView, String> {
        let Some(session) = self.session.as_mut() else {
            return Err("Start a run first.".to_string());
        };
        if session.phase != DemoPhase::FightPreview {
            return Err("There is no pending fight.".to_string());
        }
        let Some(pending) = session.pending_fight.take() else {
            return Err("There is no pending fight.".to_string());
        };
        let encounter_index = session.run_state.encounter_index as u64;
        let spawn_seed = derive_seed(session.run_state.run_seed, "spawn", encounter_index);
        let combat_seed = derive_seed(session.run_state.run_seed, "combat", encounter_index);
        let mut spawn_rng = SimRng::from_seed(spawn_seed);
        let Some(enemy) = self.spawner.spawn_for_level(
            effective_enemy_level(&session.run_state, pending.tier),
            &mut spawn_rng,
        ) else {
            return Err("No enemy available for this fight.".to_string());
        };
        let builder = DemoCombatantBuilder {
            player_base: session.player_config.clone(),
            enemy_weapon_id: self.enemy_weapon_id,
            weapon_catalog: &self.weapon_catalog,
            armor_catalog: &self.armor_catalog,
            shield_catalog: &self.shield_catalog,
            npc_presets: &self.npc_presets,
            talent_catalog: &self.talent_catalog,
        };
        let mut player_combatant = builder.build_player(&session.run_state);
        let mut enemy_combatant = builder.build_enemy(&enemy);
        player_combatant.team_id = 0;
        enemy_combatant.team_id = 1;
        let mut sim = SimState::with_rng(demo_sim_config(), SimRng::from_seed(combat_seed));
        sim.reset_with_combatants(vec![player_combatant, enemy_combatant]);
        session.live_fight = Some(DemoLiveFight {
            sim,
            enemy,
            enemy_name: pending.enemy_name,
            kind: pending.kind,
            tier: pending.tier,
            max_seconds: DEMO_FIGHT_MAX_SECONDS,
            seen_events: 0,
            log_lines: Vec::new(),
            running: false,
            decision_count: 0,
        });
        session.phase = DemoPhase::CombatPlayback;
        session.last_log =
            vec!["Combat started. Step one second at a time or enable auto-play.".to_string()];
        Ok(self.view())
    }

    pub(crate) fn fight_command(
        &mut self,
        request: FightCommandRequest,
    ) -> Result<DemoView, String> {
        let Some(session) = self.session.as_mut() else {
            return Err("Start a run first.".to_string());
        };
        if session.phase != DemoPhase::CombatPlayback {
            return Err("There is no live fight.".to_string());
        }
        match request.command.as_str() {
            "play" => {
                if let Some(live) = session.live_fight.as_mut() {
                    live.running = true;
                }
            }
            "pause" => {
                if let Some(live) = session.live_fight.as_mut() {
                    live.running = false;
                }
            }
            "step" | "tick" => {
                let seconds = request.seconds.unwrap_or(1).clamp(1, 30);
                self.advance_live_fight(seconds);
            }
            "next_attack" | "next" => {
                self.advance_to_next_attack();
            }
            "skip" => {
                self.advance_live_fight(120);
            }
            _ => return Err("Unknown fight command.".to_string()),
        }
        Ok(self.view())
    }

    fn advance_to_next_attack(&mut self) {
        let Some(initial_events) = self
            .session
            .as_ref()
            .and_then(|session| session.live_fight.as_ref())
            .map(|live| live.sim.combat_events.len())
        else {
            return;
        };
        for _ in 0..DEMO_FIGHT_MAX_SECONDS {
            self.advance_live_fight(1);
            let Some(live) = self
                .session
                .as_ref()
                .and_then(|session| session.live_fight.as_ref())
            else {
                break;
            };
            if live.sim.combat_events.len() > initial_events
                || live.sim.elapsed_seconds >= live.max_seconds
            {
                break;
            }
        }
    }

    pub(crate) fn claim_reward(&mut self) -> Result<DemoView, String> {
        let Some(session) = self.session.as_ref() else {
            return Err("Start a run first.".to_string());
        };
        if session.phase != DemoPhase::RewardReview {
            return Err("There is no reward to claim.".to_string());
        }
        self.complete_selected_node();
        Ok(self.view())
    }

    fn advance_live_fight(&mut self, seconds: u32) {
        for _ in 0..seconds {
            let done = {
                let Some(session) = self.session.as_mut() else {
                    return;
                };
                let Some(live) = session.live_fight.as_mut() else {
                    return;
                };
                live.sim.update(SIM_STEP_SECONDS);
                ingest_live_fight_events(live);
                live.sim.done || live.sim.elapsed_seconds >= live.max_seconds
            };
            if done {
                self.finalize_live_fight();
                break;
            }
        }
    }

    fn finalize_live_fight(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let Some(live) = session.live_fight.take() else {
            return;
        };
        let previous_level = session.run_state.player.level;
        let previous_gold = session.run_state.inventory.gold;
        let player_hp = live.sim.combatants[0].state.hp;
        let enemy_hp = live.sim.combatants[1].state.hp;
        let won = live.sim.done && player_hp > 0 && enemy_hp <= 0;
        let fight = hackmaster_sim::core::gameplay::FightResult {
            won,
            remaining_hp: player_hp,
            turns: live.sim.elapsed_seconds,
            events: live.sim.combat_events.clone(),
        };
        let outcome = apply_fight_result(
            session.run_state.clone(),
            Some(live.enemy),
            fight,
            &self.loot_table,
            Some(&self.xp_curve),
            0,
            false,
            live.tier,
        );
        let level_gained = outcome.state.player.level > previous_level;
        let mut next_state = outcome.state.clone();
        if level_gained {
            grant_demo_level_points(&mut next_state.player);
        }
        let gold = next_state.inventory.gold.saturating_sub(previous_gold);
        let reward = outcome.reward.clone();
        let fight_view = DemoFightSummary {
            enemy: live.enemy_name.clone(),
            won,
            turns: live.sim.elapsed_seconds,
            remaining_hp: player_hp,
            hits_dealt: format_hit_list(&live.sim.combat_events, 0, 1),
            hits_taken: format_hit_list(&live.sim.combat_events, 1, 0),
            combat_log: live.log_lines.clone(),
        };
        session.run_state = next_state;
        session.last_fight = Some(fight_view);
        session.last_reward = Some(DemoReward {
            gold,
            xp: reward.as_ref().map(|reward| reward.xp).unwrap_or(0),
            items: reward.map(|reward| reward.items).unwrap_or_default(),
            level_gained,
        });
        session.last_log = vec![
            format!(
                "{} against {}.",
                if won { "Victory" } else { "Defeat" },
                live.enemy_name
            ),
            format!(
                "Fight lasted {}s as a {} node.",
                live.sim.elapsed_seconds,
                live.kind.label()
            ),
            format!("Combat decisions recorded: {}.", live.decision_count),
            format_wound_line(&session.run_state.wounds),
        ];
        if won {
            session.phase = DemoPhase::RewardReview;
        } else {
            session.phase = DemoPhase::RunOver;
            session.terminal = Some("You were defeated in the field.".to_string());
        }
    }

    fn complete_selected_node(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let Some(node_id) = session.selected_node.take() else {
            return;
        };
        if let Some(node) = session.map.iter_mut().find(|node| node.id == node_id) {
            node.completed = true;
            if node.kind == NodeKind::Boss {
                session.phase = DemoPhase::RunOver;
                session.terminal = Some("Demo boss defeated. Run complete.".to_string());
                return;
            }
        }
        session.current_floor = session.current_floor.saturating_add(1);
        session.phase = DemoPhase::ChoosingNode;
    }

    fn find_preset(&self, name: Option<&str>) -> Option<&FighterPreset> {
        name.and_then(|name| {
            self.quick_starts
                .entries()
                .iter()
                .find(|preset| preset.name.eq_ignore_ascii_case(name))
        })
        .or_else(|| self.quick_starts.entries().first())
    }

    pub(crate) fn view(&self) -> DemoView {
        let preset_names = self
            .quick_starts
            .entries()
            .iter()
            .map(|preset| preset.name.clone())
            .collect::<Vec<_>>();
        let Some(session) = self.session.as_ref() else {
            return DemoView {
                has_run: false,
                presets: preset_names,
                player: None,
                inventory: None,
                map: Vec::new(),
                available_nodes: Vec::new(),
                phase: "start".to_string(),
                pending_event: None,
                pending_fight: None,
                live_fight: None,
                last_log: vec!["Roll a character to start.".to_string()],
                last_reward: None,
                last_fight: None,
                terminal: None,
            };
        };
        let available_nodes = if session.phase == DemoPhase::ChoosingNode {
            session
                .map
                .iter()
                .filter(|node| node.floor == session.current_floor && !node.completed)
                .map(|node| node.id)
                .collect()
        } else {
            Vec::new()
        };
        DemoView {
            has_run: true,
            presets: preset_names,
            player: Some(DemoPlayerView::from_state(&session.run_state)),
            inventory: Some(DemoInventoryView::from_inventory(
                &session.run_state.inventory,
            )),
            map: session.map.clone(),
            available_nodes,
            phase: session.phase.label().to_string(),
            pending_event: session
                .pending_event
                .as_ref()
                .map(DemoEventView::from_pending),
            pending_fight: session
                .pending_fight
                .clone()
                .map(DemoFightPreview::from_pending),
            live_fight: session
                .live_fight
                .as_ref()
                .map(DemoFightPlaybackView::from_live),
            last_log: session.last_log.clone(),
            last_reward: session.last_reward.clone(),
            last_fight: session.last_fight.clone(),
            terminal: session.terminal.clone(),
        }
    }
}

fn demo_sim_config() -> SimConfig {
    SimConfig::new(DEMO_START_DISTANCE_FT, DEMO_STOP_DISTANCE_FT)
}

#[derive(Clone)]
struct DemoSession {
    run_state: RunState,
    player_config: PlayerConfig,
    map: Vec<DemoNode>,
    current_floor: u32,
    phase: DemoPhase,
    pending_event: Option<PendingDemoEvent>,
    pending_fight: Option<PendingDemoFight>,
    live_fight: Option<DemoLiveFight>,
    selected_node: Option<usize>,
    last_log: Vec<String>,
    last_reward: Option<DemoReward>,
    last_fight: Option<DemoFightSummary>,
    terminal: Option<String>,
}

#[derive(Clone)]
struct PendingDemoEvent {
    event: EventSpec,
    resolve_seed: u64,
}

#[derive(Clone)]
struct PendingDemoFight {
    kind: NodeKind,
    enemy_name: String,
    tier: EncounterTier,
}

#[derive(Clone)]
struct DemoLiveFight {
    sim: SimState,
    enemy: EnemyProfile,
    enemy_name: String,
    kind: NodeKind,
    tier: EncounterTier,
    max_seconds: u32,
    seen_events: usize,
    log_lines: Vec<String>,
    running: bool,
    decision_count: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DemoPhase {
    ChoosingNode,
    ResolvingEvent,
    FightPreview,
    CombatPlayback,
    RewardReview,
    RunOver,
}

impl DemoPhase {
    fn label(self) -> &'static str {
        match self {
            DemoPhase::ChoosingNode => "choose_node",
            DemoPhase::ResolvingEvent => "event_choice",
            DemoPhase::FightPreview => "fight_preview",
            DemoPhase::CombatPlayback => "combat_playback",
            DemoPhase::RewardReview => "reward_review",
            DemoPhase::RunOver => "run_over",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct DemoView {
    has_run: bool,
    presets: Vec<String>,
    player: Option<DemoPlayerView>,
    inventory: Option<DemoInventoryView>,
    map: Vec<DemoNode>,
    available_nodes: Vec<usize>,
    phase: String,
    pending_event: Option<DemoEventView>,
    pending_fight: Option<DemoFightPreview>,
    live_fight: Option<DemoFightPlaybackView>,
    last_log: Vec<String>,
    last_reward: Option<DemoReward>,
    last_fight: Option<DemoFightSummary>,
    terminal: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct DemoFightPreview {
    kind: NodeKind,
    enemy_name: String,
    tier: String,
}

impl DemoFightPreview {
    fn from_pending(pending: PendingDemoFight) -> Self {
        Self {
            kind: pending.kind,
            enemy_name: pending.enemy_name,
            tier: match pending.tier {
                EncounterTier::Normal => "Normal",
                EncounterTier::Elite => "Elite",
                EncounterTier::Boss => "Boss",
            }
            .to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct DemoFightPlaybackView {
    status: String,
    enemy_name: String,
    tier: String,
    elapsed_seconds: u32,
    max_seconds: u32,
    distance_ft: f32,
    running: bool,
    combatants: Vec<DemoCombatantView>,
    log_tail: Vec<String>,
    pending_decision: Option<DemoDecisionPromptView>,
}

impl DemoFightPlaybackView {
    fn from_live(live: &DemoLiveFight) -> Self {
        let status = if live.sim.done {
            "complete"
        } else if live.running {
            "running"
        } else {
            "paused"
        };
        Self {
            status: status.to_string(),
            enemy_name: live.enemy_name.clone(),
            tier: match live.tier {
                EncounterTier::Normal => "Normal",
                EncounterTier::Elite => "Elite",
                EncounterTier::Boss => "Boss",
            }
            .to_string(),
            elapsed_seconds: live.sim.elapsed_seconds,
            max_seconds: live.max_seconds,
            distance_ft: live.sim.distance(),
            running: live.running,
            combatants: live
                .sim
                .combatants
                .iter()
                .enumerate()
                .map(|(idx, combatant)| DemoCombatantView {
                    idx,
                    name: combatant.sheet.name.clone(),
                    team_id: combatant.team_id,
                    hp: combatant.state.hp,
                    max_hp: combatant.sheet.vitals.max_hp,
                    weapon: combatant.sheet.offense.weapon.name.clone(),
                    weapon_speed_seconds: combatant.sheet.offense.weapon.speed,
                    reach_ft: combatant.sheet.offense.weapon.reach_ft,
                    next_attack_in_seconds: combatant
                        .state
                        .next_attack_time_primary
                        .map(|time| (time - live.sim.elapsed_seconds as f32).max(0.0)),
                    shield_name: combatant.sheet.defense.shield_name.clone(),
                    x: live
                        .sim
                        .actors
                        .get(idx)
                        .map(|actor| actor.position.x)
                        .unwrap_or_default(),
                    y: live
                        .sim
                        .actors
                        .get(idx)
                        .map(|actor| actor.position.y)
                        .unwrap_or_default(),
                    trauma_seconds: combatant.state.trauma_remaining_seconds,
                    knocked_seconds: combatant.state.knockback_immobile_seconds,
                    shield_intact: combatant.state.shield_intact,
                })
                .collect(),
            log_tail: live
                .log_lines
                .iter()
                .rev()
                .take(18)
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect(),
            pending_decision: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct DemoCombatantView {
    idx: usize,
    name: String,
    team_id: u8,
    hp: i32,
    max_hp: i32,
    weapon: String,
    weapon_speed_seconds: f32,
    reach_ft: f32,
    next_attack_in_seconds: Option<f32>,
    shield_name: Option<String>,
    x: i32,
    y: i32,
    trauma_seconds: i32,
    knocked_seconds: i32,
    shield_intact: bool,
}

#[derive(Clone, Debug, Serialize)]
struct DemoDecisionPromptView {
    id: String,
    time: u32,
    actor_idx: usize,
    options: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct DemoPlayerView {
    name: String,
    level: u8,
    xp: u32,
    next_level_xp: u32,
    gold: u32,
    depth: u32,
    seed: u64,
    wounds: Vec<u32>,
    wound_total: u32,
    bp: i32,
    lp: i32,
    ap: i32,
    rp: i32,
    stats: Vec<String>,
}

impl DemoPlayerView {
    fn from_state(state: &RunState) -> Self {
        let scores = state.player.ability_scores_full;
        Self {
            name: state.player.name.clone(),
            level: state.player.level,
            xp: state.player.xp,
            next_level_xp: XpCurve {
                base: 45,
                per_level: 55,
            }
            .xp_for_next_level(state.player.level),
            gold: state.inventory.gold,
            depth: state.run_depth,
            seed: state.run_seed,
            wounds: state.wounds.iter().map(|wound| wound.damage).collect(),
            wound_total: state.total_wound_damage(),
            bp: state.player.points.bp,
            lp: state.player.points.lp,
            ap: state.player.points.ap,
            rp: state.player.points.rp,
            stats: vec![
                format!("STR {}", format_score(scores.strength)),
                format!("DEX {}", format_score(scores.dexterity)),
                format!("CON {}", scores.constitution.base),
                format!("INT {}", scores.intelligence.base),
                format!("WIS {}", scores.wisdom.base),
                format!("CHA {}", scores.charisma.base),
            ],
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct DemoInventoryView {
    gold: u32,
    items: Vec<String>,
}

impl DemoInventoryView {
    fn from_inventory(inventory: &Inventory) -> Self {
        Self {
            gold: inventory.gold,
            items: inventory.items.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct DemoEventView {
    name: String,
    description: String,
    choices: Vec<DemoChoiceView>,
}

impl DemoEventView {
    fn from_pending(pending: &PendingDemoEvent) -> Self {
        Self {
            name: pending.event.name.clone(),
            description: pending.event.description.clone(),
            choices: pending
                .event
                .choices
                .iter()
                .map(|choice| DemoChoiceView {
                    id: choice.id.clone(),
                    text: choice.text.clone(),
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct DemoChoiceView {
    id: String,
    text: String,
}

#[derive(Clone, Debug, Serialize)]
struct DemoReward {
    gold: u32,
    xp: u32,
    items: Vec<String>,
    level_gained: bool,
}

#[derive(Clone, Debug, Serialize)]
struct DemoFightSummary {
    enemy: String,
    won: bool,
    turns: u32,
    remaining_hp: i32,
    hits_dealt: String,
    hits_taken: String,
    combat_log: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct DemoNode {
    id: usize,
    floor: u32,
    lane: u32,
    kind: NodeKind,
    completed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum NodeKind {
    Fight,
    Event,
    Rest,
    Elite,
    Boss,
}

impl NodeKind {
    fn label(self) -> &'static str {
        match self {
            NodeKind::Fight => "Fight",
            NodeKind::Event => "Event",
            NodeKind::Rest => "Rest",
            NodeKind::Elite => "Elite",
            NodeKind::Boss => "Boss",
        }
    }
}

impl From<EncounterTier> for NodeKind {
    fn from(tier: EncounterTier) -> Self {
        match tier {
            EncounterTier::Normal => NodeKind::Fight,
            EncounterTier::Elite => NodeKind::Elite,
            EncounterTier::Boss => NodeKind::Boss,
        }
    }
}

fn encounter_tier_for_node(kind: NodeKind) -> EncounterTier {
    match kind {
        NodeKind::Elite => EncounterTier::Elite,
        NodeKind::Boss => EncounterTier::Boss,
        _ => EncounterTier::Normal,
    }
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct NewRunRequest {
    pub(crate) seed: Option<u64>,
    pub(crate) preset: Option<String>,
    pub(crate) name: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ChooseNodeRequest {
    pub(crate) node_id: usize,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct EventChoiceRequest {
    pub(crate) choice_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct FightCommandRequest {
    pub(crate) command: String,
    pub(crate) seconds: Option<u32>,
}

struct DemoCombatantBuilder<'a> {
    player_base: PlayerConfig,
    enemy_weapon_id: WeaponId,
    weapon_catalog: &'a WeaponCatalog,
    armor_catalog: &'a ArmorCatalog,
    shield_catalog: &'a ShieldCatalog,
    npc_presets: &'a NpcPresetCatalog,
    talent_catalog: &'a TalentCatalog,
}

impl CombatantBuilder for DemoCombatantBuilder<'_> {
    fn build_player(&self, state: &RunState) -> hackmaster_sim::core::sim::Combatant {
        let mut player = self.player_base.clone();
        player.name = state.player.name.clone();
        player.level = state.player.level;
        player.progression = state.player.progression;
        player.strength_base = state.player.base_stats.strength.base;
        player.strength_pct = state.player.base_stats.strength.percentile;
        player.dex_base = state.player.base_stats.dexterity.base;
        player.dex_pct = state.player.base_stats.dexterity.percentile;
        player.intelligence = state.player.base_stats.intelligence;
        player.wisdom = state.player.base_stats.wisdom;
        player.constitution = state.player.base_stats.constitution;
        player.looks = state.player.base_stats.looks;
        player.charisma = state.player.base_stats.charisma;
        player.race_id = state.player.race_id.clone();
        player.race_applied = player.race_id.is_some();
        player.proficiencies = state.player.proficiencies.clone();
        player.talents = state.player.talents.clone();
        let mut combatant = game_logic::build_combatant(
            &player,
            self.weapon_catalog,
            self.armor_catalog,
            self.shield_catalog,
            self.npc_presets,
            self.talent_catalog,
        );
        let wound_total = state.total_wound_damage();
        if wound_total > 0 {
            let wound_total = i32::try_from(wound_total).unwrap_or(i32::MAX);
            combatant.state.hp = (combatant.sheet.vitals.max_hp - wound_total).max(0);
        }
        combatant
    }

    fn build_enemy(&self, enemy: &EnemyProfile) -> hackmaster_sim::core::sim::Combatant {
        let mut npc = PlayerConfig::new("Hobgoblin", self.enemy_weapon_id);
        npc.level = enemy.level;
        npc.npc_preset = Some(enemy.preset_id);
        game_logic::build_combatant(
            &npc,
            self.weapon_catalog,
            self.armor_catalog,
            self.shield_catalog,
            self.npc_presets,
            self.talent_catalog,
        )
    }
}

fn generate_map(seed: u64) -> Vec<DemoNode> {
    let mut rng = SimRng::from_seed(derive_seed(seed, "v2-map", 0));
    let mut id = 0;
    let mut nodes = Vec::new();
    let rows = [
        [NodeKind::Fight, NodeKind::Event, NodeKind::Fight],
        [NodeKind::Event, NodeKind::Fight, NodeKind::Rest],
        [NodeKind::Elite, NodeKind::Event, NodeKind::Fight],
    ];
    for (floor, kinds) in rows.iter().enumerate() {
        for (lane, kind) in kinds.iter().enumerate() {
            let mut kind = *kind;
            if floor == 1 && rng.gen_range(0..100) < 35 {
                kind = NodeKind::Fight;
            }
            nodes.push(DemoNode {
                id,
                floor: floor as u32,
                lane: lane as u32,
                kind,
                completed: false,
            });
            id += 1;
        }
    }
    nodes.push(DemoNode {
        id,
        floor: 3,
        lane: 1,
        kind: NodeKind::Boss,
        completed: false,
    });
    nodes
}

fn preview_enemy_name(
    spawner: &EnemySpawner,
    npc_presets: &NpcPresetCatalog,
    state: &RunState,
    tier: EncounterTier,
) -> String {
    let effective_level = effective_enemy_level(state, tier);
    let mut rng = SimRng::from_seed(derive_seed(
        state.run_seed,
        "v2-fight-preview",
        state.encounter_index as u64,
    ));
    spawner
        .spawn_for_level(effective_level, &mut rng)
        .and_then(|enemy| npc_presets.get(enemy.preset_id))
        .map(|preset| preset.name.clone())
        .unwrap_or_else(|| "Unknown foe".to_string())
}

fn effective_enemy_level(state: &RunState, tier: EncounterTier) -> u8 {
    state
        .player
        .level
        .saturating_add((state.run_depth / 2) as u8)
        .saturating_add(depth_band_bonus(state.run_depth))
        .saturating_add(match tier {
            EncounterTier::Normal => 0,
            EncounterTier::Elite => 1,
            EncounterTier::Boss => 2,
        })
}

fn depth_band_bonus(depth: u32) -> u8 {
    match depth {
        0..=4 => 0,
        5..=11 => 1,
        12..=23 => 2,
        _ => 3,
    }
}

fn ingest_live_fight_events(live: &mut DemoLiveFight) {
    if live.seen_events >= live.sim.combat_events.len() {
        return;
    }
    for event in &live.sim.combat_events[live.seen_events..] {
        live.log_lines
            .push(sim::format_combat_event_line(event, &live.sim.combatants));
    }
    live.seen_events = live.sim.combat_events.len();
}

fn grant_demo_level_points(player: &mut PlayerProfile) {
    player.points.bp = player.points.bp.saturating_add(5);
    player.points.lp = player.points.lp.saturating_add(1);
    player.points.ap = player.points.ap.saturating_add(1);
}

fn player_config_from_preset(
    preset: &FighterPreset,
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
    race_catalog: &[RaceSpec],
) -> PlayerConfig {
    let mut player = PlayerConfig::new(
        &preset.name,
        weapon_catalog
            .first_id()
            .unwrap_or_else(|| WeaponId::new(0)),
    );
    player.level = preset.level;
    player.progression = Progression::new(
        tier_from_label(&preset.progression.attack).unwrap_or(ProgressionTier::I),
        tier_from_label(&preset.progression.speed).unwrap_or(ProgressionTier::I),
        tier_from_label(&preset.progression.initiative).unwrap_or(ProgressionTier::I),
        tier_from_label(&preset.progression.health).unwrap_or(ProgressionTier::I),
    );
    player.mastery_attack = game_logic::clamp_mastery(preset.masteries.attack);
    player.mastery_defense = game_logic::clamp_mastery(preset.masteries.defense);
    player.mastery_damage = game_logic::clamp_mastery(preset.masteries.damage);
    player.mastery_speed = game_logic::clamp_mastery(preset.masteries.speed);
    player.shield_mastery_defense = game_logic::clamp_mastery(preset.masteries.shield_defense);
    player.shield_mastery_speed = game_logic::clamp_mastery(preset.masteries.shield_speed);
    player.base_hp = preset.base_hp;
    player.move_speed = preset.move_speed;
    player.strength_base = preset.strength_base;
    player.strength_pct = game_logic::normalize_percentile(preset.strength_pct);
    player.dex_base = preset.dex_base;
    player.dex_pct = game_logic::normalize_percentile(preset.dex_pct);
    player.intelligence = preset.intelligence;
    player.wisdom = preset.wisdom;
    player.constitution = preset.constitution;
    player.looks = preset.looks;
    player.charisma = preset.charisma;
    player.weapon_material_tier = preset.weapon_material_tier;
    player.offhand_weapon_material_tier = preset.offhand_weapon_material_tier;
    player.armor_material_tier = preset.armor_material_tier;
    player.projectile_material_tier = preset.projectile_material_tier;
    player.offhand_projectile_material_tier = preset.offhand_projectile_material_tier;
    player.shield_material_tier = preset.shield_material_tier;
    player.two_hand_grip = preset.two_hand_grip;
    player.use_jab = preset.maneuvers.use_jab;
    player.hold_at_bay = preset.maneuvers.hold_at_bay;
    player.called_shot = preset.maneuvers.called_shot;
    player.aggressive_attack = preset.maneuvers.aggressive_attack;
    player.charge = preset.maneuvers.charge;
    player.ready_against_charge = preset.maneuvers.ready_against_charge;
    player.tactical_move = preset.maneuvers.tactical_move;
    player.fight_defensively = preset.maneuvers.fight_defensively;
    player.fight_defensively_penalty = preset.maneuvers.fight_defensively_penalty;
    player.full_parry = preset.maneuvers.full_parry;
    player.give_ground = preset.maneuvers.give_ground;
    player.scamper_back = preset.maneuvers.scamper_back;
    player.fighting_withdrawal = preset.maneuvers.fighting_withdrawal;
    player.flee = preset.maneuvers.flee;
    player.mounted = preset.maneuvers.mounted;
    player.defensive_dualwielding = preset.defensive_dualwielding;
    player.offensive_dualwielding = preset.offensive_dualwielding;
    player.proficiencies = preset.proficiencies.clone();
    player.talents = preset.talents.clone();
    player.race_id = preset.race_id.clone();
    player.race_applied = player.race_id.is_some();
    player.knockback_step =
        game_logic::knockback_step_for_race_id(player.race_id.as_deref(), race_catalog);
    player.weapon_id = find_weapon_id_by_name(weapon_catalog, &preset.weapon)
        .or_else(|| weapon_catalog.first_id())
        .unwrap_or_else(|| WeaponId::new(0));
    player.offhand_weapon_id = preset
        .offhand_weapon
        .as_deref()
        .and_then(|name| find_weapon_id_by_name(weapon_catalog, name));
    player.armor_id = find_armor_id_by_name(armor_catalog, &preset.armor)
        .or_else(|| armor_catalog.first_id())
        .unwrap_or_else(|| ArmorId::new(0));
    player.shield_id = find_shield_id_by_name(shield_catalog, &preset.shield)
        .or_else(|| shield_catalog.first_id())
        .unwrap_or_else(|| ShieldId::new(0));
    if let Some(weapon) = weapon_catalog.get(player.weapon_id) {
        game_logic::sanitize_projectile_tier(&mut player, weapon);
    }
    player
}

fn player_profile_from_config(config: &PlayerConfig) -> PlayerProfile {
    let ability_scores_full = AbilitySetFull {
        strength: AbilityScore::new(config.strength_base, config.strength_pct),
        intelligence: AbilityScore::new(config.intelligence, 1),
        wisdom: AbilityScore::new(config.wisdom, 1),
        dexterity: AbilityScore::new(config.dex_base, config.dex_pct),
        constitution: AbilityScore::new(config.constitution, 1),
        looks: AbilityScore::new(config.looks, 1),
        charisma: AbilityScore::new(config.charisma, 1),
    };
    PlayerProfile {
        name: config.name.clone(),
        level: config.level,
        xp: 0,
        base_stats: AbilitySet::from(ability_scores_full),
        ability_scores_full,
        progression: config.progression,
        points: PointPools::default(),
        banked_points: PointPools::default(),
        honor: 0,
        alignment: None,
        race_id: config.race_id.clone(),
        background: None,
        quirks: Vec::new(),
        flaws: Vec::new(),
        skills: Vec::new(),
        skill_levels: Vec::new(),
        proficiencies: config.proficiencies.clone(),
        talents: config.talents.clone(),
        weapon_masteries: Vec::new(),
    }
}

fn hobgoblin_spawner(npc_presets: &NpcPresetCatalog) -> EnemySpawner {
    let mut spawner = EnemySpawner::default();
    for (index, preset) in npc_presets.entries().iter().enumerate() {
        if let Some(level) = hobgoblin_level(&preset.name) {
            spawner.push(EnemySpawnEntry {
                preset_id: NpcPresetId::new(index),
                min_level: level,
                max_level: u8::MAX,
                weight: 1,
            });
        }
    }
    spawner
}

fn hobgoblin_level(name: &str) -> Option<u8> {
    let lower = name.to_ascii_lowercase();
    if lower == "hobgoblin" {
        Some(1)
    } else if let Some(rest) = lower.strip_prefix("hobgoblin ") {
        rest.trim().parse::<u8>().ok()
    } else {
        None
    }
}

fn tier_from_label(label: &str) -> Option<ProgressionTier> {
    match label.trim() {
        "I" | "1" => Some(ProgressionTier::I),
        "II" | "2" => Some(ProgressionTier::II),
        "III" | "3" => Some(ProgressionTier::III),
        "IV" | "4" => Some(ProgressionTier::IV),
        "V" | "5" => Some(ProgressionTier::V),
        "VI" | "6" => Some(ProgressionTier::VI),
        _ => None,
    }
}

fn find_weapon_id_by_name(catalog: &WeaponCatalog, name: &str) -> Option<WeaponId> {
    catalog
        .entries()
        .iter()
        .position(|weapon| weapon.name.eq_ignore_ascii_case(name))
        .and_then(|idx| catalog.id_from_index(idx))
}

fn find_armor_id_by_name(catalog: &ArmorCatalog, name: &str) -> Option<ArmorId> {
    if name.eq_ignore_ascii_case("None") {
        return catalog.first_id();
    }
    catalog
        .entries()
        .iter()
        .position(|entry| {
            entry
                .armor
                .as_ref()
                .map(|armor| armor.name.eq_ignore_ascii_case(name))
                .unwrap_or(false)
        })
        .and_then(|idx| catalog.id_from_index(idx))
}

fn find_shield_id_by_name(catalog: &ShieldCatalog, name: &str) -> Option<ShieldId> {
    if name.eq_ignore_ascii_case("None") {
        return catalog.first_id();
    }
    catalog
        .entries()
        .iter()
        .position(|entry| {
            entry
                .shield
                .as_ref()
                .map(|shield| shield.name.eq_ignore_ascii_case(name))
                .unwrap_or(false)
        })
        .and_then(|idx| catalog.id_from_index(idx))
}

fn format_score(score: AbilityScore) -> String {
    format!("{}/{:02}", score.base, score.percentile)
}

fn format_wound_line(wounds: &[Wound]) -> String {
    if wounds.is_empty() {
        "No lasting wounds.".to_string()
    } else {
        format!(
            "Wounds: {}",
            wounds
                .iter()
                .map(|wound| wound.damage.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn format_hit_list(events: &[CombatEvent], attacker_idx: usize, defender_idx: usize) -> String {
    let hits = events
        .iter()
        .filter_map(|event| {
            if event.attacker_idx != attacker_idx || event.defender_idx != defender_idx {
                return None;
            }
            let CombatEventKind::Attack(attack) = &event.kind else {
                return None;
            };
            if attack.damage <= 0 {
                return None;
            }
            Some(attack.damage.to_string())
        })
        .collect::<Vec<_>>();
    if hits.is_empty() {
        "none".to_string()
    } else {
        hits.join(", ")
    }
}
