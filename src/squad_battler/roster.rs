//! Squad roster and hero generation.

use crate::character::{AbilityScore, AbilitySet, AbilitySetFull, Progression, ProgressionTier};
use crate::core::gameplay::{EncounterTier, XpCurve, apply_xp};
use crate::core::rng::{SimRng, derive_seed};
use crate::core::types::{PlayerProfile, PointPools};
use crate::game_logic::{
    ArmorCatalog, ArmorId, PlayerConfig, ShieldCatalog, ShieldId, WeaponCatalog, WeaponHandedness,
    WeaponId,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt;

use super::rewards::{
    RecruitRarity, recruit_level_for, recruit_offer_scaling, recruit_rarity_for_roll,
};

pub const MAX_ACTIVE_SQUAD: usize = 6;
pub const MAX_BENCH: usize = 4;
pub const STARTING_HEROES: usize = 3;
pub const DEFAULT_SQUAD_XP_BASE: u32 = 45;
pub const DEFAULT_SQUAD_XP_PER_LEVEL: u32 = 55;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SquadRole {
    Frontline,
    Skirmisher,
    Archer,
    Support,
}

impl SquadRole {
    pub fn label(self) -> &'static str {
        match self {
            SquadRole::Frontline => "Frontline",
            SquadRole::Skirmisher => "Skirmisher",
            SquadRole::Archer => "Archer",
            SquadRole::Support => "Support",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SquadStatGrowth {
    Strength,
    Dexterity,
    Intelligence,
    Wisdom,
    Constitution,
    Charisma,
}

impl SquadStatGrowth {
    pub fn label(self) -> &'static str {
        match self {
            SquadStatGrowth::Strength => "Strength",
            SquadStatGrowth::Dexterity => "Dexterity",
            SquadStatGrowth::Intelligence => "Intelligence",
            SquadStatGrowth::Wisdom => "Wisdom",
            SquadStatGrowth::Constitution => "Constitution",
            SquadStatGrowth::Charisma => "Charisma",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SquadWoundSeverity {
    Light,
    Serious,
    Critical,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct SquadWound {
    pub id: String,
    pub severity: SquadWoundSeverity,
    pub damage: u32,
    pub recovery_days_remaining: u8,
}

#[derive(Clone)]
pub struct SquadMember {
    pub id: String,
    pub config: PlayerConfig,
    pub profile: PlayerProfile,
    pub role: SquadRole,
    pub rarity: RecruitRarity,
    pub weapon_name: String,
    pub current_hp: i32,
    pub max_hp: i32,
    pub status: SquadMemberStatus,
    pub wounds: Vec<SquadWound>,
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

    pub fn replace_active_and_bench_replaced(
        &mut self,
        replace_member_id: &str,
        member: SquadMember,
    ) -> Result<(), RosterError> {
        if self.bench.len() >= MAX_BENCH {
            return Err(RosterError::BenchFull);
        }
        let replaced = self.replace_active(replace_member_id, member)?;
        self.bench.push(replaced);
        Ok(())
    }

    pub fn swap_bench_to_active(
        &mut self,
        active_member_id: &str,
        bench_member_id: &str,
    ) -> Result<(), RosterError> {
        let Some(active_index) = self
            .active
            .iter()
            .position(|member| member.id == active_member_id)
        else {
            return Err(RosterError::MemberNotFound);
        };
        let Some(bench_index) = self
            .bench
            .iter()
            .position(|member| member.id == bench_member_id)
        else {
            return Err(RosterError::MemberNotFound);
        };
        std::mem::swap(&mut self.active[active_index], &mut self.bench[bench_index]);
        Ok(())
    }

    pub fn promote_bench_to_active(&mut self, bench_member_id: &str) -> Result<(), RosterError> {
        if self.active.len() >= MAX_ACTIVE_SQUAD {
            return Err(RosterError::ActiveFull);
        }
        let Some(bench_index) = self
            .bench
            .iter()
            .position(|member| member.id == bench_member_id)
        else {
            return Err(RosterError::MemberNotFound);
        };
        let member = self.bench.remove(bench_index);
        self.active.push(member);
        Ok(())
    }

    pub fn dismiss_bench(&mut self, bench_member_id: &str) -> Result<SquadMember, RosterError> {
        let Some(index) = self
            .bench
            .iter()
            .position(|member| member.id == bench_member_id)
        else {
            return Err(RosterError::MemberNotFound);
        };
        Ok(self.bench.remove(index))
    }

    pub fn recover_after_fight(
        &mut self,
        hp_recovered: i32,
        wound_recovery_days: u8,
    ) -> Vec<SquadRecoveryReport> {
        self.active
            .iter_mut()
            .chain(self.bench.iter_mut())
            .map(|member| member.recover_after_fight(hp_recovered, wound_recovery_days))
            .collect()
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
            next_level_xp: default_squad_xp_curve().xp_for_next_level(self.profile.level),
            role: self.role,
            rarity: self.rarity,
            hp: self.current_hp,
            max_hp: self.max_hp,
            weapon: self.weapon_name.clone(),
            status: self.status,
            wounds: self.wounds.clone(),
            wound_total: self.total_wound_damage(),
            level_up_available: self.can_level_up(&default_squad_xp_curve()),
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

    pub fn total_wound_damage(&self) -> u32 {
        self.wounds
            .iter()
            .map(|wound| wound.damage)
            .fold(0_u32, u32::saturating_add)
    }

    pub fn can_level_up(&self, curve: &XpCurve) -> bool {
        self.profile.level < u8::MAX
            && self.profile.xp >= curve.xp_for_next_level(self.profile.level)
    }

    pub fn apply_wound(&mut self, damage: u32, recovery_days: u8) -> SquadWound {
        let damage = damage.max(1);
        let wound = SquadWound {
            id: format!("{}-wound-{}", self.id, self.wounds.len() + 1),
            severity: wound_severity(damage, self.max_hp),
            damage,
            recovery_days_remaining: recovery_days.max(1),
        };
        self.wounds.push(wound.clone());
        wound
    }

    pub fn apply_post_fight_injury(&mut self, remaining_hp: i32) -> SquadInjuryReport {
        let hp_before = self.current_hp;
        if self.status == SquadMemberStatus::Dead {
            return SquadInjuryReport {
                member_id: self.id.clone(),
                hp_before,
                hp_after: self.current_hp,
                status: self.status,
                wound_added: None,
            };
        }

        self.current_hp = remaining_hp.clamp(0, self.max_hp);
        self.status = if self.current_hp <= 0 {
            SquadMemberStatus::Downed
        } else {
            SquadMemberStatus::Ready
        };

        let wound_added = if self.current_hp <= 0 {
            Some(self.apply_wound((self.max_hp / 3).max(1) as u32, 3))
        } else if self.current_hp * 2 <= self.max_hp {
            let missing_hp = self.max_hp.saturating_sub(self.current_hp).max(1);
            Some(self.apply_wound((missing_hp / 4).max(1) as u32, 1))
        } else {
            None
        };

        SquadInjuryReport {
            member_id: self.id.clone(),
            hp_before,
            hp_after: self.current_hp,
            status: self.status,
            wound_added,
        }
    }

    pub fn recover_after_fight(
        &mut self,
        hp_recovered: i32,
        wound_recovery_days: u8,
    ) -> SquadRecoveryReport {
        let hp_before = self.current_hp;
        let mut wounds_healed = Vec::new();
        if self.status != SquadMemberStatus::Dead {
            self.current_hp = self
                .current_hp
                .saturating_add(hp_recovered.max(0))
                .min(self.max_hp);
            if self.current_hp > 0 && self.status == SquadMemberStatus::Downed {
                self.status = SquadMemberStatus::Ready;
            }
        }

        let mut kept = Vec::new();
        for mut wound in self.wounds.drain(..) {
            if wound_recovery_days >= wound.recovery_days_remaining {
                wounds_healed.push(wound);
            } else {
                wound.recovery_days_remaining = wound
                    .recovery_days_remaining
                    .saturating_sub(wound_recovery_days);
                kept.push(wound);
            }
        }
        self.wounds = kept;

        SquadRecoveryReport {
            member_id: self.id.clone(),
            hp_recovered: self.current_hp.saturating_sub(hp_before),
            wounds_healed,
            status: self.status,
        }
    }

    pub fn award_xp(&mut self, curve: &XpCurve, gained_xp: u32) -> SquadLevelUpReport {
        let previous_level = self.profile.level;
        let result = apply_xp(&mut self.profile, curve, gained_xp);
        self.config.level = self.profile.level;
        self.apply_level_progression_from(previous_level, result.levels_gained)
    }

    pub fn apply_level_up_progression(&mut self, levels_gained: u8) -> SquadLevelUpReport {
        let previous_level = self.profile.level.saturating_sub(levels_gained);
        self.apply_level_progression_from(previous_level, levels_gained)
    }

    fn apply_level_progression_from(
        &mut self,
        previous_level: u8,
        levels_gained: u8,
    ) -> SquadLevelUpReport {
        let max_hp_before = self.max_hp;
        let points_before = self.profile.points;
        let mut stat_growth = Vec::new();

        if levels_gained == 0 {
            return SquadLevelUpReport {
                member_id: self.id.clone(),
                previous_level,
                new_level: self.profile.level,
                levels_gained,
                max_hp_gained: 0,
                stat_growth,
                points_gained: PointPools::default(),
            };
        }

        for level in previous_level.saturating_add(1)..=self.profile.level {
            let hp_gain = level_hp_gain(self.profile.base_stats.constitution, self.rarity);
            self.max_hp = self.max_hp.saturating_add(hp_gain);
            self.current_hp = self.current_hp.saturating_add(hp_gain).min(self.max_hp);
            self.profile.points.bp = self.profile.points.bp.saturating_add(5);
            self.profile.points.lp = self.profile.points.lp.saturating_add(1);
            self.profile.points.ap = self.profile.points.ap.saturating_add(1);

            if let Some(stat) = role_stat_growth(self.role, level) {
                self.increase_stat(stat, 1);
                stat_growth.push(SquadStatGrowthEntry { stat, amount: 1 });
            }
        }

        SquadLevelUpReport {
            member_id: self.id.clone(),
            previous_level,
            new_level: self.profile.level,
            levels_gained,
            max_hp_gained: self.max_hp.saturating_sub(max_hp_before),
            stat_growth,
            points_gained: PointPools {
                bp: self.profile.points.bp.saturating_sub(points_before.bp),
                lp: self.profile.points.lp.saturating_sub(points_before.lp),
                ap: self.profile.points.ap.saturating_sub(points_before.ap),
                rp: self.profile.points.rp.saturating_sub(points_before.rp),
            },
        }
    }

    fn increase_stat(&mut self, stat: SquadStatGrowth, amount: u8) {
        match stat {
            SquadStatGrowth::Strength => {
                self.config.strength_base = add_stat(self.config.strength_base, amount);
                self.profile.base_stats.strength.base = self.config.strength_base;
                self.profile.ability_scores_full.strength.base = self.config.strength_base;
            }
            SquadStatGrowth::Dexterity => {
                self.config.dex_base = add_stat(self.config.dex_base, amount);
                self.profile.base_stats.dexterity.base = self.config.dex_base;
                self.profile.ability_scores_full.dexterity.base = self.config.dex_base;
            }
            SquadStatGrowth::Intelligence => {
                self.config.intelligence = add_stat(self.config.intelligence, amount);
                self.profile.base_stats.intelligence = self.config.intelligence;
                self.profile.ability_scores_full.intelligence.base = self.config.intelligence;
            }
            SquadStatGrowth::Wisdom => {
                self.config.wisdom = add_stat(self.config.wisdom, amount);
                self.profile.base_stats.wisdom = self.config.wisdom;
                self.profile.ability_scores_full.wisdom.base = self.config.wisdom;
            }
            SquadStatGrowth::Constitution => {
                self.config.constitution = add_stat(self.config.constitution, amount);
                self.profile.base_stats.constitution = self.config.constitution;
                self.profile.ability_scores_full.constitution.base = self.config.constitution;
            }
            SquadStatGrowth::Charisma => {
                self.config.charisma = add_stat(self.config.charisma, amount);
                self.profile.base_stats.charisma = self.config.charisma;
                self.profile.ability_scores_full.charisma.base = self.config.charisma;
            }
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
    pub role: SquadRole,
    pub rarity: RecruitRarity,
    pub hp: i32,
    pub max_hp: i32,
    pub weapon: String,
    pub status: SquadMemberStatus,
    pub wounds: Vec<SquadWound>,
    pub wound_total: u32,
    pub level_up_available: bool,
    pub stats: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SquadView {
    pub active: Vec<SquadMemberView>,
    pub bench: Vec<SquadMemberView>,
    pub max_active: usize,
    pub max_bench: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SquadInjuryReport {
    pub member_id: String,
    pub hp_before: i32,
    pub hp_after: i32,
    pub status: SquadMemberStatus,
    pub wound_added: Option<SquadWound>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SquadRecoveryReport {
    pub member_id: String,
    pub hp_recovered: i32,
    pub wounds_healed: Vec<SquadWound>,
    pub status: SquadMemberStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SquadStatGrowthEntry {
    pub stat: SquadStatGrowth,
    pub amount: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SquadLevelUpReport {
    pub member_id: String,
    pub previous_level: u8,
    pub new_level: u8,
    pub levels_gained: u8,
    pub max_hp_gained: i32,
    pub stat_growth: Vec<SquadStatGrowthEntry>,
    pub points_gained: PointPools,
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
    roll_member_with_traits(
        id,
        seed,
        RecruitRarity::Common,
        1,
        weapon_catalog,
        armor_catalog,
        shield_catalog,
    )
}

pub fn roll_recruit_member(
    id: String,
    seed: u64,
    depth: u32,
    tier: EncounterTier,
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
) -> SquadMember {
    let mut rng = SimRng::from_seed(derive_seed(seed, "recruit-rarity", depth as u64));
    let scaling = recruit_offer_scaling(depth, tier);
    let rarity = recruit_rarity_for_roll(
        rng.gen_range(0..scaling.rarity_weights.total().max(1)),
        scaling.rarity_weights,
    );
    let level = recruit_level_for(depth, tier, rarity);
    roll_member_with_traits(
        id,
        seed,
        rarity,
        level,
        weapon_catalog,
        armor_catalog,
        shield_catalog,
    )
}

pub fn roll_member_with_traits(
    id: String,
    seed: u64,
    rarity: RecruitRarity,
    level: u8,
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
    let role = role_for_weapon(&weapon_name);
    let mut config = PlayerConfig::new(&name, weapon_id);
    config.level = level.max(1);
    config.progression = random_progression(&mut rng);
    config.base_hp = rng
        .gen_range(8_u32..=13)
        .saturating_add(u32::from(rarity.hp_bonus()));
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
    apply_rarity_stat_bonus(&mut config, role, rarity);
    if config.two_hand_grip {
        config.shield_id =
            find_shield_id_by_label(shield_catalog, "None").unwrap_or(config.shield_id);
    }

    let mut profile = player_profile_from_config(&config);
    profile.xp = default_squad_xp_curve().xp_for_level(profile.level);
    let max_hp = config.base_hp as i32 + constitution_hp_bonus(config.constitution);
    SquadMember {
        id,
        config,
        profile,
        role,
        rarity,
        weapon_name,
        current_hp: max_hp,
        max_hp,
        status: SquadMemberStatus::Ready,
        wounds: Vec::new(),
    }
}

pub fn default_squad_xp_curve() -> XpCurve {
    XpCurve {
        base: DEFAULT_SQUAD_XP_BASE,
        per_level: DEFAULT_SQUAD_XP_PER_LEVEL,
    }
}

fn role_for_weapon(weapon_name: &str) -> SquadRole {
    let lower = weapon_name.to_ascii_lowercase();
    if lower.contains("bow") {
        SquadRole::Archer
    } else if lower.contains("glaive") || lower.contains("spear") {
        SquadRole::Skirmisher
    } else if lower.contains("mace") {
        SquadRole::Support
    } else {
        SquadRole::Frontline
    }
}

fn apply_rarity_stat_bonus(config: &mut PlayerConfig, role: SquadRole, rarity: RecruitRarity) {
    let bonus = rarity.stat_bonus();
    if bonus == 0 {
        return;
    }

    match role {
        SquadRole::Frontline => {
            config.strength_base = add_stat(config.strength_base, bonus);
        }
        SquadRole::Skirmisher | SquadRole::Archer => {
            config.dex_base = add_stat(config.dex_base, bonus);
        }
        SquadRole::Support => {
            config.intelligence = add_stat(config.intelligence, bonus);
        }
    }

    if rarity == RecruitRarity::Elite {
        config.constitution = add_stat(config.constitution, 1);
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

fn level_hp_gain(con: u8, rarity: RecruitRarity) -> i32 {
    4 + constitution_hp_bonus(con).min(2) + i32::from(rarity.hp_bonus() > 0)
}

fn wound_severity(damage: u32, max_hp: i32) -> SquadWoundSeverity {
    let max_hp = u32::try_from(max_hp.max(1)).unwrap_or(1);
    if damage.saturating_mul(2) >= max_hp {
        SquadWoundSeverity::Critical
    } else if damage.saturating_mul(4) >= max_hp {
        SquadWoundSeverity::Serious
    } else {
        SquadWoundSeverity::Light
    }
}

fn role_stat_growth(role: SquadRole, level: u8) -> Option<SquadStatGrowth> {
    if level % 2 != 0 {
        return None;
    }

    Some(match role {
        SquadRole::Frontline => SquadStatGrowth::Strength,
        SquadRole::Skirmisher | SquadRole::Archer => SquadStatGrowth::Dexterity,
        SquadRole::Support => SquadStatGrowth::Intelligence,
    })
}

fn add_stat(value: u8, amount: u8) -> u8 {
    value.saturating_add(amount).min(25)
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
    fn oversized_initial_active_squad_is_rejected() {
        let active = (0..=MAX_ACTIVE_SQUAD)
            .map(|idx| member(&format!("hero-{idx}")))
            .collect();

        assert_eq!(
            SquadRoster::new(active).err(),
            Some(RosterError::ActiveFull)
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

    #[test]
    fn replace_active_targets_specific_member_id() {
        let mut roster =
            SquadRoster::new(vec![member("front"), member("rear")]).expect("valid roster");

        let replaced = roster
            .replace_active("front", member("new-front"))
            .expect("replacement should succeed");

        assert_eq!(replaced.id, "front");
        assert_eq!(roster.active()[0].id, "new-front");
        assert_eq!(roster.active()[1].id, "rear");
        assert_eq!(
            roster.replace_active("missing", member("unused")).err(),
            Some(RosterError::MemberNotFound)
        );
    }

    #[test]
    fn replace_active_can_bench_replaced_member_when_room_exists() {
        let mut roster = SquadRoster::new(vec![member("front")]).expect("valid roster");

        roster
            .replace_active_and_bench_replaced("front", member("new-front"))
            .expect("bench has room");

        assert_eq!(roster.active()[0].id, "new-front");
        assert_eq!(roster.bench()[0].id, "front");
    }

    #[test]
    fn bench_member_can_swap_into_active_slot() {
        let mut roster =
            SquadRoster::new(vec![member("active-a"), member("active-b")]).expect("valid roster");
        roster.add_bench(member("bench-a")).expect("bench room");

        roster
            .swap_bench_to_active("active-b", "bench-a")
            .expect("swap should succeed");

        assert_eq!(roster.active()[0].id, "active-a");
        assert_eq!(roster.active()[1].id, "bench-a");
        assert_eq!(roster.bench()[0].id, "active-b");
    }

    #[test]
    fn bench_member_can_promote_when_active_has_room() {
        let mut roster = SquadRoster::new(vec![member("active-a")]).expect("valid roster");
        roster.add_bench(member("bench-a")).expect("bench room");

        roster
            .promote_bench_to_active("bench-a")
            .expect("promotion should succeed");

        assert_eq!(roster.active()[1].id, "bench-a");
        assert!(roster.bench().is_empty());
    }

    #[test]
    fn bench_member_can_be_dismissed() {
        let mut roster = SquadRoster::new(Vec::new()).expect("valid roster");
        roster.add_bench(member("bench-a")).expect("bench room");
        roster.add_bench(member("bench-b")).expect("bench room");

        let dismissed = roster
            .dismiss_bench("bench-a")
            .expect("dismiss should succeed");

        assert_eq!(dismissed.id, "bench-a");
        assert_eq!(roster.bench().len(), 1);
        assert_eq!(roster.bench()[0].id, "bench-b");
        assert_eq!(
            roster.dismiss_bench("missing").err(),
            Some(RosterError::MemberNotFound)
        );
    }

    #[test]
    fn wounds_track_individual_injuries_and_recover_after_fight() {
        let mut hero = member("hurt");

        let injury = hero.apply_post_fight_injury(0);

        assert_eq!(injury.status, SquadMemberStatus::Downed);
        assert!(injury.wound_added.is_some());
        assert_eq!(hero.status, SquadMemberStatus::Downed);
        assert_eq!(hero.wounds.len(), 1);
        assert_eq!(hero.total_wound_damage(), hero.wounds[0].damage);

        let recovery = hero.recover_after_fight(3, 3);

        assert_eq!(recovery.status, SquadMemberStatus::Ready);
        assert_eq!(recovery.wounds_healed.len(), 1);
        assert!(hero.current_hp > 0);
        assert!(hero.wounds.is_empty());
    }

    #[test]
    fn roster_recovery_applies_to_active_and_bench_members() {
        let mut active = member("active");
        active.apply_wound(2, 2);
        let mut bench = member("bench");
        bench.apply_wound(3, 2);
        let mut roster = SquadRoster::new(vec![active]).expect("valid roster");
        roster.add_bench(bench).expect("bench room");

        let reports = roster.recover_after_fight(0, 2);

        assert_eq!(reports.len(), 2);
        assert!(roster.active()[0].wounds.is_empty());
        assert!(roster.bench()[0].wounds.is_empty());
    }

    #[test]
    fn award_xp_applies_level_hp_stats_and_points() {
        let mut hero = member("leveler");
        let max_hp_before = hero.max_hp;
        let strength_before = hero.profile.base_stats.strength.base;

        let report = hero.award_xp(&default_squad_xp_curve(), DEFAULT_SQUAD_XP_BASE);

        assert_eq!(hero.profile.level, 2);
        assert_eq!(hero.config.level, 2);
        assert_eq!(report.levels_gained, 1);
        assert!(report.max_hp_gained > 0);
        assert!(hero.max_hp > max_hp_before);
        assert_eq!(report.points_gained.bp, 5);
        assert_eq!(report.points_gained.lp, 1);
        assert_eq!(report.points_gained.ap, 1);
        assert_eq!(
            hero.profile.base_stats.strength.base,
            strength_before.saturating_add(1).min(25)
        );
        assert_eq!(
            report.stat_growth,
            vec![SquadStatGrowthEntry {
                stat: SquadStatGrowth::Strength,
                amount: 1,
            }]
        );
    }

    #[test]
    fn recruit_traits_set_rarity_level_and_view_fields() {
        let (weapons, armor, shields) = empty_catalogs();
        let mut recruit = roll_member_with_traits(
            "elite".to_string(),
            10,
            RecruitRarity::Elite,
            3,
            &weapons,
            &armor,
            &shields,
        );
        recruit.apply_wound(4, 2);

        let view = recruit.view();

        assert_eq!(view.id, "elite");
        assert_eq!(view.level, 3);
        assert_eq!(view.rarity, RecruitRarity::Elite);
        assert_eq!(view.role, SquadRole::Frontline);
        assert_eq!(view.wounds.len(), 1);
        assert_eq!(view.wound_total, 4);
        assert!(!view.level_up_available);
    }

    #[test]
    fn route_scaled_recruit_uses_depth_and_tier_level_bounds() {
        let (weapons, armor, shields) = empty_catalogs();
        let recruit = roll_recruit_member(
            "scaled".to_string(),
            99,
            6,
            EncounterTier::Elite,
            &weapons,
            &armor,
            &shields,
        );
        let scaling = recruit_offer_scaling(6, EncounterTier::Elite);

        assert!(recruit.profile.level >= scaling.min_level);
        assert!(recruit.profile.level <= scaling.max_level);
        assert_eq!(
            recruit.profile.xp,
            default_squad_xp_curve().xp_for_level(recruit.profile.level)
        );
    }
}
