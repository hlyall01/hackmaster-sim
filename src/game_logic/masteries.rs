//! Group-specific combat mastery and migration of older fighter presets.
use super::*;
use crate::character::MasteryState;
use std::collections::BTreeMap;

pub type WeaponMasteries = BTreeMap<WeaponGroup, MasteryState>;

pub const MASTERY_GROUPS: [WeaponGroup; 14] = [
    WeaponGroup::Unarmed,
    WeaponGroup::Axes,
    WeaponGroup::Basic,
    WeaponGroup::Blunt,
    WeaponGroup::Ensnaring,
    WeaponGroup::Lashes,
    WeaponGroup::LargeSwords,
    WeaponGroup::SmallSwords,
    WeaponGroup::Polearms,
    WeaponGroup::Spears,
    WeaponGroup::Double,
    WeaponGroup::Bows,
    WeaponGroup::Crossbows,
    WeaponGroup::Shields,
];

pub fn mastery_group_label(group: WeaponGroup) -> &'static str {
    match group {
        WeaponGroup::Unarmed => "Unarmed",
        WeaponGroup::Axes => "Axes",
        WeaponGroup::Basic => "Basic",
        WeaponGroup::Blunt => "Blunt",
        WeaponGroup::Ensnaring => "Ensnaring",
        WeaponGroup::Lashes => "Lashes",
        WeaponGroup::LargeSwords => "Large Swords",
        WeaponGroup::SmallSwords => "Small Swords",
        WeaponGroup::Polearms => "Polearms",
        WeaponGroup::Spears => "Spears",
        WeaponGroup::Double => "Double",
        WeaponGroup::Bows => "Bows",
        WeaponGroup::Crossbows => "Crossbows",
        WeaponGroup::Shields => "Shields",
    }
}

pub fn mastery_has_attack_and_damage(group: WeaponGroup) -> bool {
    group != WeaponGroup::Shields
}

pub fn mastery_has_defense(group: WeaponGroup) -> bool {
    !matches!(group, WeaponGroup::Bows | WeaponGroup::Crossbows)
}

pub fn normalized_group_mastery(group: WeaponGroup, points: MasteryState) -> MasteryState {
    MasteryState {
        attack: if mastery_has_attack_and_damage(group) {
            clamp_mastery(points.attack)
        } else {
            0
        },
        damage: if mastery_has_attack_and_damage(group) {
            clamp_mastery(points.damage)
        } else {
            0
        },
        defense: if mastery_has_defense(group) {
            clamp_mastery(points.defense)
        } else {
            0
        },
        speed: clamp_mastery(points.speed),
    }
}

impl PlayerConfig {
    /// Missing groups have zero mastery, regardless of the equipped weapon.
    pub fn mastery(&self, group: WeaponGroup) -> MasteryState {
        normalized_group_mastery(
            group,
            self.weapon_masteries
                .get(&group)
                .copied()
                .unwrap_or_default(),
        )
    }

    pub fn mastery_mut(&mut self, group: WeaponGroup) -> &mut MasteryState {
        self.weapon_masteries.entry(group).or_default()
    }
}

impl FighterMasteries {
    pub fn is_empty(&self) -> bool {
        self.attack == 0
            && self.damage == 0
            && self.defense == 0
            && self.speed == 0
            && self.shield_defense == 0
            && self.shield_speed == 0
    }
}

/// Legacy global bonuses belong to the preset's saved weapon groups, never to
/// whichever weapon happens to be equipped after loading the preset.
pub fn weapon_masteries_for_preset(
    preset: &FighterPreset,
    weapons: &WeaponCatalog,
) -> WeaponMasteries {
    if let Some(groups) = &preset.weapon_masteries {
        return groups
            .iter()
            .map(|(&group, &points)| (group, normalized_group_mastery(group, points)))
            .collect();
    }
    let legacy = &preset.masteries;
    let points = MasteryState {
        attack: legacy.attack,
        damage: legacy.damage,
        defense: legacy.defense,
        speed: legacy.speed,
    };
    let mut groups = WeaponMasteries::new();
    for name in std::iter::once(preset.weapon.as_str()).chain(preset.offhand_weapon.as_deref()) {
        if let Some(weapon) = weapons
            .entries()
            .iter()
            .find(|weapon| weapon.name.eq_ignore_ascii_case(name))
        {
            groups.insert(weapon.group, normalized_group_mastery(weapon.group, points));
        }
    }
    groups.insert(
        WeaponGroup::Shields,
        normalized_group_mastery(
            WeaponGroup::Shields,
            MasteryState {
                defense: legacy.shield_defense,
                speed: legacy.shield_speed,
                ..MasteryState::default()
            },
        ),
    );
    groups
}
