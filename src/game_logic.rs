use crate::character::{
    AbilityScore, AbilitySet, Armor, ArmorType, Character, DerivedStats, Equipment, Progression,
    Shield, Weapon, WeaponGroup, WeaponMastery,
};
use crate::core::catalog::Catalog;
pub use crate::core::ids::{
    ArmorId, ArmorTag, FighterPresetId, FighterPresetTag, NpcPresetId, NpcPresetTag, ShieldId,
    ShieldTag, TalentId, TalentTag, WeaponId, WeaponTag,
};
use crate::core::rules::DamageExprCache;
use crate::core::types::{
    AbilityKind, RaceSpec, TalentEffect, TalentRequirement, TalentSelection, TalentSpec,
};
use crate::sim::{
    self, Combatant, CombatantSheet, DefenseProfile, MobilityProfile, OffenseProfile, Vitals,
    WeaponProfile,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub type WeaponCatalog = Catalog<WeaponTag, WeaponPreset>;
pub type ArmorCatalog = Catalog<ArmorTag, ArmorEntry>;
pub type ShieldCatalog = Catalog<ShieldTag, ShieldEntry>;
pub type NpcPresetCatalog = Catalog<NpcPresetTag, NpcPreset>;
pub type FighterPresetCatalog = Catalog<FighterPresetTag, FighterPreset>;
pub type TalentCatalog = Catalog<TalentTag, TalentSpec>;

#[derive(Clone)]
pub struct WeaponPreset {
    pub name: String,
    pub price_gp: u32,
    pub group: WeaponGroup,
    pub speed: f32,
    pub speed_label: String,
    pub jab_speed: Option<f32>,
    pub jab_speed_label: Option<String>,
    pub jab_special_expr: Option<String>,
    pub damage_expr: String,
    pub shield_damage_expr: Option<String>,
    pub reach_label: String,
    pub reach_ft: f32,
    pub range_bands_feet: Option<[f32; 4]>,
    pub armor_pen: i32,
    pub hacking_or_piercing: bool,
    pub defense_bonus_always: bool,
    pub size: WeaponSize,
    pub handedness: WeaponHandedness,
    pub ammunition: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponSize {
    Small,
    Medium,
    Large,
}

impl WeaponSize {
    pub fn min_speed(self) -> f32 {
        match self {
            WeaponSize::Small => 2.0,
            WeaponSize::Medium => 3.0,
            WeaponSize::Large => 4.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponHandedness {
    OneHanded,
    TwoHanded,
}

pub const TWO_HANDED_DAMAGE_BONUS: i32 = 3;
pub const TWO_HANDED_SPEED_PENALTY: f32 = 2.0;
pub const TALENT_CATEGORY_WEAPON_STYLES: &str = "Weapon Styles";
const TALENT_ID_TWELVE_PATHS: &str = "twelve_paths";
const TALENT_ID_ARMEROCI_POLE: &str = "armeroci_pole";
const TALENT_ID_CRESCENT_MOON: &str = "crescent_moon";
const TALENT_ID_DOOMRAZOR: &str = "doomrazor";
const TALENT_ID_FALLING_SUN: &str = "falling_sun";
const TALENT_ID_FYMBLWNGER: &str = "fymblwnger";
const TALENT_ID_HAMMERER: &str = "hammerer";
const TALENT_ID_HOBBLER: &str = "hobbler";
const TALENT_ID_ITHICAN_PRINCE: &str = "ithican_prince";
const TALENT_ID_KANIAN_IMPALER: &str = "kanian_impaler";
const TALENT_ID_QUIET_RIVER: &str = "quiet_river";
const TALENT_ID_REGENSTAT: &str = "regenstat";
const TALENT_ID_RETURNER: &str = "returner";
const TALENT_ID_RHDWNG_FLOW: &str = "rhdwng_flow";
const TALENT_ID_ROHAVALAN_BRIDGE: &str = "rohavalan_bridge";
const TALENT_ID_SCORN_OF_THE_DISSENDRI: &str = "scorn_of_the_dissendri";
const TALENT_ID_SHIELD_OF_BLADES: &str = "shield_of_blades";
const TALENT_ID_SIX_PATHS: &str = "six_paths";
const TALENT_ID_STORM_OF_BLADES: &str = "storm_of_blades";
const TALENT_ID_THREE_MOUNTAINS: &str = "three_mountains";
const TALENT_ID_UNBREAKABLE_WALL: &str = "unbreakable_wall";
const TALENT_ID_DUELIST: &str = "duelist";
const TALENT_ID_CONTENDER: &str = "contender";
#[cfg(test)]
const TALENT_ID_TWO_WEAPON_FIGHTING: &str = "two_weapon_fighting";
#[cfg(test)]
const TALENT_ID_IMPROVED_TWO_WEAPON_FIGHTING: &str = "improved_two_weapon_fighting";
#[cfg(test)]
const TALENT_ID_GREATER_TWO_WEAPON_FIGHTING: &str = "greater_two_weapon_fighting";
const TALENT_ID_PERFECT_TWO_WEAPON_FIGHTING: &str = "perfect_two_weapon_fighting";
#[cfg(test)]
const TALENT_ID_CURSE_OF_AXE: &str = "curse_of_axe";
#[cfg(test)]
const CURSE_OF_AXE_WEAPON_NAME: &str = "Greataxe";
const TALENT_ID_DECEPTIVE_DEFENDER: &str = "deceptive_defender";
const TALENT_ID_POWER_ATTACK: &str = "power_attack";
const TALENT_ID_PRECISION_AIMING: &str = "precision_aiming";
const TALENT_ID_PRECISION_COMBATANT: &str = "precision_combatant";
#[cfg(test)]
const CURSE_OF_AXE_D6_TRIGGERS: &[i32] = &[4, 5, 6];
const TWELVE_PATHS_DAMAGE_PENALTY: i32 = 3;
const ARMEROCI_POLE_REACH_BONUS_FT: f32 = 1.0;
const ARMEROCI_POLE_SPEED_PENALTY: f32 = 2.0;
const HOBBLER_ATTACK_PENALTY: i32 = 4;
const RETURNER_DEFENSE_PENALTY: i32 = 4;
pub const FIGHT_DEFENSIVELY_PENALTY_OPTIONS: [i32; 4] = [2, 4, 6, 8];
const DEFAULT_FIGHT_DEFENSIVELY_PENALTY: i32 = FIGHT_DEFENSIVELY_PENALTY_OPTIONS[0];
pub const CALLED_SHOT_DECEPTIVE_DEFENDER_DELAY_EXPR: &str = "4d4p";
pub const CALLED_SHOT_TARGET_DEFENSE_BONUS_LIGHT: i32 = 4;
pub const CALLED_SHOT_TARGET_DEFENSE_BONUS_MEDIUM: i32 = 8;
pub const CALLED_SHOT_TARGET_DEFENSE_BONUS_HEAVY: i32 = 16;

pub fn weapon_allows_two_handed_mode(weapon: &WeaponPreset) -> bool {
    weapon.handedness == WeaponHandedness::OneHanded
        && matches!(weapon.size, WeaponSize::Medium | WeaponSize::Large)
}

fn effective_two_hand_grip(weapon: &WeaponPreset, two_hand_grip: bool) -> bool {
    weapon.handedness == WeaponHandedness::TwoHanded
        || (two_hand_grip && weapon_allows_two_handed_mode(weapon))
}

fn two_hand_damage_bonus(weapon: &WeaponPreset, two_hand_grip: bool) -> i32 {
    if effective_two_hand_grip(weapon, two_hand_grip) && weapon_allows_two_handed_mode(weapon) {
        TWO_HANDED_DAMAGE_BONUS
    } else {
        0
    }
}

fn two_hand_speed_penalty(weapon: &WeaponPreset, two_hand_grip: bool) -> f32 {
    if effective_two_hand_grip(weapon, two_hand_grip) && weapon_allows_two_handed_mode(weapon) {
        TWO_HANDED_SPEED_PENALTY
    } else {
        0.0
    }
}

pub fn normalize_fight_defensively_penalty(penalty: i32) -> i32 {
    if penalty <= FIGHT_DEFENSIVELY_PENALTY_OPTIONS[0] {
        FIGHT_DEFENSIVELY_PENALTY_OPTIONS[0]
    } else if penalty <= FIGHT_DEFENSIVELY_PENALTY_OPTIONS[1] {
        FIGHT_DEFENSIVELY_PENALTY_OPTIONS[1]
    } else if penalty <= FIGHT_DEFENSIVELY_PENALTY_OPTIONS[2] {
        FIGHT_DEFENSIVELY_PENALTY_OPTIONS[2]
    } else {
        FIGHT_DEFENSIVELY_PENALTY_OPTIONS[3]
    }
}

fn default_fight_defensively_penalty() -> i32 {
    DEFAULT_FIGHT_DEFENSIVELY_PENALTY
}

fn is_default_fight_defensively_penalty(value: &i32) -> bool {
    *value == DEFAULT_FIGHT_DEFENSIVELY_PENALTY
}

#[derive(Clone)]
pub struct ArmorEntry {
    pub label: String,
    pub armor: Option<Armor>,
}

#[derive(Clone)]
pub struct ShieldPreset {
    pub name: String,
    pub price_gp: u32,
    pub defense_bonus: i32,
    pub dr: i32,
    pub cover_value: i32,
    pub breakage_thresholds: [i32; 4],
    pub weight_lbs: f32,
}

#[derive(Clone)]
pub struct ShieldEntry {
    pub label: String,
    pub shield: Option<ShieldPreset>,
}

#[derive(Clone, Deserialize)]
pub struct NpcPreset {
    pub name: String,
    pub hp: i32,
    pub attack_bonus: i32,
    pub damage_bonus: i32,
    pub defense_mod: i32,
    pub armor_dr: i32,
    pub top: i32,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct FighterProgression {
    pub attack: String,
    pub speed: String,
    pub initiative: String,
    pub health: String,
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct FighterMasteries {
    #[serde(default)]
    pub attack: i32,
    #[serde(default)]
    pub defense: i32,
    #[serde(default)]
    pub damage: i32,
    #[serde(default)]
    pub speed: i32,
    #[serde(default)]
    pub shield_defense: i32,
    #[serde(default)]
    pub shield_speed: i32,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CombatManeuverConfig {
    #[serde(default, skip_serializing_if = "is_false")]
    pub use_jab: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub hold_at_bay: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub called_shot: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub power_attack: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub aggressive_attack: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub charge: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub ready_against_charge: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub tactical_move: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub fight_defensively: bool,
    #[serde(
        default = "default_fight_defensively_penalty",
        skip_serializing_if = "is_default_fight_defensively_penalty"
    )]
    pub fight_defensively_penalty: i32,
    #[serde(default, skip_serializing_if = "is_false")]
    pub full_parry: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub give_ground: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub scamper_back: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub fighting_withdrawal: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub flee: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub mounted: bool,
}

impl Default for CombatManeuverConfig {
    fn default() -> Self {
        Self {
            use_jab: false,
            hold_at_bay: false,
            called_shot: false,
            power_attack: false,
            aggressive_attack: false,
            charge: false,
            ready_against_charge: false,
            tactical_move: false,
            fight_defensively: false,
            fight_defensively_penalty: DEFAULT_FIGHT_DEFENSIVELY_PENALTY,
            full_parry: false,
            give_ground: false,
            scamper_back: false,
            fighting_withdrawal: false,
            flee: false,
            mounted: false,
        }
    }
}

impl CombatManeuverConfig {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Clone, Deserialize, Serialize)]
pub struct FighterPreset {
    pub name: String,
    pub level: u8,
    pub progression: FighterProgression,
    #[serde(default)]
    pub masteries: FighterMasteries,
    pub base_hp: u32,
    pub move_speed: f32,
    pub strength_base: u8,
    pub strength_pct: u8,
    pub dex_base: u8,
    pub dex_pct: u8,
    pub intelligence: u8,
    pub wisdom: u8,
    pub constitution: u8,
    pub looks: u8,
    pub charisma: u8,
    pub weapon: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offhand_weapon: Option<String>,
    pub armor: String,
    pub shield: String,
    pub weapon_material_tier: i32,
    #[serde(default)]
    pub offhand_weapon_material_tier: i32,
    pub armor_material_tier: i32,
    pub projectile_material_tier: i32,
    #[serde(default)]
    pub offhand_projectile_material_tier: i32,
    pub shield_material_tier: i32,
    pub two_hand_grip: bool,
    #[serde(default, skip_serializing_if = "CombatManeuverConfig::is_default")]
    pub maneuvers: CombatManeuverConfig,
    #[serde(default)]
    pub defensive_dualwielding: bool,
    #[serde(default)]
    pub offensive_dualwielding: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub race_id: Option<String>,
    #[serde(default)]
    pub proficiencies: Vec<String>,
    #[serde(default)]
    pub talents: Vec<TalentSelection>,
}

#[derive(Clone, Copy, Debug)]
pub struct EnvironmentConfig {
    pub temperature_c: i32,
    pub natural_surroundings: bool,
    pub bright_light: bool,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            temperature_c: 21,
            natural_surroundings: false,
            bright_light: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MiscRollModifiers {
    pub all_roll_bonus: i32,
    pub attack_bonus: i32,
    pub defense_bonus: i32,
    pub damage_bonus: i32,
    pub initiative_bonus: i32,
    pub speed_mod_bonus: i32,
    pub armor_dr_bonus: i32,
    pub hp_bonus: i32,
    pub initiative_die_bonus: i32,
}

#[derive(Clone)]
pub struct PlayerConfig {
    pub name: String,
    pub level: u8,
    pub progression: Progression,
    pub base_hp: u32,
    pub move_speed: f32,
    pub strength_base: u8,
    pub strength_pct: u8,
    pub dex_base: u8,
    pub dex_pct: u8,
    pub intelligence: u8,
    pub wisdom: u8,
    pub constitution: u8,
    pub looks: u8,
    pub charisma: u8,
    pub weapon_id: WeaponId,
    pub offhand_weapon_id: Option<WeaponId>,
    pub armor_id: ArmorId,
    pub weapon_material_tier: i32,
    pub offhand_weapon_material_tier: i32,
    pub armor_material_tier: i32,
    pub projectile_material_tier: i32,
    pub offhand_projectile_material_tier: i32,
    pub shield_id: ShieldId,
    pub shield_material_tier: i32,
    pub npc_preset: Option<NpcPresetId>,
    pub fighter_preset: Option<FighterPresetId>,
    pub mastery_attack: i32,
    pub mastery_defense: i32,
    pub mastery_damage: i32,
    pub mastery_speed: i32,
    pub shield_mastery_defense: i32,
    pub shield_mastery_speed: i32,
    pub two_hand_grip: bool,
    pub use_jab: bool,
    pub hold_at_bay: bool,
    pub called_shot: bool,
    pub power_attack: bool,
    pub aggressive_attack: bool,
    pub charge: bool,
    pub ready_against_charge: bool,
    pub tactical_move: bool,
    pub fight_defensively: bool,
    pub fight_defensively_penalty: i32,
    pub full_parry: bool,
    pub give_ground: bool,
    pub scamper_back: bool,
    pub fighting_withdrawal: bool,
    pub flee: bool,
    pub mounted: bool,
    pub defensive_dualwielding: bool,
    pub offensive_dualwielding: bool,
    pub environment: EnvironmentConfig,
    pub misc_modifiers: MiscRollModifiers,
    pub knockback_step: i32,
    pub race_id: Option<String>,
    pub race_applied: bool,
    pub proficiencies: Vec<String>,
    pub talents: Vec<TalentSelection>,
}

impl PlayerConfig {
    pub fn new(name: &str, weapon_id: WeaponId) -> Self {
        Self {
            name: name.to_string(),
            level: 1,
            progression: Progression::default(),
            base_hp: 10,
            move_speed: 20.0,
            strength_base: 10,
            strength_pct: 1,
            dex_base: 10,
            dex_pct: 1,
            intelligence: 10,
            wisdom: 10,
            constitution: 10,
            looks: 10,
            charisma: 10,
            weapon_id,
            offhand_weapon_id: None,
            armor_id: ArmorId::new(0),
            weapon_material_tier: 0,
            offhand_weapon_material_tier: 0,
            armor_material_tier: 0,
            projectile_material_tier: 0,
            offhand_projectile_material_tier: 0,
            shield_id: ShieldId::new(0),
            shield_material_tier: 0,
            npc_preset: None,
            fighter_preset: None,
            mastery_attack: 0,
            mastery_defense: 0,
            mastery_damage: 0,
            mastery_speed: 0,
            shield_mastery_defense: 0,
            shield_mastery_speed: 0,
            two_hand_grip: false,
            use_jab: false,
            hold_at_bay: false,
            called_shot: false,
            power_attack: false,
            aggressive_attack: false,
            charge: false,
            ready_against_charge: false,
            tactical_move: false,
            fight_defensively: false,
            fight_defensively_penalty: DEFAULT_FIGHT_DEFENSIVELY_PENALTY,
            full_parry: false,
            give_ground: false,
            scamper_back: false,
            fighting_withdrawal: false,
            flee: false,
            mounted: false,
            defensive_dualwielding: false,
            offensive_dualwielding: false,
            environment: EnvironmentConfig::default(),
            misc_modifiers: MiscRollModifiers::default(),
            knockback_step: DEFAULT_KNOCKBACK_STEP,
            race_id: None,
            race_applied: false,
            proficiencies: Vec::new(),
            talents: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct TraumaDieOverride {
    sides: i32,
    penetrating: bool,
}

#[derive(Clone, Debug)]
struct CloseHitDamageRule {
    expr: String,
    margin_less_than: i32,
}

#[derive(Clone, Debug)]
struct TalentModifiers {
    hp_bonus: i32,
    drain_resistance: i32,
    armor_dr_bonus: i32,
    initiative_die_bonus: i32,
    speed_mod_bonus: i32,
    initiative_mod_bonus: i32,
    defense_bonus: i32,
    defense_bonus_by_weapon: HashMap<WeaponId, i32>,
    damage_bonus_by_group: HashMap<WeaponGroup, i32>,
    allow_dex_ranged: bool,
    trauma_die_override: Option<TraumaDieOverride>,
    attack_bonus_by_weapon: HashMap<WeaponId, i32>,
    damage_bonus_by_weapon: HashMap<WeaponId, i32>,
    weapon_speed_bonus_by_weapon: HashMap<WeaponId, i32>,
    weapon_speed_flat_bonus_by_weapon: HashMap<WeaponId, f32>,
    weapon_speed_multiplier_by_weapon: HashMap<WeaponId, f32>,
    weapon_min_speed_multiplier_by_weapon: HashMap<WeaponId, f32>,
    weapon_speed_round_up_by_weapon: HashSet<WeaponId>,
    weapon_reach_flat_bonus_by_weapon: HashMap<WeaponId, f32>,
    no_strength_damage_by_weapon: HashSet<WeaponId>,
    no_mastery_damage_by_weapon: HashSet<WeaponId>,
    force_nonpenetrating_damage_by_weapon: HashSet<WeaponId>,
    halve_damage_by_weapon: HashSet<WeaponId>,
    ignore_all_dr_by_weapon: HashSet<WeaponId>,
    internal_hemorrhage_damage_by_weapon: HashMap<WeaponId, i32>,
    expanded_attack_defense_penetration_by_weapon: HashSet<WeaponId>,
    expanded_damage_penetration_by_weapon: HashSet<WeaponId>,
    opening_engagement_extra_damage_dice_by_weapon: HashMap<WeaponId, i32>,
    always_initial_engagement_by_weapon: HashSet<WeaponId>,
    ignore_movement_defense_bonus_by_weapon: HashSet<WeaponId>,
    knockback_resets_weapon_count_by_weapon: HashSet<WeaponId>,
    hit_critical_effects_no_extra_dice_by_weapon: HashSet<WeaponId>,
    thrown_full_strength_damage_by_weapon: HashSet<WeaponId>,
    consecutive_hits_force_trauma_twenty_by_weapon: HashMap<WeaponId, i32>,
    shield_defense_bonus: i32,
    shield_cover_value_adjustment: i32,
    shield_dr_bonus_filtered: i32,
    shield_dr_bonus_by_name: HashMap<String, i32>,
    shield_breakage_uses_shield_dr: bool,
    shield_breakage_uses_shield_dr_by_name: HashSet<String>,
    ignore_armor_initiative_penalty: bool,
    ignore_armor_speed_penalty: bool,
    armor_dr_bonus_armored: i32,
    light_armor_defense_divisor: Option<i32>,
    medium_armor_dr_bonus: i32,
    medium_armor_defense_penalty_reduction: i32,
    heavy_armor_damage_bonus_divisor: Option<i32>,
    heavy_armor_damage_bonus_flat: i32,
    reach_bonus_by_group: HashMap<WeaponGroup, i32>,
    reach_multiplier_by_weapon: HashMap<WeaponId, f32>,
    close_hit_damage_by_weapon: HashMap<WeaponId, CloseHitDamageRule>,
    range_distance_multiplier: f32,
    threshold_of_pain_bonus_pct: f32,
    threshold_of_pain_level_bonus: f32,
    crit_min_by_group: HashMap<WeaponGroup, i32>,
    crit_min_ranged_by_group: HashMap<WeaponGroup, i32>,
    crit_severity_bonus_by_group: HashMap<WeaponGroup, i32>,
    light_armor_crit_extra_damage_halved: bool,
    medium_armor_crit_severity_reduction: i32,
    heavy_armor_ignore_ancillary_crit_effects: bool,
    knockback_step_bumps: i32,
    defiant: bool,
    superior_defense: bool,
    edge_counter: bool,
    fight_defensively_attack_penalty_divisor: i32,
    called_shot_delay_profile: Option<sim::CalledShotDelayProfile>,
    called_shot_target_defense_bonus_divisor: i32,
    called_shot_self_defense_penalty: Option<i32>,
    called_shot_deceptive_defender: bool,
    dualwield_offhand_damage_penalty: Option<i32>,
    dualwield_primary_recovery_penalty: Option<f32>,
    dualwield_secondary_recovery_penalty: Option<f32>,
    perfect_two_weapon_fighting: bool,
    large_sword_shield_style: bool,
    armeroci_pole_style: bool,
    crescent_moon_style: bool,
    doomrazor_style: bool,
    falling_sun_style: bool,
    fymblwnger_style: bool,
    hammerer_style: bool,
    hobbler_style: bool,
    ithican_prince_style: bool,
    kanian_impaler_style: bool,
    quiet_river_style: bool,
    regenstat_style: bool,
    returner_style: bool,
    rhdwng_flow_style: bool,
    scorn_of_the_dissendri_style: bool,
    shield_of_blades_style: bool,
    six_paths_style: bool,
    storm_of_blades_style: bool,
    three_mountains_style: bool,
    unbreakable_wall_style: bool,
    forced_weapon_loadout: Option<ForcedWeaponLoadout>,
}

#[derive(Clone, Debug)]
struct ForcedWeaponLoadout {
    weapon_name: String,
    min_weapon_material_tier: Option<i32>,
    clear_projectile_material: bool,
    disable_offhand: bool,
    force_two_hand_grip: bool,
    force_no_shield: bool,
    d6_penetration_triggers: Vec<i32>,
}

impl Default for TalentModifiers {
    fn default() -> Self {
        Self {
            hp_bonus: 0,
            drain_resistance: 0,
            armor_dr_bonus: 0,
            initiative_die_bonus: 0,
            speed_mod_bonus: 0,
            initiative_mod_bonus: 0,
            defense_bonus: 0,
            defense_bonus_by_weapon: HashMap::new(),
            damage_bonus_by_group: HashMap::new(),
            allow_dex_ranged: false,
            trauma_die_override: None,
            attack_bonus_by_weapon: HashMap::new(),
            damage_bonus_by_weapon: HashMap::new(),
            weapon_speed_bonus_by_weapon: HashMap::new(),
            weapon_speed_flat_bonus_by_weapon: HashMap::new(),
            weapon_speed_multiplier_by_weapon: HashMap::new(),
            weapon_min_speed_multiplier_by_weapon: HashMap::new(),
            weapon_speed_round_up_by_weapon: HashSet::new(),
            weapon_reach_flat_bonus_by_weapon: HashMap::new(),
            no_strength_damage_by_weapon: HashSet::new(),
            no_mastery_damage_by_weapon: HashSet::new(),
            force_nonpenetrating_damage_by_weapon: HashSet::new(),
            halve_damage_by_weapon: HashSet::new(),
            ignore_all_dr_by_weapon: HashSet::new(),
            internal_hemorrhage_damage_by_weapon: HashMap::new(),
            expanded_attack_defense_penetration_by_weapon: HashSet::new(),
            expanded_damage_penetration_by_weapon: HashSet::new(),
            opening_engagement_extra_damage_dice_by_weapon: HashMap::new(),
            always_initial_engagement_by_weapon: HashSet::new(),
            ignore_movement_defense_bonus_by_weapon: HashSet::new(),
            knockback_resets_weapon_count_by_weapon: HashSet::new(),
            hit_critical_effects_no_extra_dice_by_weapon: HashSet::new(),
            thrown_full_strength_damage_by_weapon: HashSet::new(),
            consecutive_hits_force_trauma_twenty_by_weapon: HashMap::new(),
            shield_defense_bonus: 0,
            shield_cover_value_adjustment: 0,
            shield_dr_bonus_filtered: 0,
            shield_dr_bonus_by_name: HashMap::new(),
            shield_breakage_uses_shield_dr: false,
            shield_breakage_uses_shield_dr_by_name: HashSet::new(),
            ignore_armor_initiative_penalty: false,
            ignore_armor_speed_penalty: false,
            armor_dr_bonus_armored: 0,
            light_armor_defense_divisor: None,
            medium_armor_dr_bonus: 0,
            medium_armor_defense_penalty_reduction: 0,
            heavy_armor_damage_bonus_divisor: None,
            heavy_armor_damage_bonus_flat: 0,
            reach_bonus_by_group: HashMap::new(),
            reach_multiplier_by_weapon: HashMap::new(),
            close_hit_damage_by_weapon: HashMap::new(),
            range_distance_multiplier: 1.0,
            threshold_of_pain_bonus_pct: 0.0,
            threshold_of_pain_level_bonus: 0.0,
            crit_min_by_group: HashMap::new(),
            crit_min_ranged_by_group: HashMap::new(),
            crit_severity_bonus_by_group: HashMap::new(),
            light_armor_crit_extra_damage_halved: false,
            medium_armor_crit_severity_reduction: 0,
            heavy_armor_ignore_ancillary_crit_effects: false,
            knockback_step_bumps: 0,
            defiant: false,
            superior_defense: false,
            edge_counter: false,
            fight_defensively_attack_penalty_divisor: 1,
            called_shot_delay_profile: None,
            called_shot_target_defense_bonus_divisor: 1,
            called_shot_self_defense_penalty: None,
            called_shot_deceptive_defender: false,
            dualwield_offhand_damage_penalty: None,
            dualwield_primary_recovery_penalty: None,
            dualwield_secondary_recovery_penalty: None,
            perfect_two_weapon_fighting: false,
            large_sword_shield_style: false,
            armeroci_pole_style: false,
            crescent_moon_style: false,
            doomrazor_style: false,
            falling_sun_style: false,
            fymblwnger_style: false,
            hammerer_style: false,
            hobbler_style: false,
            ithican_prince_style: false,
            kanian_impaler_style: false,
            quiet_river_style: false,
            regenstat_style: false,
            returner_style: false,
            rhdwng_flow_style: false,
            scorn_of_the_dissendri_style: false,
            shield_of_blades_style: false,
            six_paths_style: false,
            storm_of_blades_style: false,
            three_mountains_style: false,
            unbreakable_wall_style: false,
            forced_weapon_loadout: None,
        }
    }
}

impl TalentModifiers {
    fn attack_bonus_for_weapon(&self, weapon_id: WeaponId) -> i32 {
        *self.attack_bonus_by_weapon.get(&weapon_id).unwrap_or(&0)
    }

    fn damage_bonus_for_weapon(&self, weapon_id: WeaponId) -> i32 {
        *self.damage_bonus_by_weapon.get(&weapon_id).unwrap_or(&0)
    }

    fn damage_bonus_for_group(&self, group: WeaponGroup) -> i32 {
        *self.damage_bonus_by_group.get(&group).unwrap_or(&0)
    }

    fn defense_bonus_for_weapon(&self, weapon_id: WeaponId) -> i32 {
        *self.defense_bonus_by_weapon.get(&weapon_id).unwrap_or(&0)
    }

    fn weapon_speed_bonus_for_weapon(&self, weapon_id: WeaponId) -> i32 {
        *self
            .weapon_speed_bonus_by_weapon
            .get(&weapon_id)
            .unwrap_or(&0)
    }

    fn weapon_speed_flat_bonus_for_weapon(&self, weapon_id: WeaponId) -> f32 {
        *self
            .weapon_speed_flat_bonus_by_weapon
            .get(&weapon_id)
            .unwrap_or(&0.0)
    }

    fn weapon_speed_multiplier_for_weapon(&self, weapon_id: WeaponId) -> f32 {
        *self
            .weapon_speed_multiplier_by_weapon
            .get(&weapon_id)
            .unwrap_or(&1.0)
    }

    fn weapon_min_speed_multiplier_for_weapon(&self, weapon_id: WeaponId) -> f32 {
        *self
            .weapon_min_speed_multiplier_by_weapon
            .get(&weapon_id)
            .unwrap_or(&1.0)
    }

    fn weapon_speed_rounds_up_for_weapon(&self, weapon_id: WeaponId) -> bool {
        self.weapon_speed_round_up_by_weapon.contains(&weapon_id)
    }

    fn weapon_reach_flat_bonus_for_weapon(&self, weapon_id: WeaponId) -> f32 {
        *self
            .weapon_reach_flat_bonus_by_weapon
            .get(&weapon_id)
            .unwrap_or(&0.0)
    }

    fn reach_bonus_for_group(&self, group: WeaponGroup) -> i32 {
        *self.reach_bonus_by_group.get(&group).unwrap_or(&0)
    }

    fn reach_multiplier_for_weapon(&self, weapon_id: WeaponId) -> f32 {
        *self
            .reach_multiplier_by_weapon
            .get(&weapon_id)
            .unwrap_or(&1.0)
    }

    fn close_hit_damage_for_weapon(&self, weapon_id: WeaponId) -> Option<&CloseHitDamageRule> {
        self.close_hit_damage_by_weapon.get(&weapon_id)
    }

    fn crit_min_for_group(&self, group: WeaponGroup) -> i32 {
        self.crit_min_by_group
            .get(&group)
            .copied()
            .unwrap_or(20)
            .clamp(2, 20)
    }

    fn crit_min_ranged_for_group(&self, group: WeaponGroup) -> Option<i32> {
        self.crit_min_ranged_by_group.get(&group).copied()
    }

    fn crit_severity_bonus_for_group(&self, group: WeaponGroup) -> i32 {
        *self.crit_severity_bonus_by_group.get(&group).unwrap_or(&0)
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ArmorTalentAdjustments {
    speed_mod_bonus: i32,
    initiative_mod_bonus: i32,
    base_dv_bonus: i32,
    armor_dr_bonus: i32,
    heavy_armor_damage_bonus: i32,
}

fn armor_talent_adjustments(
    armor: Option<&Armor>,
    modifiers: &TalentModifiers,
) -> ArmorTalentAdjustments {
    let mut adjustments = ArmorTalentAdjustments::default();
    let Some(armor) = armor else {
        return adjustments;
    };
    let mut armor_dr = armor.damage_reduction + modifiers.armor_dr_bonus_armored;
    if matches!(armor.armor_type, ArmorType::Medium) {
        armor_dr += modifiers.medium_armor_dr_bonus;
    }
    let armor_dr = armor_dr.max(0);
    if modifiers.ignore_armor_speed_penalty && armor.speed_mod != 0 {
        adjustments.speed_mod_bonus -= armor.speed_mod;
    }
    if modifiers.ignore_armor_initiative_penalty && armor.initiative_mod < 0 {
        adjustments.initiative_mod_bonus -= armor.initiative_mod;
    }
    if modifiers.armor_dr_bonus_armored != 0 {
        adjustments.armor_dr_bonus += modifiers.armor_dr_bonus_armored;
    }
    match armor.armor_type {
        ArmorType::Light => {
            if let Some(divisor) = modifiers.light_armor_defense_divisor {
                if divisor > 0 {
                    adjustments.base_dv_bonus += armor_dr / divisor;
                }
            }
        }
        ArmorType::Medium => {
            if modifiers.medium_armor_dr_bonus != 0 {
                adjustments.armor_dr_bonus += modifiers.medium_armor_dr_bonus;
            }
            if modifiers.medium_armor_defense_penalty_reduction > 0 && armor.defense_adj < 0 {
                let reduction = modifiers
                    .medium_armor_defense_penalty_reduction
                    .min(-armor.defense_adj);
                adjustments.base_dv_bonus += reduction;
            }
        }
        ArmorType::Heavy => {
            if let Some(divisor) = modifiers.heavy_armor_damage_bonus_divisor {
                if divisor > 0 {
                    adjustments.heavy_armor_damage_bonus += armor_dr / divisor;
                }
            }
            if modifiers.heavy_armor_damage_bonus_flat != 0 {
                adjustments.heavy_armor_damage_bonus += modifiers.heavy_armor_damage_bonus_flat;
            }
        }
        ArmorType::None => {}
    }
    adjustments
}

fn talent_rank(selection: &TalentSelection) -> i32 {
    if selection.rank == 0 {
        1
    } else {
        selection.rank as i32
    }
}

fn find_talent<'a>(catalog: &'a TalentCatalog, id: &str) -> Option<&'a TalentSpec> {
    catalog.entries().iter().find(|talent| talent.id == id)
}

pub fn is_weapon_style_category(category: &str) -> bool {
    category
        .trim()
        .eq_ignore_ascii_case(TALENT_CATEGORY_WEAPON_STYLES)
}

pub fn has_other_weapon_style_selected(
    player: &PlayerConfig,
    spec: &TalentSpec,
    talent_catalog: &TalentCatalog,
) -> bool {
    if !is_weapon_style_category(&spec.category) {
        return false;
    }
    player.talents.iter().any(|selection| {
        selection.id != spec.id
            && selection.rank.max(1) > 0
            && find_talent(talent_catalog, &selection.id)
                .map(|other| {
                    is_weapon_style_category(&other.category)
                        && !weapon_styles_can_stack(player, &selection.id, &spec.id)
                })
                .unwrap_or(false)
    })
}

fn is_storm_shield_pair(left: &str, right: &str) -> bool {
    matches!(
        (left, right),
        (TALENT_ID_STORM_OF_BLADES, TALENT_ID_SHIELD_OF_BLADES)
            | (TALENT_ID_SHIELD_OF_BLADES, TALENT_ID_STORM_OF_BLADES)
    )
}

fn weapon_styles_can_stack(player: &PlayerConfig, left: &str, right: &str) -> bool {
    has_perfect_two_weapon_fighting_effect(player) && is_storm_shield_pair(left, right)
}

pub fn talent_requires_weapon(spec: &TalentSpec) -> bool {
    spec.effects.iter().any(|effect| match effect {
        TalentEffect::AttackBonusWeapon { .. }
        | TalentEffect::DamageBonusWeapon { .. }
        | TalentEffect::DefenseBonusWeapon { .. }
        | TalentEffect::WeaponReachBonus { .. } => true,
        TalentEffect::WeaponSpeedBonus { weapon_group, .. } => weapon_group.is_none(),
        _ => false,
    })
}

pub fn talent_requires_weapon_group(spec: &TalentSpec) -> bool {
    matches!(
        spec.id.as_str(),
        "weapon_focus"
            | "weapon_specialization"
            | "weapon_supremacy"
            | "ranged_weapon_specialization"
            | "ranged_weapon_supremacy"
            | "improved_critical"
            | "critical_mastery"
            | "wounding_criticals"
            | "ranged_critical_mastery"
    )
}

pub fn talent_is_implemented(spec: &TalentSpec) -> bool {
    if !spec.effects.is_empty() {
        return true;
    }
    matches!(
        spec.id.as_str(),
        "improved_critical"
            | "critical_mastery"
            | "wounding_criticals"
            | "ranged_critical_mastery"
            | "two_weapon_fighting"
            | "improved_two_weapon_fighting"
            | "greater_two_weapon_fighting"
            | "perfect_two_weapon_fighting"
            | "power_attack"
            | "stout"
            | "sturdy"
            | "defiant"
            | "superior_defense"
            | "edge_counter"
    )
}

const DATA_ONLY_TALENT_IDS: &[&str] = &[
    "weapon_focus",
    "weapon_specialization",
    "weapon_supremacy",
    "ranged_weapon_specialization",
    "ranged_weapon_supremacy",
];

const SUPPORTED_TACTICAL_TOGGLES: &[&str] = &[
    "use_jab",
    "hold_at_bay",
    "called_shot",
    "power_attack",
    "charge",
    "fight_defensively",
    "mounted",
    "defensive_dualwielding",
    "offensive_dualwielding",
];

const KNOWN_UNSUPPORTED_TACTICAL_TOGGLES: &[&str] = &[
    "aggressive_attack",
    "ready_against_charge",
    "tactical_move",
    "full_parry",
    "give_ground",
    "scamper_back",
    "fighting_withdrawal",
    "flee",
];

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SimCapabilityReport {
    pub schema_version: u32,
    pub generated_from: String,
    pub supported_talent_ids_with_direct_combat_effects: Vec<String>,
    pub supported_tactical_toggles: Vec<String>,
    pub supported_weapon_style_ids: Vec<String>,
    pub known_data_only_talent_ids: Vec<String>,
    pub known_unsupported_tactical_toggles: Vec<String>,
    pub nyi_talent_ids: Vec<String>,
    pub notes: Vec<String>,
}

fn sorted_strings(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut values: Vec<String> = values.into_iter().collect();
    values.sort();
    values.dedup();
    values
}

fn is_known_data_only_talent_id(id: &str) -> bool {
    DATA_ONLY_TALENT_IDS.contains(&id)
}

pub fn sim_capability_report(talent_catalog: &TalentCatalog) -> SimCapabilityReport {
    let supported_weapon_style_ids = sorted_strings(
        talent_catalog
            .entries()
            .iter()
            .filter(|spec| is_weapon_style_category(&spec.category))
            .map(|spec| spec.id.clone()),
    );
    let known_data_only_talent_ids = sorted_strings(
        talent_catalog
            .entries()
            .iter()
            .filter(|spec| is_known_data_only_talent_id(&spec.id))
            .map(|spec| spec.id.clone()),
    );
    let supported_talent_ids_with_direct_combat_effects = sorted_strings(
        talent_catalog
            .entries()
            .iter()
            .filter(|spec| {
                talent_is_implemented(spec)
                    && !is_weapon_style_category(&spec.category)
                    && !is_known_data_only_talent_id(&spec.id)
            })
            .map(|spec| spec.id.clone()),
    );
    let nyi_talent_ids = sorted_strings(
        talent_catalog
            .entries()
            .iter()
            .filter(|spec| {
                !talent_is_implemented(spec)
                    && !is_weapon_style_category(&spec.category)
                    && !is_known_data_only_talent_id(&spec.id)
            })
            .map(|spec| spec.id.clone()),
    );

    SimCapabilityReport {
        schema_version: 1,
        generated_from: "data/sim/talents.json".to_string(),
        supported_talent_ids_with_direct_combat_effects,
        supported_tactical_toggles: SUPPORTED_TACTICAL_TOGGLES
            .iter()
            .map(|value| value.to_string())
            .collect(),
        supported_weapon_style_ids,
        known_data_only_talent_ids,
        known_unsupported_tactical_toggles: KNOWN_UNSUPPORTED_TACTICAL_TOGGLES
            .iter()
            .map(|value| value.to_string())
            .collect(),
        nyi_talent_ids,
        notes: vec![
            "Power Attack is modeled as an explicit tactical toggle requiring the talent, STR 13+, and an eligible non-small melee weapon.".to_string(),
            "Weapon Focus, Weapon Specialization, and Weapon Supremacy are data-only for fixed-mastery duel output and should feed mastery/progression planning instead.".to_string(),
            "Weapon style IDs are reported separately because styles are trained/acquired options rather than ordinary BP purchases.".to_string(),
        ],
    }
}

#[derive(Clone)]
pub struct TalentContext<'a> {
    pub level: u8,
    pub stats: &'a AbilitySet,
    pub talents: &'a [TalentSelection],
    pub proficiencies: &'a [String],
    pub weapon_catalog: Option<&'a WeaponCatalog>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TalentRequirementFailure {
    MinLevel {
        required: u8,
        current: u8,
    },
    MinStatBase {
        stat: AbilityKind,
        required: u8,
        current: u8,
    },
    MinStatPercentile {
        stat: AbilityKind,
        required: u8,
        current: Option<u8>,
    },
    RequiresTalent {
        id: String,
        required_rank: u8,
        current_rank: u8,
    },
    MissingSizeLLargeSwordProficiency,
    MissingShieldProficiency,
    MissingArmerociPoleProficiency,
    MissingCrescentMoonProficiency,
    MissingDoomrazorProficiency,
    MissingFallingSunProficiency,
    MissingFymblwngerProficiency,
    MissingHammererProficiency,
    MissingHobblerProficiency,
    MissingIthicanPrinceProficiency,
    MissingQuietRiverProficiency,
    MissingRegenstatProficiency,
    MissingReturnerProficiency,
    MissingRhdwngFlowProficiency,
    MissingRohavalanBridgeProficiency,
    MissingScornOfTheDissendriProficiency,
    MissingSwordReachStyleProficiency,
    MissingSixPathsProficiency,
    MissingThreeMountainsProficiency,
    MissingUnbreakableWallProficiency,
}

fn ability_values(stats: &AbilitySet, stat: AbilityKind) -> (u8, Option<u8>) {
    match stat {
        AbilityKind::Strength => (stats.strength.base, Some(stats.strength.percentile)),
        AbilityKind::Dexterity => (stats.dexterity.base, Some(stats.dexterity.percentile)),
        AbilityKind::Intelligence => (stats.intelligence, None),
        AbilityKind::Wisdom => (stats.wisdom, None),
        AbilityKind::Constitution => (stats.constitution, None),
        AbilityKind::Looks => (stats.looks, None),
        AbilityKind::Charisma => (stats.charisma, None),
    }
}

fn selection_weapon_group(selection: &TalentSelection) -> Option<WeaponGroup> {
    selection.weapon.as_deref().and_then(weapon_group_from_str)
}

fn requires_matching_weapon_group(spec: &TalentSpec, required_id: &str) -> bool {
    match spec.id.as_str() {
        "weapon_specialization" => required_id == "weapon_focus",
        "weapon_supremacy" => required_id == "weapon_specialization",
        "ranged_weapon_specialization" => required_id == "weapon_focus",
        "ranged_weapon_supremacy" => required_id == "ranged_weapon_specialization",
        "critical_mastery" => required_id == "improved_critical",
        "wounding_criticals" => required_id == "improved_critical",
        "ranged_critical_mastery" => required_id == "improved_critical",
        _ => false,
    }
}

fn normalize_proficiency_token(value: &str) -> String {
    let lowered = value.to_ascii_lowercase().replace("proficiency", " ");
    let mut out = String::new();
    let mut last_space = false;
    for ch in lowered.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_space = false;
        } else if !last_space {
            out.push(' ');
            last_space = true;
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn has_shield_proficiency(proficiencies: &[String]) -> bool {
    proficiencies.iter().any(|entry| {
        let token = normalize_proficiency_token(entry);
        if token.is_empty() {
            return false;
        }
        let mut has_shield_word = false;
        let mut has_buckler_word = false;
        for word in token.split_whitespace() {
            if word == "shield" || word == "shields" {
                has_shield_word = true;
            }
            if word == "buckler" || word == "bucklers" {
                has_buckler_word = true;
            }
        }
        has_shield_word || has_buckler_word
    })
}

fn has_size_l_large_sword_proficiency(context: &TalentContext<'_>) -> bool {
    let mut normalized_proficiencies: Vec<String> = Vec::new();
    for entry in context.proficiencies {
        let token = normalize_proficiency_token(entry);
        if token.is_empty() {
            continue;
        }
        let mut has_large = false;
        let mut has_sword = false;
        for word in token.split_whitespace() {
            if word == "large" {
                has_large = true;
            }
            if word == "sword" || word == "swords" {
                has_sword = true;
            }
        }
        if has_large && has_sword {
            return true;
        }
        normalized_proficiencies.push(token);
    }
    let Some(weapon_catalog) = context.weapon_catalog else {
        return false;
    };
    weapon_catalog
        .entries()
        .iter()
        .filter(|weapon| {
            weapon.group == WeaponGroup::LargeSwords && weapon.size == WeaponSize::Large
        })
        .map(|weapon| normalize_proficiency_token(&weapon.name))
        .any(|weapon_token| {
            normalized_proficiencies
                .iter()
                .any(|entry| entry == &weapon_token)
        })
}

fn normalized_proficiencies(context: &TalentContext<'_>) -> Vec<String> {
    context
        .proficiencies
        .iter()
        .map(|entry| normalize_proficiency_token(entry))
        .filter(|entry| !entry.is_empty())
        .collect()
}

fn has_words(token: &str, words: &[&str]) -> bool {
    let token_words: Vec<&str> = token.split_whitespace().collect();
    words.iter().all(|word| {
        let singular = word.trim_end_matches('s');
        token_words
            .iter()
            .any(|entry| *entry == *word || *entry == singular || *entry == format!("{singular}s"))
    })
}

fn has_weapon_group_proficiency(context: &TalentContext<'_>, group: WeaponGroup) -> bool {
    let normalized = normalized_proficiencies(context);
    if normalized.is_empty() {
        return false;
    }
    let group_words: &[&str] = match group {
        WeaponGroup::SmallSwords => &["small", "sword"],
        WeaponGroup::LargeSwords => &["large", "sword"],
        WeaponGroup::Polearms => &["polearm"],
        WeaponGroup::Spears => &["spear"],
        WeaponGroup::Axes => &["axe"],
        WeaponGroup::Blunt => &["hammer"],
        WeaponGroup::Shields => &["shield"],
        _ => &[],
    };
    if !group_words.is_empty() && normalized.iter().any(|entry| has_words(entry, group_words)) {
        return true;
    }
    let Some(weapon_catalog) = context.weapon_catalog else {
        return false;
    };
    let names: Vec<String> = weapon_catalog
        .entries()
        .iter()
        .filter(|weapon| weapon.group == group)
        .map(|weapon| normalize_proficiency_token(&weapon.name))
        .collect();
    normalized
        .iter()
        .any(|entry| names.iter().any(|name| name == entry))
}

fn has_any_weapon_name_proficiency(context: &TalentContext<'_>, names: &[&str]) -> bool {
    let target: Vec<String> = names
        .iter()
        .map(|name| normalize_proficiency_token(name))
        .collect();
    normalized_proficiencies(context)
        .iter()
        .any(|entry| target.iter().any(|name| name == entry))
}

fn has_weapon_matching<F>(context: &TalentContext<'_>, predicate: F) -> bool
where
    F: Fn(&WeaponPreset) -> bool,
{
    let normalized = normalized_proficiencies(context);
    if normalized.is_empty() {
        return false;
    }
    let Some(weapon_catalog) = context.weapon_catalog else {
        return false;
    };
    weapon_catalog
        .entries()
        .iter()
        .filter(|weapon| predicate(weapon))
        .map(|weapon| normalize_proficiency_token(&weapon.name))
        .any(|weapon_token| normalized.iter().any(|entry| entry == &weapon_token))
}

fn has_size_m_small_or_large_sword_proficiency(context: &TalentContext<'_>) -> bool {
    has_weapon_matching(context, |weapon| {
        weapon.size == WeaponSize::Medium
            && matches!(
                weapon.group,
                WeaponGroup::SmallSwords | WeaponGroup::LargeSwords
            )
    }) || has_weapon_group_proficiency(context, WeaponGroup::SmallSwords)
        || has_weapon_group_proficiency(context, WeaponGroup::LargeSwords)
}

fn has_size_m_large_sword_proficiency(context: &TalentContext<'_>) -> bool {
    has_weapon_matching(context, |weapon| {
        weapon.size == WeaponSize::Medium && weapon.group == WeaponGroup::LargeSwords
    }) || has_weapon_group_proficiency(context, WeaponGroup::LargeSwords)
}

fn has_small_sword_proficiency(context: &TalentContext<'_>) -> bool {
    has_weapon_group_proficiency(context, WeaponGroup::SmallSwords)
}

fn has_crescent_moon_proficiency(context: &TalentContext<'_>) -> bool {
    has_small_sword_proficiency(context) && has_size_m_large_sword_proficiency(context)
}

fn has_doomrazor_proficiency(context: &TalentContext<'_>) -> bool {
    has_weapon_matching(context, |weapon| {
        !is_ranged_weapon(weapon) && weapon.hacking_or_piercing
    })
}

fn has_falling_sun_proficiency(context: &TalentContext<'_>) -> bool {
    has_any_weapon_name_proficiency(context, &["Flamberge", "Two-handed sword"])
}

fn has_armeroci_pole_proficiency(context: &TalentContext<'_>) -> bool {
    has_weapon_matching(context, |weapon| {
        (weapon.group == WeaponGroup::LargeSwords || weapon.group == WeaponGroup::Polearms)
            && weapon.reach_ft >= 5.0
    }) || has_weapon_group_proficiency(context, WeaponGroup::Polearms)
        || has_weapon_group_proficiency(context, WeaponGroup::LargeSwords)
}

fn has_hobbler_proficiency(context: &TalentContext<'_>) -> bool {
    has_weapon_group_proficiency(context, WeaponGroup::Polearms)
        || has_weapon_group_proficiency(context, WeaponGroup::Spears)
}

fn has_quiet_river_proficiency(context: &TalentContext<'_>) -> bool {
    has_any_weapon_name_proficiency(context, &["Fist"])
        || has_weapon_group_proficiency(context, WeaponGroup::Unarmed)
}

fn has_throwing_weapon_proficiency(context: &TalentContext<'_>) -> bool {
    has_weapon_matching(context, |weapon| {
        weapon.range_bands_feet.is_some() && weapon.ammunition.is_none()
    })
}

fn has_rohavalan_bridge_proficiency(context: &TalentContext<'_>) -> bool {
    has_any_weapon_name_proficiency(context, &["Staff"])
        || has_weapon_group_proficiency(context, WeaponGroup::Polearms)
}

fn has_size_s_melee_weapon_proficiency(context: &TalentContext<'_>) -> bool {
    has_weapon_matching(context, |weapon| {
        weapon.size == WeaponSize::Small && !is_ranged_weapon(weapon)
    })
}

fn has_size_s_or_m_sword_reach_proficiency(context: &TalentContext<'_>) -> bool {
    has_weapon_matching(context, |weapon| {
        matches!(
            weapon.group,
            WeaponGroup::SmallSwords | WeaponGroup::LargeSwords
        ) && matches!(weapon.size, WeaponSize::Small | WeaponSize::Medium)
            && weapon.reach_ft >= 2.0
    }) || has_weapon_group_proficiency(context, WeaponGroup::SmallSwords)
        || has_weapon_group_proficiency(context, WeaponGroup::LargeSwords)
}

pub fn evaluate_talent_requirements(
    spec: &TalentSpec,
    context: &TalentContext<'_>,
) -> Vec<TalentRequirementFailure> {
    let mut failures = Vec::new();
    let spec_selection = context
        .talents
        .iter()
        .find(|selection| selection.id == spec.id);
    for requirement in &spec.requirements {
        match requirement {
            TalentRequirement::MinLevel { level } => {
                if context.level < *level {
                    failures.push(TalentRequirementFailure::MinLevel {
                        required: *level,
                        current: context.level,
                    });
                }
            }
            TalentRequirement::MinStat {
                stat,
                min_base,
                min_percentile,
            } => {
                let (current_base, current_percentile) = ability_values(context.stats, *stat);
                if let Some(required) = min_base {
                    if current_base < *required {
                        failures.push(TalentRequirementFailure::MinStatBase {
                            stat: *stat,
                            required: *required,
                            current: current_base,
                        });
                    }
                }
                if let Some(required) = min_percentile {
                    let meets_percentile = current_percentile
                        .map(|current| current >= *required)
                        .unwrap_or(false);
                    if !meets_percentile {
                        failures.push(TalentRequirementFailure::MinStatPercentile {
                            stat: *stat,
                            required: *required,
                            current: current_percentile,
                        });
                    }
                }
            }
            TalentRequirement::RequiresTalent { id, min_rank } => {
                let required_rank = min_rank.unwrap_or(1).max(1);
                let requires_group = requires_matching_weapon_group(spec, id);
                let desired_group = if requires_group {
                    spec_selection.and_then(selection_weapon_group)
                } else {
                    None
                };
                let current_rank = if requires_group && spec_selection.is_none() {
                    context
                        .talents
                        .iter()
                        .filter(|selection| selection.id == *id)
                        .map(|selection| selection.rank.max(1))
                        .max()
                        .unwrap_or(0)
                } else {
                    context
                        .talents
                        .iter()
                        .filter(|selection| selection.id == *id)
                        .filter(|selection| {
                            desired_group
                                .map(|group| selection_weapon_group(selection) == Some(group))
                                .unwrap_or(true)
                        })
                        .map(|selection| selection.rank.max(1))
                        .max()
                        .unwrap_or(0)
                };
                if current_rank < required_rank {
                    failures.push(TalentRequirementFailure::RequiresTalent {
                        id: id.clone(),
                        required_rank,
                        current_rank,
                    });
                }
            }
        }
    }
    match spec.id.as_str() {
        TALENT_ID_TWELVE_PATHS => {
            if !has_size_l_large_sword_proficiency(context) {
                failures.push(TalentRequirementFailure::MissingSizeLLargeSwordProficiency);
            }
            if !has_shield_proficiency(context.proficiencies) {
                failures.push(TalentRequirementFailure::MissingShieldProficiency);
            }
        }
        TALENT_ID_ARMEROCI_POLE => {
            if !has_armeroci_pole_proficiency(context) {
                failures.push(TalentRequirementFailure::MissingArmerociPoleProficiency);
            }
        }
        TALENT_ID_CRESCENT_MOON => {
            if !has_crescent_moon_proficiency(context) {
                failures.push(TalentRequirementFailure::MissingCrescentMoonProficiency);
            }
        }
        TALENT_ID_DOOMRAZOR => {
            if !has_doomrazor_proficiency(context) {
                failures.push(TalentRequirementFailure::MissingDoomrazorProficiency);
            }
        }
        TALENT_ID_FALLING_SUN => {
            if !has_falling_sun_proficiency(context) {
                failures.push(TalentRequirementFailure::MissingFallingSunProficiency);
            }
        }
        TALENT_ID_FYMBLWNGER => {
            if !has_any_weapon_name_proficiency(
                context,
                &["Battle axe", "Executioner's axe", "Greataxe"],
            ) {
                failures.push(TalentRequirementFailure::MissingFymblwngerProficiency);
            }
        }
        TALENT_ID_HAMMERER => {
            if !has_any_weapon_name_proficiency(
                context,
                &["Greathammer", "Hammer", "Maul", "Warhammer"],
            ) {
                failures.push(TalentRequirementFailure::MissingHammererProficiency);
            }
        }
        TALENT_ID_HOBBLER => {
            if !has_hobbler_proficiency(context) {
                failures.push(TalentRequirementFailure::MissingHobblerProficiency);
            }
        }
        TALENT_ID_ITHICAN_PRINCE => {
            if !(has_shield_proficiency(context.proficiencies)
                && has_weapon_group_proficiency(context, WeaponGroup::SmallSwords))
            {
                failures.push(TalentRequirementFailure::MissingIthicanPrinceProficiency);
            }
        }
        TALENT_ID_QUIET_RIVER => {
            if !has_quiet_river_proficiency(context) {
                failures.push(TalentRequirementFailure::MissingQuietRiverProficiency);
            }
        }
        TALENT_ID_REGENSTAT => {
            if !has_size_m_small_or_large_sword_proficiency(context) {
                failures.push(TalentRequirementFailure::MissingRegenstatProficiency);
            }
        }
        TALENT_ID_RETURNER => {
            if !has_size_l_large_sword_proficiency(context) {
                failures.push(TalentRequirementFailure::MissingReturnerProficiency);
            }
        }
        TALENT_ID_RHDWNG_FLOW => {
            if !has_throwing_weapon_proficiency(context) {
                failures.push(TalentRequirementFailure::MissingRhdwngFlowProficiency);
            }
        }
        TALENT_ID_ROHAVALAN_BRIDGE => {
            if !has_rohavalan_bridge_proficiency(context) {
                failures.push(TalentRequirementFailure::MissingRohavalanBridgeProficiency);
            }
        }
        TALENT_ID_SCORN_OF_THE_DISSENDRI => {
            if !has_size_s_melee_weapon_proficiency(context) {
                failures.push(TalentRequirementFailure::MissingScornOfTheDissendriProficiency);
            }
        }
        TALENT_ID_SHIELD_OF_BLADES | TALENT_ID_STORM_OF_BLADES => {
            if !has_size_s_or_m_sword_reach_proficiency(context) {
                failures.push(TalentRequirementFailure::MissingSwordReachStyleProficiency);
            }
        }
        TALENT_ID_SIX_PATHS => {
            if !(has_size_m_large_sword_proficiency(context)
                && has_shield_proficiency(context.proficiencies))
            {
                failures.push(TalentRequirementFailure::MissingSixPathsProficiency);
            }
        }
        TALENT_ID_THREE_MOUNTAINS => {
            if !has_weapon_group_proficiency(context, WeaponGroup::Blunt) {
                failures.push(TalentRequirementFailure::MissingThreeMountainsProficiency);
            }
        }
        TALENT_ID_UNBREAKABLE_WALL => {
            if !has_shield_proficiency(context.proficiencies) {
                failures.push(TalentRequirementFailure::MissingUnbreakableWallProficiency);
            }
        }
        _ => {}
    }
    failures
}

fn weapon_group_from_str(value: &str) -> Option<WeaponGroup> {
    match value.trim().to_ascii_lowercase().as_str() {
        "unarmed" => Some(WeaponGroup::Unarmed),
        "axes" => Some(WeaponGroup::Axes),
        "basic" => Some(WeaponGroup::Basic),
        "blunt" => Some(WeaponGroup::Blunt),
        "bows" => Some(WeaponGroup::Bows),
        "crossbows" => Some(WeaponGroup::Crossbows),
        "double" => Some(WeaponGroup::Double),
        "ensnaring" => Some(WeaponGroup::Ensnaring),
        "lashes" => Some(WeaponGroup::Lashes),
        "large_swords" | "large swords" => Some(WeaponGroup::LargeSwords),
        "small_swords" | "small swords" => Some(WeaponGroup::SmallSwords),
        "polearms" => Some(WeaponGroup::Polearms),
        "spears" => Some(WeaponGroup::Spears),
        "shields" => Some(WeaponGroup::Shields),
        _ => None,
    }
}

fn weapon_matches_effect_filter(
    weapon: &WeaponPreset,
    weapon_groups: &[String],
    weapon_names: &[String],
) -> bool {
    let group_matches = weapon_groups
        .iter()
        .filter_map(|group| weapon_group_from_str(group))
        .any(|group| weapon.group == group);
    let name_matches = weapon_names
        .iter()
        .any(|name| weapon.name.trim().eq_ignore_ascii_case(name.trim()));
    if weapon_groups.is_empty() && weapon_names.is_empty() {
        true
    } else {
        group_matches || name_matches
    }
}

fn weapon_matches_effect_filter_with_min_reach(
    weapon: &WeaponPreset,
    weapon_groups: &[String],
    weapon_names: &[String],
    min_reach_ft: Option<f32>,
) -> bool {
    weapon_matches_effect_filter(weapon, weapon_groups, weapon_names)
        && min_reach_ft
            .map(|min_reach_ft| weapon.reach_ft >= min_reach_ft)
            .unwrap_or(true)
}

fn filtered_weapon_ids<'a>(
    weapon_catalog: &'a WeaponCatalog,
    weapon_groups: &'a [String],
    weapon_names: &'a [String],
) -> impl Iterator<Item = WeaponId> + 'a {
    weapon_catalog
        .entries()
        .iter()
        .enumerate()
        .filter(move |(_, weapon)| {
            weapon_matches_effect_filter(weapon, weapon_groups, weapon_names)
        })
        .filter_map(|(idx, _)| weapon_catalog.id_from_index(idx))
}

fn filtered_weapon_ids_with_min_reach<'a>(
    weapon_catalog: &'a WeaponCatalog,
    weapon_groups: &'a [String],
    weapon_names: &'a [String],
    min_reach_ft: Option<f32>,
) -> impl Iterator<Item = WeaponId> + 'a {
    weapon_catalog
        .entries()
        .iter()
        .enumerate()
        .filter(move |(_, weapon)| {
            weapon_matches_effect_filter_with_min_reach(
                weapon,
                weapon_groups,
                weapon_names,
                min_reach_ft,
            )
        })
        .filter_map(|(idx, _)| weapon_catalog.id_from_index(idx))
}

fn shield_filter_key(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

fn called_shot_delay_profile_from_str(value: &str) -> Option<sim::CalledShotDelayProfile> {
    match value.trim().to_ascii_lowercase().as_str() {
        "standard" => Some(sim::CalledShotDelayProfile::Standard),
        "precision_combatant" | "precision combatant" => {
            Some(sim::CalledShotDelayProfile::PrecisionCombatant)
        }
        "precision_aiming" | "precision aiming" => {
            Some(sim::CalledShotDelayProfile::PrecisionAiming)
        }
        _ => None,
    }
}

fn talent_effects_active(spec: &TalentSpec, player: &PlayerConfig) -> bool {
    match spec.id.as_str() {
        "natural_attunement" | "natural_protection" | "natural_awareness" => {
            player.environment.natural_surroundings
        }
        _ => true,
    }
}

fn is_proficiency_requirement_failure(failure: &TalentRequirementFailure) -> bool {
    matches!(
        failure,
        TalentRequirementFailure::MissingSizeLLargeSwordProficiency
            | TalentRequirementFailure::MissingShieldProficiency
            | TalentRequirementFailure::MissingArmerociPoleProficiency
            | TalentRequirementFailure::MissingCrescentMoonProficiency
            | TalentRequirementFailure::MissingDoomrazorProficiency
            | TalentRequirementFailure::MissingFallingSunProficiency
            | TalentRequirementFailure::MissingFymblwngerProficiency
            | TalentRequirementFailure::MissingHammererProficiency
            | TalentRequirementFailure::MissingHobblerProficiency
            | TalentRequirementFailure::MissingIthicanPrinceProficiency
            | TalentRequirementFailure::MissingQuietRiverProficiency
            | TalentRequirementFailure::MissingRegenstatProficiency
            | TalentRequirementFailure::MissingReturnerProficiency
            | TalentRequirementFailure::MissingRhdwngFlowProficiency
            | TalentRequirementFailure::MissingRohavalanBridgeProficiency
            | TalentRequirementFailure::MissingScornOfTheDissendriProficiency
            | TalentRequirementFailure::MissingSwordReachStyleProficiency
            | TalentRequirementFailure::MissingSixPathsProficiency
            | TalentRequirementFailure::MissingThreeMountainsProficiency
            | TalentRequirementFailure::MissingUnbreakableWallProficiency
    )
}

fn style_effects_active(
    selection: &TalentSelection,
    spec: &TalentSpec,
    context: &TalentContext<'_>,
    player: &PlayerConfig,
) -> bool {
    let failures = evaluate_talent_requirements(spec, context);
    let blocked = failures
        .iter()
        .any(|failure| !is_proficiency_requirement_failure(failure));
    !blocked && talent_effects_active(spec, player) && selection.rank.max(1) > 0
}

fn active_weapon_style_specs<'a>(
    player: &PlayerConfig,
    talent_catalog: &'a TalentCatalog,
    weapon_catalog: Option<&WeaponCatalog>,
) -> Vec<&'a TalentSpec> {
    let stats = ability_set_from_player(player);
    let context = TalentContext {
        level: player.level,
        stats: &stats,
        talents: &player.talents,
        proficiencies: &player.proficiencies,
        weapon_catalog,
    };
    let mut active_styles: Vec<&TalentSpec> = Vec::new();
    for selection in &player.talents {
        let Some(spec) = find_talent(talent_catalog, &selection.id) else {
            continue;
        };
        if !is_weapon_style_category(&spec.category) {
            continue;
        }
        if style_effects_active(selection, spec, &context, player) {
            if active_styles.is_empty()
                || active_styles
                    .iter()
                    .any(|active| weapon_styles_can_stack(player, &active.id, &spec.id))
            {
                active_styles.push(spec);
            }
        }
    }
    active_styles
}

fn has_large_sword_shield_style_effect(spec: &TalentSpec) -> bool {
    spec.effects
        .iter()
        .any(|effect| matches!(effect, TalentEffect::LargeSwordShieldStyle))
}

fn is_small_shield_or_buckler_name(name: &str) -> bool {
    let label = name.trim().to_ascii_lowercase();
    label == "buckler" || (label.starts_with("small ") && label.contains("shield"))
}

fn is_medium_or_small_shield_name(name: &str) -> bool {
    let label = name.trim().to_ascii_lowercase();
    label == "buckler"
        || (label.starts_with("small ") && label.contains("shield"))
        || (label.starts_with("medium ") && label.contains("shield"))
}

fn is_large_or_tower_shield_name(name: &str) -> bool {
    let label = name.trim().to_ascii_lowercase();
    (label.starts_with("large ") && label.contains("shield")) || label.contains("tower shield")
}

fn armeroci_pole_style_active(modifiers: &TalentModifiers, weapon: &WeaponPreset) -> bool {
    modifiers.armeroci_pole_style
        && matches!(
            weapon.group,
            WeaponGroup::LargeSwords | WeaponGroup::Polearms
        )
        && weapon.reach_ft >= 5.0
}

fn essence_advancement_stats(level: u8) -> (i32, i32) {
    match level {
        1..=4 => (1, 0),
        5..=7 => (3, 2),
        8..=10 => (4, 4),
        11..=13 => (6, 6),
        14..=16 => (7, 8),
        _ => (9, 10),
    }
}

fn falling_sun_style_active(modifiers: &TalentModifiers, weapon: &WeaponPreset) -> bool {
    modifiers.falling_sun_style
        && matches!(
            weapon.name.trim().to_ascii_lowercase().as_str(),
            "flamberge" | "two-handed sword"
        )
}

fn doomrazor_style_active(modifiers: &TalentModifiers, weapon: &WeaponPreset) -> bool {
    modifiers.doomrazor_style && !is_ranged_weapon(weapon) && weapon.hacking_or_piercing
}

fn fymblwnger_style_active(modifiers: &TalentModifiers, weapon: &WeaponPreset) -> bool {
    modifiers.fymblwnger_style
        && matches!(
            weapon.name.trim().to_ascii_lowercase().as_str(),
            "battle axe" | "executioner's axe" | "greataxe"
        )
}

fn hammerer_style_active(modifiers: &TalentModifiers, weapon: &WeaponPreset) -> bool {
    modifiers.hammerer_style
        && matches!(
            weapon.name.trim().to_ascii_lowercase().as_str(),
            "greathammer" | "hammer" | "maul" | "warhammer"
        )
}

fn hobbler_style_active(modifiers: &TalentModifiers, weapon: &WeaponPreset) -> bool {
    modifiers.hobbler_style && matches!(weapon.group, WeaponGroup::Polearms | WeaponGroup::Spears)
}

fn is_one_handed_sword(weapon: &WeaponPreset) -> bool {
    weapon.handedness == WeaponHandedness::OneHanded
        && matches!(
            weapon.group,
            WeaponGroup::SmallSwords | WeaponGroup::LargeSwords
        )
}

fn shield_of_blades_style_active(
    modifiers: &TalentModifiers,
    player: &PlayerConfig,
    primary_weapon: &WeaponPreset,
    weapon_catalog: &WeaponCatalog,
    defensive_dualwielding: bool,
) -> bool {
    modifiers.shield_of_blades_style
        && defensive_dualwielding
        && is_one_handed_sword(primary_weapon)
        && player
            .offhand_weapon_id
            .and_then(|id| weapon_catalog.get(id))
            .map(is_one_handed_sword)
            .unwrap_or(false)
}

fn quiet_river_style_active(
    modifiers: &TalentModifiers,
    weapon: &WeaponPreset,
    armor_type: ArmorType,
    shield: Option<&Shield>,
) -> bool {
    modifiers.quiet_river_style
        && weapon.name.trim().eq_ignore_ascii_case("fist")
        && matches!(armor_type, ArmorType::None)
        && shield.is_none()
}

fn rhdwng_flow_style_active(modifiers: &TalentModifiers, weapon: &WeaponPreset) -> bool {
    modifiers.rhdwng_flow_style && weapon.range_bands_feet.is_some() && weapon.ammunition.is_none()
}

fn ithican_prince_style_active(
    modifiers: &TalentModifiers,
    weapon: &WeaponPreset,
    shield: Option<&Shield>,
) -> bool {
    modifiers.ithican_prince_style
        && weapon.group == WeaponGroup::SmallSwords
        && shield
            .map(|entry| entry.name.trim().eq_ignore_ascii_case("buckler"))
            .unwrap_or(false)
}

fn regenstat_style_active(
    modifiers: &TalentModifiers,
    weapon: &WeaponPreset,
    two_hand_grip: bool,
    offhand_weapon_id: Option<WeaponId>,
    shield: Option<&Shield>,
) -> bool {
    if !modifiers.regenstat_style {
        return false;
    }
    if !matches!(
        weapon.group,
        WeaponGroup::SmallSwords | WeaponGroup::LargeSwords
    ) || weapon.size != WeaponSize::Medium
    {
        return false;
    }
    if weapon.handedness == WeaponHandedness::TwoHanded {
        return true;
    }
    two_hand_grip || (offhand_weapon_id.is_none() && shield.is_none())
}

fn returner_style_active(modifiers: &TalentModifiers, weapon: &WeaponPreset) -> bool {
    modifiers.returner_style
        && weapon.group == WeaponGroup::LargeSwords
        && weapon.size == WeaponSize::Large
}

fn six_paths_style_active(
    modifiers: &TalentModifiers,
    weapon: &WeaponPreset,
    shield: Option<&Shield>,
) -> bool {
    modifiers.six_paths_style
        && weapon.group == WeaponGroup::LargeSwords
        && weapon.size == WeaponSize::Medium
        && shield
            .map(|entry| is_medium_or_small_shield_name(&entry.name))
            .unwrap_or(false)
}

fn three_mountains_style_active(modifiers: &TalentModifiers, weapon: &WeaponPreset) -> bool {
    modifiers.three_mountains_style && weapon.group == WeaponGroup::Blunt
}

fn unbreakable_wall_style_active(modifiers: &TalentModifiers, shield: Option<&Shield>) -> bool {
    modifiers.unbreakable_wall_style
        && shield
            .map(|entry| is_large_or_tower_shield_name(&entry.name))
            .unwrap_or(false)
}

fn twelve_paths_style_active(
    modifiers: &TalentModifiers,
    weapon: &WeaponPreset,
    shield: Option<&Shield>,
) -> bool {
    modifiers.large_sword_shield_style
        && weapon.group == WeaponGroup::LargeSwords
        && weapon.size == WeaponSize::Large
        && shield
            .map(|entry| is_small_shield_or_buckler_name(&entry.name))
            .unwrap_or(false)
}

fn resolve_talent_modifiers(
    player: &PlayerConfig,
    talent_catalog: &TalentCatalog,
    weapon_catalog: &WeaponCatalog,
) -> TalentModifiers {
    let mut modifiers = TalentModifiers::default();
    modifiers.perfect_two_weapon_fighting = has_perfect_two_weapon_fighting_effect(player);
    let stats = ability_set_from_player(player);
    let context = TalentContext {
        level: player.level,
        stats: &stats,
        talents: &player.talents,
        proficiencies: &player.proficiencies,
        weapon_catalog: Some(weapon_catalog),
    };
    let mut weapon_id_lookup: HashMap<String, WeaponId> = HashMap::new();
    for (idx, weapon) in weapon_catalog.entries().iter().enumerate() {
        if let Some(id) = weapon_catalog.id_from_index(idx) {
            weapon_id_lookup.insert(weapon.name.to_ascii_lowercase(), id);
        }
    }
    let weapon_id_by_name_cached =
        |name: &str| weapon_id_lookup.get(&name.to_ascii_lowercase()).copied();
    let active_weapon_style_ids: HashSet<&str> =
        active_weapon_style_specs(player, talent_catalog, Some(weapon_catalog))
            .into_iter()
            .map(|spec| spec.id.as_str())
            .collect();
    modifiers.kanian_impaler_style = active_weapon_style_ids.contains(TALENT_ID_KANIAN_IMPALER);
    for selection in &player.talents {
        let Some(spec) = find_talent(talent_catalog, &selection.id) else {
            continue;
        };
        if is_weapon_style_category(&spec.category)
            && !active_weapon_style_ids.contains(spec.id.as_str())
        {
            continue;
        }
        if !style_effects_active(selection, spec, &context, player) {
            continue;
        }
        let rank = talent_rank(selection);
        for effect in &spec.effects {
            match effect {
                TalentEffect::HitPointBonus { amount } => {
                    modifiers.hp_bonus += amount * rank;
                }
                TalentEffect::EssenceAdvancement => {
                    let (hp_bonus, drain_resistance) = essence_advancement_stats(player.level);
                    modifiers.hp_bonus += hp_bonus * rank;
                    modifiers.drain_resistance += drain_resistance * rank;
                }
                TalentEffect::ArmorDrBonus { amount } => {
                    modifiers.armor_dr_bonus += amount * rank;
                }
                TalentEffect::SpeedModBonus { amount } => {
                    modifiers.speed_mod_bonus += amount * rank;
                }
                TalentEffect::InitiativeModBonus { amount } => {
                    modifiers.initiative_mod_bonus += amount * rank;
                }
                TalentEffect::InitiativeDieBonus { steps } => {
                    modifiers.initiative_die_bonus += steps * rank;
                }
                TalentEffect::AttackBonusWeapon { amount } => {
                    if let Some(weapon_name) = selection.weapon.as_deref() {
                        if let Some(weapon_id) = weapon_id_by_name_cached(weapon_name) {
                            let entry = modifiers
                                .attack_bonus_by_weapon
                                .entry(weapon_id)
                                .or_insert(0);
                            *entry += amount * rank;
                        }
                    }
                }
                TalentEffect::DamageBonusWeapon { amount } => {
                    if let Some(weapon_name) = selection.weapon.as_deref() {
                        if let Some(weapon_id) = weapon_id_by_name_cached(weapon_name) {
                            let entry = modifiers
                                .damage_bonus_by_weapon
                                .entry(weapon_id)
                                .or_insert(0);
                            *entry += amount * rank;
                        }
                    }
                }
                TalentEffect::DamageBonusWeaponGroup {
                    amount,
                    weapon_group,
                } => {
                    let Some(group) = weapon_group_from_str(weapon_group) else {
                        continue;
                    };
                    let entry = modifiers.damage_bonus_by_group.entry(group).or_insert(0);
                    *entry += amount * rank;
                }
                TalentEffect::DefenseBonusWeapon { amount } => {
                    if let Some(weapon_name) = selection.weapon.as_deref() {
                        if let Some(weapon_id) = weapon_id_by_name_cached(weapon_name) {
                            let entry = modifiers
                                .defense_bonus_by_weapon
                                .entry(weapon_id)
                                .or_insert(0);
                            *entry += amount * rank;
                        }
                    }
                }
                TalentEffect::Dodge {
                    defense_bonus,
                    allow_dex_ranged,
                } => {
                    modifiers.defense_bonus += defense_bonus * rank;
                    modifiers.allow_dex_ranged |= *allow_dex_ranged;
                }
                TalentEffect::TraumaDieOverride { sides, penetrating } => {
                    modifiers.trauma_die_override = Some(TraumaDieOverride {
                        sides: *sides,
                        penetrating: *penetrating,
                    });
                }
                TalentEffect::ThresholdOfPainMultiplier { multiplier } => {
                    if *multiplier > 0.0 {
                        // Historical data calls this a multiplier, but the rule is a
                        // percentage-point bonus to the Threshold of Pain formula.
                        modifiers.threshold_of_pain_bonus_pct += (multiplier - 1.0) * rank as f32;
                    }
                }
                TalentEffect::ThresholdOfPainLevelBonus { per_level_pct } => {
                    if *per_level_pct > 0.0 {
                        modifiers.threshold_of_pain_level_bonus += per_level_pct * rank as f32;
                    }
                }
                TalentEffect::FastHealer => {}
                TalentEffect::WeaponSpeedBonus {
                    amount,
                    ranged_only,
                    weapon_group,
                } => {
                    let weapon_id = if let Some(group_name) = weapon_group.as_deref() {
                        let Some(group) = weapon_group_from_str(group_name) else {
                            continue;
                        };
                        let Some(weapon) = weapon_catalog.get(player.weapon_id) else {
                            continue;
                        };
                        if weapon.group != group {
                            continue;
                        }
                        player.weapon_id
                    } else if let Some(weapon_name) = selection.weapon.as_deref() {
                        let Some(weapon_id) = weapon_id_by_name_cached(weapon_name) else {
                            continue;
                        };
                        weapon_id
                    } else {
                        continue;
                    };
                    let Some(weapon) = weapon_catalog.get(weapon_id) else {
                        continue;
                    };
                    if *ranged_only && !is_ranged_weapon(weapon) {
                        continue;
                    }
                    let entry = modifiers
                        .weapon_speed_bonus_by_weapon
                        .entry(weapon_id)
                        .or_insert(0);
                    *entry += amount * rank;
                }
                TalentEffect::WeaponSpeedMultiplier {
                    multiplier,
                    min_multiplier,
                    weapon_groups,
                    weapon_names,
                } => {
                    if *multiplier <= 0.0 {
                        continue;
                    }
                    for (idx, weapon) in weapon_catalog.entries().iter().enumerate() {
                        if !weapon_matches_effect_filter(weapon, weapon_groups, weapon_names) {
                            continue;
                        }
                        let Some(weapon_id) = weapon_catalog.id_from_index(idx) else {
                            continue;
                        };
                        let entry = modifiers
                            .weapon_speed_multiplier_by_weapon
                            .entry(weapon_id)
                            .or_insert(1.0);
                        *entry *= multiplier.powi(rank);
                        if let Some(min_multiplier) = min_multiplier {
                            if *min_multiplier > 0.0 {
                                let entry = modifiers
                                    .weapon_min_speed_multiplier_by_weapon
                                    .entry(weapon_id)
                                    .or_insert(1.0);
                                *entry *= min_multiplier.powi(rank);
                            }
                        }
                        if spec.id == TALENT_ID_ROHAVALAN_BRIDGE {
                            modifiers.weapon_speed_round_up_by_weapon.insert(weapon_id);
                        }
                    }
                }
                TalentEffect::WeaponSpeedFlatBonus {
                    amount,
                    min_reach_ft,
                    weapon_groups,
                    weapon_names,
                } => {
                    for weapon_id in filtered_weapon_ids_with_min_reach(
                        weapon_catalog,
                        weapon_groups,
                        weapon_names,
                        *min_reach_ft,
                    ) {
                        let entry = modifiers
                            .weapon_speed_flat_bonus_by_weapon
                            .entry(weapon_id)
                            .or_insert(0.0);
                        *entry += *amount * rank as f32;
                    }
                }
                TalentEffect::WeaponReachBonus { amount } => {
                    if let Some(weapon_name) = selection.weapon.as_deref() {
                        if let Some(weapon_id) = weapon_id_by_name_cached(weapon_name) {
                            if let Some(weapon) = weapon_catalog.get(weapon_id) {
                                let entry = modifiers
                                    .reach_bonus_by_group
                                    .entry(weapon.group)
                                    .or_insert(0);
                                *entry += amount * rank;
                            }
                        }
                    }
                }
                TalentEffect::WeaponReachMultiplier {
                    multiplier,
                    weapon_groups,
                    weapon_names,
                } => {
                    if *multiplier <= 0.0 {
                        continue;
                    }
                    for (idx, weapon) in weapon_catalog.entries().iter().enumerate() {
                        if !weapon_matches_effect_filter(weapon, weapon_groups, weapon_names) {
                            continue;
                        }
                        let Some(weapon_id) = weapon_catalog.id_from_index(idx) else {
                            continue;
                        };
                        let entry = modifiers
                            .reach_multiplier_by_weapon
                            .entry(weapon_id)
                            .or_insert(1.0);
                        *entry *= multiplier.powi(rank);
                    }
                }
                TalentEffect::WeaponReachFlatBonus {
                    amount,
                    min_reach_ft,
                    weapon_groups,
                    weapon_names,
                } => {
                    for weapon_id in filtered_weapon_ids_with_min_reach(
                        weapon_catalog,
                        weapon_groups,
                        weapon_names,
                        *min_reach_ft,
                    ) {
                        let entry = modifiers
                            .weapon_reach_flat_bonus_by_weapon
                            .entry(weapon_id)
                            .or_insert(0.0);
                        *entry += *amount * rank as f32;
                    }
                }
                TalentEffect::CloseHitDamageExpr {
                    expr,
                    margin_less_than,
                    weapon_groups,
                    weapon_names,
                } => {
                    if expr.trim().is_empty() || *margin_less_than <= 0 {
                        continue;
                    }
                    for (idx, weapon) in weapon_catalog.entries().iter().enumerate() {
                        if !weapon_matches_effect_filter(weapon, weapon_groups, weapon_names) {
                            continue;
                        }
                        let Some(weapon_id) = weapon_catalog.id_from_index(idx) else {
                            continue;
                        };
                        modifiers.close_hit_damage_by_weapon.insert(
                            weapon_id,
                            CloseHitDamageRule {
                                expr: expr.clone(),
                                margin_less_than: *margin_less_than,
                            },
                        );
                    }
                }
                TalentEffect::WeaponAttackBonus {
                    amount,
                    weapon_groups,
                    weapon_names,
                } => {
                    for weapon_id in
                        filtered_weapon_ids(weapon_catalog, weapon_groups, weapon_names)
                    {
                        let entry = modifiers
                            .attack_bonus_by_weapon
                            .entry(weapon_id)
                            .or_insert(0);
                        *entry += amount * rank;
                    }
                }
                TalentEffect::CritMinRollWeaponGroup {
                    min_roll,
                    ranged_only,
                } => {
                    if let Some(group) = selection_weapon_group(selection) {
                        if *ranged_only {
                            let entry = modifiers
                                .crit_min_ranged_by_group
                                .entry(group)
                                .or_insert(20);
                            *entry = (*entry).min(*min_roll);
                        } else {
                            let entry = modifiers.crit_min_by_group.entry(group).or_insert(20);
                            *entry = (*entry).min(*min_roll);
                        }
                    }
                }
                TalentEffect::CritSeverityBonusWeaponGroup { amount } => {
                    if let Some(group) = selection_weapon_group(selection) {
                        let entry = modifiers
                            .crit_severity_bonus_by_group
                            .entry(group)
                            .or_insert(0);
                        *entry += amount * rank;
                    }
                }
                TalentEffect::WeaponDamageOptions {
                    no_strength_bonus,
                    no_mastery_bonus,
                    force_nonpenetrating,
                    halve_damage,
                    ignore_all_dr,
                    internal_hemorrhage_damage,
                    melee_only,
                    hacking_or_piercing,
                    weapon_groups,
                    weapon_names,
                } => {
                    for (idx, weapon) in weapon_catalog.entries().iter().enumerate() {
                        if !weapon_matches_effect_filter(weapon, weapon_groups, weapon_names) {
                            continue;
                        }
                        if *melee_only && is_ranged_weapon(weapon) {
                            continue;
                        }
                        if let Some(required) = hacking_or_piercing {
                            if weapon.hacking_or_piercing != *required {
                                continue;
                            }
                        }
                        let Some(weapon_id) = weapon_catalog.id_from_index(idx) else {
                            continue;
                        };
                        if *no_strength_bonus {
                            modifiers.no_strength_damage_by_weapon.insert(weapon_id);
                        }
                        if *no_mastery_bonus {
                            modifiers.no_mastery_damage_by_weapon.insert(weapon_id);
                        }
                        if *force_nonpenetrating {
                            modifiers
                                .force_nonpenetrating_damage_by_weapon
                                .insert(weapon_id);
                        }
                        if *halve_damage {
                            modifiers.halve_damage_by_weapon.insert(weapon_id);
                        }
                        if *ignore_all_dr {
                            modifiers.ignore_all_dr_by_weapon.insert(weapon_id);
                        }
                        if *internal_hemorrhage_damage != 0 {
                            let entry = modifiers
                                .internal_hemorrhage_damage_by_weapon
                                .entry(weapon_id)
                                .or_insert(0);
                            *entry += internal_hemorrhage_damage * rank;
                        }
                    }
                }
                TalentEffect::ExpandedPenetration {
                    attack_defense_max_minus_one,
                    damage_max_minus_one,
                    weapon_groups,
                    weapon_names,
                } => {
                    for weapon_id in
                        filtered_weapon_ids(weapon_catalog, weapon_groups, weapon_names)
                    {
                        if *attack_defense_max_minus_one {
                            modifiers
                                .expanded_attack_defense_penetration_by_weapon
                                .insert(weapon_id);
                        }
                        if *damage_max_minus_one {
                            modifiers
                                .expanded_damage_penetration_by_weapon
                                .insert(weapon_id);
                        }
                    }
                }
                TalentEffect::OpeningEngagementExtraDamageDice {
                    dice,
                    min_reach_ft,
                    weapon_groups,
                    weapon_names,
                } => {
                    for weapon_id in filtered_weapon_ids_with_min_reach(
                        weapon_catalog,
                        weapon_groups,
                        weapon_names,
                        *min_reach_ft,
                    ) {
                        let entry = modifiers
                            .opening_engagement_extra_damage_dice_by_weapon
                            .entry(weapon_id)
                            .or_insert(0);
                        *entry += dice * rank;
                    }
                }
                TalentEffect::AlwaysInitialEngagementIfReachAtLeastOpponent {
                    min_reach_ft,
                    weapon_groups,
                    weapon_names,
                } => {
                    for weapon_id in filtered_weapon_ids_with_min_reach(
                        weapon_catalog,
                        weapon_groups,
                        weapon_names,
                        *min_reach_ft,
                    ) {
                        modifiers
                            .always_initial_engagement_by_weapon
                            .insert(weapon_id);
                    }
                }
                TalentEffect::IgnoreDefenderMovementDefenseBonus {
                    weapon_groups,
                    weapon_names,
                } => {
                    for weapon_id in
                        filtered_weapon_ids(weapon_catalog, weapon_groups, weapon_names)
                    {
                        modifiers
                            .ignore_movement_defense_bonus_by_weapon
                            .insert(weapon_id);
                    }
                }
                TalentEffect::KnockbackResetsWeaponCount {
                    weapon_groups,
                    weapon_names,
                } => {
                    for weapon_id in
                        filtered_weapon_ids(weapon_catalog, weapon_groups, weapon_names)
                    {
                        modifiers
                            .knockback_resets_weapon_count_by_weapon
                            .insert(weapon_id);
                    }
                }
                TalentEffect::HitCriticalEffectsNoExtraDice {
                    weapon_groups,
                    weapon_names,
                } => {
                    for weapon_id in
                        filtered_weapon_ids(weapon_catalog, weapon_groups, weapon_names)
                    {
                        modifiers
                            .hit_critical_effects_no_extra_dice_by_weapon
                            .insert(weapon_id);
                    }
                }
                TalentEffect::IntAttackBonusToDamageAndDefense {
                    fraction: _,
                    weapon_groups,
                    shield_names: _,
                } => {
                    for weapon_id in filtered_weapon_ids(weapon_catalog, weapon_groups, &[]) {
                        modifiers
                            .attack_bonus_by_weapon
                            .entry(weapon_id)
                            .or_insert(0);
                    }
                    modifiers.ithican_prince_style = true;
                }
                TalentEffect::ThrownFullStrengthDamage {
                    thrown_only,
                    weapon_groups,
                    weapon_names,
                } => {
                    for (idx, weapon) in weapon_catalog.entries().iter().enumerate() {
                        if !weapon_matches_effect_filter(weapon, weapon_groups, weapon_names) {
                            continue;
                        }
                        if *thrown_only
                            && (weapon.range_bands_feet.is_none() || weapon.ammunition.is_some())
                        {
                            continue;
                        }
                        let Some(weapon_id) = weapon_catalog.id_from_index(idx) else {
                            continue;
                        };
                        modifiers
                            .thrown_full_strength_damage_by_weapon
                            .insert(weapon_id);
                    }
                }
                TalentEffect::ConsecutiveHitsForceTraumaTwenty {
                    hits,
                    weapon_groups,
                    weapon_names,
                } => {
                    for weapon_id in
                        filtered_weapon_ids(weapon_catalog, weapon_groups, weapon_names)
                    {
                        modifiers
                            .consecutive_hits_force_trauma_twenty_by_weapon
                            .insert(weapon_id, *hits);
                    }
                }
                TalentEffect::ShieldDrBonusFiltered {
                    amount,
                    shield_names,
                } => {
                    if shield_names.is_empty() {
                        modifiers.shield_dr_bonus_filtered += amount * rank;
                    } else {
                        for shield_name in shield_names {
                            let entry = modifiers
                                .shield_dr_bonus_by_name
                                .entry(shield_filter_key(shield_name))
                                .or_insert(0);
                            *entry += amount * rank;
                        }
                    }
                }
                TalentEffect::ShieldBreakageUsesShieldDr { shield_names } => {
                    if shield_names.is_empty() {
                        modifiers.shield_breakage_uses_shield_dr = true;
                    } else {
                        for shield_name in shield_names {
                            modifiers
                                .shield_breakage_uses_shield_dr_by_name
                                .insert(shield_filter_key(shield_name));
                        }
                    }
                }
                TalentEffect::KnockbackStepBonus { amount } => {
                    modifiers.knockback_step_bumps += amount * rank;
                }
                TalentEffect::IncomingCritExtraDamageHalved => {
                    modifiers.light_armor_crit_extra_damage_halved = true;
                }
                TalentEffect::IncomingCritSeverityReduction { amount } => {
                    modifiers.medium_armor_crit_severity_reduction += amount * rank;
                }
                TalentEffect::IgnoreAncillaryCritEffects => {
                    modifiers.heavy_armor_ignore_ancillary_crit_effects = true;
                }
                TalentEffect::IncomingCritDamageRollTwiceTakeLower => {
                    modifiers.defiant = true;
                }
                TalentEffect::NearPerfectDefenseMinRoll { roll } => {
                    if *roll <= 18 {
                        modifiers.superior_defense = true;
                    }
                }
                TalentEffect::PerfectDefenseCounterForceCritical => {
                    modifiers.edge_counter = true;
                }
                TalentEffect::FightDefensivelyAttackPenaltyDivisor { divisor } => {
                    modifiers.fight_defensively_attack_penalty_divisor = modifiers
                        .fight_defensively_attack_penalty_divisor
                        .max(*divisor);
                }
                TalentEffect::CalledShotDelayProfile { profile } => {
                    if let Some(profile) = called_shot_delay_profile_from_str(profile) {
                        modifiers.called_shot_delay_profile = Some(profile);
                    }
                }
                TalentEffect::CalledShotTargetDefenseBonusDivisor { divisor } => {
                    modifiers.called_shot_target_defense_bonus_divisor = modifiers
                        .called_shot_target_defense_bonus_divisor
                        .max(*divisor);
                }
                TalentEffect::CalledShotSelfDefensePenalty { amount } => {
                    modifiers.called_shot_self_defense_penalty = Some(
                        modifiers
                            .called_shot_self_defense_penalty
                            .map(|current| current.min(*amount))
                            .unwrap_or(*amount),
                    );
                }
                TalentEffect::CalledShotDeceptiveDefender => {
                    modifiers.called_shot_deceptive_defender = true;
                }
                TalentEffect::DualWieldOffhandDamagePenalty { amount } => {
                    modifiers.dualwield_offhand_damage_penalty = Some(
                        modifiers
                            .dualwield_offhand_damage_penalty
                            .map(|current| current.max(*amount))
                            .unwrap_or(*amount),
                    );
                }
                TalentEffect::DualWieldPrimaryRecoveryPenalty { amount } => {
                    modifiers.dualwield_primary_recovery_penalty = Some(
                        modifiers
                            .dualwield_primary_recovery_penalty
                            .map(|current| current.min(*amount))
                            .unwrap_or(*amount),
                    );
                }
                TalentEffect::DualWieldSecondaryRecoveryPenalty { amount } => {
                    modifiers.dualwield_secondary_recovery_penalty = Some(
                        modifiers
                            .dualwield_secondary_recovery_penalty
                            .map(|current| current.min(*amount))
                            .unwrap_or(*amount),
                    );
                }
                TalentEffect::PerfectTwoWeaponFighting => {
                    modifiers.perfect_two_weapon_fighting = true;
                }
                TalentEffect::RangeDistanceMultiplier { multiplier } => {
                    if *multiplier > 0.0 {
                        modifiers.range_distance_multiplier *= *multiplier;
                    }
                }
                TalentEffect::ArmorInitiativePenaltyNegation => {
                    modifiers.ignore_armor_initiative_penalty = true;
                }
                TalentEffect::ArmorSpeedPenaltyNegation => {
                    modifiers.ignore_armor_speed_penalty = true;
                }
                TalentEffect::ArmorDrBonusArmored { amount } => {
                    modifiers.armor_dr_bonus_armored += amount * rank;
                }
                TalentEffect::LightArmorDefenseBonusFromDr { divisor } => {
                    modifiers.light_armor_defense_divisor = Some(*divisor);
                }
                TalentEffect::MediumArmorDrBonus { amount } => {
                    modifiers.medium_armor_dr_bonus += amount * rank;
                }
                TalentEffect::MediumArmorDefensePenaltyReduction { amount } => {
                    modifiers.medium_armor_defense_penalty_reduction += amount * rank;
                }
                TalentEffect::HeavyArmorDamageBonusFromDr { divisor } => {
                    modifiers.heavy_armor_damage_bonus_divisor = Some(*divisor);
                }
                TalentEffect::HeavyArmorDamageBonus { amount } => {
                    modifiers.heavy_armor_damage_bonus_flat += amount * rank;
                }
                TalentEffect::ShieldDefenseBonus { amount } => {
                    modifiers.shield_defense_bonus += amount * rank;
                }
                TalentEffect::ShieldCoverValueAdjustment { amount } => {
                    modifiers.shield_cover_value_adjustment += amount * rank;
                }
                TalentEffect::ForcedWeaponLoadout {
                    weapon_name,
                    min_weapon_material_tier,
                    clear_projectile_material,
                    disable_offhand,
                    force_two_hand_grip,
                    force_no_shield,
                    d6_penetration_triggers,
                } => {
                    modifiers.forced_weapon_loadout = Some(ForcedWeaponLoadout {
                        weapon_name: weapon_name.clone(),
                        min_weapon_material_tier: *min_weapon_material_tier,
                        clear_projectile_material: *clear_projectile_material,
                        disable_offhand: *disable_offhand,
                        force_two_hand_grip: *force_two_hand_grip,
                        force_no_shield: *force_no_shield,
                        d6_penetration_triggers: d6_penetration_triggers.clone(),
                    });
                }
                TalentEffect::LargeSwordShieldStyle => {
                    modifiers.large_sword_shield_style = true;
                }
                TalentEffect::ArmerociPoleStyle => {
                    modifiers.armeroci_pole_style = true;
                }
                TalentEffect::CrescentMoonStyle => {
                    modifiers.crescent_moon_style = true;
                }
                TalentEffect::DoomrazorStyle => {
                    modifiers.doomrazor_style = true;
                }
                TalentEffect::FallingSunStyle => {
                    modifiers.falling_sun_style = true;
                }
                TalentEffect::FymblwngerStyle => {
                    modifiers.fymblwnger_style = true;
                }
                TalentEffect::HammererStyle => {
                    modifiers.hammerer_style = true;
                }
                TalentEffect::HobblerStyle => {
                    modifiers.hobbler_style = true;
                }
                TalentEffect::IthicanPrinceStyle => {
                    modifiers.ithican_prince_style = true;
                }
                TalentEffect::QuietRiverStyle => {
                    modifiers.quiet_river_style = true;
                }
                TalentEffect::RegenstatStyle => {
                    modifiers.regenstat_style = true;
                }
                TalentEffect::ReturnerStyle => {
                    modifiers.returner_style = true;
                }
                TalentEffect::RhdwngFlowStyle => {
                    modifiers.rhdwng_flow_style = true;
                }
                TalentEffect::ScornOfTheDissendriStyle => {
                    modifiers.scorn_of_the_dissendri_style = true;
                }
                TalentEffect::ShieldOfBladesStyle => {
                    modifiers.shield_of_blades_style = true;
                }
                TalentEffect::SixPathsStyle => {
                    modifiers.six_paths_style = true;
                }
                TalentEffect::StormOfBladesStyle => {
                    modifiers.storm_of_blades_style = true;
                }
                TalentEffect::ThreeMountainsStyle => {
                    modifiers.three_mountains_style = true;
                }
                TalentEffect::UnbreakableWallStyle => {
                    modifiers.unbreakable_wall_style = true;
                }
            }
        }
    }
    modifiers
}

fn player_has_talent(player: &PlayerConfig, id: &str) -> bool {
    player.talents.iter().any(|talent| talent.id == id)
}

fn positive_int_dex_attack_bonus(character: &Character) -> i32 {
    character.ability_mods.intelligence.attack.max(0)
        + character.ability_mods.dexterity.attack.max(0)
}

pub fn power_attack_available_for_player(player: &PlayerConfig, weapon: &WeaponPreset) -> bool {
    player_has_talent(player, TALENT_ID_POWER_ATTACK)
        && player.strength_base >= 13
        && !is_ranged_weapon(weapon)
        && !matches!(weapon.size, WeaponSize::Small)
}

fn power_attack_active(player: &PlayerConfig, weapon: &WeaponPreset) -> bool {
    player.power_attack && power_attack_available_for_player(player, weapon)
}

fn power_attack_attack_penalty(
    player: &PlayerConfig,
    weapon: &WeaponPreset,
    character: &Character,
) -> i32 {
    if power_attack_active(player, weapon) {
        positive_int_dex_attack_bonus(character)
    } else {
        0
    }
}

fn power_attack_strength_damage_bonus(
    player: &PlayerConfig,
    weapon: &WeaponPreset,
    strength_damage_base: i32,
) -> i32 {
    if power_attack_active(player, weapon) {
        strength_damage_for_weapon(weapon, strength_damage_base)
    } else {
        0
    }
}

fn fight_defensively_attack_penalty_with_modifiers(
    player: &PlayerConfig,
    modifiers: &TalentModifiers,
) -> i32 {
    if !player.fight_defensively {
        return 0;
    }
    let base_penalty = normalize_fight_defensively_penalty(player.fight_defensively_penalty);
    let divisor = modifiers.fight_defensively_attack_penalty_divisor.max(1);
    base_penalty / divisor
}

fn fight_defensively_defense_bonus_for_player(player: &PlayerConfig) -> i32 {
    if !player.fight_defensively {
        return 0;
    }
    normalize_fight_defensively_penalty(player.fight_defensively_penalty) / 2
}

fn has_precision_combatant_effect(player: &PlayerConfig) -> bool {
    player_has_talent(player, TALENT_ID_PRECISION_COMBATANT)
        || player_has_talent(player, TALENT_ID_CONTENDER)
        || player_has_talent(player, TALENT_ID_DUELIST)
}

fn has_deceptive_defender_effect(player: &PlayerConfig) -> bool {
    player_has_talent(player, TALENT_ID_DECEPTIVE_DEFENDER)
        || player_has_talent(player, TALENT_ID_CONTENDER)
        || player_has_talent(player, TALENT_ID_DUELIST)
}

fn called_shot_defense_bonus_with_modifiers(
    modifiers: &TalentModifiers,
    armor_type: ArmorType,
) -> i32 {
    let bonus = called_shot_target_defense_bonus_base_for_armor_type(armor_type);
    let divisor = modifiers.called_shot_target_defense_bonus_divisor.max(1);
    (bonus / divisor).max(1)
}

fn called_shot_defense_penalty_for_player(player: &PlayerConfig) -> i32 {
    if player_has_talent(player, TALENT_ID_DUELIST) {
        0
    } else if has_precision_combatant_effect(player) {
        2
    } else {
        4
    }
}

fn called_shot_defense_penalty_with_modifiers(modifiers: &TalentModifiers) -> i32 {
    modifiers.called_shot_self_defense_penalty.unwrap_or(4)
}

fn called_shot_delay_profile_for_player(player: &PlayerConfig) -> sim::CalledShotDelayProfile {
    if player_has_talent(player, TALENT_ID_PRECISION_AIMING) {
        sim::CalledShotDelayProfile::PrecisionAiming
    } else if has_precision_combatant_effect(player) {
        sim::CalledShotDelayProfile::PrecisionCombatant
    } else {
        sim::CalledShotDelayProfile::Standard
    }
}

fn called_shot_delay_profile_with_modifiers(
    modifiers: &TalentModifiers,
) -> sim::CalledShotDelayProfile {
    modifiers
        .called_shot_delay_profile
        .unwrap_or(sim::CalledShotDelayProfile::Standard)
}

fn called_shot_target_defense_bonus_base_for_armor_type(armor_type: ArmorType) -> i32 {
    match armor_type {
        ArmorType::Heavy => CALLED_SHOT_TARGET_DEFENSE_BONUS_HEAVY,
        ArmorType::Medium => CALLED_SHOT_TARGET_DEFENSE_BONUS_MEDIUM,
        ArmorType::Light | ArmorType::None => CALLED_SHOT_TARGET_DEFENSE_BONUS_LIGHT,
    }
}

fn called_shot_target_defense_bonus_for_armor_type(
    player: &PlayerConfig,
    armor_type: ArmorType,
) -> i32 {
    let mut bonus = called_shot_target_defense_bonus_base_for_armor_type(armor_type);
    if player_has_talent(player, TALENT_ID_PRECISION_AIMING)
        || has_precision_combatant_effect(player)
    {
        bonus /= 2;
    }
    bonus.max(1)
}

pub fn called_shot_target_defense_bonus_for_player(player: &PlayerConfig) -> i32 {
    called_shot_target_defense_bonus_for_armor_type(player, ArmorType::Medium)
}

pub fn called_shot_self_defense_penalty_for_player(player: &PlayerConfig) -> i32 {
    called_shot_defense_penalty_for_player(player)
}

pub fn called_shot_deceptive_defender_effect_active(player: &PlayerConfig) -> bool {
    has_deceptive_defender_effect(player)
}

pub fn called_shot_delay_expr_for_player(player: &PlayerConfig, is_ranged: bool) -> &'static str {
    match called_shot_delay_profile_for_player(player) {
        sim::CalledShotDelayProfile::Standard => {
            if is_ranged {
                "1d4p"
            } else {
                "2d4p"
            }
        }
        sim::CalledShotDelayProfile::PrecisionCombatant => "1d4p",
        sim::CalledShotDelayProfile::PrecisionAiming => "1d2",
    }
}

pub fn called_shot_target_defense_bonuses_for_player(player: &PlayerConfig) -> (i32, i32, i32) {
    (
        called_shot_target_defense_bonus_for_armor_type(player, ArmorType::Light),
        called_shot_target_defense_bonus_for_armor_type(player, ArmorType::Medium),
        called_shot_target_defense_bonus_for_armor_type(player, ArmorType::Heavy),
    )
}

pub fn called_shot_target_defense_bonus_against_target(
    attacker: &PlayerConfig,
    target: &PlayerConfig,
    armor_catalog: &ArmorCatalog,
) -> i32 {
    let target_armor_type = if target.npc_preset.is_some() {
        ArmorType::Medium
    } else {
        armor_catalog
            .get(target.armor_id)
            .and_then(|entry| entry.armor.as_ref())
            .map(|armor| armor.armor_type)
            .unwrap_or(ArmorType::None)
    };
    called_shot_target_defense_bonus_for_armor_type(attacker, target_armor_type)
}

fn kanian_impaler_knockback_adjustment(style_active: bool, weapon: &WeaponPreset) -> i32 {
    if style_active && weapon.group == WeaponGroup::Spears && weapon.size == WeaponSize::Large {
        -5
    } else {
        0
    }
}

fn resolve_misc_modifiers(player: &PlayerConfig) -> MiscRollModifiers {
    let mut modifiers = player.misc_modifiers;
    if let Some(race_id) = player.race_id.as_deref() {
        match race_id {
            "armeroci" => {
                modifiers.defense_bonus += 1;
                modifiers.initiative_die_bonus += 1;
            }
            "vorova_female" | "vorova_male" => {
                let temp = player.environment.temperature_c;
                let mut cold_bonus = if temp < 0 { 1 } else { 0 };
                let mut hot_penalty = if temp > 40 {
                    -2
                } else if temp > 30 {
                    -1
                } else {
                    0
                };
                if player_has_talent(player, "heat_adaptation") && hot_penalty < 0 {
                    hot_penalty += 1;
                }
                if player_has_talent(player, "frostheart") {
                    cold_bonus *= 3;
                    hot_penalty *= 3;
                }
                modifiers.all_roll_bonus += cold_bonus + hot_penalty;
            }
            "cirodes" => {
                if player.environment.bright_light && !player_has_talent(player, "light_adaptation")
                {
                    modifiers.attack_bonus -= 2;
                }
            }
            _ => {}
        }
    }
    modifiers
}

pub fn sanitize_player_ids(
    player: &mut PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
    talent_catalog: &TalentCatalog,
) {
    if weapon_catalog.get(player.weapon_id).is_none() {
        if let Some(id) = weapon_catalog.first_id() {
            player.weapon_id = id;
        }
    }
    if let Some(offhand_id) = player.offhand_weapon_id {
        if weapon_catalog.get(offhand_id).is_none() {
            player.offhand_weapon_id = None;
        }
    }
    if armor_catalog.get(player.armor_id).is_none() {
        if let Some(id) = armor_catalog.first_id() {
            player.armor_id = id;
        }
    }
    if shield_catalog.get(player.shield_id).is_none() {
        if let Some(id) = shield_catalog.first_id() {
            player.shield_id = id;
        }
    }
    enforce_forced_talent_equipment(player, weapon_catalog, shield_catalog, talent_catalog);
}

pub fn enforce_forced_talent_equipment(
    player: &mut PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    shield_catalog: &ShieldCatalog,
    talent_catalog: &TalentCatalog,
) {
    let modifiers = resolve_talent_modifiers(player, talent_catalog, weapon_catalog);
    let Some(loadout) = modifiers.forced_weapon_loadout.as_ref() else {
        return;
    };
    for (idx, weapon) in weapon_catalog.entries().iter().enumerate() {
        if weapon.name == loadout.weapon_name {
            if let Some(id) = weapon_catalog.id_from_index(idx) {
                player.weapon_id = id;
            }
            break;
        }
    }
    if let Some(min_tier) = loadout.min_weapon_material_tier {
        player.weapon_material_tier = player.weapon_material_tier.max(min_tier);
    }
    if loadout.clear_projectile_material {
        player.projectile_material_tier = 0;
    }
    if loadout.disable_offhand {
        player.offhand_weapon_id = None;
        player.offensive_dualwielding = false;
        player.defensive_dualwielding = false;
    }
    if loadout.force_two_hand_grip {
        player.two_hand_grip = true;
    }
    if loadout.force_no_shield {
        for (idx, entry) in shield_catalog.entries().iter().enumerate() {
            if entry.shield.is_none() || entry.label.eq_ignore_ascii_case("None") {
                if let Some(id) = shield_catalog.id_from_index(idx) {
                    player.shield_id = id;
                }
                break;
            }
        }
    }
}

pub fn weapon_uses_projectiles(weapon: &WeaponPreset) -> bool {
    uses_projectiles(&weapon.name, weapon.ammunition.is_some())
}

pub fn sanitize_projectile_tier(player: &mut PlayerConfig, weapon: &WeaponPreset) {
    if !weapon_uses_projectiles(weapon) {
        player.projectile_material_tier = 0;
    }
}

pub fn normalize_percentile(value: u8) -> u8 {
    if value >= 51 { 51 } else { 1 }
}

pub const DEFAULT_KNOCKBACK_STEP: i32 = 15;

pub fn knockback_step_for_race(race: &RaceSpec) -> i32 {
    let size = race.knockback_size.as_deref().unwrap_or(race.size.as_str());
    knockback_step_for_size_label(size)
}

pub fn knockback_step_for_race_id(race_id: Option<&str>, races: &[RaceSpec]) -> i32 {
    let Some(race_id) = race_id else {
        return DEFAULT_KNOCKBACK_STEP;
    };
    races
        .iter()
        .find(|race| race.id == race_id)
        .map(knockback_step_for_race)
        .unwrap_or(DEFAULT_KNOCKBACK_STEP)
}

fn knockback_step_for_size_label(label: &str) -> i32 {
    match label.trim().to_ascii_lowercase().as_str() {
        "small" => 10,
        "large" => 20,
        _ => DEFAULT_KNOCKBACK_STEP,
    }
}

fn bump_knockback_step(step: i32, bumps: i32) -> i32 {
    let mut step = step.max(1);
    for _ in 0..bumps.max(0) {
        step = match step {
            0..=10 => 15,
            11..=15 => 20,
            _ => 20,
        };
    }
    step
}

pub fn apply_race_adjustments(player: &mut PlayerConfig, race: &RaceSpec) {
    player.base_hp = race.base_hp.max(1);
    player.strength_base =
        clamp_stat_adjustment(player.strength_base, race.ability_adjustments.strength);
    player.dex_base = clamp_stat_adjustment(player.dex_base, race.ability_adjustments.dexterity);
    player.intelligence =
        clamp_stat_adjustment(player.intelligence, race.ability_adjustments.intelligence);
    player.wisdom = clamp_stat_adjustment(player.wisdom, race.ability_adjustments.wisdom);
    player.constitution =
        clamp_stat_adjustment(player.constitution, race.ability_adjustments.constitution);
    player.looks = clamp_stat_adjustment(player.looks, race.ability_adjustments.looks);
    player.charisma = clamp_stat_adjustment(player.charisma, race.ability_adjustments.charisma);
    player.race_id = Some(race.id.clone());
    player.knockback_step = knockback_step_for_race(race);
    player.race_applied = true;
}

fn clamp_stat_adjustment(base: u8, delta: i32) -> u8 {
    let adjusted = base as i32 + delta;
    adjusted.clamp(1, 25) as u8
}

pub fn clamp_mastery(value: i32) -> i32 {
    value.clamp(0, 6)
}

pub fn shield_equipped(player: &PlayerConfig, weapon: &WeaponPreset) -> bool {
    standard_shield_allowed(player, weapon) && player.shield_id.index() > 0
}

pub fn shield_equipped_with_catalog(
    player: &PlayerConfig,
    weapon: &WeaponPreset,
    shield_catalog: &ShieldCatalog,
    talent_catalog: &TalentCatalog,
    weapon_catalog: &WeaponCatalog,
) -> bool {
    let shield = shield_catalog
        .get(player.shield_id)
        .and_then(|entry| entry.shield.as_ref());
    can_equip_shield(player, weapon, shield, talent_catalog, weapon_catalog)
}

pub fn has_perfect_two_weapon_fighting_effect(player: &PlayerConfig) -> bool {
    player_has_talent(player, TALENT_ID_PERFECT_TWO_WEAPON_FIGHTING)
}

fn dualwield_mode_flags(player: &PlayerConfig, weapon: &WeaponPreset) -> (bool, bool, bool) {
    dualwield_mode_flags_with_perfect(
        player,
        weapon,
        has_perfect_two_weapon_fighting_effect(player),
    )
}

fn dualwield_mode_flags_with_perfect(
    player: &PlayerConfig,
    weapon: &WeaponPreset,
    has_perfect_two_weapon_fighting: bool,
) -> (bool, bool, bool) {
    if weapon.handedness != WeaponHandedness::OneHanded || player.two_hand_grip {
        return (false, false, false);
    }
    let offensive = player.offensive_dualwielding;
    let perfect_with_offense = offensive && has_perfect_two_weapon_fighting;
    let defensive = if offensive {
        perfect_with_offense
    } else {
        player.defensive_dualwielding
    };
    (defensive, offensive, perfect_with_offense)
}

pub fn defensive_dualwielding_active(player: &PlayerConfig, weapon: &WeaponPreset) -> bool {
    dualwield_mode_flags(player, weapon).0
}

pub fn offensive_dualwielding_active(player: &PlayerConfig, weapon: &WeaponPreset) -> bool {
    dualwield_mode_flags(player, weapon).1
}

pub fn default_offhand_weapon_id(
    player: &PlayerConfig,
    primary_weapon: &WeaponPreset,
    weapon_catalog: &WeaponCatalog,
) -> Option<WeaponId> {
    if primary_weapon.handedness != WeaponHandedness::OneHanded || player.two_hand_grip {
        return None;
    }
    if let Some(offhand_id) = player.offhand_weapon_id {
        if weapon_catalog
            .get(offhand_id)
            .map(|weapon| weapon.handedness == WeaponHandedness::OneHanded)
            .unwrap_or(false)
        {
            return Some(offhand_id);
        }
    }
    Some(player.weapon_id)
}

pub fn effective_attack_mastery(player: &PlayerConfig) -> i32 {
    clamp_mastery(player.mastery_attack)
}

pub fn effective_defense_mastery(player: &PlayerConfig, weapon: &WeaponPreset) -> i32 {
    if shield_equipped(player, weapon) {
        clamp_mastery(player.shield_mastery_defense)
    } else {
        clamp_mastery(player.mastery_defense)
    }
}

fn defense_mastery_bonus(
    player: &PlayerConfig,
    has_shield: bool,
    twelve_paths_active: bool,
    defensive_dualwielding: bool,
) -> i32 {
    let mastery = if twelve_paths_active && has_shield {
        clamp_mastery(player.mastery_defense) + clamp_mastery(player.shield_mastery_defense)
    } else if has_shield {
        clamp_mastery(player.shield_mastery_defense)
    } else {
        clamp_mastery(player.mastery_defense)
    };
    mastery * if defensive_dualwielding { 2 } else { 1 }
}

pub fn effective_damage_mastery(player: &PlayerConfig) -> i32 {
    clamp_mastery(player.mastery_damage)
}

pub fn effective_speed_mastery(player: &PlayerConfig, weapon: &WeaponPreset) -> i32 {
    let weapon_speed = clamp_mastery(player.mastery_speed);
    if shield_equipped(player, weapon) {
        let shield_speed = clamp_mastery(player.shield_mastery_speed);
        weapon_speed.min(shield_speed)
    } else {
        weapon_speed
    }
}

pub struct RollSummary {
    pub attack_bonus: i32,
    pub strength_damage: i32,
    pub is_ranged_weapon: bool,
}

pub struct DefenseDisplaySummary {
    pub shield_bonus: Option<i32>,
    pub shield_cover_value: Option<i32>,
    pub melee_roll_label: String,
    pub ranged_roll_label: String,
    pub melee_with_shield_dv: Option<i32>,
}

pub struct PlayerSummary {
    pub derived: DerivedStats,
    pub roll: RollSummary,
    pub defense: DefenseDisplaySummary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DerivedStatId {
    HitPoints,
    DrainResistance,
    ThresholdOfPain,
    AttackBonus,
    EffectiveAttackBonus,
    EffectiveDamageBonus,
    SpeedModifier,
    MainhandWeaponSpeed,
    InitiativeModifier,
    BaseDefense,
    MeleeDefense,
    RangedDefense,
    ArmorDr,
    CarryCapacity,
    LoadCategory,
    MainhandShieldDamage,
    MainhandAttackRoll,
    MainhandDamageRoll,
    OffhandWeaponSpeed,
    OffhandShieldDamage,
    OffhandAttackRoll,
    OffhandDamageRoll,
}

#[derive(Clone, Debug)]
pub struct BreakdownLine {
    pub value: String,
    pub source: String,
    pub numeric_amount: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct StatBreakdown {
    pub result: String,
    pub lines: Vec<BreakdownLine>,
    pub notes: Vec<String>,
}

impl StatBreakdown {
    fn new(result: impl Into<String>) -> Self {
        Self {
            result: result.into(),
            lines: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn add_i32(&mut self, amount: i32, source: impl Into<String>) {
        self.lines.push(BreakdownLine {
            value: format!("{amount:+}"),
            source: source.into(),
            numeric_amount: Some(amount as f64),
        });
    }

    fn add_f32(&mut self, amount: f32, source: impl Into<String>) {
        self.lines.push(BreakdownLine {
            value: format!("{amount:+.1}"),
            source: source.into(),
            numeric_amount: Some(amount as f64),
        });
    }

    fn add_text(&mut self, value: impl Into<String>, source: impl Into<String>) {
        self.lines.push(BreakdownLine {
            value: value.into(),
            source: source.into(),
            numeric_amount: None,
        });
    }

    fn note(&mut self, note: impl Into<String>) {
        self.notes.push(note.into());
    }

    pub fn additive_total(&self) -> f64 {
        self.lines
            .iter()
            .filter_map(|line| line.numeric_amount)
            .sum()
    }
}

#[derive(Clone, Debug, Default)]
pub struct DerivedStatBreakdowns {
    entries: HashMap<DerivedStatId, StatBreakdown>,
}

impl DerivedStatBreakdowns {
    pub fn get(&self, id: DerivedStatId) -> Option<&StatBreakdown> {
        self.entries.get(&id)
    }

    fn insert(&mut self, id: DerivedStatId, breakdown: StatBreakdown) {
        self.entries.insert(id, breakdown);
    }
}

fn weapon_for_player_with_modifiers<'a>(
    player: &PlayerConfig,
    weapon_catalog: &'a WeaponCatalog,
    modifiers: &TalentModifiers,
) -> &'a WeaponPreset {
    if let Some(loadout) = modifiers.forced_weapon_loadout.as_ref() {
        if let Some(weapon) = weapon_catalog
            .entries()
            .iter()
            .find(|weapon| weapon.name == loadout.weapon_name)
        {
            return weapon;
        }
    }
    weapon_catalog
        .get(player.weapon_id)
        .or_else(|| weapon_catalog.entries().first())
        .expect("weapon catalog is empty")
}

fn weapon_id_for_player_with_modifiers(
    player: &PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    modifiers: &TalentModifiers,
) -> WeaponId {
    if let Some(loadout) = modifiers.forced_weapon_loadout.as_ref() {
        for (idx, weapon) in weapon_catalog.entries().iter().enumerate() {
            if weapon.name == loadout.weapon_name {
                if let Some(id) = weapon_catalog.id_from_index(idx) {
                    return id;
                }
            }
        }
    }
    player.weapon_id
}

fn weapon_material_tier_with_modifiers(
    player: &PlayerConfig,
    weapon: &WeaponPreset,
    modifiers: &TalentModifiers,
) -> i32 {
    if let Some(loadout) = modifiers.forced_weapon_loadout.as_ref() {
        if weapon.name == loadout.weapon_name {
            return loadout
                .min_weapon_material_tier
                .map(|min_tier| player.weapon_material_tier.max(min_tier))
                .unwrap_or(player.weapon_material_tier);
        }
    }
    player.weapon_material_tier
}

fn effective_two_hand_grip_with_modifiers(
    player: &PlayerConfig,
    weapon: &WeaponPreset,
    modifiers: &TalentModifiers,
) -> bool {
    if modifiers
        .forced_weapon_loadout
        .as_ref()
        .map(|loadout| loadout.force_two_hand_grip && weapon.name == loadout.weapon_name)
        .unwrap_or(false)
    {
        true
    } else {
        player.two_hand_grip
    }
}

fn damage_expr_for_player_weapon(_player: &PlayerConfig, weapon: &WeaponPreset) -> String {
    weapon.damage_expr.clone()
}

fn damage_expr_cache_for_player_weapon(
    weapon: &WeaponPreset,
    modifiers: &TalentModifiers,
) -> DamageExprCache {
    if let Some(loadout) = modifiers.forced_weapon_loadout.as_ref() {
        if weapon.name == loadout.weapon_name && !loadout.d6_penetration_triggers.is_empty() {
            return DamageExprCache::new_with_d6_penetration_triggers(
                &weapon.damage_expr,
                &loadout.d6_penetration_triggers,
            );
        }
    }
    DamageExprCache::new(&weapon.damage_expr)
}

pub fn ability_set_from_player(player: &PlayerConfig) -> AbilitySet {
    AbilitySet {
        strength: AbilityScore::new(
            player.strength_base,
            normalize_percentile(player.strength_pct),
        ),
        intelligence: player.intelligence,
        wisdom: player.wisdom,
        dexterity: AbilityScore::new(player.dex_base, normalize_percentile(player.dex_pct)),
        constitution: player.constitution,
        looks: player.looks,
        charisma: player.charisma,
    }
}

pub fn player_summary(
    player: &PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
    talent_catalog: &TalentCatalog,
) -> PlayerSummary {
    let modifiers = resolve_talent_modifiers(player, talent_catalog, weapon_catalog);
    let weapon = weapon_for_player_with_modifiers(player, weapon_catalog, &modifiers);
    let character = build_character(
        player,
        weapon_catalog,
        armor_catalog,
        shield_catalog,
        talent_catalog,
    );
    let misc_modifiers = resolve_misc_modifiers(player);
    let armor_adjustments =
        armor_talent_adjustments(character.equipment.armor.as_ref(), &modifiers);
    let (defensive_dualwielding, offensive_dualwielding, perfect_two_weapon_fighting_active) =
        dualwield_mode_flags_with_perfect(player, weapon, modifiers.perfect_two_weapon_fighting);
    let weapon_id = weapon_id_for_player_with_modifiers(player, weapon_catalog, &modifiers);
    let fight_defensively_attack_penalty =
        fight_defensively_attack_penalty_with_modifiers(player, &modifiers);
    let fight_defensively_defense_bonus = fight_defensively_defense_bonus_for_player(player);
    let called_shot_defense_penalty = if player.called_shot {
        called_shot_defense_penalty_with_modifiers(&modifiers)
    } else {
        0
    };
    let defense_bonus_weapon =
        modifiers.defense_bonus_for_weapon(weapon_id) * if defensive_dualwielding { 2 } else { 1 };
    let mut derived = character.derived();
    derived.attack_bonus += misc_modifiers.attack_bonus + misc_modifiers.all_roll_bonus;
    derived.speed_mod += armor_adjustments.speed_mod_bonus
        + modifiers.speed_mod_bonus
        + misc_modifiers.speed_mod_bonus;
    derived.initiative_mod += armor_adjustments.initiative_mod_bonus
        + modifiers.initiative_mod_bonus
        + misc_modifiers.initiative_bonus
        + misc_modifiers.all_roll_bonus;
    derived.base_dv += armor_adjustments.base_dv_bonus;
    derived.initiative_die = derived
        .initiative_die
        .improved(modifiers.initiative_die_bonus + misc_modifiers.initiative_die_bonus);
    derived.hit_points =
        (derived.hit_points as i32 + modifiers.hp_bonus + misc_modifiers.hp_bonus).max(1) as u32;
    derived.drain_resistance += modifiers.drain_resistance;
    derived.armor_dr = (derived.armor_dr
        + armor_adjustments.armor_dr_bonus
        + modifiers.armor_dr_bonus
        + misc_modifiers.armor_dr_bonus)
        .max(0);
    if offensive_dualwielding && !perfect_two_weapon_fighting_active {
        derived.base_dv = 0;
    }
    derived.base_dv += modifiers.defense_bonus
        + defense_bonus_weapon
        + misc_modifiers.defense_bonus
        + misc_modifiers.all_roll_bonus;
    let twelve_paths_active =
        twelve_paths_style_active(&modifiers, weapon, character.equipment.shield.as_ref());
    let ithican_prince_active =
        ithican_prince_style_active(&modifiers, weapon, character.equipment.shield.as_ref());
    let hobbler_active = hobbler_style_active(&modifiers, weapon);
    let returner_active = returner_style_active(&modifiers, weapon);
    let ithican_half_int_bonus = if ithican_prince_active {
        character.ability_mods.intelligence.attack / 2
    } else {
        0
    };
    let style_defense_bonus = ithican_half_int_bonus
        - if returner_active {
            RETURNER_DEFENSE_PENALTY
        } else {
            0
        };
    derived.base_dv += style_defense_bonus;
    let defense = defense_display_summary(
        player,
        weapon,
        weapon_catalog,
        &character,
        &derived,
        &modifiers,
        twelve_paths_active,
        fight_defensively_defense_bonus,
        called_shot_defense_penalty,
    );
    let roll = roll_summary(
        player,
        weapon,
        weapon_catalog,
        &character,
        &derived,
        &modifiers,
        &misc_modifiers,
        armor_adjustments.heavy_armor_damage_bonus,
        twelve_paths_active,
        if hobbler_active {
            -HOBBLER_ATTACK_PENALTY
        } else {
            0
        },
        ithican_half_int_bonus,
        fight_defensively_attack_penalty,
    );
    PlayerSummary {
        derived,
        roll,
        defense,
    }
}

fn breakdown_talent_source<F>(
    player: &PlayerConfig,
    talent_catalog: &TalentCatalog,
    predicate: F,
) -> String
where
    F: Fn(&TalentEffect) -> bool,
{
    let mut names = player
        .talents
        .iter()
        .filter_map(|selection| {
            let spec = find_talent(talent_catalog, &selection.id)?;
            spec.effects
                .iter()
                .any(&predicate)
                .then_some(spec.name.clone())
        })
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    match names.as_slice() {
        [] => "Talent modifiers".to_string(),
        [name] => format!("Talent: {name}"),
        _ => format!("Talents: {}", names.join(", ")),
    }
}

fn estimated_gear_weight(character: &Character) -> Option<u32> {
    let mut total = 0.0f32;
    if let Some(weapon) = character.equipment.weapon.as_ref() {
        total += weapon.reach_ft;
    }
    if let Some(shield) = character.equipment.shield.as_ref() {
        total += match shield.name.as_str() {
            "Buckler" => 2.0,
            "Small Shield" => 3.0,
            "Medium Shield" => 6.0,
            "Large Shield" => 10.0,
            _ => 5.0,
        };
    }
    if let Some(armor) = character.equipment.armor.as_ref() {
        total += armor.weight_lbs;
    }
    (total > 0.0).then_some(total as u32)
}

pub fn derived_stat_breakdowns(
    player: &PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
    talent_catalog: &TalentCatalog,
    summary: &PlayerSummary,
    combatant: &Combatant,
) -> DerivedStatBreakdowns {
    let modifiers = resolve_talent_modifiers(player, talent_catalog, weapon_catalog);
    let weapon = weapon_for_player_with_modifiers(player, weapon_catalog, &modifiers);
    let weapon_id = weapon_id_for_player_with_modifiers(player, weapon_catalog, &modifiers);
    let character = build_character(
        player,
        weapon_catalog,
        armor_catalog,
        shield_catalog,
        talent_catalog,
    );
    let catalog_armor = armor_catalog
        .get(player.armor_id)
        .and_then(|entry| entry.armor.as_ref());
    let base_derived = character.derived();
    let misc = resolve_misc_modifiers(player);
    let armor_adjustments =
        armor_talent_adjustments(character.equipment.armor.as_ref(), &modifiers);
    let (defensive_dualwielding, offensive_dualwielding, perfect_two_weapon_fighting_active) =
        dualwield_mode_flags_with_perfect(player, weapon, modifiers.perfect_two_weapon_fighting);
    let has_shield = character.equipment.shield.is_some();
    let twelve_paths_active =
        twelve_paths_style_active(&modifiers, weapon, character.equipment.shield.as_ref());
    let ithican_prince_active =
        ithican_prince_style_active(&modifiers, weapon, character.equipment.shield.as_ref());
    let hobbler_active = hobbler_style_active(&modifiers, weapon);
    let returner_active = returner_style_active(&modifiers, weapon);
    let fight_defensively_attack_penalty =
        fight_defensively_attack_penalty_with_modifiers(player, &modifiers);
    let fight_defensively_defense_bonus = fight_defensively_defense_bonus_for_player(player);
    let called_shot_defense_penalty = if player.called_shot {
        called_shot_defense_penalty_with_modifiers(&modifiers)
    } else {
        0
    };
    let defense_bonus_weapon =
        modifiers.defense_bonus_for_weapon(weapon_id) * if defensive_dualwielding { 2 } else { 1 };
    let defense_mastery = defense_mastery_bonus(
        player,
        has_shield,
        twelve_paths_active,
        defensive_dualwielding,
    );
    let shield_of_blades_active = shield_of_blades_style_active(
        &modifiers,
        player,
        weapon,
        weapon_catalog,
        defensive_dualwielding,
    );
    let ithican_half_int_bonus = if ithican_prince_active {
        character.ability_mods.intelligence.attack / 2
    } else {
        0
    };
    let style_defense_bonus = ithican_half_int_bonus
        - if returner_active {
            RETURNER_DEFENSE_PENALTY
        } else {
            0
        };

    let mut breakdowns = DerivedStatBreakdowns::default();

    let mut hp = StatBreakdown::new(summary.derived.hit_points.to_string());
    hp.add_i32(player.base_hp as i32, "Base hit points");
    hp.add_i32(
        base_derived.hit_points as i32 - player.base_hp as i32,
        format!(
            "Constitution {} × health multiplier {:.1}, rounded",
            player.constitution, base_derived.health_mult
        ),
    );
    if modifiers.hp_bonus != 0 {
        hp.add_i32(
            modifiers.hp_bonus,
            breakdown_talent_source(player, talent_catalog, |effect| {
                matches!(
                    effect,
                    TalentEffect::HitPointBonus { .. } | TalentEffect::EssenceAdvancement
                )
            }),
        );
    }
    if misc.hp_bonus != 0 {
        hp.add_i32(misc.hp_bonus, "Miscellaneous HP modifier");
    }
    hp.note(format!(
        "Health multiplier {:.1} comes from level {} and Health progression.",
        base_derived.health_mult, player.level
    ));
    breakdowns.insert(DerivedStatId::HitPoints, hp);

    let mut drain = StatBreakdown::new(summary.derived.drain_resistance.to_string());
    drain.add_i32(0, "Base drain resistance");
    if modifiers.drain_resistance != 0 {
        drain.add_i32(
            modifiers.drain_resistance,
            breakdown_talent_source(player, talent_catalog, |effect| {
                matches!(effect, TalentEffect::EssenceAdvancement)
            }),
        );
    }
    breakdowns.insert(DerivedStatId::DrainResistance, drain);

    let mut threshold = StatBreakdown::new(combatant.sheet.vitals.threshold_of_pain.to_string());
    threshold.add_text(
        combatant.sheet.vitals.max_hp.to_string(),
        "Maximum hit points",
    );
    threshold.add_text("30%", "Base Threshold of Pain percentage");
    threshold.add_text(format!("+{}%", player.level), "Character level");
    if modifiers.threshold_of_pain_bonus_pct != 0.0 {
        threshold.add_text(
            format!("{:+.0}%", modifiers.threshold_of_pain_bonus_pct * 100.0),
            breakdown_talent_source(player, talent_catalog, |effect| {
                matches!(effect, TalentEffect::ThresholdOfPainMultiplier { .. })
            }),
        );
    }
    if modifiers.threshold_of_pain_level_bonus != 0.0 {
        threshold.add_text(
            format!(
                "+{:.0}%",
                player.level as f32 * modifiers.threshold_of_pain_level_bonus * 100.0
            ),
            breakdown_talent_source(player, talent_catalog, |effect| {
                matches!(effect, TalentEffect::ThresholdOfPainLevelBonus { .. })
            }),
        );
    }
    threshold.note("Maximum HP × total percentage, rounded up.");
    breakdowns.insert(DerivedStatId::ThresholdOfPain, threshold);

    let intelligence_attack = character.ability_mods.intelligence.attack;
    let dexterity_attack = character.ability_mods.dexterity.attack;
    let progression_attack = base_derived.attack_bonus - intelligence_attack - dexterity_attack;
    let mut attack = StatBreakdown::new(summary.derived.attack_bonus.to_string());
    attack.add_i32(
        progression_attack,
        format!(
            "Level {} / Attack progression {:?}",
            player.level, player.progression.attack
        ),
    );
    attack.add_i32(
        intelligence_attack,
        format!("Intelligence {}", player.intelligence),
    );
    attack.add_i32(
        dexterity_attack,
        format!("Dexterity {}/{}", player.dex_base, player.dex_pct),
    );
    if misc.attack_bonus != 0 {
        attack.add_i32(misc.attack_bonus, "Miscellaneous attack modifier");
    }
    if misc.all_roll_bonus != 0 {
        attack.add_i32(misc.all_roll_bonus, "Miscellaneous all-roll modifier");
    }
    breakdowns.insert(DerivedStatId::AttackBonus, attack);

    let is_ranged = is_ranged_weapon(weapon);
    let projectile_weapon = uses_projectiles(&weapon.name, weapon.ammunition.is_some());
    let (material_attack_bonus, material_damage_bonus) = material_bonuses(
        weapon_material_tier_with_modifiers(player, weapon, &modifiers),
        player.projectile_material_tier,
        is_ranged,
        projectile_weapon,
    );
    let attack_mastery = effective_attack_mastery(player);
    let weapon_attack_bonus = modifiers.attack_bonus_for_weapon(weapon_id);
    let power_attack_penalty = power_attack_attack_penalty(player, weapon, &character);
    let style_attack_bonus = if hobbler_active {
        -HOBBLER_ATTACK_PENALTY
    } else {
        0
    };
    let mut effective_attack = StatBreakdown::new(summary.roll.attack_bonus.to_string());
    effective_attack.add_i32(summary.derived.attack_bonus, "Derived attack bonus");
    if material_attack_bonus != 0 {
        effective_attack.add_i32(material_attack_bonus, "Weapon/projectile material");
    }
    if attack_mastery != 0 {
        effective_attack.add_i32(attack_mastery, "Attack mastery");
    }
    if weapon_attack_bonus != 0 {
        effective_attack.add_i32(
            weapon_attack_bonus,
            breakdown_talent_source(player, talent_catalog, |effect| {
                matches!(
                    effect,
                    TalentEffect::AttackBonusWeapon { .. } | TalentEffect::WeaponAttackBonus { .. }
                )
            }),
        );
    }
    if style_attack_bonus != 0 {
        effective_attack.add_i32(style_attack_bonus, "Weapon style: Hobbler");
    }
    if power_attack_penalty != 0 {
        effective_attack.add_i32(-power_attack_penalty, "Power Attack");
    }
    if fight_defensively_attack_penalty != 0 {
        effective_attack.add_i32(
            -fight_defensively_attack_penalty,
            if modifiers.fight_defensively_attack_penalty_divisor > 1 {
                "Fight Defensively (reduced by Combat Expertise)"
            } else {
                "Fight Defensively"
            },
        );
    }
    breakdowns.insert(
        DerivedStatId::EffectiveAttackBonus,
        effective_attack.clone(),
    );
    breakdowns.insert(DerivedStatId::MainhandAttackRoll, effective_attack);

    let effective_two_hand = effective_two_hand_grip_with_modifiers(player, weapon, &modifiers);
    let strength_damage_base =
        strength_damage_for_weapon(weapon, character.ability_mods.strength.damage);
    let two_hand_bonus = two_hand_damage_bonus(weapon, effective_two_hand);
    let damage_mastery = effective_damage_mastery(player);
    let weapon_damage_bonus = modifiers.damage_bonus_for_weapon(weapon_id);
    let group_damage_bonus = modifiers.damage_bonus_for_group(weapon.group);
    let armor_damage_bonus = if is_ranged {
        0
    } else {
        armor_adjustments.heavy_armor_damage_bonus
    };
    let twelve_paths_damage = if twelve_paths_active {
        -TWELVE_PATHS_DAMAGE_PENALTY
    } else {
        0
    };
    let power_attack_damage =
        power_attack_strength_damage_bonus(player, weapon, character.ability_mods.strength.damage);
    let mut effective_damage = StatBreakdown::new(summary.roll.strength_damage.to_string());
    effective_damage.add_i32(
        strength_damage_base,
        format!(
            "Strength {}/{} damage modifier",
            player.strength_base, player.strength_pct
        ),
    );
    if two_hand_bonus != 0 {
        effective_damage.add_i32(two_hand_bonus, "Two-handed grip");
    }
    if material_damage_bonus != 0 {
        effective_damage.add_i32(material_damage_bonus, "Weapon/projectile material");
    }
    if damage_mastery != 0 {
        effective_damage.add_i32(damage_mastery, "Damage mastery");
    }
    if weapon_damage_bonus != 0 {
        effective_damage.add_i32(
            weapon_damage_bonus,
            breakdown_talent_source(player, talent_catalog, |effect| {
                matches!(effect, TalentEffect::DamageBonusWeapon { .. })
            }),
        );
    }
    if group_damage_bonus != 0 {
        effective_damage.add_i32(
            group_damage_bonus,
            breakdown_talent_source(player, talent_catalog, |effect| {
                matches!(effect, TalentEffect::DamageBonusWeaponGroup { .. })
            }),
        );
    }
    if misc.damage_bonus != 0 {
        effective_damage.add_i32(misc.damage_bonus, "Miscellaneous damage modifier");
    }
    if misc.all_roll_bonus != 0 {
        effective_damage.add_i32(misc.all_roll_bonus, "Miscellaneous all-roll modifier");
    }
    if armor_damage_bonus != 0 {
        effective_damage.add_i32(
            armor_damage_bonus,
            breakdown_talent_source(player, talent_catalog, |effect| {
                matches!(
                    effect,
                    TalentEffect::HeavyArmorDamageBonusFromDr { .. }
                        | TalentEffect::HeavyArmorDamageBonus { .. }
                )
            }),
        );
    }
    if twelve_paths_damage != 0 {
        effective_damage.add_i32(twelve_paths_damage, "Weapon style: Twelve Paths");
    }
    if ithican_half_int_bonus != 0 {
        effective_damage.add_i32(ithican_half_int_bonus, "Weapon style: Ithican Prince");
    }
    if power_attack_damage != 0 {
        effective_damage.add_i32(power_attack_damage, "Power Attack");
    }
    breakdowns.insert(
        DerivedStatId::EffectiveDamageBonus,
        effective_damage.clone(),
    );
    let mut mainhand_damage = effective_damage;
    mainhand_damage.note(format!("Add weapon damage dice {}.", weapon.damage_expr));
    breakdowns.insert(DerivedStatId::MainhandDamageRoll, mainhand_damage);

    let armor_speed = character
        .equipment
        .armor
        .as_ref()
        .map(|armor| armor.speed_mod)
        .unwrap_or(0);
    let progression_speed = base_derived.speed_mod - armor_speed;
    let mut speed = StatBreakdown::new(summary.derived.speed_mod.to_string());
    speed.add_i32(
        progression_speed,
        format!(
            "Level {} / Speed progression {:?}",
            player.level, player.progression.speed
        ),
    );
    if armor_speed != 0 {
        speed.add_i32(armor_speed, "Armor speed modifier");
    }
    if armor_adjustments.speed_mod_bonus != 0 {
        speed.add_i32(
            armor_adjustments.speed_mod_bonus,
            breakdown_talent_source(player, talent_catalog, |effect| {
                matches!(effect, TalentEffect::ArmorSpeedPenaltyNegation)
            }),
        );
    }
    if modifiers.speed_mod_bonus != 0 {
        speed.add_i32(
            modifiers.speed_mod_bonus,
            breakdown_talent_source(player, talent_catalog, |effect| {
                matches!(effect, TalentEffect::SpeedModBonus { .. })
            }),
        );
    }
    if misc.speed_mod_bonus != 0 {
        speed.add_i32(misc.speed_mod_bonus, "Miscellaneous speed modifier");
    }
    breakdowns.insert(DerivedStatId::SpeedModifier, speed);

    let effective_two_hand = effective_two_hand_grip_with_modifiers(player, weapon, &modifiers);
    let two_hand_speed = two_hand_speed_penalty(weapon, effective_two_hand);
    let has_offhand = player.offhand_weapon_id.is_some();
    let free_hand_speed = if weapon.handedness == WeaponHandedness::OneHanded
        && !effective_two_hand
        && !has_offhand
        && !has_shield
        && !defensive_dualwielding
    {
        -1.0
    } else {
        0.0
    };
    let armeroci_speed =
        if modifiers.armeroci_pole_style && armeroci_pole_style_active(&modifiers, weapon) {
            ARMEROCI_POLE_SPEED_PENALTY
        } else {
            0.0
        };
    let falling_sun_speed =
        if modifiers.falling_sun_style && falling_sun_style_active(&modifiers, weapon) {
            2.0
        } else {
            0.0
        };
    let speed_mastery = effective_speed_mastery(player, weapon) as f32;
    let weapon_speed_talent = modifiers.weapon_speed_bonus_for_weapon(weapon_id) as f32;
    let weapon_speed_flat = modifiers.weapon_speed_flat_bonus_for_weapon(weapon_id);
    let speed_multiplier = modifiers.weapon_speed_multiplier_for_weapon(weapon_id);
    let min_speed_multiplier = modifiers.weapon_min_speed_multiplier_for_weapon(weapon_id);
    let speed_rounds_up = modifiers.weapon_speed_rounds_up_for_weapon(weapon_id);
    let base_weapon_speed = if player.use_jab {
        weapon.jab_speed.unwrap_or(weapon.speed)
    } else {
        weapon.speed
    };
    let mut weapon_speed =
        StatBreakdown::new(format!("{:.1}", combatant.sheet.offense.weapon.speed));
    weapon_speed.add_f32(
        base_weapon_speed,
        if player.use_jab {
            "Weapon jab speed"
        } else {
            "Weapon base speed"
        },
    );
    weapon_speed.add_f32(summary.derived.speed_mod as f32, "Derived speed modifier");
    if speed_mastery != 0.0 {
        weapon_speed.add_f32(-speed_mastery, "Speed mastery");
    }
    if two_hand_speed != 0.0 {
        weapon_speed.add_f32(two_hand_speed, "Two-handed grip");
    }
    if free_hand_speed != 0.0 {
        weapon_speed.add_f32(free_hand_speed, "Free hand");
    }
    if armeroci_speed != 0.0 {
        weapon_speed.add_f32(armeroci_speed, "Weapon style: Armeroci Pole");
    }
    if falling_sun_speed != 0.0 {
        weapon_speed.add_f32(falling_sun_speed, "Weapon style: Falling Sun");
    }
    if weapon_speed_talent != 0.0 {
        weapon_speed.add_f32(
            weapon_speed_talent,
            breakdown_talent_source(player, talent_catalog, |effect| {
                matches!(effect, TalentEffect::WeaponSpeedBonus { .. })
            }),
        );
    }
    if weapon_speed_flat != 0.0 {
        weapon_speed.add_f32(
            weapon_speed_flat,
            breakdown_talent_source(player, talent_catalog, |effect| {
                matches!(effect, TalentEffect::WeaponSpeedFlatBonus { .. })
            }),
        );
    }
    if speed_multiplier != 1.0 {
        weapon_speed.note(format!(
            "Speed total is multiplied by {speed_multiplier:.2}."
        ));
    }
    weapon_speed.note(format!(
        "Minimum speed is {:.1}.",
        weapon.size.min_speed() * min_speed_multiplier
    ));
    if speed_rounds_up {
        weapon_speed.note("Final speed is rounded up.");
    }
    breakdowns.insert(DerivedStatId::MainhandWeaponSpeed, weapon_speed);

    let dexterity_initiative = character.ability_mods.dexterity.initiative;
    let wisdom_initiative = character.ability_mods.wisdom.initiative;
    let armor_initiative = character
        .equipment
        .armor
        .as_ref()
        .map(|armor| armor.initiative_mod)
        .unwrap_or(0);
    let progression_initiative =
        base_derived.initiative_mod - dexterity_initiative - wisdom_initiative - armor_initiative;
    let mut initiative = StatBreakdown::new(summary.derived.initiative_mod.to_string());
    initiative.add_i32(
        progression_initiative,
        format!(
            "Level {} / Initiative progression {:?}",
            player.level, player.progression.initiative
        ),
    );
    initiative.add_i32(
        dexterity_initiative,
        format!("Dexterity {}/{}", player.dex_base, player.dex_pct),
    );
    initiative.add_i32(wisdom_initiative, format!("Wisdom {}", player.wisdom));
    if armor_initiative != 0 {
        initiative.add_i32(armor_initiative, "Armor initiative modifier");
    }
    if armor_adjustments.initiative_mod_bonus != 0 {
        initiative.add_i32(
            armor_adjustments.initiative_mod_bonus,
            breakdown_talent_source(player, talent_catalog, |effect| {
                matches!(effect, TalentEffect::ArmorInitiativePenaltyNegation)
            }),
        );
    }
    if modifiers.initiative_mod_bonus != 0 {
        initiative.add_i32(
            modifiers.initiative_mod_bonus,
            breakdown_talent_source(player, talent_catalog, |effect| {
                matches!(effect, TalentEffect::InitiativeModBonus { .. })
            }),
        );
    }
    if misc.initiative_bonus != 0 {
        initiative.add_i32(misc.initiative_bonus, "Miscellaneous initiative modifier");
    }
    if misc.all_roll_bonus != 0 {
        initiative.add_i32(misc.all_roll_bonus, "Miscellaneous all-roll modifier");
    }
    initiative.note(format!(
        "Initiative die: {:?} after die-quality modifiers.",
        summary.derived.initiative_die
    ));
    breakdowns.insert(DerivedStatId::InitiativeModifier, initiative);

    let dexterity_defense = character.ability_mods.dexterity.defense;
    let wisdom_defense = character.ability_mods.wisdom.defense;
    let adjusted_armor_defense = character
        .equipment
        .armor
        .as_ref()
        .map(|armor| armor.defense_adj)
        .unwrap_or(0);
    let catalog_armor_defense = catalog_armor.map(|armor| armor.defense_adj).unwrap_or(0);
    let mut base_defense = StatBreakdown::new(summary.derived.base_dv.to_string());
    base_defense.add_i32(crate::character::BASE_DV, "Unshielded base defense");
    base_defense.add_i32(
        dexterity_defense,
        format!("Dexterity {}/{}", player.dex_base, player.dex_pct),
    );
    base_defense.add_i32(wisdom_defense, format!("Wisdom {}", player.wisdom));
    if catalog_armor_defense != 0 {
        base_defense.add_i32(
            catalog_armor_defense,
            catalog_armor
                .map(|armor| format!("Armor: {}", armor.name))
                .unwrap_or_else(|| "Armor".to_string()),
        );
    }
    let armor_material_defense = adjusted_armor_defense - catalog_armor_defense;
    if armor_material_defense != 0 {
        base_defense.add_i32(
            armor_material_defense,
            format!("Armor material tier {}", player.armor_material_tier),
        );
    }
    if armor_adjustments.base_dv_bonus != 0 {
        base_defense.add_i32(
            armor_adjustments.base_dv_bonus,
            breakdown_talent_source(player, talent_catalog, |effect| {
                matches!(
                    effect,
                    TalentEffect::LightArmorDefenseBonusFromDr { .. }
                        | TalentEffect::MediumArmorDefensePenaltyReduction { .. }
                )
            }),
        );
    }
    if offensive_dualwielding && !perfect_two_weapon_fighting_active {
        base_defense.note("Offensive dual-wielding overrides the base subtotal to 0.");
    }
    if modifiers.defense_bonus != 0 {
        base_defense.add_i32(
            modifiers.defense_bonus,
            breakdown_talent_source(player, talent_catalog, |effect| {
                matches!(effect, TalentEffect::Dodge { .. })
            }),
        );
    }
    if defense_bonus_weapon != 0 {
        base_defense.add_i32(
            defense_bonus_weapon,
            format!(
                "{}{}",
                breakdown_talent_source(player, talent_catalog, |effect| {
                    matches!(effect, TalentEffect::DefenseBonusWeapon { .. })
                }),
                if defensive_dualwielding {
                    " (applies to both weapons)"
                } else {
                    ""
                }
            ),
        );
    }
    if misc.defense_bonus != 0 {
        base_defense.add_i32(misc.defense_bonus, "Miscellaneous defense modifier");
    }
    if misc.all_roll_bonus != 0 {
        base_defense.add_i32(misc.all_roll_bonus, "Miscellaneous all-roll modifier");
    }
    if style_defense_bonus != 0 {
        base_defense.add_i32(
            style_defense_bonus,
            if ithican_prince_active {
                "Weapon style: Ithican Prince"
            } else {
                "Weapon style: Returner"
            },
        );
    }
    breakdowns.insert(DerivedStatId::BaseDefense, base_defense);

    let mut melee_defense = StatBreakdown::new(summary.defense.melee_roll_label.clone());
    melee_defense.add_i32(summary.derived.base_dv, "Base DV");
    if defense_mastery != 0 {
        melee_defense.add_i32(
            defense_mastery,
            if defensive_dualwielding {
                "Defense mastery ×2 (defensive dual-wielding)"
            } else {
                "Defense mastery"
            },
        );
    }
    if shield_of_blades_active {
        melee_defense.add_i32(4, "Weapon style: Shield of Blades");
    } else if weapon.defense_bonus_always {
        melee_defense.add_i32(4, format!("{} weapon defense", weapon.name));
    }
    if let Some(shield_bonus) = summary.defense.shield_bonus {
        melee_defense.add_i32(4, "Using a shield removes the unshielded −4");
        melee_defense.add_i32(
            shield_bonus,
            character
                .equipment
                .shield
                .as_ref()
                .map(|shield| format!("Shield: {}", shield.name))
                .unwrap_or_else(|| "Shield defense bonus".to_string()),
        );
    }
    if fight_defensively_defense_bonus != 0 {
        melee_defense.add_i32(fight_defensively_defense_bonus, "Fight Defensively");
    }
    if called_shot_defense_penalty != 0 {
        melee_defense.add_i32(-called_shot_defense_penalty, "Your active Called Shot");
    }
    let conditional_weapon_defense = (defensive_dualwielding
        || effective_two_hand_grip_with_modifiers(player, weapon, &modifiers))
        && !weapon.defense_bonus_always
        && !shield_of_blades_active;
    if conditional_weapon_defense {
        melee_defense.note("+4 weapon defense becomes available after your attack.");
    }
    melee_defense.note(
        if offensive_dualwielding && !perfect_two_weapon_fighting_active {
            "Defense die is d10p while offensively dual-wielding."
        } else {
            "Defense die is d20p."
        },
    );
    if has_deceptive_defender_effect(player) {
        melee_defense.note(
            "Deceptive Defender: +1 against each opponent’s initial attack (not included above).",
        );
        melee_defense.note(
            "Deceptive Defender: +4 against every Called Shot, and that attack is delayed 4d4p (not included above).",
        );
    }
    breakdowns.insert(DerivedStatId::MeleeDefense, melee_defense);

    let mut ranged_defense = StatBreakdown::new(summary.defense.ranged_roll_label.clone());
    if has_shield {
        if let Some(shield_bonus) = summary.defense.shield_bonus {
            ranged_defense.add_i32(shield_bonus, "Shield defense bonus");
        }
        ranged_defense.note("A shield uses d20p against ranged attacks; its cover cap applies.");
    } else {
        if modifiers.allow_dex_ranged {
            ranged_defense.add_i32(
                character.ability_mods.dexterity.defense,
                format!(
                    "Dexterity via {}",
                    breakdown_talent_source(player, talent_catalog, |effect| matches!(
                        effect,
                        TalentEffect::Dodge { .. }
                    ))
                ),
            );
            if modifiers.defense_bonus != 0 {
                ranged_defense.add_i32(modifiers.defense_bonus, "General defense talents");
            }
        }
        ranged_defense.note("Stationary ranged defense uses d12p; moving uses d20p.");
    }
    if called_shot_defense_penalty != 0 {
        ranged_defense.add_i32(-called_shot_defense_penalty, "Your active Called Shot");
    }
    if has_deceptive_defender_effect(player) {
        ranged_defense.note(
            "Deceptive Defender: +1 against each opponent’s initial attack (not included above).",
        );
        ranged_defense.note(
            "Deceptive Defender: +4 against every Called Shot, and that attack is delayed 4d4p (not included above).",
        );
    }
    breakdowns.insert(DerivedStatId::RangedDefense, ranged_defense);

    let catalog_armor_dr = catalog_armor
        .map(|armor| armor.damage_reduction)
        .unwrap_or(0);
    let adjusted_armor_dr = character
        .equipment
        .armor
        .as_ref()
        .map(|armor| armor.damage_reduction)
        .unwrap_or(0);
    let mut armor_dr = StatBreakdown::new(summary.derived.armor_dr.to_string());
    armor_dr.add_i32(
        catalog_armor_dr,
        catalog_armor
            .map(|armor| format!("Armor: {}", armor.name))
            .unwrap_or_else(|| "No armor".to_string()),
    );
    let armor_material_dr = adjusted_armor_dr - catalog_armor_dr;
    if armor_material_dr != 0 {
        armor_dr.add_i32(
            armor_material_dr,
            format!("Armor material tier {}", player.armor_material_tier),
        );
    }
    if armor_adjustments.armor_dr_bonus != 0 {
        armor_dr.add_i32(armor_adjustments.armor_dr_bonus, "Armor talent adjustment");
    }
    if modifiers.armor_dr_bonus != 0 {
        armor_dr.add_i32(
            modifiers.armor_dr_bonus,
            breakdown_talent_source(player, talent_catalog, |effect| {
                matches!(effect, TalentEffect::ArmorDrBonus { .. })
            }),
        );
    }
    if misc.armor_dr_bonus != 0 {
        armor_dr.add_i32(misc.armor_dr_bonus, "Miscellaneous armor DR");
    }
    breakdowns.insert(DerivedStatId::ArmorDr, armor_dr);

    let mut carry = StatBreakdown::new(format!("{:?}", summary.derived.carry_capacity));
    carry.add_text(
        format!(
            "{}/{}/{}/{} lb",
            summary.derived.carry_capacity.0,
            summary.derived.carry_capacity.1,
            summary.derived.carry_capacity.2,
            summary.derived.carry_capacity.3
        ),
        format!(
            "Strength {}/{} carry table",
            player.strength_base, player.strength_pct
        ),
    );
    breakdowns.insert(DerivedStatId::CarryCapacity, carry);

    let mut load = StatBreakdown::new(summary.derived.load_category);
    if let Some(weight) = estimated_gear_weight(&character) {
        load.add_text(format!("{weight} lb"), "Estimated equipped gear weight");
    }
    load.add_text(
        format!("{:?}", summary.derived.carry_capacity),
        "Strength carry thresholds",
    );
    load.note("Load is the band containing the current estimated gear weight.");
    breakdowns.insert(DerivedStatId::LoadCategory, load);

    let mainhand_shield_damage = combatant
        .sheet
        .offense
        .weapon
        .shield_damage_expr
        .as_deref()
        .unwrap_or("-");
    let mut shield_damage = StatBreakdown::new(mainhand_shield_damage);
    shield_damage.add_text(mainhand_shield_damage, format!("Weapon: {}", weapon.name));
    breakdowns.insert(DerivedStatId::MainhandShieldDamage, shield_damage);

    if let (Some(offhand), Some(offhand_id)) = (
        combatant.sheet.offense.offhand.as_ref(),
        player.offhand_weapon_id,
    ) {
        if let Some(offhand_weapon) = weapon_catalog.get(offhand_id) {
            let offhand_speed_mastery = effective_speed_mastery(player, offhand_weapon);
            let offhand_speed_talent = modifiers.weapon_speed_bonus_for_weapon(offhand_id);
            let mut offhand_speed = StatBreakdown::new(format!("{:.1}", offhand.weapon.speed));
            offhand_speed.add_f32(offhand_weapon.speed, "Offhand weapon base speed");
            offhand_speed.add_f32(summary.derived.speed_mod as f32, "Derived speed modifier");
            if offhand_speed_mastery != 0 {
                offhand_speed.add_i32(-offhand_speed_mastery, "Speed mastery");
            }
            if offhand_speed_talent != 0 {
                offhand_speed.add_i32(offhand_speed_talent, "Offhand weapon speed talents");
            }
            offhand_speed.note(format!(
                "Minimum speed is {:.1}.",
                offhand_weapon.size.min_speed()
            ));
            breakdowns.insert(DerivedStatId::OffhandWeaponSpeed, offhand_speed);

            let offhand_is_ranged = is_ranged_weapon(offhand_weapon);
            let offhand_uses_projectiles =
                uses_projectiles(&offhand_weapon.name, offhand_weapon.ammunition.is_some());
            let (offhand_material_attack, offhand_material_damage) = material_bonuses(
                player.offhand_weapon_material_tier,
                player.offhand_projectile_material_tier,
                offhand_is_ranged,
                offhand_uses_projectiles,
            );
            let offhand_power_penalty =
                power_attack_attack_penalty(player, offhand_weapon, &character);
            let mut offhand_attack = StatBreakdown::new(offhand.attack_bonus.to_string());
            offhand_attack.add_i32(summary.derived.attack_bonus, "Derived attack bonus");
            if attack_mastery != 0 {
                offhand_attack.add_i32(attack_mastery, "Attack mastery");
            }
            if offhand_material_attack != 0 {
                offhand_attack.add_i32(offhand_material_attack, "Offhand material");
            }
            let offhand_talent_attack = modifiers.attack_bonus_for_weapon(offhand_id);
            if offhand_talent_attack != 0 {
                offhand_attack.add_i32(offhand_talent_attack, "Offhand weapon talents");
            }
            if offhand_power_penalty != 0 {
                offhand_attack.add_i32(-offhand_power_penalty, "Power Attack");
            }
            breakdowns.insert(DerivedStatId::OffhandAttackRoll, offhand_attack);

            let mut offhand_damage = StatBreakdown::new(offhand.strength_damage.to_string());
            offhand_damage.add_i32(
                strength_damage_for_weapon(offhand_weapon, character.ability_mods.strength.damage),
                "Strength damage modifier",
            );
            if offhand_material_damage != 0 {
                offhand_damage.add_i32(offhand_material_damage, "Offhand material");
            }
            if damage_mastery != 0 {
                offhand_damage.add_i32(damage_mastery, "Damage mastery");
            }
            let offhand_weapon_damage = modifiers.damage_bonus_for_weapon(offhand_id);
            if offhand_weapon_damage != 0 {
                offhand_damage.add_i32(offhand_weapon_damage, "Offhand weapon talents");
            }
            let offhand_group_damage = modifiers.damage_bonus_for_group(offhand_weapon.group);
            if offhand_group_damage != 0 {
                offhand_damage.add_i32(offhand_group_damage, "Offhand weapon-group talents");
            }
            if !offhand_is_ranged && armor_adjustments.heavy_armor_damage_bonus != 0 {
                offhand_damage.add_i32(
                    armor_adjustments.heavy_armor_damage_bonus,
                    "Heavy armor damage bonus",
                );
            }
            let offhand_power_damage = power_attack_strength_damage_bonus(
                player,
                offhand_weapon,
                character.ability_mods.strength.damage,
            );
            if offhand_power_damage != 0 {
                offhand_damage.add_i32(offhand_power_damage, "Power Attack");
            }
            offhand_damage.note(format!(
                "Add weapon damage dice {}.",
                offhand.weapon.damage_expr
            ));
            offhand_damage.note(format!(
                "Offhand damage modifier in combat: {:+}.",
                combatant.sheet.maneuvers.dualwield_offhand_damage_penalty
            ));
            breakdowns.insert(DerivedStatId::OffhandDamageRoll, offhand_damage);

            let offhand_shield_damage = offhand.weapon.shield_damage_expr.as_deref().unwrap_or("-");
            let mut offhand_shield = StatBreakdown::new(offhand_shield_damage);
            offhand_shield.add_text(
                offhand_shield_damage,
                format!("Weapon: {}", offhand_weapon.name),
            );
            breakdowns.insert(DerivedStatId::OffhandShieldDamage, offhand_shield);
        }
    }

    breakdowns
}

fn defense_display_summary(
    player: &PlayerConfig,
    weapon: &WeaponPreset,
    weapon_catalog: &WeaponCatalog,
    character: &Character,
    derived: &DerivedStats,
    modifiers: &TalentModifiers,
    twelve_paths_active: bool,
    fight_defensively_defense_bonus: i32,
    called_shot_defense_penalty: i32,
) -> DefenseDisplaySummary {
    let (defensive_dualwielding, offensive_dualwielding, perfect_two_weapon_fighting_active) =
        dualwield_mode_flags_with_perfect(player, weapon, modifiers.perfect_two_weapon_fighting);
    let shield_of_blades_active = shield_of_blades_style_active(
        modifiers,
        player,
        weapon,
        weapon_catalog,
        defensive_dualwielding,
    );
    let weapon_defense_bonus_always = weapon.defense_bonus_always || shield_of_blades_active;
    let has_shield = character.equipment.shield.is_some();
    let defense_mastery = defense_mastery_bonus(
        player,
        has_shield,
        twelve_paths_active,
        defensive_dualwielding,
    );
    let shield_bonus = character
        .equipment
        .shield
        .as_ref()
        .map(|shield| shield.defense_bonus + modifiers.shield_defense_bonus);
    let shield_cover_value = character
        .equipment
        .shield
        .as_ref()
        .map(|shield| (shield.cover_value + modifiers.shield_cover_value_adjustment).max(0));
    let weapon_note = if weapon.defense_bonus_always {
        " (+4 weapon)"
    } else {
        ""
    };
    let melee_die = if offensive_dualwielding && !perfect_two_weapon_fighting_active {
        "d10p"
    } else {
        "d20p"
    };
    let after_attack_bonus = (defensive_dualwielding
        || effective_two_hand_grip_with_modifiers(player, weapon, modifiers))
        && !weapon_defense_bonus_always;
    let weapon_defense_bonus = if weapon_defense_bonus_always { 4 } else { 0 };
    let shield_of_blades_defense_bonus = if shield_of_blades_active && !weapon.defense_bonus_always
    {
        4
    } else {
        0
    };
    let (melee_roll_label, melee_with_shield_dv) = if let Some(shield_bonus) = shield_bonus {
        let melee_base = derived.base_dv
            + defense_mastery
            + shield_of_blades_defense_bonus
            + 4
            + fight_defensively_defense_bonus
            - called_shot_defense_penalty;
        (
            format!(
                "Defense roll (melee): {melee_die} + {melee_base} + {shield_bonus}{weapon_note}"
            ),
            Some(
                derived.base_dv
                    + defense_mastery
                    + weapon_defense_bonus
                    + fight_defensively_defense_bonus
                    - called_shot_defense_penalty
                    + 4
                    + shield_bonus,
            ),
        )
    } else {
        let dual_note = if after_attack_bonus {
            " (+4 after you attack)"
        } else {
            ""
        };
        let melee_base = derived.base_dv
            + defense_mastery
            + shield_of_blades_defense_bonus
            + fight_defensively_defense_bonus
            - called_shot_defense_penalty;
        (
            format!(
                "Defense roll (melee): {melee_die} + {}{weapon_note}{dual_note}",
                melee_base
            ),
            None,
        )
    };
    let ranged_roll_label = if let Some(shield_bonus) = shield_bonus {
        if called_shot_defense_penalty > 0 {
            format!(
                "Defense roll (ranged): d20p + {shield_bonus} - {called_shot_defense_penalty} (cover cap applies)"
            )
        } else {
            format!("Defense roll (ranged): d20p + {shield_bonus} (cover cap applies)")
        }
    } else {
        if called_shot_defense_penalty > 0 {
            format!(
                "Defense roll (ranged): d12p if stationary, else d20p - {called_shot_defense_penalty}"
            )
        } else {
            "Defense roll (ranged): d12p if stationary, else d20p".to_string()
        }
    };

    DefenseDisplaySummary {
        shield_bonus,
        shield_cover_value,
        melee_roll_label,
        ranged_roll_label,
        melee_with_shield_dv,
    }
}

fn roll_summary(
    player: &PlayerConfig,
    weapon: &WeaponPreset,
    weapon_catalog: &WeaponCatalog,
    character: &Character,
    derived: &DerivedStats,
    modifiers: &TalentModifiers,
    misc_modifiers: &MiscRollModifiers,
    armor_damage_bonus: i32,
    twelve_paths_active: bool,
    style_attack_bonus: i32,
    style_damage_bonus: i32,
    fight_defensively_attack_penalty: i32,
) -> RollSummary {
    let is_ranged_weapon = is_ranged_weapon(weapon);
    let uses_projectiles = uses_projectiles(&weapon.name, weapon.ammunition.is_some());
    let weapon_id = weapon_id_for_player_with_modifiers(player, weapon_catalog, modifiers);
    let (material_attack_bonus, material_damage_bonus) = material_bonuses(
        weapon_material_tier_with_modifiers(player, weapon, modifiers),
        player.projectile_material_tier,
        is_ranged_weapon,
        uses_projectiles,
    );
    let attack_mastery = effective_attack_mastery(player);
    let damage_mastery = effective_damage_mastery(player);
    let power_attack_penalty = power_attack_attack_penalty(player, weapon, character);
    let attack_bonus = derived.attack_bonus
        + material_attack_bonus
        + attack_mastery
        + modifiers.attack_bonus_for_weapon(weapon_id)
        + style_attack_bonus
        - power_attack_penalty
        - fight_defensively_attack_penalty;
    let effective_two_hand = effective_two_hand_grip_with_modifiers(player, weapon, modifiers);
    let two_hand_bonus = two_hand_damage_bonus(weapon, effective_two_hand);
    let mut strength_damage =
        strength_damage_for_weapon(weapon, character.ability_mods.strength.damage)
            + two_hand_bonus
            + material_damage_bonus
            + damage_mastery
            + modifiers.damage_bonus_for_weapon(weapon_id)
            + modifiers.damage_bonus_for_group(weapon.group)
            + misc_modifiers.damage_bonus
            + misc_modifiers.all_roll_bonus;
    if !is_ranged_weapon {
        strength_damage += armor_damage_bonus;
    }
    if twelve_paths_active {
        strength_damage -= TWELVE_PATHS_DAMAGE_PENALTY;
    }
    strength_damage += style_damage_bonus;
    strength_damage +=
        power_attack_strength_damage_bonus(player, weapon, character.ability_mods.strength.damage);

    RollSummary {
        attack_bonus,
        strength_damage,
        is_ranged_weapon,
    }
}

pub fn build_character(
    player: &PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
    talent_catalog: &TalentCatalog,
) -> Character {
    let modifiers = resolve_talent_modifiers(player, talent_catalog, weapon_catalog);
    let weapon_preset = weapon_for_player_with_modifiers(player, weapon_catalog, &modifiers);
    let weapon = Weapon {
        name: weapon_preset.name.clone(),
        group: weapon_preset.group,
        speed: weapon_preset.speed,
        damage_expr: damage_expr_for_player_weapon(player, weapon_preset),
        reach_ft: weapon_preset.reach_ft,
        armor_pen: weapon_preset.armor_pen,
        defense_bonus_always: weapon_preset.defense_bonus_always,
    };
    let armor = armor_catalog
        .get(player.armor_id)
        .and_then(|entry| entry.armor.clone());
    let armor = armor.map(|armor| apply_armor_material_tier(armor, player.armor_material_tier));
    let shield = shield_catalog
        .get(player.shield_id)
        .and_then(|entry| entry.shield.clone());

    let abilities = ability_set_from_player(player);

    let mastery = WeaponMastery {
        group: weapon_preset.group,
        points: Default::default(),
        base_threshold: base_weapon_threshold(weapon_preset.group),
    };

    let shield = if modifiers
        .forced_weapon_loadout
        .as_ref()
        .map(|loadout| loadout.force_no_shield && weapon_preset.name == loadout.weapon_name)
        .unwrap_or(false)
    {
        None
    } else if can_equip_shield(
        player,
        weapon_preset,
        shield.as_ref(),
        talent_catalog,
        weapon_catalog,
    ) {
        shield.map(|shield| apply_shield_material_tier(shield, player.shield_material_tier))
    } else {
        None
    };

    let equipment = Equipment {
        weapon: Some(weapon),
        shield,
        armor,
        weapon_material: None,
        armor_material: None,
        shield_material: None,
    };

    Character::builder(&player.name)
        .level(player.level, player.progression)
        .base_hp(player.base_hp)
        .abilities(abilities)
        .weapon_mastery(mastery)
        .equipment(equipment)
        .build()
}

pub fn build_combatants(
    players: &[PlayerConfig; 2],
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
    npc_presets: &NpcPresetCatalog,
    talent_catalog: &TalentCatalog,
) -> Vec<Combatant> {
    let mut first = build_combatant(
        &players[0],
        weapon_catalog,
        armor_catalog,
        shield_catalog,
        npc_presets,
        talent_catalog,
    );
    let mut second = build_combatant(
        &players[1],
        weapon_catalog,
        armor_catalog,
        shield_catalog,
        npc_presets,
        talent_catalog,
    );
    first.team_id = 0;
    second.team_id = 1;
    vec![first, second]
}

pub fn build_combatant(
    player: &PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
    npc_presets: &NpcPresetCatalog,
    talent_catalog: &TalentCatalog,
) -> Combatant {
    let modifiers = resolve_talent_modifiers(player, talent_catalog, weapon_catalog);
    let weapon_preset = weapon_for_player_with_modifiers(player, weapon_catalog, &modifiers);
    let weapon_id = weapon_id_for_player_with_modifiers(player, weapon_catalog, &modifiers);
    let character = build_character(
        player,
        weapon_catalog,
        armor_catalog,
        shield_catalog,
        talent_catalog,
    );
    let misc_modifiers = resolve_misc_modifiers(player);
    let armor_adjustments =
        armor_talent_adjustments(character.equipment.armor.as_ref(), &modifiers);
    let mut derived = character.derived();
    derived.attack_bonus += misc_modifiers.attack_bonus + misc_modifiers.all_roll_bonus;
    derived.speed_mod += armor_adjustments.speed_mod_bonus
        + modifiers.speed_mod_bonus
        + misc_modifiers.speed_mod_bonus;
    derived.initiative_mod += armor_adjustments.initiative_mod_bonus
        + modifiers.initiative_mod_bonus
        + misc_modifiers.initiative_bonus
        + misc_modifiers.all_roll_bonus;
    derived.base_dv += armor_adjustments.base_dv_bonus;
    derived.armor_dr = (derived.armor_dr + armor_adjustments.armor_dr_bonus).max(0);
    let weapon_name = character
        .equipment
        .weapon
        .as_ref()
        .map(|weapon| weapon.name.clone())
        .unwrap_or_else(|| "Unarmed".to_string());
    let base_weapon_speed = character
        .equipment
        .weapon
        .as_ref()
        .map(|weapon| weapon.speed)
        .unwrap_or(10.0);
    let speed_mod = derived.speed_mod as f32;
    let reach_bonus = modifiers.reach_bonus_for_group(weapon_preset.group) as f32
        + modifiers.weapon_reach_flat_bonus_for_weapon(weapon_id);
    let mut weapon_reach = (character
        .equipment
        .weapon
        .as_ref()
        .map(|weapon| weapon.reach_ft)
        .unwrap_or(1.0)
        .max(1.0)
        + reach_bonus)
        .max(1.0);
    let armor_is_heavy = character
        .equipment
        .armor
        .as_ref()
        .map(|armor| matches!(armor.armor_type, ArmorType::Heavy))
        .unwrap_or(false);
    let armor_type = character
        .equipment
        .armor
        .as_ref()
        .map(|armor| armor.armor_type)
        .unwrap_or(ArmorType::None);
    let armor_penetration = character
        .equipment
        .weapon
        .as_ref()
        .map(|weapon| weapon.armor_pen)
        .unwrap_or(0);
    let shield_data = character.equipment.shield.as_ref();
    let weapon_defense_always = character
        .equipment
        .weapon
        .as_ref()
        .map(|weapon| weapon.defense_bonus_always)
        .unwrap_or(false);
    let has_weapon = character.equipment.weapon.is_some();
    let weapon_damage = character
        .equipment
        .weapon
        .as_ref()
        .map(|weapon| weapon.damage_expr.clone())
        .unwrap_or_else(|| "d4p".to_string());
    let shield_damage_expr = weapon_preset
        .shield_damage_expr
        .clone()
        .filter(|expr| expr != "-" && !expr.is_empty());
    let range_bands_feet = weapon_preset
        .range_bands_feet
        .or_else(|| sim::range_bands_for_weapon_name(&weapon_preset.name));

    let effective_two_hand =
        effective_two_hand_grip_with_modifiers(player, weapon_preset, &modifiers);
    let two_hand_damage_bonus = two_hand_damage_bonus(weapon_preset, effective_two_hand);
    let two_hand_speed_penalty = two_hand_speed_penalty(weapon_preset, effective_two_hand);
    let use_jab = player.use_jab && weapon_preset.jab_speed.is_some();
    let min_speed = weapon_preset.size.min_speed();
    let has_shield = character.equipment.shield.is_some();
    let armeroci_pole_active = armeroci_pole_style_active(&modifiers, weapon_preset)
        || modifiers
            .opening_engagement_extra_damage_dice_by_weapon
            .get(&weapon_id)
            .copied()
            .unwrap_or(0)
            > 0
        || modifiers
            .always_initial_engagement_by_weapon
            .contains(&weapon_id);
    let falling_sun_active = falling_sun_style_active(&modifiers, weapon_preset)
        || modifiers
            .expanded_attack_defense_penetration_by_weapon
            .contains(&weapon_id)
        || modifiers
            .expanded_damage_penetration_by_weapon
            .contains(&weapon_id);
    let doomrazor_active = doomrazor_style_active(&modifiers, weapon_preset)
        || modifiers
            .force_nonpenetrating_damage_by_weapon
            .contains(&weapon_id)
        || modifiers.no_strength_damage_by_weapon.contains(&weapon_id)
        || modifiers.no_mastery_damage_by_weapon.contains(&weapon_id)
        || modifiers
            .internal_hemorrhage_damage_by_weapon
            .get(&weapon_id)
            .copied()
            .unwrap_or(0)
            > 0;
    let fymblwnger_active = fymblwnger_style_active(&modifiers, weapon_preset)
        || modifiers
            .ignore_movement_defense_bonus_by_weapon
            .contains(&weapon_id);
    let hammerer_active = hammerer_style_active(&modifiers, weapon_preset)
        || modifiers
            .knockback_resets_weapon_count_by_weapon
            .contains(&weapon_id);
    let hobbler_active = hobbler_style_active(&modifiers, weapon_preset)
        || modifiers
            .hit_critical_effects_no_extra_dice_by_weapon
            .contains(&weapon_id);
    let quiet_river_active =
        quiet_river_style_active(&modifiers, weapon_preset, armor_type, shield_data)
            || ((modifiers.halve_damage_by_weapon.contains(&weapon_id)
                || modifiers.ignore_all_dr_by_weapon.contains(&weapon_id))
                && weapon_preset.name.trim().eq_ignore_ascii_case("fist")
                && matches!(armor_type, ArmorType::None)
                && shield_data.is_none());
    let rhdwng_flow_active = rhdwng_flow_style_active(&modifiers, weapon_preset)
        || modifiers
            .thrown_full_strength_damage_by_weapon
            .contains(&weapon_id);
    let ithican_prince_active = ithican_prince_style_active(&modifiers, weapon_preset, shield_data);
    let regenstat_active = regenstat_style_active(
        &modifiers,
        weapon_preset,
        effective_two_hand,
        player.offhand_weapon_id,
        shield_data,
    );
    let returner_active = returner_style_active(&modifiers, weapon_preset);
    let six_paths_active = six_paths_style_active(&modifiers, weapon_preset, shield_data);
    let three_mountains_active = three_mountains_style_active(&modifiers, weapon_preset)
        || modifiers
            .consecutive_hits_force_trauma_twenty_by_weapon
            .contains_key(&weapon_id);
    let shield_filter_key = shield_data.map(|shield| shield_filter_key(&shield.name));
    let unbreakable_wall_active = unbreakable_wall_style_active(&modifiers, shield_data)
        || modifiers.shield_dr_bonus_filtered != 0 && shield_data.is_some()
        || modifiers.shield_breakage_uses_shield_dr && shield_data.is_some()
        || shield_filter_key
            .as_ref()
            .map(|name| {
                modifiers.shield_dr_bonus_by_name.contains_key(name)
                    || modifiers
                        .shield_breakage_uses_shield_dr_by_name
                        .contains(name)
            })
            .unwrap_or(false);
    let twelve_paths_active = twelve_paths_style_active(&modifiers, weapon_preset, shield_data);
    if modifiers.armeroci_pole_style && armeroci_pole_style_active(&modifiers, weapon_preset) {
        weapon_reach = (weapon_reach + ARMEROCI_POLE_REACH_BONUS_FT).max(1.0);
    }
    let armeroci_speed_penalty =
        if modifiers.armeroci_pole_style && armeroci_pole_style_active(&modifiers, weapon_preset) {
            ARMEROCI_POLE_SPEED_PENALTY
        } else {
            0.0
        };
    let falling_sun_speed_penalty = if modifiers.falling_sun_style && falling_sun_active {
        2.0
    } else {
        0.0
    };
    let weapon_speed_flat_bonus = modifiers.weapon_speed_flat_bonus_for_weapon(weapon_id);
    let speed_multiplier = modifiers.weapon_speed_multiplier_for_weapon(weapon_id);
    let min_speed_multiplier = modifiers.weapon_min_speed_multiplier_for_weapon(weapon_id);
    let speed_rounds_up = modifiers.weapon_speed_rounds_up_for_weapon(weapon_id);
    let reach_multiplier = modifiers.reach_multiplier_for_weapon(weapon_id);
    let has_offhand = player.offhand_weapon_id.is_some();
    let (
        defensive_dualwielding_selected,
        mut offensive_dualwielding,
        perfect_two_weapon_fighting_active,
    ) = dualwield_mode_flags_with_perfect(
        player,
        weapon_preset,
        modifiers.perfect_two_weapon_fighting,
    );
    let mut defensive_dualwielding = defensive_dualwielding_selected;
    let shield_of_blades_active = shield_of_blades_style_active(
        &modifiers,
        player,
        weapon_preset,
        weapon_catalog,
        defensive_dualwielding_selected,
    );
    let weapon_defense_always = weapon_defense_always || shield_of_blades_active;
    let dualwield_offhand_damage_penalty = modifiers.dualwield_offhand_damage_penalty.unwrap_or(-2);
    let dualwield_primary_recovery_penalty =
        modifiers.dualwield_primary_recovery_penalty.unwrap_or(2.0);
    let dualwield_secondary_recovery_penalty = modifiers
        .dualwield_secondary_recovery_penalty
        .unwrap_or(2.0);
    let mut offensive_dualwielding_defense_penalty =
        offensive_dualwielding && !perfect_two_weapon_fighting_active;
    let fight_defensively_attack_penalty =
        fight_defensively_attack_penalty_with_modifiers(player, &modifiers);
    let fight_defensively_defense_bonus = fight_defensively_defense_bonus_for_player(player);
    let called_shot_defense_bonus =
        called_shot_defense_bonus_with_modifiers(&modifiers, ArmorType::Medium);
    let called_shot_defense_penalty = called_shot_defense_penalty_with_modifiers(&modifiers);
    let called_shot_delay_profile = called_shot_delay_profile_with_modifiers(&modifiers);
    let called_shot_deceptive_defender = modifiers.called_shot_deceptive_defender;
    let mut called_shot_target_defense_bonus_base = character
        .equipment
        .armor
        .as_ref()
        .map(|armor| called_shot_defense_bonus_with_modifiers(&modifiers, armor.armor_type))
        .unwrap_or_else(|| called_shot_defense_bonus_with_modifiers(&modifiers, ArmorType::Light));
    if offensive_dualwielding {
        defensive_dualwielding = perfect_two_weapon_fighting_active;
    }
    let free_hand_speed_bonus = if weapon_preset.handedness == WeaponHandedness::OneHanded
        && !effective_two_hand
        && !has_offhand
        && !has_shield
        && !defensive_dualwielding
    {
        -1.0
    } else {
        0.0
    };
    let speed_mastery = if has_shield {
        clamp_mastery(player.mastery_speed).min(clamp_mastery(player.shield_mastery_speed)) as f32
    } else {
        clamp_mastery(player.mastery_speed) as f32
    };
    let jab_speed = (weapon_preset.jab_speed.unwrap_or(base_weapon_speed) + speed_mod
        - speed_mastery
        + free_hand_speed_bonus
        + armeroci_speed_penalty
        + falling_sun_speed_penalty
        + modifiers.weapon_speed_bonus_for_weapon(weapon_id) as f32
        + weapon_speed_flat_bonus)
        .max(min_speed);
    let jab_speed = (jab_speed * speed_multiplier).max(min_speed * min_speed_multiplier);
    let jab_speed = if speed_rounds_up {
        jab_speed.ceil()
    } else {
        jab_speed
    };
    let jab_special_expr = if use_jab {
        weapon_preset.jab_special_expr.clone()
    } else {
        None
    };

    let mut name = character.name.clone();
    let primary_is_ranged = is_ranged_weapon(weapon_preset);
    let mut crit_min_roll = modifiers.crit_min_for_group(weapon_preset.group);
    if primary_is_ranged {
        if let Some(ranged_min) = modifiers.crit_min_ranged_for_group(weapon_preset.group) {
            crit_min_roll = crit_min_roll.min(ranged_min);
        }
    }
    let mut crit_min_roll_ranged = if primary_is_ranged {
        modifiers
            .crit_min_ranged_for_group(weapon_preset.group)
            .map(|value| value.min(crit_min_roll))
    } else {
        None
    };
    let mut crit_severity_bonus = modifiers.crit_severity_bonus_for_group(weapon_preset.group);
    let primary_uses_projectiles =
        uses_projectiles(&weapon_preset.name, weapon_preset.ammunition.is_some());
    let (material_attack_bonus, material_damage_bonus) = material_bonuses(
        weapon_material_tier_with_modifiers(player, weapon_preset, &modifiers),
        player.projectile_material_tier,
        primary_is_ranged,
        primary_uses_projectiles,
    );
    let attack_mastery = effective_attack_mastery(player);
    let mut attack_bonus_base = derived.attack_bonus + attack_mastery;
    let power_attack_penalty = power_attack_attack_penalty(player, weapon_preset, &character);
    attack_bonus_base -= power_attack_penalty;
    if offensive_dualwielding && !perfect_two_weapon_fighting_active {
        derived.base_dv = 0;
    }
    let defense_mastery = defense_mastery_bonus(
        player,
        has_shield,
        twelve_paths_active,
        defensive_dualwielding,
    );
    let defense_bonus_weapon =
        modifiers.defense_bonus_for_weapon(weapon_id) * if defensive_dualwielding { 2 } else { 1 };
    let defense_bonus =
        modifiers.defense_bonus + misc_modifiers.defense_bonus + misc_modifiers.all_roll_bonus;
    let damage_mastery = effective_damage_mastery(player);
    let mut attack_bonus =
        attack_bonus_base + material_attack_bonus + modifiers.attack_bonus_for_weapon(weapon_id);
    if modifiers.hobbler_style && hobbler_style_active(&modifiers, weapon_preset) {
        attack_bonus -= HOBBLER_ATTACK_PENALTY;
        attack_bonus_base -= HOBBLER_ATTACK_PENALTY;
    } else if hobbler_active {
        attack_bonus_base -= HOBBLER_ATTACK_PENALTY;
    }
    let mut defense_mod = derived.base_dv + defense_mastery + defense_bonus + defense_bonus_weapon;
    if quiet_river_active {
        defense_mod =
            derived.base_dv + (defense_mastery * 2) + defense_bonus + defense_bonus_weapon;
    }
    if returner_active {
        defense_mod -= RETURNER_DEFENSE_PENALTY;
    }
    let mut dex_defense_bonus = character.ability_mods.dexterity.defense;
    let mut natural_dr = (modifiers.armor_dr_bonus + misc_modifiers.armor_dr_bonus).max(0);
    let mut armor_dr = (derived.armor_dr + natural_dr).max(0);
    let mut strength_damage_base = character.ability_mods.strength.damage;
    let mut unarmed_damage_bonus = modifiers.damage_bonus_for_group(WeaponGroup::Unarmed);
    let mut strength_damage = strength_damage_for_weapon(weapon_preset, strength_damage_base)
        + two_hand_damage_bonus
        + material_damage_bonus
        + damage_mastery
        + modifiers.damage_bonus_for_weapon(weapon_id)
        + modifiers.damage_bonus_for_group(weapon_preset.group)
        + misc_modifiers.damage_bonus
        + misc_modifiers.all_roll_bonus;
    if !primary_is_ranged {
        strength_damage += armor_adjustments.heavy_armor_damage_bonus;
    }
    if twelve_paths_active {
        strength_damage -= TWELVE_PATHS_DAMAGE_PENALTY;
    }
    if ithican_prince_active {
        let half_int_bonus = character.ability_mods.intelligence.attack / 2;
        defense_mod += half_int_bonus;
        strength_damage += half_int_bonus;
    }
    strength_damage +=
        power_attack_strength_damage_bonus(player, weapon_preset, strength_damage_base);
    if doomrazor_active || modifiers.no_strength_damage_by_weapon.contains(&weapon_id) {
        strength_damage -= strength_damage_for_weapon(weapon_preset, strength_damage_base);
    }
    if doomrazor_active || modifiers.no_mastery_damage_by_weapon.contains(&weapon_id) {
        strength_damage -= damage_mastery;
    }
    let mut max_hp =
        (derived.hit_points as i32 + modifiers.hp_bonus + misc_modifiers.hp_bonus).max(1);
    let level_pct = player.level as f32 * 0.01;
    let level_bonus_pct = player.level as f32 * modifiers.threshold_of_pain_level_bonus;
    let top_pct = 0.30 + level_pct + level_bonus_pct + modifiers.threshold_of_pain_bonus_pct;
    let mut threshold_of_pain = ((max_hp as f32) * top_pct).ceil() as i32;
    let mut shield_name = shield_data.map(|shield| shield.name.clone());
    let mut shield_defense_bonus = shield_data.map(|shield| shield.defense_bonus).unwrap_or(0)
        + modifiers.shield_defense_bonus;
    let mut shield_dr = shield_data.map(|shield| shield.dr).unwrap_or(0);
    if modifiers.unbreakable_wall_style && unbreakable_wall_active {
        shield_dr += 2;
    }
    if modifiers.shield_dr_bonus_filtered != 0 && shield_data.map(|_| true).unwrap_or(false) {
        shield_dr += modifiers.shield_dr_bonus_filtered;
    }
    if let Some(shield_key) = shield_filter_key.as_ref() {
        shield_dr += modifiers
            .shield_dr_bonus_by_name
            .get(shield_key)
            .copied()
            .unwrap_or(0);
    }
    let mut shield_cover_value = shield_data.map(|shield| shield.cover_value);
    if let Some(cover_value) = shield_cover_value.as_mut() {
        *cover_value = (*cover_value + modifiers.shield_cover_value_adjustment).max(0);
    }
    let mut shield_breakage =
        shield_data.map(|shield| breakage_steps_from_thresholds(shield.breakage_thresholds));
    let mut ranged_defense_mod = if modifiers.allow_dex_ranged {
        character.ability_mods.dexterity.defense + defense_bonus
    } else {
        0
    };
    let (mut trauma_die_sides, mut trauma_die_penetrating) = modifiers
        .trauma_die_override
        .map(|override_die| (override_die.sides, override_die.penetrating))
        .unwrap_or((20, false));
    if let Some(preset) = player.npc_preset.and_then(|id| npc_presets.get(id)) {
        name = preset.name.clone();
        attack_bonus = preset.attack_bonus;
        attack_bonus_base = preset.attack_bonus;
        defense_mod = preset.defense_mod;
        armor_dr = preset.armor_dr;
        natural_dr = 0;
        strength_damage = preset.damage_bonus;
        strength_damage_base = 0;
        unarmed_damage_bonus = 0;
        max_hp = preset.hp.max(1);
        threshold_of_pain = preset.top.max(1);
        crit_min_roll = 20;
        crit_min_roll_ranged = None;
        crit_severity_bonus = 0;
        shield_name = None;
        shield_defense_bonus = 0;
        shield_dr = 0;
        shield_cover_value = None;
        shield_breakage = None;
        ranged_defense_mod = 0;
        dex_defense_bonus = 0;
        trauma_die_sides = 20;
        trauma_die_penetrating = false;
        defensive_dualwielding = false;
        offensive_dualwielding = false;
        called_shot_target_defense_bonus_base = CALLED_SHOT_TARGET_DEFENSE_BONUS_MEDIUM;
    }

    let weapon_speed = if use_jab {
        jab_speed
    } else {
        let speed = (base_weapon_speed + two_hand_speed_penalty + speed_mod - speed_mastery
            + free_hand_speed_bonus
            + armeroci_speed_penalty
            + falling_sun_speed_penalty
            + modifiers.weapon_speed_bonus_for_weapon(weapon_id) as f32
            + weapon_speed_flat_bonus)
            .max(min_speed);
        let speed = (speed * speed_multiplier).max(min_speed * min_speed_multiplier);
        if speed_rounds_up { speed.ceil() } else { speed }
    };
    weapon_reach = (weapon_reach * reach_multiplier).max(0.5);
    let damage_expr_cache = if falling_sun_active
        || modifiers
            .expanded_damage_penetration_by_weapon
            .contains(&weapon_id)
    {
        DamageExprCache::new_with_max_minus_one_penetration(&weapon_damage)
    } else {
        damage_expr_cache_for_player_weapon(weapon_preset, &modifiers)
    };
    let shield_damage_expr_cache = shield_damage_expr.as_deref().map(DamageExprCache::new);
    let jab_special_expr_cache = jab_special_expr.as_deref().map(DamageExprCache::new);
    let is_unarmed_weapon = weapon_preset.group == WeaponGroup::Unarmed;
    let is_small_weapon = matches!(weapon_preset.size, WeaponSize::Small);
    let knockback_step =
        bump_knockback_step(player.knockback_step.max(1), modifiers.knockback_step_bumps);
    let mut offhand_profile = None;
    let mut storm_of_blades = false;
    if offensive_dualwielding {
        if let Some(offhand_id) = player.offhand_weapon_id {
            if let Some(offhand_preset) = weapon_catalog.get(offhand_id) {
                if offhand_preset.handedness == WeaponHandedness::OneHanded {
                    storm_of_blades = modifiers.storm_of_blades_style
                        && is_one_handed_sword(weapon_preset)
                        && is_one_handed_sword(offhand_preset);
                    let offhand_is_ranged = is_ranged_weapon(offhand_preset);
                    let offhand_uses_projectiles =
                        uses_projectiles(&offhand_preset.name, offhand_preset.ammunition.is_some());
                    let (material_attack_bonus, material_damage_bonus) = material_bonuses(
                        player.offhand_weapon_material_tier,
                        player.offhand_projectile_material_tier,
                        offhand_is_ranged,
                        offhand_uses_projectiles,
                    );
                    let offhand_power_attack_penalty =
                        power_attack_attack_penalty(player, offhand_preset, &character);
                    let offhand_attack_bonus = derived.attack_bonus
                        + attack_mastery
                        + material_attack_bonus
                        + modifiers.attack_bonus_for_weapon(offhand_id)
                        - offhand_power_attack_penalty;
                    let mut offhand_strength_damage =
                        strength_damage_for_weapon(offhand_preset, strength_damage_base)
                            + material_damage_bonus
                            + damage_mastery
                            + modifiers.damage_bonus_for_weapon(offhand_id)
                            + modifiers.damage_bonus_for_group(offhand_preset.group)
                            + misc_modifiers.damage_bonus
                            + misc_modifiers.all_roll_bonus;
                    if !offhand_is_ranged {
                        offhand_strength_damage += armor_adjustments.heavy_armor_damage_bonus;
                    }
                    offhand_strength_damage += power_attack_strength_damage_bonus(
                        player,
                        offhand_preset,
                        strength_damage_base,
                    );
                    let offhand_reach = (offhand_preset.reach_ft.max(1.0)
                        + modifiers.reach_bonus_for_group(offhand_preset.group) as f32)
                        .max(1.0);
                    let offhand_speed_mastery =
                        effective_speed_mastery(player, offhand_preset) as f32;
                    let offhand_min_speed = offhand_preset.size.min_speed();
                    let offhand_speed = (offhand_preset.speed + speed_mod - offhand_speed_mastery
                        + modifiers.weapon_speed_bonus_for_weapon(offhand_id) as f32)
                        .max(offhand_min_speed);
                    let offhand_damage_expr = offhand_preset.damage_expr.clone();
                    let offhand_damage_expr_cache = DamageExprCache::new(&offhand_damage_expr);
                    let offhand_shield_damage_expr = offhand_preset
                        .shield_damage_expr
                        .clone()
                        .filter(|expr| expr != "-" && !expr.is_empty());
                    let offhand_shield_damage_expr_cache = offhand_shield_damage_expr
                        .as_deref()
                        .map(DamageExprCache::new);
                    let offhand_range_bands = offhand_preset
                        .range_bands_feet
                        .or_else(|| sim::range_bands_for_weapon_name(&offhand_preset.name));
                    let mut offhand_crit_min_roll =
                        modifiers.crit_min_for_group(offhand_preset.group);
                    if offhand_is_ranged {
                        if let Some(ranged_min) =
                            modifiers.crit_min_ranged_for_group(offhand_preset.group)
                        {
                            offhand_crit_min_roll = offhand_crit_min_roll.min(ranged_min);
                        }
                    }
                    let offhand_crit_min_roll_ranged = if offhand_is_ranged {
                        modifiers
                            .crit_min_ranged_for_group(offhand_preset.group)
                            .map(|value| value.min(offhand_crit_min_roll))
                    } else {
                        None
                    };
                    let offhand_crit_severity_bonus =
                        modifiers.crit_severity_bonus_for_group(offhand_preset.group);
                    let offhand_is_unarmed = offhand_preset.group == WeaponGroup::Unarmed;
                    let offhand_is_small = matches!(offhand_preset.size, WeaponSize::Small);
                    let offhand_knockback_adjustment = kanian_impaler_knockback_adjustment(
                        modifiers.kanian_impaler_style,
                        offhand_preset,
                    );
                    offhand_profile = Some(sim::OffhandProfile {
                        attack_bonus: offhand_attack_bonus,
                        strength_damage: offhand_strength_damage,
                        weapon: Arc::new(WeaponProfile {
                            name: offhand_preset.name.clone(),
                            damage_expr: offhand_damage_expr,
                            damage_expr_cache: offhand_damage_expr_cache,
                            shield_damage_expr: offhand_shield_damage_expr,
                            shield_damage_expr_cache: offhand_shield_damage_expr_cache,
                            armor_penetration: offhand_preset.armor_pen,
                            speed: offhand_speed,
                            reach_ft: offhand_reach,
                            range_bands_feet: offhand_range_bands,
                            range_distance_multiplier: modifiers.range_distance_multiplier,
                            two_hand_grip: false,
                            use_jab: false,
                            jab_special_expr: None,
                            jab_special_expr_cache: None,
                            has_weapon: true,
                            defense_bonus_always: offhand_preset.defense_bonus_always,
                            uses_projectiles: offhand_uses_projectiles,
                            is_small_weapon: offhand_is_small,
                            is_unarmed: offhand_is_unarmed,
                            hacking_or_piercing: offhand_preset.hacking_or_piercing,
                            force_nonpenetrating_damage: false,
                            halve_damage: false,
                            ignore_all_dr: false,
                            internal_hemorrhage_damage: 0,
                            use_close_hit_damage_expr: None,
                            use_close_hit_damage_expr_cache: None,
                            use_close_hit_margin_less_than: 0,
                            crit_min_roll: offhand_crit_min_roll,
                            crit_min_roll_ranged: offhand_crit_min_roll_ranged,
                            crit_severity_bonus: offhand_crit_severity_bonus,
                            defender_knockback_step_adjustment: offhand_knockback_adjustment,
                        }),
                    });
                }
            }
        }
    }
    if offhand_profile.is_none() {
        offensive_dualwielding = false;
        if perfect_two_weapon_fighting_active {
            defensive_dualwielding = defensive_dualwielding_selected;
            offensive_dualwielding_defense_penalty = false;
        }
    }
    let mut sheet_modifiers = sim::ModifierStack::default();
    if modifiers.defiant {
        sheet_modifiers.add_i32(sim::StatIdI32::FlagDefiant, sim::ModifierOpI32::Set(1));
    }
    if modifiers.superior_defense {
        sheet_modifiers.add_i32(
            sim::StatIdI32::FlagSuperiorDefense,
            sim::ModifierOpI32::Set(1),
        );
    }
    if modifiers.edge_counter {
        sheet_modifiers.add_i32(sim::StatIdI32::FlagEdgeCounter, sim::ModifierOpI32::Set(1));
    }
    if matches!(armor_type, ArmorType::Light) && modifiers.light_armor_crit_extra_damage_halved {
        sheet_modifiers.add_i32(
            sim::StatIdI32::FlagIncomingCritExtraDamageHalved,
            sim::ModifierOpI32::Set(1),
        );
    }
    if matches!(armor_type, ArmorType::Medium) && modifiers.medium_armor_crit_severity_reduction > 0
    {
        sheet_modifiers.add_i32(
            sim::StatIdI32::IncomingCritSeverityReduction,
            sim::ModifierOpI32::Add(modifiers.medium_armor_crit_severity_reduction),
        );
    }
    if modifiers.heavy_armor_ignore_ancillary_crit_effects {
        sheet_modifiers.add_i32(
            sim::StatIdI32::FlagIgnoreAncillaryCritEffects,
            sim::ModifierOpI32::Set(1),
        );
    }
    if twelve_paths_active {
        sheet_modifiers.add_i32(
            sim::StatIdI32::FlagLargeSwordShieldStyle,
            sim::ModifierOpI32::Set(1),
        );
    }
    if armeroci_pole_active {
        sheet_modifiers.add_i32(
            sim::StatIdI32::FlagArmerociPoleStyle,
            sim::ModifierOpI32::Set(1),
        );
    }
    if falling_sun_active {
        sheet_modifiers.add_i32(
            sim::StatIdI32::FlagFallingSunStyle,
            sim::ModifierOpI32::Set(1),
        );
    }
    if fymblwnger_active {
        sheet_modifiers.add_i32(
            sim::StatIdI32::FlagFymblwngerStyle,
            sim::ModifierOpI32::Set(1),
        );
    }
    if hammerer_active {
        sheet_modifiers.add_i32(
            sim::StatIdI32::FlagHammererStyle,
            sim::ModifierOpI32::Set(1),
        );
    }
    if hobbler_active {
        sheet_modifiers.add_i32(sim::StatIdI32::FlagHobblerStyle, sim::ModifierOpI32::Set(1));
    }
    if quiet_river_active {
        sheet_modifiers.add_i32(
            sim::StatIdI32::FlagQuietRiverStyle,
            sim::ModifierOpI32::Set(1),
        );
    }
    if rhdwng_flow_active {
        sheet_modifiers.add_i32(
            sim::StatIdI32::FlagRhdwngFlowStyle,
            sim::ModifierOpI32::Set(1),
        );
    }
    if ithican_prince_active {
        sheet_modifiers.add_i32(
            sim::StatIdI32::FlagIthicanPrinceStyle,
            sim::ModifierOpI32::Set(1),
        );
    }
    if regenstat_active {
        sheet_modifiers.add_i32(
            sim::StatIdI32::FlagRegenstatStyle,
            sim::ModifierOpI32::Set(1),
        );
    }
    if returner_active {
        sheet_modifiers.add_i32(
            sim::StatIdI32::FlagReturnerStyle,
            sim::ModifierOpI32::Set(1),
        );
    }
    if six_paths_active {
        sheet_modifiers.add_i32(
            sim::StatIdI32::FlagSixPathsStyle,
            sim::ModifierOpI32::Set(1),
        );
    }
    if three_mountains_active {
        sheet_modifiers.add_i32(
            sim::StatIdI32::FlagThreeMountainsStyle,
            sim::ModifierOpI32::Set(1),
        );
    }
    if unbreakable_wall_active {
        sheet_modifiers.add_i32(
            sim::StatIdI32::FlagUnbreakableWallStyle,
            sim::ModifierOpI32::Set(1),
        );
    }
    let defender_knockback_step_adjustment =
        kanian_impaler_knockback_adjustment(modifiers.kanian_impaler_style, weapon_preset);
    let sheet = CombatantSheet {
        name,
        offense: OffenseProfile {
            attack_bonus,
            attack_bonus_base,
            strength_damage,
            strength_damage_base,
            unarmed_damage_bonus,
            weapon: Arc::new(WeaponProfile {
                name: weapon_name,
                damage_expr: weapon_damage,
                damage_expr_cache,
                shield_damage_expr,
                shield_damage_expr_cache,
                armor_penetration,
                speed: weapon_speed,
                reach_ft: weapon_reach,
                range_bands_feet,
                range_distance_multiplier: modifiers.range_distance_multiplier,
                two_hand_grip: effective_two_hand,
                use_jab,
                jab_special_expr,
                jab_special_expr_cache,
                has_weapon,
                defense_bonus_always: weapon_defense_always,
                uses_projectiles: primary_uses_projectiles,
                is_small_weapon,
                is_unarmed: is_unarmed_weapon,
                hacking_or_piercing: weapon_preset.hacking_or_piercing,
                force_nonpenetrating_damage: doomrazor_active
                    || modifiers
                        .force_nonpenetrating_damage_by_weapon
                        .contains(&weapon_id),
                halve_damage: quiet_river_active
                    || modifiers.halve_damage_by_weapon.contains(&weapon_id),
                ignore_all_dr: quiet_river_active
                    || modifiers.ignore_all_dr_by_weapon.contains(&weapon_id),
                internal_hemorrhage_damage: modifiers
                    .internal_hemorrhage_damage_by_weapon
                    .get(&weapon_id)
                    .copied()
                    .unwrap_or(if doomrazor_active { 1 } else { 0 }),
                use_close_hit_damage_expr: modifiers
                    .close_hit_damage_for_weapon(weapon_id)
                    .map(|rule| rule.expr.clone()),
                use_close_hit_damage_expr_cache: modifiers
                    .close_hit_damage_for_weapon(weapon_id)
                    .map(|rule| DamageExprCache::new(&rule.expr)),
                use_close_hit_margin_less_than: modifiers
                    .close_hit_damage_for_weapon(weapon_id)
                    .map(|rule| rule.margin_less_than)
                    .unwrap_or(0),
                crit_min_roll,
                crit_min_roll_ranged,
                crit_severity_bonus,
                defender_knockback_step_adjustment,
            }),
            offhand: offhand_profile,
        },
        defense: DefenseProfile {
            defense_mod,
            ranged_defense_mod,
            dex_defense_bonus,
            armor_dr,
            natural_dr,
            knockback_step,
            armor_is_heavy,
            shield_name,
            shield_defense_bonus,
            shield_dr,
            shield_cover_value,
            shield_breakage,
        },
        mobility: MobilityProfile {
            move_speed: player.move_speed,
        },
        vitals: Vitals {
            max_hp,
            infinite_hp: false,
            constitution: player.constitution,
            drain_resistance: modifiers.drain_resistance,
            threshold_of_pain,
            trauma_die_sides,
            trauma_die_penetrating,
        },
        maneuvers: sim::ManeuverProfile {
            hold_at_bay: player.hold_at_bay,
            called_shot: player.called_shot,
            called_shot_defense_bonus,
            called_shot_defense_penalty,
            called_shot_delay_profile,
            called_shot_deceptive_defender,
            called_shot_target_defense_bonus_base,
            power_attack: power_attack_active(player, weapon_preset),
            aggressive_attack: player.aggressive_attack,
            charge: player.charge,
            ready_against_charge: player.ready_against_charge,
            tactical_move: player.tactical_move,
            fight_defensively: player.fight_defensively,
            fight_defensively_attack_penalty,
            fight_defensively_defense_bonus,
            full_parry: player.full_parry,
            give_ground: player.give_ground,
            scamper_back: player.scamper_back,
            fighting_withdrawal: player.fighting_withdrawal,
            flee: player.flee,
            mounted: player.mounted,
            defensive_dualwielding,
            offensive_dualwielding,
            offensive_dualwielding_defense_penalty,
            dualwield_offhand_damage_penalty,
            dualwield_primary_recovery_penalty,
            dualwield_secondary_recovery_penalty,
            storm_of_blades,
            passive: false,
        },
        modifiers: sheet_modifiers,
    };

    Combatant::new(sheet)
}

pub fn stop_distance_for_players(
    players: &[PlayerConfig; 2],
    weapon_catalog: &WeaponCatalog,
    talent_catalog: &TalentCatalog,
) -> f32 {
    let melee_reach_from_label = |label: &str| {
        if !label.contains('/') {
            return None;
        }
        let reach_token = label.split('/').next().unwrap_or("").trim();
        reach_token
            .split_whitespace()
            .next()
            .and_then(|token| token.parse::<f32>().ok())
    };
    let reach_for_player = |player: &PlayerConfig| {
        let modifiers = resolve_talent_modifiers(player, talent_catalog, weapon_catalog);
        let weapon = weapon_for_player_with_modifiers(player, weapon_catalog, &modifiers);
        let weapon_id = weapon_id_for_player_with_modifiers(player, weapon_catalog, &modifiers);
        let reach_bonus = modifiers.reach_bonus_for_group(weapon.group) as f32
            + modifiers.weapon_reach_flat_bonus_for_weapon(weapon_id);
        let armeroci_reach_bonus =
            if modifiers.armeroci_pole_style && armeroci_pole_style_active(&modifiers, weapon) {
                ARMEROCI_POLE_REACH_BONUS_FT
            } else {
                0.0
            };
        let reach_multiplier = modifiers.reach_multiplier_for_weapon(weapon_id);
        let base_reach = if is_ranged_weapon(weapon) {
            melee_reach_from_label(&weapon.reach_label).unwrap_or(1.0)
        } else {
            weapon.reach_ft
        };
        ((base_reach + reach_bonus + armeroci_reach_bonus) * reach_multiplier).max(0.5)
    };
    let reach_a = reach_for_player(&players[0]);
    let reach_b = reach_for_player(&players[1]);
    reach_a.max(reach_b)
}

fn standard_shield_allowed(player: &PlayerConfig, weapon: &WeaponPreset) -> bool {
    weapon.handedness == WeaponHandedness::OneHanded
        && !player.two_hand_grip
        && !defensive_dualwielding_active(player, weapon)
        && !offensive_dualwielding_active(player, weapon)
}

pub fn shield_option_allowed(
    player: &PlayerConfig,
    weapon: &WeaponPreset,
    shield: Option<&ShieldPreset>,
    talent_catalog: &TalentCatalog,
    weapon_catalog: &WeaponCatalog,
) -> bool {
    let Some(shield) = shield else {
        return true;
    };
    if standard_shield_allowed(player, weapon) {
        return true;
    }
    if !is_small_shield_or_buckler_name(&shield.name) {
        return false;
    }
    if weapon.group != WeaponGroup::LargeSwords || weapon.size != WeaponSize::Large {
        return false;
    }
    active_weapon_style_specs(player, talent_catalog, Some(weapon_catalog))
        .iter()
        .any(|spec| has_large_sword_shield_style_effect(spec))
}

fn can_equip_shield(
    player: &PlayerConfig,
    weapon: &WeaponPreset,
    shield: Option<&ShieldPreset>,
    talent_catalog: &TalentCatalog,
    weapon_catalog: &WeaponCatalog,
) -> bool {
    shield_option_allowed(player, weapon, shield, talent_catalog, weapon_catalog)
        && shield.is_some()
}

fn apply_shield_material_tier(shield: ShieldPreset, tier: i32) -> Shield {
    let tier = tier.clamp(0, 5);
    let mut defense_bonus = shield.defense_bonus;
    let mut dr = shield.dr;
    let mut breakage_thresholds = shield.breakage_thresholds;
    if tier > 0 {
        defense_bonus += tier;
        dr += tier;
        breakage_thresholds = [
            breakage_thresholds[0] + tier * 2,
            breakage_thresholds[1] + tier * 3,
            breakage_thresholds[2] + tier * 4,
            breakage_thresholds[3] + tier * 5,
        ];
    }
    Shield {
        name: shield.name,
        defense_bonus,
        dr,
        cover_value: shield.cover_value,
        breakage_thresholds,
        weight_lbs: shield.weight_lbs,
    }
}

fn breakage_steps_from_thresholds(thresholds: [i32; 4]) -> [sim::ShieldBreakageStep; 4] {
    [
        sim::ShieldBreakageStep {
            threshold: thresholds[0],
            save_mod: Some(6),
        },
        sim::ShieldBreakageStep {
            threshold: thresholds[1],
            save_mod: Some(0),
        },
        sim::ShieldBreakageStep {
            threshold: thresholds[2],
            save_mod: Some(-6),
        },
        sim::ShieldBreakageStep {
            threshold: thresholds[3],
            save_mod: None,
        },
    ]
}

pub fn is_ranged_weapon(weapon: &WeaponPreset) -> bool {
    weapon.range_bands_feet.is_some() || sim::max_range_for_weapon_name(&weapon.name).is_some()
}

pub fn uses_projectiles(weapon_name: &str, has_ammo: bool) -> bool {
    has_ammo || weapon_name == "Sling"
}

pub fn material_bonuses(
    weapon_tier: i32,
    projectile_tier: i32,
    is_ranged: bool,
    uses_projectiles: bool,
) -> (i32, i32) {
    let weapon_tier = weapon_tier.clamp(0, 5);
    let projectile_tier = projectile_tier.clamp(0, 5);
    if is_ranged && uses_projectiles {
        (projectile_tier, weapon_tier + projectile_tier)
    } else {
        (weapon_tier, weapon_tier)
    }
}

pub fn apply_armor_material_tier(mut armor: Armor, tier: i32) -> Armor {
    let tier = tier.clamp(0, 5);
    if tier > 0 {
        armor.damage_reduction += tier;
        if armor.defense_adj < 0 {
            armor.defense_adj = (armor.defense_adj + tier).min(0);
        }
    }
    armor
}

pub fn strength_damage_for_weapon(weapon: &WeaponPreset, base: i32) -> i32 {
    if is_ranged_weapon(weapon) && uses_projectiles(&weapon.name, weapon.ammunition.is_some()) {
        0
    } else {
        base
    }
}

pub fn base_weapon_threshold(group: WeaponGroup) -> f32 {
    match group {
        WeaponGroup::Bows | WeaponGroup::Crossbows => 150.0,
        WeaponGroup::Shields => 200.0,
        _ => 100.0,
    }
}

pub fn threshold_of_pain(max_hp: i32, level: u8) -> i32 {
    let pct = 0.30 + (level as f32 * 0.01);
    ((max_hp as f32) * pct).ceil() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character;

    fn sample_catalogs() -> (WeaponCatalog, ArmorCatalog, ShieldCatalog) {
        crate::data::load_catalogs().expect("Failed to load catalogs")
    }

    fn sample_talents() -> TalentCatalog {
        crate::data::load_talents(crate::data::TALENTS_PATH).expect("Failed to load talents")
    }

    fn sample_npc_presets() -> NpcPresetCatalog {
        crate::data::load_npc_presets("data/npc_presets.json").expect("Failed to load NPC presets")
    }

    fn one_handed_weapon_id(weapons: &WeaponCatalog) -> WeaponId {
        weapons
            .entries()
            .iter()
            .position(|weapon| weapon.handedness == WeaponHandedness::OneHanded)
            .and_then(|idx| weapons.id_from_index(idx))
            .unwrap_or(WeaponId::new(0))
    }

    fn one_handed_sword_weapon_id(weapons: &WeaponCatalog) -> WeaponId {
        weapons
            .entries()
            .iter()
            .position(is_one_handed_sword)
            .and_then(|idx| weapons.id_from_index(idx))
            .unwrap_or(WeaponId::new(0))
    }

    fn unarmed_weapon_id(weapons: &WeaponCatalog) -> WeaponId {
        weapons
            .entries()
            .iter()
            .position(|weapon| weapon.group == WeaponGroup::Unarmed)
            .and_then(|idx| weapons.id_from_index(idx))
            .unwrap_or(WeaponId::new(0))
    }

    fn jab_weapon_id(weapons: &WeaponCatalog) -> WeaponId {
        weapons
            .entries()
            .iter()
            .position(|weapon| weapon.jab_speed.is_some())
            .and_then(|idx| weapons.id_from_index(idx))
            .expect("No jab-capable weapon found")
    }

    fn non_jab_weapon_id(weapons: &WeaponCatalog) -> WeaponId {
        weapons
            .entries()
            .iter()
            .position(|weapon| weapon.jab_speed.is_none())
            .and_then(|idx| weapons.id_from_index(idx))
            .expect("No non-jab weapon found")
    }

    fn weapon_name(weapons: &WeaponCatalog, id: WeaponId) -> String {
        weapons
            .get(id)
            .map(|weapon| weapon.name.clone())
            .unwrap_or_else(|| "Fist".to_string())
    }

    fn weapon_id_by_group(weapons: &WeaponCatalog, group: WeaponGroup) -> WeaponId {
        weapons
            .entries()
            .iter()
            .position(|weapon| weapon.group == group)
            .and_then(|idx| weapons.id_from_index(idx))
            .expect("No weapon found for weapon group")
    }

    fn weapon_id_by_group_ranged(weapons: &WeaponCatalog, group: WeaponGroup) -> WeaponId {
        weapons
            .entries()
            .iter()
            .enumerate()
            .find(|(_, weapon)| weapon.group == group && is_ranged_weapon(weapon))
            .and_then(|(idx, _)| weapons.id_from_index(idx))
            .expect("No ranged weapon found for weapon group")
    }

    fn weapon_id_matching<F>(weapons: &WeaponCatalog, predicate: F) -> WeaponId
    where
        F: Fn(&WeaponPreset) -> bool,
    {
        weapons
            .entries()
            .iter()
            .enumerate()
            .find(|(_, weapon)| predicate(weapon))
            .and_then(|(idx, _)| weapons.id_from_index(idx))
            .expect("No matching weapon found")
    }

    fn base_player(weapon_id: WeaponId) -> PlayerConfig {
        let mut player = PlayerConfig::new("Test", weapon_id);
        player.level = 3;
        player.strength_base = 15;
        player.dex_base = 15;
        player.intelligence = 15;
        player.wisdom = 15;
        player.constitution = 15;
        player.looks = 15;
        player.charisma = 15;
        player.dex_pct = 1;
        player
    }

    #[test]
    fn combat_maneuvers_propagate_to_combatant_sheet() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = sample_npc_presets();
        let weapon_id = weapon_id_matching(&weapons, |weapon| {
            weapon.handedness == WeaponHandedness::OneHanded
                && !is_ranged_weapon(weapon)
                && !matches!(weapon.size, WeaponSize::Small)
        });
        let base = base_player(weapon_id);
        let build_maneuvers = |player: &PlayerConfig| {
            build_combatant(player, &weapons, &armor, &shields, &npc_presets, &talents)
                .sheet
                .maneuvers
        };

        let mut player = base.clone();
        player.hold_at_bay = true;
        assert!(build_maneuvers(&player).hold_at_bay);

        let mut player = base.clone();
        player.called_shot = true;
        assert!(build_maneuvers(&player).called_shot);

        let mut player = base.clone();
        player.power_attack = true;
        add_talent(&mut player, TALENT_ID_POWER_ATTACK, None);
        assert!(build_maneuvers(&player).power_attack);

        let mut player = base.clone();
        player.aggressive_attack = true;
        assert!(build_maneuvers(&player).aggressive_attack);

        let mut player = base.clone();
        player.charge = true;
        assert!(build_maneuvers(&player).charge);

        let mut player = base.clone();
        player.ready_against_charge = true;
        assert!(build_maneuvers(&player).ready_against_charge);

        let mut player = base.clone();
        player.tactical_move = true;
        assert!(build_maneuvers(&player).tactical_move);

        let mut player = base.clone();
        player.fight_defensively = true;
        player.fight_defensively_penalty = 6;
        let maneuvers = build_maneuvers(&player);
        assert!(maneuvers.fight_defensively);
        assert_eq!(maneuvers.fight_defensively_attack_penalty, 6);
        assert_eq!(maneuvers.fight_defensively_defense_bonus, 3);

        let mut player = base.clone();
        player.full_parry = true;
        assert!(build_maneuvers(&player).full_parry);

        let mut player = base.clone();
        player.give_ground = true;
        assert!(build_maneuvers(&player).give_ground);

        let mut player = base.clone();
        player.scamper_back = true;
        assert!(build_maneuvers(&player).scamper_back);

        let mut player = base.clone();
        player.fighting_withdrawal = true;
        assert!(build_maneuvers(&player).fighting_withdrawal);

        let mut player = base.clone();
        player.flee = true;
        assert!(build_maneuvers(&player).flee);

        let mut player = base.clone();
        player.mounted = true;
        assert!(build_maneuvers(&player).mounted);

        let mut player = base.clone();
        player.defensive_dualwielding = true;
        assert!(build_maneuvers(&player).defensive_dualwielding);

        let mut player = base.clone();
        player.offensive_dualwielding = true;
        player.offhand_weapon_id = Some(one_handed_weapon_id(&weapons));
        assert!(build_maneuvers(&player).offensive_dualwielding);
    }

    #[test]
    fn jab_toggle_requires_jab_weapon() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = sample_npc_presets();

        let mut jab_player = base_player(jab_weapon_id(&weapons));
        jab_player.use_jab = true;
        let jab_combatant = build_combatant(
            &jab_player,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert!(jab_combatant.sheet.offense.weapon.use_jab);

        let mut no_jab_player = base_player(non_jab_weapon_id(&weapons));
        no_jab_player.use_jab = true;
        let no_jab_combatant = build_combatant(
            &no_jab_player,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert!(!no_jab_combatant.sheet.offense.weapon.use_jab);
    }

    fn add_talent(player: &mut PlayerConfig, id: &str, weapon: Option<String>) {
        player.talents.push(TalentSelection {
            id: id.to_string(),
            rank: 1,
            weapon,
        });
    }

    #[test]
    fn fight_defensively_penalty_is_quantized_and_feat_adjusted() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = sample_npc_presets();
        let build_maneuvers = |player: &PlayerConfig| {
            build_combatant(player, &weapons, &armor, &shields, &npc_presets, &talents)
                .sheet
                .maneuvers
        };
        let weapon_id = one_handed_weapon_id(&weapons);

        let mut player = base_player(weapon_id);
        player.fight_defensively = true;
        player.fight_defensively_penalty = 1;
        let maneuvers = build_maneuvers(&player);
        assert_eq!(maneuvers.fight_defensively_attack_penalty, 2);
        assert_eq!(maneuvers.fight_defensively_defense_bonus, 1);

        player.fight_defensively_penalty = 5;
        let maneuvers = build_maneuvers(&player);
        assert_eq!(maneuvers.fight_defensively_attack_penalty, 6);
        assert_eq!(maneuvers.fight_defensively_defense_bonus, 3);

        player.fight_defensively_penalty = 12;
        let maneuvers = build_maneuvers(&player);
        assert_eq!(maneuvers.fight_defensively_attack_penalty, 8);
        assert_eq!(maneuvers.fight_defensively_defense_bonus, 4);

        let mut expertise_player = base_player(weapon_id);
        expertise_player.fight_defensively = true;
        expertise_player.fight_defensively_penalty = 8;
        add_talent(&mut expertise_player, "combat_expertise", None);
        let maneuvers = build_maneuvers(&expertise_player);
        assert_eq!(maneuvers.fight_defensively_attack_penalty, 4);
        assert_eq!(maneuvers.fight_defensively_defense_bonus, 4);

        let mut duelist_player = base_player(weapon_id);
        duelist_player.fight_defensively = true;
        duelist_player.fight_defensively_penalty = 6;
        add_talent(&mut duelist_player, "contender", None);
        add_talent(&mut duelist_player, "duelist", None);
        let maneuvers = build_maneuvers(&duelist_player);
        assert_eq!(maneuvers.fight_defensively_attack_penalty, 3);
        assert_eq!(maneuvers.fight_defensively_defense_bonus, 3);
    }

    #[test]
    fn called_shot_talents_adjust_maneuver_profile() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = sample_npc_presets();
        let build_maneuvers = |player: &PlayerConfig| {
            build_combatant(player, &weapons, &armor, &shields, &npc_presets, &talents)
                .sheet
                .maneuvers
        };
        let weapon_id = one_handed_weapon_id(&weapons);

        let mut baseline = base_player(weapon_id);
        baseline.called_shot = true;
        let maneuvers = build_maneuvers(&baseline);
        assert_eq!(maneuvers.called_shot_defense_bonus, 8);
        assert_eq!(maneuvers.called_shot_defense_penalty, 4);
        assert_eq!(
            maneuvers.called_shot_delay_profile,
            sim::CalledShotDelayProfile::Standard
        );
        assert!(!maneuvers.called_shot_deceptive_defender);

        let mut precision_combatant = baseline.clone();
        add_talent(
            &mut precision_combatant,
            TALENT_ID_PRECISION_COMBATANT,
            None,
        );
        let maneuvers = build_maneuvers(&precision_combatant);
        assert_eq!(maneuvers.called_shot_defense_bonus, 4);
        assert_eq!(maneuvers.called_shot_defense_penalty, 2);
        assert_eq!(
            maneuvers.called_shot_delay_profile,
            sim::CalledShotDelayProfile::PrecisionCombatant
        );

        let mut precision_aiming = baseline.clone();
        add_talent(&mut precision_aiming, TALENT_ID_PRECISION_AIMING, None);
        let maneuvers = build_maneuvers(&precision_aiming);
        assert_eq!(maneuvers.called_shot_defense_bonus, 4);
        assert_eq!(maneuvers.called_shot_defense_penalty, 4);
        assert_eq!(
            maneuvers.called_shot_delay_profile,
            sim::CalledShotDelayProfile::PrecisionAiming
        );

        let mut deceptive = baseline.clone();
        add_talent(&mut deceptive, TALENT_ID_DECEPTIVE_DEFENDER, None);
        let maneuvers = build_maneuvers(&deceptive);
        assert!(maneuvers.called_shot_deceptive_defender);

        let mut contender = baseline.clone();
        add_talent(&mut contender, TALENT_ID_CONTENDER, None);
        let maneuvers = build_maneuvers(&contender);
        assert_eq!(maneuvers.called_shot_defense_bonus, 4);
        assert_eq!(maneuvers.called_shot_defense_penalty, 2);
        assert_eq!(
            maneuvers.called_shot_delay_profile,
            sim::CalledShotDelayProfile::PrecisionCombatant
        );
        assert!(maneuvers.called_shot_deceptive_defender);

        let mut duelist = baseline.clone();
        add_talent(&mut duelist, TALENT_ID_CONTENDER, None);
        add_talent(&mut duelist, TALENT_ID_DUELIST, None);
        let maneuvers = build_maneuvers(&duelist);
        assert_eq!(maneuvers.called_shot_defense_bonus, 4);
        assert_eq!(maneuvers.called_shot_defense_penalty, 0);
        assert_eq!(
            maneuvers.called_shot_delay_profile,
            sim::CalledShotDelayProfile::PrecisionCombatant
        );
        assert!(maneuvers.called_shot_deceptive_defender);
    }

    #[test]
    fn power_attack_off_matches_baseline_even_with_talent() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = sample_npc_presets();
        let weapon_id = weapon_id_matching(&weapons, |weapon| {
            weapon.handedness == WeaponHandedness::OneHanded
                && !is_ranged_weapon(weapon)
                && !matches!(weapon.size, WeaponSize::Small)
        });
        let baseline = base_player(weapon_id);
        let mut player = baseline.clone();
        add_talent(&mut player, TALENT_ID_POWER_ATTACK, None);

        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);

        assert_eq!(
            summary.roll.attack_bonus,
            baseline_summary.roll.attack_bonus
        );
        assert_eq!(
            summary.roll.strength_damage,
            baseline_summary.roll.strength_damage
        );
        assert_eq!(
            combatant.sheet.offense.attack_bonus,
            baseline_combatant.sheet.offense.attack_bonus
        );
        assert_eq!(
            combatant.sheet.offense.strength_damage,
            baseline_combatant.sheet.offense.strength_damage
        );
        assert!(!combatant.sheet.maneuvers.power_attack);
    }

    #[test]
    fn power_attack_removes_positive_int_dex_attack_and_doubles_strength_damage() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = sample_npc_presets();
        let weapon_id = weapon_id_matching(&weapons, |weapon| {
            weapon.handedness == WeaponHandedness::OneHanded
                && !is_ranged_weapon(weapon)
                && !matches!(weapon.size, WeaponSize::Small)
        });
        let weapon = weapons.get(weapon_id).expect("missing weapon");
        let baseline = base_player(weapon_id);
        let mut player = baseline.clone();
        player.power_attack = true;
        add_talent(&mut player, TALENT_ID_POWER_ATTACK, None);

        let character = build_character(&player, &weapons, &armor, &shields, &talents);
        let expected_attack_penalty = positive_int_dex_attack_bonus(&character);
        let expected_damage_bonus =
            strength_damage_for_weapon(weapon, character.ability_mods.strength.damage);
        assert!(expected_attack_penalty > 0);
        assert!(expected_damage_bonus > 0);

        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        assert_eq!(
            baseline_summary.roll.attack_bonus - summary.roll.attack_bonus,
            expected_attack_penalty
        );
        assert_eq!(
            summary.roll.strength_damage - baseline_summary.roll.strength_damage,
            expected_damage_bonus
        );

        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(
            baseline_combatant.sheet.offense.attack_bonus - combatant.sheet.offense.attack_bonus,
            expected_attack_penalty
        );
        assert_eq!(
            combatant.sheet.offense.strength_damage
                - baseline_combatant.sheet.offense.strength_damage,
            expected_damage_bonus
        );
        assert!(combatant.sheet.maneuvers.power_attack);
    }

    #[test]
    fn power_attack_requires_talent_strength_and_eligible_melee_weapon() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = sample_npc_presets();
        let eligible_id = weapon_id_matching(&weapons, |weapon| {
            !is_ranged_weapon(weapon) && !matches!(weapon.size, WeaponSize::Small)
        });
        let small_melee_id = weapon_id_matching(&weapons, |weapon| {
            !is_ranged_weapon(weapon) && matches!(weapon.size, WeaponSize::Small)
        });
        let ranged_id = weapon_id_matching(&weapons, is_ranged_weapon);

        for (weapon_id, add_power_attack_talent, strength_base) in [
            (eligible_id, false, 15),
            (eligible_id, true, 12),
            (small_melee_id, true, 15),
            (ranged_id, true, 15),
        ] {
            let mut baseline = base_player(weapon_id);
            baseline.strength_base = strength_base;
            let mut player = baseline.clone();
            player.power_attack = true;
            if add_power_attack_talent {
                add_talent(&mut player, TALENT_ID_POWER_ATTACK, None);
            }

            let baseline_combatant = build_combatant(
                &baseline,
                &weapons,
                &armor,
                &shields,
                &npc_presets,
                &talents,
            );
            let combatant =
                build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
            assert_eq!(
                combatant.sheet.offense.attack_bonus,
                baseline_combatant.sheet.offense.attack_bonus
            );
            assert_eq!(
                combatant.sheet.offense.strength_damage,
                baseline_combatant.sheet.offense.strength_damage
            );
            assert!(!combatant.sheet.maneuvers.power_attack);
        }
    }

    #[test]
    fn power_attack_applies_to_eligible_offhand_attacks() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = sample_npc_presets();
        let weapon_id = weapon_id_matching(&weapons, |weapon| {
            weapon.handedness == WeaponHandedness::OneHanded
                && !is_ranged_weapon(weapon)
                && !matches!(weapon.size, WeaponSize::Small)
        });
        let weapon = weapons.get(weapon_id).expect("missing weapon");
        let mut baseline = base_player(weapon_id);
        baseline.offensive_dualwielding = true;
        baseline.offhand_weapon_id = Some(weapon_id);
        let mut player = baseline.clone();
        player.power_attack = true;
        add_talent(&mut player, TALENT_ID_POWER_ATTACK, None);

        let character = build_character(&player, &weapons, &armor, &shields, &talents);
        let expected_attack_penalty = positive_int_dex_attack_bonus(&character);
        let expected_damage_bonus =
            strength_damage_for_weapon(weapon, character.ability_mods.strength.damage);
        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let baseline_offhand = baseline_combatant
            .sheet
            .offense
            .offhand
            .as_ref()
            .expect("baseline offhand");
        let offhand = combatant
            .sheet
            .offense
            .offhand
            .as_ref()
            .expect("power attack offhand");

        assert_eq!(
            baseline_offhand.attack_bonus - offhand.attack_bonus,
            expected_attack_penalty
        );
        assert_eq!(
            offhand.strength_damage - baseline_offhand.strength_damage,
            expected_damage_bonus
        );
        assert!(combatant.sheet.maneuvers.power_attack);
    }

    #[test]
    fn called_shot_updates_player_summary_defense_penalty() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let (shield_id, _) = find_shield(&shields, |_| true);

        let mut baseline = base_player(weapon_id);
        baseline.shield_id = shield_id;
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);

        let mut called = baseline.clone();
        called.called_shot = true;
        let called_summary = player_summary(&called, &weapons, &armor, &shields, &talents);

        assert_eq!(
            called_summary.defense.melee_with_shield_dv,
            baseline_summary
                .defense
                .melee_with_shield_dv
                .map(|dv| dv - called_shot_defense_penalty_for_player(&called)),
        );
    }

    #[test]
    fn called_shot_delay_expr_reflects_weapon_mode_and_talents() {
        let (weapons, _armor, _shields) = sample_catalogs();
        let melee_player = base_player(one_handed_weapon_id(&weapons));
        assert_eq!(
            called_shot_delay_expr_for_player(&melee_player, false),
            "2d4p"
        );
        assert_eq!(
            called_shot_delay_expr_for_player(&melee_player, true),
            "1d4p"
        );

        let mut precision_combatant = melee_player.clone();
        add_talent(
            &mut precision_combatant,
            TALENT_ID_PRECISION_COMBATANT,
            None,
        );
        assert_eq!(
            called_shot_delay_expr_for_player(&precision_combatant, false),
            "1d4p"
        );
        assert_eq!(
            called_shot_delay_expr_for_player(&precision_combatant, true),
            "1d4p"
        );

        let mut precision_aiming = melee_player.clone();
        add_talent(&mut precision_aiming, TALENT_ID_PRECISION_AIMING, None);
        assert_eq!(
            called_shot_delay_expr_for_player(&precision_aiming, false),
            "1d2"
        );
        assert_eq!(
            called_shot_delay_expr_for_player(&precision_aiming, true),
            "1d2"
        );
    }

    #[test]
    fn called_shot_target_defense_bonus_depends_on_target_armor_type() {
        let (weapons, armor, _shields) = sample_catalogs();
        let weapon_id = one_handed_weapon_id(&weapons);
        let attacker = base_player(weapon_id);
        assert_eq!(
            called_shot_target_defense_bonuses_for_player(&attacker),
            (4, 8, 16)
        );

        let mut precision_attacker = attacker.clone();
        add_talent(&mut precision_attacker, TALENT_ID_PRECISION_COMBATANT, None);
        assert_eq!(
            called_shot_target_defense_bonuses_for_player(&precision_attacker),
            (2, 4, 8)
        );

        let (light_armor_id, _) = find_armor(&armor, |entry| entry.armor_type == ArmorType::Light);
        let (medium_armor_id, _) =
            find_armor(&armor, |entry| entry.armor_type == ArmorType::Medium);
        let (heavy_armor_id, _) = find_armor(&armor, |entry| entry.armor_type == ArmorType::Heavy);

        let mut target = base_player(weapon_id);
        target.armor_id = light_armor_id;
        assert_eq!(
            called_shot_target_defense_bonus_against_target(&attacker, &target, &armor),
            4
        );
        assert_eq!(
            called_shot_target_defense_bonus_against_target(&precision_attacker, &target, &armor),
            2
        );

        target.armor_id = medium_armor_id;
        assert_eq!(
            called_shot_target_defense_bonus_against_target(&attacker, &target, &armor),
            8
        );
        assert_eq!(
            called_shot_target_defense_bonus_against_target(&precision_attacker, &target, &armor),
            4
        );

        target.armor_id = heavy_armor_id;
        assert_eq!(
            called_shot_target_defense_bonus_against_target(&attacker, &target, &armor),
            16
        );
        assert_eq!(
            called_shot_target_defense_bonus_against_target(&precision_attacker, &target, &armor),
            8
        );

        target.npc_preset = Some(NpcPresetId::new(0));
        assert_eq!(
            called_shot_target_defense_bonus_against_target(&attacker, &target, &armor),
            8
        );
    }

    #[test]
    fn combat_expertise_halves_fight_defensively_attack_penalty_only() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = sample_npc_presets();
        let build_maneuvers = |player: &PlayerConfig| {
            build_combatant(player, &weapons, &armor, &shields, &npc_presets, &talents)
                .sheet
                .maneuvers
        };

        let weapon_id = one_handed_weapon_id(&weapons);
        let mut player = base_player(weapon_id);
        player.fight_defensively = true;
        player.fight_defensively_penalty = 8;

        let without_feat = build_maneuvers(&player);
        add_talent(&mut player, "combat_expertise", None);
        let with_feat = build_maneuvers(&player);

        assert_eq!(without_feat.fight_defensively_attack_penalty, 8);
        assert_eq!(without_feat.fight_defensively_defense_bonus, 4);
        assert_eq!(with_feat.fight_defensively_attack_penalty, 4);
        assert_eq!(with_feat.fight_defensively_defense_bonus, 4);
    }

    #[test]
    fn duelist_halves_fight_defensively_attack_penalty_without_combat_expertise() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = sample_npc_presets();
        let build_maneuvers = |player: &PlayerConfig| {
            build_combatant(player, &weapons, &armor, &shields, &npc_presets, &talents)
                .sheet
                .maneuvers
        };

        let weapon_id = one_handed_weapon_id(&weapons);
        let mut player = base_player(weapon_id);
        player.fight_defensively = true;
        player.fight_defensively_penalty = 6;
        add_talent(&mut player, "contender", None);
        add_talent(&mut player, "duelist", None);

        let with_duelist = build_maneuvers(&player);
        assert_eq!(with_duelist.fight_defensively_attack_penalty, 3);
        assert_eq!(with_duelist.fight_defensively_defense_bonus, 3);
    }

    #[test]
    fn fight_defensively_feat_attack_penalty_reduction_does_not_stack() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = sample_npc_presets();
        let build_maneuvers = |player: &PlayerConfig| {
            build_combatant(player, &weapons, &armor, &shields, &npc_presets, &talents)
                .sheet
                .maneuvers
        };

        let weapon_id = one_handed_weapon_id(&weapons);
        let mut player = base_player(weapon_id);
        player.fight_defensively = true;
        player.fight_defensively_penalty = 8;
        add_talent(&mut player, "combat_expertise", None);
        add_talent(&mut player, "contender", None);
        add_talent(&mut player, "duelist", None);

        let maneuvers = build_maneuvers(&player);
        assert_eq!(maneuvers.fight_defensively_attack_penalty, 4);
        assert_eq!(maneuvers.fight_defensively_defense_bonus, 4);
    }

    #[test]
    fn fight_defensively_updates_player_summary_effective_attack_and_defense() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let (shield_id, _) = find_shield(&shields, |_| true);

        let mut baseline = base_player(weapon_id);
        baseline.shield_id = shield_id;
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);

        let mut player = baseline.clone();
        player.fight_defensively = true;
        player.fight_defensively_penalty = 6;
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);

        assert_eq!(
            summary.roll.attack_bonus,
            baseline_summary.roll.attack_bonus - 6
        );
        assert_eq!(
            summary.defense.melee_with_shield_dv,
            baseline_summary
                .defense
                .melee_with_shield_dv
                .map(|dv| dv + 3),
        );
    }

    #[test]
    fn combat_expertise_updates_player_summary_fight_defensively_attack_penalty() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let (shield_id, _) = find_shield(&shields, |_| true);

        let mut baseline = base_player(weapon_id);
        baseline.shield_id = shield_id;
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);

        let mut player = baseline.clone();
        player.fight_defensively = true;
        player.fight_defensively_penalty = 8;
        add_talent(&mut player, "combat_expertise", None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);

        assert_eq!(
            summary.roll.attack_bonus,
            baseline_summary.roll.attack_bonus - 4
        );
        assert_eq!(
            summary.defense.melee_with_shield_dv,
            baseline_summary
                .defense
                .melee_with_shield_dv
                .map(|dv| dv + 4),
        );
    }

    fn find_armor_opt<F>(armor: &ArmorCatalog, predicate: F) -> Option<(ArmorId, Armor)>
    where
        F: Fn(&Armor) -> bool,
    {
        for (idx, entry) in armor.entries().iter().enumerate() {
            if let Some(armor) = entry.armor.as_ref() {
                if predicate(armor) {
                    return Some((ArmorId::new(idx), armor.clone()));
                }
            }
        }
        None
    }

    fn find_armor<F>(armor: &ArmorCatalog, predicate: F) -> (ArmorId, Armor)
    where
        F: Fn(&Armor) -> bool,
    {
        find_armor_opt(armor, predicate).expect("No armor matched predicate")
    }

    fn find_shield<F>(shields: &ShieldCatalog, predicate: F) -> (ShieldId, ShieldPreset)
    where
        F: Fn(&ShieldPreset) -> bool,
    {
        for (idx, entry) in shields.entries().iter().enumerate() {
            if let Some(shield) = entry.shield.as_ref() {
                if predicate(shield) {
                    return (ShieldId::new(idx), shield.clone());
                }
            }
        }
        panic!("No shield matched predicate");
    }

    fn weapon_group_label(group: WeaponGroup) -> &'static str {
        match group {
            WeaponGroup::Unarmed => "Unarmed",
            WeaponGroup::Axes => "Axes",
            WeaponGroup::Basic => "Basic",
            WeaponGroup::Blunt => "Blunt",
            WeaponGroup::Bows => "Bows",
            WeaponGroup::Crossbows => "Crossbows",
            WeaponGroup::Double => "Double",
            WeaponGroup::Ensnaring => "Ensnaring",
            WeaponGroup::Lashes => "Lashes",
            WeaponGroup::LargeSwords => "Large swords",
            WeaponGroup::SmallSwords => "Small swords",
            WeaponGroup::Polearms => "Polearms",
            WeaponGroup::Spears => "Spears",
            WeaponGroup::Shields => "Shields",
        }
    }

    fn find_talent_spec<'a>(talents: &'a TalentCatalog, talent_id: &str) -> Option<&'a TalentSpec> {
        talents
            .entries()
            .iter()
            .find(|talent| talent.id == talent_id)
    }

    fn default_weapon_selection_for_talent(
        spec: &TalentSpec,
        player: &PlayerConfig,
        weapons: &WeaponCatalog,
    ) -> Option<String> {
        if talent_requires_weapon_group(spec) {
            weapons
                .get(player.weapon_id)
                .map(|weapon| weapon_group_label(weapon.group).to_string())
        } else if talent_requires_weapon(spec) {
            Some(weapon_name(weapons, player.weapon_id))
        } else {
            None
        }
    }

    fn upsert_talent_selection(
        player: &mut PlayerConfig,
        talent_id: &str,
        rank: u8,
        weapon: Option<String>,
    ) {
        let desired_rank = rank.max(1);
        let weapon_key = weapon.clone();
        if let Some(existing) = player
            .talents
            .iter_mut()
            .find(|selection| selection.id == talent_id && selection.weapon == weapon_key)
        {
            existing.rank = existing.rank.max(desired_rank);
        } else {
            player.talents.push(TalentSelection {
                id: talent_id.to_string(),
                rank: desired_rank,
                weapon,
            });
        }
    }

    fn add_talent_with_requirements(
        player: &mut PlayerConfig,
        talents: &TalentCatalog,
        weapons: &WeaponCatalog,
        talent_id: &str,
        include_target: bool,
    ) {
        let Some(spec) = find_talent_spec(talents, talent_id).cloned() else {
            return;
        };
        for requirement in &spec.requirements {
            if let TalentRequirement::RequiresTalent { id, min_rank } = requirement {
                add_talent_with_requirements(player, talents, weapons, id, true);
                if let Some(required_spec) = find_talent_spec(talents, id) {
                    let required_weapon =
                        default_weapon_selection_for_talent(required_spec, player, weapons);
                    upsert_talent_selection(
                        player,
                        id,
                        min_rank.unwrap_or(1).max(1),
                        required_weapon,
                    );
                }
            }
        }
        if include_target {
            let weapon = default_weapon_selection_for_talent(&spec, player, weapons);
            upsert_talent_selection(player, &spec.id, 1, weapon);
        }
    }

    fn is_shared_defense_or_derived_effect(effect: &TalentEffect) -> bool {
        matches!(
            effect,
            TalentEffect::HitPointBonus { .. }
                | TalentEffect::ArmorDrBonus { .. }
                | TalentEffect::DefenseBonusWeapon { .. }
                | TalentEffect::Dodge { .. }
                | TalentEffect::ArmorDrBonusArmored { .. }
                | TalentEffect::LightArmorDefenseBonusFromDr { .. }
                | TalentEffect::MediumArmorDrBonus { .. }
                | TalentEffect::MediumArmorDefensePenaltyReduction { .. }
                | TalentEffect::ShieldDefenseBonus { .. }
                | TalentEffect::ShieldCoverValueAdjustment { .. }
        )
    }

    fn talent_has_shared_defense_or_derived_effect(spec: &TalentSpec) -> bool {
        spec.effects.iter().any(is_shared_defense_or_derived_effect)
    }

    fn spec_requires_light_armor(spec: &TalentSpec) -> bool {
        spec.effects
            .iter()
            .any(|effect| matches!(effect, TalentEffect::LightArmorDefenseBonusFromDr { .. }))
    }

    fn spec_requires_medium_armor(spec: &TalentSpec) -> bool {
        spec.effects.iter().any(|effect| {
            matches!(
                effect,
                TalentEffect::MediumArmorDrBonus { .. }
                    | TalentEffect::MediumArmorDefensePenaltyReduction { .. }
            )
        })
    }

    fn spec_requires_armored_bonus(spec: &TalentSpec) -> bool {
        spec.effects
            .iter()
            .any(|effect| matches!(effect, TalentEffect::ArmorDrBonusArmored { .. }))
    }

    fn spec_requires_shield(spec: &TalentSpec) -> bool {
        spec.effects.iter().any(|effect| {
            matches!(
                effect,
                TalentEffect::ShieldDefenseBonus { .. }
                    | TalentEffect::ShieldCoverValueAdjustment { .. }
            )
        })
    }

    fn find_weapon_for_speed_bonus<F>(
        weapons: &WeaponCatalog,
        armor: &ArmorCatalog,
        shields: &ShieldCatalog,
        talents: &TalentCatalog,
        base_player: &PlayerConfig,
        predicate: F,
    ) -> (WeaponId, WeaponPreset)
    where
        F: Fn(&WeaponPreset) -> bool,
    {
        let npc_presets = Catalog::new(Vec::new());
        for (idx, weapon) in weapons.entries().iter().enumerate() {
            if !predicate(weapon) {
                continue;
            }
            let mut player = base_player.clone();
            player.weapon_id = WeaponId::new(idx);
            let combatant =
                build_combatant(&player, weapons, armor, shields, &npc_presets, talents);
            let min_speed = weapon.size.min_speed();
            if combatant.sheet.offense.weapon.speed - 1.0 >= min_speed {
                return (WeaponId::new(idx), weapon.clone());
            }
        }
        panic!("No weapon matched predicate with speed margin");
    }

    #[test]
    fn talent_requirements_block_min_level() {
        let spec = TalentSpec {
            id: "requires_level".to_string(),
            name: "Requires Level".to_string(),
            description: "".to_string(),
            cost_bp: None,
            cost_lp: None,
            cost_rp: None,
            category: "Test".to_string(),
            race_categories: Vec::new(),
            race_ids: Vec::new(),
            requirements: vec![TalentRequirement::MinLevel { level: 3 }],
            max_rank: 1,
            effects: Vec::new(),
        };
        let stats = AbilitySet {
            strength: AbilityScore::new(10, 1),
            intelligence: 10,
            wisdom: 10,
            dexterity: AbilityScore::new(10, 1),
            constitution: 10,
            looks: 10,
            charisma: 10,
        };
        let context = TalentContext {
            level: 2,
            stats: &stats,
            talents: &[],
            proficiencies: &[],
            weapon_catalog: None,
        };
        let failures = evaluate_talent_requirements(&spec, &context);
        assert_eq!(
            failures,
            vec![TalentRequirementFailure::MinLevel {
                required: 3,
                current: 2
            }]
        );
    }

    #[test]
    fn talent_requirements_block_missing_stats() {
        let spec = TalentSpec {
            id: "requires_stats".to_string(),
            name: "Requires Stats".to_string(),
            description: "".to_string(),
            cost_bp: None,
            cost_lp: None,
            cost_rp: None,
            category: "Test".to_string(),
            race_categories: Vec::new(),
            race_ids: Vec::new(),
            requirements: vec![TalentRequirement::MinStat {
                stat: AbilityKind::Strength,
                min_base: Some(12),
                min_percentile: Some(51),
            }],
            max_rank: 1,
            effects: Vec::new(),
        };
        let stats = AbilitySet {
            strength: AbilityScore::new(10, 1),
            intelligence: 10,
            wisdom: 10,
            dexterity: AbilityScore::new(10, 1),
            constitution: 10,
            looks: 10,
            charisma: 10,
        };
        let context = TalentContext {
            level: 1,
            stats: &stats,
            talents: &[],
            proficiencies: &[],
            weapon_catalog: None,
        };
        let failures = evaluate_talent_requirements(&spec, &context);
        assert!(failures.contains(&TalentRequirementFailure::MinStatBase {
            stat: AbilityKind::Strength,
            required: 12,
            current: 10,
        }));
        assert!(
            failures.contains(&TalentRequirementFailure::MinStatPercentile {
                stat: AbilityKind::Strength,
                required: 51,
                current: Some(1),
            })
        );
    }

    #[test]
    fn talent_requirements_block_missing_prereq_talent() {
        let spec = TalentSpec {
            id: "requires_talent".to_string(),
            name: "Requires Talent".to_string(),
            description: "".to_string(),
            cost_bp: None,
            cost_lp: None,
            cost_rp: None,
            category: "Test".to_string(),
            race_categories: Vec::new(),
            race_ids: Vec::new(),
            requirements: vec![TalentRequirement::RequiresTalent {
                id: "prereq".to_string(),
                min_rank: Some(2),
            }],
            max_rank: 1,
            effects: Vec::new(),
        };
        let stats = AbilitySet {
            strength: AbilityScore::new(10, 1),
            intelligence: 10,
            wisdom: 10,
            dexterity: AbilityScore::new(10, 1),
            constitution: 10,
            looks: 10,
            charisma: 10,
        };
        let selections = vec![TalentSelection {
            id: "prereq".to_string(),
            rank: 1,
            weapon: None,
        }];
        let context = TalentContext {
            level: 1,
            stats: &stats,
            talents: &selections,
            proficiencies: &[],
            weapon_catalog: None,
        };
        let failures = evaluate_talent_requirements(&spec, &context);
        assert_eq!(
            failures,
            vec![TalentRequirementFailure::RequiresTalent {
                id: "prereq".to_string(),
                required_rank: 2,
                current_rank: 1
            }]
        );
    }

    #[test]
    fn talent_requirements_enforce_weapon_group_match() {
        let spec = TalentSpec {
            id: "weapon_specialization".to_string(),
            name: "Weapon Specialization".to_string(),
            description: "".to_string(),
            cost_bp: None,
            cost_lp: None,
            cost_rp: None,
            category: "Test".to_string(),
            race_categories: Vec::new(),
            race_ids: Vec::new(),
            requirements: vec![TalentRequirement::RequiresTalent {
                id: "weapon_focus".to_string(),
                min_rank: Some(1),
            }],
            max_rank: 1,
            effects: Vec::new(),
        };
        let stats = AbilitySet {
            strength: AbilityScore::new(10, 1),
            intelligence: 10,
            wisdom: 10,
            dexterity: AbilityScore::new(10, 1),
            constitution: 10,
            looks: 10,
            charisma: 10,
        };
        let context = TalentContext {
            level: 1,
            stats: &stats,
            talents: &[
                TalentSelection {
                    id: "weapon_focus".to_string(),
                    rank: 1,
                    weapon: Some("Axes".to_string()),
                },
                TalentSelection {
                    id: "weapon_specialization".to_string(),
                    rank: 1,
                    weapon: Some("Polearms".to_string()),
                },
            ],
            proficiencies: &[],
            weapon_catalog: None,
        };
        let failures = evaluate_talent_requirements(&spec, &context);
        assert_eq!(
            failures,
            vec![TalentRequirementFailure::RequiresTalent {
                id: "weapon_focus".to_string(),
                required_rank: 1,
                current_rank: 0
            }]
        );
    }

    #[test]
    fn talent_requirements_allow_matching_weapon_group() {
        let spec = TalentSpec {
            id: "weapon_specialization".to_string(),
            name: "Weapon Specialization".to_string(),
            description: "".to_string(),
            cost_bp: None,
            cost_lp: None,
            cost_rp: None,
            category: "Test".to_string(),
            race_categories: Vec::new(),
            race_ids: Vec::new(),
            requirements: vec![TalentRequirement::RequiresTalent {
                id: "weapon_focus".to_string(),
                min_rank: Some(1),
            }],
            max_rank: 1,
            effects: Vec::new(),
        };
        let stats = AbilitySet {
            strength: AbilityScore::new(10, 1),
            intelligence: 10,
            wisdom: 10,
            dexterity: AbilityScore::new(10, 1),
            constitution: 10,
            looks: 10,
            charisma: 10,
        };
        let context = TalentContext {
            level: 1,
            stats: &stats,
            talents: &[
                TalentSelection {
                    id: "weapon_focus".to_string(),
                    rank: 1,
                    weapon: Some("Axes".to_string()),
                },
                TalentSelection {
                    id: "weapon_specialization".to_string(),
                    rank: 1,
                    weapon: Some("Axes".to_string()),
                },
            ],
            proficiencies: &[],
            weapon_catalog: None,
        };
        let failures = evaluate_talent_requirements(&spec, &context);
        assert!(failures.is_empty());
    }

    #[test]
    fn material_bonuses_melee_use_weapon_tier() {
        let (attack, damage) = material_bonuses(2, 4, false, false);
        assert_eq!((attack, damage), (2, 2));
    }

    #[test]
    fn material_bonuses_ranged_projectile_use_ammo_for_attack() {
        let (attack, damage) = material_bonuses(2, 3, true, true);
        assert_eq!((attack, damage), (3, 5));
    }

    #[test]
    fn armor_material_increases_dr_and_reduces_penalty() {
        let armor = Armor {
            name: "Test".to_string(),
            price_gp: 0,
            region: character::ArmorRegion::Northern,
            damage_reduction: 4,
            defense_adj: -2,
            initiative_mod: 0,
            speed_mod: 0,
            armor_type: character::ArmorType::Light,
            weight_lbs: 10.0,
        };
        let adjusted = apply_armor_material_tier(armor, 3);
        assert_eq!(adjusted.damage_reduction, 7);
        assert_eq!(adjusted.defense_adj, 0);
    }

    #[test]
    fn raurosi_leather_and_manica_match_supplied_armor_stats() {
        let (_, armor, _) = sample_catalogs();
        let (_, raurosi) = find_armor(&armor, |entry| entry.name == "Raurosi Leather?");
        assert_eq!(raurosi.damage_reduction, 2);
        assert_eq!(raurosi.defense_adj, -1);
        assert_eq!(raurosi.initiative_mod, 1);
        assert_eq!(raurosi.speed_mod, 0);
        assert_eq!(raurosi.armor_type, character::ArmorType::Light);
        assert_eq!(raurosi.weight_lbs, 10.0);

        let adjusted_raurosi = apply_armor_material_tier(raurosi, 3);
        assert_eq!(adjusted_raurosi.damage_reduction, 5);
        assert_eq!(adjusted_raurosi.defense_adj, 0);
        assert_eq!(adjusted_raurosi.initiative_mod, 1);

        let (_, manica) = find_armor(&armor, |entry| entry.name == "Manica");
        assert_eq!(manica.damage_reduction, 3);
        assert_eq!(manica.defense_adj, -4);
        assert_eq!(manica.initiative_mod, 1);
        assert_eq!(manica.speed_mod, 0);
        assert_eq!(manica.armor_type, character::ArmorType::Light);
        assert_eq!(manica.weight_lbs, 15.0);
    }

    #[test]
    fn shield_material_increases_breakage_thresholds() {
        let shield = ShieldPreset {
            name: "Test".to_string(),
            price_gp: 0,
            defense_bonus: 4,
            dr: 4,
            cover_value: 16,
            breakage_thresholds: [6, 9, 12, 15],
            weight_lbs: 6.0,
        };
        let adjusted = apply_shield_material_tier(shield, 2);
        assert_eq!(adjusted.breakage_thresholds, [10, 15, 20, 25]);
    }

    #[test]
    fn mastery_attack_bonus_applies_to_roll() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = Catalog::new(Vec::new());
        let mut player = PlayerConfig::new("Test", WeaponId::new(0));
        player.weapon_id = one_handed_weapon_id(&weapons);
        player.mastery_attack = 3;
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.mastery_attack = 0;
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        assert_eq!(
            summary.roll.attack_bonus - baseline_summary.roll.attack_bonus,
            3
        );
    }

    #[test]
    fn mastery_damage_applies_to_roll() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = Catalog::new(Vec::new());
        let mut player = PlayerConfig::new("Test", WeaponId::new(0));
        player.weapon_id = one_handed_weapon_id(&weapons);
        player.mastery_damage = 4;
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.mastery_damage = 0;
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        assert_eq!(
            summary.roll.strength_damage - baseline_summary.roll.strength_damage,
            4
        );
    }

    #[test]
    fn mastery_defense_applies_without_shield() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = Catalog::new(Vec::new());
        let mut player = PlayerConfig::new("Test", WeaponId::new(0));
        player.weapon_id = one_handed_weapon_id(&weapons);
        player.mastery_defense = 2;
        let npc_presets = Catalog::new(Vec::new());
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline = player.clone();
        baseline.mastery_defense = 0;
        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert_eq!(
            combatant.sheet.defense.defense_mod - baseline_combatant.sheet.defense.defense_mod,
            2
        );
    }

    #[test]
    fn defensive_dualwielding_doubles_mastery_and_weapon_defense_bonus_only() {
        let (weapons, armor, shields) = sample_catalogs();
        let weapon_id = one_handed_weapon_id(&weapons);
        let weapon_name = weapons
            .get(weapon_id)
            .map(|weapon| weapon.name.clone())
            .unwrap_or_else(|| "Fist".to_string());
        let talents = Catalog::new(vec![
            TalentSpec {
                id: "defense_bonus_weapon".to_string(),
                name: "Defense Bonus (weapon)".to_string(),
                description: "".to_string(),
                cost_bp: None,
                cost_lp: None,
                cost_rp: None,
                category: "Test".to_string(),
                race_categories: Vec::new(),
                race_ids: Vec::new(),
                requirements: Vec::new(),
                max_rank: 1,
                effects: vec![TalentEffect::DefenseBonusWeapon { amount: 2 }],
            },
            TalentSpec {
                id: "dodge".to_string(),
                name: "Dodge".to_string(),
                description: "".to_string(),
                cost_bp: None,
                cost_lp: None,
                cost_rp: None,
                category: "Test".to_string(),
                race_categories: Vec::new(),
                race_ids: Vec::new(),
                requirements: Vec::new(),
                max_rank: 1,
                effects: vec![TalentEffect::Dodge {
                    defense_bonus: 1,
                    allow_dex_ranged: false,
                }],
            },
        ]);
        let mut player = PlayerConfig::new("Test", weapon_id);
        player.mastery_defense = 3;
        player.defensive_dualwielding = true;
        player.talents = vec![
            TalentSelection {
                id: "defense_bonus_weapon".to_string(),
                rank: 1,
                weapon: Some(weapon_name),
            },
            TalentSelection {
                id: "dodge".to_string(),
                rank: 1,
                weapon: None,
            },
        ];
        let npc_presets = Catalog::new(Vec::new());
        let dual = build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline = player.clone();
        baseline.defensive_dualwielding = false;
        let normal = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        let diff = dual.sheet.defense.defense_mod - normal.sheet.defense.defense_mod;
        assert_eq!(diff, player.mastery_defense + 2);
        assert_ne!(diff, player.mastery_defense + 3);
    }

    #[test]
    fn offensive_dualwielding_sets_base_dv_to_zero() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = Catalog::new(Vec::new());
        let weapon_id = one_handed_weapon_id(&weapons);
        let mut player = PlayerConfig::new("Test", weapon_id);
        player.offensive_dualwielding = true;
        player.offhand_weapon_id = Some(weapon_id);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        assert_eq!(summary.derived.base_dv, 0);
    }

    #[test]
    fn two_weapon_fighting_talents_adjust_offensive_dualwield_profile() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = sample_npc_presets();
        let weapon_id = one_handed_weapon_id(&weapons);
        let build_maneuvers = |player: &PlayerConfig| {
            build_combatant(player, &weapons, &armor, &shields, &npc_presets, &talents)
                .sheet
                .maneuvers
        };

        let mut baseline = base_player(weapon_id);
        baseline.offensive_dualwielding = true;
        baseline.offhand_weapon_id = Some(weapon_id);
        let maneuvers = build_maneuvers(&baseline);
        assert!(maneuvers.offensive_dualwielding);
        assert!(!maneuvers.defensive_dualwielding);
        assert!(maneuvers.offensive_dualwielding_defense_penalty);
        assert_eq!(maneuvers.dualwield_offhand_damage_penalty, -2);
        assert_eq!(maneuvers.dualwield_primary_recovery_penalty, 2.0);
        assert_eq!(maneuvers.dualwield_secondary_recovery_penalty, 2.0);

        let mut two_weapon = baseline.clone();
        add_talent(&mut two_weapon, TALENT_ID_TWO_WEAPON_FIGHTING, None);
        let maneuvers = build_maneuvers(&two_weapon);
        assert_eq!(maneuvers.dualwield_offhand_damage_penalty, 0);
        assert_eq!(maneuvers.dualwield_primary_recovery_penalty, 2.0);
        assert_eq!(maneuvers.dualwield_secondary_recovery_penalty, 2.0);

        let mut improved = baseline.clone();
        add_talent(&mut improved, TALENT_ID_TWO_WEAPON_FIGHTING, None);
        add_talent(&mut improved, TALENT_ID_IMPROVED_TWO_WEAPON_FIGHTING, None);
        let maneuvers = build_maneuvers(&improved);
        assert_eq!(maneuvers.dualwield_offhand_damage_penalty, 0);
        assert_eq!(maneuvers.dualwield_primary_recovery_penalty, 1.0);
        assert_eq!(maneuvers.dualwield_secondary_recovery_penalty, 2.0);

        let mut greater = baseline.clone();
        greater.level = 6;
        add_talent(&mut greater, TALENT_ID_TWO_WEAPON_FIGHTING, None);
        add_talent(&mut greater, TALENT_ID_IMPROVED_TWO_WEAPON_FIGHTING, None);
        add_talent(&mut greater, TALENT_ID_GREATER_TWO_WEAPON_FIGHTING, None);
        let maneuvers = build_maneuvers(&greater);
        assert_eq!(maneuvers.dualwield_offhand_damage_penalty, 0);
        assert_eq!(maneuvers.dualwield_primary_recovery_penalty, 1.0);
        assert_eq!(maneuvers.dualwield_secondary_recovery_penalty, 1.0);
    }

    #[test]
    fn perfect_two_weapon_fighting_combines_offense_and_defense_modes() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = sample_npc_presets();
        let weapon_id = one_handed_weapon_id(&weapons);

        let mut baseline = base_player(weapon_id);
        baseline.offensive_dualwielding = true;
        baseline.offhand_weapon_id = Some(weapon_id);
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);

        let mut perfect = baseline.clone();
        perfect.level = 9;
        add_talent(&mut perfect, TALENT_ID_TWO_WEAPON_FIGHTING, None);
        add_talent(&mut perfect, TALENT_ID_IMPROVED_TWO_WEAPON_FIGHTING, None);
        add_talent(&mut perfect, TALENT_ID_GREATER_TWO_WEAPON_FIGHTING, None);
        add_talent(&mut perfect, TALENT_ID_PERFECT_TWO_WEAPON_FIGHTING, None);
        let perfect_combatant =
            build_combatant(&perfect, &weapons, &armor, &shields, &npc_presets, &talents);
        let perfect_summary = player_summary(&perfect, &weapons, &armor, &shields, &talents);

        assert!(perfect_combatant.sheet.maneuvers.offensive_dualwielding);
        assert!(perfect_combatant.sheet.maneuvers.defensive_dualwielding);
        assert!(
            !perfect_combatant
                .sheet
                .maneuvers
                .offensive_dualwielding_defense_penalty
        );
        assert!(
            perfect_summary.derived.base_dv > baseline_summary.derived.base_dv,
            "perfect two-weapon fighting should preserve defensive value while offensively dual-wielding"
        );
        assert!(
            perfect_summary.defense.melee_roll_label.contains("d20p"),
            "perfect two-weapon fighting should keep d20p melee defense die"
        );
    }

    #[test]
    fn force_selected_perfect_two_weapon_fighting_combines_modes_below_level_nine() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = sample_npc_presets();
        let weapon_id = one_handed_weapon_id(&weapons);

        let mut player = base_player(weapon_id);
        player.level = 7;
        player.defensive_dualwielding = true;
        player.offensive_dualwielding = true;
        player.offhand_weapon_id = Some(weapon_id);
        add_talent(&mut player, TALENT_ID_TWO_WEAPON_FIGHTING, None);
        add_talent(&mut player, TALENT_ID_IMPROVED_TWO_WEAPON_FIGHTING, None);
        add_talent(&mut player, TALENT_ID_GREATER_TWO_WEAPON_FIGHTING, None);
        add_talent(&mut player, TALENT_ID_PERFECT_TWO_WEAPON_FIGHTING, None);

        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);

        assert!(combatant.sheet.maneuvers.offensive_dualwielding);
        assert!(combatant.sheet.maneuvers.defensive_dualwielding);
        assert!(
            !combatant
                .sheet
                .maneuvers
                .offensive_dualwielding_defense_penalty
        );
    }

    #[test]
    fn default_offhand_weapon_restores_primary_for_one_handed_dualwielding() {
        let (weapons, _armor, _shields) = sample_catalogs();
        let primary_id = one_handed_weapon_id(&weapons);
        let primary = weapons.get(primary_id).expect("missing primary weapon");
        let two_handed_id = weapon_id_matching(&weapons, |weapon| {
            weapon.handedness == WeaponHandedness::TwoHanded
        });
        let mut player = base_player(primary_id);

        assert_eq!(
            default_offhand_weapon_id(&player, primary, &weapons),
            Some(primary_id)
        );

        player.offhand_weapon_id = Some(two_handed_id);
        assert_eq!(
            default_offhand_weapon_id(&player, primary, &weapons),
            Some(primary_id)
        );

        player.two_hand_grip = true;
        assert_eq!(default_offhand_weapon_id(&player, primary, &weapons), None);
    }

    #[test]
    fn mastery_speed_reduces_weapon_speed() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = Catalog::new(Vec::new());
        let mut player = PlayerConfig::new("Test", WeaponId::new(0));
        player.weapon_id = one_handed_weapon_id(&weapons);
        player.mastery_speed = 3;
        let npc_presets = Catalog::new(Vec::new());
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline = player.clone();
        baseline.mastery_speed = 0;
        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert_eq!(
            baseline_combatant.sheet.offense.weapon.speed - combatant.sheet.offense.weapon.speed,
            3.0
        );
    }

    #[test]
    fn defensive_dualwielding_disables_free_hand_speed_bonus() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = Catalog::new(Vec::new());
        let base_player = PlayerConfig::new("Test", WeaponId::new(0));
        let (weapon_id, _weapon) = find_weapon_for_speed_bonus(
            &weapons,
            &armor,
            &shields,
            &talents,
            &base_player,
            |weapon| weapon.handedness == WeaponHandedness::OneHanded,
        );
        let npc_presets = Catalog::new(Vec::new());
        let mut baseline = base_player.clone();
        baseline.weapon_id = weapon_id;
        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        let mut dual = baseline.clone();
        dual.defensive_dualwielding = true;
        let dual_combatant =
            build_combatant(&dual, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(
            dual_combatant.sheet.offense.weapon.speed,
            baseline_combatant.sheet.offense.weapon.speed + 1.0
        );
    }

    #[test]
    fn shield_mastery_defense_overrides_weapon_mastery() {
        let (weapons, _armor, _shields) = sample_catalogs();
        let mut player = PlayerConfig::new("Test", WeaponId::new(0));
        player.weapon_id = one_handed_weapon_id(&weapons);
        player.mastery_defense = 5;
        player.shield_mastery_defense = 1;
        player.shield_id = ShieldId::new(1);
        let weapon = weapons
            .get(player.weapon_id)
            .unwrap_or_else(|| weapons.entries().first().expect("weapon catalog empty"));
        let mastery = effective_defense_mastery(&player, weapon);
        assert_eq!(mastery, 1);
    }

    #[test]
    fn shield_mastery_speed_uses_lower_when_shielded() {
        let (weapons, _armor, _shields) = sample_catalogs();
        let mut player = PlayerConfig::new("Test", WeaponId::new(0));
        player.weapon_id = one_handed_weapon_id(&weapons);
        player.mastery_speed = 5;
        player.shield_mastery_speed = 2;
        player.shield_id = ShieldId::new(1);
        let weapon = weapons
            .get(player.weapon_id)
            .unwrap_or_else(|| weapons.entries().first().expect("weapon catalog empty"));
        let mastery = effective_speed_mastery(&player, weapon);
        assert_eq!(mastery, 2);
    }

    #[test]
    fn shield_mastery_speed_ignored_without_shield() {
        let (weapons, _armor, _shields) = sample_catalogs();
        let mut player = PlayerConfig::new("Test", WeaponId::new(0));
        player.weapon_id = one_handed_weapon_id(&weapons);
        player.mastery_speed = 4;
        player.shield_mastery_speed = 1;
        player.shield_id = ShieldId::new(0);
        let weapon = weapons
            .get(player.weapon_id)
            .unwrap_or_else(|| weapons.entries().first().expect("weapon catalog empty"));
        let mastery = effective_speed_mastery(&player, weapon);
        assert_eq!(mastery, 4);
    }

    #[test]
    fn talent_hit_point_bonus_increases_max_hp() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let npc_presets = Catalog::new(Vec::new());
        for (talent_id, bonus) in [("hit_point_bonus", 2), ("hearty", 5), ("solid", 8)] {
            let mut player = base_player(weapon_id);
            add_talent(&mut player, talent_id, None);
            let with_bonus =
                build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
            let mut baseline = player.clone();
            baseline.talents.clear();
            let without_bonus = build_combatant(
                &baseline,
                &weapons,
                &armor,
                &shields,
                &npc_presets,
                &talents,
            );
            assert_eq!(
                with_bonus.sheet.vitals.max_hp - without_bonus.sheet.vitals.max_hp,
                bonus,
                "{talent_id} should add {bonus} hp"
            );
        }
    }

    #[test]
    fn confluence_scales_hp_and_records_drain_resistance() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let npc_presets = Catalog::new(Vec::new());
        let confluence = talents
            .entries()
            .iter()
            .find(|talent| talent.id == "essence_health_bonus")
            .expect("missing Confluence talent");
        assert_eq!(confluence.name, "Confluence");

        for (level, hp_bonus, drain_resistance) in [
            (1, 1, 0),
            (4, 1, 0),
            (5, 3, 2),
            (7, 3, 2),
            (8, 4, 4),
            (10, 4, 4),
            (11, 6, 6),
            (13, 6, 6),
            (14, 7, 8),
            (16, 7, 8),
            (17, 9, 10),
            (20, 9, 10),
        ] {
            let mut player = base_player(weapon_id);
            player.level = level;
            add_talent(&mut player, "essence_health_bonus", None);
            let with_confluence =
                build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
            let summary = player_summary(&player, &weapons, &armor, &shields, &talents);

            let mut baseline = player.clone();
            baseline.talents.clear();
            let without_confluence = build_combatant(
                &baseline,
                &weapons,
                &armor,
                &shields,
                &npc_presets,
                &talents,
            );

            assert_eq!(
                with_confluence.sheet.vitals.max_hp - without_confluence.sheet.vitals.max_hp,
                hp_bonus,
                "level {level} should add {hp_bonus} hp"
            );
            assert_eq!(
                with_confluence.sheet.vitals.drain_resistance, drain_resistance,
                "level {level} should record {drain_resistance} drain resistance"
            );
            assert_eq!(
                summary.derived.drain_resistance, drain_resistance,
                "level {level} summary should record {drain_resistance} drain resistance"
            );
        }
    }

    #[test]
    fn talent_tough_as_nails_overrides_trauma_die() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let mut player = base_player(weapon_id);
        add_talent(&mut player, "tough_as_nails", None);
        let npc_presets = Catalog::new(Vec::new());
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(combatant.sheet.vitals.trauma_die_sides, 12);
        assert!(combatant.sheet.vitals.trauma_die_penetrating);
    }

    #[test]
    fn talent_tough_hide_increases_armor_dr() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let npc_presets = Catalog::new(Vec::new());
        let mut player = base_player(weapon_id);
        add_talent(&mut player, "tough_hide", None);
        let with_bonus =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let without_bonus = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert_eq!(
            with_bonus.sheet.defense.armor_dr - without_bonus.sheet.defense.armor_dr,
            1
        );
    }

    #[test]
    fn talent_attack_bonus_weapon_increases_attack() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let weapon_label = weapon_name(&weapons, weapon_id);
        for talent_id in ["attack_bonus_weapon", "crack_shot"] {
            let mut player = base_player(weapon_id);
            add_talent(&mut player, talent_id, Some(weapon_label.clone()));
            let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
            let mut baseline = player.clone();
            baseline.talents.clear();
            let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
            assert_eq!(
                summary.roll.attack_bonus - baseline_summary.roll.attack_bonus,
                1,
                "{talent_id} should add +1 attack"
            );
        }
    }

    #[test]
    fn talent_damage_bonus_weapon_increases_damage() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let weapon_label = weapon_name(&weapons, weapon_id);
        let mut player = base_player(weapon_id);
        add_talent(&mut player, "damage_bonus_weapon", Some(weapon_label));
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        assert_eq!(
            summary.roll.strength_damage - baseline_summary.roll.strength_damage,
            1
        );
    }

    #[test]
    fn talent_defense_bonus_weapon_increases_defense() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let weapon_label = weapon_name(&weapons, weapon_id);
        let mut player = base_player(weapon_id);
        add_talent(&mut player, "defense_bonus_weapon", Some(weapon_label));
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        assert_eq!(
            summary.derived.base_dv - baseline_summary.derived.base_dv,
            1
        );
    }

    #[test]
    fn talent_dodge_grants_defense_and_ranged_bonus() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let npc_presets = Catalog::new(Vec::new());
        let mut player = base_player(weapon_id);
        add_talent(&mut player, "dodge", None);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert_eq!(
            combatant.sheet.defense.defense_mod - baseline_combatant.sheet.defense.defense_mod,
            1
        );
        let character = build_character(&baseline, &weapons, &armor, &shields, &talents);
        let expected_ranged = character.ability_mods.dexterity.defense + 1;
        assert_eq!(baseline_combatant.sheet.defense.ranged_defense_mod, 0);
        assert_eq!(combatant.sheet.defense.ranged_defense_mod, expected_ranged);
    }

    #[test]
    fn talent_advanced_sighting_scales_range_distance() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let npc_presets = Catalog::new(Vec::new());
        let mut player = base_player(weapon_id);
        add_talent(&mut player, "advanced_sighting", None);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let multiplier = combatant.sheet.offense.weapon.range_distance_multiplier;
        assert!((multiplier - 0.666).abs() < 0.0001);
    }

    #[test]
    fn talent_armor_focus_removes_initiative_penalty() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let Some((armor_id, armor_entry)) = find_armor_opt(&armor, |a| a.initiative_mod < 0) else {
            return;
        };
        let mut player = base_player(weapon_id);
        player.armor_id = armor_id;
        add_talent(&mut player, "armor_focus", None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        let expected_delta = -armor_entry.initiative_mod;
        assert_eq!(
            summary.derived.initiative_mod - baseline_summary.derived.initiative_mod,
            expected_delta
        );
    }

    #[test]
    fn talent_armor_specialization_removes_speed_penalty() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let Some((armor_id, armor_entry)) = find_armor_opt(&armor, |a| a.speed_mod != 0) else {
            return;
        };
        let mut player = base_player(weapon_id);
        player.armor_id = armor_id;
        add_talent(&mut player, "armor_focus", None);
        add_talent(&mut player, "armor_specialization", None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.talents = vec![TalentSelection {
            id: "armor_focus".to_string(),
            rank: 1,
            weapon: None,
        }];
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        let expected_delta = -armor_entry.speed_mod;
        assert_eq!(
            summary.derived.speed_mod - baseline_summary.derived.speed_mod,
            expected_delta
        );
    }

    #[test]
    fn talent_armor_master_increases_dr_when_armored() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let (armor_id, _armor_entry) = find_armor(&armor, |a| a.damage_reduction > 0);
        let mut player = base_player(weapon_id);
        player.armor_id = armor_id;
        add_talent(&mut player, "armor_focus", None);
        add_talent(&mut player, "armor_specialization", None);
        add_talent(&mut player, "armor_master", None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.talents = vec![
            TalentSelection {
                id: "armor_focus".to_string(),
                rank: 1,
                weapon: None,
            },
            TalentSelection {
                id: "armor_specialization".to_string(),
                rank: 1,
                weapon: None,
            },
        ];
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        assert_eq!(
            summary.derived.armor_dr - baseline_summary.derived.armor_dr,
            1
        );
    }

    #[test]
    fn talent_heavy_armor_optimization_adds_damage_from_dr() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let (armor_id, armor_entry) = find_armor(&armor, |a| {
            matches!(a.armor_type, ArmorType::Heavy) && a.damage_reduction > 0
        });
        let mut player = base_player(weapon_id);
        player.armor_id = armor_id;
        add_talent(&mut player, "heavy_armor_optimization", None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        let expected_bonus = armor_entry.damage_reduction / 4;
        assert_eq!(
            summary.roll.strength_damage - baseline_summary.roll.strength_damage,
            expected_bonus
        );
        let npc_presets = Catalog::new(Vec::new());
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(
            combatant
                .sheet
                .modifiers
                .apply_i32(0, sim::StatIdI32::FlagIgnoreAncillaryCritEffects),
            1
        );
    }

    #[test]
    fn talent_heavy_armor_optimization_sets_crit_immunity_flag_without_heavy_armor() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let (armor_id, _) = find_armor(&armor, |a| matches!(a.armor_type, ArmorType::Medium));
        let mut player = base_player(weapon_id);
        player.armor_id = armor_id;
        add_talent(&mut player, "heavy_armor_optimization", None);
        let npc_presets = Catalog::new(Vec::new());
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(
            combatant
                .sheet
                .modifiers
                .apply_i32(0, sim::StatIdI32::FlagIgnoreAncillaryCritEffects),
            1
        );
    }

    #[test]
    fn talent_improved_reach_adds_reach() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let weapon_label = weapon_name(&weapons, weapon_id);
        let npc_presets = Catalog::new(Vec::new());
        let mut player = base_player(weapon_id);
        add_talent(&mut player, "improved_reach", Some(weapon_label));
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert_eq!(
            combatant.sheet.offense.weapon.reach_ft
                - baseline_combatant.sheet.offense.weapon.reach_ft,
            1.0
        );
    }

    #[test]
    fn talent_light_armor_optimization_adds_defense_from_dr() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let (armor_id, armor_entry) = find_armor(&armor, |a| {
            matches!(a.armor_type, ArmorType::Light) && a.damage_reduction > 0
        });
        let mut player = base_player(weapon_id);
        player.armor_id = armor_id;
        add_talent(&mut player, "light_armor_optimization", None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        let expected_bonus = armor_entry.damage_reduction / 2;
        assert_eq!(
            summary.derived.base_dv - baseline_summary.derived.base_dv,
            expected_bonus
        );
        let npc_presets = Catalog::new(Vec::new());
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(
            combatant
                .sheet
                .modifiers
                .apply_i32(0, sim::StatIdI32::FlagIncomingCritExtraDamageHalved),
            1
        );
    }

    #[test]
    fn talent_medium_armor_optimization_adds_dr_and_defense() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let (armor_id, armor_entry) = find_armor(&armor, |a| {
            matches!(a.armor_type, ArmorType::Medium) && a.defense_adj < 0
        });
        let mut player = base_player(weapon_id);
        player.armor_id = armor_id;
        add_talent(&mut player, "medium_armor_optimization", None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        let expected_defense_bonus = (-armor_entry.defense_adj).min(1);
        assert_eq!(
            summary.derived.armor_dr - baseline_summary.derived.armor_dr,
            1
        );
        assert_eq!(
            summary.derived.base_dv - baseline_summary.derived.base_dv,
            expected_defense_bonus
        );
        let npc_presets = Catalog::new(Vec::new());
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(
            combatant
                .sheet
                .modifiers
                .apply_i32(0, sim::StatIdI32::IncomingCritSeverityReduction),
            10
        );
    }

    #[test]
    fn talent_shield_focus_increases_shield_defense() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let npc_presets = Catalog::new(Vec::new());
        let (shield_id, _shield_entry) = find_shield(&shields, |_shield| true);
        let mut player = base_player(weapon_id);
        player.shield_id = shield_id;
        add_talent(&mut player, "shield_focus", None);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert_eq!(
            combatant.sheet.defense.shield_defense_bonus
                - baseline_combatant.sheet.defense.shield_defense_bonus,
            1
        );
    }

    #[test]
    fn talent_shield_specialization_adjusts_cover_value() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let npc_presets = Catalog::new(Vec::new());
        let (shield_id, _shield_entry) = find_shield(&shields, |shield| shield.cover_value > 0);
        let mut player = base_player(weapon_id);
        player.shield_id = shield_id;
        add_talent(&mut player, "shield_focus", None);
        let mut with_specialization = player.clone();
        add_talent(&mut with_specialization, "shield_specialization", None);
        let baseline = build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let specialized = build_combatant(
            &with_specialization,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        let baseline_cover = baseline.sheet.defense.shield_cover_value.unwrap_or(0);
        let expected_cover = (baseline_cover - 5).max(0);
        assert_eq!(
            specialized.sheet.defense.shield_cover_value.unwrap_or(0),
            expected_cover
        );
    }

    #[test]
    fn shared_defense_and_derived_talent_deltas_match_between_summary_and_combatant() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = one_handed_weapon_id(&weapons);
        let (shield_id, _) = find_shield(&shields, |_shield| true);
        let (light_armor_id, _) = find_armor(&armor, |entry| {
            matches!(entry.armor_type, ArmorType::Light) && entry.damage_reduction > 0
        });
        let (medium_armor_id, _) = find_armor(&armor, |entry| {
            matches!(entry.armor_type, ArmorType::Medium)
                && entry.damage_reduction > 0
                && entry.defense_adj < 0
        });
        let (armored_armor_id, _) = find_armor(&armor, |entry| entry.damage_reduction > 0);
        let target_specs: Vec<&TalentSpec> = talents
            .entries()
            .iter()
            .filter(|spec| talent_has_shared_defense_or_derived_effect(spec))
            .collect();
        assert!(!target_specs.is_empty(), "No defense/derived talents found");

        for spec in target_specs {
            let mut player = base_player(weapon_id);
            player.level = 20;
            player.strength_base = 25;
            player.strength_pct = 100;
            player.dex_base = 25;
            player.dex_pct = 100;
            player.intelligence = 25;
            player.wisdom = 25;
            player.constitution = 25;
            player.looks = 25;
            player.charisma = 25;
            player.environment.natural_surroundings = true;
            if spec_requires_shield(spec) {
                player.shield_id = shield_id;
            }
            if spec_requires_light_armor(spec) {
                player.armor_id = light_armor_id;
            } else if spec_requires_medium_armor(spec) {
                player.armor_id = medium_armor_id;
            } else if spec_requires_armored_bonus(spec) {
                player.armor_id = armored_armor_id;
            }

            let mut baseline = player.clone();
            add_talent_with_requirements(&mut player, &talents, &weapons, &spec.id, true);
            add_talent_with_requirements(&mut baseline, &talents, &weapons, &spec.id, false);

            let stats = ability_set_from_player(&player);
            let context = TalentContext {
                level: player.level,
                stats: &stats,
                talents: &player.talents,
                proficiencies: &player.proficiencies,
                weapon_catalog: Some(&weapons),
            };
            assert!(
                evaluate_talent_requirements(spec, &context).is_empty(),
                "{} requirements should be satisfied in parity test",
                spec.id
            );

            let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
            let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
            let combatant =
                build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
            let baseline_combatant = build_combatant(
                &baseline,
                &weapons,
                &armor,
                &shields,
                &npc_presets,
                &talents,
            );

            let hp_delta_summary =
                summary.derived.hit_points as i32 - baseline_summary.derived.hit_points as i32;
            let hp_delta_combatant =
                combatant.sheet.vitals.max_hp - baseline_combatant.sheet.vitals.max_hp;
            assert_eq!(
                hp_delta_summary, hp_delta_combatant,
                "{} hp delta mismatch",
                spec.id
            );

            let armor_delta_summary = summary.derived.armor_dr - baseline_summary.derived.armor_dr;
            let armor_delta_combatant =
                combatant.sheet.defense.armor_dr - baseline_combatant.sheet.defense.armor_dr;
            assert_eq!(
                armor_delta_summary, armor_delta_combatant,
                "{} armor DR delta mismatch",
                spec.id
            );

            let dv_delta_summary = summary.derived.base_dv - baseline_summary.derived.base_dv;
            let dv_delta_combatant =
                combatant.sheet.defense.defense_mod - baseline_combatant.sheet.defense.defense_mod;
            assert_eq!(
                dv_delta_summary, dv_delta_combatant,
                "{} defense delta mismatch",
                spec.id
            );

            let shield_delta_summary = summary.defense.shield_bonus.unwrap_or(0)
                - baseline_summary.defense.shield_bonus.unwrap_or(0);
            let shield_delta_combatant = combatant.sheet.defense.shield_defense_bonus
                - baseline_combatant.sheet.defense.shield_defense_bonus;
            assert_eq!(
                shield_delta_summary, shield_delta_combatant,
                "{} shield defense delta mismatch",
                spec.id
            );

            let cover_delta_summary = summary.defense.shield_cover_value.unwrap_or(0)
                - baseline_summary.defense.shield_cover_value.unwrap_or(0);
            let cover_delta_combatant = combatant.sheet.defense.shield_cover_value.unwrap_or(0)
                - baseline_combatant
                    .sheet
                    .defense
                    .shield_cover_value
                    .unwrap_or(0);
            assert_eq!(
                cover_delta_summary, cover_delta_combatant,
                "{} shield cover delta mismatch",
                spec.id
            );

            assert!(
                hp_delta_summary != 0
                    || armor_delta_summary != 0
                    || dv_delta_summary != 0
                    || shield_delta_summary != 0
                    || cover_delta_summary != 0,
                "{} had no shared defense/derived delta",
                spec.id
            );
        }
    }

    #[test]
    fn twelve_paths_requires_large_sword_and_shield_proficiencies() {
        let (weapons, _armor, _shields) = sample_catalogs();
        let talents = sample_talents();
        let stats = AbilitySet {
            strength: AbilityScore::new(10, 1),
            intelligence: 10,
            wisdom: 10,
            dexterity: AbilityScore::new(10, 1),
            constitution: 10,
            looks: 10,
            charisma: 10,
        };
        let spec = find_talent_spec(&talents, TALENT_ID_TWELVE_PATHS)
            .expect("twelve_paths talent missing");
        let context_missing = TalentContext {
            level: 1,
            stats: &stats,
            talents: &[],
            proficiencies: &[],
            weapon_catalog: Some(&weapons),
        };
        let failures_missing = evaluate_talent_requirements(spec, &context_missing);
        assert!(
            failures_missing.contains(&TalentRequirementFailure::MissingSizeLLargeSwordProficiency)
        );
        assert!(failures_missing.contains(&TalentRequirementFailure::MissingShieldProficiency));

        let large_sword_name = weapons
            .entries()
            .iter()
            .find(|weapon| {
                weapon.group == WeaponGroup::LargeSwords && weapon.size == WeaponSize::Large
            })
            .map(|weapon| weapon.name.clone())
            .expect("no size L large sword found");
        let prof_large_sword_only = vec![large_sword_name.clone()];
        let context_large_only = TalentContext {
            level: 1,
            stats: &stats,
            talents: &[],
            proficiencies: &prof_large_sword_only,
            weapon_catalog: Some(&weapons),
        };
        let failures_large_only = evaluate_talent_requirements(spec, &context_large_only);
        assert!(
            !failures_large_only
                .contains(&TalentRequirementFailure::MissingSizeLLargeSwordProficiency)
        );
        assert!(failures_large_only.contains(&TalentRequirementFailure::MissingShieldProficiency));

        let prof_full = vec![large_sword_name, "Shields".to_string()];
        let context_full = TalentContext {
            level: 1,
            stats: &stats,
            talents: &[],
            proficiencies: &prof_full,
            weapon_catalog: Some(&weapons),
        };
        let failures_full = evaluate_talent_requirements(spec, &context_full);
        assert!(failures_full.is_empty());
    }

    #[test]
    fn twelve_paths_allows_size_l_large_sword_with_small_shield() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = weapons
            .entries()
            .iter()
            .enumerate()
            .find(|(_, weapon)| {
                weapon.group == WeaponGroup::LargeSwords && weapon.size == WeaponSize::Large
            })
            .and_then(|(idx, _)| weapons.id_from_index(idx))
            .expect("no size L large sword found");
        let (shield_id, _shield) = find_shield(&shields, |shield| {
            is_small_shield_or_buckler_name(&shield.name)
        });

        let mut player = base_player(weapon_id);
        player.shield_id = shield_id;
        player.proficiencies = vec!["Large swords".to_string(), "Shields".to_string()];
        let baseline = build_character(&player, &weapons, &armor, &shields, &talents);
        assert!(baseline.equipment.shield.is_none());

        add_talent(&mut player, TALENT_ID_TWELVE_PATHS, None);
        let styled = build_character(&player, &weapons, &armor, &shields, &talents);
        assert!(styled.equipment.shield.is_some());
    }

    #[test]
    fn shield_option_allows_twelve_paths_with_named_large_sword_proficiency() {
        let (weapons, _armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = weapons
            .entries()
            .iter()
            .enumerate()
            .find(|(_, weapon)| weapon.name.eq_ignore_ascii_case("Flamberge"))
            .and_then(|(idx, _)| weapons.id_from_index(idx))
            .expect("no flamberge found");
        let (_shield_id, shield) = find_shield(&shields, |shield| {
            is_small_shield_or_buckler_name(&shield.name)
        });
        let mut player = base_player(weapon_id);
        player.proficiencies = vec!["Flamberge".to_string(), "Shields".to_string()];
        add_talent(&mut player, TALENT_ID_TWELVE_PATHS, None);
        let weapon = weapons
            .get(player.weapon_id)
            .expect("selected weapon missing");
        assert!(shield_option_allowed(
            &player,
            weapon,
            Some(&shield),
            &talents,
            &weapons,
        ));
    }

    #[test]
    fn shield_option_allows_twelve_paths_without_proficiencies_when_selected() {
        let (weapons, _armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = weapons
            .entries()
            .iter()
            .enumerate()
            .find(|(_, weapon)| weapon.name.eq_ignore_ascii_case("Flamberge"))
            .and_then(|(idx, _)| weapons.id_from_index(idx))
            .expect("no flamberge found");
        let (_shield_id, shield) = find_shield(&shields, |shield| {
            is_small_shield_or_buckler_name(&shield.name)
        });
        let mut player = base_player(weapon_id);
        add_talent(&mut player, TALENT_ID_TWELVE_PATHS, None);
        let weapon = weapons
            .get(player.weapon_id)
            .expect("selected weapon missing");
        assert!(shield_option_allowed(
            &player,
            weapon,
            Some(&shield),
            &talents,
            &weapons,
        ));
    }

    #[test]
    fn twelve_paths_limits_shields_to_small_or_buckler() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = weapons
            .entries()
            .iter()
            .enumerate()
            .find(|(_, weapon)| {
                weapon.group == WeaponGroup::LargeSwords && weapon.size == WeaponSize::Large
            })
            .and_then(|(idx, _)| weapons.id_from_index(idx))
            .expect("no size L large sword found");
        let (shield_id, _shield) = find_shield(&shields, |shield| {
            !is_small_shield_or_buckler_name(&shield.name)
        });

        let mut player = base_player(weapon_id);
        player.shield_id = shield_id;
        player.proficiencies = vec!["Large swords".to_string(), "Shields".to_string()];
        add_talent(&mut player, TALENT_ID_TWELVE_PATHS, None);
        let styled = build_character(&player, &weapons, &armor, &shields, &talents);
        assert!(styled.equipment.shield.is_none());
    }

    #[test]
    fn twelve_paths_applies_damage_penalty_when_active() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = weapons
            .entries()
            .iter()
            .enumerate()
            .find(|(_, weapon)| {
                weapon.group == WeaponGroup::LargeSwords && weapon.size == WeaponSize::Large
            })
            .and_then(|(idx, _)| weapons.id_from_index(idx))
            .expect("no size L large sword found");
        let (shield_id, _shield) = find_shield(&shields, |shield| {
            is_small_shield_or_buckler_name(&shield.name)
        });
        let mut player = base_player(weapon_id);
        player.shield_id = shield_id;
        player.proficiencies = vec!["Large swords".to_string(), "Shields".to_string()];
        add_talent(&mut player, TALENT_ID_TWELVE_PATHS, None);

        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert_eq!(
            summary.roll.strength_damage - baseline_summary.roll.strength_damage,
            -TWELVE_PATHS_DAMAGE_PENALTY
        );
        assert_eq!(
            combatant.sheet.offense.strength_damage
                - baseline_combatant.sheet.offense.strength_damage,
            -TWELVE_PATHS_DAMAGE_PENALTY
        );
    }

    #[test]
    fn twelve_paths_combines_weapon_and_shield_defense_masteries() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = weapons
            .entries()
            .iter()
            .enumerate()
            .find(|(_, weapon)| weapon.name.eq_ignore_ascii_case("Flamberge"))
            .and_then(|(idx, _)| weapons.id_from_index(idx))
            .expect("no flamberge found");
        let (shield_id, _shield) = find_shield(&shields, |shield| {
            is_small_shield_or_buckler_name(&shield.name)
        });
        let mut player = base_player(weapon_id);
        player.shield_id = shield_id;
        player.proficiencies = vec!["Flamberge".to_string(), "Shields".to_string()];
        player.mastery_defense = 2;
        player.shield_mastery_defense = 1;
        add_talent(&mut player, TALENT_ID_TWELVE_PATHS, None);

        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);

        let mut baseline = player.clone();
        baseline.mastery_defense = 0;
        baseline.shield_mastery_defense = 0;
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );

        let expected_delta = player.mastery_defense + player.shield_mastery_defense;
        assert_eq!(
            summary
                .defense
                .melee_with_shield_dv
                .expect("shielded defense summary")
                - baseline_summary
                    .defense
                    .melee_with_shield_dv
                    .expect("baseline shielded defense summary"),
            expected_delta
        );
        assert_eq!(
            (combatant.sheet.defense.defense_mod + combatant.sheet.defense.shield_defense_bonus)
                - (baseline_combatant.sheet.defense.defense_mod
                    + baseline_combatant.sheet.defense.shield_defense_bonus),
            expected_delta
        );
    }

    #[test]
    fn armeroci_pole_applies_reach_and_speed_adjustments() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let (weapon_id, weapon_name) = weapons
            .entries()
            .iter()
            .enumerate()
            .find(|(_, weapon)| {
                matches!(
                    weapon.group,
                    WeaponGroup::LargeSwords | WeaponGroup::Polearms
                ) && weapon.reach_ft >= 5.0
            })
            .and_then(|(idx, weapon)| {
                weapons
                    .id_from_index(idx)
                    .map(|id| (id, weapon.name.clone()))
            })
            .expect("no qualifying armeroci pole weapon found");
        let mut player = base_player(weapon_id);
        player.proficiencies = vec![weapon_name];
        add_talent(&mut player, TALENT_ID_ARMEROCI_POLE, None);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert!(
            (combatant.sheet.offense.weapon.reach_ft
                - baseline_combatant.sheet.offense.weapon.reach_ft
                - ARMEROCI_POLE_REACH_BONUS_FT)
                .abs()
                < 0.001
        );
        assert!(
            (combatant.sheet.offense.weapon.speed
                - baseline_combatant.sheet.offense.weapon.speed
                - ARMEROCI_POLE_SPEED_PENALTY)
                .abs()
                < 0.001
        );
    }

    #[test]
    fn falling_sun_adds_speed_penalty_and_expanded_penetration() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = weapons
            .entries()
            .iter()
            .position(|weapon| weapon.name.eq_ignore_ascii_case("Flamberge"))
            .and_then(|idx| weapons.id_from_index(idx))
            .expect("no flamberge found");
        let mut player = base_player(weapon_id);
        player.proficiencies = vec!["Flamberge".to_string()];
        add_talent(&mut player, TALENT_ID_FALLING_SUN, None);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );

        assert!(
            (combatant.sheet.offense.weapon.speed
                - baseline_combatant.sheet.offense.weapon.speed
                - 2.0)
                .abs()
                < 0.001
        );
        assert!(
            combatant
                .sheet
                .offense
                .weapon
                .damage_expr_cache
                .penetrate_on_max_minus_one()
        );
        assert!(combatant.apply_i32(crate::sim::StatIdI32::FlagFallingSunStyle, 0) > 0);
    }

    #[test]
    fn doomrazor_removes_strength_mastery_and_forces_nonpenetrating_damage() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = weapons
            .entries()
            .iter()
            .position(|weapon| weapon.name.eq_ignore_ascii_case("Dagger"))
            .and_then(|idx| weapons.id_from_index(idx))
            .expect("no dagger found");
        let mut player = base_player(weapon_id);
        player.mastery_damage = 3;
        player.proficiencies = vec!["Dagger".to_string()];
        add_talent(&mut player, TALENT_ID_DOOMRAZOR, None);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        let weapon = weapons.get(weapon_id).expect("selected weapon missing");
        let removed_damage = strength_damage_for_weapon(
            weapon,
            baseline_combatant.sheet.offense.strength_damage_base,
        ) + player.mastery_damage;

        assert_eq!(
            baseline_combatant.sheet.offense.strength_damage
                - combatant.sheet.offense.strength_damage,
            removed_damage
        );
        assert!(combatant.sheet.offense.weapon.force_nonpenetrating_damage);
        assert_eq!(combatant.sheet.offense.weapon.internal_hemorrhage_damage, 1);
    }

    #[test]
    fn quiet_river_halves_damage_ignores_dr_and_doubles_unarmed_defense_mastery() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = weapons
            .entries()
            .iter()
            .position(|weapon| weapon.name.eq_ignore_ascii_case("Fist"))
            .and_then(|idx| weapons.id_from_index(idx))
            .expect("no fist found");
        let mut player = base_player(weapon_id);
        player.mastery_defense = 3;
        player.proficiencies = vec!["Fist".to_string()];
        add_talent(&mut player, TALENT_ID_QUIET_RIVER, None);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );

        assert!(combatant.sheet.offense.weapon.halve_damage);
        assert!(combatant.sheet.offense.weapon.ignore_all_dr);
        assert_eq!(
            combatant.sheet.defense.defense_mod - baseline_combatant.sheet.defense.defense_mod,
            player.mastery_defense
        );
    }

    #[test]
    fn rhdwng_flow_marks_throwing_weapon_style_active() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = weapons
            .entries()
            .iter()
            .position(|weapon| weapon.name.eq_ignore_ascii_case("Throwing axe"))
            .and_then(|idx| weapons.id_from_index(idx))
            .expect("no throwing axe found");
        let mut player = base_player(weapon_id);
        player.proficiencies = vec!["Throwing axe".to_string()];
        add_talent(&mut player, TALENT_ID_RHDWNG_FLOW, None);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);

        assert!(combatant.apply_i32(crate::sim::StatIdI32::FlagRhdwngFlowStyle, 0) > 0);
    }

    #[test]
    fn rohavalan_bridge_halves_speed_and_reach_and_sets_close_hit_damage() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = weapons
            .entries()
            .iter()
            .position(|weapon| weapon.name.eq_ignore_ascii_case("Staff"))
            .and_then(|idx| weapons.id_from_index(idx))
            .expect("no staff found");
        let mut player = base_player(weapon_id);
        player.proficiencies = vec!["Staff".to_string()];
        add_talent(&mut player, TALENT_ID_ROHAVALAN_BRIDGE, None);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );

        assert!(
            (combatant.sheet.offense.weapon.speed
                - (baseline_combatant.sheet.offense.weapon.speed / 2.0).ceil())
            .abs()
                < 0.001
        );
        assert!(
            (combatant.sheet.offense.weapon.reach_ft
                - (baseline_combatant.sheet.offense.weapon.reach_ft / 2.0))
                .abs()
                < 0.001
        );
        assert_eq!(
            combatant
                .sheet
                .offense
                .weapon
                .use_close_hit_damage_expr
                .as_deref(),
            Some("2d4p")
        );
        assert_eq!(
            combatant
                .sheet
                .offense
                .weapon
                .use_close_hit_margin_less_than,
            10
        );
    }

    #[test]
    fn hobbler_applies_attack_penalty_to_summary_and_combatant() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = weapons
            .entries()
            .iter()
            .enumerate()
            .find(|(_, weapon)| matches!(weapon.group, WeaponGroup::Polearms | WeaponGroup::Spears))
            .and_then(|(idx, _)| weapons.id_from_index(idx))
            .expect("no polearm or spear found");
        let mut player = base_player(weapon_id);
        let proficiency = weapons
            .get(weapon_id)
            .map(|weapon| {
                if weapon.group == WeaponGroup::Polearms {
                    "Polearms"
                } else {
                    "Spears"
                }
            })
            .unwrap_or("Polearms");
        player.proficiencies = vec![proficiency.to_string()];
        add_talent(&mut player, TALENT_ID_HOBBLER, None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert_eq!(
            summary.roll.attack_bonus - baseline_summary.roll.attack_bonus,
            -HOBBLER_ATTACK_PENALTY
        );
        assert_eq!(
            combatant.sheet.offense.attack_bonus - baseline_combatant.sheet.offense.attack_bonus,
            -HOBBLER_ATTACK_PENALTY
        );
    }

    #[test]
    fn ithican_prince_applies_half_int_to_damage_and_defense() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = weapons
            .entries()
            .iter()
            .enumerate()
            .find(|(_, weapon)| weapon.group == WeaponGroup::SmallSwords)
            .and_then(|(idx, _)| weapons.id_from_index(idx))
            .expect("no small sword found");
        let (shield_id, _) = find_shield(&shields, |shield| {
            shield.name.trim().eq_ignore_ascii_case("buckler")
        });
        let mut player = base_player(weapon_id);
        player.intelligence = 18;
        player.shield_id = shield_id;
        player.proficiencies = vec!["Small swords".to_string(), "Shields".to_string()];
        add_talent(&mut player, TALENT_ID_ITHICAN_PRINCE, None);
        let styled_character = build_character(&player, &weapons, &armor, &shields, &talents);
        let expected_bonus = styled_character.ability_mods.intelligence.attack / 2;
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert_eq!(
            summary.derived.base_dv - baseline_summary.derived.base_dv,
            expected_bonus
        );
        assert_eq!(
            summary.roll.strength_damage - baseline_summary.roll.strength_damage,
            expected_bonus
        );
        assert_eq!(
            combatant.sheet.defense.defense_mod - baseline_combatant.sheet.defense.defense_mod,
            expected_bonus
        );
        assert_eq!(
            combatant.sheet.offense.strength_damage
                - baseline_combatant.sheet.offense.strength_damage,
            expected_bonus
        );
    }

    #[test]
    fn returner_applies_defense_penalty_to_summary_and_combatant() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = weapons
            .entries()
            .iter()
            .enumerate()
            .find(|(_, weapon)| {
                weapon.group == WeaponGroup::LargeSwords && weapon.size == WeaponSize::Large
            })
            .and_then(|(idx, _)| weapons.id_from_index(idx))
            .expect("no size L large sword found");
        let mut player = base_player(weapon_id);
        player.proficiencies = vec!["Large swords".to_string()];
        add_talent(&mut player, TALENT_ID_RETURNER, None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert_eq!(
            summary.derived.base_dv - baseline_summary.derived.base_dv,
            -RETURNER_DEFENSE_PENALTY
        );
        assert_eq!(
            combatant.sheet.defense.defense_mod - baseline_combatant.sheet.defense.defense_mod,
            -RETURNER_DEFENSE_PENALTY
        );
    }

    #[test]
    fn unbreakable_wall_increases_large_or_tower_shield_dr() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = one_handed_weapon_id(&weapons);
        let (shield_id, _) = find_shield(&shields, |shield| {
            is_large_or_tower_shield_name(&shield.name)
        });
        let mut player = base_player(weapon_id);
        player.shield_id = shield_id;
        player.proficiencies = vec!["Shields".to_string()];
        add_talent(&mut player, TALENT_ID_UNBREAKABLE_WALL, None);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert_eq!(
            combatant.sheet.defense.shield_dr - baseline_combatant.sheet.defense.shield_dr,
            2
        );
    }

    #[test]
    fn weapon_styles_only_apply_the_first_active_style() {
        let (weapons, armor, shields) = sample_catalogs();
        let weapon_id = one_handed_weapon_id(&weapons);
        let npc_presets = Catalog::new(Vec::new());
        let (shield_id, shield) = find_shield(&shields, |shield| shield.name == "Buckler");
        let style_one = TalentSpec {
            id: "style_one".to_string(),
            name: "Style One".to_string(),
            description: "".to_string(),
            cost_bp: None,
            cost_lp: None,
            cost_rp: None,
            category: TALENT_CATEGORY_WEAPON_STYLES.to_string(),
            race_categories: Vec::new(),
            race_ids: Vec::new(),
            requirements: Vec::new(),
            max_rank: 1,
            effects: vec![TalentEffect::ShieldDefenseBonus { amount: 1 }],
        };
        let style_two = TalentSpec {
            id: "style_two".to_string(),
            name: "Style Two".to_string(),
            description: "".to_string(),
            cost_bp: None,
            cost_lp: None,
            cost_rp: None,
            category: TALENT_CATEGORY_WEAPON_STYLES.to_string(),
            race_categories: Vec::new(),
            race_ids: Vec::new(),
            requirements: Vec::new(),
            max_rank: 1,
            effects: vec![TalentEffect::ShieldDefenseBonus { amount: 4 }],
        };
        let talent_catalog = Catalog::new(vec![style_one, style_two]);

        let mut player = base_player(weapon_id);
        player.shield_id = shield_id;
        player.talents = vec![
            TalentSelection {
                id: "style_one".to_string(),
                rank: 1,
                weapon: None,
            },
            TalentSelection {
                id: "style_two".to_string(),
                rank: 1,
                weapon: None,
            },
        ];
        let first_active = build_combatant(
            &player,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talent_catalog,
        );
        assert_eq!(
            first_active.sheet.defense.shield_defense_bonus,
            shield.defense_bonus + 1
        );

        player.talents.reverse();
        let second_active = build_combatant(
            &player,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talent_catalog,
        );
        assert_eq!(
            second_active.sheet.defense.shield_defense_bonus,
            shield.defense_bonus + 4
        );
    }

    #[test]
    fn storm_and_shield_of_blades_selection_requires_perfect_two_weapon_fighting() {
        let talents = sample_talents();
        let shield = find_talent(&talents, TALENT_ID_SHIELD_OF_BLADES).expect("missing shield");
        let other_style = find_talent(&talents, TALENT_ID_THREE_MOUNTAINS).expect("missing style");
        let (weapons, _armor, _shields) = sample_catalogs();
        let weapon_id = one_handed_weapon_id(&weapons);

        let mut player = base_player(weapon_id);
        add_talent(&mut player, TALENT_ID_STORM_OF_BLADES, None);

        assert!(has_other_weapon_style_selected(&player, shield, &talents));

        add_talent(&mut player, TALENT_ID_PERFECT_TWO_WEAPON_FIGHTING, None);

        assert!(!has_other_weapon_style_selected(&player, shield, &talents));
        assert!(has_other_weapon_style_selected(
            &player,
            other_style,
            &talents
        ));

        let unbreakable =
            find_talent(&talents, TALENT_ID_UNBREAKABLE_WALL).expect("missing Unbreakable Wall");
        let mut wren = base_player(weapon_id);
        add_talent(&mut wren, TALENT_ID_KANIAN_IMPALER, None);
        assert!(has_other_weapon_style_selected(
            &wren,
            unbreakable,
            &talents
        ));
    }

    #[test]
    fn storm_and_shield_of_blades_both_apply_with_perfect_two_weapon_fighting() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_sword_weapon_id(&weapons);
        let mut player = base_player(weapon_id);
        player.level = 9;
        player.offhand_weapon_id = Some(weapon_id);
        player.offensive_dualwielding = true;
        add_talent(&mut player, TALENT_ID_TWO_WEAPON_FIGHTING, None);
        add_talent(&mut player, TALENT_ID_IMPROVED_TWO_WEAPON_FIGHTING, None);
        add_talent(&mut player, TALENT_ID_GREATER_TWO_WEAPON_FIGHTING, None);
        add_talent(&mut player, TALENT_ID_PERFECT_TWO_WEAPON_FIGHTING, None);
        add_talent(&mut player, TALENT_ID_STORM_OF_BLADES, None);
        add_talent(&mut player, TALENT_ID_SHIELD_OF_BLADES, None);

        let modifiers = resolve_talent_modifiers(&player, &talents, &weapons);

        assert!(modifiers.perfect_two_weapon_fighting);
        assert!(modifiers.storm_of_blades_style);
        assert!(modifiers.shield_of_blades_style);

        let npc_presets = Catalog::new(Vec::new());
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        assert!(combatant.sheet.maneuvers.storm_of_blades);
    }

    #[test]
    fn shield_of_blades_makes_defensive_sword_bonus_always_on() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = sample_npc_presets();
        let sword_id = one_handed_sword_weapon_id(&weapons);
        let mut player = base_player(sword_id);
        player.offhand_weapon_id = Some(sword_id);
        player.defensive_dualwielding = true;

        let baseline = player_summary(&player, &weapons, &armor, &shields, &talents);
        assert!(
            baseline
                .defense
                .melee_roll_label
                .contains("(+4 after you attack)")
        );

        add_talent(&mut player, TALENT_ID_SHIELD_OF_BLADES, None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        assert_eq!(
            summary.defense.melee_roll_label,
            format!(
                "Defense roll (melee): d20p + {}",
                summary.derived.base_dv + 4
            )
        );

        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        assert!(combatant.sheet.offense.weapon.defense_bonus_always);
        assert!(!combatant.state.defense_plus_four_ready);
    }

    #[test]
    fn volfango_defense_breakdown_totals_twenty_three_and_lists_conditional_bonuses() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = sample_npc_presets();
        let sword_id = one_handed_sword_weapon_id(&weapons);
        let sword_name = weapon_name(&weapons, sword_id);
        let (raurosi_leather_id, _) =
            find_armor(&armor, |entry| entry.name == "Raurosi Leather?");
        let mut player = PlayerConfig::new("Volfango Drakos", sword_id);
        player.level = 8;
        player.dex_base = 20;
        player.dex_pct = 85;
        player.wisdom = 12;
        player.armor_id = raurosi_leather_id;
        player.armor_material_tier = 3;
        player.mastery_defense = 3;
        player.offhand_weapon_id = Some(sword_id);
        player.defensive_dualwielding = true;
        player.fight_defensively = true;
        player.fight_defensively_penalty = 8;
        add_talent(&mut player, "dodge", None);
        add_talent(&mut player, "defense_bonus_weapon", Some(sword_name));
        add_talent(&mut player, "light_armor_optimization", None);
        add_talent(&mut player, TALENT_ID_SHIELD_OF_BLADES, None);
        add_talent(&mut player, TALENT_ID_DECEPTIVE_DEFENDER, None);
        add_talent(&mut player, "combat_expertise", None);

        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let breakdowns = derived_stat_breakdowns(
            &player, &weapons, &armor, &shields, &talents, &summary, &combatant,
        );
        let defense = breakdowns
            .get(DerivedStatId::MeleeDefense)
            .expect("missing melee defense breakdown");

        assert_eq!(
            summary.defense.melee_roll_label,
            "Defense roll (melee): d20p + 23"
        );
        assert_eq!(defense.additive_total(), 23.0);
        for (id, expected) in [
            (DerivedStatId::HitPoints, summary.derived.hit_points as f64),
            (
                DerivedStatId::DrainResistance,
                summary.derived.drain_resistance as f64,
            ),
            (
                DerivedStatId::AttackBonus,
                summary.derived.attack_bonus as f64,
            ),
            (
                DerivedStatId::EffectiveAttackBonus,
                summary.roll.attack_bonus as f64,
            ),
            (
                DerivedStatId::EffectiveDamageBonus,
                summary.roll.strength_damage as f64,
            ),
            (
                DerivedStatId::SpeedModifier,
                summary.derived.speed_mod as f64,
            ),
            (
                DerivedStatId::InitiativeModifier,
                summary.derived.initiative_mod as f64,
            ),
            (DerivedStatId::BaseDefense, summary.derived.base_dv as f64),
            (DerivedStatId::ArmorDr, summary.derived.armor_dr as f64),
        ] {
            let breakdown = breakdowns.get(id).expect("missing numeric breakdown");
            assert_eq!(
                breakdown.additive_total(),
                expected,
                "{id:?} breakdown did not reproduce the displayed value"
            );
        }
        assert!(defense.lines.iter().any(
            |line| line.source.contains("Shield of Blades") && line.numeric_amount == Some(4.0)
        ));
        assert!(
            defense
                .notes
                .iter()
                .any(|note| note.contains("initial attack"))
        );
        assert!(
            defense
                .notes
                .iter()
                .any(|note| note.contains("every Called Shot"))
        );
    }

    #[test]
    fn talent_weapon_speed_bonus_applies() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let base_weapon_id = one_handed_weapon_id(&weapons);
        let base = base_player(base_weapon_id);
        let npc_presets = Catalog::new(Vec::new());

        let (swift_id, swift_weapon) =
            find_weapon_for_speed_bonus(&weapons, &armor, &shields, &talents, &base, |_weapon| {
                true
            });
        let swift_name = weapon_name(&weapons, swift_id);
        let mut swift_player = base.clone();
        swift_player.weapon_id = swift_id;
        add_talent(&mut swift_player, "swift", Some(swift_name));
        let swift_combatant = build_combatant(
            &swift_player,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        let mut swift_baseline = swift_player.clone();
        swift_baseline.talents.clear();
        let swift_baseline = build_combatant(
            &swift_baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert_eq!(
            swift_baseline.sheet.offense.weapon.speed - swift_combatant.sheet.offense.weapon.speed,
            1.0,
            "swift should reduce speed for {}",
            swift_weapon.name
        );

        let (ranged_id, ranged_weapon) =
            find_weapon_for_speed_bonus(&weapons, &armor, &shields, &talents, &base, |weapon| {
                is_ranged_weapon(weapon)
            });
        let ranged_name = weapon_name(&weapons, ranged_id);
        let mut ranged_player = base.clone();
        ranged_player.weapon_id = ranged_id;
        add_talent(&mut ranged_player, "greased_lightning", Some(ranged_name));
        let ranged_combatant = build_combatant(
            &ranged_player,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        let mut ranged_baseline = ranged_player.clone();
        ranged_baseline.talents.clear();
        let ranged_baseline = build_combatant(
            &ranged_baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert_eq!(
            ranged_baseline.sheet.offense.weapon.speed
                - ranged_combatant.sheet.offense.weapon.speed,
            1.0,
            "greased_lightning should reduce speed for {}",
            ranged_weapon.name
        );

        let (double_id, double_weapon) =
            find_weapon_for_speed_bonus(&weapons, &armor, &shields, &talents, &base, |weapon| {
                weapon.group == WeaponGroup::Double
            });
        let mut double_player = base.clone();
        double_player.weapon_id = double_id;
        add_talent(&mut double_player, "double_weapon_focus", None);
        let double_combatant = build_combatant(
            &double_player,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        let mut double_baseline = double_player.clone();
        double_baseline.talents.clear();
        let double_baseline = build_combatant(
            &double_baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert_eq!(
            double_baseline.sheet.offense.weapon.speed
                - double_combatant.sheet.offense.weapon.speed,
            1.0,
            "double_weapon_focus should reduce speed for {}",
            double_weapon.name
        );
    }

    #[test]
    fn talent_improved_awareness_improves_initiative_die() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let mut player = base_player(weapon_id);
        add_talent(&mut player, "improved_awareness", None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        assert_eq!(
            summary.derived.initiative_die,
            baseline_summary.derived.initiative_die.improved(1)
        );
    }

    #[test]
    fn talent_natural_attunement_applies_in_natural_surroundings() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let mut player = base_player(weapon_id);
        player.environment.natural_surroundings = true;
        add_talent(&mut player, "natural_attunement", None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        assert_eq!(
            summary.derived.speed_mod - baseline_summary.derived.speed_mod,
            -1
        );
    }

    #[test]
    fn talent_natural_awareness_applies_in_natural_surroundings() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let mut player = base_player(weapon_id);
        player.environment.natural_surroundings = true;
        add_talent(&mut player, "natural_awareness", None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        assert_eq!(
            summary.derived.initiative_mod - baseline_summary.derived.initiative_mod,
            -2
        );
    }

    #[test]
    fn talent_natural_protection_applies_in_natural_surroundings() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let mut player = base_player(weapon_id);
        player.environment.natural_surroundings = true;
        add_talent(&mut player, "natural_protection", None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        assert_eq!(
            summary.derived.armor_dr - baseline_summary.derived.armor_dr,
            1
        );
    }

    #[test]
    fn race_armeroci_grants_defense_and_initiative_die() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let mut player = base_player(weapon_id);
        player.race_id = Some("armeroci".to_string());
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.race_id = None;
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        assert_eq!(
            summary.derived.base_dv - baseline_summary.derived.base_dv,
            1
        );
        assert_eq!(
            summary.derived.initiative_die,
            baseline_summary.derived.initiative_die.improved(1)
        );
    }

    #[test]
    fn race_vorova_temperature_modifies_rolls() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let mut player = base_player(weapon_id);
        player.race_id = Some("vorova_female".to_string());
        player.environment.temperature_c = -5;
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.environment.temperature_c = 20;
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        assert_eq!(
            summary.roll.attack_bonus - baseline_summary.roll.attack_bonus,
            1
        );
    }

    #[test]
    fn race_vorova_heat_adaptation_reduces_hot_penalty() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let mut player = base_player(weapon_id);
        player.race_id = Some("vorova_female".to_string());
        player.environment.temperature_c = 32;
        add_talent(&mut player, "heat_adaptation", None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        assert_eq!(
            summary.roll.attack_bonus - baseline_summary.roll.attack_bonus,
            1
        );
    }

    #[test]
    fn race_vorova_frostheart_triples_temperature_effects() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let mut player = base_player(weapon_id);
        player.race_id = Some("vorova_female".to_string());
        player.environment.temperature_c = -5;
        add_talent(&mut player, "frostheart", None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        assert_eq!(
            summary.roll.attack_bonus - baseline_summary.roll.attack_bonus,
            2
        );
    }

    #[test]
    fn environment_bonuses_apply_to_combatant_sheet() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = sample_npc_presets();
        let base_weapon_id = one_handed_weapon_id(&weapons);
        let base = base_player(base_weapon_id);

        let (swift_id, swift_weapon) =
            find_weapon_for_speed_bonus(&weapons, &armor, &shields, &talents, &base, |_weapon| {
                true
            });
        let mut natural_player = base.clone();
        natural_player.weapon_id = swift_id;
        natural_player.race_id = Some("armeroci".to_string());
        add_talent(&mut natural_player, "natural_attunement", None);
        let mut natural_env = natural_player.clone();
        natural_env.environment.natural_surroundings = true;
        let natural_combatant = build_combatant(
            &natural_env,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        let mut baseline_env = natural_player.clone();
        baseline_env.environment.natural_surroundings = false;
        let baseline_combatant = build_combatant(
            &baseline_env,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert_eq!(
            baseline_combatant.sheet.offense.weapon.speed
                - natural_combatant.sheet.offense.weapon.speed,
            1.0,
            "natural_attunement should reduce speed for {}",
            swift_weapon.name
        );

        let mut cold_player = base.clone();
        cold_player.race_id = Some("vorova_female".to_string());
        cold_player.environment.temperature_c = -5;
        let cold_combatant = build_combatant(
            &cold_player,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        let mut baseline_temp = cold_player.clone();
        baseline_temp.environment.temperature_c = 20;
        let baseline_temp_combatant = build_combatant(
            &baseline_temp,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert_eq!(
            cold_combatant.sheet.offense.attack_bonus
                - baseline_temp_combatant.sheet.offense.attack_bonus,
            1
        );
    }

    #[test]
    fn talent_pain_tolerant_adds_to_threshold_of_pain_percentage() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let npc_presets = Catalog::new(Vec::new());
        let mut player = base_player(weapon_id);
        add_talent(&mut player, "pain_tolerant", None);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_combatant = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        let expected = ((baseline_combatant.sheet.vitals.max_hp as f32)
            * (0.30 + (player.level as f32 * 0.01) + 0.10))
            .ceil() as i32;
        assert_eq!(combatant.sheet.vitals.threshold_of_pain, expected);
    }

    #[test]
    fn talent_hardened_uses_barbarian_threshold_of_pain() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let npc_presets = Catalog::new(Vec::new());
        for talent_id in [
            "hardened",
            "fighter_hardened",
            "knight_hardened",
            "ranger_hardened",
        ] {
            let mut player = base_player(weapon_id);
            add_talent(&mut player, talent_id, None);
            let combatant =
                build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
            let mut baseline = player.clone();
            baseline.talents.clear();
            let baseline_combatant = build_combatant(
                &baseline,
                &weapons,
                &armor,
                &shields,
                &npc_presets,
                &talents,
            );
            let max_hp = baseline_combatant.sheet.vitals.max_hp;
            let pct = 0.30 + (player.level as f32 * 0.02);
            let expected = ((max_hp as f32) * pct).ceil() as i32;
            assert_eq!(
                combatant.sheet.vitals.threshold_of_pain, expected,
                "{talent_id} should use the hardened formula"
            );
        }
    }

    #[test]
    fn talent_natural_strength_adds_unarmed_damage() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = unarmed_weapon_id(&weapons);
        let mut player = base_player(weapon_id);
        add_talent(&mut player, "natural_strength", None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary = player_summary(&baseline, &weapons, &armor, &shields, &talents);
        assert_eq!(
            summary.roll.strength_damage - baseline_summary.roll.strength_damage,
            2
        );
    }

    #[test]
    fn talent_unbreakable_increases_tough_hide_bonus() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let npc_presets = Catalog::new(Vec::new());
        let mut player = base_player(weapon_id);
        player.level = 11;
        add_talent(&mut player, "tough_hide", None);
        add_talent(&mut player, "unbreakable", None);
        let with_unbreakable =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline = player.clone();
        baseline.talents = vec![TalentSelection {
            id: "tough_hide".to_string(),
            rank: 1,
            weapon: None,
        }];
        let without_unbreakable = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert_eq!(
            with_unbreakable.sheet.defense.armor_dr - without_unbreakable.sheet.defense.armor_dr,
            1
        );
    }

    #[test]
    fn talent_stout_increases_knockback_step() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = one_handed_weapon_id(&weapons);
        let mut player = base_player(weapon_id);
        player.knockback_step = DEFAULT_KNOCKBACK_STEP;
        add_talent(&mut player, "stout", None);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(combatant.sheet.defense.knockback_step, 20);
    }

    #[test]
    fn talent_sturdy_increases_knockback_step() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = one_handed_weapon_id(&weapons);
        let mut player = base_player(weapon_id);
        player.knockback_step = DEFAULT_KNOCKBACK_STEP;
        add_talent(&mut player, "sturdy", None);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(combatant.sheet.defense.knockback_step, 20);
    }

    #[test]
    fn kanian_impaler_loads_from_talent_catalog() {
        let talents = sample_talents();
        let talent = talents
            .entries()
            .iter()
            .find(|talent| talent.id == "kanian_impaler")
            .expect("Missing Kanian Impaler talent");
        assert_eq!(talent.name, "Kanian Impaler");
    }

    #[test]
    fn kanian_impaler_treats_opponents_smaller_with_large_spears() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = weapons
            .entries()
            .iter()
            .enumerate()
            .find(|(_, weapon)| {
                weapon.group == WeaponGroup::Spears && weapon.size == WeaponSize::Large
            })
            .map(|(idx, _)| WeaponId::new(idx))
            .expect("Missing size L spear");
        let mut player = base_player(weapon_id);
        add_talent(&mut player, "kanian_impaler", None);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(
            combatant
                .sheet
                .offense
                .weapon
                .defender_knockback_step_adjustment,
            -5
        );
    }

    #[test]
    fn inactive_kanian_impaler_does_not_stack_with_unbreakable_wall() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = weapons
            .entries()
            .iter()
            .enumerate()
            .find(|(_, weapon)| {
                weapon.group == WeaponGroup::Spears && weapon.size == WeaponSize::Large
            })
            .map(|(idx, _)| WeaponId::new(idx))
            .expect("Missing size L spear");
        let mut player = base_player(weapon_id);
        player.proficiencies.push("Shields".to_string());
        add_talent(&mut player, TALENT_ID_UNBREAKABLE_WALL, None);
        add_talent(&mut player, TALENT_ID_KANIAN_IMPALER, None);

        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);

        assert_eq!(
            combatant
                .sheet
                .offense
                .weapon
                .defender_knockback_step_adjustment,
            0
        );
    }

    #[test]
    fn kanian_impaler_does_not_apply_to_medium_spears() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = weapons
            .entries()
            .iter()
            .enumerate()
            .find(|(_, weapon)| {
                weapon.group == WeaponGroup::Spears && weapon.size == WeaponSize::Medium
            })
            .map(|(idx, _)| WeaponId::new(idx))
            .expect("Missing size M spear");
        let mut player = base_player(weapon_id);
        add_talent(&mut player, "kanian_impaler", None);
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(
            combatant
                .sheet
                .offense
                .weapon
                .defender_knockback_step_adjustment,
            0
        );
    }

    #[test]
    fn talent_armored_to_the_teeth_adds_heavy_armor_damage() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = one_handed_weapon_id(&weapons);
        let (armor_id, _) = find_armor(&armor, |entry| entry.armor_type == ArmorType::Heavy);
        let mut player = base_player(weapon_id);
        player.armor_id = armor_id;
        add_talent(&mut player, "armored_to_the_teeth", None);
        let with_talent =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let without_talent = build_combatant(
            &baseline,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
        assert_eq!(
            with_talent.sheet.offense.strength_damage
                - without_talent.sheet.offense.strength_damage,
            1
        );
    }

    #[test]
    fn knockback_step_respects_race_override_size() {
        let races = crate::data::load_races("data/races.json").expect("Failed to load races");
        let armeroci = races
            .iter()
            .find(|race| race.id == "armeroci")
            .expect("Missing armeroci race");
        let limmtrig = races
            .iter()
            .find(|race| race.id == "limmtrig")
            .expect("Missing limmtrig race");
        assert_eq!(knockback_step_for_race(armeroci), 10);
        assert_eq!(knockback_step_for_race(limmtrig), 20);
    }

    #[test]
    fn talent_improved_critical_lowers_crit_min() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = weapon_id_by_group(&weapons, WeaponGroup::Axes);
        let mut player = base_player(weapon_id);
        add_talent(&mut player, "improved_critical", Some("axes".to_string()));
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(combatant.sheet.offense.weapon.crit_min_roll, 19);
    }

    #[test]
    fn talent_critical_mastery_lowers_crit_min_further() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = weapon_id_by_group(&weapons, WeaponGroup::Axes);
        let mut player = base_player(weapon_id);
        player.level = 15;
        add_talent(&mut player, "improved_critical", Some("axes".to_string()));
        add_talent(&mut player, "critical_mastery", Some("axes".to_string()));
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(combatant.sheet.offense.weapon.crit_min_roll, 18);
    }

    #[test]
    fn talent_wounding_criticals_increases_severity() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = weapon_id_by_group(&weapons, WeaponGroup::Axes);
        let mut player = base_player(weapon_id);
        player.level = 11;
        add_talent(&mut player, "improved_critical", Some("axes".to_string()));
        add_talent(&mut player, "wounding_criticals", Some("axes".to_string()));
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(combatant.sheet.offense.weapon.crit_severity_bonus, 3);
    }

    #[test]
    fn curse_of_axe_forces_steel_greataxe_with_custom_penetration() {
        let (weapons, armor, shields) =
            crate::data::load_catalogs().expect("Failed to load catalogs");
        let talents =
            crate::data::load_talents(crate::data::TALENTS_PATH).expect("Failed to load talents");
        let npc_presets = Catalog::new(Vec::new());
        let mut player = base_player(weapons.first_id().expect("weapon catalog empty"));
        player.weapon_material_tier = 0;
        add_talent(&mut player, TALENT_ID_CURSE_OF_AXE, None);

        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);

        assert_eq!(
            combatant.sheet.offense.weapon.name,
            CURSE_OF_AXE_WEAPON_NAME
        );
        assert_eq!(combatant.sheet.offense.weapon.damage_expr, "3d6p+3^2");
        assert_eq!(
            combatant
                .sheet
                .offense
                .weapon
                .damage_expr_cache
                .d6_penetration_triggers(),
            Some(CURSE_OF_AXE_D6_TRIGGERS)
        );
        assert!(combatant.sheet.offense.attack_bonus >= 2);
    }

    #[test]
    fn talent_ranged_critical_mastery_lowers_ranged_crit_min() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = Catalog::new(Vec::new());
        let weapon_id = weapon_id_by_group_ranged(&weapons, WeaponGroup::Bows);
        let mut player = base_player(weapon_id);
        player.level = 15;
        add_talent(&mut player, "improved_critical", Some("bows".to_string()));
        add_talent(
            &mut player,
            "ranged_critical_mastery",
            Some("bows".to_string()),
        );
        let combatant =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(combatant.sheet.offense.weapon.crit_min_roll, 18);
        assert_eq!(
            combatant.sheet.offense.weapon.crit_min_roll_ranged,
            Some(18)
        );
    }

    #[test]
    fn talent_is_implemented_marks_runtime_talents_as_supported() {
        let talents = sample_talents();
        for talent_id in [
            "improved_critical",
            "critical_mastery",
            "wounding_criticals",
            "ranged_critical_mastery",
            "two_weapon_fighting",
            "improved_two_weapon_fighting",
            "greater_two_weapon_fighting",
            "perfect_two_weapon_fighting",
            "power_attack",
        ] {
            let spec = talents
                .entries()
                .iter()
                .find(|entry| entry.id == talent_id)
                .expect("missing runtime talent");
            assert!(
                talent_is_implemented(spec),
                "{talent_id} should be treated as implemented"
            );
        }
    }

    #[test]
    fn talent_is_implemented_keeps_empty_non_runtime_talents_as_nyi() {
        let talents = sample_talents();
        let spec = talents
            .entries()
            .iter()
            .find(|entry| entry.id == "great_cleave")
            .expect("missing great_cleave");
        assert!(
            !talent_is_implemented(spec),
            "great_cleave should remain NYI until runtime behavior exists"
        );
    }

    #[test]
    fn capability_report_lists_supported_power_attack_and_weapon_styles() {
        let talents = sample_talents();
        let report = sim_capability_report(&talents);

        assert!(
            report
                .supported_tactical_toggles
                .contains(&"power_attack".to_string())
        );
        assert!(
            report
                .supported_talent_ids_with_direct_combat_effects
                .contains(&"power_attack".to_string())
        );
        assert!(
            report
                .supported_weapon_style_ids
                .contains(&"rohavalan_bridge".to_string())
        );
        assert!(
            report
                .supported_weapon_style_ids
                .contains(&"shield_of_blades".to_string())
        );
        assert!(
            report
                .supported_weapon_style_ids
                .contains(&"storm_of_blades".to_string())
        );
    }

    #[test]
    fn capability_report_separates_data_only_and_nyi_talents() {
        let talents = sample_talents();
        let report = sim_capability_report(&talents);

        for id in ["weapon_focus", "weapon_specialization", "weapon_supremacy"] {
            let id = id.to_string();
            assert!(report.known_data_only_talent_ids.contains(&id));
            assert!(
                !report
                    .supported_talent_ids_with_direct_combat_effects
                    .contains(&id)
            );
            assert!(!report.nyi_talent_ids.contains(&id));
        }
        assert!(report.nyi_talent_ids.contains(&"great_cleave".to_string()));
        assert!(
            report
                .known_unsupported_tactical_toggles
                .contains(&"aggressive_attack".to_string())
        );
    }
}
