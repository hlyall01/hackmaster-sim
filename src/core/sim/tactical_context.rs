//! Static dependencies of each tactical channel. Facts are read at the decision point.
use crate::core::tactics::{NumericComparison, TacticalAction, TacticalChannel, TacticalCondition, TacticalPolicy};

#[derive(Clone, Debug, Default)]
pub(super) struct ChannelPlan {
    pub rules: Vec<usize>,
    pub needs: ContextNeeds,
    pub single_rule: Option<(usize, SingleCondition)>,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum SingleCondition {
    Always,
    EnemyDr(NumericComparison, f32),
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct ContextNeeds {
    pub reaction: bool,
    pub retreat: bool,
    pub enemy_charging: bool,
    pub distance: bool,
    pub engaged: bool,
    pub time: bool,
    pub reach: bool,
    pub attack_speed: bool,
    pub jab: bool,
    pub styles: bool,
    pub my_hp: bool,
    pub enemy_hp: bool,
    pub weapon_group: bool,
    pub armor: bool,
    pub my_styles: bool,
    pub enemy_styles: bool,
    pub enemy_dr: bool,
}

impl ContextNeeds {
    pub fn reaction() -> Self {
        Self { reaction: true, retreat: true, enemy_charging: true, ..Self::default() }
    }

    #[cfg(test)]
    pub fn all() -> Self {
        Self {
            reaction: true, retreat: true, enemy_charging: true, distance: true,
            engaged: true, time: true, reach: true, attack_speed: true, jab: true,
            styles: true, my_hp: true, enemy_hp: true, weapon_group: true, armor: true,
            my_styles: true, enemy_styles: true, enemy_dr: true,
        }
    }

    fn include(&mut self, condition: &TacticalCondition) {
        use TacticalCondition::*;
        match condition {
            Always | MyHasActiveShield { .. } | EnemyHasActiveShield { .. } | MyHasAttacked { .. } => {}
            MyHpPercent { .. } => self.my_hp = true,
            EnemyHpPercent { .. } => self.enemy_hp = true,
            DistanceFt { .. } => self.distance = true,
            EngagedInMelee { .. } => self.engaged = true,
            ReachComparedToEnemy { .. } => self.reach = true,
            RetreatSpaceAvailable { .. } => self.retreat = true,
            MyWeaponCanJab { .. } => self.jab = true,
            EnemyWeaponGroup { .. } => self.weapon_group = true,
            EnemyArmorType { .. } => self.armor = true,
            EnemyCharging { .. } => self.enemy_charging = true,
            EnemyTimeToReachSeconds { .. } => {
                self.time = true;
                self.reach = true;
                self.distance = true;
            }
            MyActiveStyle { .. } => self.my_styles = true,
            EnemyActiveStyle { .. } => self.enemy_styles = true,
            EnemyDr { .. } => self.enemy_dr = true,
            EnemyAttackSpeedSeconds { .. } | EnemyAttackSpeedComparedToMine { .. } => self.attack_speed = true,
        }
    }
}

impl ChannelPlan {
    pub fn new(policy: &TacticalPolicy, channel: TacticalChannel) -> Self {
        let mut plan = Self::default();
        if policy.enabled {
            for (i, rule) in policy.rules.iter().enumerate() {
                if !rule.enabled || rule.action.channel() != channel || rule.decision != rule.action.decision_point() {
                    continue;
                }
                plan.rules.push(i);
                for condition in &rule.conditions {
                    plan.needs.include(condition);
                }
                match rule.action {
                    TacticalAction::GiveGround | TacticalAction::ScamperBack => {
                        plan.needs.reaction = true;
                        plan.needs.retreat = true;
                    }
                    TacticalAction::UseWeaponStyle { .. } => plan.needs.styles = true,
                    TacticalAction::Jab => plan.needs.jab = true,
                    _ => {}
                }
            }
        }
        if let [index] = plan.rules[..] {
            let rule = &policy.rules[index];
            // These actions have no equipment or reaction legality checks.
            if matches!(rule.action, TacticalAction::CalledShot | TacticalAction::NormalAttack |
                TacticalAction::NeutralStance | TacticalAction::FightDefensively { .. }) {
                plan.single_rule = match rule.conditions.as_slice() {
                    [] | [TacticalCondition::Always] => Some((index, SingleCondition::Always)),
                    [TacticalCondition::EnemyDr { comparison, value }] =>
                        Some((index, SingleCondition::EnemyDr(*comparison, *value))),
                    _ => None,
                };
            }
        }
        plan
    }
}
