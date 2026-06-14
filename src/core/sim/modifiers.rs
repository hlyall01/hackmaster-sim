#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatIdI32 {
    AttackBonus,
    AttackBonusBase,
    StrengthDamage,
    StrengthDamageBase,
    UnarmedDamageBonus,
    ArmorPenetration,
    DefenseMod,
    RangedDefenseMod,
    ArmorDr,
    NaturalDr,
    ShieldDefenseBonus,
    ShieldDr,
    ShieldCoverValue,
    CritMinRoll,
    CritSeverityBonus,
    IncomingCritSeverityReduction,
    KnockbackStep,
    FlagDefiant,
    FlagSuperiorDefense,
    FlagEdgeCounter,
    FlagIncomingCritExtraDamageHalved,
    FlagIgnoreAncillaryCritEffects,
    FlagLargeSwordShieldStyle,
    FlagArmerociPoleStyle,
    FlagFallingSunStyle,
    FlagFymblwngerStyle,
    FlagHammererStyle,
    FlagHobblerStyle,
    FlagIthicanPrinceStyle,
    FlagQuietRiverStyle,
    FlagRegenstatStyle,
    FlagReturnerStyle,
    FlagRhdwngFlowStyle,
    FlagSixPathsStyle,
    FlagThreeMountainsStyle,
    FlagUnbreakableWallStyle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatIdF32 {
    WeaponSpeed,
    WeaponReach,
    MoveSpeed,
    RangeDistanceMultiplier,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModifierOpI32 {
    Add(i32),
    Set(i32),
    Min(i32),
    Max(i32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ModifierOpF32 {
    Add(f32),
    Mul(f32),
    Set(f32),
    Min(f32),
    Max(f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModifierI32 {
    pub stat: StatIdI32,
    pub op: ModifierOpI32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModifierF32 {
    pub stat: StatIdF32,
    pub op: ModifierOpF32,
}

#[derive(Clone, Debug, Default)]
pub struct ModifierStack {
    mods_i32: Vec<ModifierI32>,
    mods_f32: Vec<ModifierF32>,
}

impl ModifierStack {
    pub fn is_empty(&self) -> bool {
        self.mods_i32.is_empty() && self.mods_f32.is_empty()
    }

    pub fn add_i32(&mut self, stat: StatIdI32, op: ModifierOpI32) {
        self.mods_i32.push(ModifierI32 { stat, op });
    }

    pub fn add_f32(&mut self, stat: StatIdF32, op: ModifierOpF32) {
        self.mods_f32.push(ModifierF32 { stat, op });
    }

    pub fn extend(&mut self, other: &ModifierStack) {
        self.mods_i32.extend(other.mods_i32.iter().copied());
        self.mods_f32.extend(other.mods_f32.iter().copied());
    }

    pub fn apply_i32(&self, mut value: i32, stat: StatIdI32) -> i32 {
        let mut add_total = 0;
        let mut set_value: Option<i32> = None;
        let mut min_value: Option<i32> = None;
        let mut max_value: Option<i32> = None;
        for modifier in &self.mods_i32 {
            if modifier.stat != stat {
                continue;
            }
            match modifier.op {
                ModifierOpI32::Add(amount) => add_total += amount,
                ModifierOpI32::Set(amount) => set_value = Some(amount),
                ModifierOpI32::Min(amount) => {
                    min_value = Some(min_value.map_or(amount, |current| current.min(amount)));
                }
                ModifierOpI32::Max(amount) => {
                    max_value = Some(max_value.map_or(amount, |current| current.max(amount)));
                }
            }
        }
        if let Some(set_value) = set_value {
            value = set_value;
        }
        value += add_total;
        if let Some(min_value) = min_value {
            value = value.min(min_value);
        }
        if let Some(max_value) = max_value {
            value = value.max(max_value);
        }
        value
    }

    pub fn apply_f32(&self, mut value: f32, stat: StatIdF32) -> f32 {
        let mut add_total = 0.0;
        let mut mul_total = 1.0;
        let mut set_value: Option<f32> = None;
        let mut min_value: Option<f32> = None;
        let mut max_value: Option<f32> = None;
        for modifier in &self.mods_f32 {
            if modifier.stat != stat {
                continue;
            }
            match modifier.op {
                ModifierOpF32::Add(amount) => add_total += amount,
                ModifierOpF32::Mul(amount) => mul_total *= amount,
                ModifierOpF32::Set(amount) => set_value = Some(amount),
                ModifierOpF32::Min(amount) => {
                    min_value = Some(min_value.map_or(amount, |current| current.min(amount)));
                }
                ModifierOpF32::Max(amount) => {
                    max_value = Some(max_value.map_or(amount, |current| current.max(amount)));
                }
            }
        }
        if let Some(set_value) = set_value {
            value = set_value;
        }
        value += add_total;
        value *= mul_total;
        if let Some(min_value) = min_value {
            value = value.min(min_value);
        }
        if let Some(max_value) = max_value {
            value = value.max(max_value);
        }
        value
    }
}

#[derive(Clone, Debug)]
pub struct TemporaryEffect {
    pub id: String,
    pub remaining_seconds: i32,
    pub modifiers: ModifierStack,
}

pub fn modifiers_for_magic_item(tag: &str) -> ModifierStack {
    let mut stack = ModifierStack::default();
    match tag.trim().to_ascii_lowercase().as_str() {
        "keen" => {
            stack.add_i32(StatIdI32::CritMinRoll, ModifierOpI32::Min(19));
        }
        "fortified" => {
            stack.add_i32(StatIdI32::ArmorDr, ModifierOpI32::Add(1));
        }
        "quick" => {
            stack.add_f32(StatIdF32::WeaponSpeed, ModifierOpF32::Add(-1.0));
        }
        "surefooted" => {
            stack.add_f32(StatIdF32::MoveSpeed, ModifierOpF32::Add(2.0));
        }
        _ => {}
    }
    stack
}

impl TemporaryEffect {
    pub fn new(id: impl Into<String>, duration_seconds: i32) -> Self {
        Self {
            id: id.into(),
            remaining_seconds: duration_seconds.max(0),
            modifiers: ModifierStack::default(),
        }
    }
}
