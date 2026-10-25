//! Player choices and resolved effects for the ranged weapon styles.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BowPace {
    Walk,
    #[default]
    Jog,
    Run,
}

impl BowPace {
    pub fn label(self) -> &'static str {
        match self {
            Self::Walk => "Walk",
            Self::Jog => "Jog",
            Self::Run => "Run (-4 attack)",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GravesendAim {
    #[default]
    InvertRange,
    PierceArmor,
    ForceTrauma,
}

impl GravesendAim {
    pub fn seconds(self) -> f32 {
        match self {
            Self::InvertRange => 5.0,
            Self::PierceArmor => 10.0,
            Self::ForceTrauma => 20.0,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::InvertRange => "5 seconds: invert range bands",
            Self::PierceArmor => "10 seconds: +2 attack, ignore armor",
            Self::ForceTrauma => "20 seconds: also force Trauma failure",
        }
    }
    pub fn ignores_armor(self) -> bool {
        self != Self::InvertRange
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RangedStyleConfig {
    pub whistling_wind_pace: BowPace,
    pub gravesend_aim: GravesendAim,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RangedStyleProfile {
    pub whistling_wind_pace: Option<BowPace>,
    pub gravesend_aim: Option<GravesendAim>,
}

impl RangedStyleProfile {
    pub fn aim_seconds(self) -> f32 {
        self.gravesend_aim.map_or(0.0, GravesendAim::seconds)
    }
    pub fn attack_bonus(self) -> i32 {
        if self.gravesend_aim.is_some_and(GravesendAim::ignores_armor) {
            2
        } else {
            0
        }
    }
    pub fn range_modifier(self, normal: i32) -> i32 {
        if self.gravesend_aim.is_none() {
            return normal;
        }
        match normal {
            0 => -8,
            -4 => -6,
            -6 => -4,
            -8 => 0,
            _ => normal,
        }
    }
}

pub(crate) fn shooting_move_speed(combatant: &super::Combatant, modified_speed: f32) -> f32 {
    let weapon = &combatant.sheet.offense.weapon;
    if combatant.sheet.maneuvers.mounted
        || !weapon.uses_projectiles
        || super::movement::max_range_for_weapon(weapon).is_none()
    {
        return modified_speed;
    }
    let pace = weapon
        .ranged_styles
        .whistling_wind_pace
        .unwrap_or(BowPace::Walk);
    let fraction = match pace {
        BowPace::Walk => 0.25,
        BowPace::Jog => 0.5,
        BowPace::Run if combatant.state.moved_last_tick && !combatant.magic_blocks_running() => {
            0.75
        }
        BowPace::Run => 0.5,
    };
    let normal_speed = combatant.sheet.mobility.move_speed.max(modified_speed);
    modified_speed.min(normal_speed * fraction)
}

pub(crate) fn bow_running(combatant: &super::Combatant, modified_speed: f32) -> bool {
    combatant
        .sheet
        .offense
        .weapon
        .ranged_styles
        .whistling_wind_pace
        == Some(BowPace::Run)
        && !combatant.sheet.maneuvers.mounted
        && combatant.state.moved_last_tick
        && !combatant.magic_blocks_running()
        && combatant.state.trauma_remaining_seconds == 0
        && combatant.state.knockback_immobile_seconds == 0
        && shooting_move_speed(combatant, modified_speed)
            > combatant.sheet.mobility.move_speed * 0.5
}
