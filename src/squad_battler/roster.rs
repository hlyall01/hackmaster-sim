//! Squad roster and hero generation.

use crate::character::{AbilityScore, AbilitySet, AbilitySetFull, Progression, ProgressionTier};
use crate::core::rng::{SimRng, derive_seed};
use crate::core::types::PlayerProfile;
use crate::game_logic::{
    ArmorCatalog, ArmorId, PlayerConfig, ShieldCatalog, ShieldId, WeaponCatalog, WeaponHandedness,
    WeaponId,
};
use rand::Rng;
use serde::{Deserialize, Serialize};

pub const MAX_ACTIVE_SQUAD: usize = 6;
pub const MAX_BENCH: usize = 4;
pub const STARTING_HEROES: usize = 3;

const FIRST_NAMES: &[&str] = &[
    "Aldren", "Bessa", "Corvin", "Damaris", "Edrik", "Fenna", "Garran", "Helvi", "Iven", "Jora",
    "Kessel", "Lysa",
];
const EPITHETS: &[&str] = &[
    "Ash-Vowed",
    "Brasshand",
    "Cairn-Born",
    "Duskward",
    "Elmshield",
    "Flint-Eyed",
    "Grimwater",
    "Hearthbound",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SquadMemberStatus {
    Ready,
    Downed,
    Dead,
}

#[derive(Clone)]
pub struct SquadMember {
    pub id: String,
    pub config: PlayerConfig,
    pub profile: PlayerProfile,
    pub weapon_name: String,
    pub current_hp: i32,
    pub max_hp: i32,
    pub status: SquadMemberStatus,
}

impl SquadMember {
    pub fn view(&self) -> SquadMemberView {
        SquadMemberView {
            id: self.id.clone(),
            name: self.profile.name.clone(),
            level: self.profile.level,
            xp: self.profile.xp,
            next_level_xp: 45 + (self.profile.level as u32).saturating_sub(1) * 55,
            hp: self.current_hp,
            max_hp: self.max_hp,
            weapon: self.weapon_name.clone(),
            status: self.status,
            stats: vec![
                format!(
                    "STR {}/{}",
                    self.profile.base_stats.strength.base,
                    self.profile.base_stats.strength.percentile
                ),
                format!(
                    "DEX {}/{}",
                    self.profile.base_stats.dexterity.base,
                    self.profile.base_stats.dexterity.percentile
                ),
                format!("CON {}", self.profile.base_stats.constitution),
            ],
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SquadMemberView {
    pub id: String,
    pub name: String,
    pub level: u8,
    pub xp: u32,
    pub next_level_xp: u32,
    pub hp: i32,
    pub max_hp: i32,
    pub weapon: String,
    pub status: SquadMemberStatus,
    pub stats: Vec<String>,
}

pub fn roll_starting_squad(
    seed: u64,
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
) -> Vec<SquadMember> {
    (0..STARTING_HEROES)
        .map(|idx| {
            roll_member(
                format!("hero-{}", idx + 1),
                derive_seed(seed, "starting-hero", idx as u64),
                weapon_catalog,
                armor_catalog,
                shield_catalog,
            )
        })
        .collect()
}

pub fn roll_member(
    id: String,
    seed: u64,
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
) -> SquadMember {
    let mut rng = SimRng::from_seed(seed);
    let name = format!(
        "{} {}",
        FIRST_NAMES[rng.gen_range(0..FIRST_NAMES.len())],
        EPITHETS[rng.gen_range(0..EPITHETS.len())]
    );
    let weapon_id = random_weapon_id(&mut rng, weapon_catalog);
    let weapon_name = weapon_catalog
        .get(weapon_id)
        .map(|weapon| weapon.name.clone())
        .unwrap_or_else(|| "Weapon".to_string());
    let mut config = PlayerConfig::new(&name, weapon_id);
    config.level = 1;
    config.progression = random_progression(&mut rng);
    config.base_hp = rng.gen_range(8..=13);
    config.move_speed = 20.0;
    config.strength_base = roll_ability(&mut rng);
    config.strength_pct = rng.gen_range(1..=100);
    config.dex_base = roll_ability(&mut rng);
    config.dex_pct = rng.gen_range(1..=100);
    config.constitution = roll_ability(&mut rng);
    config.intelligence = roll_ability(&mut rng);
    config.wisdom = roll_ability(&mut rng);
    config.looks = roll_ability(&mut rng);
    config.charisma = roll_ability(&mut rng);
    config.armor_id = random_armor_id(&mut rng, armor_catalog);
    config.shield_id = random_shield_id(&mut rng, shield_catalog);
    config.two_hand_grip = weapon_catalog
        .get(config.weapon_id)
        .map(|weapon| weapon.handedness == WeaponHandedness::TwoHanded)
        .unwrap_or(false);
    if config.two_hand_grip {
        config.shield_id =
            find_shield_id_by_label(shield_catalog, "None").unwrap_or(config.shield_id);
    }

    let profile = player_profile_from_config(&config);
    let max_hp = config.base_hp as i32 + constitution_hp_bonus(config.constitution);
    SquadMember {
        id,
        config,
        profile,
        weapon_name,
        current_hp: max_hp,
        max_hp,
        status: SquadMemberStatus::Ready,
    }
}

fn roll_ability(rng: &mut SimRng) -> u8 {
    (rng.gen_range(1..=6) + rng.gen_range(1..=6) + rng.gen_range(1..=6)).clamp(3, 18)
}

fn random_progression(rng: &mut SimRng) -> Progression {
    let tier = |rng: &mut SimRng| match rng.gen_range(0..3) {
        0 => ProgressionTier::I,
        1 => ProgressionTier::II,
        _ => ProgressionTier::III,
    };
    Progression::new(tier(rng), tier(rng), tier(rng), tier(rng))
}

fn random_weapon_id(rng: &mut SimRng, catalog: &WeaponCatalog) -> WeaponId {
    let preferred = [
        "Battle Axe",
        "Glaive",
        "Longsword",
        "Short sword",
        "Spear",
        "Mace",
        "Short bow",
    ];
    let name = preferred[rng.gen_range(0..preferred.len())];
    find_weapon_id_by_name(catalog, name)
        .unwrap_or_else(|| catalog.first_id().unwrap_or(WeaponId::new(0)))
}

fn random_armor_id(rng: &mut SimRng, catalog: &ArmorCatalog) -> ArmorId {
    let preferred = ["Quilted", "Leather", "Chainshirt", "Ringmail", "None"];
    let name = preferred[rng.gen_range(0..preferred.len())];
    find_armor_id_by_label(catalog, name)
        .unwrap_or_else(|| catalog.first_id().unwrap_or(ArmorId::new(0)))
}

fn random_shield_id(rng: &mut SimRng, catalog: &ShieldCatalog) -> ShieldId {
    let preferred = ["None", "Small wooden shield", "Small metallic shield"];
    let name = preferred[rng.gen_range(0..preferred.len())];
    find_shield_id_by_label(catalog, name)
        .unwrap_or_else(|| catalog.first_id().unwrap_or(ShieldId::new(0)))
}

fn find_weapon_id_by_name(catalog: &WeaponCatalog, name: &str) -> Option<WeaponId> {
    catalog
        .entries()
        .iter()
        .position(|weapon| weapon.name.eq_ignore_ascii_case(name))
        .map(WeaponId::new)
}

fn find_armor_id_by_label(catalog: &ArmorCatalog, label: &str) -> Option<ArmorId> {
    catalog
        .entries()
        .iter()
        .position(|entry| entry.label.eq_ignore_ascii_case(label))
        .map(ArmorId::new)
}

fn find_shield_id_by_label(catalog: &ShieldCatalog, label: &str) -> Option<ShieldId> {
    catalog
        .entries()
        .iter()
        .position(|entry| entry.label.eq_ignore_ascii_case(label))
        .map(ShieldId::new)
}

fn constitution_hp_bonus(con: u8) -> i32 {
    ((con as i32 - 10) / 2).max(0)
}

fn player_profile_from_config(config: &PlayerConfig) -> PlayerProfile {
    let base_stats = AbilitySet {
        strength: AbilityScore::new(config.strength_base, config.strength_pct),
        intelligence: config.intelligence,
        wisdom: config.wisdom,
        dexterity: AbilityScore::new(config.dex_base, config.dex_pct),
        constitution: config.constitution,
        looks: config.looks,
        charisma: config.charisma,
    };
    let mut profile = PlayerProfile::new(config.name.clone(), base_stats);
    profile.level = config.level;
    profile.progression = config.progression;
    profile.ability_scores_full = AbilitySetFull::from(base_stats);
    profile.proficiencies = config.proficiencies.clone();
    profile.talents = config.talents.clone();
    profile.race_id = config.race_id.clone();
    profile
}
