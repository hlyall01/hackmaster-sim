use hackmaster_sim::character::{
    AbilityScore, AbilitySet, AbilitySetFull, Progression, ProgressionTier,
};
use hackmaster_sim::core::gameplay::{
    CombatantBuilder, EnemySpawnEntry, EnemySpawner, EncounterTier, EventCatalog,
    EventSpec, LootItemEntry, LootTable, RunState, Wound, XpCurve, apply_downtime,
    apply_fight_result, apply_xp, choose_event, resolve_event_choice,
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
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
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
                std::thread::spawn(move || handle_connection(stream, demo));
            }
            Err(err) => eprintln!("Connection failed: {err}"),
        }
    }
}

#[derive(Clone)]
struct DemoApp {
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
    fn new() -> Result<Self, String> {
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

    fn new_run(&mut self, request: NewRunRequest) -> Result<DemoView, String> {
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

    fn choose_node(&mut self, node_id: usize) -> Result<DemoView, String> {
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
                let enemy_name = preview_enemy_name(&spawner, &npc_presets, &session.run_state, tier);
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
                let event_seed = derive_seed(session.run_state.run_seed, "v2-event-kind", encounter_index);
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

    fn resolve_event_choice(&mut self, choice_id: String) -> Result<DemoView, String> {
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
            session.last_log.push("The event spills into a fight.".to_string());
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

    fn start_pending_fight(&mut self) -> Result<DemoView, String> {
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
        let Some(enemy) = self
            .spawner
            .spawn_for_level(effective_enemy_level(&session.run_state, pending.tier), &mut spawn_rng)
        else {
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
        session.last_log = vec![
            "Combat started. Step one second at a time or enable auto-play.".to_string(),
        ];
        Ok(self.view())
    }

    fn fight_command(&mut self, request: FightCommandRequest) -> Result<DemoView, String> {
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
            "skip" => {
                self.advance_live_fight(120);
            }
            _ => return Err("Unknown fight command.".to_string()),
        }
        Ok(self.view())
    }

    fn claim_reward(&mut self) -> Result<DemoView, String> {
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
            items: reward
                .map(|reward| reward.items)
                .unwrap_or_default(),
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

    fn view(&self) -> DemoView {
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
            map: session.map.clone(),
            available_nodes,
            phase: session.phase.label().to_string(),
            pending_event: session.pending_event.as_ref().map(DemoEventView::from_pending),
            pending_fight: session.pending_fight.clone().map(DemoFightPreview::from_pending),
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
struct DemoView {
    has_run: bool,
    presets: Vec<String>,
    player: Option<DemoPlayerView>,
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
struct NewRunRequest {
    seed: Option<u64>,
    preset: Option<String>,
    name: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct ChooseNodeRequest {
    node_id: usize,
}

#[derive(Clone, Debug, Deserialize)]
struct EventChoiceRequest {
    choice_id: String,
}

#[derive(Clone, Debug, Deserialize)]
struct FightCommandRequest {
    command: String,
    seconds: Option<u32>,
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

fn handle_connection(mut stream: TcpStream, demo: Arc<Mutex<DemoApp>>) {
    let Ok(request) = read_request(&mut stream) else {
        return;
    };
    let response = route_request(request, demo);
    let _ = stream.write_all(response.as_bytes());
}

fn read_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    let mut buffer = [0_u8; 64 * 1024];
    let size = stream.read(&mut buffer).map_err(|err| err.to_string())?;
    let raw = String::from_utf8_lossy(&buffer[..size]).to_string();
    let mut parts = raw.split("\r\n\r\n");
    let head = parts.next().unwrap_or_default();
    let body = parts.next().unwrap_or_default().to_string();
    let mut lines = head.lines();
    let first = lines.next().ok_or_else(|| "empty request".to_string())?;
    let mut first_parts = first.split_whitespace();
    let method = first_parts.next().unwrap_or_default().to_string();
    let path = first_parts.next().unwrap_or_default().to_string();
    Ok(HttpRequest { method, path, body })
}

struct HttpRequest {
    method: String,
    path: String,
    body: String,
}

fn route_request(request: HttpRequest, demo: Arc<Mutex<DemoApp>>) -> String {
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => html_response(INDEX_HTML),
        ("GET", "/api/state") => {
            let demo = demo.lock().expect("demo lock poisoned");
            json_response(200, &demo.view())
        }
        ("POST", "/api/new-run") => {
            let parsed = serde_json::from_str::<NewRunRequest>(&request.body)
                .unwrap_or(NewRunRequest {
                    seed: None,
                    preset: None,
                    name: None,
                });
            let mut demo = demo.lock().expect("demo lock poisoned");
            match demo.new_run(parsed) {
                Ok(view) => json_response(200, &view),
                Err(err) => error_response(400, err),
            }
        }
        ("POST", "/api/choose-node") => {
            let parsed = serde_json::from_str::<ChooseNodeRequest>(&request.body);
            match parsed {
                Ok(request) => {
                    let mut demo = demo.lock().expect("demo lock poisoned");
                    match demo.choose_node(request.node_id) {
                        Ok(view) => json_response(200, &view),
                        Err(err) => error_response(400, err),
                    }
                }
                Err(err) => error_response(400, format!("Bad request: {err}")),
            }
        }
        ("POST", "/api/event-choice") => {
            let parsed = serde_json::from_str::<EventChoiceRequest>(&request.body);
            match parsed {
                Ok(request) => {
                    let mut demo = demo.lock().expect("demo lock poisoned");
                    match demo.resolve_event_choice(request.choice_id) {
                        Ok(view) => json_response(200, &view),
                        Err(err) => error_response(400, err),
                    }
                }
                Err(err) => error_response(400, format!("Bad request: {err}")),
            }
        }
        ("POST", "/api/start-fight") => {
            let mut demo = demo.lock().expect("demo lock poisoned");
            match demo.start_pending_fight() {
                Ok(view) => json_response(200, &view),
                Err(err) => error_response(400, err),
            }
        }
        ("POST", "/api/fight-command") => {
            let parsed = serde_json::from_str::<FightCommandRequest>(&request.body);
            match parsed {
                Ok(request) => {
                    let mut demo = demo.lock().expect("demo lock poisoned");
                    match demo.fight_command(request) {
                        Ok(view) => json_response(200, &view),
                        Err(err) => error_response(400, err),
                    }
                }
                Err(err) => error_response(400, format!("Bad request: {err}")),
            }
        }
        ("POST", "/api/claim-reward") => {
            let mut demo = demo.lock().expect("demo lock poisoned");
            match demo.claim_reward() {
                Ok(view) => json_response(200, &view),
                Err(err) => error_response(400, err),
            }
        }
        _ => error_response(404, "Not found".to_string()),
    }
}

fn html_response(body: &str) -> String {
    http_response(200, "text/html; charset=utf-8", body.to_string())
}

fn json_response<T: Serialize>(status: u16, body: &T) -> String {
    match serde_json::to_string(body) {
        Ok(json) => http_response(status, "application/json", json),
        Err(err) => error_response(500, err.to_string()),
    }
}

fn error_response(status: u16, message: String) -> String {
    let body = serde_json::json!({ "error": message }).to_string();
    http_response(status, "application/json", body)
}

fn http_response(status: u16, content_type: &str, body: String) -> String {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Error",
    };
    format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
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

const INDEX_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>HackMaster Ascent Demo</title>
  <style>
    :root {
      color-scheme: dark;
      --bg: #141615;
      --panel: #20231f;
      --panel-2: #29271f;
      --line: #4b4435;
      --text: #ede6d1;
      --muted: #a89f8b;
      --gold: #d5a84a;
      --red: #a94338;
      --green: #7fa26a;
      --blue: #6d8fa3;
      --steel: #778089;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      min-height: 100vh;
      background: radial-gradient(circle at 50% -20%, #34301f 0, var(--bg) 42%, #0f1110 100%);
      color: var(--text);
      font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      letter-spacing: 0;
    }
    button, input, select {
      font: inherit;
    }
    button {
      border: 1px solid #6f603d;
      background: linear-gradient(#3a3325, #251f18);
      color: var(--text);
      padding: 9px 12px;
      border-radius: 6px;
      cursor: pointer;
      font-weight: 700;
    }
    button:hover { border-color: var(--gold); color: #fff5cf; }
    button:disabled { opacity: .45; cursor: default; }
    input, select {
      width: 100%;
      border: 1px solid var(--line);
      background: #171916;
      color: var(--text);
      padding: 9px 10px;
      border-radius: 6px;
    }
    .game-shell {
      height: 100vh;
      display: grid;
      grid-template-rows: auto minmax(0, 1fr);
      gap: 12px;
      padding: 14px;
    }
    .hud {
      display: grid;
      grid-template-columns: minmax(260px, 360px) minmax(420px, 1fr) auto;
      gap: 14px;
      align-items: stretch;
      border: 1px solid #584a35;
      border-radius: 8px;
      background:
        linear-gradient(180deg, rgba(56,47,33,.98), rgba(24,26,23,.98)),
        repeating-linear-gradient(90deg, rgba(255,255,255,.04) 0 1px, transparent 1px 22px);
      box-shadow: 0 16px 42px rgba(0,0,0,.32);
      padding: 10px;
    }
    .hud-brand {
      display: flex;
      align-items: center;
      gap: 12px;
      min-width: 0;
    }
    .sigil {
      display: grid;
      place-items: center;
      width: 46px;
      height: 46px;
      border: 1px solid var(--gold);
      border-radius: 8px;
      color: #fff1c8;
      background: #181612;
      font-weight: 900;
      box-shadow: inset 0 0 18px rgba(213,168,74,.2);
    }
    .hud h1 { font-size: 18px; }
    .hud-metrics {
      display: grid;
      grid-template-columns: repeat(5, minmax(92px, 1fr));
      gap: 8px;
      align-content: stretch;
    }
    .hud-card {
      border: 1px solid rgba(213,168,74,.22);
      border-radius: 6px;
      background: rgba(0,0,0,.18);
      padding: 7px 9px;
      min-width: 0;
    }
    .hud-card span {
      display: block;
      color: var(--muted);
      font-size: 11px;
      font-weight: 800;
      text-transform: uppercase;
    }
    .hud-card strong {
      display: block;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      color: #fff1c8;
      font-size: 14px;
    }
    .hud-actions {
      display: flex;
      align-items: center;
      gap: 8px;
      justify-content: end;
    }
    .app {
      display: grid;
      grid-template-columns: 300px minmax(500px, 1fr) 390px;
      gap: 14px;
      min-height: 0;
    }
    .panel {
      background:
        linear-gradient(180deg, rgba(41,39,31,.98), rgba(28,31,28,.98)),
        repeating-linear-gradient(180deg, rgba(255,255,255,.025) 0 1px, transparent 1px 18px);
      border: 1px solid var(--line);
      border-radius: 8px;
      box-shadow: 0 18px 48px rgba(0,0,0,.28);
      overflow: hidden;
      min-height: 0;
    }
    .panel-inner { padding: 16px; }
    h1, h2, h3 { margin: 0; line-height: 1.05; }
    h1 { font-size: 20px; }
    h2 { font-size: 16px; color: var(--gold); }
    h3 { font-size: 14px; color: #d9ceb7; }
    .sub { color: var(--muted); font-size: 13px; line-height: 1.45; }
    .stack { display: grid; gap: 12px; }
    .row { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
    .section-title {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 10px;
      padding-bottom: 9px;
      border-bottom: 1px solid rgba(213,168,74,.2);
    }
    .roll-box {
      border: 1px solid rgba(169,67,56,.48);
      background:
        linear-gradient(180deg, rgba(82,34,29,.46), rgba(19,20,18,.35));
      border-radius: 8px;
      padding: 12px;
    }
    .sheet-name {
      display: grid;
      gap: 4px;
      padding: 12px;
      border: 1px solid rgba(213,168,74,.28);
      border-radius: 8px;
      background: rgba(0,0,0,.18);
    }
    .xpbar {
      height: 8px;
      border: 1px solid rgba(255,255,255,.12);
      border-radius: 999px;
      background: #151614;
      overflow: hidden;
    }
    .xpbar span {
      display: block;
      width: var(--xp, 0%);
      height: 100%;
      background: linear-gradient(90deg, #6d8fa3, #d5a84a);
    }
    .stat-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
    .stat {
      border: 1px solid rgba(213,168,74,.28);
      background: rgba(20,22,21,.58);
      border-radius: 6px;
      padding: 8px;
      font-size: 13px;
      color: #e8dfc6;
    }
    .metric {
      display: flex;
      align-items: baseline;
      justify-content: space-between;
      gap: 8px;
      border-bottom: 1px solid rgba(255,255,255,.06);
      padding: 7px 0;
      font-size: 13px;
    }
    .metric strong { color: #fff1c8; }
    .map {
      position: relative;
      height: 100%;
      padding: 18px;
    }
    .map-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      margin-bottom: 18px;
    }
    .map-grid {
      display: grid;
      grid-template-columns: repeat(4, 1fr);
      gap: 16px;
      height: calc(100% - 66px);
      align-items: center;
      border: 1px solid rgba(213,168,74,.16);
      border-radius: 8px;
      background:
        radial-gradient(circle at 50% 45%, rgba(213,168,74,.08), transparent 42%),
        linear-gradient(180deg, rgba(0,0,0,.14), rgba(0,0,0,.22));
      padding: 14px;
    }
    .node-scene {
      grid-column: 1 / -1;
      align-self: stretch;
      display: grid;
      align-content: center;
      justify-items: center;
      gap: 18px;
      min-height: 540px;
      padding: 48px;
      text-align: center;
      border: 1px solid rgba(213,168,74,.32);
      border-radius: 8px;
      background:
        linear-gradient(rgba(20,22,21,.72), rgba(20,22,21,.86)),
        radial-gradient(circle at 50% 30%, rgba(213,168,74,.16), transparent 45%);
    }
    .node-scene h2 {
      color: #f0dfb1;
      font-size: 32px;
      max-width: 720px;
    }
    .node-scene p {
      max-width: 720px;
      margin: 0;
      color: #c9bea5;
      line-height: 1.55;
      font-size: 17px;
    }
    .scene-mark {
      display: grid;
      place-items: center;
      width: 86px;
      height: 86px;
      border: 1px solid var(--gold);
      border-radius: 8px;
      background: rgba(0,0,0,.22);
      color: var(--gold);
      font-size: 44px;
      box-shadow: inset 0 0 24px rgba(213,168,74,.16);
    }
    .floor {
      display: grid;
      gap: 18px;
      align-content: center;
      min-height: 360px;
      position: relative;
    }
    .floor:not(:last-child)::after {
      content: "";
      position: absolute;
      top: 50%;
      right: -16px;
      width: 16px;
      border-top: 1px dashed rgba(213,168,74,.36);
    }
    .node {
      display: grid;
      place-items: center;
      width: 112px;
      min-height: 76px;
      border: 1px solid var(--line);
      background: #191b18;
      border-radius: 8px;
      color: var(--muted);
      margin: 0 auto;
      position: relative;
      transition: transform .15s ease, border-color .15s ease, background .15s ease;
    }
    .node.available {
      color: #fff5cf;
      border-color: var(--gold);
      background: linear-gradient(180deg, #423622, #211f19);
      transform: translateY(-2px);
    }
    .node.completed {
      color: var(--green);
      border-color: rgba(127,162,106,.75);
      background: rgba(33,48,34,.72);
    }
    .node .icon { font-size: 25px; line-height: 1; }
    .node .label { font-size: 12px; font-weight: 800; text-transform: uppercase; margin-top: 6px; }
    .node button {
      position: absolute;
      inset: 0;
      opacity: 0;
    }
    .right-scroll {
      height: 100%;
      overflow: auto;
      padding-right: 2px;
    }
    .choice-list { display: grid; gap: 8px; }
    .log {
      display: grid;
      gap: 6px;
      max-height: 220px;
      overflow: auto;
      font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
      font-size: 12px;
      color: #d7d0bc;
      background: rgba(0,0,0,.18);
      border: 1px solid rgba(255,255,255,.07);
      border-radius: 6px;
      padding: 10px;
    }
    .reward {
      border: 1px solid rgba(213,168,74,.35);
      background: rgba(213,168,74,.08);
      border-radius: 6px;
      padding: 10px;
      display: grid;
      gap: 5px;
      font-size: 13px;
    }
    .reward-scene {
      width: min(760px, 100%);
      display: grid;
      gap: 16px;
      text-align: left;
    }
    .loot-grid {
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 10px;
    }
    .loot-token {
      border: 1px solid rgba(213,168,74,.3);
      border-radius: 8px;
      background: rgba(0,0,0,.18);
      padding: 12px;
      min-height: 82px;
      display: grid;
      align-content: center;
      gap: 4px;
    }
    .loot-token strong {
      color: #fff1c8;
      font-size: 19px;
    }
    .fight {
      border: 1px solid rgba(169,67,56,.45);
      background: rgba(169,67,56,.08);
      border-radius: 6px;
      padding: 10px;
      display: grid;
      gap: 6px;
      font-size: 13px;
    }
    .combat-scene {
      width: min(860px, 100%);
      display: grid;
      gap: 18px;
      text-align: left;
    }
    .combat-title {
      display: flex;
      justify-content: space-between;
      align-items: start;
      gap: 12px;
    }
    .combat-grid {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 12px;
    }
    .combatant {
      border: 1px solid rgba(213,168,74,.28);
      background: rgba(12,13,12,.42);
      border-radius: 8px;
      padding: 12px;
      display: grid;
      gap: 8px;
    }
    .combatant.enemy {
      border-color: rgba(169,67,56,.45);
      background: rgba(45,18,16,.34);
    }
    .hpbar {
      height: 12px;
      border: 1px solid rgba(255,255,255,.12);
      border-radius: 999px;
      background: #151614;
      overflow: hidden;
    }
    .hpbar span {
      display: block;
      height: 100%;
      background: linear-gradient(90deg, #7fa26a, #d5a84a);
      width: var(--hp, 0%);
    }
    .enemy .hpbar span {
      background: linear-gradient(90deg, #a94338, #d5a84a);
    }
    .arena-track {
      position: relative;
      height: 86px;
      border: 1px solid rgba(213,168,74,.24);
      border-radius: 8px;
      background:
        linear-gradient(90deg, rgba(127,162,106,.11), transparent 36%, transparent 64%, rgba(169,67,56,.11)),
        repeating-linear-gradient(90deg, rgba(255,255,255,.055) 0 1px, transparent 1px 12.5%);
      overflow: hidden;
    }
    .fighter-token {
      position: absolute;
      top: 50%;
      transform: translate(-50%, -50%);
      width: 58px;
      height: 58px;
      display: grid;
      place-items: center;
      border-radius: 8px;
      border: 1px solid var(--gold);
      background: #171916;
      color: #fff1c8;
      font-weight: 900;
      box-shadow: 0 8px 24px rgba(0,0,0,.35);
    }
    .fighter-token.enemy {
      border-color: #b95b4f;
      color: #ffd0c9;
    }
    .combat-controls {
      display: flex;
      gap: 8px;
      flex-wrap: wrap;
      align-items: center;
    }
    .combat-log {
      min-height: 142px;
      max-height: 210px;
    }
    .decision-slot {
      border: 1px dashed rgba(213,168,74,.36);
      border-radius: 8px;
      padding: 10px;
      color: var(--muted);
      background: rgba(0,0,0,.14);
      font-size: 13px;
    }
    .pill {
      display: inline-flex;
      border: 1px solid var(--line);
      background: rgba(0,0,0,.16);
      border-radius: 999px;
      padding: 5px 8px;
      color: var(--muted);
      font-size: 12px;
      font-weight: 800;
      text-transform: uppercase;
    }
    .danger { color: #f0a096; }
    .ok { color: #a9d48e; }

    :root {
      --bg: #20120d;
      --panel: #2b1a12;
      --panel-2: #3a2217;
      --line: #7b4b2d;
      --text: #f7e8c0;
      --muted: #c9ad82;
      --gold: #f0bd55;
      --red: #b6402d;
      --green: #628f55;
      --blue: #356f86;
      --steel: #7c6d5d;
      --parchment: #c99152;
      --parchment-dark: #7b4528;
      --ink: #2b170f;
    }
    body {
      background:
        linear-gradient(180deg, rgba(32,18,13,.9), rgba(16,10,9,.96)),
        radial-gradient(ellipse at 50% 18%, rgba(83,42,25,.58), transparent 54%),
        linear-gradient(120deg, #21140f, #151412 46%, #251610);
      font-family: Georgia, Cambria, "Times New Roman", serif;
      color: var(--text);
      overflow: hidden;
    }
    body::before {
      content: "";
      position: fixed;
      inset: 0;
      pointer-events: none;
      opacity: .22;
      background:
        repeating-linear-gradient(0deg, transparent 0 10px, rgba(255,255,255,.035) 10px 11px),
        repeating-linear-gradient(90deg, transparent 0 17px, rgba(0,0,0,.08) 17px 18px);
      mix-blend-mode: soft-light;
    }
    button {
      border: 2px solid #3a1b12;
      border-radius: 10px 10px 14px 14px;
      background:
        linear-gradient(180deg, #d18a42 0%, #9d3d28 52%, #572113 100%);
      color: #ffe9af;
      padding: 10px 16px;
      font-weight: 900;
      text-shadow: 0 2px 0 rgba(0,0,0,.42);
      box-shadow:
        inset 0 2px 0 rgba(255,236,168,.36),
        inset 0 -4px 0 rgba(53,18,11,.55),
        0 6px 0 rgba(21,10,7,.82);
    }
    button:hover {
      border-color: #f1c264;
      color: #fff7d0;
      transform: translateY(-1px);
    }
    button:active {
      transform: translateY(3px);
      box-shadow:
        inset 0 2px 0 rgba(255,236,168,.3),
        inset 0 -2px 0 rgba(53,18,11,.5),
        0 2px 0 rgba(21,10,7,.82);
    }
    input, select {
      border: 2px solid #5d341e;
      background: #ead0a0;
      color: var(--ink);
      border-radius: 6px;
      font-weight: 700;
      box-shadow: inset 0 2px 4px rgba(58,27,12,.32);
    }
    .game-shell {
      padding: 10px;
      gap: 10px;
      background:
        linear-gradient(90deg, rgba(0,0,0,.28), transparent 18%, transparent 82%, rgba(0,0,0,.3));
    }
    .hud {
      border: 3px solid #2a160e;
      border-radius: 10px;
      background:
        linear-gradient(180deg, #402416, #20120c 65%, #130b08),
        repeating-linear-gradient(90deg, rgba(255,255,255,.04) 0 1px, transparent 1px 18px);
      box-shadow:
        inset 0 0 0 2px rgba(241,194,100,.24),
        0 10px 24px rgba(0,0,0,.46);
    }
    .sigil {
      border-width: 2px;
      border-radius: 50%;
      background: radial-gradient(circle at 35% 25%, #f2c769, #8d3a24 62%, #321409);
      color: #211008;
      font-weight: 900;
    }
    .hud h1 {
      color: #ffd77a;
      font-size: 24px;
      text-shadow: 0 3px 0 #3a160d;
    }
    .hud-card {
      border: 2px solid #442515;
      border-radius: 7px;
      background: linear-gradient(180deg, #51301d, #21130d);
      box-shadow: inset 0 0 0 1px rgba(255,219,134,.12);
    }
    .hud-card span,
    .pill {
      color: #d9bd88;
      letter-spacing: 0;
    }
    .hud-card strong {
      color: #fff0bc;
      font-size: 16px;
    }
    .panel {
      border: 3px solid #2b170e;
      border-radius: 10px;
      background:
        linear-gradient(180deg, rgba(74,40,23,.98), rgba(31,18,13,.98)),
        repeating-linear-gradient(45deg, rgba(255,255,255,.035) 0 1px, transparent 1px 12px);
      box-shadow:
        inset 0 0 0 2px rgba(240,189,85,.18),
        inset 0 16px 24px rgba(255,213,121,.06),
        0 16px 28px rgba(0,0,0,.44);
    }
    .panel-inner {
      padding: 14px;
    }
    h1, h2, h3 {
      font-weight: 900;
      text-shadow: 0 2px 0 rgba(0,0,0,.45);
    }
    h2 {
      color: #ffd77a;
      font-size: 18px;
    }
    h3 {
      color: #ffe8aa;
      font-size: 15px;
    }
    .sub {
      color: #ddc08d;
      font-size: 14px;
    }
    .section-title {
      border-bottom: 2px solid rgba(31,13,7,.55);
      box-shadow: 0 2px 0 rgba(255,218,132,.12);
    }
    .roll-box,
    .sheet-name,
    .stat,
    .metric,
    .reward,
    .fight,
    .combatant,
    .loot-token,
    .decision-slot {
      border: 2px solid #4a2817;
      border-radius: 8px;
      background:
        linear-gradient(180deg, rgba(240,200,137,.16), rgba(35,19,12,.34)),
        #2f1b12;
      box-shadow: inset 0 0 0 1px rgba(255,229,161,.12);
    }
    .sheet-name {
      background:
        linear-gradient(180deg, #d69d5a, #7d4125 72%, #3b1c10);
      color: #fff2c2;
    }
    .metric {
      padding: 9px 10px;
      border-bottom: 2px solid #4a2817;
    }
    .stat {
      color: #ffe9b0;
      text-align: center;
      font-weight: 900;
    }
    .xpbar,
    .hpbar {
      height: 14px;
      border: 2px solid #2a150c;
      background: #26140d;
      box-shadow: inset 0 2px 4px rgba(0,0,0,.55);
    }
    .xpbar span {
      background: linear-gradient(90deg, #2789a0, #f0bd55);
    }
    .hpbar span {
      background: linear-gradient(90deg, #63a85d, #f0bd55);
    }
    .enemy .hpbar span {
      background: linear-gradient(90deg, #ba402d, #f0bd55);
    }
    .map {
      padding: 14px;
    }
    .map-grid {
      border: 4px solid #2d180f;
      border-radius: 12px;
      background:
        linear-gradient(rgba(111,65,36,.12), rgba(111,65,36,.12)),
        repeating-linear-gradient(35deg, rgba(56,31,16,.07) 0 2px, transparent 2px 16px),
        radial-gradient(ellipse at 50% 52%, #d7a45f 0, #b9773c 52%, #71391f 100%);
      box-shadow:
        inset 0 0 0 3px rgba(255,221,136,.22),
        inset 0 0 80px rgba(60,26,11,.65);
    }
    .node-scene {
      border: 0;
      border-radius: 0;
      background:
        radial-gradient(ellipse at 50% 45%, rgba(255,226,152,.22), transparent 48%),
        linear-gradient(180deg, rgba(80,43,24,.15), rgba(43,22,13,.34));
      box-shadow: none;
    }
    .node-scene h2 {
      color: #ffe5a2;
      font-size: 36px;
      text-shadow: 0 4px 0 #35160b;
    }
    .node-scene p {
      color: #ffe3ae;
      font-size: 18px;
      text-shadow: 0 2px 0 rgba(35,13,6,.55);
    }
    .scene-mark {
      border: 4px solid #2b160d;
      border-radius: 50%;
      background: radial-gradient(circle at 35% 25%, #ffe08a, #b7402d 58%, #35140b);
      color: #210e08;
      font-size: 38px;
      font-weight: 900;
      box-shadow:
        inset 0 0 0 2px rgba(255,237,177,.38),
        0 9px 0 rgba(41,18,10,.8);
    }
    .floor {
      gap: 24px;
    }
    .floor:not(:last-child)::after {
      right: -18px;
      width: 22px;
      border-top: 4px dotted rgba(74,36,18,.58);
      filter: drop-shadow(0 1px 0 rgba(255,223,145,.25));
    }
    .node {
      width: 88px;
      min-height: 88px;
      border: 4px solid #2d160c;
      border-radius: 50%;
      background: radial-gradient(circle at 35% 25%, #f6d27d, #7a3b23 72%, #2b140b);
      color: #fff0bd;
      box-shadow:
        inset 0 0 0 2px rgba(255,238,183,.32),
        0 8px 0 rgba(40,18,10,.78),
        0 14px 24px rgba(0,0,0,.35);
    }
    .node.kind-event {
      background: radial-gradient(circle at 35% 25%, #f6e0a2, #7b5630 70%, #2b170d);
    }
    .node.kind-rest {
      background: radial-gradient(circle at 35% 25%, #d7f08a, #466b33 70%, #172211);
    }
    .node.kind-elite {
      background: radial-gradient(circle at 35% 25%, #ffd76f, #a15b19 66%, #301407);
    }
    .node.kind-boss {
      background: radial-gradient(circle at 35% 25%, #e1b5ff, #56336d 68%, #201028);
    }
    .node.available {
      color: #fff8d8;
      border-color: #ffe18a;
      background: radial-gradient(circle at 35% 25%, #fff0a8, #bd4930 66%, #43190d);
      transform: translateY(-5px) scale(1.05);
      box-shadow:
        inset 0 0 0 2px rgba(255,247,207,.5),
        0 10px 0 rgba(40,18,10,.78),
        0 0 22px rgba(255,205,96,.42);
    }
    .node.completed {
      color: #d7c7a1;
      filter: saturate(.55);
      opacity: .72;
    }
    .node .icon {
      font-size: 28px;
      font-weight: 900;
    }
    .node .label {
      font-size: 11px;
      color: #fff0bd;
      text-shadow: 0 2px 0 rgba(0,0,0,.55);
    }
    .arena-track {
      height: 146px;
      border: 4px solid #2d160c;
      border-radius: 12px;
      background:
        linear-gradient(180deg, rgba(36,62,70,.62), transparent 38%),
        linear-gradient(180deg, #684326 0%, #8a522a 52%, #3b2114 100%);
      box-shadow:
        inset 0 0 0 2px rgba(255,224,146,.18),
        inset 0 -34px 42px rgba(49,23,11,.65);
    }
    .arena-track::after {
      content: "";
      position: absolute;
      left: 5%;
      right: 5%;
      bottom: 22px;
      border-bottom: 5px dotted rgba(44,22,12,.48);
    }
    .fighter-token {
      width: 70px;
      height: 88px;
      border: 4px solid #29150b;
      border-radius: 36px 36px 18px 18px;
      background: radial-gradient(circle at 42% 24%, #ffeab6, #b0412b 58%, #3b170c);
      color: #fff5c8;
      font-size: 19px;
      text-shadow: 0 2px 0 rgba(0,0,0,.6);
      box-shadow:
        inset 0 0 0 2px rgba(255,236,172,.28),
        0 10px 0 rgba(42,18,10,.8),
        0 16px 26px rgba(0,0,0,.42);
      z-index: 1;
    }
    .fighter-token.enemy {
      background: radial-gradient(circle at 42% 24%, #ffd6bf, #6f3527 62%, #21100b);
    }
    .fighter-token::after {
      content: "";
      position: absolute;
      left: 10%;
      right: 10%;
      bottom: -15px;
      height: 10px;
      border-radius: 50%;
      background: rgba(26,11,6,.58);
      filter: blur(2px);
      z-index: -1;
    }
    .combat-scene,
    .reward-scene {
      width: min(900px, 100%);
    }
    .combat-title {
      padding: 4px 0 8px;
      border-bottom: 3px solid rgba(45,22,12,.42);
    }
    .combatant.enemy {
      background:
        linear-gradient(180deg, rgba(186,64,45,.18), rgba(35,18,13,.54)),
        #2f1b12;
    }
    .combat-controls {
      justify-content: center;
    }
    .log {
      border: 2px solid #30180d;
      background: rgba(28,14,8,.74);
      color: #efd49b;
      border-radius: 8px;
    }
    .pill {
      border: 2px solid #4a2817;
      border-radius: 7px;
      background: linear-gradient(180deg, #4a2a19, #21120c);
      color: #f4d98d;
      font-size: 11px;
    }
    @media (max-width: 1050px) {
      body { overflow: auto; }
      .game-shell { height: auto; min-height: 100vh; }
      .hud { grid-template-columns: 1fr; }
      .hud-metrics { grid-template-columns: repeat(2, minmax(0, 1fr)); }
      .hud-actions { justify-content: stretch; }
      .hud-actions button { width: 100%; }
      .app { grid-template-columns: 1fr; }
      .map { height: auto; min-height: 560px; }
      .right-scroll { height: auto; }
      .combat-grid { grid-template-columns: 1fr; }
      .loot-grid { grid-template-columns: 1fr; }
      .floor:not(:last-child)::after { display: none; }
    }
  </style>
</head>
<body>
  <main class="game-shell">
    <header class="hud">
      <div class="hud-brand">
        <span class="sigil">HM</span>
        <div>
          <h1>HackMaster Ascent</h1>
          <div id="phase" class="sub">Roll a character to begin.</div>
        </div>
      </div>
      <div id="hudMetrics" class="hud-metrics"></div>
      <div class="hud-actions">
        <span id="runStatus" class="pill">No run</span>
        <button onclick="newRun()">New Run</button>
      </div>
    </header>
    <div class="app">
      <aside class="panel"><div class="panel-inner stack">
        <div class="section-title">
          <h2>Character Sheet</h2>
          <span class="pill">Run</span>
        </div>
        <div class="roll-box stack">
          <h2>Roll Character</h2>
          <select id="preset"></select>
          <input id="name" placeholder="Name override" />
          <input id="seed" placeholder="Seed, blank for random" />
          <button onclick="newRun()">Embark</button>
        </div>
        <div id="character" class="stack"></div>
      </div></aside>
      <section class="panel map">
        <div class="map-header">
          <div>
            <h2>Route</h2>
            <div class="sub">Branching run map and active encounter scene.</div>
          </div>
          <span id="routeStatus" class="pill">Route</span>
        </div>
        <div id="map" class="map-grid"></div>
      </section>
      <aside class="right-scroll stack">
        <section class="panel"><div class="panel-inner stack" id="encounter"></div></section>
        <section class="panel"><div class="panel-inner stack">
          <div class="section-title"><h2>Latest Reward</h2><span class="pill">Loot</span></div>
          <div id="reward"></div>
        </div></section>
        <section class="panel"><div class="panel-inner stack">
          <div class="section-title"><h2>Fight Summary</h2><span class="pill">Combat</span></div>
          <div id="fight"></div>
        </div></section>
        <section class="panel"><div class="panel-inner stack">
          <div class="section-title"><h2>Log</h2><span class="pill">History</span></div>
          <div id="log" class="log"></div>
        </div></section>
      </aside>
    </div>
  </main>
  <script>
    let state = null;
    let autoTimer = null;
    let autoBusy = false;
    const nodeIcons = { fight: "F", event: "?", rest: "R", elite: "E", boss: "B" };
    const nodeLabels = { fight: "Fight", event: "Event", rest: "Rest", elite: "Elite", boss: "Boss" };

    async function api(path, body) {
      const options = body === undefined ? {} : {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body)
      };
      const response = await fetch(path, options);
      const json = await response.json();
      if (!response.ok) throw new Error(json.error || "Request failed");
      state = json;
      render();
    }

    async function loadState() { await api("/api/state"); }
    async function newRun() {
      const seedRaw = document.getElementById("seed").value.trim();
      await api("/api/new-run", {
        preset: document.getElementById("preset").value,
        name: document.getElementById("name").value.trim() || null,
        seed: seedRaw ? Number(seedRaw) : null
      });
    }
    async function chooseNode(id) { await api("/api/choose-node", { node_id: id }); }
    async function eventChoice(id) { await api("/api/event-choice", { choice_id: id }); }
    async function startFight() { await api("/api/start-fight", {}); }
    async function claimReward() { await api("/api/claim-reward", {}); }
    async function fightCommand(command, seconds = 1) {
      await api("/api/fight-command", { command, seconds });
    }
    async function autoTick() {
      if (autoBusy) return;
      autoBusy = true;
      try {
        await fightCommand("tick", 1);
      } catch (err) {
        stopAutoTimer();
        renderError(err);
      } finally {
        autoBusy = false;
      }
    }
    function syncAutoTimer() {
      const shouldRun = Boolean(state && state.live_fight && state.live_fight.running);
      if (shouldRun && !autoTimer) {
        autoTimer = setInterval(autoTick, 1000);
      }
      if (!shouldRun) stopAutoTimer();
    }
    function stopAutoTimer() {
      if (autoTimer) {
        clearInterval(autoTimer);
        autoTimer = null;
      }
    }

    function render() {
      renderPresets();
      renderHud();
      renderCharacter();
      renderMap();
      renderEncounter();
      renderReward();
      renderFight();
      renderLog();
      syncAutoTimer();
    }

    function renderPresets() {
      const select = document.getElementById("preset");
      const previous = select.value;
      select.innerHTML = (state.presets || []).map(name => `<option value="${escapeHtml(name)}">${escapeHtml(name)}</option>`).join("");
      if (previous) select.value = previous;
    }

    function renderHud() {
      document.getElementById("phase").textContent = state.terminal || phaseText(state.phase);
      document.getElementById("runStatus").textContent = state.has_run ? phaseLabel(state.phase) : "No run";
      const metrics = document.getElementById("hudMetrics");
      if (!state.player) {
        metrics.innerHTML = [
          hudCard("Level", "-"),
          hudCard("XP", "-"),
          hudCard("Gold", "-"),
          hudCard("Depth", "-"),
          hudCard("Wounds", "-")
        ].join("");
        return;
      }
      const p = state.player;
      metrics.innerHTML = [
        hudCard("Level", p.level),
        hudCard("XP", `${p.xp}/${p.next_level_xp}`),
        hudCard("Gold", p.gold),
        hudCard("Depth", p.depth),
        hudCard("Wounds", p.wound_total || "none")
      ].join("");
    }

    function hudCard(label, value) {
      return `<div class="hud-card"><span>${escapeHtml(label)}</span><strong>${escapeHtml(value)}</strong></div>`;
    }

    function renderCharacter() {
      const el = document.getElementById("character");
      if (!state.player) {
        el.innerHTML = `<div class="sub">No character rolled.</div>`;
        return;
      }
      const p = state.player;
      const xpPct = clamp((p.xp / Math.max(1, p.next_level_xp)) * 100, 0, 100);
      el.innerHTML = `
        <div class="stack">
          <div class="sheet-name">
            <div class="row"><h2>${escapeHtml(p.name)}</h2><span class="pill">Level ${p.level}</span></div>
            <div class="xpbar" style="--xp:${xpPct}%"><span></span></div>
            <div class="sub">XP ${p.xp} / ${p.next_level_xp}</div>
          </div>
          ${metricRow("Gold", p.gold)}
          ${metricRow("Depth", p.depth)}
          ${metricRow("Wounds", `<strong class="${p.wound_total ? "danger" : "ok"}">${p.wound_total || "none"}</strong>`)}
          ${metricRow("Seed", p.seed)}
          <div class="stat-grid">${p.stats.map(s => `<div class="stat">${escapeHtml(s)}</div>`).join("")}</div>
          <div class="sub">Points: BP ${p.bp}, LP ${p.lp}, AP ${p.ap}, RP ${p.rp}</div>
        </div>`;
    }

    function metricRow(label, value) {
      const rendered = typeof value === "string" && value.includes("<strong") ? value : `<strong>${escapeHtml(value)}</strong>`;
      return `<div class="metric"><span>${escapeHtml(label)}</span>${rendered}</div>`;
    }

    function renderMap() {
      document.getElementById("routeStatus").textContent = state.has_run ? phaseLabel(state.phase) : "Route";
      const map = document.getElementById("map");
      if (state.live_fight) {
        map.innerHTML = renderLiveFightScene(state.live_fight);
        return;
      }
      if (state.phase === "reward_review" && state.last_reward) {
        map.innerHTML = renderRewardScene();
        return;
      }
      if (state.pending_event) {
        const event = state.pending_event;
        map.innerHTML = `<div class="node-scene event-scene">
          <div class="scene-mark">?</div>
          <h2>${escapeHtml(event.name)}</h2>
          <p>${escapeHtml(event.description)}</p>
          <div class="choice-list">${event.choices.map(c => `<button onclick="eventChoice('${escapeJs(c.id)}')">${escapeHtml(c.text)}</button>`).join("")}</div>
        </div>`;
        return;
      }
      if (state.pending_fight) {
        const fight = state.pending_fight;
        map.innerHTML = `<div class="node-scene fight-scene">
          <div class="scene-mark">F</div>
          <h2>${escapeHtml(fight.tier)} Fight</h2>
          <p>You have committed to a route and are sizing up ${escapeHtml(fight.enemy_name)}. Combat will move to a timeline from here.</p>
          <button onclick="startFight()">Fight</button>
        </div>`;
        return;
      }
      const floors = [0,1,2,3];
      map.innerHTML = floors.map(floor => {
        const nodes = (state.map || []).filter(n => n.floor === floor);
        return `<div class="floor">${nodes.map(nodeHtml).join("")}</div>`;
      }).join("");
    }

    function nodeHtml(node) {
      const available = (state.available_nodes || []).includes(node.id);
      const classes = ["node", `kind-${node.kind}`, available ? "available" : "", node.completed ? "completed" : ""].join(" ");
      const disabled = available ? "" : "disabled";
      return `<div class="${classes}">
        <div class="icon">${nodeIcons[node.kind]}</div>
        <div class="label">${nodeLabels[node.kind]}</div>
        <button ${disabled} onclick="chooseNode(${node.id})">Choose ${nodeLabels[node.kind]}</button>
      </div>`;
    }

    function renderRewardScene() {
      const reward = state.last_reward;
      const fight = state.last_fight;
      return `<div class="node-scene">
        <div class="reward-scene">
          <div class="combat-title">
            <div>
              <h2>${fight && fight.won ? "Victory Spoils" : "Encounter Reward"}</h2>
              <p>${fight ? `Defeated ${escapeHtml(fight.enemy)} in ${fight.turns}s.` : "Resolve the reward before choosing the next route."}</p>
            </div>
            <span class="pill">Reward</span>
          </div>
          <div class="loot-grid">
            <div class="loot-token"><span class="sub">Gold</span><strong>+${reward.gold}</strong></div>
            <div class="loot-token"><span class="sub">XP</span><strong>+${reward.xp}</strong></div>
            <div class="loot-token"><span class="sub">Level</span><strong>${reward.level_gained ? "Gained" : "Held"}</strong></div>
          </div>
          <div class="reward">${rewardDetails(reward)}</div>
          <div class="combat-controls"><button onclick="claimReward()">Continue Route</button></div>
        </div>
      </div>`;
    }

    function renderLiveFightScene(fight) {
      const player = fight.combatants.find(c => c.team_id === 0) || fight.combatants[0];
      const enemy = fight.combatants.find(c => c.team_id === 1) || fight.combatants[1];
      const distance = Number(fight.distance_ft || 0).toFixed(1);
      const enemyPosition = arenaEnemyPosition(fight.distance_ft);
      return `<div class="node-scene fight-scene">
        <div class="combat-scene">
          <div class="combat-title">
            <div>
              <h2>${escapeHtml(fight.tier)} Fight: ${escapeHtml(fight.enemy_name)}</h2>
              <p>${fight.elapsed_seconds}s elapsed of ${fight.max_seconds}s. Range ${distance} ft. Status: ${escapeHtml(fight.status)}.</p>
            </div>
            <span class="pill">${fight.running ? "Auto" : "Paused"}</span>
          </div>
          <div class="arena-track">
            <div class="fighter-token" style="left: 18%">${initials(player && player.name)}</div>
            <div class="fighter-token enemy" style="left: ${enemyPosition}%">${initials(enemy && enemy.name)}</div>
          </div>
          <div class="combat-grid">
            ${combatantCard(player, false)}
            ${combatantCard(enemy, true)}
          </div>
          <div class="combat-controls">
            <button onclick="fightCommand('step', 1)">Step 1s</button>
            <button onclick="fightCommand('play', 1)" ${fight.running ? "disabled" : ""}>Auto</button>
            <button onclick="fightCommand('pause', 1)" ${fight.running ? "" : "disabled"}>Pause</button>
            <button onclick="fightCommand('skip', 1)">Finish</button>
          </div>
          ${decisionHtml(fight.pending_decision)}
          <div class="log combat-log">${(fight.log_tail || []).map(escapeHtml).join("<br>") || "Combat is about to begin."}</div>
        </div>
      </div>`;
    }

    function combatantCard(combatant, enemy) {
      if (!combatant) return `<div class="combatant"><div class="sub">No combatant.</div></div>`;
      const hp = `${combatant.hp} / ${combatant.max_hp}`;
      const hpPct = clamp((combatant.hp / Math.max(1, combatant.max_hp)) * 100, 0, 100);
      const tags = [
        combatant.weapon,
        `${formatSeconds(combatant.weapon_speed_seconds)}s speed`,
        `${formatFeet(combatant.reach_ft)} reach`,
        combatant.next_attack_in_seconds === null || combatant.next_attack_in_seconds === undefined
          ? null
          : `next ${formatSeconds(combatant.next_attack_in_seconds)}s`,
        shieldLabel(combatant),
        combatant.trauma_seconds > 0 ? `${combatant.trauma_seconds}s trauma` : null,
        combatant.knocked_seconds > 0 ? `${combatant.knocked_seconds}s knocked` : null
      ].filter(Boolean);
      return `<div class="combatant ${enemy ? "enemy" : ""}">
        <div class="row"><h3>${escapeHtml(combatant.name)}</h3><strong>${hp}</strong></div>
        <div class="hpbar" style="--hp:${hpPct}%"><span></span></div>
        <div class="sub">${tags.map(escapeHtml).join(" | ")}</div>
      </div>`;
    }

    function shieldLabel(combatant) {
      if (!combatant || !combatant.shield_name) return null;
      return combatant.shield_intact
        ? `${combatant.shield_name} ready`
        : `${combatant.shield_name} broken`;
    }

    function decisionHtml(decision) {
      if (!decision) {
        return `<div class="decision-slot">No tactical prompt this second.</div>`;
      }
      return `<div class="decision-slot">
        <strong>Decision for actor ${decision.actor_idx}</strong>
        <div class="combat-controls">${decision.options.map(option => `<button>${escapeHtml(option)}</button>`).join("")}</div>
      </div>`;
    }

    function renderEncounter() {
      const el = document.getElementById("encounter");
      if (state.terminal) {
        el.innerHTML = `<h2>Run Complete</h2><div class="sub">${escapeHtml(state.terminal)}</div><button onclick="newRun()">Roll Again</button>`;
        return;
      }
      if (state.live_fight) {
        const fight = state.live_fight;
        el.innerHTML = `<h2>Live Combat</h2>
          <div class="sub">${escapeHtml(fight.enemy_name)} is active at ${fight.elapsed_seconds}s. Watch the log, step time forward, or let auto-play tick.</div>
          <div class="combat-controls">
            <button onclick="fightCommand('step', 1)">Step 1s</button>
            <button onclick="fightCommand('play', 1)" ${fight.running ? "disabled" : ""}>Auto</button>
            <button onclick="fightCommand('pause', 1)" ${fight.running ? "" : "disabled"}>Pause</button>
            <button onclick="fightCommand('skip', 1)">Finish</button>
          </div>`;
        return;
      }
      if (state.pending_event) {
        const event = state.pending_event;
        el.innerHTML = `<h2>${escapeHtml(event.name)}</h2>
          <div class="sub">${escapeHtml(event.description)}</div>
          <div class="choice-list">${event.choices.map(c => `<button onclick="eventChoice('${escapeJs(c.id)}')">${escapeHtml(c.text)}</button>`).join("")}</div>`;
        return;
      }
      if (state.pending_fight) {
        const fight = state.pending_fight;
        el.innerHTML = `<h2>${escapeHtml(fight.tier)} Fight</h2>
          <div class="sub">Enemy scouted: ${escapeHtml(fight.enemy_name)}. This is now a fight scene, not an instant node resolution.</div>
          <button onclick="startFight()">Fight</button>`;
        return;
      }
      if (state.phase === "reward_review" && state.last_reward) {
        el.innerHTML = `<div class="section-title"><h2>Reward Review</h2><span class="pill">Claim</span></div>
          <div class="reward">${rewardDetails(state.last_reward)}</div>
          <button onclick="claimReward()">Continue Route</button>`;
        return;
      }
      if (state.phase === "choose_node") {
        el.innerHTML = `<h2>Choose Node</h2><div class="sub">Pick an available route node on the map. Fights resolve through the existing HackMaster combat engine.</div>`;
      } else {
        el.innerHTML = `<h2>Encounter</h2><div class="sub">Roll a character or resolve the pending choice.</div>`;
      }
    }

    function renderReward() {
      const el = document.getElementById("reward");
      if (!state.last_reward) {
        el.innerHTML = `<div class="sub">No reward yet.</div>`;
        return;
      }
      const r = state.last_reward;
      el.innerHTML = `<div class="reward">
        ${rewardDetails(r)}
        ${state.phase === "reward_review" ? `<button onclick="claimReward()">Continue Route</button>` : ""}
      </div>`;
    }

    function rewardDetails(r) {
      return `<div>Gold +${r.gold}</div>
        <div>XP +${r.xp}</div>
        <div>Items: ${r.items.length ? r.items.map(escapeHtml).join(", ") : "none"}</div>
        <div>${r.level_gained ? "Level gained. Points granted." : "No level-up."}</div>`;
    }

    function renderFight() {
      const el = document.getElementById("fight");
      if (state.live_fight) {
        const fight = state.live_fight;
        el.innerHTML = `<div class="fight">
          <div><strong>${escapeHtml(fight.status)}</strong> vs ${escapeHtml(fight.enemy_name)}</div>
          <div>${fight.elapsed_seconds}s elapsed | ${Number(fight.distance_ft || 0).toFixed(1)} ft range</div>
          <div class="log">${(fight.log_tail || []).map(escapeHtml).join("<br>") || "No strikes yet."}</div>
        </div>`;
        return;
      }
      if (!state.last_fight) {
        el.innerHTML = `<div class="sub">No fight resolved yet.</div>`;
        return;
      }
      const f = state.last_fight;
      el.innerHTML = `<div class="fight">
        <div><strong class="${f.won ? "ok" : "danger"}">${f.won ? "Victory" : "Defeat"}</strong> vs ${escapeHtml(f.enemy)}</div>
        <div>Turns: ${f.turns}s | HP left: ${f.remaining_hp}</div>
        <div>Hits dealt: ${escapeHtml(f.hits_dealt)}</div>
        <div>Hits taken: ${escapeHtml(f.hits_taken)}</div>
        <div class="log">${f.combat_log.map(escapeHtml).join("<br>")}</div>
      </div>`;
    }

    function renderLog() {
      const el = document.getElementById("log");
      el.innerHTML = (state.last_log || []).map(escapeHtml).join("<br>") || "No log.";
    }

    function phaseText(phase) {
      if (phase === "choose_node") return "Choose a route node.";
      if (phase === "event_choice") return "Resolve the event choice.";
      if (phase === "fight_preview") return "Fight scene selected.";
      if (phase === "combat_playback") return "Combat is running second by second.";
      if (phase === "reward_review") return "Claim rewards and progression.";
      if (phase === "run_over") return "Run over.";
      return "Roll a character to begin.";
    }
    function phaseLabel(phase) {
      return String(phase || "No run").replace(/_/g, " ");
    }
    function arenaEnemyPosition(distanceFt) {
      const playerPosition = 18;
      const contactPosition = playerPosition + 7;
      const farPosition = 82;
      const maxVisualRange = 20;
      const meleeRange = 1;
      const distance = clamp(Number(distanceFt || 0), 0, maxVisualRange);
      if (distance <= meleeRange) return contactPosition;
      return contactPosition + ((distance - meleeRange) / (maxVisualRange - meleeRange)) * (farPosition - contactPosition);
    }
    function clamp(value, min, max) {
      return Math.max(min, Math.min(max, Number.isFinite(value) ? value : min));
    }
    function initials(value) {
      return String(value || "?")
        .split(/\s+/)
        .filter(Boolean)
        .slice(0, 2)
        .map(part => part[0].toUpperCase())
        .join("") || "?";
    }
    function renderError(err) {
      const el = document.getElementById("log");
      if (el) el.textContent = err.message;
    }
    function formatSeconds(value) {
      const number = Number(value || 0);
      return Number.isInteger(number) ? String(number) : number.toFixed(1);
    }
    function formatFeet(value) {
      const number = Number(value || 0);
      return `${Number.isInteger(number) ? number : number.toFixed(1)}ft`;
    }

    function escapeHtml(value) {
      return String(value).replace(/[&<>"']/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
    }
    function escapeJs(value) {
      return String(value).replace(/\\/g, "\\\\").replace(/'/g, "\\'");
    }
    loadState().catch(err => {
      document.getElementById("log").textContent = err.message;
    });
  </script>
</body>
</html>"#;
