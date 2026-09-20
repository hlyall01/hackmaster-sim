use super::{Combatant, StatIdI32, WeaponSlot};

/// The same impact rule is used by combat resolution and the Derived display.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnockbackRule {
    pub base_threshold: i32,
    pub weapon_adjustment: i32,
    pub mounted_adjustment: i32,
    pub damage_per_step: i32,
    pub damage_multiplier: i32,
}

impl KnockbackRule {
    pub fn distance_ft(self, raw_damage: i32) -> f32 {
        // Multiply damage before dividing into steps; doubling rounded distance
        // would incorrectly discard partial steps on a charge.
        let impact = i64::from(raw_damage.max(0)) * i64::from(self.damage_multiplier);
        (impact / i64::from(self.damage_per_step) * 5) as f32
    }
}

pub fn knockback_rule_for_attack(
    attacker: &Combatant,
    defender: &Combatant,
    weapon_slot: Option<WeaponSlot>,
    is_ranged: bool,
    charging: bool,
) -> KnockbackRule {
    let base_threshold = defender.apply_i32(
        StatIdI32::KnockbackStep,
        defender.sheet.defense.knockback_step,
    );
    let weapon = match weapon_slot {
        Some(WeaponSlot::Primary) => Some(&attacker.sheet.offense.weapon),
        Some(WeaponSlot::Secondary) => attacker
            .sheet
            .offense
            .offhand
            .as_ref()
            .map(|hand| &hand.weapon),
        None => None,
    };
    let weapon_adjustment = weapon.map_or(0, |weapon| weapon.defender_knockback_step_adjustment);
    let mounted_adjustment = if !is_ranged
        && attacker.sheet.maneuvers.mounted
        && attacker.sheet.maneuvers.mounted_combat.trot_or_faster
    {
        -5
    } else {
        0
    };
    KnockbackRule {
        base_threshold,
        weapon_adjustment,
        mounted_adjustment,
        damage_per_step: base_threshold
            .saturating_add(weapon_adjustment)
            .saturating_add(mounted_adjustment)
            .max(1),
        damage_multiplier: if charging { 2 } else { 1 },
    }
}
