use super::modifiers::{ModifierStack, StatIdF32, StatIdI32, TemporaryEffect};
use crate::core::rules::DamageExprCache;
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
pub struct SimConfig {
    pub start_distance: f32,
    pub stop_distance: f32,
}

impl SimConfig {
    pub fn new(start_distance: f32, stop_distance: f32) -> Self {
        Self {
            start_distance,
            stop_distance,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SimActor {
    pub position: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponSlot {
    Primary,
    Secondary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DamageDie {
    pub sides: i32,
    pub penetrating: bool,
}

#[derive(Clone, Debug, Default)]
pub struct WeaponCache {
    pub max_range: Option<Option<f32>>,
    pub damage_dice: Option<Vec<DamageDie>>,
    pub jab_damage_dice: Option<Vec<DamageDie>>,
}

#[derive(Clone, Debug, Default)]
pub struct CombatantCache {
    pub primary: WeaponCache,
    pub secondary: WeaponCache,
}

#[derive(Clone, Debug)]
pub struct WeaponProfile {
    pub name: String,
    pub damage_expr: String,
    pub damage_expr_cache: DamageExprCache,
    pub shield_damage_expr: Option<String>,
    pub shield_damage_expr_cache: Option<DamageExprCache>,
    pub armor_penetration: i32,
    pub speed: f32,
    pub reach_ft: f32,
    pub range_bands_feet: Option<[f32; 4]>,
    pub range_distance_multiplier: f32,
    pub two_hand_grip: bool,
    pub use_jab: bool,
    pub jab_special_expr: Option<String>,
    pub jab_special_expr_cache: Option<DamageExprCache>,
    pub has_weapon: bool,
    pub defense_bonus_always: bool,
    pub uses_projectiles: bool,
    pub is_small_weapon: bool,
    pub is_unarmed: bool,
    pub crit_min_roll: i32,
    pub crit_min_roll_ranged: Option<i32>,
    pub crit_severity_bonus: i32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ManeuverProfile {
    pub hold_at_bay: bool,
    pub aggressive_attack: bool,
    pub charge: bool,
    pub ready_against_charge: bool,
    pub tactical_move: bool,
    pub fight_defensively: bool,
    pub full_parry: bool,
    pub give_ground: bool,
    pub scamper_back: bool,
    pub fighting_withdrawal: bool,
    pub flee: bool,
    pub defensive_dualwielding: bool,
    pub offensive_dualwielding: bool,
}

#[derive(Clone, Debug)]
pub struct OffenseProfile {
    pub attack_bonus: i32,
    pub attack_bonus_base: i32,
    pub strength_damage: i32,
    pub strength_damage_base: i32,
    pub unarmed_damage_bonus: i32,
    pub weapon: Arc<WeaponProfile>,
    pub offhand: Option<OffhandProfile>,
}

#[derive(Clone, Debug)]
pub struct OffhandProfile {
    pub attack_bonus: i32,
    pub strength_damage: i32,
    pub weapon: Arc<WeaponProfile>,
}

#[derive(Clone, Debug)]
pub struct DefenseProfile {
    pub defense_mod: i32,
    pub ranged_defense_mod: i32,
    pub armor_dr: i32,
    pub natural_dr: i32,
    pub knockback_step: i32,
    pub armor_is_heavy: bool,
    pub shield_name: Option<String>,
    pub shield_defense_bonus: i32,
    pub shield_dr: i32,
    pub shield_cover_value: Option<i32>,
    pub shield_breakage: Option<[ShieldBreakageStep; 4]>,
}

#[derive(Clone, Copy, Debug)]
pub struct MobilityProfile {
    pub move_speed: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Vitals {
    pub max_hp: i32,
    pub constitution: u8,
    pub threshold_of_pain: i32,
    pub trauma_die_sides: i32,
    pub trauma_die_penetrating: bool,
}

#[derive(Clone, Debug)]
pub struct CombatantSheet {
    pub name: String,
    pub offense: OffenseProfile,
    pub defense: DefenseProfile,
    pub mobility: MobilityProfile,
    pub vitals: Vitals,
    pub maneuvers: ManeuverProfile,
    pub modifiers: ModifierStack,
}

#[derive(Clone, Debug)]
pub struct CombatantState {
    pub hp: i32,
    pub next_attack_time_primary: Option<f32>,
    pub next_attack_time_secondary: Option<f32>,
    pub defense_plus_four_ready: bool,
    pub moved_last_tick: bool,
    pub trauma_remaining_seconds: i32,
    pub knockback_immobile_seconds: i32,
    pub knockback_applied_this_tick: bool,
    pub shield_intact: bool,
    pub active_effects: Vec<TemporaryEffect>,
    pub cache: CombatantCache,
}

#[derive(Clone, Debug)]
pub struct Combatant {
    pub sheet: CombatantSheet,
    pub state: CombatantState,
}

#[derive(Clone, Copy, Debug)]
pub struct ShieldBreakageStep {
    pub threshold: i32,
    pub save_mod: Option<i32>,
}

#[derive(Clone, Debug)]
pub struct CombatEvent {
    pub time: u32,
    pub attacker_idx: usize,
    pub defender_idx: usize,
    pub kind: CombatEventKind,
}

#[derive(Clone, Debug)]
pub enum CombatEventKind {
    Attack(AttackEvent),
    KnockAside(KnockAsideEvent),
}

#[derive(Clone, Debug)]
pub struct AttackRollBreakdown {
    pub attack_die: i32,
    pub defense_die: i32,
    pub attack_bonus: i32,
    pub range_mod: i32,
    pub defense_base: i32,
    pub weapon_defense_bonus: i32,
    pub shield_defense_bonus: i32,
    pub attack_total: i32,
    pub defense_total: i32,
}

#[derive(Clone, Debug)]
pub struct DamageBreakdown {
    pub rolled_damage: i32,
    pub strength_damage: i32,
    pub raw_damage: i32,
    pub armor_dr: i32,
    pub armor_penetration: i32,
    pub effective_armor_dr: i32,
    pub final_damage: i32,
}

#[derive(Clone, Debug)]
pub struct ShieldDamageBreakdown {
    pub rolled_damage: i32,
    pub strength_damage: i32,
    pub raw_damage: i32,
    pub shield_dr: i32,
    pub armor_dr: i32,
    pub armor_penetration: i32,
    pub effective_armor_dr: i32,
    pub hp_damage: i32,
    pub shield_broken: bool,
}

#[derive(Clone, Debug)]
pub struct CriticalHit {
    pub severity: i32,
    pub extra_dice: i32,
    pub extra_damage: i32,
    pub speed_reset: bool,
    pub trauma_seconds: Option<i32>,
    pub instant_kill: bool,
}

#[derive(Clone, Debug)]
pub struct AttackEvent {
    pub hit: bool,
    pub shield_block: bool,
    pub damage: i32,
    pub shield_damage: i32,
    pub knockback_ft: f32,
    pub hold_at_bay: bool,
    pub use_jab: bool,
    pub is_ranged: bool,
    pub trauma_applied: bool,
    pub trauma_seconds: Option<i32>,
    pub roll: AttackRollBreakdown,
    pub damage_breakdown: Option<DamageBreakdown>,
    pub shield_damage_breakdown: Option<ShieldDamageBreakdown>,
    pub defender_hp_after: i32,
    pub critical: Option<CriticalHit>,
}

#[derive(Clone, Debug)]
pub struct KnockAsideRollBreakdown {
    pub attack_die: i32,
    pub defense_die: i32,
    pub attack_bonus: i32,
    pub defense_base: i32,
    pub weapon_defense_bonus: i32,
    pub attack_total: i32,
    pub defense_total: i32,
}

#[derive(Clone, Debug)]
pub struct KnockAsideEvent {
    pub success: bool,
    pub roll: KnockAsideRollBreakdown,
}

impl Default for WeaponProfile {
    fn default() -> Self {
        let damage_expr = "d4p".to_string();
        let damage_expr_cache = DamageExprCache::new(&damage_expr);
        Self {
            name: "Weapon".to_string(),
            damage_expr,
            damage_expr_cache,
            shield_damage_expr: None,
            shield_damage_expr_cache: None,
            armor_penetration: 0,
            speed: 10.0,
            reach_ft: 1.0,
            range_bands_feet: None,
            range_distance_multiplier: 1.0,
            two_hand_grip: false,
            use_jab: false,
            jab_special_expr: None,
            jab_special_expr_cache: None,
            has_weapon: false,
            defense_bonus_always: false,
            uses_projectiles: false,
            is_small_weapon: false,
            is_unarmed: false,
            crit_min_roll: 20,
            crit_min_roll_ranged: None,
            crit_severity_bonus: 0,
        }
    }
}

impl Default for OffenseProfile {
    fn default() -> Self {
        Self {
            attack_bonus: 0,
            attack_bonus_base: 0,
            strength_damage: 0,
            strength_damage_base: 0,
            unarmed_damage_bonus: 0,
            weapon: Arc::new(WeaponProfile::default()),
            offhand: None,
        }
    }
}

impl Default for DefenseProfile {
    fn default() -> Self {
        Self {
            defense_mod: 0,
            ranged_defense_mod: 0,
            armor_dr: 0,
            natural_dr: 0,
            knockback_step: 15,
            armor_is_heavy: false,
            shield_name: None,
            shield_defense_bonus: 0,
            shield_dr: 0,
            shield_cover_value: None,
            shield_breakage: None,
        }
    }
}

impl Default for MobilityProfile {
    fn default() -> Self {
        Self { move_speed: 5.0 }
    }
}

impl Default for Vitals {
    fn default() -> Self {
        Self {
            max_hp: 10,
            constitution: 10,
            threshold_of_pain: 3,
            trauma_die_sides: 20,
            trauma_die_penetrating: false,
        }
    }
}

impl Default for CombatantSheet {
    fn default() -> Self {
        Self {
            name: "Combatant".to_string(),
            offense: OffenseProfile::default(),
            defense: DefenseProfile::default(),
            mobility: MobilityProfile::default(),
            vitals: Vitals::default(),
            maneuvers: ManeuverProfile::default(),
            modifiers: ModifierStack::default(),
        }
    }
}

impl CombatantState {
    pub(crate) fn new(sheet: &CombatantSheet) -> Self {
        let mut state = Self {
            hp: sheet.vitals.max_hp,
            next_attack_time_primary: None,
            next_attack_time_secondary: None,
            defense_plus_four_ready: false,
            moved_last_tick: false,
            trauma_remaining_seconds: 0,
            knockback_immobile_seconds: 0,
            knockback_applied_this_tick: false,
            shield_intact: sheet.defense.shield_name.is_some(),
            active_effects: Vec::new(),
            cache: CombatantCache::default(),
        };
        state.refresh_defense_plus_four_ready(sheet, 0.0);
        state
    }

    pub(crate) fn refresh_defense_plus_four_ready(&mut self, sheet: &CombatantSheet, now: f32) {
        let ready = defense_plus_four_ready_at(sheet, self, now);
        self.defense_plus_four_ready = ready;
    }

    pub fn add_effect(&mut self, effect: TemporaryEffect) {
        self.active_effects.push(effect);
    }

    pub fn tick_effects(&mut self) {
        for effect in &mut self.active_effects {
            effect.remaining_seconds -= 1;
        }
        self.active_effects
            .retain(|effect| effect.remaining_seconds > 0);
    }

    pub(crate) fn weapon_cache_mut(&mut self, slot: WeaponSlot) -> &mut WeaponCache {
        match slot {
            WeaponSlot::Primary => &mut self.cache.primary,
            WeaponSlot::Secondary => &mut self.cache.secondary,
        }
    }

    pub fn invalidate_weapon_cache(&mut self, slot: WeaponSlot) {
        match slot {
            WeaponSlot::Primary => self.cache.primary = WeaponCache::default(),
            WeaponSlot::Secondary => self.cache.secondary = WeaponCache::default(),
        }
    }

    pub fn invalidate_range_cache(&mut self, slot: WeaponSlot) {
        let cache = self.weapon_cache_mut(slot);
        cache.max_range = None;
    }

    pub fn invalidate_damage_dice_cache(&mut self, slot: WeaponSlot) {
        let cache = self.weapon_cache_mut(slot);
        cache.damage_dice = None;
        cache.jab_damage_dice = None;
    }

    pub(crate) fn set_next_attack_time(&mut self, slot: WeaponSlot, time: Option<f32>) {
        match slot {
            WeaponSlot::Primary => self.next_attack_time_primary = time,
            WeaponSlot::Secondary => self.next_attack_time_secondary = time,
        }
    }

    pub(crate) fn clear_attack_timers(&mut self) {
        self.next_attack_time_primary = None;
        self.next_attack_time_secondary = None;
    }
}

fn defense_plus_four_eligible(sheet: &CombatantSheet) -> bool {
    let weapon = &sheet.offense.weapon;
    (weapon.two_hand_grip || sheet.maneuvers.defensive_dualwielding)
        && weapon.has_weapon
        && !weapon.defense_bonus_always
}

pub(crate) fn defense_plus_four_ready_at(
    sheet: &CombatantSheet,
    state: &CombatantState,
    now: f32,
) -> bool {
    if !defense_plus_four_eligible(sheet) {
        return false;
    }
    if state.trauma_remaining_seconds > 0 {
        return false;
    }
    match state.next_attack_time_primary {
        Some(next_attack) => now + 0.0001 >= next_attack,
        None => true,
    }
}

impl Combatant {
    pub fn new(sheet: CombatantSheet) -> Self {
        let state = CombatantState::new(&sheet);
        Self { sheet, state }
    }

    pub(crate) fn reset_state(&mut self) {
        self.state = CombatantState::new(&self.sheet);
    }

    pub(crate) fn apply_i32(&self, stat: StatIdI32, base: i32) -> i32 {
        let mut value = self.sheet.modifiers.apply_i32(base, stat);
        for effect in &self.state.active_effects {
            value = effect.modifiers.apply_i32(value, stat);
        }
        value
    }

    pub(crate) fn apply_f32(&self, stat: StatIdF32, base: f32) -> f32 {
        let mut value = self.sheet.modifiers.apply_f32(base, stat);
        for effect in &self.state.active_effects {
            value = effect.modifiers.apply_f32(value, stat);
        }
        value
    }
}

impl Default for Combatant {
    fn default() -> Self {
        Self::new(CombatantSheet::default())
    }
}
