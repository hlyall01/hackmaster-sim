//! Route nodes and enemy squad generation.

use crate::core::rng::{SimRng, derive_seed};
use rand::Rng;
use serde::{Deserialize, Serialize};

pub const DEFAULT_ROUTE_SEED: u64 = 0x5155_4144_524f_5554;
pub const DEFAULT_ROUTE_FLOORS: u32 = 4;
pub const DEFAULT_ROUTE_LANES: u32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SquadNodeKind {
    Fight,
    Recruit,
    Event,
    Elite,
    Boss,
    Rest,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SquadEncounterTier {
    #[default]
    Normal,
    Elite,
    Boss,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct SquadNodeReward {
    pub gold_min: u32,
    pub gold_max: u32,
    pub xp_per_survivor: u32,
    pub recruit_chance_percent: u8,
    pub item_chance_percent: u8,
    pub reward_multiplier_percent: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct SquadEventDescriptor {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub outcome: SquadEventOutcomeDescriptor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SquadEventOutcomeKind {
    Gold,
    RecruitLead,
    Scout,
    Ambush,
    Training,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct SquadEventOutcomeDescriptor {
    pub kind: SquadEventOutcomeKind,
    pub label: String,
    pub gold_delta: i32,
    pub xp_bonus: u32,
    pub recruit_bonus_percent: u8,
    pub risk: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct SquadRestDescriptor {
    pub label: String,
    pub heal_percent: u8,
    pub clears_downed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct EnemySquadTemplate {
    pub id: String,
    pub label: String,
    pub tier: SquadEncounterTier,
    pub depth: u32,
    pub squad_size: usize,
    pub base_level: u8,
    pub level_spread: u8,
    pub preset_offset: usize,
    pub reward_multiplier_percent: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct SquadRouteNode {
    pub id: usize,
    pub floor: u32,
    pub lane: u32,
    pub kind: SquadNodeKind,
    pub completed: bool,
    #[serde(default)]
    pub required_depth: u32,
    #[serde(default)]
    pub tier: SquadEncounterTier,
    #[serde(default)]
    pub difficulty: u32,
    #[serde(default)]
    pub reward: SquadNodeReward,
    #[serde(default)]
    pub event: Option<SquadEventDescriptor>,
    #[serde(default)]
    pub rest: Option<SquadRestDescriptor>,
    #[serde(default)]
    pub enemy_template: Option<EnemySquadTemplate>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct SquadRouteGenerationConfig {
    pub floors_before_boss: u32,
    pub lanes_per_floor: u32,
}

impl Default for SquadRouteGenerationConfig {
    fn default() -> Self {
        Self {
            floors_before_boss: DEFAULT_ROUTE_FLOORS,
            lanes_per_floor: DEFAULT_ROUTE_LANES,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct SquadRouteMap {
    pub seed: u64,
    pub floors_before_boss: u32,
    pub lanes_per_floor: u32,
    pub nodes: Vec<SquadRouteNode>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SquadRouteStatusKind {
    Empty,
    InProgress,
    Complete,
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct SquadRouteStatus {
    pub kind: SquadRouteStatusKind,
    pub current_floor: Option<u32>,
    pub available_nodes: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SquadRunOverReason {
    RouteComplete,
    CompanyDefeated,
}

pub fn placeholder_route() -> Vec<SquadRouteNode> {
    generate_route(DEFAULT_ROUTE_SEED)
}

pub fn generate_route(seed: u64) -> Vec<SquadRouteNode> {
    generate_route_map(seed, SquadRouteGenerationConfig::default()).nodes
}

pub fn generate_route_map(seed: u64, config: SquadRouteGenerationConfig) -> SquadRouteMap {
    let floors_before_boss = config.floors_before_boss.max(1);
    let lanes_per_floor = config.lanes_per_floor.max(1);
    let mut nodes = Vec::with_capacity((floors_before_boss * lanes_per_floor + 1) as usize);
    let mut next_id = 0;

    for floor in 0..floors_before_boss {
        let mut kinds = floor_kinds(seed, floor, lanes_per_floor);
        ensure_floor_has_fight(&mut kinds);
        for (lane, kind) in kinds.into_iter().enumerate() {
            nodes.push(build_node(seed, next_id, floor, lane as u32, floor, kind));
            next_id += 1;
        }
    }

    nodes.push(build_node(
        seed,
        next_id,
        floors_before_boss,
        lanes_per_floor / 2,
        floors_before_boss,
        SquadNodeKind::Boss,
    ));

    SquadRouteMap {
        seed,
        floors_before_boss,
        lanes_per_floor,
        nodes,
    }
}

pub fn available_node_ids(route: &[SquadRouteNode], depth: u32) -> Vec<usize> {
    let Some(current_floor) = current_floor_for_depth(route, depth) else {
        return Vec::new();
    };
    route
        .iter()
        .filter(|node| {
            !node.completed && node.floor == current_floor && node.required_depth <= depth
        })
        .map(|node| node.id)
        .collect()
}

pub fn route_status(route: &[SquadRouteNode], depth: u32) -> SquadRouteStatus {
    if route.is_empty() {
        return SquadRouteStatus {
            kind: SquadRouteStatusKind::Empty,
            current_floor: None,
            available_nodes: Vec::new(),
        };
    }
    if is_route_complete(route) {
        return SquadRouteStatus {
            kind: SquadRouteStatusKind::Complete,
            current_floor: final_floor(route),
            available_nodes: Vec::new(),
        };
    }

    let current_floor = current_floor_for_depth(route, depth);
    let available_nodes = available_node_ids(route, depth);
    let kind = if available_nodes.is_empty() {
        SquadRouteStatusKind::Blocked
    } else {
        SquadRouteStatusKind::InProgress
    };

    SquadRouteStatus {
        kind,
        current_floor,
        available_nodes,
    }
}

pub fn with_node_completed(route: &[SquadRouteNode], node_id: usize) -> Vec<SquadRouteNode> {
    let mut route = route.to_vec();
    if let Some(node) = route.iter_mut().find(|node| node.id == node_id) {
        node.completed = true;
    }
    route
}

pub fn is_route_complete(route: &[SquadRouteNode]) -> bool {
    route
        .iter()
        .any(|node| node.kind == SquadNodeKind::Boss && node.completed)
}

pub fn run_over_reason(
    route: &[SquadRouteNode],
    active_squad_count: usize,
) -> Option<SquadRunOverReason> {
    if active_squad_count == 0 {
        Some(SquadRunOverReason::CompanyDefeated)
    } else if is_route_complete(route) {
        Some(SquadRunOverReason::RouteComplete)
    } else {
        None
    }
}

pub fn enemy_squad_template_for_node(node: &SquadRouteNode) -> Option<EnemySquadTemplate> {
    match node.kind {
        SquadNodeKind::Fight | SquadNodeKind::Elite | SquadNodeKind::Boss => {
            Some(enemy_squad_template(node.tier, node.required_depth))
        }
        SquadNodeKind::Recruit | SquadNodeKind::Event | SquadNodeKind::Rest => None,
    }
}

pub fn enemy_squad_template(tier: SquadEncounterTier, depth: u32) -> EnemySquadTemplate {
    let squad_size = match tier {
        SquadEncounterTier::Normal => 2 + (depth as usize % 3),
        SquadEncounterTier::Elite => 4 + (depth as usize % 3).min(2),
        SquadEncounterTier::Boss => 6,
    }
    .min(6);
    let base_level = 1_u8
        .saturating_add((depth / 2) as u8)
        .saturating_add(tier_level_bonus(tier));
    let reward_multiplier_percent = reward_multiplier_percent(tier);
    let id = format!(
        "{}-depth-{}",
        match tier {
            SquadEncounterTier::Normal => "warband",
            SquadEncounterTier::Elite => "elite-warband",
            SquadEncounterTier::Boss => "boss-retinue",
        },
        depth
    );
    let label = match tier {
        SquadEncounterTier::Normal => "Patrol Warband",
        SquadEncounterTier::Elite => "Veteran Warband",
        SquadEncounterTier::Boss => "Boss Retinue",
    };

    EnemySquadTemplate {
        id,
        label: label.to_string(),
        tier,
        depth,
        squad_size,
        base_level,
        level_spread: if tier == SquadEncounterTier::Boss {
            2
        } else {
            1
        },
        preset_offset: (depth as usize).saturating_mul(2)
            + match tier {
                SquadEncounterTier::Normal => 0,
                SquadEncounterTier::Elite => 3,
                SquadEncounterTier::Boss => 7,
            },
        reward_multiplier_percent,
    }
}

pub fn enemy_squad_templates_for_depth(depth: u32) -> Vec<EnemySquadTemplate> {
    [
        SquadEncounterTier::Normal,
        SquadEncounterTier::Elite,
        SquadEncounterTier::Boss,
    ]
    .into_iter()
    .map(|tier| enemy_squad_template(tier, depth))
    .collect()
}

fn build_node(
    seed: u64,
    id: usize,
    floor: u32,
    lane: u32,
    required_depth: u32,
    kind: SquadNodeKind,
) -> SquadRouteNode {
    let tier = tier_for_kind(kind);
    let difficulty = difficulty_for(kind, required_depth);
    let reward = reward_for(kind, required_depth);
    let event = (kind == SquadNodeKind::Event).then(|| event_descriptor(seed, id, required_depth));
    let rest = (kind == SquadNodeKind::Rest).then(|| rest_descriptor(required_depth));
    let enemy_template = matches!(
        kind,
        SquadNodeKind::Fight | SquadNodeKind::Elite | SquadNodeKind::Boss
    )
    .then(|| enemy_squad_template(tier, required_depth));

    SquadRouteNode {
        id,
        floor,
        lane,
        kind,
        completed: false,
        required_depth,
        tier,
        difficulty,
        reward,
        event,
        rest,
        enemy_template,
    }
}

fn floor_kinds(seed: u64, floor: u32, lanes: u32) -> Vec<SquadNodeKind> {
    let mut rng = SimRng::from_seed(derive_seed(seed, "squad-route-floor", u64::from(floor)));
    let base = match floor {
        0 => vec![
            SquadNodeKind::Fight,
            SquadNodeKind::Recruit,
            SquadNodeKind::Fight,
        ],
        1 => vec![
            SquadNodeKind::Fight,
            SquadNodeKind::Event,
            SquadNodeKind::Rest,
        ],
        2 => vec![
            SquadNodeKind::Elite,
            SquadNodeKind::Fight,
            SquadNodeKind::Event,
        ],
        _ => vec![
            SquadNodeKind::Rest,
            SquadNodeKind::Elite,
            SquadNodeKind::Fight,
        ],
    };
    let mut kinds = (0..lanes)
        .map(|idx| {
            base.get(idx as usize % base.len())
                .copied()
                .unwrap_or(SquadNodeKind::Fight)
        })
        .collect::<Vec<_>>();
    if lanes > 1 {
        let rotation = rng.gen_range(0..lanes) as usize;
        kinds.rotate_left(rotation);
    }
    if floor > 0 && rng.gen_range(0..100) < 25 {
        if let Some(kind) = kinds
            .iter_mut()
            .find(|kind| matches!(kind, SquadNodeKind::Fight | SquadNodeKind::Recruit))
        {
            *kind = SquadNodeKind::Event;
        }
    }
    kinds
}

fn ensure_floor_has_fight(kinds: &mut [SquadNodeKind]) {
    if kinds.iter().any(|kind| {
        matches!(
            kind,
            SquadNodeKind::Fight | SquadNodeKind::Elite | SquadNodeKind::Boss
        )
    }) {
        return;
    }
    if let Some(first) = kinds.first_mut() {
        *first = SquadNodeKind::Fight;
    }
}

fn current_floor_for_depth(route: &[SquadRouteNode], depth: u32) -> Option<u32> {
    let final_floor = final_floor(route)?;
    Some(depth.min(final_floor))
}

fn final_floor(route: &[SquadRouteNode]) -> Option<u32> {
    route.iter().map(|node| node.floor).max()
}

fn tier_for_kind(kind: SquadNodeKind) -> SquadEncounterTier {
    match kind {
        SquadNodeKind::Elite => SquadEncounterTier::Elite,
        SquadNodeKind::Boss => SquadEncounterTier::Boss,
        SquadNodeKind::Fight
        | SquadNodeKind::Recruit
        | SquadNodeKind::Event
        | SquadNodeKind::Rest => SquadEncounterTier::Normal,
    }
}

fn tier_level_bonus(tier: SquadEncounterTier) -> u8 {
    match tier {
        SquadEncounterTier::Normal => 0,
        SquadEncounterTier::Elite => 1,
        SquadEncounterTier::Boss => 2,
    }
}

fn reward_multiplier_percent(tier: SquadEncounterTier) -> u32 {
    match tier {
        SquadEncounterTier::Normal => 100,
        SquadEncounterTier::Elite => 160,
        SquadEncounterTier::Boss => 250,
    }
}

fn difficulty_for(kind: SquadNodeKind, depth: u32) -> u32 {
    let base = 10 + depth.saturating_mul(3);
    match kind {
        SquadNodeKind::Fight => base,
        SquadNodeKind::Recruit => 0,
        SquadNodeKind::Event => 4 + depth.saturating_mul(2),
        SquadNodeKind::Elite => base + 8,
        SquadNodeKind::Boss => base + 18,
        SquadNodeKind::Rest => 0,
    }
}

fn reward_for(kind: SquadNodeKind, depth: u32) -> SquadNodeReward {
    let tier = tier_for_kind(kind);
    let multiplier = reward_multiplier_percent(tier);
    let base_gold = 12 + depth.saturating_mul(3);
    let scaled = |value: u32| value.saturating_mul(multiplier) / 100;
    match kind {
        SquadNodeKind::Fight | SquadNodeKind::Elite | SquadNodeKind::Boss => SquadNodeReward {
            gold_min: scaled(base_gold),
            gold_max: scaled(base_gold + 10),
            xp_per_survivor: scaled(22 + depth.saturating_mul(4)),
            recruit_chance_percent: if kind == SquadNodeKind::Boss { 0 } else { 20 },
            item_chance_percent: match kind {
                SquadNodeKind::Elite => 45,
                SquadNodeKind::Boss => 80,
                _ => 20,
            },
            reward_multiplier_percent: multiplier,
        },
        SquadNodeKind::Recruit => SquadNodeReward {
            recruit_chance_percent: 100,
            reward_multiplier_percent: 100,
            ..SquadNodeReward::default()
        },
        SquadNodeKind::Event => SquadNodeReward {
            gold_min: depth.saturating_mul(2),
            gold_max: 8 + depth.saturating_mul(4),
            xp_per_survivor: 8 + depth.saturating_mul(2),
            recruit_chance_percent: 25,
            item_chance_percent: 15,
            reward_multiplier_percent: 100,
        },
        SquadNodeKind::Rest => SquadNodeReward::default(),
    }
}

fn event_descriptor(seed: u64, node_id: usize, depth: u32) -> SquadEventDescriptor {
    let mut rng = SimRng::from_seed(derive_seed(seed, "squad-route-event", node_id as u64));
    let roll = rng.gen_range(0..5);
    match roll {
        0 => SquadEventDescriptor {
            id: format!("cache-{node_id}"),
            title: "Hidden Pay Chest".to_string(),
            summary: "The route passes an abandoned pay wagon half buried in mud.".to_string(),
            outcome: SquadEventOutcomeDescriptor {
                kind: SquadEventOutcomeKind::Gold,
                label: "Likely gold, slight ambush risk".to_string(),
                gold_delta: 12 + depth as i32 * 4,
                xp_bonus: 0,
                recruit_bonus_percent: 0,
                risk: "low".to_string(),
            },
        },
        1 => SquadEventDescriptor {
            id: format!("deserters-{node_id}"),
            title: "Deserter Camp".to_string(),
            summary: "Campfire smoke reveals soldiers looking for safer employment.".to_string(),
            outcome: SquadEventOutcomeDescriptor {
                kind: SquadEventOutcomeKind::RecruitLead,
                label: "Improves the next recruit offer".to_string(),
                gold_delta: 0,
                xp_bonus: 0,
                recruit_bonus_percent: 35,
                risk: "medium".to_string(),
            },
        },
        2 => SquadEventDescriptor {
            id: format!("high-ground-{node_id}"),
            title: "High Ground".to_string(),
            summary: "A ridge gives the company a clear view of enemy movement.".to_string(),
            outcome: SquadEventOutcomeDescriptor {
                kind: SquadEventOutcomeKind::Scout,
                label: "Scouts the next enemy squad".to_string(),
                gold_delta: 0,
                xp_bonus: 5 + depth,
                recruit_bonus_percent: 0,
                risk: "none".to_string(),
            },
        },
        3 => SquadEventDescriptor {
            id: format!("false-road-{node_id}"),
            title: "False Road".to_string(),
            summary: "Fresh tracks split around a suspiciously quiet ravine.".to_string(),
            outcome: SquadEventOutcomeDescriptor {
                kind: SquadEventOutcomeKind::Ambush,
                label: "Avoid or trigger an ambush fight".to_string(),
                gold_delta: 0,
                xp_bonus: 10 + depth.saturating_mul(2),
                recruit_bonus_percent: 0,
                risk: "high".to_string(),
            },
        },
        _ => SquadEventDescriptor {
            id: format!("drill-master-{node_id}"),
            title: "Drill Master".to_string(),
            summary: "An old mercenary offers one hard lesson before moving on.".to_string(),
            outcome: SquadEventOutcomeDescriptor {
                kind: SquadEventOutcomeKind::Training,
                label: "Small XP gain for survivors".to_string(),
                gold_delta: -(4 + depth as i32),
                xp_bonus: 12 + depth.saturating_mul(3),
                recruit_bonus_percent: 0,
                risk: "low".to_string(),
            },
        },
    }
}

fn rest_descriptor(depth: u32) -> SquadRestDescriptor {
    SquadRestDescriptor {
        label: if depth < 3 {
            "Guarded camp".to_string()
        } else {
            "Fortified camp".to_string()
        },
        heal_percent: (30 + depth.saturating_mul(3)).min(50) as u8,
        clears_downed: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_generation_is_deterministic_for_same_seed() {
        let first = generate_route_map(42, SquadRouteGenerationConfig::default());
        let second = generate_route_map(42, SquadRouteGenerationConfig::default());

        assert_eq!(first, second);
    }

    #[test]
    fn route_generation_uses_seed_for_layout_and_events() {
        let first = generate_route(42);
        let second = generate_route(43);

        assert_ne!(first, second);
    }

    #[test]
    fn route_contains_multiple_floors_and_terminal_boss() {
        let route = generate_route(7);

        assert!(route.iter().any(|node| node.floor > 0));
        assert!(route.iter().any(|node| node.kind == SquadNodeKind::Rest));
        assert!(route.iter().any(|node| node.kind == SquadNodeKind::Event));
        assert!(route.iter().any(|node| node.kind == SquadNodeKind::Elite));

        let boss = route.last().expect("boss node should exist");
        assert_eq!(boss.kind, SquadNodeKind::Boss);
        assert_eq!(boss.tier, SquadEncounterTier::Boss);
        assert!(boss.enemy_template.is_some());
    }

    #[test]
    fn availability_respects_depth_floor_and_completion() {
        let route = generate_route(9);
        let floor_zero = available_node_ids(&route, 0);

        assert!(!floor_zero.is_empty());
        assert!(
            floor_zero
                .iter()
                .all(|id| route.iter().any(|node| node.id == *id && node.floor == 0))
        );

        let completed = with_node_completed(&route, floor_zero[0]);
        let after_completion = available_node_ids(&completed, 0);
        assert!(!after_completion.contains(&floor_zero[0]));

        let floor_one = available_node_ids(&route, 1);
        assert!(
            floor_one
                .iter()
                .all(|id| route.iter().any(|node| node.id == *id && node.floor == 1))
        );
    }

    #[test]
    fn route_status_reports_completion_from_boss_node() {
        let route = generate_route(11);
        let status = route_status(&route, 0);
        assert_eq!(status.kind, SquadRouteStatusKind::InProgress);

        let boss_id = route
            .iter()
            .find(|node| node.kind == SquadNodeKind::Boss)
            .map(|node| node.id)
            .expect("boss node should exist");
        let route = with_node_completed(&route, boss_id);

        assert_eq!(
            route_status(&route, 99).kind,
            SquadRouteStatusKind::Complete
        );
        assert_eq!(
            run_over_reason(&route, 3),
            Some(SquadRunOverReason::RouteComplete)
        );
        assert_eq!(
            run_over_reason(&route, 0),
            Some(SquadRunOverReason::CompanyDefeated)
        );
    }

    #[test]
    fn enemy_templates_scale_by_tier_and_depth() {
        let normal = enemy_squad_template(SquadEncounterTier::Normal, 0);
        let elite = enemy_squad_template(SquadEncounterTier::Elite, 4);
        let boss = enemy_squad_template(SquadEncounterTier::Boss, 4);

        assert!(elite.squad_size >= normal.squad_size);
        assert!(boss.squad_size >= elite.squad_size);
        assert!(elite.base_level > normal.base_level);
        assert!(boss.reward_multiplier_percent > elite.reward_multiplier_percent);
    }
}
