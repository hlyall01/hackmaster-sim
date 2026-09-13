//! Mounted combat conditions (PHB pp. 186, 201, 233).
//! These are fixed scenario inputs, not a simulation of horse movement.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RidingMastery {
    #[default]
    Average,
    Advanced,
    Expert,
    Master,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MountType {
    RidingHorse,
    #[default]
    Rounsey,
    Courser,
    Destrier,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MountedTargetSize {
    #[default]
    FromDefender,
    Medium,
    Other,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct MountedCombatConfig {
    pub riding: RidingMastery,
    pub mount: MountType,
    pub trot_or_faster: bool,
    pub target_size: MountedTargetSize,
}

impl MountedCombatConfig {
    /// PHB p. 233: applies to Defense rolls with any weapon or mount type.
    pub fn defense_bonus(self, mounted: bool) -> i32 {
        if !mounted { 0 } else if self.trot_or_faster { 6 } else { 2 }
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn moving_lance(self, weapon_name: &str) -> bool {
        weapon_name.eq_ignore_ascii_case("Lance")
            && self.mount != MountType::RidingHorse
            && self.trot_or_faster
    }

    pub fn attack_bonus(self, weapon_name: &str, ranged: bool) -> i32 {
        if ranged {
            // The +2 height bonus belongs to melee; mounted archery uses Riding penalties.
            return match self.riding {
                RidingMastery::Average => -6,
                RidingMastery::Advanced => -4,
                RidingMastery::Expert => -2,
                RidingMastery::Master => 0,
            };
        }
        let bonus = if self.moving_lance(weapon_name) { 6 } else { 2 };
        bonus
            - if self.riding == RidingMastery::Average {
                2
            } else {
                0
            }
    }

    pub(crate) fn damage_plan(self, weapon_name: &str, target_medium: bool) -> MountedDamagePlan {
        let target_medium = match self.target_size {
            MountedTargetSize::FromDefender => target_medium,
            MountedTargetSize::Medium => true,
            MountedTargetSize::Other => false,
        };
        let warhorse = if self.trot_or_faster {
            match self.mount {
                MountType::Courser => 1,
                MountType::Destrier => 2,
                _ => 0,
            }
        } else {
            0
        };
        let ordinary = if target_medium {
            if is_saddle_weapon(weapon_name) { 2 } else { 1 }
        } else {
            0
        };
        if self.moving_lance(weapon_name) {
            // Fixed table ruling: double base dice, then add mounted and horse dice.
            MountedDamagePlan {
                double_base_dice: true,
                extra_smallest: ordinary + warhorse,
            }
        } else {
            MountedDamagePlan {
                double_base_dice: false,
                extra_smallest: warhorse
                    + if weapon_name.eq_ignore_ascii_case("Lance") {
                        1
                    } else {
                        ordinary
                    },
            }
        }
    }
}

// H suffix on weapon names in the PHB weapon tables, not the H (hacking) type.
// Sabre is a knightly mounted proficiency, but has no H suffix in that table.
pub fn is_saddle_weapon(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "horseman's pick" | "horseman's mace" | "horseman's flail" | "lance"
    )
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct MountedDamagePlan {
    pub double_base_dice: bool,
    pub extra_smallest: usize,
}
