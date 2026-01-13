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
    pub two_hand_grip: bool,
    pub use_jab: bool,
    pub jab_special_expr: Option<String>,
    pub jab_special_expr_cache: Option<DamageExprCache>,
    pub has_weapon: bool,
    pub defense_bonus_always: bool,
    pub uses_projectiles: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ManeuverProfile {
    pub hold_at_bay: bool,
}

#[derive(Clone, Debug)]
pub struct OffenseProfile {
    pub attack_bonus: i32,
    pub strength_damage: i32,
    pub weapon: WeaponProfile,
}

#[derive(Clone, Debug)]
pub struct DefenseProfile {
    pub defense_mod: i32,
    pub ranged_defense_mod: i32,
    pub armor_dr: i32,
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
}

#[derive(Clone, Debug)]
pub struct CombatantState {
    pub hp: i32,
    pub next_attack_time: Option<f32>,
    pub defense_plus_four_ready: bool,
    pub moved_last_tick: bool,
    pub trauma_remaining_seconds: i32,
    pub knockback_immobile_seconds: i32,
    pub knockback_applied_this_tick: bool,
    pub shield_intact: bool,
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
            two_hand_grip: false,
            use_jab: false,
            jab_special_expr: None,
            jab_special_expr_cache: None,
            has_weapon: false,
            defense_bonus_always: false,
            uses_projectiles: false,
        }
    }
}

impl Default for OffenseProfile {
    fn default() -> Self {
        Self {
            attack_bonus: 0,
            strength_damage: 0,
            weapon: WeaponProfile::default(),
        }
    }
}

impl Default for DefenseProfile {
    fn default() -> Self {
        Self {
            defense_mod: 0,
            ranged_defense_mod: 0,
            armor_dr: 0,
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
        }
    }
}

impl CombatantState {
    pub(crate) fn new(sheet: &CombatantSheet) -> Self {
        Self {
            hp: sheet.vitals.max_hp,
            next_attack_time: None,
            defense_plus_four_ready: false,
            moved_last_tick: false,
            trauma_remaining_seconds: 0,
            knockback_immobile_seconds: 0,
            knockback_applied_this_tick: false,
            shield_intact: sheet.defense.shield_name.is_some(),
        }
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
}

impl Default for Combatant {
    fn default() -> Self {
        Self::new(CombatantSheet::default())
    }
}
use crate::core::rules::DamageExprCache;
