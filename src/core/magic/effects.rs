//! Declarative spell effects. IDs and names never select runtime behavior.
use super::{MagicError, MagicSaveKind};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectTarget {
    Caster,
    Enemy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RangePolicy {
    SpellRange,
    WeaponReach,
    SweepDistance,
    AnyEnemy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CastingAbility {
    Strength,
    Constitution,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaveOutcome {
    Negate,
    Half,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveRule {
    pub kind: MagicSaveKind,
    pub outcome: SaveOutcome,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageRule {
    pub sides: i32,
    pub penetrating: bool,
    pub bonus_per_die: i32,
    pub flat_bonus: i32,
    /// Natural DR always applies; worn armor is an explicit exception.
    #[serde(default)]
    pub armor_reduction: bool,
    pub phantom: bool,
    pub knockback: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttackRule {
    pub attack_die: i32,
    pub defense_die: i32,
    pub defense_bonus: i32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovementDefenseRule {
    pub melee_bonus: i32,
    pub ranged_distance: f32,
}

/// The attack-echo callback is the one legacy event-hook adapter. Its tuning is
/// catalog data too; no new spell should add another ID-based dispatch branch.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EchoRules {
    pub base_duration: u32,
    pub default_delay: u32,
    pub minimum_delay: u32,
    pub duration_cost_per_second: u32,
    pub cost_per_second_faster: u32,
    pub cost_per_extra_echo: u32,
    pub full_damage_cost: u32,
    pub normal_damage_divisor: i32,
    pub defense_per_echo: i32,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum DamageDelivery {
    Volley,
    Chain {
        turn_degrees: f32,
        dice_lost_per_target: u32,
    },
    MovingLine {
        caster_immune: bool,
        ignore_traumatized: bool,
    },
    DelayedRadius {
        caster_immune: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum StrengthRule {
    Set {
        fractional_score: u32,
    },
    AddRoll {
        sides: u32,
        fractional_units_per_point: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplacementDistance {
    ForceOverWeight,
    KnockbackImpact,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplacementDirection {
    TowardTarget,
    AwayFromCaster,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum EffectRule {
    TimedBuff {
        movement_defense: Option<MovementDefenseRule>,
        average_incoming_damage: bool,
    },
    Strength {
        score: StrengthRule,
        maximum_base: u8,
        priority: i32,
        consume_on_attack: bool,
    },
    Damage {
        delivery: DamageDelivery,
        damage: DamageRule,
        attack: Option<AttackRule>,
        save: Option<SaveRule>,
    },
    Displace {
        subject: EffectTarget,
        distance: DisplacementDistance,
        direction: DisplacementDirection,
        stop_distance: f32,
        recovery: bool,
        save: Option<SaveRule>,
    },
    ExtendTrauma {
        save: Option<SaveRule>,
    },
    RecoveryField {
        extra_seconds: i32,
        save: SaveRule,
    },
    SummonMelee {
        count: u32,
        hp_percent: u32,
        defense_die: i32,
        defense_bonus: i32,
        virtual_strength: u32,
        strength_damage: i32,
        maximum_strength_base: u8,
        damage: DamageRule,
    },
    Sequence {
        effects: Vec<EffectRule>,
    },
}

impl EffectRule {
    pub fn channelled(&self) -> bool {
        match self {
            Self::SummonMelee { .. } => true,
            Self::Sequence { effects } => effects.iter().any(Self::channelled),
            _ => false,
        }
    }

    fn validate(&self, depth: usize) -> Result<(), String> {
        if depth > 8 {
            return Err("Effect sequences may nest at most eight levels".into());
        }
        let die = |sides: i32| (2..=1000).contains(&sides);
        let damage_valid = |d: &DamageRule| {
            die(d.sides)
                && d.bonus_per_die.unsigned_abs() <= 10000
                && d.flat_bonus.unsigned_abs() <= 10000
        };
        let valid = match self {
            Self::TimedBuff {
                movement_defense, ..
            } => movement_defense.as_ref().is_none_or(|m| {
                m.melee_bonus.unsigned_abs() <= 10000
                    && m.ranged_distance.is_finite()
                    && (0.0..=1_000_000.0).contains(&m.ranged_distance)
            }),
            Self::Strength {
                score,
                maximum_base,
                ..
            } => {
                *maximum_base > 0
                    && *maximum_base <= 25
                    && match score {
                        StrengthRule::Set { fractional_score } => {
                            *fractional_score > 0 && *fractional_score <= 2599
                        }
                        StrengthRule::AddRoll {
                            sides,
                            fractional_units_per_point,
                        } => {
                            (1..=100).contains(sides)
                                && (1..=100).contains(fractional_units_per_point)
                        }
                    }
            }
            Self::Damage {
                delivery,
                damage,
                attack,
                ..
            } => {
                damage_valid(damage)
                    && attack.is_none_or(|a| {
                        die(a.attack_die)
                            && die(a.defense_die)
                            && a.defense_bonus.unsigned_abs() <= 10000
                    })
                    && match delivery {
                        DamageDelivery::Chain {
                            turn_degrees,
                            dice_lost_per_target,
                        } => {
                            turn_degrees.is_finite()
                                && (0.0..=180.0).contains(turn_degrees)
                                && *dice_lost_per_target > 0
                        }
                        _ => true,
                    }
            }
            Self::Displace {
                stop_distance,
                subject,
                direction,
                save,
                ..
            } => {
                stop_distance.is_finite()
                    && *stop_distance >= 0.0
                    && matches!(
                        (subject, direction),
                        (EffectTarget::Caster, DisplacementDirection::TowardTarget)
                            | (EffectTarget::Enemy, DisplacementDirection::AwayFromCaster)
                    )
                    && save.is_none_or(|s| s.outcome == SaveOutcome::Negate)
            }
            Self::ExtendTrauma { save } => save.is_none_or(|s| s.outcome == SaveOutcome::Negate),
            Self::RecoveryField {
                extra_seconds,
                save,
            } => (1..=1000).contains(extra_seconds) && save.outcome == SaveOutcome::Negate,
            Self::SummonMelee {
                count,
                hp_percent,
                defense_die,
                defense_bonus,
                virtual_strength,
                maximum_strength_base,
                damage,
                ..
            } => {
                (1..=16).contains(count)
                    && (1..=100).contains(hp_percent)
                    && die(*defense_die)
                    && defense_bonus.unsigned_abs() <= 10000
                    && (100..=2599).contains(virtual_strength)
                    && *maximum_strength_base > 0
                    && *maximum_strength_base <= 25
                    && damage_valid(damage)
            }
            Self::Sequence { effects } => {
                !effects.is_empty()
                    && effects.len() <= 16
                    && effects.iter().all(|e| e.validate(depth + 1).is_ok())
                    && effects.iter().filter(|e| e.channelled()).count() <= 1
            }
        };
        if valid {
            Ok(())
        } else {
            Err("Invalid effect rule or unsupported combination".into())
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EffectValues {
    pub range: f32,
    pub duration: u32,
    pub radius: f32,
    pub dice: u32,
    pub projectiles: u32,
    pub bonus: i32,
    pub force: u32,
    pub width: f32,
    pub speed: f32,
    pub strength: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectStat {
    Range,
    Duration,
    Radius,
    Dice,
    Projectiles,
    Bonus,
    Force,
    Width,
    Speed,
    Strength,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectScaling {
    pub stat: EffectStat,
    pub per_rank: f64,
}

impl EffectValues {
    pub fn apply(&mut self, scaling: &EffectScaling, ranks: u32) -> Result<(), MagicError> {
        if !scaling.per_rank.is_finite()
            || scaling.per_rank < 0.0
            || (!matches!(
                scaling.stat,
                EffectStat::Range | EffectStat::Radius | EffectStat::Width | EffectStat::Speed
            ) && scaling.per_rank.fract() != 0.0)
        {
            return Err(MagicError::InvalidSpell);
        }
        let amount = scaling.per_rank * f64::from(ranks);
        fn unsigned(value: &mut u32, amount: f64) -> Result<(), MagicError> {
            let result = f64::from(*value) + amount;
            if !result.is_finite()
                || result < 0.0
                || result > i32::MAX as f64
                || result.fract() != 0.0
            {
                return Err(MagicError::CostOverflow);
            }
            *value = result as u32;
            Ok(())
        }
        fn float(value: &mut f32, amount: f64) -> Result<(), MagicError> {
            let result = f64::from(*value) + amount;
            if !result.is_finite() || !(0.0..=1_000_000.0).contains(&result) {
                return Err(MagicError::CostOverflow);
            }
            *value = result as f32;
            Ok(())
        }
        match scaling.stat {
            EffectStat::Range => float(&mut self.range, amount),
            EffectStat::Duration => unsigned(&mut self.duration, amount),
            EffectStat::Radius => float(&mut self.radius, amount),
            EffectStat::Dice => unsigned(&mut self.dice, amount),
            EffectStat::Projectiles => unsigned(&mut self.projectiles, amount),
            EffectStat::Force => unsigned(&mut self.force, amount),
            EffectStat::Width => float(&mut self.width, amount),
            EffectStat::Speed => float(&mut self.speed, amount),
            EffectStat::Strength => unsigned(&mut self.strength, amount),
            EffectStat::Bonus => {
                let result = f64::from(self.bonus) + amount;
                if !result.is_finite() || result.abs() > 1_000_000.0 || result.fract() != 0.0 {
                    return Err(MagicError::CostOverflow);
                }
                self.bonus = result as i32;
                Ok(())
            }
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if [self.range, self.radius, self.width, self.speed]
            .iter()
            .any(|x| !x.is_finite() || !(0.0..=1_000_000.0).contains(x))
            || self.duration > i32::MAX as u32
            || self.dice > 1000
            || self.projectiles > 1000
            || self.force > 1_000_000
            || self.strength > 10000
        {
            Err("Effect values outside supported numeric limits".into())
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectDefinition {
    pub target: EffectTarget,
    pub ai_range: RangePolicy,
    pub casting_ability: CastingAbility,
    pub values: EffectValues,
    pub effect: EffectRule,
}

impl EffectDefinition {
    pub fn validate(&self) -> Result<(), String> {
        self.effect.validate(0)?;
        self.validate_values(&self.values)
    }

    pub fn validate_values(&self, values: &EffectValues) -> Result<(), String> {
        values.validate()?;
        fn geometry(rule: &EffectRule, target: EffectTarget, v: &EffectValues) -> bool {
            match rule {
                EffectRule::TimedBuff { .. } | EffectRule::Strength { .. } => {
                    target == EffectTarget::Caster && v.duration > 0
                }
                EffectRule::Damage { delivery, .. } => {
                    v.dice > 0
                        && match delivery {
                            DamageDelivery::Volley => v.projectiles > 0,
                            DamageDelivery::Chain { .. } => {
                                target == EffectTarget::Enemy && v.range > 0.0
                            }
                            DamageDelivery::MovingLine { .. } => {
                                target == EffectTarget::Enemy
                                    && v.duration > 0
                                    && v.width > 0.0
                                    && v.speed > 0.0
                            }
                            DamageDelivery::DelayedRadius { .. } => v.radius > 0.0,
                        }
                }
                EffectRule::Displace { .. } => target == EffectTarget::Enemy && v.force > 0,
                EffectRule::ExtendTrauma { .. } | EffectRule::RecoveryField { .. } => {
                    v.duration > 0 && v.radius > 0.0
                }
                EffectRule::SummonMelee { .. } => {
                    target == EffectTarget::Enemy && v.range > 0.0 && v.duration > 0
                }
                EffectRule::Sequence { effects } => effects.iter().all(|e| geometry(e, target, v)),
            }
        }
        if !geometry(&self.effect, self.target, values)
            || (self.ai_range == RangePolicy::SpellRange && values.range <= 0.0)
            || (self.ai_range == RangePolicy::SweepDistance
                && (values.speed <= 0.0 || values.duration == 0))
        {
            Err("Effect targeting or geometry is missing required values".into())
        } else {
            Ok(())
        }
    }
}
