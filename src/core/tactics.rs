//! Deterministic tactical-directive rules shared by the simulator and UI.

use serde::{Deserialize, Serialize};

pub const MAX_TACTICAL_CONDITIONS: usize = 2;
pub const SHIELD_OF_BLADES_STYLE_ID: &str = "shield_of_blades";
pub const STORM_OF_BLADES_STYLE_ID: &str = "storm_of_blades";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TacticalDecisionPoint {
    #[default]
    NextAttackOpportunity,
    IncomingAttackReaction,
}

impl TacticalDecisionPoint {
    pub fn label(self) -> &'static str {
        match self {
            Self::NextAttackOpportunity => "Next attack",
            Self::IncomingAttackReaction => "Incoming attack",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TacticalChannel {
    WeaponStyle,
    AttackMode,
    Stance,
    Reaction,
}

impl TacticalChannel {
    pub const ALL: [Self; 4] = [
        Self::WeaponStyle,
        Self::AttackMode,
        Self::Stance,
        Self::Reaction,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::WeaponStyle => "Style",
            Self::AttackMode => "Attack",
            Self::Stance => "Stance",
            Self::Reaction => "Reaction",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NumericComparison {
    Less,
    LessOrEqual,
    Equal,
    GreaterOrEqual,
    Greater,
}

impl NumericComparison {
    pub const ALL: [Self; 5] = [
        Self::Less,
        Self::LessOrEqual,
        Self::Equal,
        Self::GreaterOrEqual,
        Self::Greater,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Less => "<",
            Self::LessOrEqual => "<=",
            Self::Equal => "=",
            Self::GreaterOrEqual => ">=",
            Self::Greater => ">",
        }
    }

    pub fn matches(self, actual: f32, expected: f32) -> bool {
        match self {
            Self::Less => actual < expected,
            Self::LessOrEqual => actual <= expected,
            Self::Equal => (actual - expected).abs() < 0.001,
            Self::GreaterOrEqual => actual >= expected,
            Self::Greater => actual > expected,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RelativeComparison {
    Less,
    Equal,
    Greater,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeedComparison {
    Faster,
    Equal,
    Slower,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TacticalCondition {
    #[default]
    Always,
    MyHpPercent {
        comparison: NumericComparison,
        value: f32,
    },
    EnemyHpPercent {
        comparison: NumericComparison,
        value: f32,
    },
    DistanceFt {
        comparison: NumericComparison,
        value: f32,
    },
    ReachComparedToEnemy {
        comparison: RelativeComparison,
    },
    RetreatSpaceAvailable {
        value: bool,
    },
    MyWeaponCanJab {
        value: bool,
    },
    MyHasActiveShield {
        value: bool,
    },
    EnemyWeaponGroup {
        value: String,
        #[serde(default)]
        negated: bool,
    },
    EnemyHasActiveShield {
        value: bool,
    },
    EnemyArmorType {
        value: String,
        #[serde(default)]
        negated: bool,
    },
    EnemyCharging {
        value: bool,
    },
    MyHasAttacked {
        value: bool,
    },
    EnemyTimeToReachSeconds {
        comparison: NumericComparison,
        value: f32,
    },
    MyActiveStyle {
        style_id: String,
        #[serde(default)]
        negated: bool,
    },
    EnemyActiveStyle {
        style_id: String,
        #[serde(default)]
        negated: bool,
    },
    EnemyDr {
        comparison: NumericComparison,
        value: f32,
    },
    EnemyAttackSpeedSeconds {
        comparison: NumericComparison,
        value: f32,
    },
    EnemyAttackSpeedComparedToMine {
        comparison: SpeedComparison,
    },
}

impl TacticalCondition {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Always => "Always",
            Self::MyHpPercent { .. } => "My HP %",
            Self::EnemyHpPercent { .. } => "Enemy HP %",
            Self::DistanceFt { .. } => "Distance (ft)",
            Self::ReachComparedToEnemy { .. } => "My reach",
            Self::RetreatSpaceAvailable { .. } => "Retreat space",
            Self::MyWeaponCanJab { .. } => "Weapon can Jab",
            Self::MyHasActiveShield { .. } => "My shield active",
            Self::EnemyWeaponGroup { .. } => "Enemy weapon group",
            Self::EnemyHasActiveShield { .. } => "Enemy shield active",
            Self::EnemyArmorType { .. } => "Enemy armor type",
            Self::EnemyCharging { .. } => "Enemy charging",
            Self::MyHasAttacked { .. } => "I have attacked this combat",
            Self::EnemyTimeToReachSeconds { .. } => "Enemy time to reach",
            Self::MyActiveStyle { .. } => "My active style",
            Self::EnemyActiveStyle { .. } => "Enemy active style",
            Self::EnemyDr { .. } => "Enemy DR",
            Self::EnemyAttackSpeedSeconds { .. } => "Enemy attack speed",
            Self::EnemyAttackSpeedComparedToMine { .. } => "Enemy speed vs mine",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TacticalAction {
    RetainWeaponStyle,
    NeutralWeaponStyle,
    UseWeaponStyle {
        style_ids: Vec<String>,
    },
    #[default]
    NormalAttack,
    Jab,
    NeutralStance,
    FightDefensively {
        penalty: i32,
    },
    StandGround,
    GiveGround,
}

impl TacticalAction {
    pub fn channel(&self) -> TacticalChannel {
        match self {
            Self::RetainWeaponStyle | Self::NeutralWeaponStyle | Self::UseWeaponStyle { .. } => {
                TacticalChannel::WeaponStyle
            }
            Self::NormalAttack | Self::Jab => TacticalChannel::AttackMode,
            Self::NeutralStance | Self::FightDefensively { .. } => TacticalChannel::Stance,
            Self::StandGround | Self::GiveGround => TacticalChannel::Reaction,
        }
    }

    pub fn decision_point(&self) -> TacticalDecisionPoint {
        match self.channel() {
            TacticalChannel::Reaction => TacticalDecisionPoint::IncomingAttackReaction,
            _ => TacticalDecisionPoint::NextAttackOpportunity,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::RetainWeaponStyle => "Retain current style".to_string(),
            Self::NeutralWeaponStyle => "Use no weapon style".to_string(),
            Self::UseWeaponStyle { style_ids } => {
                format!("Use style: {}", style_ids.join(" + "))
            }
            Self::NormalAttack => "Normal attack".to_string(),
            Self::Jab => "Jab".to_string(),
            Self::NeutralStance => "Neutral stance".to_string(),
            Self::FightDefensively { penalty } => {
                format!("Fight defensively (-{penalty}/+{})", penalty / 2)
            }
            Self::StandGround => "Stand ground".to_string(),
            Self::GiveGround => "Give Ground".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct TacticalRule {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub decision: TacticalDecisionPoint,
    #[serde(default)]
    pub conditions: Vec<TacticalCondition>,
    pub action: TacticalAction,
}

impl TacticalRule {
    pub fn new(action: TacticalAction, conditions: Vec<TacticalCondition>) -> Self {
        Self {
            enabled: true,
            decision: action.decision_point(),
            conditions,
            action,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct TacticalPolicy {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub rules: Vec<TacticalRule>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct TacticalPreset {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening_style_ids: Option<Vec<String>>,
    #[serde(default)]
    pub rules: Vec<TacticalRule>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TacticalContext {
    pub my_hp_percent: f32,
    pub enemy_hp_percent: f32,
    pub distance_ft: f32,
    pub my_reach_ft: f32,
    pub enemy_reach_ft: f32,
    pub retreat_space_available: bool,
    pub my_weapon_can_jab: bool,
    pub my_has_active_shield: bool,
    pub enemy_weapon_group: String,
    pub enemy_has_active_shield: bool,
    pub enemy_armor_type: String,
    pub enemy_charging: bool,
    pub my_has_attacked: bool,
    pub enemy_time_to_reach_seconds: f32,
    pub my_active_style_ids: Vec<String>,
    pub enemy_active_style_ids: Vec<String>,
    pub available_style_ids: Vec<String>,
    pub style_pair_allowed: bool,
    pub enemy_dr: f32,
    pub my_attack_speed_seconds: f32,
    pub enemy_attack_speed_seconds: f32,
    pub give_ground_legal: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TacticalDecision {
    pub action: TacticalAction,
    pub matched_rule_index: Option<usize>,
}

pub fn validate_policy(policy: &TacticalPolicy) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for (idx, rule) in policy.rules.iter().enumerate() {
        if rule.conditions.len() > MAX_TACTICAL_CONDITIONS {
            errors.push(format!(
                "Rule {} has {} conditions; at most {} are allowed.",
                idx + 1,
                rule.conditions.len(),
                MAX_TACTICAL_CONDITIONS
            ));
        }
        if rule.decision != rule.action.decision_point() {
            errors.push(format!(
                "Rule {} uses action '{}' at the wrong decision point.",
                idx + 1,
                rule.action.label()
            ));
        }
        if let TacticalAction::FightDefensively { penalty } = rule.action
            && ![2, 4, 6, 8].contains(&penalty)
        {
            errors.push(format!(
                "Rule {} has invalid Fight Defensively penalty {}.",
                idx + 1,
                penalty
            ));
        }
        if let TacticalAction::UseWeaponStyle { ref style_ids } = rule.action
            && !valid_style_selection_shape(style_ids)
        {
            errors.push(format!(
                "Rule {} selects an invalid weapon-style combination.",
                idx + 1
            ));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn evaluate_channel(
    policy: &TacticalPolicy,
    decision: TacticalDecisionPoint,
    channel: TacticalChannel,
    context: &TacticalContext,
) -> TacticalDecision {
    if policy.enabled {
        for (idx, rule) in policy.rules.iter().enumerate() {
            if !rule.enabled
                || rule.decision != decision
                || rule.action.channel() != channel
                || rule.conditions.len() > MAX_TACTICAL_CONDITIONS
                || !rule
                    .conditions
                    .iter()
                    .all(|condition| condition_matches(condition, context))
                || !action_is_legal(&rule.action, context)
            {
                continue;
            }
            return TacticalDecision {
                action: rule.action.clone(),
                matched_rule_index: Some(idx),
            };
        }
    }
    TacticalDecision {
        action: fallback_action(channel),
        matched_rule_index: None,
    }
}

pub fn action_warning(action: &TacticalAction, context: &TacticalContext) -> Option<String> {
    if action_is_legal(action, context) {
        return None;
    }
    Some(match action {
        TacticalAction::Jab => "Current weapon cannot Jab.".to_string(),
        TacticalAction::GiveGround => "Give Ground is not currently legal.".to_string(),
        TacticalAction::UseWeaponStyle { style_ids } => format!(
            "Style selection '{}' is not currently available.",
            style_ids.join(" + ")
        ),
        _ => "Action is not currently available.".to_string(),
    })
}

pub fn valid_style_selection_shape(style_ids: &[String]) -> bool {
    match style_ids {
        [single] => !single.trim().is_empty(),
        [left, right] => is_shield_storm_pair(left, right),
        _ => false,
    }
}

pub fn is_shield_storm_pair(left: &str, right: &str) -> bool {
    matches!(
        (left, right),
        (SHIELD_OF_BLADES_STYLE_ID, STORM_OF_BLADES_STYLE_ID)
            | (STORM_OF_BLADES_STYLE_ID, SHIELD_OF_BLADES_STYLE_ID)
    )
}

pub fn canonicalize_style_selection(mut style_ids: Vec<String>) -> Vec<String> {
    if style_ids.len() == 2
        && style_ids[0] == STORM_OF_BLADES_STYLE_ID
        && style_ids[1] == SHIELD_OF_BLADES_STYLE_ID
    {
        style_ids.swap(0, 1);
    }
    style_ids
}

fn fallback_action(channel: TacticalChannel) -> TacticalAction {
    match channel {
        TacticalChannel::WeaponStyle => TacticalAction::RetainWeaponStyle,
        TacticalChannel::AttackMode => TacticalAction::NormalAttack,
        TacticalChannel::Stance => TacticalAction::NeutralStance,
        TacticalChannel::Reaction => TacticalAction::StandGround,
    }
}

fn condition_matches(condition: &TacticalCondition, context: &TacticalContext) -> bool {
    match condition {
        TacticalCondition::Always => true,
        TacticalCondition::MyHpPercent { comparison, value } => {
            comparison.matches(context.my_hp_percent, *value)
        }
        TacticalCondition::EnemyHpPercent { comparison, value } => {
            comparison.matches(context.enemy_hp_percent, *value)
        }
        TacticalCondition::DistanceFt { comparison, value } => {
            comparison.matches(context.distance_ft, *value)
        }
        TacticalCondition::ReachComparedToEnemy { comparison } => {
            relative_matches(context.my_reach_ft, context.enemy_reach_ft, *comparison)
        }
        TacticalCondition::RetreatSpaceAvailable { value } => {
            context.retreat_space_available == *value
        }
        TacticalCondition::MyWeaponCanJab { value } => context.my_weapon_can_jab == *value,
        TacticalCondition::MyHasActiveShield { value } => context.my_has_active_shield == *value,
        TacticalCondition::EnemyWeaponGroup { value, negated } => {
            equals_with_negation(&context.enemy_weapon_group, value, *negated)
        }
        TacticalCondition::EnemyHasActiveShield { value } => {
            context.enemy_has_active_shield == *value
        }
        TacticalCondition::EnemyArmorType { value, negated } => {
            equals_with_negation(&context.enemy_armor_type, value, *negated)
        }
        TacticalCondition::EnemyCharging { value } => context.enemy_charging == *value,
        TacticalCondition::MyHasAttacked { value } => context.my_has_attacked == *value,
        TacticalCondition::EnemyTimeToReachSeconds { comparison, value } => {
            comparison.matches(context.enemy_time_to_reach_seconds, *value)
        }
        TacticalCondition::MyActiveStyle { style_id, negated } => {
            contains_with_negation(&context.my_active_style_ids, style_id, *negated)
        }
        TacticalCondition::EnemyActiveStyle { style_id, negated } => {
            contains_with_negation(&context.enemy_active_style_ids, style_id, *negated)
        }
        TacticalCondition::EnemyDr { comparison, value } => {
            comparison.matches(context.enemy_dr, *value)
        }
        TacticalCondition::EnemyAttackSpeedSeconds { comparison, value } => {
            comparison.matches(context.enemy_attack_speed_seconds, *value)
        }
        TacticalCondition::EnemyAttackSpeedComparedToMine { comparison } => speed_matches(
            context.enemy_attack_speed_seconds,
            context.my_attack_speed_seconds,
            *comparison,
        ),
    }
}

fn action_is_legal(action: &TacticalAction, context: &TacticalContext) -> bool {
    match action {
        TacticalAction::Jab => context.my_weapon_can_jab,
        TacticalAction::GiveGround => context.give_ground_legal,
        TacticalAction::UseWeaponStyle { style_ids } => {
            valid_style_selection_shape(style_ids)
                && style_ids.iter().all(|style_id| {
                    context
                        .available_style_ids
                        .iter()
                        .any(|available| available.eq_ignore_ascii_case(style_id))
                })
                && (style_ids.len() == 1
                    || (context.style_pair_allowed
                        && is_shield_storm_pair(&style_ids[0], &style_ids[1])))
        }
        _ => true,
    }
}

fn relative_matches(actual: f32, other: f32, comparison: RelativeComparison) -> bool {
    match comparison {
        RelativeComparison::Less => actual < other,
        RelativeComparison::Equal => (actual - other).abs() < 0.001,
        RelativeComparison::Greater => actual > other,
    }
}

fn speed_matches(enemy: f32, mine: f32, comparison: SpeedComparison) -> bool {
    match comparison {
        SpeedComparison::Faster => enemy < mine,
        SpeedComparison::Equal => (enemy - mine).abs() < 0.001,
        SpeedComparison::Slower => enemy > mine,
    }
}

fn equals_with_negation(actual: &str, expected: &str, negated: bool) -> bool {
    let matches = actual.eq_ignore_ascii_case(expected);
    if negated { !matches } else { matches }
}

fn contains_with_negation(values: &[String], expected: &str, negated: bool) -> bool {
    let matches = values
        .iter()
        .any(|value| value.eq_ignore_ascii_case(expected));
    if negated { !matches } else { matches }
}

const fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> TacticalContext {
        TacticalContext {
            my_hp_percent: 35.0,
            enemy_hp_percent: 75.0,
            distance_ft: 10.0,
            my_reach_ft: 10.0,
            enemy_reach_ft: 5.0,
            retreat_space_available: true,
            my_weapon_can_jab: true,
            my_has_active_shield: true,
            enemy_weapon_group: "Blunt".to_string(),
            enemy_has_active_shield: true,
            enemy_armor_type: "Heavy".to_string(),
            enemy_charging: false,
            my_has_attacked: true,
            enemy_time_to_reach_seconds: 3.0,
            my_active_style_ids: vec!["falling_sun".to_string()],
            enemy_active_style_ids: vec!["hammerer".to_string()],
            available_style_ids: vec![
                "falling_sun".to_string(),
                SHIELD_OF_BLADES_STYLE_ID.to_string(),
                STORM_OF_BLADES_STYLE_ID.to_string(),
            ],
            style_pair_allowed: true,
            enemy_dr: 6.0,
            my_attack_speed_seconds: 8.0,
            enemy_attack_speed_seconds: 11.0,
            give_ground_legal: true,
        }
    }

    #[test]
    fn first_matching_legal_rule_wins_within_channel() {
        let policy = TacticalPolicy {
            enabled: true,
            rules: vec![
                TacticalRule::new(
                    TacticalAction::Jab,
                    vec![TacticalCondition::EnemyDr {
                        comparison: NumericComparison::GreaterOrEqual,
                        value: 5.0,
                    }],
                ),
                TacticalRule::new(
                    TacticalAction::NormalAttack,
                    vec![TacticalCondition::Always],
                ),
            ],
        };
        let decision = evaluate_channel(
            &policy,
            TacticalDecisionPoint::NextAttackOpportunity,
            TacticalChannel::AttackMode,
            &context(),
        );
        assert_eq!(decision.action, TacticalAction::Jab);
        assert_eq!(decision.matched_rule_index, Some(0));
    }

    #[test]
    fn illegal_action_falls_through() {
        let mut ctx = context();
        ctx.my_weapon_can_jab = false;
        let policy = TacticalPolicy {
            enabled: true,
            rules: vec![
                TacticalRule::new(TacticalAction::Jab, vec![TacticalCondition::Always]),
                TacticalRule::new(
                    TacticalAction::NormalAttack,
                    vec![TacticalCondition::Always],
                ),
            ],
        };
        let decision = evaluate_channel(
            &policy,
            TacticalDecisionPoint::NextAttackOpportunity,
            TacticalChannel::AttackMode,
            &ctx,
        );
        assert_eq!(decision.action, TacticalAction::NormalAttack);
        assert_eq!(decision.matched_rule_index, Some(1));
    }

    #[test]
    fn disabled_policy_uses_locked_fallbacks() {
        let policy = TacticalPolicy::default();
        assert_eq!(
            evaluate_channel(
                &policy,
                TacticalDecisionPoint::NextAttackOpportunity,
                TacticalChannel::WeaponStyle,
                &context()
            )
            .action,
            TacticalAction::RetainWeaponStyle
        );
        assert_eq!(
            evaluate_channel(
                &policy,
                TacticalDecisionPoint::IncomingAttackReaction,
                TacticalChannel::Reaction,
                &context()
            )
            .action,
            TacticalAction::StandGround
        );
    }

    #[test]
    fn combat_fundamental_conditions_use_current_values() {
        let ctx = context();
        let conditions = [
            TacticalCondition::MyHpPercent {
                comparison: NumericComparison::LessOrEqual,
                value: 35.0,
            },
            TacticalCondition::EnemyHpPercent {
                comparison: NumericComparison::Greater,
                value: 70.0,
            },
            TacticalCondition::DistanceFt {
                comparison: NumericComparison::Equal,
                value: 10.0,
            },
            TacticalCondition::RetreatSpaceAvailable { value: true },
            TacticalCondition::MyWeaponCanJab { value: true },
            TacticalCondition::MyHasActiveShield { value: true },
            TacticalCondition::EnemyWeaponGroup {
                value: "blunt".to_string(),
                negated: false,
            },
            TacticalCondition::EnemyHasActiveShield { value: true },
            TacticalCondition::EnemyArmorType {
                value: "heavy".to_string(),
                negated: false,
            },
            TacticalCondition::EnemyCharging { value: false },
            TacticalCondition::MyHasAttacked { value: true },
            TacticalCondition::EnemyTimeToReachSeconds {
                comparison: NumericComparison::GreaterOrEqual,
                value: 3.0,
            },
            TacticalCondition::MyActiveStyle {
                style_id: "falling_sun".to_string(),
                negated: false,
            },
            TacticalCondition::EnemyActiveStyle {
                style_id: "hammerer".to_string(),
                negated: false,
            },
            TacticalCondition::EnemyAttackSpeedSeconds {
                comparison: NumericComparison::GreaterOrEqual,
                value: 11.0,
            },
        ];
        for condition in conditions {
            assert!(
                condition_matches(&condition, &ctx),
                "condition should match: {condition:?}"
            );
        }
        assert!(condition_matches(
            &TacticalCondition::EnemyDr {
                comparison: NumericComparison::GreaterOrEqual,
                value: 6.0
            },
            &ctx
        ));
        assert!(condition_matches(
            &TacticalCondition::EnemyAttackSpeedComparedToMine {
                comparison: SpeedComparison::Slower
            },
            &ctx
        ));
        assert!(condition_matches(
            &TacticalCondition::ReachComparedToEnemy {
                comparison: RelativeComparison::Greater
            },
            &ctx
        ));
    }

    #[test]
    fn disabled_rules_are_ignored() {
        let policy = TacticalPolicy {
            enabled: true,
            rules: vec![
                TacticalRule {
                    enabled: false,
                    decision: TacticalDecisionPoint::NextAttackOpportunity,
                    conditions: vec![TacticalCondition::Always],
                    action: TacticalAction::Jab,
                },
                TacticalRule::new(
                    TacticalAction::NormalAttack,
                    vec![TacticalCondition::Always],
                ),
            ],
        };
        let decision = evaluate_channel(
            &policy,
            TacticalDecisionPoint::NextAttackOpportunity,
            TacticalChannel::AttackMode,
            &context(),
        );
        assert_eq!(decision.action, TacticalAction::NormalAttack);
        assert_eq!(decision.matched_rule_index, Some(1));
    }

    #[test]
    fn only_shield_and_storm_form_a_two_style_selection() {
        assert!(valid_style_selection_shape(&[
            SHIELD_OF_BLADES_STYLE_ID.to_string(),
            STORM_OF_BLADES_STYLE_ID.to_string()
        ]));
        assert!(!valid_style_selection_shape(&[
            "falling_sun".to_string(),
            "hammerer".to_string()
        ]));
        assert_eq!(
            canonicalize_style_selection(vec![
                STORM_OF_BLADES_STYLE_ID.to_string(),
                SHIELD_OF_BLADES_STYLE_ID.to_string()
            ]),
            vec![
                SHIELD_OF_BLADES_STYLE_ID.to_string(),
                STORM_OF_BLADES_STYLE_ID.to_string()
            ]
        );
    }

    #[test]
    fn policy_validation_rejects_more_than_two_conditions() {
        let policy = TacticalPolicy {
            enabled: true,
            rules: vec![TacticalRule::new(
                TacticalAction::NormalAttack,
                vec![
                    TacticalCondition::Always,
                    TacticalCondition::Always,
                    TacticalCondition::Always,
                ],
            )],
        };
        assert!(validate_policy(&policy).is_err());
    }
}
