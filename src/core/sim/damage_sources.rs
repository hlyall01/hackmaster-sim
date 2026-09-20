use super::types::{WeaponProfile, WeaponSlot};

/// Captured when damage resolves, so later equipment changes cannot relabel it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DamageSource {
    Weapon { name: String, slot: WeaponSlot },
    Unarmed { name: String },
    Spell { name: String },
    Ability { name: String },
}

impl DamageSource {
    pub(super) fn weapon(weapon: &WeaponProfile, slot: WeaponSlot) -> Self {
        if weapon.is_unarmed {
            Self::Unarmed {
                name: weapon.name.clone(),
            }
        } else {
            Self::Weapon {
                name: weapon.name.clone(),
                slot,
            }
        }
    }
}

impl std::fmt::Display for DamageSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Weapon { name, slot } => {
                let hand = match slot {
                    WeaponSlot::Primary => "main hand",
                    WeaponSlot::Secondary => "off hand",
                };
                write!(f, "{name} ({hand})")
            }
            Self::Unarmed { name } => write!(f, "{name} (unarmed)"),
            Self::Spell { name } => write!(f, "{name} (spell)"),
            Self::Ability { name } => write!(f, "{name} (ability)"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DamageSourceStats {
    pub source: DamageSource,
    pub total_hp_damage: u64,
    pub avg_hp_damage_per_fight: f32,
    /// Fraction of the team's total HP damage, or zero if no HP damage was dealt.
    pub damage_share: f32,
    pub combat_dps: f32,
}
