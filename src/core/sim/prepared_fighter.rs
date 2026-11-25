//! Immutable sheet values used by release bulk jobs. Temporary modifiers are resolved live.
use super::{Combatant, CombatantSheet, KnockbackRule, ShieldBreakageStep, StatIdF32, StatIdI32};

#[derive(Debug)]
pub(super) struct PreparedFighter {
    pub attack: i32,
    pub strength: i32,
    pub penetration: i32,
    pub lost_shield_penalty: bool,
    pub reach: f32,
    pub drops_shield_for_eyesmite: bool,
    pub knockback: KnockbackRule,
    shield_tiers: Option<[u8; 64]>,
}

pub(super) fn shield_tier(steps: &[ShieldBreakageStep; 4], damage: i32) -> u8 {
    steps.iter().rposition(|s| damage >= s.threshold).map_or(0, |i| (i + 1) as u8)
}

pub(super) fn drops_shield(name: Option<&str>) -> bool {
    name.is_some_and(|name| {
        let normalized = name.trim().to_ascii_lowercase();
        normalized == "buckler" || (normalized.starts_with("small ") && normalized.contains("shield"))
    })
}

impl PreparedFighter {
    pub fn new(sheet: &CombatantSheet, opponent_adjustment: i32) -> Self {
        let weapon = &sheet.offense.weapon;
        let value = |stat, base| sheet.modifiers.apply_i32(base, stat);
        let base_threshold = value(StatIdI32::KnockbackStep, sheet.defense.knockback_step);
        Self {
            attack: value(StatIdI32::AttackBonus, sheet.offense.attack_bonus),
            strength: value(StatIdI32::StrengthDamage, sheet.offense.strength_damage),
            penetration: value(StatIdI32::ArmorPenetration, weapon.armor_penetration),
            lost_shield_penalty: sheet.modifiers.positive_at_zero(StatIdI32::FlagLargeSwordShieldStyle),
            reach: sheet.modifiers.apply_f32(weapon.reach_ft, StatIdF32::WeaponReach).max(1.0),
            drops_shield_for_eyesmite: drops_shield(sheet.defense.shield_name.as_deref()),
            knockback: KnockbackRule {
                base_threshold, weapon_adjustment: opponent_adjustment, mounted_adjustment: 0,
                damage_per_step: base_threshold.saturating_add(opponent_adjustment).max(1), damage_multiplier: 1,
            },
            shield_tiers: sheet.defense.shield_breakage.map(|steps|
                std::array::from_fn(|damage| shield_tier(&steps, damage as i32))),
        }
    }

    pub fn break_tier(&self, raw: i32) -> Option<u8> {
        self.shield_tiers.as_ref()?.get(raw as usize).copied()
    }
}

impl Combatant {
    pub(super) fn effective_reach(&self) -> f32 {
        if self.state.active_effects.is_empty() {
            if let Some(p) = &self.state.prepared { return p.reach; }
        }
        self.apply_f32(StatIdF32::WeaponReach, self.sheet.offense.weapon.reach_ft).max(1.0)
    }

    pub(super) fn refresh_prepared_sheet(&mut self) {
        if let Some(previous) = &self.state.prepared {
            self.state.prepared = Some(PreparedFighter::new(&self.sheet, previous.knockback.weapon_adjustment).into());
        }
    }
}
