use crate::character::{
    AbilityScore, AbilitySet, Armor, Character, DerivedStats, Equipment, Progression, Weapon,
    WeaponGroup, WeaponMastery,
};
use crate::sim::{self, Combatant};
use eframe::egui::Color32;
use serde::Deserialize;
use std::fs;

const EMBEDDED_WEAPONS_JSON: &str = include_str!("../data/weapons.json");
const EMBEDDED_ARMOR_JSON: &str = include_str!("../data/armor.json");
const EMBEDDED_MATERIALS_JSON: &str = include_str!("../data/materials.json");
const EMBEDDED_NPC_PRESETS_JSON: &str = include_str!("../data/npc_presets.json");

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
    pub reach_label: String,
    pub reach_ft: f32,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponHandedness {
    OneHanded,
    TwoHanded,
}

#[derive(Clone)]
pub struct ArmorEntry {
    pub label: String,
    pub armor: Option<Armor>,
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

#[derive(Deserialize)]
struct NpcPresetsFile {
    presets: Vec<NpcPreset>,
}

#[derive(Clone)]
pub struct PlayerConfig {
    pub name: String,
    pub color: Color32,
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
    pub weapon_index: usize,
    pub armor_index: usize,
    pub weapon_material_tier: i32,
    pub armor_material_tier: i32,
    pub projectile_material_tier: i32,
    pub npc_preset: Option<usize>,
    pub two_hand_grip: bool,
    pub use_jab: bool,
}

impl PlayerConfig {
    pub fn new(name: &str, color: Color32, weapon_index: usize) -> Self {
        Self {
            name: name.to_string(),
            color,
            level: 1,
            progression: Progression::default(),
            base_hp: 10,
            move_speed: 5.0,
            strength_base: 10,
            strength_pct: 1,
            dex_base: 10,
            dex_pct: 1,
            intelligence: 10,
            wisdom: 10,
            constitution: 10,
            looks: 10,
            charisma: 10,
            weapon_index,
            armor_index: 0,
            weapon_material_tier: 0,
            armor_material_tier: 0,
            projectile_material_tier: 0,
            npc_preset: None,
            two_hand_grip: false,
            use_jab: false,
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

pub struct RollSummary {
    pub attack_bonus: i32,
    pub strength_damage: i32,
    pub is_ranged_weapon: bool,
}

pub struct PlayerSummary {
    pub derived: DerivedStats,
    pub roll: RollSummary,
}

pub fn player_summary(
    player: &PlayerConfig,
    weapon_catalog: &[WeaponPreset],
    armor_catalog: &[ArmorEntry],
) -> PlayerSummary {
    let weapon = &weapon_catalog[player.weapon_index];
    let character = build_character(player, weapon_catalog, armor_catalog);
    let derived = character.derived();
    let roll = roll_summary(player, weapon, &character, &derived);
    PlayerSummary { derived, roll }
}

fn roll_summary(
    player: &PlayerConfig,
    weapon: &WeaponPreset,
    character: &Character,
    derived: &DerivedStats,
) -> RollSummary {
    let is_ranged_weapon = is_ranged_weapon(&weapon.name);
    let uses_projectiles = uses_projectiles(&weapon.name, weapon.ammunition.is_some());
    let (material_attack_bonus, material_damage_bonus) = material_bonuses(
        player.weapon_material_tier,
        player.projectile_material_tier,
        is_ranged_weapon,
        uses_projectiles,
    );
    let attack_bonus = derived.attack_bonus + material_attack_bonus;
    let is_two_handed = weapon.handedness == WeaponHandedness::TwoHanded;
    let can_two_hand = weapon.handedness == WeaponHandedness::OneHanded
        && (weapon.size == WeaponSize::Medium || weapon.size == WeaponSize::Large);
    let effective_two_hand = is_two_handed || (player.two_hand_grip && can_two_hand);
    let two_hand_bonus = if effective_two_hand && can_two_hand { 3 } else { 0 };
    let strength_damage = strength_damage_for_weapon(
        &weapon.name,
        character.ability_mods.strength.damage,
    ) + two_hand_bonus
        + material_damage_bonus;

    RollSummary {
        attack_bonus,
        strength_damage,
        is_ranged_weapon,
    }
}

pub fn build_character(
    player: &PlayerConfig,
    weapon_catalog: &[WeaponPreset],
    armor_catalog: &[ArmorEntry],
) -> Character {
    let weapon_preset = &weapon_catalog[player.weapon_index];
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
        .get(player.armor_index)
        .and_then(|entry| entry.armor.clone());
    let armor = armor.map(|armor| apply_armor_material_tier(armor, player.armor_material_tier));

    let abilities = AbilitySet {
        strength: AbilityScore::new(player.strength_base, player.strength_pct),
        intelligence: player.intelligence,
        wisdom: player.wisdom,
        dexterity: AbilityScore::new(player.dex_base, player.dex_pct),
        constitution: player.constitution,
        looks: player.looks,
        charisma: player.charisma,
    };

    let mastery = WeaponMastery {
        group: weapon_preset.group,
        points: Default::default(),
        base_threshold: base_weapon_threshold(weapon_preset.group),
    };

    let equipment = Equipment {
        weapon: Some(weapon),
        shield: None,
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
    weapon_catalog: &[WeaponPreset],
    armor_catalog: &[ArmorEntry],
    npc_presets: &[NpcPreset],
) -> [Combatant; 2] {
    [
        build_combatant(&players[0], weapon_catalog, armor_catalog, npc_presets),
        build_combatant(&players[1], weapon_catalog, armor_catalog, npc_presets),
    ]
}

pub fn build_combatant(
    player: &PlayerConfig,
    weapon_catalog: &[WeaponPreset],
    armor_catalog: &[ArmorEntry],
    npc_presets: &[NpcPreset],
) -> Combatant {
    let weapon_preset = &weapon_catalog[player.weapon_index];
    let character = build_character(player, weapon_catalog, armor_catalog);
    let derived = character.derived();
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

    let is_two_handed = weapon_preset.handedness == WeaponHandedness::TwoHanded;
    let can_two_hand = weapon_preset.handedness == WeaponHandedness::OneHanded
        && (weapon_preset.size == WeaponSize::Medium || weapon_preset.size == WeaponSize::Large);
    let effective_two_hand = is_two_handed || (player.two_hand_grip && can_two_hand);
    let two_hand_damage_bonus = if effective_two_hand && can_two_hand { 3 } else { 0 };
    let two_hand_speed_bonus = if effective_two_hand && can_two_hand { 2.0 } else { 0.0 };
    let use_jab = player.use_jab && weapon_preset.jab_speed.is_some();
    let jab_speed = weapon_preset.jab_speed.unwrap_or(weapon_speed);
    let jab_special_expr = if use_jab {
        weapon_preset.jab_special_expr.clone()
    } else {
        None
    };

    let mut name = character.name;
    let is_ranged_weapon = is_ranged_weapon(&weapon_name);
    let uses_projectiles =
        uses_projectiles(&weapon_preset.name, weapon_preset.ammunition.is_some());
    let (material_attack_bonus, material_damage_bonus) = material_bonuses(
        player.weapon_material_tier,
        player.projectile_material_tier,
        is_ranged_weapon,
        uses_projectiles,
    );
    let mut attack_bonus = derived.attack_bonus + material_attack_bonus;
    let mut defense_mod = derived.base_dv;
    let mut armor_dr = derived.armor_dr;
    let mut strength_damage = strength_damage_for_weapon(
        &weapon_name,
        character.ability_mods.strength.damage,
    ) + two_hand_damage_bonus
        + material_damage_bonus;
    let mut max_hp = derived.hit_points as i32;
    if let Some(preset) = player.npc_preset.and_then(|idx| npc_presets.get(idx)) {
        name = preset.name.clone();
        attack_bonus = preset.attack_bonus;
        defense_mod = preset.defense_mod;
        armor_dr = preset.armor_dr;
        strength_damage = preset.damage_bonus;
        max_hp = preset.hp.max(1);
    }

    Combatant::new(
        name,
        weapon_name,
        attack_bonus,
        defense_mod,
        armor_dr,
        armor_is_heavy,
        armor_penetration,
        weapon_damage,
        strength_damage,
        if use_jab {
            jab_speed
        } else {
            weapon_speed + two_hand_speed_bonus
        },
        weapon_reach,
        player.move_speed,
        effective_two_hand,
        use_jab,
        jab_special_expr,
        has_weapon,
        weapon_defense_always,
        max_hp,
    )
}

pub fn stop_distance_for_players(players: &[PlayerConfig; 2], weapon_catalog: &[WeaponPreset]) -> f32 {
    let reach_a = weapon_catalog
        .get(players[0].weapon_index)
        .map(|weapon| {
            sim::max_range_for_weapon(&weapon.name).unwrap_or_else(|| weapon.reach_ft.max(1.0))
        })
        .unwrap_or(1.0);
    let reach_b = weapon_catalog
        .get(players[1].weapon_index)
        .map(|weapon| {
            sim::max_range_for_weapon(&weapon.name).unwrap_or_else(|| weapon.reach_ft.max(1.0))
        })
        .unwrap_or(1.0);
    reach_a.max(reach_b)
}

pub fn default_weapon_catalog() -> Vec<WeaponPreset> {
    vec![
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
    ]
}

pub fn default_armor_catalog() -> Vec<ArmorEntry> {
    vec![ArmorEntry {
        label: "None".to_string(),
        armor: None,
    }]
}

pub fn load_catalogs() -> Result<(Vec<WeaponPreset>, Vec<ArmorEntry>), String> {
    let weapons = load_weapon_catalog("data/weapons.json")?;
    let armor = load_armor_catalog("data/armor.json")?;
    let _materials = load_materials("data/materials.json")?;
    Ok((weapons, armor))
}

pub fn load_npc_presets(path: &str) -> Result<Vec<NpcPreset>, String> {
    let data = fs::read_to_string(path).unwrap_or_else(|_| EMBEDDED_NPC_PRESETS_JSON.to_string());
    let parsed: NpcPresetsFile = serde_json::from_str(&data).map_err(|err| err.to_string())?;
    Ok(parsed.presets)
}

#[derive(Deserialize)]
struct WeaponsFile {
    weapons: Vec<WeaponJson>,
    #[allow(dead_code)]
    shields: Vec<ShieldJson>,
}

#[derive(Deserialize)]
struct WeaponJson {
    name: String,
    group: String,
    speed: String,
    jab_speed: Option<String>,
    jab_special: Option<String>,
    damage: Option<String>,
    ammunition: Option<String>,
    armor_penetration: Option<i32>,
    defense_bonus_always: Option<bool>,
    #[serde(rename = "reach_or_range")]
    reach_or_range: Option<String>,
    size: String,
    handedness: String,
}

#[derive(Deserialize)]
struct ShieldJson {
    #[allow(dead_code)]
    name: String,
}

#[derive(Deserialize)]
struct ArmorFile {
    armor: Vec<ArmorJson>,
}

#[derive(Deserialize)]
struct ArmorJson {
    name: String,
    region: String,
    damage_reduction: i32,
    defense_adjustment: i32,
    initiative_modifier: i32,
    speed_modifier: i32,
    #[serde(rename = "type")]
    armor_type: String,
    weight_lbs: Option<f32>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct MaterialsFile {
    metals: Vec<MaterialJson>,
    fabrics: Vec<MaterialJson>,
    woods: Vec<MaterialJson>,
}

#[derive(Deserialize)]
struct MaterialJson {
    #[allow(dead_code)]
    tier: i32,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    weight_multiplier: f32,
}

fn load_weapon_catalog(path: &str) -> Result<Vec<WeaponPreset>, String> {
    let data = fs::read_to_string(path).unwrap_or_else(|_| EMBEDDED_WEAPONS_JSON.to_string());
    let parsed: WeaponsFile = serde_json::from_str(&data).map_err(|err| err.to_string())?;
    let mut catalog = Vec::new();
    for entry in parsed.weapons {
        let group = match weapon_group_from_str(&entry.group) {
            Some(group) => group,
            None => continue,
        };
        let size = match weapon_size_from_str(&entry.size) {
            Some(size) => size,
            None => continue,
        };
        let handedness = match weapon_handedness_from_str(&entry.handedness) {
            Some(handedness) => handedness,
            None => continue,
        };
        let (speed_label, jab_label) = split_speed_label(&entry.speed, entry.jab_speed.as_deref());
        let speed_value = parse_leading_number(&speed_label);
        let jab_speed_value = jab_label
            .as_deref()
            .map(parse_leading_number)
            .filter(|value| *value > 0.0);
        let reach_label = entry
            .reach_or_range
            .clone()
            .unwrap_or_else(|| "-".to_string());
        let reach_ft = parse_reach_ft(&reach_label);
        let damage_expr = entry.damage.unwrap_or_else(|| "-".to_string());
        catalog.push(WeaponPreset {
            name: entry.name,
            group,
            speed: speed_value,
            speed_label,
            jab_speed: jab_speed_value,
            jab_speed_label: jab_label,
            jab_special_expr: entry.jab_special.clone(),
            damage_expr,
            reach_label,
            reach_ft,
            armor_pen: entry.armor_penetration.unwrap_or(0),
            defense_bonus_always: entry.defense_bonus_always.unwrap_or(false),
            size,
            handedness,
            ammunition: entry.ammunition.clone(),
        });
    }
    if catalog.is_empty() {
        Err("No weapons loaded from JSON".to_string())
    } else {
        Ok(catalog)
    }
}

fn load_armor_catalog(path: &str) -> Result<Vec<ArmorEntry>, String> {
    let data = fs::read_to_string(path).unwrap_or_else(|_| EMBEDDED_ARMOR_JSON.to_string());
    let parsed: ArmorFile = serde_json::from_str(&data).map_err(|err| err.to_string())?;
    let mut catalog = Vec::new();
    catalog.push(ArmorEntry {
        label: "None".to_string(),
        armor: None,
    });
    for entry in parsed.armor {
        if entry.name == "None" {
            continue;
        }
        let region = match armor_region_from_str(&entry.region) {
            Some(region) => region,
            None => continue,
        };
        let armor_type = match armor_type_from_str(&entry.armor_type) {
            Some(kind) => kind,
            None => continue,
        };
        let label = format!("{} ({})", entry.name, entry.region);
        let armor = Armor {
            name: leak_str(entry.name),
            region,
            damage_reduction: entry.damage_reduction,
            defense_adj: entry.defense_adjustment,
            initiative_mod: entry.initiative_modifier,
            speed_mod: entry.speed_modifier,
            armor_type,
            weight_lbs: entry.weight_lbs.unwrap_or(0.0),
        };
        catalog.push(ArmorEntry {
            label,
            armor: Some(armor),
        });
    }
    Ok(catalog)
}

fn load_materials(path: &str) -> Result<MaterialsFile, String> {
    let data = fs::read_to_string(path).unwrap_or_else(|_| EMBEDDED_MATERIALS_JSON.to_string());
    serde_json::from_str(&data).map_err(|err| err.to_string())
}

fn split_speed_label(speed: &str, jab_speed: Option<&str>) -> (String, Option<String>) {
    if let Some(jab) = jab_speed {
        return (speed.trim().to_string(), Some(jab.trim().to_string()));
    }
    let open = speed.find('(');
    let close = speed.find(')');
    if let (Some(open), Some(close)) = (open, close) {
        if open < close {
            let base = speed[..open].trim();
            let jab = speed[open + 1..close].trim();
            if !jab.is_empty() {
                return (base.to_string(), Some(jab.to_string()));
            }
        }
    }
    (speed.trim().to_string(), None)
}

fn weapon_group_from_str(group: &str) -> Option<WeaponGroup> {
    match group {
        "Unarmed" => Some(WeaponGroup::Unarmed),
        "Axes" => Some(WeaponGroup::Axes),
        "Basic" => Some(WeaponGroup::Basic),
        "Blunt" => Some(WeaponGroup::Blunt),
        "Bows" => Some(WeaponGroup::Bows),
        "Crossbows" => Some(WeaponGroup::Crossbows),
        "Double" => Some(WeaponGroup::Double),
        "Ensnaring" => Some(WeaponGroup::Ensnaring),
        "Lashes" => Some(WeaponGroup::Lashes),
        "Large Swords" => Some(WeaponGroup::LargeSwords),
        "Small Swords" => Some(WeaponGroup::SmallSwords),
        "Polearms" => Some(WeaponGroup::Polearms),
        "Spears" => Some(WeaponGroup::Spears),
        "Shields" => Some(WeaponGroup::Shields),
        _ => None,
    }
}

fn armor_region_from_str(region: &str) -> Option<crate::character::ArmorRegion> {
    match region {
        "Northern" => Some(crate::character::ArmorRegion::Northern),
        "Southern" => Some(crate::character::ArmorRegion::Southern),
        _ => None,
    }
}

fn armor_type_from_str(kind: &str) -> Option<crate::character::ArmorType> {
    match kind {
        "None" => Some(crate::character::ArmorType::None),
        "Light" => Some(crate::character::ArmorType::Light),
        "Medium" => Some(crate::character::ArmorType::Medium),
        "Heavy" => Some(crate::character::ArmorType::Heavy),
        _ => None,
    }
}

fn weapon_size_from_str(size: &str) -> Option<WeaponSize> {
    match size {
        "S" => Some(WeaponSize::Small),
        "M" => Some(WeaponSize::Medium),
        "L" => Some(WeaponSize::Large),
        _ => None,
    }
}

fn weapon_handedness_from_str(handedness: &str) -> Option<WeaponHandedness> {
    match handedness {
        "1h" => Some(WeaponHandedness::OneHanded),
        "2h" => Some(WeaponHandedness::TwoHanded),
        _ => None,
    }
}

fn parse_leading_number(value: &str) -> f32 {
    let mut started = false;
    let mut buf = String::new();
    for ch in value.chars() {
        if ch.is_ascii_digit() || (ch == '.' && started) {
            started = true;
            buf.push(ch);
        } else if started {
            break;
        }
    }
    buf.parse::<f32>().unwrap_or(0.0)
}

fn parse_reach_ft(value: &str) -> f32 {
    if value.contains('/') {
        return 0.0;
    }
    parse_leading_number(value)
}

fn leak_str(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
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
        reach_label: reach_label.to_string(),
        reach_ft,
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


pub fn is_ranged_weapon(name: &str) -> bool {
    sim::max_range_for_weapon(name).is_some()
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

pub fn strength_damage_for_weapon(weapon_name: &str, base: i32) -> i32 {
    if is_ranged_weapon(weapon_name) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character;

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
}
