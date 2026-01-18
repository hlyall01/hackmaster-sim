use crate::character::{
    AbilityScore, AbilitySet, Armor, ArmorType, Character, DerivedStats, Equipment, Progression,
    Shield, Weapon, WeaponGroup, WeaponMastery,
};
use crate::core::catalog::Catalog;
use crate::core::rules::DamageExprCache;
use crate::core::types::{
    AbilityKind, RaceSpec, TalentEffect, TalentRequirement, TalentSelection, TalentSpec,
};
pub use crate::core::ids::{
    ArmorId, ArmorTag, FighterPresetId, FighterPresetTag, NpcPresetId, NpcPresetTag, ShieldId,
    ShieldTag, TalentId, TalentTag, WeaponId, WeaponTag,
};
use crate::sim::{
    self, Combatant, CombatantSheet, DefenseProfile, MobilityProfile, OffenseProfile, Vitals,
    WeaponProfile,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

#[derive(Clone)]
pub struct ArmorEntry {
    pub label: String,
    pub armor: Option<Armor>,
}

#[derive(Clone)]
pub struct ShieldPreset {
    pub name: String,
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

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct CombatManeuverConfig {
    #[serde(default, skip_serializing_if = "is_false")]
    pub use_jab: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub hold_at_bay: bool,
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
    pub talents: Vec<TalentSelection>,
}

#[derive(Clone, Copy, Debug)]
pub struct EnvironmentConfig {
    pub temperature_c: i32,
    pub natural_surroundings: bool,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            temperature_c: 21,
            natural_surroundings: false,
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
    pub environment: EnvironmentConfig,
    pub misc_modifiers: MiscRollModifiers,
    pub knockback_step: i32,
    pub race_id: Option<String>,
    pub race_applied: bool,
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
            aggressive_attack: false,
            charge: false,
            ready_against_charge: false,
            tactical_move: false,
            fight_defensively: false,
            full_parry: false,
            give_ground: false,
            scamper_back: false,
            fighting_withdrawal: false,
            flee: false,
            defensive_dualwielding: false,
            offensive_dualwielding: false,
            environment: EnvironmentConfig::default(),
            misc_modifiers: MiscRollModifiers::default(),
            knockback_step: DEFAULT_KNOCKBACK_STEP,
            race_id: None,
            race_applied: false,
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
struct TalentModifiers {
    hp_bonus: i32,
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
    shield_defense_bonus: i32,
    shield_cover_value_adjustment: i32,
    ignore_armor_initiative_penalty: bool,
    ignore_armor_speed_penalty: bool,
    armor_dr_bonus_armored: i32,
    light_armor_defense_divisor: Option<i32>,
    medium_armor_dr_bonus: i32,
    medium_armor_defense_penalty_reduction: i32,
    heavy_armor_damage_bonus_divisor: Option<i32>,
    heavy_armor_damage_bonus_flat: i32,
    reach_bonus_by_group: HashMap<WeaponGroup, i32>,
    range_distance_multiplier: f32,
    threshold_of_pain_multiplier: f32,
    threshold_of_pain_level_bonus: f32,
    crit_min_by_group: HashMap<WeaponGroup, i32>,
    crit_min_ranged_by_group: HashMap<WeaponGroup, i32>,
    crit_severity_bonus_by_group: HashMap<WeaponGroup, i32>,
    knockback_step_bumps: i32,
    defiant: bool,
    superior_defense: bool,
    edge_counter: bool,
}

impl Default for TalentModifiers {
    fn default() -> Self {
        Self {
            hp_bonus: 0,
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
            shield_defense_bonus: 0,
            shield_cover_value_adjustment: 0,
            ignore_armor_initiative_penalty: false,
            ignore_armor_speed_penalty: false,
            armor_dr_bonus_armored: 0,
            light_armor_defense_divisor: None,
            medium_armor_dr_bonus: 0,
            medium_armor_defense_penalty_reduction: 0,
            heavy_armor_damage_bonus_divisor: None,
            heavy_armor_damage_bonus_flat: 0,
            reach_bonus_by_group: HashMap::new(),
            range_distance_multiplier: 1.0,
            threshold_of_pain_multiplier: 1.0,
            threshold_of_pain_level_bonus: 0.0,
            crit_min_by_group: HashMap::new(),
            crit_min_ranged_by_group: HashMap::new(),
            crit_severity_bonus_by_group: HashMap::new(),
            knockback_step_bumps: 0,
            defiant: false,
            superior_defense: false,
            edge_counter: false,
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
        *self.weapon_speed_bonus_by_weapon.get(&weapon_id).unwrap_or(&0)
    }

    fn reach_bonus_for_group(&self, group: WeaponGroup) -> i32 {
        *self.reach_bonus_by_group.get(&group).unwrap_or(&0)
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
    if modifiers.ignore_armor_speed_penalty && armor.speed_mod < 0 {
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

#[derive(Clone, Debug)]
pub struct TalentContext<'a> {
    pub level: u8,
    pub stats: &'a AbilitySet,
    pub talents: &'a [TalentSelection],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TalentRequirementFailure {
    MinLevel { required: u8, current: u8 },
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
    selection
        .weapon
        .as_deref()
        .and_then(weapon_group_from_str)
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

pub fn evaluate_talent_requirements(
    spec: &TalentSpec,
    context: &TalentContext<'_>,
) -> Vec<TalentRequirementFailure> {
    let mut failures = Vec::new();
    let spec_selection = context.talents.iter().find(|selection| selection.id == spec.id);
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

fn talent_effects_active(spec: &TalentSpec, player: &PlayerConfig) -> bool {
    match spec.id.as_str() {
        "natural_attunement" | "natural_protection" | "natural_awareness" => {
            player.environment.natural_surroundings
        }
        _ => true,
    }
}

fn resolve_talent_modifiers(
    player: &PlayerConfig,
    talent_catalog: &TalentCatalog,
    weapon_catalog: &WeaponCatalog,
) -> TalentModifiers {
    let mut modifiers = TalentModifiers::default();
    let stats = ability_set_from_player(player);
    let context = TalentContext {
        level: player.level,
        stats: &stats,
        talents: &player.talents,
    };
    let mut weapon_id_lookup: HashMap<String, WeaponId> = HashMap::new();
    for (idx, weapon) in weapon_catalog.entries().iter().enumerate() {
        if let Some(id) = weapon_catalog.id_from_index(idx) {
            weapon_id_lookup.insert(weapon.name.to_ascii_lowercase(), id);
        }
    }
    let weapon_id_by_name_cached = |name: &str| {
        weapon_id_lookup
            .get(&name.to_ascii_lowercase())
            .copied()
    };
    for selection in &player.talents {
        let Some(spec) = find_talent(talent_catalog, &selection.id) else {
            continue;
        };
        if !evaluate_talent_requirements(spec, &context).is_empty() {
            continue;
        }
        if !talent_effects_active(spec, player) {
            continue;
        }
        let rank = talent_rank(selection);
        for effect in &spec.effects {
            match effect {
                TalentEffect::HitPointBonus { amount } => {
                    modifiers.hp_bonus += amount * rank;
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
                            let entry =
                                modifiers.attack_bonus_by_weapon.entry(weapon_id).or_insert(0);
                            *entry += amount * rank;
                        }
                    }
                }
                TalentEffect::DamageBonusWeapon { amount } => {
                    if let Some(weapon_name) = selection.weapon.as_deref() {
                        if let Some(weapon_id) = weapon_id_by_name_cached(weapon_name) {
                            let entry =
                                modifiers.damage_bonus_by_weapon.entry(weapon_id).or_insert(0);
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
                        modifiers.threshold_of_pain_multiplier *= multiplier.powi(rank);
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
            }
        }
        match spec.id.as_str() {
            "improved_critical" => {
                if let Some(group) = selection_weapon_group(selection) {
                    let entry = modifiers.crit_min_by_group.entry(group).or_insert(20);
                    *entry = (*entry).min(19);
                }
            }
            "critical_mastery" => {
                if let Some(group) = selection_weapon_group(selection) {
                    let entry = modifiers.crit_min_by_group.entry(group).or_insert(20);
                    *entry = (*entry).min(18);
                }
            }
            "ranged_critical_mastery" => {
                if let Some(group) = selection_weapon_group(selection) {
                    let entry = modifiers.crit_min_ranged_by_group.entry(group).or_insert(20);
                    *entry = (*entry).min(18);
                }
            }
            "wounding_criticals" => {
                if let Some(group) = selection_weapon_group(selection) {
                    let entry = modifiers
                        .crit_severity_bonus_by_group
                        .entry(group)
                        .or_insert(0);
                    *entry += 3 * rank;
                }
            }
            "stout" | "sturdy" => {
                modifiers.knockback_step_bumps += rank;
            }
            "defiant" => {
                modifiers.defiant = true;
            }
            "superior_defense" => {
                modifiers.superior_defense = true;
            }
            "edge_counter" => {
                modifiers.edge_counter = true;
            }
            _ => {}
        }
    }
    modifiers
}

fn player_has_talent(player: &PlayerConfig, id: &str) -> bool {
    player.talents.iter().any(|talent| talent.id == id)
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
                let mut hot_penalty = if temp > 40 { -2 } else if temp > 30 { -1 } else { 0 };
                if player_has_talent(player, "heat_adaptation") && hot_penalty < 0 {
                    hot_penalty += 1;
                }
                if player_has_talent(player, "frostheart") {
                    cold_bonus *= 3;
                    hot_penalty *= 3;
                }
                modifiers.all_roll_bonus += cold_bonus + hot_penalty;
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
    let size = race
        .knockback_size
        .as_deref()
        .unwrap_or(race.size.as_str());
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
    player.dex_base =
        clamp_stat_adjustment(player.dex_base, race.ability_adjustments.dexterity);
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
    can_equip_shield(player, weapon) && player.shield_id.index() > 0
}

pub fn defensive_dualwielding_active(player: &PlayerConfig, weapon: &WeaponPreset) -> bool {
    player.defensive_dualwielding
        && !player.offensive_dualwielding
        && weapon.handedness == WeaponHandedness::OneHanded
        && !player.two_hand_grip
}

pub fn offensive_dualwielding_active(player: &PlayerConfig, weapon: &WeaponPreset) -> bool {
    player.offensive_dualwielding
        && !player.defensive_dualwielding
        && weapon.handedness == WeaponHandedness::OneHanded
        && !player.two_hand_grip
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

pub struct PlayerSummary {
    pub derived: DerivedStats,
    pub roll: RollSummary,
}

fn weapon_for_player<'a>(
    player: &PlayerConfig,
    weapon_catalog: &'a WeaponCatalog,
) -> &'a WeaponPreset {
    weapon_catalog
        .get(player.weapon_id)
        .or_else(|| weapon_catalog.entries().first())
        .expect("weapon catalog is empty")
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
    let weapon = weapon_for_player(player, weapon_catalog);
    let character = build_character(player, weapon_catalog, armor_catalog, shield_catalog);
    let modifiers = resolve_talent_modifiers(player, talent_catalog, weapon_catalog);
    let misc_modifiers = resolve_misc_modifiers(player);
    let armor_adjustments = armor_talent_adjustments(character.equipment.armor.as_ref(), &modifiers);
    let defensive_dualwielding = defensive_dualwielding_active(player, weapon);
    let defense_bonus_weapon =
        modifiers.defense_bonus_for_weapon(player.weapon_id)
            * if defensive_dualwielding { 2 } else { 1 };
    let mut derived = character.derived();
    derived.attack_bonus += misc_modifiers.attack_bonus + misc_modifiers.all_roll_bonus;
    derived.speed_mod +=
        armor_adjustments.speed_mod_bonus + modifiers.speed_mod_bonus + misc_modifiers.speed_mod_bonus;
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
    derived.armor_dr = (derived.armor_dr
        + armor_adjustments.armor_dr_bonus
        + modifiers.armor_dr_bonus
        + misc_modifiers.armor_dr_bonus)
        .max(0);
    derived.base_dv += modifiers.defense_bonus
        + defense_bonus_weapon
        + misc_modifiers.defense_bonus
        + misc_modifiers.all_roll_bonus;
    let roll = roll_summary(
        player,
        weapon,
        &character,
        &derived,
        &modifiers,
        &misc_modifiers,
        armor_adjustments.heavy_armor_damage_bonus,
    );
    PlayerSummary { derived, roll }
}

fn roll_summary(
    player: &PlayerConfig,
    weapon: &WeaponPreset,
    character: &Character,
    derived: &DerivedStats,
    modifiers: &TalentModifiers,
    misc_modifiers: &MiscRollModifiers,
    armor_damage_bonus: i32,
) -> RollSummary {
    let is_ranged_weapon = is_ranged_weapon(weapon);
    let uses_projectiles = uses_projectiles(&weapon.name, weapon.ammunition.is_some());
    let (material_attack_bonus, material_damage_bonus) = material_bonuses(
        player.weapon_material_tier,
        player.projectile_material_tier,
        is_ranged_weapon,
        uses_projectiles,
    );
    let attack_mastery = effective_attack_mastery(player);
    let damage_mastery = effective_damage_mastery(player);
    let attack_bonus = derived.attack_bonus
        + material_attack_bonus
        + attack_mastery
        + modifiers.attack_bonus_for_weapon(player.weapon_id);
    let two_hand_bonus = two_hand_damage_bonus(weapon, player.two_hand_grip);
    let mut strength_damage =
        strength_damage_for_weapon(weapon, character.ability_mods.strength.damage)
        + two_hand_bonus
        + material_damage_bonus
        + damage_mastery
        + modifiers.damage_bonus_for_weapon(player.weapon_id)
        + modifiers.damage_bonus_for_group(weapon.group)
        + misc_modifiers.damage_bonus
        + misc_modifiers.all_roll_bonus;
    if !is_ranged_weapon {
        strength_damage += armor_damage_bonus;
    }

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
) -> Character {
    let weapon_preset = weapon_for_player(player, weapon_catalog);
    let weapon = Weapon {
        name: weapon_preset.name.clone(),
        group: weapon_preset.group,
        speed: weapon_preset.speed,
        damage_expr: weapon_preset.damage_expr.clone(),
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

    let shield = if can_equip_shield(player, weapon_preset) {
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
    let weapon_preset = weapon_for_player(player, weapon_catalog);
    let character = build_character(player, weapon_catalog, armor_catalog, shield_catalog);
    let modifiers = resolve_talent_modifiers(player, talent_catalog, weapon_catalog);
    let misc_modifiers = resolve_misc_modifiers(player);
    let armor_adjustments = armor_talent_adjustments(character.equipment.armor.as_ref(), &modifiers);
    let mut derived = character.derived();
    derived.attack_bonus += misc_modifiers.attack_bonus + misc_modifiers.all_roll_bonus;
    derived.speed_mod +=
        armor_adjustments.speed_mod_bonus + modifiers.speed_mod_bonus + misc_modifiers.speed_mod_bonus;
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
    let weapon_speed = character
        .equipment
        .weapon
        .as_ref()
        .map(|weapon| weapon.speed)
        .unwrap_or(10.0);
    let speed_mod = derived.speed_mod as f32;
    let reach_bonus = modifiers.reach_bonus_for_group(weapon_preset.group) as f32;
    let weapon_reach = (character
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

    let effective_two_hand = effective_two_hand_grip(weapon_preset, player.two_hand_grip);
    let two_hand_damage_bonus = two_hand_damage_bonus(weapon_preset, player.two_hand_grip);
    let two_hand_speed_penalty = two_hand_speed_penalty(weapon_preset, player.two_hand_grip);
    let use_jab = player.use_jab && weapon_preset.jab_speed.is_some();
    let min_speed = weapon_preset.size.min_speed();
    let speed_mastery = effective_speed_mastery(player, weapon_preset) as f32;
    let jab_speed =
        (weapon_preset.jab_speed.unwrap_or(weapon_speed)
            + speed_mod
            - speed_mastery
            + modifiers.weapon_speed_bonus_for_weapon(player.weapon_id) as f32)
            .max(min_speed);
    let jab_special_expr = if use_jab {
        weapon_preset.jab_special_expr.clone()
    } else {
        None
    };

    let mut name = character.name;
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
        player.weapon_material_tier,
        player.projectile_material_tier,
        primary_is_ranged,
        primary_uses_projectiles,
    );
    let attack_mastery = effective_attack_mastery(player);
    let mut attack_bonus_base = derived.attack_bonus + attack_mastery;
    let mut defensive_dualwielding = defensive_dualwielding_active(player, weapon_preset);
    let mut offensive_dualwielding = offensive_dualwielding_active(player, weapon_preset);
    if offensive_dualwielding {
        defensive_dualwielding = false;
    }
    let defense_mastery = effective_defense_mastery(player, weapon_preset)
        * if defensive_dualwielding { 2 } else { 1 };
    let defense_bonus_weapon =
        modifiers.defense_bonus_for_weapon(player.weapon_id)
            * if defensive_dualwielding { 2 } else { 1 };
    let defense_bonus =
        modifiers.defense_bonus + misc_modifiers.defense_bonus + misc_modifiers.all_roll_bonus;
    let damage_mastery = effective_damage_mastery(player);
    let mut attack_bonus =
        attack_bonus_base + material_attack_bonus + modifiers.attack_bonus_for_weapon(player.weapon_id);
    let mut defense_mod = derived.base_dv + defense_mastery + defense_bonus + defense_bonus_weapon;
    let mut dex_defense_bonus = character.ability_mods.dexterity.defense;
    let mut natural_dr = (modifiers.armor_dr_bonus + misc_modifiers.armor_dr_bonus).max(0);
    let mut armor_dr = (derived.armor_dr + natural_dr).max(0);
    let mut strength_damage_base = character.ability_mods.strength.damage;
    let mut unarmed_damage_bonus = modifiers.damage_bonus_for_group(WeaponGroup::Unarmed);
    let mut strength_damage =
        strength_damage_for_weapon(weapon_preset, strength_damage_base)
            + two_hand_damage_bonus
            + material_damage_bonus
        + damage_mastery
        + modifiers.damage_bonus_for_weapon(player.weapon_id)
        + modifiers.damage_bonus_for_group(weapon_preset.group)
        + misc_modifiers.damage_bonus
        + misc_modifiers.all_roll_bonus;
    if !primary_is_ranged {
        strength_damage += armor_adjustments.heavy_armor_damage_bonus;
    }
    let mut max_hp =
        (derived.hit_points as i32 + modifiers.hp_bonus + misc_modifiers.hp_bonus).max(1);
    let mut threshold_of_pain = threshold_of_pain(max_hp, player.level);
    if modifiers.threshold_of_pain_level_bonus != 0.0 {
        let level_pct = player.level as f32 * 0.01;
        let bonus_pct = player.level as f32 * modifiers.threshold_of_pain_level_bonus;
        let pct = 0.30 + level_pct + bonus_pct;
        threshold_of_pain = ((max_hp as f32) * pct).ceil() as i32;
    }
    if modifiers.threshold_of_pain_multiplier != 1.0 {
        threshold_of_pain =
            ((threshold_of_pain as f32) * modifiers.threshold_of_pain_multiplier).ceil() as i32;
        threshold_of_pain = threshold_of_pain.max(1);
    }
    let mut shield_name = shield_data.map(|shield| shield.name.clone());
    let mut shield_defense_bonus = shield_data.map(|shield| shield.defense_bonus).unwrap_or(0)
        + modifiers.shield_defense_bonus;
    let mut shield_dr = shield_data.map(|shield| shield.dr).unwrap_or(0);
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
    }

    let weapon_speed = if use_jab {
        jab_speed
    } else {
        (weapon_speed
            + two_hand_speed_penalty
            + speed_mod
            - speed_mastery
            + modifiers.weapon_speed_bonus_for_weapon(player.weapon_id) as f32)
            .max(min_speed)
    };
    let damage_expr_cache = DamageExprCache::new(&weapon_damage);
    let shield_damage_expr_cache = shield_damage_expr
        .as_deref()
        .map(DamageExprCache::new);
    let jab_special_expr_cache = jab_special_expr
        .as_deref()
        .map(DamageExprCache::new);
    let is_unarmed_weapon = weapon_preset.group == WeaponGroup::Unarmed;
    let is_small_weapon = matches!(weapon_preset.size, WeaponSize::Small);
    let knockback_step = bump_knockback_step(
        player.knockback_step.max(1),
        modifiers.knockback_step_bumps,
    );
    let mut offhand_profile = None;
    if offensive_dualwielding {
        if let Some(offhand_id) = player.offhand_weapon_id {
            if let Some(offhand_preset) = weapon_catalog.get(offhand_id) {
                if offhand_preset.handedness == WeaponHandedness::OneHanded {
                    let offhand_is_ranged = is_ranged_weapon(offhand_preset);
                    let offhand_uses_projectiles = uses_projectiles(
                        &offhand_preset.name,
                        offhand_preset.ammunition.is_some(),
                    );
                    let (material_attack_bonus, material_damage_bonus) = material_bonuses(
                        player.offhand_weapon_material_tier,
                        player.offhand_projectile_material_tier,
                        offhand_is_ranged,
                        offhand_uses_projectiles,
                    );
                    let offhand_attack_bonus = derived.attack_bonus
                        + attack_mastery
                        + material_attack_bonus
                        + modifiers.attack_bonus_for_weapon(offhand_id);
                    let mut offhand_strength_damage = strength_damage_for_weapon(
                        offhand_preset,
                        strength_damage_base,
                    ) + material_damage_bonus
                        + damage_mastery
                        + modifiers.damage_bonus_for_weapon(offhand_id)
                        + modifiers.damage_bonus_for_group(offhand_preset.group)
                        + misc_modifiers.damage_bonus
                        + misc_modifiers.all_roll_bonus;
                    if !offhand_is_ranged {
                        offhand_strength_damage += armor_adjustments.heavy_armor_damage_bonus;
                    }
                    let offhand_reach = (offhand_preset.reach_ft.max(1.0)
                        + modifiers.reach_bonus_for_group(offhand_preset.group) as f32)
                        .max(1.0);
                    let offhand_speed_mastery =
                        effective_speed_mastery(player, offhand_preset) as f32;
                    let offhand_min_speed = offhand_preset.size.min_speed();
                    let offhand_speed = (offhand_preset.speed
                        + speed_mod
                        - offhand_speed_mastery
                        + modifiers.weapon_speed_bonus_for_weapon(offhand_id) as f32)
                        .max(offhand_min_speed);
                    let offhand_damage_expr = offhand_preset.damage_expr.clone();
                    let offhand_damage_expr_cache = DamageExprCache::new(&offhand_damage_expr);
                    let offhand_shield_damage_expr = offhand_preset
                        .shield_damage_expr
                        .clone()
                        .filter(|expr| expr != "-" && !expr.is_empty());
                    let offhand_shield_damage_expr_cache =
                        offhand_shield_damage_expr.as_deref().map(DamageExprCache::new);
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
                            crit_min_roll: offhand_crit_min_roll,
                            crit_min_roll_ranged: offhand_crit_min_roll_ranged,
                            crit_severity_bonus: offhand_crit_severity_bonus,
                        }),
                    });
                }
            }
        }
    }
    if offhand_profile.is_none() {
        offensive_dualwielding = false;
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
        sheet_modifiers.add_i32(
            sim::StatIdI32::FlagEdgeCounter,
            sim::ModifierOpI32::Set(1),
        );
    }
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
                crit_min_roll,
                crit_min_roll_ranged,
                crit_severity_bonus,
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
            constitution: player.constitution,
            threshold_of_pain,
            trauma_die_sides,
            trauma_die_penetrating,
        },
        maneuvers: sim::ManeuverProfile {
            hold_at_bay: player.hold_at_bay,
            aggressive_attack: player.aggressive_attack,
            charge: player.charge,
            ready_against_charge: player.ready_against_charge,
            tactical_move: player.tactical_move,
            fight_defensively: player.fight_defensively,
            full_parry: player.full_parry,
            give_ground: player.give_ground,
            scamper_back: player.scamper_back,
            fighting_withdrawal: player.fighting_withdrawal,
            flee: player.flee,
            defensive_dualwielding,
            offensive_dualwielding,
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
        weapon_catalog
            .get(player.weapon_id)
            .map(|weapon| {
                let modifiers = resolve_talent_modifiers(player, talent_catalog, weapon_catalog);
                let reach_bonus = modifiers.reach_bonus_for_group(weapon.group) as f32;
                let base_reach = if is_ranged_weapon(weapon) {
                    melee_reach_from_label(&weapon.reach_label).unwrap_or(1.0)
                } else {
                    weapon.reach_ft
                };
                (base_reach + reach_bonus).max(1.0)
            })
            .unwrap_or(1.0)
    };
    let reach_a = reach_for_player(&players[0]);
    let reach_b = reach_for_player(&players[1]);
    reach_a.max(reach_b)
}


fn can_equip_shield(player: &PlayerConfig, weapon: &WeaponPreset) -> bool {
    weapon.handedness == WeaponHandedness::OneHanded
        && !player.two_hand_grip
        && !defensive_dualwielding_active(player, weapon)
        && !offensive_dualwielding_active(player, weapon)
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
        crate::data::load_talents("data/talents.json").expect("Failed to load talents")
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
        let weapon_id = one_handed_weapon_id(&weapons);
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
        assert!(build_maneuvers(&player).fight_defensively);

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
        let jab_combatant =
            build_combatant(&jab_player, &weapons, &armor, &shields, &npc_presets, &talents);
        assert!(jab_combatant.sheet.offense.weapon.use_jab);

        let mut no_jab_player = base_player(non_jab_weapon_id(&weapons));
        no_jab_player.use_jab = true;
        let no_jab_combatant =
            build_combatant(&no_jab_player, &weapons, &armor, &shields, &npc_presets, &talents);
        assert!(!no_jab_combatant.sheet.offense.weapon.use_jab);
    }

    fn add_talent(player: &mut PlayerConfig, id: &str, weapon: Option<String>) {
        player.talents.push(TalentSelection {
            id: id.to_string(),
            rank: 1,
            weapon,
        });
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
            let combatant = build_combatant(&player, weapons, armor, shields, &npc_presets, talents);
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
        };
        let failures = evaluate_talent_requirements(&spec, &context);
        assert!(failures.contains(&TalentRequirementFailure::MinStatBase {
            stat: AbilityKind::Strength,
            required: 12,
            current: 10,
        }));
        assert!(failures.contains(&TalentRequirementFailure::MinStatPercentile {
            stat: AbilityKind::Strength,
            required: 51,
            current: Some(1),
        }));
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
    fn shield_material_increases_breakage_thresholds() {
        let shield = ShieldPreset {
            name: "Test".to_string(),
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
        let baseline_summary =
            player_summary(&baseline, &weapons, &armor, &shields, &talents);
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
        let baseline_summary =
            player_summary(&baseline, &weapons, &armor, &shields, &talents);
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
        let baseline_combatant =
            build_combatant(
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
        let dual = build_combatant(
            &player,
            &weapons,
            &armor,
            &shields,
            &npc_presets,
            &talents,
        );
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
        let baseline_combatant =
            build_combatant(
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
            let without_bonus =
                build_combatant(&baseline, &weapons, &armor, &shields, &npc_presets, &talents);
            assert_eq!(
                with_bonus.sheet.vitals.max_hp - without_bonus.sheet.vitals.max_hp,
                bonus,
                "{talent_id} should add {bonus} hp"
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
        let without_bonus =
            build_combatant(&baseline, &weapons, &armor, &shields, &npc_presets, &talents);
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
            let baseline_summary =
                player_summary(&baseline, &weapons, &armor, &shields, &talents);
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
        let baseline_summary =
            player_summary(&baseline, &weapons, &armor, &shields, &talents);
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
        let baseline_summary =
            player_summary(&baseline, &weapons, &armor, &shields, &talents);
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
        let baseline_combatant =
            build_combatant(&baseline, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(
            combatant.sheet.defense.defense_mod - baseline_combatant.sheet.defense.defense_mod,
            1
        );
        let character = build_character(&baseline, &weapons, &armor, &shields);
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
        assert!((multiplier - 0.6667).abs() < 0.0001);
    }

    #[test]
    fn talent_armor_focus_removes_initiative_penalty() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let Some((armor_id, armor_entry)) = find_armor_opt(&armor, |a| a.initiative_mod < 0)
        else {
            return;
        };
        let mut player = base_player(weapon_id);
        player.armor_id = armor_id;
        add_talent(&mut player, "armor_focus", None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary =
            player_summary(&baseline, &weapons, &armor, &shields, &talents);
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
        let Some((armor_id, armor_entry)) = find_armor_opt(&armor, |a| a.speed_mod < 0) else {
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
        let baseline_summary =
            player_summary(&baseline, &weapons, &armor, &shields, &talents);
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
        let baseline_summary =
            player_summary(&baseline, &weapons, &armor, &shields, &talents);
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
        let (armor_id, armor_entry) = find_armor(
            &armor,
            |a| matches!(a.armor_type, ArmorType::Heavy) && a.damage_reduction > 0,
        );
        let mut player = base_player(weapon_id);
        player.armor_id = armor_id;
        add_talent(&mut player, "heavy_armor_optimization", None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary =
            player_summary(&baseline, &weapons, &armor, &shields, &talents);
        let expected_bonus = armor_entry.damage_reduction / 4;
        assert_eq!(
            summary.roll.strength_damage - baseline_summary.roll.strength_damage,
            expected_bonus
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
        let baseline_combatant =
            build_combatant(&baseline, &weapons, &armor, &shields, &npc_presets, &talents);
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
        let (armor_id, armor_entry) = find_armor(
            &armor,
            |a| matches!(a.armor_type, ArmorType::Light) && a.damage_reduction > 0,
        );
        let mut player = base_player(weapon_id);
        player.armor_id = armor_id;
        add_talent(&mut player, "light_armor_optimization", None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary =
            player_summary(&baseline, &weapons, &armor, &shields, &talents);
        let expected_bonus = armor_entry.damage_reduction / 2;
        assert_eq!(
            summary.derived.base_dv - baseline_summary.derived.base_dv,
            expected_bonus
        );
    }

    #[test]
    fn talent_medium_armor_optimization_adds_dr_and_defense() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let weapon_id = one_handed_weapon_id(&weapons);
        let (armor_id, armor_entry) = find_armor(
            &armor,
            |a| matches!(a.armor_type, ArmorType::Medium) && a.defense_adj < 0,
        );
        let mut player = base_player(weapon_id);
        player.armor_id = armor_id;
        add_talent(&mut player, "medium_armor_optimization", None);
        let summary = player_summary(&player, &weapons, &armor, &shields, &talents);
        let mut baseline = player.clone();
        baseline.talents.clear();
        let baseline_summary =
            player_summary(&baseline, &weapons, &armor, &shields, &talents);
        let expected_defense_bonus = (-armor_entry.defense_adj).min(1);
        assert_eq!(
            summary.derived.armor_dr - baseline_summary.derived.armor_dr,
            1
        );
        assert_eq!(
            summary.derived.base_dv - baseline_summary.derived.base_dv,
            expected_defense_bonus
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
        let baseline_combatant =
            build_combatant(&baseline, &weapons, &armor, &shields, &npc_presets, &talents);
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
        let baseline =
            build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
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
    fn talent_weapon_speed_bonus_applies() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let base_weapon_id = one_handed_weapon_id(&weapons);
        let base = base_player(base_weapon_id);
        let npc_presets = Catalog::new(Vec::new());

        let (swift_id, swift_weapon) = find_weapon_for_speed_bonus(
            &weapons,
            &armor,
            &shields,
            &talents,
            &base,
            |_weapon| true,
        );
        let swift_name = weapon_name(&weapons, swift_id);
        let mut swift_player = base.clone();
        swift_player.weapon_id = swift_id;
        add_talent(&mut swift_player, "swift", Some(swift_name));
        let swift_combatant =
            build_combatant(&swift_player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut swift_baseline = swift_player.clone();
        swift_baseline.talents.clear();
        let swift_baseline =
            build_combatant(&swift_baseline, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(
            swift_baseline.sheet.offense.weapon.speed - swift_combatant.sheet.offense.weapon.speed,
            1.0,
            "swift should reduce speed for {}",
            swift_weapon.name
        );

        let (ranged_id, ranged_weapon) = find_weapon_for_speed_bonus(
            &weapons,
            &armor,
            &shields,
            &talents,
            &base,
            |weapon| is_ranged_weapon(weapon),
        );
        let ranged_name = weapon_name(&weapons, ranged_id);
        let mut ranged_player = base.clone();
        ranged_player.weapon_id = ranged_id;
        add_talent(&mut ranged_player, "greased_lightning", Some(ranged_name));
        let ranged_combatant =
            build_combatant(&ranged_player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut ranged_baseline = ranged_player.clone();
        ranged_baseline.talents.clear();
        let ranged_baseline =
            build_combatant(&ranged_baseline, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(
            ranged_baseline.sheet.offense.weapon.speed
                - ranged_combatant.sheet.offense.weapon.speed,
            1.0,
            "greased_lightning should reduce speed for {}",
            ranged_weapon.name
        );

        let (double_id, double_weapon) = find_weapon_for_speed_bonus(
            &weapons,
            &armor,
            &shields,
            &talents,
            &base,
            |weapon| weapon.group == WeaponGroup::Double,
        );
        let mut double_player = base.clone();
        double_player.weapon_id = double_id;
        add_talent(&mut double_player, "double_weapon_focus", None);
        let double_combatant =
            build_combatant(&double_player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut double_baseline = double_player.clone();
        double_baseline.talents.clear();
        let double_baseline =
            build_combatant(&double_baseline, &weapons, &armor, &shields, &npc_presets, &talents);
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
        let baseline_summary =
            player_summary(&baseline, &weapons, &armor, &shields, &talents);
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
        assert_eq!(summary.roll.attack_bonus - baseline_summary.roll.attack_bonus, 1);
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
        assert_eq!(summary.roll.attack_bonus - baseline_summary.roll.attack_bonus, 1);
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
        assert_eq!(summary.roll.attack_bonus - baseline_summary.roll.attack_bonus, 2);
    }

    #[test]
    fn environment_bonuses_apply_to_combatant_sheet() {
        let (weapons, armor, shields) = sample_catalogs();
        let talents = sample_talents();
        let npc_presets = sample_npc_presets();
        let base_weapon_id = one_handed_weapon_id(&weapons);
        let base = base_player(base_weapon_id);

        let (swift_id, swift_weapon) = find_weapon_for_speed_bonus(
            &weapons,
            &armor,
            &shields,
            &talents,
            &base,
            |_weapon| true,
        );
        let mut natural_player = base.clone();
        natural_player.weapon_id = swift_id;
        natural_player.race_id = Some("armeroci".to_string());
        add_talent(&mut natural_player, "natural_attunement", None);
        let mut natural_env = natural_player.clone();
        natural_env.environment.natural_surroundings = true;
        let natural_combatant =
            build_combatant(&natural_env, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline_env = natural_player.clone();
        baseline_env.environment.natural_surroundings = false;
        let baseline_combatant =
            build_combatant(&baseline_env, &weapons, &armor, &shields, &npc_presets, &talents);
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
        let cold_combatant =
            build_combatant(&cold_player, &weapons, &armor, &shields, &npc_presets, &talents);
        let mut baseline_temp = cold_player.clone();
        baseline_temp.environment.temperature_c = 20;
        let baseline_temp_combatant =
            build_combatant(&baseline_temp, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(
            cold_combatant.sheet.offense.attack_bonus
                - baseline_temp_combatant.sheet.offense.attack_bonus,
            1
        );
    }

    #[test]
    fn talent_pain_tolerant_increases_threshold_of_pain() {
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
        let baseline_combatant =
            build_combatant(&baseline, &weapons, &armor, &shields, &npc_presets, &talents);
        let expected =
            ((baseline_combatant.sheet.vitals.threshold_of_pain as f32) * 1.1).ceil() as i32;
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
            let baseline_combatant =
                build_combatant(&baseline, &weapons, &armor, &shields, &npc_presets, &talents);
            let max_hp = baseline_combatant.sheet.vitals.max_hp;
            let pct = 0.30 + (player.level as f32 * 0.02);
            let expected = ((max_hp as f32) * pct).ceil() as i32;
            assert_eq!(
                combatant.sheet.vitals.threshold_of_pain,
                expected,
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
        let baseline_summary =
            player_summary(&baseline, &weapons, &armor, &shields, &talents);
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
        let without_unbreakable =
            build_combatant(&baseline, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(
            with_unbreakable.sheet.defense.armor_dr
                - without_unbreakable.sheet.defense.armor_dr,
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
        let without_talent =
            build_combatant(&baseline, &weapons, &armor, &shields, &npc_presets, &talents);
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
        let combatant = build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
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
        let combatant = build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
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
        let combatant = build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(combatant.sheet.offense.weapon.crit_severity_bonus, 3);
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
        let combatant = build_combatant(&player, &weapons, &armor, &shields, &npc_presets, &talents);
        assert_eq!(combatant.sheet.offense.weapon.crit_min_roll, 18);
        assert_eq!(
            combatant.sheet.offense.weapon.crit_min_roll_ranged,
            Some(18)
        );
    }
}
