//! Two carried weapon setups and their per-fight draw/drop state.
use super::Combatant;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponSetup {
    Melee,
    #[default]
    Ranged,
}

impl WeaponSetup {
    pub fn label(self) -> &'static str {
        match self {
            Self::Melee => "Melee",
            Self::Ranged => "Ranged",
        }
    }
    pub(crate) fn index(self) -> usize {
        match self {
            Self::Melee => 0,
            Self::Ranged => 1,
        }
    }
}

fn default_auto_switch() -> bool {
    true
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RangedWeaponSetup {
    pub weapon: String,
    #[serde(default)]
    pub material_tier: i32,
    #[serde(default)]
    pub projectile_material_tier: i32,
    #[serde(default)]
    pub iron: bool,
    /// Missing values inherit the legacy shared Called Shot preference.
    #[serde(default)]
    pub called_shot: Option<bool>,
    #[serde(default)]
    pub default_style_ids: Option<Vec<String>>,
    #[serde(default)]
    pub start_with: WeaponSetup,
    #[serde(default = "default_auto_switch")]
    pub auto_switch_to_melee: bool,
}

#[derive(Clone, Debug)]
pub struct WeaponSetups {
    pub melee: Combatant,
    pub ranged: Combatant,
    pub drawing: Combatant,
    pub melee_draw_seconds: f32,
    pub start_with: WeaponSetup,
    pub auto_switch_to_melee: bool,
}

#[derive(Clone, Debug)]
pub struct PendingWeaponSetup {
    pub target: WeaponSetup,
    pub equipment_ready_at: f32,
    pub attack_ready_at: f32,
}

#[derive(Clone, Debug, Default)]
pub struct WeaponSetupState {
    pub active: WeaponSetup,
    pub pending: Option<PendingWeaponSetup>,
    pub ranged_dropped: bool,
    pub recoveries: [Option<f32>; 2],
    pub has_attacked: [bool; 2],
    pub melee_shield_intact: bool,
    pub attack_ready_at: f32,
}

impl Combatant {
    pub(crate) fn install_weapon_setup_profile(&mut self, source: &Combatant) {
        self.sheet = source.sheet.clone();
        self.magic = source.magic.clone();
        self.tactical_profiles = source.tactical_profiles.clone();
        self.active_style_ids = source.active_style_ids.clone();
        self.active_fight_defensively_penalty = source.active_fight_defensively_penalty;
        self.weapon_group = source.weapon_group.clone();
        self.armor_type = source.armor_type.clone();
        self.state
            .invalidate_weapon_cache(super::WeaponSlot::Primary);
        self.state
            .invalidate_weapon_cache(super::WeaponSlot::Secondary);
    }

    pub(crate) fn finish_weapon_setup(&mut self, now: f32) -> bool {
        let Some(pending) = self.state.weapon_setup.pending.clone() else {
            return false;
        };
        if now < pending.equipment_ready_at
            || self.state.trauma_remaining_seconds > 0
            || self.racial_actions_blocked()
        {
            return false;
        }
        let Some(setups) = self.weapon_setups.clone() else {
            return false;
        };
        let profile = match pending.target {
            WeaponSetup::Melee => &setups.melee,
            WeaponSetup::Ranged => &setups.ranged,
        };
        self.install_weapon_setup_profile(profile);
        self.state.weapon_setup.active = pending.target;
        self.state.weapon_setup.pending = None;
        if pending.target == WeaponSetup::Ranged {
            self.state.weapon_setup.ranged_dropped = false;
        }
        self.state.shield_intact =
            pending.target == WeaponSetup::Melee && self.state.weapon_setup.melee_shield_intact;
        self.state.weapon_setup.attack_ready_at = pending.attack_ready_at;
        self.state.clear_attack_timers();
        self.state.regenstat_stacks = 0;
        self.state.returner_counter_available = false;
        self.state.three_mountains_hit_streak = 0;
        // An opening melee attack is independent of earlier shots from the bow.
        self.state.armeroci_opening_strike_available = pending.target == WeaponSetup::Melee
            && !self.state.weapon_setup.has_attacked[WeaponSetup::Melee.index()]
            && self
                .sheet
                .modifiers
                .apply_i32(0, super::StatIdI32::FlagArmerociPoleStyle)
                > 0;
        for id in &self.active_style_ids {
            if !self.state.activated_style_ids.contains(id) {
                self.state.activated_style_ids.push(id.clone());
            }
        }
        true
    }

    pub(crate) fn reset_starting_weapon_setup(&mut self) {
        let Some(setups) = self.weapon_setups.clone() else {
            return;
        };
        let profile = match setups.start_with {
            WeaponSetup::Melee => &setups.melee,
            WeaponSetup::Ranged => &setups.ranged,
        };
        self.install_weapon_setup_profile(profile);
    }
}
