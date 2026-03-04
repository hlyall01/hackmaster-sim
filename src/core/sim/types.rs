use super::modifiers::{ModifierStack, StatIdF32, StatIdI32, TemporaryEffect};
use crate::core::rules::DamageExprCache;
use std::sync::Arc;

const DEFAULT_GRID_HEIGHT: i32 = 7;
const DEFAULT_GRID_PADDING: i32 = 4;
const DEFAULT_TILE_SIZE_FT: f32 = 1.0;

#[derive(Clone, Copy, Debug)]
pub struct SimConfig {
    pub start_distance: f32,
    pub stop_distance: f32,
    pub grid_width: i32,
    pub grid_height: i32,
    pub tile_size_ft: f32,
}

impl SimConfig {
    pub fn new(start_distance: f32, stop_distance: f32) -> Self {
        let start_tiles = (start_distance / DEFAULT_TILE_SIZE_FT).ceil() as i32;
        let grid_width = (start_tiles + DEFAULT_GRID_PADDING * 2 + 1).max(10);
        Self {
            start_distance,
            stop_distance,
            grid_width,
            grid_height: DEFAULT_GRID_HEIGHT,
            tile_size_ft: DEFAULT_TILE_SIZE_FT,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
}

impl GridPos {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn manhattan_distance(self, other: GridPos) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    pub fn clamp(self, width: i32, height: i32) -> Self {
        Self {
            x: self.x.clamp(0, width.saturating_sub(1)),
            y: self.y.clamp(0, height.saturating_sub(1)),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SimActor {
    pub position: GridPos,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalledShotDelayProfile {
    Standard,
    PrecisionCombatant,
    PrecisionAiming,
}

impl Default for CalledShotDelayProfile {
    fn default() -> Self {
        Self::Standard
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ManeuverProfile {
    pub hold_at_bay: bool,
    pub called_shot: bool,
    pub called_shot_defense_bonus: i32,
    pub called_shot_defense_penalty: i32,
    pub called_shot_delay_profile: CalledShotDelayProfile,
    pub called_shot_deceptive_defender: bool,
    pub called_shot_target_defense_bonus_base: i32,
    pub aggressive_attack: bool,
    pub charge: bool,
    pub ready_against_charge: bool,
    pub tactical_move: bool,
    pub fight_defensively: bool,
    pub fight_defensively_attack_penalty: i32,
    pub fight_defensively_defense_bonus: i32,
    pub full_parry: bool,
    pub give_ground: bool,
    pub scamper_back: bool,
    pub fighting_withdrawal: bool,
    pub flee: bool,
    pub mounted: bool,
    pub defensive_dualwielding: bool,
    pub offensive_dualwielding: bool,
    pub offensive_dualwielding_defense_penalty: bool,
    pub dualwield_offhand_damage_penalty: i32,
    pub dualwield_primary_recovery_penalty: f32,
    pub dualwield_secondary_recovery_penalty: f32,
}

impl Default for ManeuverProfile {
    fn default() -> Self {
        Self {
            hold_at_bay: false,
            called_shot: false,
            called_shot_defense_bonus: 8,
            called_shot_defense_penalty: 4,
            called_shot_delay_profile: CalledShotDelayProfile::Standard,
            called_shot_deceptive_defender: false,
            called_shot_target_defense_bonus_base: 8,
            aggressive_attack: false,
            charge: false,
            ready_against_charge: false,
            tactical_move: false,
            fight_defensively: false,
            fight_defensively_attack_penalty: 0,
            fight_defensively_defense_bonus: 0,
            full_parry: false,
            give_ground: false,
            scamper_back: false,
            fighting_withdrawal: false,
            flee: false,
            mounted: false,
            defensive_dualwielding: false,
            offensive_dualwielding: false,
            offensive_dualwielding_defense_penalty: false,
            dualwield_offhand_damage_penalty: -2,
            dualwield_primary_recovery_penalty: 2.0,
            dualwield_secondary_recovery_penalty: 2.0,
        }
    }
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
    pub dex_defense_bonus: i32,
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
    pub charge_distance_ft: f32,
    pub charge_target_idx: Option<usize>,
    pub charge_attacks: u32,
    pub saw_trauma: bool,
    pub max_knockback_ft: f32,
    pub max_crit_hit_dealt: i32,
    pub max_noncrit_hit_dealt: i32,
    pub max_shield_hit_dealt: i32,
    pub total_hp_damage_dealt: u32,
    pub total_hp_damage_taken: u32,
    pub total_shield_damage_dealt: u32,
    pub total_shield_damage_taken: u32,
    pub shield_blocks_taken: u32,
    pub total_shield_breaks_taken: u32,
    pub total_shield_hits_survived_before_break: u32,
    pub total_instakills_dealt: u32,
    pub total_knockback_inflicted_ft: f32,
    pub total_knockback_taken_ft: f32,
    pub charge_started_within_20ft: bool,
    pub charge_threshold_started_within_20ft: bool,
    pub trauma_remaining_seconds: i32,
    pub knockback_immobile_seconds: i32,
    pub knockback_applied_this_tick: bool,
    pub shield_intact: bool,
    pub armeroci_opening_strike_available: bool,
    pub regenstat_stacks: i32,
    pub returner_counter_available: bool,
    pub returner_skip_opening_attack: bool,
    pub returner_double_counter_ready: bool,
    pub three_mountains_hit_streak: i32,
    pub force_trauma_roll_20: bool,
    pub deceptive_defender_seen_attackers: Vec<usize>,
    pub active_effects: Vec<TemporaryEffect>,
    pub cache: CombatantCache,
}

#[derive(Clone, Debug)]
pub struct Combatant {
    pub sheet: CombatantSheet,
    pub state: CombatantState,
    pub team_id: u8,
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
    pub is_charge: bool,
    pub weapon_slot: WeaponSlot,
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
            dex_defense_bonus: 0,
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
        let returner_style = sheet.modifiers.apply_i32(0, StatIdI32::FlagReturnerStyle) > 0;
        let armeroci_style = sheet
            .modifiers
            .apply_i32(0, StatIdI32::FlagArmerociPoleStyle)
            > 0;
        let mut state = Self {
            hp: sheet.vitals.max_hp,
            next_attack_time_primary: None,
            next_attack_time_secondary: None,
            defense_plus_four_ready: false,
            moved_last_tick: false,
            charge_distance_ft: 0.0,
            charge_target_idx: None,
            charge_attacks: 0,
            saw_trauma: false,
            max_knockback_ft: 0.0,
            max_crit_hit_dealt: 0,
            max_noncrit_hit_dealt: 0,
            max_shield_hit_dealt: 0,
            total_hp_damage_dealt: 0,
            total_hp_damage_taken: 0,
            total_shield_damage_dealt: 0,
            total_shield_damage_taken: 0,
            shield_blocks_taken: 0,
            total_shield_breaks_taken: 0,
            total_shield_hits_survived_before_break: 0,
            total_instakills_dealt: 0,
            total_knockback_inflicted_ft: 0.0,
            total_knockback_taken_ft: 0.0,
            charge_started_within_20ft: false,
            charge_threshold_started_within_20ft: false,
            trauma_remaining_seconds: 0,
            knockback_immobile_seconds: 0,
            knockback_applied_this_tick: false,
            shield_intact: sheet.defense.shield_name.is_some(),
            armeroci_opening_strike_available: armeroci_style,
            regenstat_stacks: 0,
            returner_counter_available: returner_style,
            returner_skip_opening_attack: returner_style,
            returner_double_counter_ready: false,
            three_mountains_hit_streak: 0,
            force_trauma_roll_20: false,
            deceptive_defender_seen_attackers: Vec::new(),
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
        Self::new_with_team(sheet, 0)
    }

    pub fn new_with_team(sheet: CombatantSheet, team_id: u8) -> Self {
        let state = CombatantState::new(&sheet);
        Self {
            sheet,
            state,
            team_id,
        }
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
