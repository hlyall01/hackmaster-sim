use super::types::{WeaponProfile, WeaponSlot};

/// Captured when damage resolves, so later equipment changes cannot relabel it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum DamageSource {
    Weapon { name: String, slot: WeaponSlot },
    Unarmed { name: String },
    Spell { name: String },
    Ability { name: String },
}

/// Capture immutable equipment at impact; only materialise a display string at the output boundary.
#[derive(Clone, Debug)]
pub(super) enum ImpactSource {
    Weapon(std::sync::Arc<WeaponProfile>, WeaponSlot),
    Punch,
    Eyesmite,
    Echo,
    Spell(std::sync::Arc<crate::core::magic::SpellCatalogEntry>),
    NamedSpell(String),
}

impl ImpactSource {
    pub(super) fn spell(effect: &super::ConfiguredEffect) -> Self {
        if effect.name == effect.catalog.name {
            Self::Spell(effect.catalog.clone())
        } else {
            Self::NamedSpell(effect.name.clone())
        }
    }

    pub(super) fn describe(&self) -> DamageSource {
        match self {
            Self::Weapon(weapon, slot) => DamageSource::weapon(weapon, *slot),
            Self::Punch => DamageSource::Unarmed { name: "Near-perfect defense punch".into() },
            Self::Eyesmite => DamageSource::Ability { name: "Eyesmite".into() },
            Self::Echo => DamageSource::Spell { name: "Echo Strike".into() },
            Self::Spell(spell) => DamageSource::Spell { name: spell.name.clone() },
            Self::NamedSpell(name) => DamageSource::Spell { name: name.clone() },
        }
    }

    pub(super) fn same_identity(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Weapon(a, slot_a), Self::Weapon(b, slot_b)) =>
                slot_a == slot_b && std::sync::Arc::ptr_eq(a, b),
            (Self::Spell(a), Self::Spell(b)) => a.name == b.name,
            (Self::NamedSpell(a), Self::NamedSpell(b)) => a == b,
            (Self::Echo, Self::Echo) | (Self::Punch, Self::Punch) | (Self::Eyesmite, Self::Eyesmite) => true,
            _ => false,
        }
    }
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
#[derive(serde::Serialize, serde::Deserialize)]
pub struct DamageSourceStats {
    pub source: DamageSource,
    pub total_hp_damage: u64,
    pub avg_hp_damage_per_fight: f32,
    /// Fraction of the team's total HP damage, or zero if no HP damage was dealt.
    pub damage_share: f32,
    pub combat_dps: f32,
}
