#![allow(dead_code)]

use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProgressionTier {
    I,
    II,
    III,
    IV,
    V,
    VI,
}

impl ProgressionTier {
    pub fn attack_index(self) -> usize {
        match self {
            ProgressionTier::I => 0,
            ProgressionTier::II => 1,
            ProgressionTier::III => 2,
            ProgressionTier::IV => 3,
            ProgressionTier::V => 4,
            ProgressionTier::VI => 5,
        }
    }

    pub fn speed_index(self) -> usize {
        self.attack_index()
    }

    pub fn initiative_index(self) -> usize {
        self.attack_index().min(4)
    }

    pub fn health_index(self) -> usize {
        self.attack_index().min(4)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Progression {
    pub attack: ProgressionTier,
    pub speed: ProgressionTier,
    pub initiative: ProgressionTier,
    pub health: ProgressionTier,
}

impl Progression {
    pub fn new(
        attack: ProgressionTier,
        speed: ProgressionTier,
        initiative: ProgressionTier,
        health: ProgressionTier,
    ) -> Self {
        Self {
            attack,
            speed,
            initiative,
            health,
        }
    }
}

impl Default for Progression {
    fn default() -> Self {
        Self::new(
            ProgressionTier::I,
            ProgressionTier::I,
            ProgressionTier::I,
            ProgressionTier::I,
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub enum InitiativeDieQuality {
    Standard,
    OneBetter,
    TwoBetter,
    ThreeBetter,
    FourBetter,
}

impl Default for InitiativeDieQuality {
    fn default() -> Self {
        InitiativeDieQuality::Standard
    }
}

#[derive(Clone, Copy, Debug)]
pub struct AbilityScore {
    pub base: u8,
    pub percentile: u8,
}

pub const BASE_DV: i32 = -4;

impl AbilityScore {
    pub fn new(base: u8, percentile: u8) -> Self {
        Self { base, percentile }
    }
}

impl Default for AbilityScore {
    fn default() -> Self {
        Self {
            base: 10,
            percentile: 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AbilitySet {
    pub strength: AbilityScore,
    pub intelligence: u8,
    pub wisdom: u8,
    pub dexterity: AbilityScore,
    pub constitution: u8,
    pub looks: u8,
    pub charisma: u8,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StrengthMods {
    pub damage: i32,
    pub feat: i32,
    pub lift: u32,
    pub carry_none: u32,
    pub carry_light: u32,
    pub carry_medium: u32,
    pub carry_heavy: u32,
    pub drag: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DexMods {
    pub initiative: i32,
    pub attack: i32,
    pub defense: i32,
    pub dodge_save: i32,
    pub feat_of_agility: i32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct IntMods {
    pub attack: i32,
    pub lp_bonus: i32,
    pub weapon_exp_threshold_pct: i32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WisMods {
    pub initiative: i32,
    pub lp_bonus: i32,
    pub defense: i32,
    pub mental_save: i32,
    pub base_ff: i32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ConMods {
    pub physical_save: i32,
    pub base_ff: i32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LooksMods {
    pub charisma: i32,
    pub honor: i32,
    pub fame: i32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ChaMods {
    pub lp_bonus: i32,
    pub honor: i32,
    pub turning: i32,
    pub morale: i32,
    pub max_proteges: i32,
}

#[derive(Clone, Debug, Default)]
pub struct AbilityDerived {
    pub strength: StrengthMods,
    pub dexterity: DexMods,
    pub intelligence: IntMods,
    pub wisdom: WisMods,
    pub constitution: ConMods,
    pub looks: LooksMods,
    pub charisma: ChaMods,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArmorRegion {
    Northern,
    Southern,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArmorType {
    None,
    Light,
    Medium,
    Heavy,
}

#[derive(Clone, Debug)]
pub struct Armor {
    pub name: &'static str,
    pub region: ArmorRegion,
    pub damage_reduction: i32,
    pub defense_adj: i32,
    pub initiative_mod: i32,
    pub speed_mod: i32,
    pub armor_type: ArmorType,
    pub weight_lbs: f32,
}

#[derive(Clone, Debug)]
pub struct Shield {
    pub name: &'static str,
    pub defense_bonus: i32,
    pub dr: i32,
    pub cover_value: i32,
    pub breakage_thresholds: [i32; 4],
    pub weight_lbs: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WeaponGroup {
    Unarmed,
    Axes,
    Basic,
    Blunt,
    Bows,
    Crossbows,
    Double,
    Ensnaring,
    Lashes,
    LargeSwords,
    SmallSwords,
    Polearms,
    Spears,
    Shields,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MasteryAspect {
    Attack,
    Defense,
    Damage,
    Speed,
}

#[derive(Clone, Debug, Default)]
pub struct MasteryState {
    pub attack: i32,
    pub defense: i32,
    pub damage: i32,
    pub speed: i32,
}

impl MasteryState {
    pub fn max_tier(&self) -> i32 {
        self.attack.min(self.defense).min(self.damage).min(self.speed)
    }
}

#[derive(Clone, Debug)]
pub struct WeaponMastery {
    pub group: WeaponGroup,
    pub points: MasteryState,
    pub base_threshold: f32,
}

#[derive(Clone, Debug)]
pub struct Weapon {
    pub name: String,
    pub group: WeaponGroup,
    pub speed: f32,
    pub damage_expr: String,
    pub reach_ft: f32,
    pub armor_pen: i32,
    pub defense_bonus_always: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialKind {
    Metal,
    Fabric,
    Wood,
}

#[derive(Clone, Debug)]
pub struct Material {
    pub tier: i32,
    pub name: &'static str,
    pub weight_mult: f32,
    pub kind: MaterialKind,
}

#[derive(Clone, Debug, Default)]
pub struct Equipment {
    pub weapon: Option<Weapon>,
    pub shield: Option<Shield>,
    pub armor: Option<Armor>,
    pub weapon_material: Option<Material>,
    pub armor_material: Option<Material>,
    pub shield_material: Option<Material>,
}

#[derive(Clone, Debug)]
pub struct Character {
    pub name: String,
    pub level: u8,
    pub progression: Progression,
    pub base_hp: u32,
    pub abilities: AbilitySet,
    pub ability_mods: AbilityDerived,
    pub weapon_masteries: HashMap<WeaponGroup, WeaponMastery>,
    pub equipment: Equipment,
}

#[derive(Clone, Debug, Default)]
pub struct DerivedStats {
    pub attack_bonus: i32,
    pub speed_mod: i32,
    pub initiative_mod: i32,
    pub initiative_die: InitiativeDieQuality,
    pub health_mult: f32,
    pub hit_points: u32,
    pub base_dv: i32,
    pub armor_dr: i32,
    pub load_category: &'static str,
    pub carry_capacity: (u32, u32, u32, u32),
}

impl Character {
    pub fn builder(name: &str) -> CharacterBuilder {
        CharacterBuilder::new(name.to_string())
    }

    pub fn derived(&self) -> DerivedStats {
        let ability = &self.ability_mods;
        let attack_bonus = attack_bonus_for(self.level, self.progression.attack)
            + ability.intelligence.attack
            + ability.dexterity.attack;
        let armor_speed_mod = self
            .equipment
            .armor
            .as_ref()
            .map(|armor| armor.speed_mod)
            .unwrap_or(0);
        let speed_mod = speed_mod_for(self.level, self.progression.speed) + armor_speed_mod;
        let initiative_mod =
            initiative_mod_for(self.level, self.progression.initiative)
                + ability.dexterity.initiative
                + ability.wisdom.initiative
                + self
                    .equipment
                    .armor
                    .as_ref()
                    .map(|armor| armor.initiative_mod)
                    .unwrap_or(0);
        let initiative_die = initiative_die_for(self.level, self.progression.initiative);
        let health_mult = health_mult_for(self.level, self.progression.health);
        let hp_from_con = self.abilities.constitution as f32 * health_mult;
        let hit_points = (self.base_hp as f32 + hp_from_con).round() as u32;

        let armor_dr = self
            .equipment
            .armor
            .as_ref()
            .map(|a| a.damage_reduction)
            .unwrap_or(0);
        let base_dv = BASE_DV
            + ability.dexterity.defense
            + self
                .equipment
                .armor
                .as_ref()
                .map(|a| a.defense_adj)
                .unwrap_or(0);

        let str_mods = self.ability_mods.strength;
        let load_category = if let Some(weight) = self.total_gear_weight() {
            if weight <= str_mods.carry_none {
                "none"
            } else if weight <= str_mods.carry_light {
                "light"
            } else if weight <= str_mods.carry_medium {
                "medium"
            } else if weight <= str_mods.carry_heavy {
                "heavy"
            } else {
                "overloaded"
            }
        } else {
            "unknown"
        };

        DerivedStats {
            attack_bonus,
            speed_mod,
            initiative_mod,
            initiative_die,
            health_mult,
            hit_points,
            base_dv,
            armor_dr,
            load_category,
            carry_capacity: (
                str_mods.carry_none,
                str_mods.carry_light,
                str_mods.carry_medium,
                str_mods.carry_heavy,
            ),
        }
    }

    fn total_gear_weight(&self) -> Option<u32> {
        let mut total = 0.0f32;
        if let Some(w) = self.equipment.weapon.as_ref() {
            total += w.reach_ft; // placeholder until weapon weights are wired
        }
        if let Some(s) = self.equipment.shield.as_ref() {
            total += match s.name {
                "Buckler" => 2.0,
                "Small Shield" => 3.0,
                "Medium Shield" => 6.0,
                "Large Shield" => 10.0,
                _ => 5.0,
            };
        }
        if let Some(a) = self.equipment.armor.as_ref() {
            total += a.weight_lbs;
        }
        if total <= 0.0 {
            None
        } else {
            Some(total as u32)
        }
    }
}

pub struct CharacterBuilder {
    name: String,
    level: u8,
    progression: Progression,
    base_hp: u32,
    abilities: AbilitySet,
    weapon_masteries: HashMap<WeaponGroup, WeaponMastery>,
    equipment: Equipment,
}

impl CharacterBuilder {
    pub fn new(name: String) -> Self {
        Self {
            name,
            level: 1,
            progression: Progression::default(),
            base_hp: 10,
            abilities: AbilitySet::default(),
            weapon_masteries: HashMap::new(),
            equipment: Equipment::default(),
        }
    }

    pub fn level(mut self, level: u8, progression: Progression) -> Self {
        self.level = level;
        self.progression = progression;
        self
    }

    pub fn base_hp(mut self, base_hp: u32) -> Self {
        self.base_hp = base_hp;
        self
    }

    pub fn abilities(mut self, abilities: AbilitySet) -> Self {
        self.abilities = abilities;
        self
    }

    pub fn weapon_mastery(mut self, mastery: WeaponMastery) -> Self {
        self.weapon_masteries.insert(mastery.group, mastery);
        self
    }

    pub fn equipment(mut self, equipment: Equipment) -> Self {
        self.equipment = equipment;
        self
    }

    pub fn build(self) -> Character {
        let ability_mods = derive_abilities(&self.abilities);
        Character {
            name: self.name,
            level: self.level,
            progression: self.progression,
            base_hp: self.base_hp,
            abilities: self.abilities,
            ability_mods,
            weapon_masteries: self.weapon_masteries,
            equipment: self.equipment,
        }
    }
}

fn derive_abilities(abilities: &AbilitySet) -> AbilityDerived {
    AbilityDerived {
        strength: lookup_strength(&abilities.strength),
        dexterity: lookup_dex(&abilities.dexterity),
        intelligence: lookup_int(abilities.intelligence),
        wisdom: lookup_wis(abilities.wisdom),
        constitution: lookup_con(abilities.constitution),
        looks: lookup_looks(abilities.looks),
        charisma: lookup_cha(abilities.charisma),
    }
}

fn attack_bonus_for(level: u8, tier: ProgressionTier) -> i32 {
    let idx = level.clamp(1, 20) as usize - 1;
    ATTACK_BONUS_TABLE[idx][tier.attack_index()]
}

fn speed_mod_for(level: u8, tier: ProgressionTier) -> i32 {
    let idx = level.clamp(1, 20) as usize - 1;
    SPEED_TABLE[idx][tier.speed_index()]
}

fn initiative_mod_for(level: u8, tier: ProgressionTier) -> i32 {
    let idx = level.clamp(1, 20) as usize - 1;
    INITIATIVE_TABLE[idx][tier.initiative_index()]
}

fn initiative_die_for(level: u8, tier: ProgressionTier) -> InitiativeDieQuality {
    let idx = level.clamp(1, 20) as usize - 1;
    INITIATIVE_DIE_TABLE[idx][tier.initiative_index()]
}

fn health_mult_for(level: u8, tier: ProgressionTier) -> f32 {
    let idx = level.clamp(1, 20) as usize - 1;
    HEALTH_TABLE[idx][tier.health_index()]
}

// --- Ability lookups (data from references) ---

#[derive(Clone, Copy)]
struct StrengthRow {
    base: u8,
    pct: u8,
    mods: StrengthMods,
}

const STRENGTH_TABLE: &[StrengthRow] = &[
    StrengthRow { base: 1, pct: 1, mods: StrengthMods { damage: -7, feat: -14, lift: 32, carry_none: 3, carry_light: 5, carry_medium: 10, carry_heavy: 15, drag: 80 } },
    StrengthRow { base: 1, pct: 51, mods: StrengthMods { damage: -6, feat: -13, lift: 42, carry_none: 3, carry_light: 6, carry_medium: 13, carry_heavy: 20, drag: 105 } },
    StrengthRow { base: 2, pct: 1, mods: StrengthMods { damage: -6, feat: -12, lift: 52, carry_none: 4, carry_light: 8, carry_medium: 16, carry_heavy: 24, drag: 130 } },
    StrengthRow { base: 2, pct: 51, mods: StrengthMods { damage: -5, feat: -11, lift: 58, carry_none: 5, carry_light: 9, carry_medium: 18, carry_heavy: 27, drag: 145 } },
    StrengthRow { base: 3, pct: 1, mods: StrengthMods { damage: -5, feat: -10, lift: 64, carry_none: 5, carry_light: 10, carry_medium: 20, carry_heavy: 30, drag: 160 } },
    StrengthRow { base: 3, pct: 51, mods: StrengthMods { damage: -4, feat: -9, lift: 76, carry_none: 6, carry_light: 11, carry_medium: 22, carry_heavy: 33, drag: 190 } },
    StrengthRow { base: 4, pct: 1, mods: StrengthMods { damage: -4, feat: -9, lift: 88, carry_none: 6, carry_light: 12, carry_medium: 24, carry_heavy: 36, drag: 220 } },
    StrengthRow { base: 4, pct: 51, mods: StrengthMods { damage: -4, feat: -8, lift: 99, carry_none: 7, carry_light: 13, carry_medium: 26, carry_heavy: 39, drag: 248 } },
    StrengthRow { base: 5, pct: 1, mods: StrengthMods { damage: -3, feat: -7, lift: 110, carry_none: 7, carry_light: 15, carry_medium: 29, carry_heavy: 44, drag: 275 } },
    StrengthRow { base: 5, pct: 51, mods: StrengthMods { damage: -3, feat: -7, lift: 120, carry_none: 8, carry_light: 16, carry_medium: 31, carry_heavy: 47, drag: 300 } },
    StrengthRow { base: 6, pct: 1, mods: StrengthMods { damage: -3, feat: -6, lift: 130, carry_none: 8, carry_light: 16, carry_medium: 32, carry_heavy: 48, drag: 325 } },
    StrengthRow { base: 6, pct: 51, mods: StrengthMods { damage: -2, feat: -5, lift: 140, carry_none: 9, carry_light: 17, carry_medium: 34, carry_heavy: 51, drag: 350 } },
    StrengthRow { base: 7, pct: 1, mods: StrengthMods { damage: -2, feat: -5, lift: 149, carry_none: 9, carry_light: 18, carry_medium: 36, carry_heavy: 54, drag: 373 } },
    StrengthRow { base: 7, pct: 51, mods: StrengthMods { damage: -2, feat: -4, lift: 157, carry_none: 10, carry_light: 19, carry_medium: 38, carry_heavy: 57, drag: 393 } },
    StrengthRow { base: 8, pct: 1, mods: StrengthMods { damage: -1, feat: -3, lift: 166, carry_none: 10, carry_light: 20, carry_medium: 39, carry_heavy: 59, drag: 415 } },
    StrengthRow { base: 8, pct: 51, mods: StrengthMods { damage: -1, feat: -3, lift: 173, carry_none: 10, carry_light: 20, carry_medium: 40, carry_heavy: 60, drag: 433 } },
    StrengthRow { base: 9, pct: 1, mods: StrengthMods { damage: -1, feat: -2, lift: 181, carry_none: 11, carry_light: 21, carry_medium: 42, carry_heavy: 63, drag: 453 } },
    StrengthRow { base: 9, pct: 51, mods: StrengthMods { damage: -1, feat: -1, lift: 187, carry_none: 11, carry_light: 22, carry_medium: 43, carry_heavy: 65, drag: 468 } },
    StrengthRow { base: 10, pct: 1, mods: StrengthMods { damage: 0, feat: 0, lift: 194, carry_none: 11, carry_light: 22, carry_medium: 44, carry_heavy: 66, drag: 485 } },
    StrengthRow { base: 10, pct: 51, mods: StrengthMods { damage: 0, feat: 0, lift: 200, carry_none: 11, carry_light: 23, carry_medium: 45, carry_heavy: 68, drag: 500 } },
    StrengthRow { base: 11, pct: 1, mods: StrengthMods { damage: 0, feat: 0, lift: 205, carry_none: 12, carry_light: 24, carry_medium: 48, carry_heavy: 72, drag: 513 } },
    StrengthRow { base: 11, pct: 51, mods: StrengthMods { damage: 0, feat: 0, lift: 210, carry_none: 13, carry_light: 26, carry_medium: 52, carry_heavy: 78, drag: 525 } },
    StrengthRow { base: 12, pct: 1, mods: StrengthMods { damage: 1, feat: 1, lift: 215, carry_none: 14, carry_light: 28, carry_medium: 56, carry_heavy: 84, drag: 538 } },
    StrengthRow { base: 12, pct: 51, mods: StrengthMods { damage: 1, feat: 2, lift: 220, carry_none: 15, carry_light: 31, carry_medium: 61, carry_heavy: 92, drag: 550 } },
    StrengthRow { base: 13, pct: 1, mods: StrengthMods { damage: 1, feat: 3, lift: 225, carry_none: 17, carry_light: 33, carry_medium: 66, carry_heavy: 99, drag: 563 } },
    StrengthRow { base: 13, pct: 51, mods: StrengthMods { damage: 1, feat: 4, lift: 230, carry_none: 18, carry_light: 36, carry_medium: 71, carry_heavy: 107, drag: 575 } },
    StrengthRow { base: 14, pct: 1, mods: StrengthMods { damage: 2, feat: 5, lift: 235, carry_none: 19, carry_light: 39, carry_medium: 77, carry_heavy: 116, drag: 588 } },
    StrengthRow { base: 14, pct: 51, mods: StrengthMods { damage: 2, feat: 6, lift: 240, carry_none: 21, carry_light: 42, carry_medium: 84, carry_heavy: 126, drag: 600 } },
    StrengthRow { base: 15, pct: 1, mods: StrengthMods { damage: 2, feat: 7, lift: 245, carry_none: 23, carry_light: 46, carry_medium: 91, carry_heavy: 137, drag: 613 } },
    StrengthRow { base: 15, pct: 51, mods: StrengthMods { damage: 3, feat: 8, lift: 267, carry_none: 25, carry_light: 50, carry_medium: 99, carry_heavy: 149, drag: 668 } },
    StrengthRow { base: 16, pct: 1, mods: StrengthMods { damage: 3, feat: 9, lift: 291, carry_none: 27, carry_light: 54, carry_medium: 108, carry_heavy: 162, drag: 728 } },
    StrengthRow { base: 16, pct: 51, mods: StrengthMods { damage: 3, feat: 10, lift: 318, carry_none: 30, carry_light: 50, carry_medium: 118, carry_heavy: 177, drag: 795 } },
    StrengthRow { base: 17, pct: 1, mods: StrengthMods { damage: 4, feat: 11, lift: 347, carry_none: 32, carry_light: 65, carry_medium: 129, carry_heavy: 194, drag: 868 } },
    StrengthRow { base: 17, pct: 51, mods: StrengthMods { damage: 4, feat: 12, lift: 380, carry_none: 36, carry_light: 71, carry_medium: 142, carry_heavy: 213, drag: 950 } },
    StrengthRow { base: 18, pct: 1, mods: StrengthMods { damage: 4, feat: 13, lift: 417, carry_none: 39, carry_light: 78, carry_medium: 156, carry_heavy: 234, drag: 1043 } },
    StrengthRow { base: 18, pct: 51, mods: StrengthMods { damage: 5, feat: 14, lift: 458, carry_none: 43, carry_light: 86, carry_medium: 171, carry_heavy: 257, drag: 1145 } },
    StrengthRow { base: 19, pct: 1, mods: StrengthMods { damage: 5, feat: 15, lift: 504, carry_none: 47, carry_light: 95, carry_medium: 189, carry_heavy: 284, drag: 1260 } },
    StrengthRow { base: 19, pct: 51, mods: StrengthMods { damage: 6, feat: 16, lift: 554, carry_none: 52, carry_light: 105, carry_medium: 209, carry_heavy: 314, drag: 1385 } },
    StrengthRow { base: 20, pct: 1, mods: StrengthMods { damage: 6, feat: 17, lift: 612, carry_none: 58, carry_light: 116, carry_medium: 231, carry_heavy: 347, drag: 1530 } },
    StrengthRow { base: 20, pct: 51, mods: StrengthMods { damage: 7, feat: 18, lift: 675, carry_none: 64, carry_light: 128, carry_medium: 256, carry_heavy: 384, drag: 1688 } },
    StrengthRow { base: 21, pct: 1, mods: StrengthMods { damage: 7, feat: 19, lift: 743, carry_none: 70, carry_light: 140, carry_medium: 280, carry_heavy: 420, drag: 1857 } },
    StrengthRow { base: 21, pct: 51, mods: StrengthMods { damage: 8, feat: 20, lift: 817, carry_none: 77, carry_light: 154, carry_medium: 308, carry_heavy: 462, drag: 2042 } },
    StrengthRow { base: 22, pct: 1, mods: StrengthMods { damage: 8, feat: 21, lift: 898, carry_none: 85, carry_light: 170, carry_medium: 340, carry_heavy: 510, drag: 2245 } },
    StrengthRow { base: 22, pct: 51, mods: StrengthMods { damage: 9, feat: 22, lift: 988, carry_none: 94, carry_light: 188, carry_medium: 376, carry_heavy: 564, drag: 2471 } },
    StrengthRow { base: 23, pct: 1, mods: StrengthMods { damage: 9, feat: 23, lift: 1088, carry_none: 103, carry_light: 206, carry_medium: 412, carry_heavy: 618, drag: 2719 } },
    StrengthRow { base: 23, pct: 51, mods: StrengthMods { damage: 10, feat: 24, lift: 1196, carry_none: 113, carry_light: 226, carry_medium: 456, carry_heavy: 678, drag: 2990 } },
    StrengthRow { base: 24, pct: 1, mods: StrengthMods { damage: 10, feat: 25, lift: 1316, carry_none: 125, carry_light: 250, carry_medium: 500, carry_heavy: 750, drag: 3289 } },
    StrengthRow { base: 24, pct: 51, mods: StrengthMods { damage: 11, feat: 26, lift: 1447, carry_none: 137, carry_light: 274, carry_medium: 548, carry_heavy: 822, drag: 3618 } },
    StrengthRow { base: 25, pct: 1, mods: StrengthMods { damage: 11, feat: 27, lift: 1592, carry_none: 151, carry_light: 302, carry_medium: 604, carry_heavy: 906, drag: 3980 } },
    StrengthRow { base: 25, pct: 51, mods: StrengthMods { damage: 12, feat: 28, lift: 1750, carry_none: 166, carry_light: 333, carry_medium: 667, carry_heavy: 1000, drag: 4375 } },
];

fn lookup_strength(score: &AbilityScore) -> StrengthMods {
    STRENGTH_TABLE
        .iter()
        .find(|row| row.base == score.base && row.pct == score.percentile)
        .map(|row| row.mods)
        .unwrap_or_default()
}

#[derive(Clone, Copy)]
struct DexRow {
    base: u8,
    pct: u8,
    mods: DexMods,
}

const DEX_TABLE: &[DexRow] = &[
    DexRow { base: 1, pct: 1, mods: DexMods { initiative: 8, attack: -5, defense: -6, dodge_save: -4, feat_of_agility: -14 } },
    DexRow { base: 1, pct: 51, mods: DexMods { initiative: 8, attack: -4, defense: -6, dodge_save: -4, feat_of_agility: -13 } },
    DexRow { base: 2, pct: 1, mods: DexMods { initiative: 8, attack: -4, defense: -6, dodge_save: -4, feat_of_agility: -12 } },
    DexRow { base: 2, pct: 51, mods: DexMods { initiative: 7, attack: -4, defense: -5, dodge_save: -4, feat_of_agility: -11 } },
    DexRow { base: 3, pct: 1, mods: DexMods { initiative: 7, attack: -4, defense: -5, dodge_save: -3, feat_of_agility: -10 } },
    DexRow { base: 3, pct: 51, mods: DexMods { initiative: 7, attack: -3, defense: -5, dodge_save: -3, feat_of_agility: -9 } },
    DexRow { base: 4, pct: 1, mods: DexMods { initiative: 6, attack: -3, defense: -4, dodge_save: -3, feat_of_agility: -9 } },
    DexRow { base: 4, pct: 51, mods: DexMods { initiative: 6, attack: -3, defense: -4, dodge_save: -3, feat_of_agility: -8 } },
    DexRow { base: 5, pct: 1, mods: DexMods { initiative: 6, attack: -3, defense: -4, dodge_save: -2, feat_of_agility: -7 } },
    DexRow { base: 5, pct: 51, mods: DexMods { initiative: 5, attack: -2, defense: -3, dodge_save: -2, feat_of_agility: -7 } },
    DexRow { base: 6, pct: 1, mods: DexMods { initiative: 5, attack: -2, defense: -3, dodge_save: -2, feat_of_agility: -6 } },
    DexRow { base: 6, pct: 51, mods: DexMods { initiative: 5, attack: -2, defense: -3, dodge_save: -2, feat_of_agility: -5 } },
    DexRow { base: 7, pct: 1, mods: DexMods { initiative: 4, attack: -2, defense: -2, dodge_save: -1, feat_of_agility: -5 } },
    DexRow { base: 7, pct: 51, mods: DexMods { initiative: 4, attack: -1, defense: -2, dodge_save: -1, feat_of_agility: -4 } },
    DexRow { base: 8, pct: 1, mods: DexMods { initiative: 4, attack: -1, defense: -2, dodge_save: -1, feat_of_agility: -3 } },
    DexRow { base: 8, pct: 51, mods: DexMods { initiative: 3, attack: -1, defense: -1, dodge_save: -1, feat_of_agility: -3 } },
    DexRow { base: 9, pct: 1, mods: DexMods { initiative: 3, attack: -1, defense: -1, dodge_save: 0, feat_of_agility: -2 } },
    DexRow { base: 9, pct: 51, mods: DexMods { initiative: 3, attack: 0, defense: -1, dodge_save: 0, feat_of_agility: -1 } },
    DexRow { base: 10, pct: 1, mods: DexMods { initiative: 2, attack: 0, defense: 0, dodge_save: 0, feat_of_agility: 0 } },
    DexRow { base: 10, pct: 51, mods: DexMods { initiative: 2, attack: 0, defense: 0, dodge_save: 0, feat_of_agility: 0 } },
    DexRow { base: 11, pct: 1, mods: DexMods { initiative: 2, attack: 0, defense: 0, dodge_save: 0, feat_of_agility: 0 } },
    DexRow { base: 11, pct: 51, mods: DexMods { initiative: 1, attack: 0, defense: 1, dodge_save: 0, feat_of_agility: 0 } },
    DexRow { base: 12, pct: 1, mods: DexMods { initiative: 1, attack: 1, defense: 1, dodge_save: 0, feat_of_agility: 1 } },
    DexRow { base: 12, pct: 51, mods: DexMods { initiative: 1, attack: 1, defense: 1, dodge_save: 0, feat_of_agility: 2 } },
    DexRow { base: 13, pct: 1, mods: DexMods { initiative: 0, attack: 1, defense: 2, dodge_save: 1, feat_of_agility: 3 } },
    DexRow { base: 13, pct: 51, mods: DexMods { initiative: 0, attack: 1, defense: 2, dodge_save: 1, feat_of_agility: 4 } },
    DexRow { base: 14, pct: 1, mods: DexMods { initiative: 0, attack: 2, defense: 2, dodge_save: 1, feat_of_agility: 5 } },
    DexRow { base: 14, pct: 51, mods: DexMods { initiative: -1, attack: 2, defense: 3, dodge_save: 1, feat_of_agility: 6 } },
    DexRow { base: 15, pct: 1, mods: DexMods { initiative: -1, attack: 2, defense: 3, dodge_save: 2, feat_of_agility: 7 } },
    DexRow { base: 15, pct: 51, mods: DexMods { initiative: -1, attack: 2, defense: 3, dodge_save: 2, feat_of_agility: 8 } },
    DexRow { base: 16, pct: 1, mods: DexMods { initiative: -2, attack: 3, defense: 4, dodge_save: 2, feat_of_agility: 9 } },
    DexRow { base: 16, pct: 51, mods: DexMods { initiative: -2, attack: 3, defense: 4, dodge_save: 2, feat_of_agility: 10 } },
    DexRow { base: 17, pct: 1, mods: DexMods { initiative: -2, attack: 3, defense: 4, dodge_save: 2, feat_of_agility: 11 } },
    DexRow { base: 17, pct: 51, mods: DexMods { initiative: -3, attack: 3, defense: 5, dodge_save: 2, feat_of_agility: 12 } },
    DexRow { base: 18, pct: 1, mods: DexMods { initiative: -3, attack: 4, defense: 5, dodge_save: 3, feat_of_agility: 13 } },
    DexRow { base: 18, pct: 51, mods: DexMods { initiative: -3, attack: 4, defense: 5, dodge_save: 3, feat_of_agility: 14 } },
    DexRow { base: 19, pct: 1, mods: DexMods { initiative: -4, attack: 4, defense: 6, dodge_save: 3, feat_of_agility: 15 } },
    DexRow { base: 19, pct: 51, mods: DexMods { initiative: -4, attack: 4, defense: 6, dodge_save: 3, feat_of_agility: 16 } },
    DexRow { base: 20, pct: 1, mods: DexMods { initiative: -4, attack: 5, defense: 6, dodge_save: 3, feat_of_agility: 17 } },
    DexRow { base: 20, pct: 51, mods: DexMods { initiative: -5, attack: 5, defense: 7, dodge_save: 3, feat_of_agility: 18 } },
    DexRow { base: 21, pct: 1, mods: DexMods { initiative: -5, attack: 5, defense: 7, dodge_save: 3, feat_of_agility: 19 } },
    DexRow { base: 21, pct: 51, mods: DexMods { initiative: -5, attack: 5, defense: 7, dodge_save: 3, feat_of_agility: 20 } },
    DexRow { base: 22, pct: 1, mods: DexMods { initiative: -5, attack: 5, defense: 7, dodge_save: 4, feat_of_agility: 21 } },
    DexRow { base: 22, pct: 51, mods: DexMods { initiative: -6, attack: 6, defense: 8, dodge_save: 4, feat_of_agility: 22 } },
    DexRow { base: 23, pct: 1, mods: DexMods { initiative: -6, attack: 6, defense: 8, dodge_save: 4, feat_of_agility: 23 } },
    DexRow { base: 23, pct: 51, mods: DexMods { initiative: -6, attack: 6, defense: 8, dodge_save: 4, feat_of_agility: 24 } },
    DexRow { base: 24, pct: 1, mods: DexMods { initiative: -6, attack: 6, defense: 8, dodge_save: 4, feat_of_agility: 25 } },
    DexRow { base: 24, pct: 51, mods: DexMods { initiative: -7, attack: 6, defense: 8, dodge_save: 4, feat_of_agility: 26 } },
    DexRow { base: 25, pct: 1, mods: DexMods { initiative: -7, attack: 6, defense: 9, dodge_save: 4, feat_of_agility: 27 } },
    DexRow { base: 25, pct: 51, mods: DexMods { initiative: -7, attack: 7, defense: 9, dodge_save: 4, feat_of_agility: 28 } },
];

fn lookup_dex(score: &AbilityScore) -> DexMods {
    DEX_TABLE
        .iter()
        .find(|row| row.base == score.base && row.pct == score.percentile)
        .map(|row| row.mods)
        .unwrap_or_default()
}

#[derive(Clone, Copy)]
struct SimpleRow<T> {
    score: u8,
    mods: T,
}

const INT_TABLE: &[SimpleRow<IntMods>] = &[
    SimpleRow { score: 1, mods: IntMods { attack: -5, lp_bonus: 0, weapon_exp_threshold_pct: 18 } },
    SimpleRow { score: 2, mods: IntMods { attack: -4, lp_bonus: 0, weapon_exp_threshold_pct: 16 } },
    SimpleRow { score: 3, mods: IntMods { attack: -3, lp_bonus: 0, weapon_exp_threshold_pct: 14 } },
    SimpleRow { score: 4, mods: IntMods { attack: -2, lp_bonus: 0, weapon_exp_threshold_pct: 12 } },
    SimpleRow { score: 5, mods: IntMods { attack: -2, lp_bonus: 0, weapon_exp_threshold_pct: 10 } },
    SimpleRow { score: 6, mods: IntMods { attack: -2, lp_bonus: 0, weapon_exp_threshold_pct: 8 } },
    SimpleRow { score: 7, mods: IntMods { attack: -1, lp_bonus: 0, weapon_exp_threshold_pct: 6 } },
    SimpleRow { score: 8, mods: IntMods { attack: -1, lp_bonus: 0, weapon_exp_threshold_pct: 4 } },
    SimpleRow { score: 9, mods: IntMods { attack: -1, lp_bonus: 0, weapon_exp_threshold_pct: 2 } },
    SimpleRow { score: 10, mods: IntMods { attack: 0, lp_bonus: 0, weapon_exp_threshold_pct: 0 } },
    SimpleRow { score: 11, mods: IntMods { attack: 0, lp_bonus: 1, weapon_exp_threshold_pct: -2 } },
    SimpleRow { score: 12, mods: IntMods { attack: 1, lp_bonus: 2, weapon_exp_threshold_pct: -4 } },
    SimpleRow { score: 13, mods: IntMods { attack: 1, lp_bonus: 3, weapon_exp_threshold_pct: -6 } },
    SimpleRow { score: 14, mods: IntMods { attack: 1, lp_bonus: 6, weapon_exp_threshold_pct: -8 } },
    SimpleRow { score: 15, mods: IntMods { attack: 2, lp_bonus: 10, weapon_exp_threshold_pct: -10 } },
    SimpleRow { score: 16, mods: IntMods { attack: 2, lp_bonus: 15, weapon_exp_threshold_pct: -12 } },
    SimpleRow { score: 17, mods: IntMods { attack: 2, lp_bonus: 21, weapon_exp_threshold_pct: -14 } },
    SimpleRow { score: 18, mods: IntMods { attack: 3, lp_bonus: 28, weapon_exp_threshold_pct: -16 } },
    SimpleRow { score: 19, mods: IntMods { attack: 3, lp_bonus: 36, weapon_exp_threshold_pct: -18 } },
    SimpleRow { score: 20, mods: IntMods { attack: 3, lp_bonus: 45, weapon_exp_threshold_pct: -20 } },
    SimpleRow { score: 21, mods: IntMods { attack: 4, lp_bonus: 55, weapon_exp_threshold_pct: -22 } },
    SimpleRow { score: 22, mods: IntMods { attack: 4, lp_bonus: 66, weapon_exp_threshold_pct: -24 } },
    SimpleRow { score: 23, mods: IntMods { attack: 5, lp_bonus: 78, weapon_exp_threshold_pct: -26 } },
    SimpleRow { score: 24, mods: IntMods { attack: 5, lp_bonus: 91, weapon_exp_threshold_pct: -28 } },
    SimpleRow { score: 25, mods: IntMods { attack: 6, lp_bonus: 105, weapon_exp_threshold_pct: -30 } },
];

const WIS_TABLE: &[SimpleRow<WisMods>] = &[
    SimpleRow { score: 1, mods: WisMods { initiative: 7, lp_bonus: 0, defense: -4, mental_save: -4, base_ff: 3 } },
    SimpleRow { score: 2, mods: WisMods { initiative: 6, lp_bonus: 0, defense: -3, mental_save: -4, base_ff: 3 } },
    SimpleRow { score: 3, mods: WisMods { initiative: 5, lp_bonus: 0, defense: -3, mental_save: -3, base_ff: 2 } },
    SimpleRow { score: 4, mods: WisMods { initiative: 4, lp_bonus: 0, defense: -2, mental_save: -3, base_ff: 2 } },
    SimpleRow { score: 5, mods: WisMods { initiative: 4, lp_bonus: 0, defense: -2, mental_save: -2, base_ff: 2 } },
    SimpleRow { score: 6, mods: WisMods { initiative: 4, lp_bonus: 0, defense: -2, mental_save: -2, base_ff: 1 } },
    SimpleRow { score: 7, mods: WisMods { initiative: 3, lp_bonus: 0, defense: -1, mental_save: -1, base_ff: 1 } },
    SimpleRow { score: 8, mods: WisMods { initiative: 3, lp_bonus: 0, defense: -1, mental_save: -1, base_ff: 1 } },
    SimpleRow { score: 9, mods: WisMods { initiative: 3, lp_bonus: 0, defense: -1, mental_save: 0, base_ff: 0 } },
    SimpleRow { score: 10, mods: WisMods { initiative: 2, lp_bonus: 0, defense: 0, mental_save: 0, base_ff: 0 } },
    SimpleRow { score: 11, mods: WisMods { initiative: 2, lp_bonus: 1, defense: 0, mental_save: 0, base_ff: 0 } },
    SimpleRow { score: 12, mods: WisMods { initiative: 1, lp_bonus: 2, defense: 1, mental_save: 0, base_ff: -1 } },
    SimpleRow { score: 13, mods: WisMods { initiative: 1, lp_bonus: 3, defense: 1, mental_save: 1, base_ff: -1 } },
    SimpleRow { score: 14, mods: WisMods { initiative: 1, lp_bonus: 6, defense: 1, mental_save: 1, base_ff: -1 } },
    SimpleRow { score: 15, mods: WisMods { initiative: 0, lp_bonus: 10, defense: 2, mental_save: 2, base_ff: -2 } },
    SimpleRow { score: 16, mods: WisMods { initiative: 0, lp_bonus: 15, defense: 2, mental_save: 2, base_ff: -2 } },
    SimpleRow { score: 17, mods: WisMods { initiative: 0, lp_bonus: 21, defense: 2, mental_save: 2, base_ff: -2 } },
    SimpleRow { score: 18, mods: WisMods { initiative: -1, lp_bonus: 28, defense: 3, mental_save: 3, base_ff: -3 } },
    SimpleRow { score: 19, mods: WisMods { initiative: -1, lp_bonus: 36, defense: 3, mental_save: 3, base_ff: -3 } },
    SimpleRow { score: 20, mods: WisMods { initiative: -1, lp_bonus: 45, defense: 3, mental_save: 3, base_ff: -3 } },
    SimpleRow { score: 21, mods: WisMods { initiative: -2, lp_bonus: 55, defense: 4, mental_save: 3, base_ff: -4 } },
    SimpleRow { score: 22, mods: WisMods { initiative: -2, lp_bonus: 66, defense: 4, mental_save: 4, base_ff: -4 } },
    SimpleRow { score: 23, mods: WisMods { initiative: -2, lp_bonus: 78, defense: 4, mental_save: 4, base_ff: -4 } },
    SimpleRow { score: 24, mods: WisMods { initiative: -3, lp_bonus: 91, defense: 4, mental_save: 4, base_ff: -5 } },
    SimpleRow { score: 25, mods: WisMods { initiative: -3, lp_bonus: 105, defense: 5, mental_save: 4, base_ff: -5 } },
];

const CON_TABLE: &[SimpleRow<ConMods>] = &[
    SimpleRow { score: 1, mods: ConMods { physical_save: -5, base_ff: 5 } },
    SimpleRow { score: 2, mods: ConMods { physical_save: -4, base_ff: 4 } },
    SimpleRow { score: 3, mods: ConMods { physical_save: -3, base_ff: 4 } },
    SimpleRow { score: 4, mods: ConMods { physical_save: -3, base_ff: 3 } },
    SimpleRow { score: 5, mods: ConMods { physical_save: -2, base_ff: 3 } },
    SimpleRow { score: 6, mods: ConMods { physical_save: -2, base_ff: 2 } },
    SimpleRow { score: 7, mods: ConMods { physical_save: -1, base_ff: 2 } },
    SimpleRow { score: 8, mods: ConMods { physical_save: -1, base_ff: 1 } },
    SimpleRow { score: 9, mods: ConMods { physical_save: 0, base_ff: 1 } },
    SimpleRow { score: 10, mods: ConMods { physical_save: 0, base_ff: 0 } },
    SimpleRow { score: 11, mods: ConMods { physical_save: 0, base_ff: -1 } },
    SimpleRow { score: 12, mods: ConMods { physical_save: 0, base_ff: -1 } },
    SimpleRow { score: 13, mods: ConMods { physical_save: 1, base_ff: -2 } },
    SimpleRow { score: 14, mods: ConMods { physical_save: 1, base_ff: -2 } },
    SimpleRow { score: 15, mods: ConMods { physical_save: 2, base_ff: -3 } },
    SimpleRow { score: 16, mods: ConMods { physical_save: 2, base_ff: -3 } },
    SimpleRow { score: 17, mods: ConMods { physical_save: 2, base_ff: -4 } },
    SimpleRow { score: 18, mods: ConMods { physical_save: 3, base_ff: -4 } },
    SimpleRow { score: 19, mods: ConMods { physical_save: 3, base_ff: -5 } },
    SimpleRow { score: 20, mods: ConMods { physical_save: 3, base_ff: -5 } },
    SimpleRow { score: 21, mods: ConMods { physical_save: 4, base_ff: -6 } },
    SimpleRow { score: 22, mods: ConMods { physical_save: 4, base_ff: -6 } },
    SimpleRow { score: 23, mods: ConMods { physical_save: 4, base_ff: -7 } },
    SimpleRow { score: 24, mods: ConMods { physical_save: 4, base_ff: -7 } },
    SimpleRow { score: 25, mods: ConMods { physical_save: 4, base_ff: -8 } },
];

const LOOKS_TABLE: &[SimpleRow<LooksMods>] = &[
    SimpleRow { score: 1, mods: LooksMods { charisma: -6, honor: -6, fame: 9 } },
    SimpleRow { score: 2, mods: LooksMods { charisma: -5, honor: -5, fame: 6 } },
    SimpleRow { score: 3, mods: LooksMods { charisma: -5, honor: -4, fame: 4 } },
    SimpleRow { score: 4, mods: LooksMods { charisma: -4, honor: -3, fame: 2 } },
    SimpleRow { score: 5, mods: LooksMods { charisma: -3, honor: -3, fame: 1 } },
    SimpleRow { score: 6, mods: LooksMods { charisma: -2, honor: -2, fame: 1 } },
    SimpleRow { score: 7, mods: LooksMods { charisma: -2, honor: -2, fame: 0 } },
    SimpleRow { score: 8, mods: LooksMods { charisma: -1, honor: -1, fame: 0 } },
    SimpleRow { score: 9, mods: LooksMods { charisma: -1, honor: -1, fame: 0 } },
    SimpleRow { score: 10, mods: LooksMods { charisma: 0, honor: 0, fame: 0 } },
    SimpleRow { score: 11, mods: LooksMods { charisma: 0, honor: 0, fame: 0 } },
    SimpleRow { score: 12, mods: LooksMods { charisma: 0, honor: 1, fame: 0 } },
    SimpleRow { score: 13, mods: LooksMods { charisma: 1, honor: 1, fame: 0 } },
    SimpleRow { score: 14, mods: LooksMods { charisma: 1, honor: 2, fame: 1 } },
    SimpleRow { score: 15, mods: LooksMods { charisma: 2, honor: 2, fame: 2 } },
    SimpleRow { score: 16, mods: LooksMods { charisma: 2, honor: 3, fame: 3 } },
    SimpleRow { score: 17, mods: LooksMods { charisma: 3, honor: 3, fame: 5 } },
    SimpleRow { score: 18, mods: LooksMods { charisma: 4, honor: 4, fame: 7 } },
    SimpleRow { score: 19, mods: LooksMods { charisma: 5, honor: 4, fame: 8 } },
    SimpleRow { score: 20, mods: LooksMods { charisma: 6, honor: 5, fame: 9 } },
    SimpleRow { score: 21, mods: LooksMods { charisma: 7, honor: 5, fame: 11 } },
    SimpleRow { score: 22, mods: LooksMods { charisma: 8, honor: 6, fame: 13 } },
    SimpleRow { score: 23, mods: LooksMods { charisma: 9, honor: 6, fame: 14 } },
    SimpleRow { score: 24, mods: LooksMods { charisma: 10, honor: 7, fame: 15 } },
    SimpleRow { score: 25, mods: LooksMods { charisma: 11, honor: 7, fame: 17 } },
];

const CHA_TABLE: &[SimpleRow<ChaMods>] = &[
    SimpleRow { score: 1, mods: ChaMods { lp_bonus: 0, honor: -6, turning: -9, morale: -5, max_proteges: 0 } },
    SimpleRow { score: 2, mods: ChaMods { lp_bonus: 0, honor: -5, turning: -8, morale: -4, max_proteges: 1 } },
    SimpleRow { score: 3, mods: ChaMods { lp_bonus: 0, honor: -4, turning: -7, morale: -4, max_proteges: 1 } },
    SimpleRow { score: 4, mods: ChaMods { lp_bonus: 0, honor: -3, turning: -6, morale: -3, max_proteges: 1 } },
    SimpleRow { score: 5, mods: ChaMods { lp_bonus: 0, honor: -3, turning: -5, morale: -3, max_proteges: 1 } },
    SimpleRow { score: 6, mods: ChaMods { lp_bonus: 0, honor: -2, turning: -4, morale: -2, max_proteges: 2 } },
    SimpleRow { score: 7, mods: ChaMods { lp_bonus: 0, honor: -2, turning: -3, morale: -2, max_proteges: 2 } },
    SimpleRow { score: 8, mods: ChaMods { lp_bonus: 0, honor: -1, turning: -2, morale: -1, max_proteges: 2 } },
    SimpleRow { score: 9, mods: ChaMods { lp_bonus: 0, honor: -1, turning: -1, morale: -1, max_proteges: 2 } },
    SimpleRow { score: 10, mods: ChaMods { lp_bonus: 0, honor: 0, turning: 0, morale: 0, max_proteges: 3 } },
    SimpleRow { score: 11, mods: ChaMods { lp_bonus: 0, honor: 0, turning: 1, morale: 1, max_proteges: 3 } },
    SimpleRow { score: 12, mods: ChaMods { lp_bonus: 1, honor: 1, turning: 2, morale: 1, max_proteges: 3 } },
    SimpleRow { score: 13, mods: ChaMods { lp_bonus: 3, honor: 1, turning: 3, morale: 2, max_proteges: 3 } },
    SimpleRow { score: 14, mods: ChaMods { lp_bonus: 6, honor: 2, turning: 4, morale: 2, max_proteges: 3 } },
    SimpleRow { score: 15, mods: ChaMods { lp_bonus: 10, honor: 2, turning: 5, morale: 3, max_proteges: 3 } },
    SimpleRow { score: 16, mods: ChaMods { lp_bonus: 15, honor: 3, turning: 6, morale: 3, max_proteges: 4 } },
    SimpleRow { score: 17, mods: ChaMods { lp_bonus: 21, honor: 3, turning: 7, morale: 4, max_proteges: 4 } },
    SimpleRow { score: 18, mods: ChaMods { lp_bonus: 28, honor: 4, turning: 8, morale: 4, max_proteges: 4 } },
    SimpleRow { score: 19, mods: ChaMods { lp_bonus: 36, honor: 4, turning: 9, morale: 5, max_proteges: 4 } },
    SimpleRow { score: 20, mods: ChaMods { lp_bonus: 45, honor: 5, turning: 10, morale: 5, max_proteges: 4 } },
    SimpleRow { score: 21, mods: ChaMods { lp_bonus: 55, honor: 5, turning: 11, morale: 6, max_proteges: 5 } },
    SimpleRow { score: 22, mods: ChaMods { lp_bonus: 66, honor: 6, turning: 12, morale: 6, max_proteges: 5 } },
    SimpleRow { score: 23, mods: ChaMods { lp_bonus: 78, honor: 6, turning: 13, morale: 7, max_proteges: 5 } },
    SimpleRow { score: 24, mods: ChaMods { lp_bonus: 91, honor: 7, turning: 14, morale: 7, max_proteges: 5 } },
    SimpleRow { score: 25, mods: ChaMods { lp_bonus: 105, honor: 7, turning: 15, morale: 8, max_proteges: 6 } },
];

fn lookup_int(score: u8) -> IntMods {
    INT_TABLE
        .iter()
        .find(|row| row.score == score)
        .map(|row| row.mods)
        .unwrap_or_default()
}

fn lookup_wis(score: u8) -> WisMods {
    WIS_TABLE
        .iter()
        .find(|row| row.score == score)
        .map(|row| row.mods)
        .unwrap_or_default()
}

fn lookup_con(score: u8) -> ConMods {
    CON_TABLE
        .iter()
        .find(|row| row.score == score)
        .map(|row| row.mods)
        .unwrap_or_default()
}

fn lookup_looks(score: u8) -> LooksMods {
    LOOKS_TABLE
        .iter()
        .find(|row| row.score == score)
        .map(|row| row.mods)
        .unwrap_or_default()
}

fn lookup_cha(score: u8) -> ChaMods {
    CHA_TABLE
        .iter()
        .find(|row| row.score == score)
        .map(|row| row.mods)
        .unwrap_or_default()
}

// --- Advancement tables (data from references) ---

const ATTACK_BONUS_TABLE: [[i32; 6]; 20] = [
    [0, 0, 0, 0, 1, 1],
    [0, 0, 0, 0, 1, 1],
    [0, 0, 1, 1, 2, 2],
    [0, 1, 1, 1, 2, 2],
    [1, 1, 2, 2, 3, 3],
    [1, 1, 2, 2, 3, 3],
    [1, 1, 2, 3, 4, 4],
    [1, 2, 3, 3, 4, 4],
    [1, 2, 3, 4, 5, 5],
    [1, 2, 3, 4, 5, 5],
    [2, 2, 4, 5, 6, 6],
    [2, 3, 4, 5, 6, 7],
    [2, 3, 4, 6, 7, 7],
    [2, 3, 5, 6, 7, 8],
    [2, 3, 5, 7, 8, 9],
    [2, 4, 5, 7, 8, 9],
    [3, 4, 6, 8, 9, 10],
    [3, 4, 6, 8, 9, 11],
    [3, 4, 6, 9, 10, 11],
    [3, 5, 7, 9, 10, 12],
];

const SPEED_TABLE: [[i32; 6]; 20] = [
    [1, 0, 0, 0, 0, 0],
    [1, 0, 0, 0, 0, 0],
    [1, 0, 0, 0, 0, 0],
    [1, 0, 0, 0, 0, -1],
    [1, 0, 0, 0, -1, -1],
    [1, 0, 0, 0, -1, -1],
    [1, 0, -1, -1, -1, -1],
    [1, 0, -1, -1, -1, -2],
    [1, 0, -1, -1, -2, -2],
    [1, 0, -1, -1, -2, -2],
    [0, -1, -1, -2, -2, -3],
    [0, -1, -1, -2, -2, -3],
    [0, -1, -1, -2, -2, -3],
    [0, -1, -2, -2, -3, -3],
    [0, -1, -2, -2, -3, -4],
    [0, -1, -2, -3, -3, -4],
    [0, -1, -2, -3, -3, -4],
    [0, -1, -2, -3, -3, -4],
    [0, -1, -2, -3, -3, -4],
    [0, -1, -2, -3, -4, -5],
];

const INITIATIVE_TABLE: [[i32; 5]; 20] = [
    [2, 1, 0, 0, -1],
    [2, 1, 0, 0, -1],
    [2, 1, 0, 0, -1],
    [2, 1, 0, -1, -1],
    [1, 1, 0, -1, -2],
    [1, 0, -1, -1, -2],
    [1, 0, -1, -1, -2],
    [1, 0, -1, -2, -2],
    [1, 0, -1, -2, -2],
    [1, 0, -1, -2, -3],
    [0, 0, -1, -2, -3],
    [0, -1, -2, -2, -3],
    [0, -1, -2, -2, -3],
    [0, -1, -2, -3, -3],
    [0, -1, -2, -3, -3],
    [0, -1, -2, -3, -4],
    [0, -1, -2, -3, -4],
    [0, -1, -2, -3, -4],
    [-1, -2, -3, -3, -4],
    [-1, -2, -3, -3, -4],
];

const INITIATIVE_DIE_TABLE: [[InitiativeDieQuality; 5]; 20] = [
    [q(), q(), q(), q(), q()],
    [q(), q(), q(), q(), InitiativeDieQuality::OneBetter],
    [q(), q(), q(), q(), InitiativeDieQuality::OneBetter],
    [q(), q(), q(), InitiativeDieQuality::OneBetter, InitiativeDieQuality::OneBetter],
    [q(), q(), q(), InitiativeDieQuality::OneBetter, InitiativeDieQuality::TwoBetter],
    [q(), q(), InitiativeDieQuality::OneBetter, InitiativeDieQuality::OneBetter, InitiativeDieQuality::TwoBetter],
    [q(), q(), InitiativeDieQuality::OneBetter, InitiativeDieQuality::OneBetter, InitiativeDieQuality::TwoBetter],
    [q(), InitiativeDieQuality::OneBetter, InitiativeDieQuality::OneBetter, InitiativeDieQuality::TwoBetter, InitiativeDieQuality::TwoBetter],
    [q(), InitiativeDieQuality::OneBetter, InitiativeDieQuality::OneBetter, InitiativeDieQuality::TwoBetter, InitiativeDieQuality::TwoBetter],
    [q(), InitiativeDieQuality::OneBetter, InitiativeDieQuality::OneBetter, InitiativeDieQuality::TwoBetter, InitiativeDieQuality::TwoBetter],
    [q(), InitiativeDieQuality::OneBetter, InitiativeDieQuality::TwoBetter, InitiativeDieQuality::TwoBetter, InitiativeDieQuality::ThreeBetter],
    [q(), InitiativeDieQuality::OneBetter, InitiativeDieQuality::TwoBetter, InitiativeDieQuality::TwoBetter, InitiativeDieQuality::ThreeBetter],
    [q(), InitiativeDieQuality::OneBetter, InitiativeDieQuality::TwoBetter, InitiativeDieQuality::TwoBetter, InitiativeDieQuality::ThreeBetter],
    [q(), InitiativeDieQuality::OneBetter, InitiativeDieQuality::TwoBetter, InitiativeDieQuality::ThreeBetter, InitiativeDieQuality::ThreeBetter],
    [q(), InitiativeDieQuality::OneBetter, InitiativeDieQuality::TwoBetter, InitiativeDieQuality::ThreeBetter, InitiativeDieQuality::ThreeBetter],
    [q(), InitiativeDieQuality::OneBetter, InitiativeDieQuality::TwoBetter, InitiativeDieQuality::ThreeBetter, InitiativeDieQuality::ThreeBetter],
    [q(), InitiativeDieQuality::OneBetter, InitiativeDieQuality::TwoBetter, InitiativeDieQuality::ThreeBetter, InitiativeDieQuality::ThreeBetter],
    [q(), InitiativeDieQuality::OneBetter, InitiativeDieQuality::TwoBetter, InitiativeDieQuality::ThreeBetter, InitiativeDieQuality::FourBetter],
    [q(), InitiativeDieQuality::OneBetter, InitiativeDieQuality::TwoBetter, InitiativeDieQuality::ThreeBetter, InitiativeDieQuality::FourBetter],
    [q(), InitiativeDieQuality::OneBetter, InitiativeDieQuality::TwoBetter, InitiativeDieQuality::ThreeBetter, InitiativeDieQuality::FourBetter],
];

const fn q() -> InitiativeDieQuality {
    InitiativeDieQuality::Standard
}

const HEALTH_TABLE: [[f32; 5]; 20] = [
    [0.8, 1.0, 1.1, 1.2, 1.3],
    [1.0, 1.1, 1.3, 1.4, 1.5],
    [1.1, 1.3, 1.4, 1.6, 1.8],
    [1.3, 1.4, 1.6, 1.8, 2.0],
    [1.4, 1.6, 1.8, 2.0, 2.2],
    [1.5, 1.8, 2.0, 2.2, 2.4],
    [1.7, 1.9, 2.2, 2.4, 2.6],
    [1.8, 2.1, 2.3, 2.6, 2.9],
    [2.0, 2.2, 2.5, 2.8, 3.1],
    [2.1, 2.4, 2.7, 3.0, 3.3],
    [2.2, 2.6, 2.9, 3.2, 3.5],
    [2.4, 2.7, 3.1, 3.4, 3.7],
    [2.5, 2.9, 3.2, 3.6, 4.0],
    [2.7, 3.0, 3.4, 3.8, 4.2],
    [2.8, 3.2, 3.6, 4.0, 4.4],
    [2.9, 3.4, 3.8, 4.2, 4.6],
    [3.1, 3.5, 4.0, 4.4, 4.8],
    [3.2, 3.7, 4.1, 4.6, 5.1],
    [3.4, 3.8, 4.3, 4.8, 5.3],
    [3.5, 4.0, 4.5, 5.0, 5.5],
];

// --- Armor and materials ---

pub const ARMOR: &[Armor] = &[
    Armor { name: "Robe", region: ArmorRegion::Northern, damage_reduction: 1, defense_adj: -1, initiative_mod: 0, speed_mod: 0, armor_type: ArmorType::None, weight_lbs: 5.0 },
    Armor { name: "Doublet", region: ArmorRegion::Northern, damage_reduction: 2, defense_adj: -3, initiative_mod: 1, speed_mod: 0, armor_type: ArmorType::Light, weight_lbs: 15.0 },
    Armor { name: "Gambeson", region: ArmorRegion::Northern, damage_reduction: 2, defense_adj: -2, initiative_mod: 0, speed_mod: 0, armor_type: ArmorType::Light, weight_lbs: 15.0 },
    Armor { name: "Chainshirt", region: ArmorRegion::Northern, damage_reduction: 3, defense_adj: -3, initiative_mod: 1, speed_mod: 0, armor_type: ArmorType::Medium, weight_lbs: 20.0 },
    Armor { name: "Ringmail", region: ArmorRegion::Northern, damage_reduction: 4, defense_adj: -4, initiative_mod: 1, speed_mod: 1, armor_type: ArmorType::Medium, weight_lbs: 30.0 },
    Armor { name: "Breastplate", region: ArmorRegion::Northern, damage_reduction: 5, defense_adj: -5, initiative_mod: 1, speed_mod: 2, armor_type: ArmorType::Medium, weight_lbs: 35.0 },
    Armor { name: "Scalemail", region: ArmorRegion::Northern, damage_reduction: 5, defense_adj: -6, initiative_mod: 3, speed_mod: 2, armor_type: ArmorType::Heavy, weight_lbs: 60.0 },
    Armor { name: "Chainmail", region: ArmorRegion::Northern, damage_reduction: 5, defense_adj: -5, initiative_mod: 2, speed_mod: 2, armor_type: ArmorType::Heavy, weight_lbs: 45.0 },
    Armor { name: "Splintmail", region: ArmorRegion::Northern, damage_reduction: 6, defense_adj: -5, initiative_mod: 2, speed_mod: 2, armor_type: ArmorType::Heavy, weight_lbs: 55.0 },
    Armor { name: "Brigandine", region: ArmorRegion::Northern, damage_reduction: 6, defense_adj: -4, initiative_mod: 2, speed_mod: 1, armor_type: ArmorType::Heavy, weight_lbs: 50.0 },
    Armor { name: "Halfplate", region: ArmorRegion::Northern, damage_reduction: 7, defense_adj: -5, initiative_mod: 2, speed_mod: 2, armor_type: ArmorType::Heavy, weight_lbs: 40.0 },
    Armor { name: "Platemail", region: ArmorRegion::Northern, damage_reduction: 8, defense_adj: -5, initiative_mod: 3, speed_mod: 2, armor_type: ArmorType::Heavy, weight_lbs: 55.0 },
    Armor { name: "Kaftan", region: ArmorRegion::Southern, damage_reduction: 1, defense_adj: 0, initiative_mod: 2, speed_mod: 0, armor_type: ArmorType::None, weight_lbs: 8.0 },
    Armor { name: "Quilted", region: ArmorRegion::Southern, damage_reduction: 1, defense_adj: 0, initiative_mod: 0, speed_mod: 0, armor_type: ArmorType::Light, weight_lbs: 10.0 },
    Armor { name: "Boiled", region: ArmorRegion::Southern, damage_reduction: 2, defense_adj: -2, initiative_mod: 1, speed_mod: 0, armor_type: ArmorType::Light, weight_lbs: 20.0 },
    Armor { name: "Rawhide", region: ArmorRegion::Southern, damage_reduction: 2, defense_adj: -1, initiative_mod: 1, speed_mod: 1, armor_type: ArmorType::Light, weight_lbs: 20.0 },
    Armor { name: "Studded", region: ArmorRegion::Southern, damage_reduction: 2, defense_adj: -1, initiative_mod: 0, speed_mod: 1, armor_type: ArmorType::Medium, weight_lbs: 25.0 },
    Armor { name: "Jazerant", region: ArmorRegion::Southern, damage_reduction: 3, defense_adj: -2, initiative_mod: 0, speed_mod: 2, armor_type: ArmorType::Medium, weight_lbs: 25.0 },
    Armor { name: "Kazhagand", region: ArmorRegion::Southern, damage_reduction: 4, defense_adj: -3, initiative_mod: 2, speed_mod: 2, armor_type: ArmorType::Medium, weight_lbs: 30.0 },
    Armor { name: "Hoopmail", region: ArmorRegion::Southern, damage_reduction: 5, defense_adj: -5, initiative_mod: 2, speed_mod: 2, armor_type: ArmorType::Medium, weight_lbs: 35.0 },
    Armor { name: "Lamellar", region: ArmorRegion::Southern, damage_reduction: 6, defense_adj: -4, initiative_mod: 1, speed_mod: 2, armor_type: ArmorType::Heavy, weight_lbs: 40.0 },
    Armor { name: "Mirror", region: ArmorRegion::Southern, damage_reduction: 7, defense_adj: -5, initiative_mod: 3, speed_mod: 1, armor_type: ArmorType::Heavy, weight_lbs: 45.0 },
];

pub const MATERIALS: &[Material] = &[
    Material { tier: 0, name: "Bronze", weight_mult: 1.2, kind: MaterialKind::Metal },
    Material { tier: 1, name: "Iron", weight_mult: 1.0, kind: MaterialKind::Metal },
    Material { tier: 2, name: "Steel", weight_mult: 1.0, kind: MaterialKind::Metal },
    Material { tier: 0, name: "Cotton", weight_mult: 1.0, kind: MaterialKind::Fabric },
    Material { tier: 1, name: "Wool", weight_mult: 1.5, kind: MaterialKind::Fabric },
    Material { tier: 2, name: "Linen", weight_mult: 1.5, kind: MaterialKind::Fabric },
    Material { tier: 0, name: "Oak", weight_mult: 1.0, kind: MaterialKind::Wood },
    Material { tier: 1, name: "Yew", weight_mult: 0.8, kind: MaterialKind::Wood },
    Material { tier: 2, name: "Ash", weight_mult: 0.8, kind: MaterialKind::Wood },
];

// --- Utility helpers ---

pub fn mastery_threshold(base_threshold: f32, intelligence: u8, completed_tiers: i32) -> f32 {
    let int_mod = lookup_int(intelligence).weapon_exp_threshold_pct as f32 / 100.0;
    let mut threshold = base_threshold * (1.0 + int_mod);
    if completed_tiers > 0 {
        threshold *= 1.0 + (0.2 * completed_tiers as f32);
    }
    threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strength_damage_bonus_matches_table() {
        let abilities = AbilitySet {
            strength: AbilityScore::new(15, 1),
            intelligence: 10,
            wisdom: 10,
            dexterity: AbilityScore::new(10, 1),
            constitution: 10,
            looks: 10,
            charisma: 10,
        };
        let derived = derive_abilities(&abilities);
        assert_eq!(derived.strength.damage, 2);
    }

    #[test]
    fn attack_bonus_includes_intelligence_and_dexterity() {
        let abilities = AbilitySet {
            strength: AbilityScore::new(10, 1),
            intelligence: 12,
            wisdom: 10,
            dexterity: AbilityScore::new(12, 1),
            constitution: 10,
            looks: 10,
            charisma: 10,
        };
        let character = Character::builder("Test")
            .level(
                5,
                Progression::new(
                    ProgressionTier::III,
                    ProgressionTier::III,
                    ProgressionTier::III,
                    ProgressionTier::III,
                ),
            )
            .abilities(abilities)
            .build();
        let derived = character.derived();
        assert_eq!(derived.attack_bonus, 4);
    }

    #[test]
    fn initiative_mod_includes_dex_and_wis() {
        let abilities = AbilitySet {
            strength: AbilityScore::new(10, 1),
            intelligence: 10,
            wisdom: 10,
            dexterity: AbilityScore::new(10, 1),
            constitution: 10,
            looks: 10,
            charisma: 10,
        };
        let character = Character::builder("Test")
            .level(
                1,
                Progression::new(
                    ProgressionTier::III,
                    ProgressionTier::III,
                    ProgressionTier::III,
                    ProgressionTier::III,
                ),
            )
            .abilities(abilities)
            .build();
        let derived = character.derived();
        assert_eq!(derived.initiative_mod, 4);
    }

    #[test]
    fn hit_points_use_base_and_constitution_with_multiplier() {
        let abilities = AbilitySet {
            strength: AbilityScore::new(10, 1),
            intelligence: 10,
            wisdom: 10,
            dexterity: AbilityScore::new(10, 1),
            constitution: 12,
            looks: 10,
            charisma: 10,
        };
        let character = Character::builder("Test")
            .level(
                1,
                Progression::new(
                    ProgressionTier::III,
                    ProgressionTier::III,
                    ProgressionTier::III,
                    ProgressionTier::III,
                ),
            )
            .base_hp(11)
            .abilities(abilities)
            .build();
        let derived = character.derived();
        assert_eq!(derived.hit_points, 24);
    }

    #[test]
    fn base_dv_includes_armor_adjustment() {
        let abilities = AbilitySet {
            strength: AbilityScore::new(10, 1),
            intelligence: 10,
            wisdom: 10,
            dexterity: AbilityScore::new(10, 1),
            constitution: 10,
            looks: 10,
            charisma: 10,
        };
        let armor = ARMOR
            .iter()
            .find(|armor| armor.name == "Chainmail" && armor.region == ArmorRegion::Northern)
            .cloned();
        let equipment = Equipment {
            armor,
            ..Default::default()
        };
        let character = Character::builder("Test")
            .level(
                1,
                Progression::new(
                    ProgressionTier::III,
                    ProgressionTier::III,
                    ProgressionTier::III,
                    ProgressionTier::III,
                ),
            )
            .abilities(abilities)
            .equipment(equipment)
            .build();
        let derived = character.derived();
        assert_eq!(derived.base_dv, -9);
    }

    #[test]
    fn speed_advancement_matches_table() {
        assert_eq!(speed_mod_for(1, ProgressionTier::I), 1);
        assert_eq!(speed_mod_for(4, ProgressionTier::VI), -1);
        assert_eq!(speed_mod_for(20, ProgressionTier::VI), -5);
    }

    #[test]
    fn initiative_advancement_matches_table() {
        assert_eq!(initiative_mod_for(1, ProgressionTier::I), 2);
        assert_eq!(initiative_mod_for(4, ProgressionTier::IV), -1);
        assert_eq!(initiative_mod_for(20, ProgressionTier::V), -4);
    }

    #[test]
    fn health_advancement_matches_table() {
        let low = health_mult_for(1, ProgressionTier::I);
        let mid = health_mult_for(10, ProgressionTier::III);
        let high = health_mult_for(20, ProgressionTier::V);
        assert!((low - 0.8).abs() < 1e-6);
        assert!((mid - 2.7).abs() < 1e-6);
        assert!((high - 5.5).abs() < 1e-6);
    }
}
