use crate::character::{
    AbilityScore, AbilitySet, Armor, Character, DerivedStats, Equipment, Progression, Shield,
    Weapon, WeaponGroup, WeaponMastery,
};
use crate::core::catalog::Catalog;
use crate::core::rules::DamageExprCache;
use crate::core::types::{TalentEffect, TalentSelection, TalentSpec};
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

#[derive(Clone, Debug, Default)]
struct TalentModifiers {
    hp_bonus: i32,
    armor_dr_bonus: i32,
    defense_bonus: i32,
    defense_bonus_by_weapon: HashMap<WeaponId, i32>,
    allow_dex_ranged: bool,
    trauma_die_override: Option<TraumaDieOverride>,
    attack_bonus_by_weapon: HashMap<WeaponId, i32>,
    damage_bonus_by_weapon: HashMap<WeaponId, i32>,
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

fn weapon_id_by_name(catalog: &WeaponCatalog, name: &str) -> Option<WeaponId> {
    catalog
        .entries()
        .iter()
        .position(|weapon| weapon.name.eq_ignore_ascii_case(name))
        .and_then(|idx| catalog.id_from_index(idx))
}

fn resolve_talent_modifiers(
    player: &PlayerConfig,
    talent_catalog: &TalentCatalog,
    weapon_catalog: &WeaponCatalog,
) -> TalentModifiers {
    let mut modifiers = TalentModifiers::default();
    for selection in &player.talents {
        let Some(spec) = find_talent(talent_catalog, &selection.id) else {
            continue;
        };
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
    let defensive_dualwielding = defensive_dualwielding_active(player, weapon);
    let defense_bonus_weapon =
        modifiers.defense_bonus_for_weapon(player.weapon_id)
            * if defensive_dualwielding { 2 } else { 1 };
    let mut derived = character.derived();
    derived.hit_points = (derived.hit_points as i32 + modifiers.hp_bonus).max(1) as u32;
    derived.armor_dr = (derived.armor_dr + modifiers.armor_dr_bonus).max(0);
    derived.base_dv += modifiers.defense_bonus + defense_bonus_weapon;
    let roll = roll_summary(player, weapon, &character, &derived, &modifiers);
    PlayerSummary { derived, roll }
}

fn roll_summary(
    player: &PlayerConfig,
    weapon: &WeaponPreset,
    character: &Character,
    derived: &DerivedStats,
    modifiers: &TalentModifiers,
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
    let strength_damage = strength_damage_for_weapon(weapon, character.ability_mods.strength.damage)
        + two_hand_bonus
        + material_damage_bonus
        + damage_mastery
        + modifiers.damage_bonus_for_weapon(player.weapon_id);

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

    let abilities = AbilitySet {
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
    };

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
    let derived = character.derived();
    let modifiers = resolve_talent_modifiers(player, talent_catalog, weapon_catalog);
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
    let weapon_reach = character
        .equipment
        .weapon
        .as_ref()
        .map(|weapon| weapon.reach_ft)
        .unwrap_or(1.0);
    let armor_is_heavy = character
        .equipment
        .armor
        .as_ref()
        .map(|armor| matches!(armor.armor_type, crate::character::ArmorType::Heavy))
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
        (weapon_preset.jab_speed.unwrap_or(weapon_speed) + speed_mod - speed_mastery)
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
    let mut max_hp = (derived.hit_points as i32 + modifiers.hp_bonus).max(1);
    let mut threshold_of_pain = threshold_of_pain(max_hp, player.level);
    let mut shield_name = shield_data.map(|shield| shield.name.to_string());
    let mut shield_defense_bonus = shield_data.map(|shield| shield.defense_bonus).unwrap_or(0);
    let mut shield_dr = shield_data.map(|shield| shield.dr).unwrap_or(0);
    let mut shield_cover_value = shield_data.map(|shield| shield.cover_value);
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
        (weapon_speed + two_hand_speed_penalty + speed_mod - speed_mastery).max(min_speed)
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
) -> f32 {
    let reach_a = weapon_catalog
        .get(players[0].weapon_id)
        .map(|weapon| {
            weapon
                .range_bands_feet
                .map(sim::max_range_for_bands)
                .or_else(|| sim::max_range_for_weapon_name(&weapon.name))
                .unwrap_or_else(|| weapon.reach_ft.max(1.0))
        })
        .unwrap_or(1.0);
    let reach_b = weapon_catalog
        .get(players[1].weapon_id)
        .map(|weapon| {
            weapon
                .range_bands_feet
                .map(sim::max_range_for_bands)
                .or_else(|| sim::max_range_for_weapon_name(&weapon.name))
                .unwrap_or_else(|| weapon.reach_ft.max(1.0))
        })
        .unwrap_or(1.0);
    reach_a.max(reach_b)
}

pub fn default_weapon_catalog() -> WeaponCatalog {
    Catalog::new(vec![
        // Unarmed
        weapon_preset("Fist", WeaponGroup::Unarmed, 10.0, "(d4p-2)+(d4p-2)", "1 foot", 1.0),
        weapon_preset("Antler", WeaponGroup::Unarmed, 10.0, "2d6p", "3 feet", 3.0),
        weapon_preset("Claw", WeaponGroup::Unarmed, 5.0, "1d8p", "1 foot", 1.0),
        weapon_preset("Fang", WeaponGroup::Unarmed, 10.0, "1d10p", "0.5 feet", 0.5),
        weapon_preset("Cestus", WeaponGroup::Unarmed, 10.0, "2d4p", "1 foot", 1.0),
        weapon_preset(
            "Gauntlet",
            WeaponGroup::Unarmed,
            10.0,
            "(d4p-1)+(d4p-1)",
            "1 foot",
            1.0,
        ),
        weapon_preset(
            "Spiked gauntlet",
            WeaponGroup::Unarmed,
            10.0,
            "1d8p",
            "1 foot",
            1.0,
        ),
        // Axes
        weapon_preset("Battle axe", WeaponGroup::Axes, 12.0, "4d3p^2", "3 feet", 3.0),
        weapon_preset(
            "Executioner's axe",
            WeaponGroup::Axes,
            18.0,
            "3d8p+3^2",
            "4 feet",
            4.0,
        ),
        weapon_preset("Greataxe", WeaponGroup::Axes, 14.0, "3d6p+3^2", "3.5 feet", 3.5),
        weapon_preset("Hand axe", WeaponGroup::Axes, 8.0, "d4p+d6p", "1.5 feet", 1.5),
        weapon_preset("Khopesh", WeaponGroup::Axes, 8.0, "2d6p", "2 feet", 2.0),
        weapon_preset("Military pick", WeaponGroup::Axes, 12.0, "3d4p^2", "3 feet", 3.0),
        weapon_preset(
            "Horseman's pick",
            WeaponGroup::Axes,
            8.0,
            "d4p+d6p^1",
            "1.5 feet",
            1.5,
        ),
        weapon_preset("Scythe", WeaponGroup::Axes, 15.0, "2d6p+3", "4.5 feet", 4.5),
        weapon_preset("Sickle", WeaponGroup::Axes, 8.0, "d6p+d3p", "1.5 feet", 1.5),
        weapon_preset(
            "Throwing axe",
            WeaponGroup::Axes,
            7.0,
            "d4p+d6p",
            "1/60 feet",
            0.0,
        ),
        // Basic
        weapon_preset("Club", WeaponGroup::Basic, 10.0, "d6p+d4p", "2.5 feet", 2.5),
        weapon_preset("Dart", WeaponGroup::Basic, 5.0, "d4p", "0.5/40 feet", 0.0),
        weapon_preset_with_ammo(
            "Sling",
            WeaponGroup::Basic,
            10.0,
            "d4p+d6p",
            "160 feet",
            0.0,
            "Stone",
        ),
        weapon_preset("Staff", WeaponGroup::Basic, 13.0, "2d4p+3", "8 feet", 8.0),
        // Blunt
        weapon_preset("Greatclub", WeaponGroup::Blunt, 16.0, "d20p+3^1", "5 feet", 5.0),
        weapon_preset(
            "Greathammer",
            WeaponGroup::Blunt,
            20.0,
            "d8p+2d10p+3^2",
            "4.5 feet",
            4.5,
        ),
        weapon_preset("Hammer", WeaponGroup::Blunt, 8.0, "2d6p^1", "1.5 feet", 1.5),
        weapon_preset("Warhammer", WeaponGroup::Blunt, 12.0, "d8p+d10p^1", "2.5 feet", 2.5),
        weapon_preset("Mace", WeaponGroup::Blunt, 11.0, "d6p+d8p^2", "2 feet", 2.0),
        weapon_preset(
            "Horseman's mace",
            WeaponGroup::Blunt,
            10.0,
            "2d6p^1",
            "1.5 feet",
            1.5,
        ),
        weapon_preset("Maul", WeaponGroup::Blunt, 15.0, "2d12p+3^2", "3 feet", 3.0),
        weapon_preset("Morningstar", WeaponGroup::Blunt, 11.0, "2d8p", "3 feet", 3.0),
        // Bows
        weapon_preset_with_ammo(
            "Longbow",
            WeaponGroup::Bows,
            12.0,
            "2d8p",
            "210 feet",
            0.0,
            "Heavy arrow",
        ),
        weapon_preset_with_ammo(
            "Recurve bow",
            WeaponGroup::Bows,
            11.0,
            "3d4p",
            "150 feet",
            0.0,
            "Light arrow",
        ),
        weapon_preset_with_ammo(
            "Shortbow",
            WeaponGroup::Bows,
            12.0,
            "2d6p",
            "150 feet",
            0.0,
            "Light arrow",
        ),
        weapon_preset_with_ammo(
            "Warbow",
            WeaponGroup::Bows,
            20.0,
            "3d6p^1",
            "300 feet",
            0.0,
            "Heavy arrow",
        ),
        // Crossbows
        weapon_preset_with_ammo(
            "Arbalest",
            WeaponGroup::Crossbows,
            90.0,
            "3d8p^1",
            "400 feet",
            0.0,
            "Heavy quarrel",
        ),
        weapon_preset_with_ammo(
            "Light crossbow",
            WeaponGroup::Crossbows,
            20.0,
            "2d6p",
            "180 feet",
            0.0,
            "Light quarrel",
        ),
        weapon_preset_with_ammo(
            "Hand crossbow",
            WeaponGroup::Crossbows,
            15.0,
            "2d4p",
            "120 feet",
            0.0,
            "Light quarrel",
        ),
        weapon_preset_with_ammo(
            "Heavy crossbow",
            WeaponGroup::Crossbows,
            60.0,
            "2d10p",
            "250 feet",
            0.0,
            "Heavy quarrel",
        ),
        // Double weapons
        weapon_preset(
            "Double axe",
            WeaponGroup::Double,
            13.0,
            "4d3p^2 and 4d3p^2",
            "3.5 feet",
            3.5,
        ),
        weapon_preset(
            "Double scimitar",
            WeaponGroup::Double,
            10.0,
            "2d8p and 2d8p",
            "3.5 feet",
            3.5,
        ),
        weapon_preset(
            "Dual scythe",
            WeaponGroup::Double,
            16.0,
            "2d6p and 2d6p",
            "4 feet",
            4.0,
        ),
        weapon_preset(
            "Hooked hammer",
            WeaponGroup::Double,
            14.0,
            "d8p+d10p^1 and 3d4p^2",
            "3 feet",
            3.0,
        ),
        weapon_preset(
            "Double mace",
            WeaponGroup::Double,
            12.0,
            "d10p+d10p^1 and d10p+d10p^1",
            "3.5 feet",
            3.5,
        ),
        weapon_preset(
            "Double spear",
            WeaponGroup::Double,
            11.0,
            "d6p+d8p+3 and d6p+d8p+3",
            "6 feet",
            6.0,
        ),
        weapon_preset(
            "Double spear (short)",
            WeaponGroup::Double,
            10.0,
            "2d6p and 2d6p",
            "5 feet",
            5.0,
        ),
        weapon_preset(
            "Double sword",
            WeaponGroup::Double,
            12.0,
            "2d8p+3 and 2d8p+3",
            "4 feet",
            4.0,
        ),
        weapon_preset(
            "Bola",
            WeaponGroup::Ensnaring,
            10.0,
            "d4p",
            "50 feet",
            0.0,
        ),
        weapon_preset(
            "Lasso",
            WeaponGroup::Ensnaring,
            15.0,
            "-",
            "50 feet",
            0.0,
        ),
        weapon_preset(
            "Net",
            WeaponGroup::Ensnaring,
            20.0,
            "-",
            "15 feet",
            0.0,
        ),
        // Lashes
        weapon_preset("Flail", WeaponGroup::Lashes, 13.0, "2d8p^1", "4 feet", 4.0),
        weapon_preset(
            "Heavy flail",
            WeaponGroup::Lashes,
            15.0,
            "d10p+d12p+3^2",
            "4 feet",
            4.0,
        ),
        weapon_preset("Scourge", WeaponGroup::Lashes, 9.0, "2d4p", "1.5 feet", 1.5),
        weapon_preset(
            "Weighted scourge",
            WeaponGroup::Lashes,
            11.0,
            "d8p+d4p",
            "1.5 feet",
            1.5,
        ),
        weapon_preset("Whip", WeaponGroup::Lashes, 8.0, "1d6p", "1.5 feet", 1.5),
        weapon_preset(
            "Weighted whip",
            WeaponGroup::Lashes,
            10.0,
            "d10p",
            "1.5 feet",
            1.5,
        ),
        // Large swords
        weapon_preset("Sabre", WeaponGroup::LargeSwords, 8.0, "d6p+d8p", "3 feet", 3.0),
        weapon_preset("Scimitar", WeaponGroup::LargeSwords, 9.0, "2d8p", "3 feet", 3.0),
        weapon_preset("Spatha", WeaponGroup::LargeSwords, 9.0, "d6p+d8p", "3 feet", 3.0),
        weapon_preset(
            "Broad sword",
            WeaponGroup::LargeSwords,
            10.0,
            "d8p+d10p+3^1",
            "3 feet",
            3.0,
        ),
        weapon_preset("Falchion", WeaponGroup::LargeSwords, 9.0, "d10p+d6p", "3 feet", 3.0),
        weapon_preset("Longsword", WeaponGroup::LargeSwords, 10.0, "2d8p", "3 feet", 3.0),
        weapon_preset("Scalpel", WeaponGroup::LargeSwords, 9.0, "d10p+d6p", "3 feet", 3.0),
        weapon_preset("Sword, bastard", WeaponGroup::LargeSwords, 12.0, "2d10p+3", "4 feet", 4.0),
        weapon_preset("Sword, broad", WeaponGroup::LargeSwords, 10.0, "d8p+d10p+3^1", "3 feet", 3.0),
        weapon_preset("Sword, khopesh", WeaponGroup::LargeSwords, 9.0, "2d6p", "3 feet", 3.0),
        weapon_preset("Sword, scimitar", WeaponGroup::LargeSwords, 9.0, "2d8p", "3 feet", 3.0),
        weapon_preset("Sword, spatha", WeaponGroup::LargeSwords, 9.0, "d6p+d8p", "3 feet", 3.0),
        weapon_preset("Sword, tulwar", WeaponGroup::LargeSwords, 9.0, "d6p+d8p", "3 feet", 3.0),
        weapon_preset(
            "Sword, two-handed",
            WeaponGroup::LargeSwords,
            15.0,
            "3d6p+3^1",
            "5 feet",
            5.0,
        ),
        weapon_preset(
            "Sword, two-handed, claymore",
            WeaponGroup::LargeSwords,
            16.0,
            "3d6p+3^1",
            "5.5 feet",
            5.5,
        ),
        // Small swords
        weapon_preset("Dagger", WeaponGroup::SmallSwords, 7.0, "2d4p", "1 foot", 1.0),
        weapon_preset(
            "Main-gauche",
            WeaponGroup::SmallSwords,
            6.0,
            "d6p",
            "1 foot",
            1.0,
        ),
        weapon_preset(
            "Ninja-to",
            WeaponGroup::SmallSwords,
            8.0,
            "d8p",
            "2 feet",
            2.0,
        ),
        weapon_preset("Knife", WeaponGroup::SmallSwords, 7.0, "d6p", "1 foot", 1.0),
        weapon_preset(
            "Short sword",
            WeaponGroup::SmallSwords,
            8.0,
            "d6p+d8p",
            "2 feet",
            2.0,
        ),
        weapon_preset("Stiletto", WeaponGroup::SmallSwords, 6.0, "d4p+d6p", "1 foot", 1.0),
        weapon_preset(
            "Sword, arming",
            WeaponGroup::SmallSwords,
            8.0,
            "d6p+d8p",
            "2 feet",
            2.0,
        ),
        weapon_preset(
            "Sword, cutlass",
            WeaponGroup::SmallSwords,
            8.0,
            "d6p+d8p",
            "2 feet",
            2.0,
        ),
        weapon_preset(
            "Sword, rapier",
            WeaponGroup::SmallSwords,
            8.0,
            "d6p+d6p",
            "2.5 feet",
            2.5,
        ),
        // Polearms
        weapon_preset("Bardiche", WeaponGroup::Polearms, 14.0, "4d4p+3", "5 feet", 5.0),
        weapon_preset("Glaive", WeaponGroup::Polearms, 14.0, "5d4p+3", "8 feet", 8.0),
        weapon_preset("Guisarme", WeaponGroup::Polearms, 12.0, "2d10p", "8 feet", 8.0),
        weapon_preset(
            "Halberd",
            WeaponGroup::Polearms,
            15.0,
            "4d4p+3",
            "7 feet",
            7.0,
        ),
        weapon_preset(
            "Pole axe",
            WeaponGroup::Polearms,
            15.0,
            "4d3p+3^1",
            "8 feet",
            8.0,
        ),
        weapon_preset("Ranseur", WeaponGroup::Polearms, 13.0, "d12p+d8p+3", "8 feet", 8.0),
        weapon_preset("Sovnya", WeaponGroup::Polearms, 16.0, "3d8p+3", "10 feet", 10.0),
        weapon_preset("Voulge", WeaponGroup::Polearms, 13.0, "2d8p+3", "8 feet", 8.0),
        weapon_preset(
            "Glaive-guisarme",
            WeaponGroup::Polearms,
            15.0,
            "3d6p+3",
            "8 feet",
            8.0,
        ),
        weapon_preset(
            "Bec de corbin",
            WeaponGroup::Polearms,
            12.0,
            "d10p+d8p+3",
            "7 feet",
            7.0,
        ),
        weapon_preset("Bill", WeaponGroup::Polearms, 12.0, "d8p+d10p+3^1", "8 feet", 8.0),
        weapon_preset(
            "Fauchard",
            WeaponGroup::Polearms,
            14.0,
            "2d12p+3",
            "9 feet",
            9.0,
        ),
        weapon_preset(
            "Fauchard-fork",
            WeaponGroup::Polearms,
            12.0,
            "d10p+d8p+3^1",
            "9 feet",
            9.0,
        ),
        weapon_preset(
            "Guisarme-voulge",
            WeaponGroup::Polearms,
            12.0,
            "d8p+d10p+3^1",
            "8 feet",
            8.0,
        ),
        weapon_preset(
            "Glaive-guisarme (short)",
            WeaponGroup::Polearms,
            11.0,
            "d10p+d6p",
            "7 feet",
            7.0,
        ),
        weapon_preset(
            "Military fork",
            WeaponGroup::Polearms,
            11.0,
            "d8p+d6p",
            "7 feet",
            7.0,
        ),
        // Spears
        weapon_preset("Hasta", WeaponGroup::Spears, 12.0, "2d6p", "7 feet", 7.0),
        weapon_preset("Javelin", WeaponGroup::Spears, 7.0, "d12p", "5/100 feet", 0.0),
        weapon_preset("Lance", WeaponGroup::Spears, 12.0, "2d8p^2", "10 feet", 10.0),
        weapon_preset(
            "Long spear",
            WeaponGroup::Spears,
            15.0,
            "d8p+d10p+3^1",
            "10 feet",
            10.0,
        ),
        weapon_preset("Pike", WeaponGroup::Spears, 18.0, "2d6p+3", "18 feet", 18.0),
        weapon_preset("Pilum", WeaponGroup::Spears, 8.0, "2d6p", "5/80 feet", 0.0),
        weapon_preset("Spear", WeaponGroup::Spears, 12.0, "2d6p", "13 feet", 13.0),
        weapon_preset("Spear, short", WeaponGroup::Spears, 10.0, "2d6p", "7 feet", 7.0),
        weapon_preset(
            "Spear, long",
            WeaponGroup::Spears,
            12.0,
            "2d6p",
            "15 feet",
            15.0,
        ),
        weapon_preset(
            "Spetum",
            WeaponGroup::Spears,
            12.0,
            "2d6p+3",
            "10 feet",
            10.0,
        ),
        weapon_preset(
            "Trident",
            WeaponGroup::Spears,
            12.0,
            "d6p+d8p+3",
            "6 feet",
            6.0,
        ),
    ])
}

pub fn default_armor_catalog() -> ArmorCatalog {
    Catalog::new(vec![ArmorEntry {
        label: "None".to_string(),
        armor: None,
    }])
}

pub fn default_shield_catalog() -> ShieldCatalog {
    Catalog::new(vec![ShieldEntry {
        label: "None".to_string(),
        shield: None,
    }])
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

fn weapon_preset(
    name: &'static str,
    group: WeaponGroup,
    speed: f32,
    damage_expr: &'static str,
    reach_label: &'static str,
    reach_ft: f32,
) -> WeaponPreset {
    WeaponPreset {
        name: name.to_string(),
        group,
        speed,
        speed_label: format!("{speed:.0}"),
        jab_speed: None,
        jab_speed_label: None,
        jab_special_expr: None,
        damage_expr: damage_expr.to_string(),
        shield_damage_expr: None,
        reach_label: reach_label.to_string(),
        reach_ft,
        range_bands_feet: None,
        armor_pen: 0,
        defense_bonus_always: false,
        size: WeaponSize::Medium,
        handedness: WeaponHandedness::OneHanded,
        ammunition: None,
    }
}

fn weapon_preset_with_ammo(
    name: &'static str,
    group: WeaponGroup,
    speed: f32,
    damage_expr: &'static str,
    reach_label: &'static str,
    reach_ft: f32,
    ammunition: &'static str,
) -> WeaponPreset {
    let mut preset = weapon_preset(name, group, speed, damage_expr, reach_label, reach_ft);
    preset.ammunition = Some(ammunition.to_string());
    preset
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
        crate::data::load_catalogs().unwrap_or_else(|_| {
            (
                default_weapon_catalog(),
                default_armor_catalog(),
                default_shield_catalog(),
            )
        })
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
                max_rank: 1,
                effects: vec![TalentEffect::DefenseBonusWeapon { amount: 2 }],
            },
            TalentSpec {
                id: "dodge".to_string(),
                name: "Dodge".to_string(),
                description: "".to_string(),
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
