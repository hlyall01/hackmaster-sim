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
use std::fmt;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RosterError {
    ActiveFull,
    BenchFull,
    MemberNotFound,
}

impl fmt::Display for RosterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RosterError::ActiveFull => write!(f, "active squad is full"),
            RosterError::BenchFull => write!(f, "bench is full"),
            RosterError::MemberNotFound => write!(f, "squad member not found"),
        }
    }
}

impl std::error::Error for RosterError {}

#[derive(Clone)]
pub struct SquadRoster {
    active: Vec<SquadMember>,
    bench: Vec<SquadMember>,
}

impl SquadRoster {
    pub fn new(active: Vec<SquadMember>) -> Result<Self, RosterError> {
        if active.len() > MAX_ACTIVE_SQUAD {
            return Err(RosterError::ActiveFull);
        }
        Ok(Self {
            active,
            bench: Vec::new(),
        })
    }

    pub fn active(&self) -> &[SquadMember] {
        &self.active
    }

    pub fn active_mut(&mut self) -> &mut [SquadMember] {
        &mut self.active
    }

    pub fn bench(&self) -> &[SquadMember] {
        &self.bench
    }

    pub fn add_active(&mut self, member: SquadMember) -> Result<(), RosterError> {
        if self.active.len() >= MAX_ACTIVE_SQUAD {
            return Err(RosterError::ActiveFull);
        }
        self.active.push(member);
        Ok(())
    }

    pub fn add_bench(&mut self, member: SquadMember) -> Result<(), RosterError> {
        if self.bench.len() >= MAX_BENCH {
            return Err(RosterError::BenchFull);
        }
        self.bench.push(member);
        Ok(())
    }

    pub fn replace_active(
        &mut self,
        replace_member_id: &str,
        member: SquadMember,
    ) -> Result<SquadMember, RosterError> {
        let Some(index) = self
            .active
            .iter()
            .position(|current| current.id == replace_member_id)
        else {
            return Err(RosterError::MemberNotFound);
        };
        Ok(std::mem::replace(&mut self.active[index], member))
    }

    pub fn remove_dead_active(&mut self) -> Vec<SquadMember> {
        let mut removed = Vec::new();
        let mut kept = Vec::new();
        for member in self.active.drain(..) {
            if member.status == SquadMemberStatus::Dead {
                removed.push(member);
            } else {
                kept.push(member);
            }
        }
        self.active = kept;
        removed
    }

    pub fn view(&self) -> SquadView {
        SquadView {
            active: self.active.iter().map(SquadMember::view).collect(),
            bench: self.bench.iter().map(SquadMember::view).collect(),
            max_active: MAX_ACTIVE_SQUAD,
            max_bench: MAX_BENCH,
        }
    }
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

#[derive(Clone, Debug, Serialize)]
pub struct SquadView {
    pub active: Vec<SquadMemberView>,
    pub bench: Vec<SquadMemberView>,
    pub max_active: usize,
    pub max_bench: usize,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_logic::{ArmorCatalog, ShieldCatalog, WeaponCatalog};

    fn empty_catalogs() -> (WeaponCatalog, ArmorCatalog, ShieldCatalog) {
        (
            WeaponCatalog::new(Vec::new()),
            ArmorCatalog::new(Vec::new()),
            ShieldCatalog::new(Vec::new()),
        )
    }

    fn member(id: &str) -> SquadMember {
        let (weapons, armor, shields) = empty_catalogs();
        roll_member(id.to_string(), 10, &weapons, &armor, &shields)
    }

    #[test]
    fn active_squad_limit_is_enforced() {
        let active = (0..MAX_ACTIVE_SQUAD)
            .map(|idx| member(&format!("hero-{idx}")))
            .collect();
        let mut roster = SquadRoster::new(active).expect("valid roster");
        assert_eq!(
            roster.add_active(member("overflow")),
            Err(RosterError::ActiveFull)
        );
    }

    #[test]
    fn bench_limit_is_enforced() {
        let mut roster = SquadRoster::new(Vec::new()).expect("valid roster");
        for idx in 0..MAX_BENCH {
            roster
                .add_bench(member(&format!("bench-{idx}")))
                .expect("bench room");
        }
        assert_eq!(
            roster.add_bench(member("overflow")),
            Err(RosterError::BenchFull)
        );
    }
}
