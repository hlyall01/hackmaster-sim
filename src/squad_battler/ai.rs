//! Pure tactical AI helpers for squad combat.
//!
//! This module deliberately uses small input structs instead of depending on
//! the current combat engine types. The combat layer can adapt its unit/grid
//! state into these inputs when tactical AI is wired in.

use serde::{Deserialize, Serialize};

pub const DEFAULT_WOUNDED_HP_RATIO: f32 = 0.35;
pub const DEFAULT_RANGED_MIN_FT: f32 = 30.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AiRole {
    Frontliner,
    Skirmisher,
    Archer,
    Bruiser,
    Wounded,
}

impl AiRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Frontliner => "frontliner",
            Self::Skirmisher => "skirmisher",
            Self::Archer => "archer",
            Self::Bruiser => "bruiser",
            Self::Wounded => "wounded",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RoleAssignmentInput {
    pub hp: i32,
    pub max_hp: i32,
    pub reach_ft: f32,
    pub max_range_ft: Option<f32>,
    pub armor_score: i32,
    pub move_tiles: i32,
}

impl RoleAssignmentInput {
    pub fn hp_ratio(self) -> f32 {
        hp_ratio(self.hp, self.max_hp)
    }

    pub fn has_ranged_weapon(self) -> bool {
        self.max_range_ft.is_some_and(|range| {
            range >= self.reach_ft.max(1.0) * 2.0 && range >= DEFAULT_RANGED_MIN_FT
        })
    }
}

pub fn assign_default_role(input: RoleAssignmentInput) -> AiRole {
    if input.hp_ratio() <= DEFAULT_WOUNDED_HP_RATIO {
        return AiRole::Wounded;
    }
    if input.has_ranged_weapon() {
        return AiRole::Archer;
    }
    if input.armor_score >= 7 && input.max_hp >= 14 {
        return AiRole::Bruiser;
    }
    if input.armor_score >= 4 || input.hp_ratio() >= 0.8 {
        return AiRole::Frontliner;
    }
    AiRole::Skirmisher
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct AiPosition {
    pub x: i32,
    pub y: i32,
}

impl AiPosition {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn manhattan_distance(self, other: Self) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    pub fn distance_ft(self, other: Self, tile_size_ft: f32) -> f32 {
        self.manhattan_distance(other) as f32 * tile_size_ft
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AiGrid {
    pub width: i32,
    pub height: i32,
}

impl AiGrid {
    pub fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }

    pub fn center_y(self) -> i32 {
        self.height.saturating_sub(1) / 2
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AiUnitInput {
    pub id: String,
    pub team_id: u8,
    pub pos: AiPosition,
    pub hp: i32,
    pub max_hp: i32,
    pub reach_ft: f32,
    pub max_range_ft: Option<f32>,
    pub armor_score: i32,
    pub move_tiles: i32,
    pub claimed_target_id: Option<String>,
}

impl AiUnitInput {
    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    pub fn hp_ratio(&self) -> f32 {
        hp_ratio(self.hp, self.max_hp)
    }

    pub fn attack_range_ft(&self) -> f32 {
        self.max_range_ft
            .unwrap_or(self.reach_ft)
            .max(self.reach_ft)
    }

    pub fn role_input(&self) -> RoleAssignmentInput {
        RoleAssignmentInput {
            hp: self.hp,
            max_hp: self.max_hp,
            reach_ft: self.reach_ft,
            max_range_ft: self.max_range_ft,
            armor_score: self.armor_score,
            move_tiles: self.move_tiles,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TieBreakMode {
    StableId,
    Seeded(u64),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetScore {
    pub target_id: String,
    pub total: i32,
    pub distance_tiles: i32,
    pub engaged_by_allies: usize,
    pub low_hp_bonus: i32,
    pub engagement_bonus: i32,
    pub unengaged_bonus: i32,
    pub focus_fire_bonus: i32,
    pub range_penalty: i32,
}

pub fn melee_target_score(
    actor: &AiUnitInput,
    target: &AiUnitInput,
    allies: &[AiUnitInput],
    ally_target_ids: &[String],
    tile_size_ft: f32,
) -> TargetScore {
    let distance_tiles = actor.pos.manhattan_distance(target.pos);
    let engaged_by_allies = engaged_by_allies(target, actor, allies, tile_size_ft);
    let low_hp_bonus = missing_hp_percent(target) / 2;
    let engagement_bonus = engaged_by_allies as i32 * 80;
    let focus_fire_bonus = focus_fire_bonus(target.id.as_str(), ally_target_ids);
    let range_penalty = if distance_tiles as f32 * tile_size_ft > actor.attack_range_ft() {
        distance_tiles * 8
    } else {
        0
    };
    let total = 1000 + low_hp_bonus + engagement_bonus + focus_fire_bonus
        - distance_tiles * 12
        - range_penalty;

    TargetScore {
        target_id: target.id.clone(),
        total,
        distance_tiles,
        engaged_by_allies,
        low_hp_bonus,
        engagement_bonus,
        unengaged_bonus: 0,
        focus_fire_bonus,
        range_penalty,
    }
}

pub fn ranged_target_score(
    actor: &AiUnitInput,
    target: &AiUnitInput,
    allies: &[AiUnitInput],
    ally_target_ids: &[String],
    tile_size_ft: f32,
) -> TargetScore {
    let distance_tiles = actor.pos.manhattan_distance(target.pos);
    let engaged_by_allies = engaged_by_allies(target, actor, allies, tile_size_ft);
    let low_hp_bonus = missing_hp_percent(target) * 2;
    let unengaged_bonus = if engaged_by_allies == 0 { 90 } else { -30 };
    let focus_fire_bonus = focus_fire_bonus(target.id.as_str(), ally_target_ids) / 2;
    let range_penalty = actor
        .max_range_ft
        .filter(|range| distance_tiles as f32 * tile_size_ft > *range)
        .map(|_| 500)
        .unwrap_or(0);
    let total = 1000 + low_hp_bonus + unengaged_bonus + focus_fire_bonus
        - distance_tiles * 5
        - range_penalty;

    TargetScore {
        target_id: target.id.clone(),
        total,
        distance_tiles,
        engaged_by_allies,
        low_hp_bonus,
        engagement_bonus: 0,
        unengaged_bonus,
        focus_fire_bonus,
        range_penalty,
    }
}

pub fn choose_target(
    actor: &AiUnitInput,
    candidates: &[AiUnitInput],
    allies: &[AiUnitInput],
    ally_target_ids: &[String],
    role: AiRole,
    tie_break: TieBreakMode,
    tile_size_ft: f32,
) -> Option<TargetScore> {
    candidates
        .iter()
        .filter(|candidate| candidate.is_alive() && candidate.team_id != actor.team_id)
        .map(|candidate| match role {
            AiRole::Archer | AiRole::Skirmisher | AiRole::Wounded
                if actor.max_range_ft.is_some() =>
            {
                ranged_target_score(actor, candidate, allies, ally_target_ids, tile_size_ft)
            }
            _ => melee_target_score(actor, candidate, allies, ally_target_ids, tile_size_ft),
        })
        .reduce(|best, score| better_score(best, score, tie_break))
}

pub fn focus_fire_count(target_id: &str, ally_target_ids: &[String]) -> usize {
    ally_target_ids
        .iter()
        .filter(|claimed| claimed.as_str() == target_id)
        .count()
}

pub fn focus_fire_bonus(target_id: &str, ally_target_ids: &[String]) -> i32 {
    focus_fire_bonus_from_count(focus_fire_count(target_id, ally_target_ids))
}

pub fn focus_fire_bonus_from_count(count: usize) -> i32 {
    (count as i32 * 45).min(180)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AiIntentKind {
    AttackMelee,
    AttackRanged,
    Advance,
    Hold,
    Kite,
    Backline,
    BreakClump,
    Sidestep,
    HoldGuard,
}

impl AiIntentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AttackMelee => "attack_melee",
            Self::AttackRanged => "attack_ranged",
            Self::Advance => "advance",
            Self::Hold => "hold",
            Self::Kite => "kite",
            Self::Backline => "backline",
            Self::BreakClump => "break_clump",
            Self::Sidestep => "sidestep",
            Self::HoldGuard => "hold_guard",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::AttackMelee => "Attack in melee",
            Self::AttackRanged => "Take ranged shot",
            Self::Advance => "Advance",
            Self::Hold => "Hold",
            Self::Kite => "Kite",
            Self::Backline => "Fall back",
            Self::BreakClump => "Spread out",
            Self::Sidestep => "Sidestep",
            Self::HoldGuard => "Hold guard",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct AiIntent {
    pub kind: AiIntentKind,
    pub target_id: Option<String>,
    pub label: String,
}

impl AiIntent {
    pub fn new(kind: AiIntentKind, target_id: Option<String>) -> Self {
        Self {
            kind,
            target_id,
            label: kind.label().to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WoundedPreference {
    Kite,
    Backline,
    HoldBackline,
}

pub fn wounded_preference(
    actor: &AiUnitInput,
    enemies: &[AiUnitInput],
    grid: AiGrid,
    tile_size_ft: f32,
) -> WoundedPreference {
    if nearest_threat_distance_ft(actor, enemies, tile_size_ft)
        .is_some_and(|distance| distance <= actor.attack_range_ft())
    {
        return WoundedPreference::Kite;
    }

    let anchor = backline_anchor(actor.team_id, grid);
    if actor.pos.manhattan_distance(anchor) > actor.move_tiles.max(1) {
        WoundedPreference::Backline
    } else {
        WoundedPreference::HoldBackline
    }
}

pub fn choose_movement_intent(
    actor: &AiUnitInput,
    target: Option<&AiUnitInput>,
    allies: &[AiUnitInput],
    enemies: &[AiUnitInput],
    role: AiRole,
    path_blocked: bool,
    grid: AiGrid,
    tile_size_ft: f32,
) -> AiIntent {
    if role == AiRole::Wounded {
        return match wounded_preference(actor, enemies, grid, tile_size_ft) {
            WoundedPreference::Kite => {
                AiIntent::new(AiIntentKind::Kite, target.map(|target| target.id.clone()))
            }
            WoundedPreference::Backline => AiIntent::new(AiIntentKind::Backline, None),
            WoundedPreference::HoldBackline => AiIntent::new(AiIntentKind::Hold, None),
        };
    }

    if path_blocked {
        return AiIntent::new(
            classify_blocked_fallback(actor, allies, enemies, tile_size_ft),
            None,
        );
    }

    let Some(target) = target else {
        return AiIntent::new(AiIntentKind::Hold, None);
    };

    let distance_ft = actor.pos.distance_ft(target.pos, tile_size_ft);
    if distance_ft <= actor.reach_ft as f32 {
        return AiIntent::new(AiIntentKind::AttackMelee, Some(target.id.clone()));
    }
    if actor.max_range_ft.is_some_and(|range| distance_ft <= range) {
        return AiIntent::new(AiIntentKind::AttackRanged, Some(target.id.clone()));
    }
    AiIntent::new(AiIntentKind::Advance, Some(target.id.clone()))
}

pub fn classify_blocked_fallback(
    actor: &AiUnitInput,
    allies: &[AiUnitInput],
    enemies: &[AiUnitInput],
    tile_size_ft: f32,
) -> AiIntentKind {
    if adjacent_allies(actor, allies) >= 2 {
        return AiIntentKind::BreakClump;
    }
    if nearest_threat_distance_ft(actor, enemies, tile_size_ft)
        .is_some_and(|distance| distance <= actor.reach_ft + tile_size_ft)
    {
        return AiIntentKind::HoldGuard;
    }
    if actor.max_range_ft.is_some() {
        return AiIntentKind::Sidestep;
    }
    AiIntentKind::BreakClump
}

pub fn backline_anchor(team_id: u8, grid: AiGrid) -> AiPosition {
    let x = if team_id == 0 {
        0
    } else {
        grid.width.saturating_sub(1)
    };
    AiPosition::new(x, grid.center_y())
}

fn better_score(left: TargetScore, right: TargetScore, tie_break: TieBreakMode) -> TargetScore {
    if right.total > left.total {
        return right;
    }
    if right.total < left.total {
        return left;
    }

    match tie_break {
        TieBreakMode::StableId => {
            if right.target_id < left.target_id {
                right
            } else {
                left
            }
        }
        TieBreakMode::Seeded(seed) => {
            let right_key = seeded_tie_key(seed, right.target_id.as_str());
            let left_key = seeded_tie_key(seed, left.target_id.as_str());
            if (right_key, right.target_id.as_str()) < (left_key, left.target_id.as_str()) {
                right
            } else {
                left
            }
        }
    }
}

fn engaged_by_allies(
    target: &AiUnitInput,
    actor: &AiUnitInput,
    allies: &[AiUnitInput],
    tile_size_ft: f32,
) -> usize {
    allies
        .iter()
        .filter(|ally| {
            ally.is_alive()
                && ally.team_id == actor.team_id
                && ally.id != actor.id
                && ally.pos.distance_ft(target.pos, tile_size_ft) <= ally.reach_ft
        })
        .count()
}

fn adjacent_allies(actor: &AiUnitInput, allies: &[AiUnitInput]) -> usize {
    allies
        .iter()
        .filter(|ally| {
            ally.is_alive()
                && ally.team_id == actor.team_id
                && ally.id != actor.id
                && actor.pos.manhattan_distance(ally.pos) <= 1
        })
        .count()
}

fn nearest_threat_distance_ft(
    actor: &AiUnitInput,
    enemies: &[AiUnitInput],
    tile_size_ft: f32,
) -> Option<f32> {
    enemies
        .iter()
        .filter(|enemy| enemy.is_alive() && enemy.team_id != actor.team_id)
        .map(|enemy| actor.pos.distance_ft(enemy.pos, tile_size_ft))
        .min_by(|left, right| left.total_cmp(right))
}

fn hp_ratio(hp: i32, max_hp: i32) -> f32 {
    if max_hp <= 0 {
        return 0.0;
    }
    (hp.max(0) as f32 / max_hp as f32).clamp(0.0, 1.0)
}

fn missing_hp_percent(unit: &AiUnitInput) -> i32 {
    ((1.0 - unit.hp_ratio()) * 100.0).round() as i32
}

fn seeded_tie_key(seed: u64, id: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64 ^ seed;
    for byte in id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    const TILE: f32 = 5.0;

    fn unit(id: &str, team_id: u8, x: i32, y: i32) -> AiUnitInput {
        AiUnitInput {
            id: id.to_string(),
            team_id,
            pos: AiPosition::new(x, y),
            hp: 12,
            max_hp: 12,
            reach_ft: 5.0,
            max_range_ft: None,
            armor_score: 4,
            move_tiles: 4,
            claimed_target_id: None,
        }
    }

    #[test]
    fn role_assignment_uses_range_armor_and_hp_state() {
        assert_eq!(
            assign_default_role(RoleAssignmentInput {
                hp: 3,
                max_hp: 12,
                reach_ft: 5.0,
                max_range_ft: Some(60.0),
                armor_score: 8,
                move_tiles: 4,
            }),
            AiRole::Wounded
        );
        assert_eq!(
            assign_default_role(RoleAssignmentInput {
                hp: 10,
                max_hp: 12,
                reach_ft: 5.0,
                max_range_ft: Some(60.0),
                armor_score: 1,
                move_tiles: 5,
            }),
            AiRole::Archer
        );
        assert_eq!(
            assign_default_role(RoleAssignmentInput {
                hp: 18,
                max_hp: 18,
                reach_ft: 5.0,
                max_range_ft: None,
                armor_score: 8,
                move_tiles: 3,
            }),
            AiRole::Bruiser
        );
        assert_eq!(
            assign_default_role(RoleAssignmentInput {
                hp: 8,
                max_hp: 12,
                reach_ft: 5.0,
                max_range_ft: None,
                armor_score: 1,
                move_tiles: 5,
            }),
            AiRole::Skirmisher
        );
    }

    #[test]
    fn melee_targeting_prefers_enemy_engaged_by_ally() {
        let actor = unit("hero-a", 0, 1, 1);
        let ally = unit("hero-b", 0, 4, 1);
        let near_unengaged = unit("enemy-a", 1, 2, 1);
        let engaged = unit("enemy-b", 1, 5, 1);

        let choice = choose_target(
            &actor,
            &[near_unengaged, engaged],
            &[actor.clone(), ally],
            &[],
            AiRole::Frontliner,
            TieBreakMode::StableId,
            TILE,
        )
        .expect("target");

        assert_eq!(choice.target_id, "enemy-b");
        assert_eq!(choice.engaged_by_allies, 1);
    }

    #[test]
    fn ranged_targeting_prefers_low_hp_unengaged_enemy() {
        let mut actor = unit("archer", 0, 0, 0);
        actor.max_range_ft = Some(60.0);
        let ally = unit("front", 0, 3, 0);
        let mut engaged_full_hp = unit("enemy-a", 1, 4, 0);
        engaged_full_hp.hp = 12;
        let mut unengaged_low_hp = unit("enemy-b", 1, 7, 0);
        unengaged_low_hp.hp = 3;

        let choice = choose_target(
            &actor,
            &[engaged_full_hp, unengaged_low_hp],
            &[actor.clone(), ally],
            &[],
            AiRole::Archer,
            TieBreakMode::StableId,
            TILE,
        )
        .expect("target");

        assert_eq!(choice.target_id, "enemy-b");
        assert!(choice.low_hp_bonus > 0);
        assert!(choice.unengaged_bonus > 0);
    }

    #[test]
    fn stable_tie_breaking_uses_target_id_order() {
        let actor = unit("hero", 0, 0, 0);
        let zed = unit("zed", 1, 3, 0);
        let alpha = unit("alpha", 1, 3, 0);

        let choice = choose_target(
            &actor,
            &[zed, alpha],
            &[actor.clone()],
            &[],
            AiRole::Frontliner,
            TieBreakMode::StableId,
            TILE,
        )
        .expect("target");

        assert_eq!(choice.target_id, "alpha");
    }

    #[test]
    fn focus_fire_bonus_scales_with_existing_claims() {
        let claims = vec![
            "enemy-a".to_string(),
            "enemy-b".to_string(),
            "enemy-a".to_string(),
        ];

        assert_eq!(focus_fire_count("enemy-a", &claims), 2);
        assert_eq!(focus_fire_bonus("enemy-a", &claims), 90);
    }

    #[test]
    fn movement_intent_attacks_advances_or_kites() {
        let mut archer = unit("archer", 0, 2, 2);
        archer.max_range_ft = Some(40.0);
        let in_range = unit("enemy", 1, 6, 2);
        let far = unit("far", 1, 20, 2);
        let wounded = AiUnitInput {
            hp: 3,
            max_hp: 12,
            ..archer.clone()
        };

        assert_eq!(
            choose_movement_intent(
                &archer,
                Some(&in_range),
                &[archer.clone()],
                std::slice::from_ref(&in_range),
                AiRole::Archer,
                false,
                AiGrid::new(12, 8),
                TILE,
            )
            .kind,
            AiIntentKind::AttackRanged
        );
        assert_eq!(
            choose_movement_intent(
                &archer,
                Some(&far),
                &[archer.clone()],
                std::slice::from_ref(&far),
                AiRole::Archer,
                false,
                AiGrid::new(12, 8),
                TILE,
            )
            .kind,
            AiIntentKind::Advance
        );
        assert_eq!(
            choose_movement_intent(
                &wounded,
                Some(&in_range),
                &[wounded.clone()],
                std::slice::from_ref(&in_range),
                AiRole::Wounded,
                false,
                AiGrid::new(12, 8),
                TILE,
            )
            .kind,
            AiIntentKind::Kite
        );
    }

    #[test]
    fn blocked_fallback_breaks_clumps_before_idling() {
        let actor = unit("hero", 0, 4, 4);
        let ally_a = unit("ally-a", 0, 4, 3);
        let ally_b = unit("ally-b", 0, 5, 4);
        let enemy = unit("enemy", 1, 9, 4);

        let intent = choose_movement_intent(
            &actor,
            Some(&enemy),
            &[actor.clone(), ally_a, ally_b],
            std::slice::from_ref(&enemy),
            AiRole::Frontliner,
            true,
            AiGrid::new(12, 8),
            TILE,
        );

        assert_eq!(intent.kind, AiIntentKind::BreakClump);
        assert_eq!(intent.label, "Spread out");
    }
}
