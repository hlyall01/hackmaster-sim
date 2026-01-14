use crate::character::{
    AbilityScore, AbilitySet, Armor, ArmorType, Character, DerivedStats, Equipment, Progression,
    Shield, Weapon, WeaponGroup, WeaponMastery,
};
use crate::core::catalog::Catalog;
use crate::core::rules::DamageExprCache;
use crate::core::types::{
    AbilityKind, TalentEffect, TalentRequirement, TalentSelection, TalentSpec,
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
    pub armor: String,
    pub shield: String,
    pub weapon_material_tier: i32,
    pub armor_material_tier: i32,
    pub projectile_material_tier: i32,
    pub shield_material_tier: i32,
    pub two_hand_grip: bool,
    pub use_jab: bool,
    #[serde(default)]
    pub hold_at_bay: bool,
    #[serde(default)]
    pub defensive_dualwielding: bool,
    #[serde(default)]
    pub talents: Vec<TalentSelection>,
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
    pub armor_id: ArmorId,
    pub weapon_material_tier: i32,
    pub armor_material_tier: i32,
    pub projectile_material_tier: i32,
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
    pub defensive_dualwielding: bool,
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
            armor_id: ArmorId::new(0),
            weapon_material_tier: 0,
            armor_material_tier: 0,
            projectile_material_tier: 0,
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
            defensive_dualwielding: false,
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
    defense_bonus: i32,
    defense_bonus_by_weapon: HashMap<WeaponId, i32>,
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
    reach_bonus_by_group: HashMap<WeaponGroup, i32>,
    range_distance_multiplier: f32,
}

impl Default for TalentModifiers {
    fn default() -> Self {
        Self {
            hp_bonus: 0,
            armor_dr_bonus: 0,
            defense_bonus: 0,
            defense_bonus_by_weapon: HashMap::new(),
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
            reach_bonus_by_group: HashMap::new(),
            range_distance_multiplier: 1.0,
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

    fn defense_bonus_for_weapon(&self, weapon_id: WeaponId) -> i32 {
        *self.defense_bonus_by_weapon.get(&weapon_id).unwrap_or(&0)
    }

    fn weapon_speed_bonus_for_weapon(&self, weapon_id: WeaponId) -> i32 {
        *self.weapon_speed_bonus_by_weapon.get(&weapon_id).unwrap_or(&0)
    }

    fn reach_bonus_for_group(&self, group: WeaponGroup) -> i32 {
        *self.reach_bonus_by_group.get(&group).unwrap_or(&0)
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

pub fn evaluate_talent_requirements(
    spec: &TalentSpec,
    context: &TalentContext<'_>,
) -> Vec<TalentRequirementFailure> {
    let mut failures = Vec::new();
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
                let current_rank = context
                    .talents
                    .iter()
                    .filter(|selection| selection.id == *id)
                    .map(|selection| selection.rank.max(1))
                    .max()
                    .unwrap_or(0);
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

fn weapon_id_by_name(catalog: &WeaponCatalog, name: &str) -> Option<WeaponId> {
    catalog
        .entries()
        .iter()
        .position(|weapon| weapon.name.eq_ignore_ascii_case(name))
        .and_then(|idx| catalog.id_from_index(idx))
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
    for selection in &player.talents {
        let Some(spec) = find_talent(talent_catalog, &selection.id) else {
            continue;
        };
        if !evaluate_talent_requirements(spec, &context).is_empty() {
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
                TalentEffect::AttackBonusWeapon { amount } => {
                    if let Some(weapon_name) = selection.weapon.as_deref() {
                        if let Some(weapon_id) = weapon_id_by_name(weapon_catalog, weapon_name) {
                            let entry =
                                modifiers.attack_bonus_by_weapon.entry(weapon_id).or_insert(0);
                            *entry += amount * rank;
                        }
                    }
                }
                TalentEffect::DamageBonusWeapon { amount } => {
                    if let Some(weapon_name) = selection.weapon.as_deref() {
                        if let Some(weapon_id) = weapon_id_by_name(weapon_catalog, weapon_name) {
                            let entry =
                                modifiers.damage_bonus_by_weapon.entry(weapon_id).or_insert(0);
                            *entry += amount * rank;
                        }
                    }
                }
                TalentEffect::DefenseBonusWeapon { amount } => {
                    if let Some(weapon_name) = selection.weapon.as_deref() {
                        if let Some(weapon_id) = weapon_id_by_name(weapon_catalog, weapon_name) {
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
                        let Some(weapon_id) = weapon_id_by_name(weapon_catalog, weapon_name) else {
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
                        if let Some(weapon_id) = weapon_id_by_name(weapon_catalog, weapon_name) {
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
                TalentEffect::ShieldDefenseBonus { amount } => {
                    modifiers.shield_defense_bonus += amount * rank;
                }
                TalentEffect::ShieldCoverValueAdjustment { amount } => {
                    modifiers.shield_cover_value_adjustment += amount * rank;
                }
            }
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

pub fn clamp_mastery(value: i32) -> i32 {
    value.clamp(0, 6)
}

pub fn shield_equipped(player: &PlayerConfig, weapon: &WeaponPreset) -> bool {
    can_equip_shield(player, weapon) && player.shield_id.index() > 0
}

pub fn defensive_dualwielding_active(player: &PlayerConfig, weapon: &WeaponPreset) -> bool {
    player.defensive_dualwielding
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
    let armor_adjustments = armor_talent_adjustments(character.equipment.armor.as_ref(), &modifiers);
    let defensive_dualwielding = defensive_dualwielding_active(player, weapon);
    let defense_bonus_weapon =
        modifiers.defense_bonus_for_weapon(player.weapon_id)
            * if defensive_dualwielding { 2 } else { 1 };
    let mut derived = character.derived();
    derived.speed_mod += armor_adjustments.speed_mod_bonus;
    derived.initiative_mod += armor_adjustments.initiative_mod_bonus;
    derived.base_dv += armor_adjustments.base_dv_bonus;
    derived.hit_points = (derived.hit_points as i32 + modifiers.hp_bonus).max(1) as u32;
    derived.armor_dr =
        (derived.armor_dr + armor_adjustments.armor_dr_bonus + modifiers.armor_dr_bonus).max(0);
    derived.base_dv += modifiers.defense_bonus + defense_bonus_weapon;
    let roll = roll_summary(
        player,
        weapon,
        &character,
        &derived,
        &modifiers,
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
        + modifiers.damage_bonus_for_weapon(player.weapon_id);
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
) -> [Combatant; 2] {
    [
        build_combatant(
            &players[0],
            weapon_catalog,
            armor_catalog,
            shield_catalog,
            npc_presets,
            talent_catalog,
        ),
        build_combatant(
            &players[1],
            weapon_catalog,
            armor_catalog,
            shield_catalog,
            npc_presets,
            talent_catalog,
        ),
    ]
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
    let armor_adjustments = armor_talent_adjustments(character.equipment.armor.as_ref(), &modifiers);
    let mut derived = character.derived();
    derived.speed_mod += armor_adjustments.speed_mod_bonus;
    derived.initiative_mod += armor_adjustments.initiative_mod_bonus;
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
    let is_ranged_weapon = is_ranged_weapon(weapon_preset);
    let uses_projectiles =
        uses_projectiles(&weapon_preset.name, weapon_preset.ammunition.is_some());
    let (material_attack_bonus, material_damage_bonus) = material_bonuses(
        player.weapon_material_tier,
        player.projectile_material_tier,
        is_ranged_weapon,
        uses_projectiles,
    );
    let attack_mastery = effective_attack_mastery(player);
    let mut defensive_dualwielding = defensive_dualwielding_active(player, weapon_preset);
    let defense_mastery = effective_defense_mastery(player, weapon_preset)
        * if defensive_dualwielding { 2 } else { 1 };
    let defense_bonus_weapon =
        modifiers.defense_bonus_for_weapon(player.weapon_id)
            * if defensive_dualwielding { 2 } else { 1 };
    let defense_bonus = modifiers.defense_bonus;
    let damage_mastery = effective_damage_mastery(player);
    let mut attack_bonus = derived.attack_bonus
        + material_attack_bonus
        + attack_mastery
        + modifiers.attack_bonus_for_weapon(player.weapon_id);
    let mut defense_mod = derived.base_dv + defense_mastery + defense_bonus + defense_bonus_weapon;
    let mut armor_dr = (derived.armor_dr + modifiers.armor_dr_bonus).max(0);
    let mut strength_damage =
        strength_damage_for_weapon(weapon_preset, character.ability_mods.strength.damage)
            + two_hand_damage_bonus
            + material_damage_bonus
        + damage_mastery
        + modifiers.damage_bonus_for_weapon(player.weapon_id);
    if !is_ranged_weapon {
        strength_damage += armor_adjustments.heavy_armor_damage_bonus;
    }
    let mut max_hp = (derived.hit_points as i32 + modifiers.hp_bonus).max(1);
    let mut threshold_of_pain = threshold_of_pain(max_hp, player.level);
    let mut shield_name = shield_data.map(|shield| shield.name.to_string());
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
        defense_mod = preset.defense_mod;
        armor_dr = preset.armor_dr;
        strength_damage = preset.damage_bonus;
        max_hp = preset.hp.max(1);
        threshold_of_pain = preset.top.max(1);
        shield_name = None;
        shield_defense_bonus = 0;
        shield_dr = 0;
        shield_cover_value = None;
        shield_breakage = None;
        ranged_defense_mod = 0;
        trauma_die_sides = 20;
        trauma_die_penetrating = false;
        defensive_dualwielding = false;
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
    let sheet = CombatantSheet {
        name,
        offense: OffenseProfile {
            attack_bonus,
            strength_damage,
            weapon: WeaponProfile {
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
                uses_projectiles,
            },
        },
        defense: DefenseProfile {
            defense_mod,
            ranged_defense_mod,
            armor_dr,
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
            defensive_dualwielding,
        },
    };

    Combatant::new(sheet)
}

pub fn stop_distance_for_players(
    players: &[PlayerConfig; 2],
    weapon_catalog: &WeaponCatalog,
    talent_catalog: &TalentCatalog,
) -> f32 {
    let reach_for_player = |player: &PlayerConfig| {
        weapon_catalog
            .get(player.weapon_id)
            .map(|weapon| {
                weapon
                    .range_bands_feet
                    .map(sim::max_range_for_bands)
                    .or_else(|| sim::max_range_for_weapon_name(&weapon.name))
                    .unwrap_or_else(|| {
                        let modifiers =
                            resolve_talent_modifiers(player, talent_catalog, weapon_catalog);
                        let reach_bonus = modifiers.reach_bonus_for_group(weapon.group) as f32;
                        (weapon.reach_ft + reach_bonus).max(1.0)
                    })
            })
            .unwrap_or(1.0)
    };
    let reach_a = reach_for_player(&players[0]);
    let reach_b = reach_for_player(&players[1]);
    reach_a.max(reach_b)
}


fn leak_str(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

fn can_equip_shield(player: &PlayerConfig, weapon: &WeaponPreset) -> bool {
    weapon.handedness == WeaponHandedness::OneHanded
        && !player.two_hand_grip
        && !defensive_dualwielding_active(player, weapon)
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
        name: leak_str(shield.name),
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

    fn one_handed_weapon_id(weapons: &WeaponCatalog) -> WeaponId {
        weapons
            .entries()
            .iter()
            .position(|weapon| weapon.handedness == WeaponHandedness::OneHanded)
            .and_then(|idx| weapons.id_from_index(idx))
            .unwrap_or(WeaponId::new(0))
    }

    #[test]
    fn talent_requirements_block_min_level() {
        let spec = TalentSpec {
            id: "requires_level".to_string(),
            name: "Requires Level".to_string(),
            description: "".to_string(),
            cost_bp: None,
            category: "Test".to_string(),
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
            category: "Test".to_string(),
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
            category: "Test".to_string(),
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
            name: "Test",
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
                category: "Test".to_string(),
                requirements: Vec::new(),
                max_rank: 1,
                effects: vec![TalentEffect::DefenseBonusWeapon { amount: 2 }],
            },
            TalentSpec {
                id: "dodge".to_string(),
                name: "Dodge".to_string(),
                description: "".to_string(),
                cost_bp: None,
                category: "Test".to_string(),
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
}
