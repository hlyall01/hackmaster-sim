pub const CHRONOBLUR_EFFECT_ID: &str = "spell_chronoblur";
pub const CHRONOBLUR_DURATION_SECONDS: i32 = 60;
pub const CHRONOBLUR_MELEE_DEFENSE_BONUS: i32 = 4;
pub const CHRONOBLUR_RANGED_DISTANCE_FEET: f32 = 20.0;

pub const STREAMLINE_EFFECT_ID: &str = "spell_streamline";
pub const STREAMLINE_DURATION_SECONDS: i32 = 5 * 60;
pub const STREAMLINE_RADIUS_FEET: f32 = 30.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatIdI32 {
    AttackBonus,
    AttackBonusBase,
    StrengthDamage,
    StrengthDamageBase,
    UnarmedDamageBonus,
    UnarmedAttackBonus,
    ArmorPenetration,
    DefenseMod,
    AllDefenseBonus,
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
    OpeningEngagementExtraDamageDice,
    FlagFallingSunStyle,
    FlagFymblwngerStyle,
    FlagProjectConfidence,
    FlagIdealSelf,
    FlagIntimidateAdversary,
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
    FlagLeftHandOfEvoniaStyle,
    FlagOnePathPiercingStyle,
    FlagOnePathCrushingStyle,
    FlagNpcCombatant,
    FlagPilgrimsPathStyle,
    FlagDeclinePursuit,
    FlagReaperOfTermonStyle,
    FlagChronoblurSpell,
    FlagStreamlineSpell,
}

impl StatIdI32 {
    pub(super) const fn bit(self) -> u64 {
        1 << (self as u32)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatIdF32 {
    StreamlineRadius,
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
    // Most combat flags are queried repeatedly with a zero base.
    positive_at_zero: u64,
    index: Option<std::sync::Arc<ModifierIndex>>,
}

// Profiles share their prepared index until an equipment or effect builder edits it.
#[derive(Clone, Debug, Default)]
struct ModifierIndex {
    integers: Vec<Option<IntegerTerms>>,
    floats: Vec<Option<FloatTerms>>,
    // Replaying the source operations on extend preserves insertion order and
    // floating-point rounding, including when extending a non-empty stack.
    integer_ops: Vec<ModifierI32>,
    float_ops: Vec<ModifierF32>,
}

#[derive(Clone, Copy, Debug)]
struct IntegerTerms {
    add: i32,
    set: Option<i32>,
    upper: i32,
    lower: i32,
}

impl Default for IntegerTerms {
    fn default() -> Self {
        Self { add: 0, set: None, upper: i32::MAX, lower: i32::MIN }
    }
}

#[derive(Clone, Copy, Debug)]
struct FloatTerms {
    add: f32,
    multiply: f32,
    set: Option<f32>,
    upper: Option<f32>,
    lower: Option<f32>,
}

impl Default for FloatTerms {
    fn default() -> Self {
        Self { add: 0.0, multiply: 1.0, set: None, upper: None, lower: None }
    }
}

impl ModifierStack {
    pub fn is_empty(&self) -> bool {
        self.index.is_none()
    }

    pub(super) fn integer_stats(&self) -> impl Iterator<Item = StatIdI32> + '_ {
        self.index.iter().flat_map(|index| index.integer_ops.iter().map(|op| op.stat))
    }

    pub(super) fn positive_bits(&self) -> u64 {
        self.positive_at_zero
    }

    fn make_mut(&mut self) -> &mut ModifierIndex {
        std::sync::Arc::make_mut(self.index.get_or_insert_with(Default::default))
    }

    pub fn add_i32(&mut self, stat: StatIdI32, op: ModifierOpI32) {
        let index = self.make_mut();
        index.integer_ops.push(ModifierI32 { stat, op });
        let key = stat as usize;
        if index.integers.len() <= key {
            index.integers.resize(key + 1, None);
        }
        let terms = index.integers[key].get_or_insert_with(IntegerTerms::default);
        match op {
            ModifierOpI32::Set(x) => terms.set = Some(x),
            ModifierOpI32::Add(x) => terms.add += x,
            ModifierOpI32::Min(x) => terms.upper = terms.upper.min(x),
            ModifierOpI32::Max(x) => terms.lower = terms.lower.max(x),
        }
        if let Some(bit) = 1u64.checked_shl(stat as u32) {
            let positive = terms.set.unwrap_or(0).wrapping_add(terms.add)
                .min(terms.upper).max(terms.lower) > 0;
            if positive { self.positive_at_zero |= bit; } else { self.positive_at_zero &= !bit; }
        }
    }

    pub fn add_f32(&mut self, stat: StatIdF32, op: ModifierOpF32) {
        let index = self.make_mut();
        index.float_ops.push(ModifierF32 { stat, op });
        let key = stat as usize;
        if index.floats.len() <= key {
            index.floats.resize(key + 1, None);
        }
        let terms = index.floats[key].get_or_insert_with(FloatTerms::default);
        match op {
            ModifierOpF32::Set(x) => terms.set = Some(x),
            ModifierOpF32::Add(x) => terms.add += x,
            ModifierOpF32::Mul(x) => terms.multiply *= x,
            ModifierOpF32::Min(x) => terms.upper = Some(terms.upper.map_or(x, |v| v.min(x))),
            ModifierOpF32::Max(x) => terms.lower = Some(terms.lower.map_or(x, |v| v.max(x))),
        }
    }

    pub fn extend(&mut self, other: &ModifierStack) {
        let Some(index) = &other.index else { return };
        for modifier in &index.integer_ops {
            self.add_i32(modifier.stat, modifier.op);
        }
        for modifier in &index.float_ops {
            self.add_f32(modifier.stat, modifier.op);
        }
    }

    #[inline]
    pub(super) fn positive_at_zero(&self, stat: StatIdI32) -> bool {
        match 1u64.checked_shl(stat as u32) {
            Some(bit) => self.positive_at_zero & bit != 0,
            None => self.apply_i32(0, stat) > 0,
        }
    }

    pub fn apply_i32(&self, value: i32, stat: StatIdI32) -> i32 {
        let Some(terms) = self.index.as_ref()
            .and_then(|index| index.integers.get(stat as usize)).and_then(Option::as_ref)
        else { return value };
        (terms.set.unwrap_or(value) + terms.add).min(terms.upper).max(terms.lower)
    }

    pub fn apply_f32(&self, value: f32, stat: StatIdF32) -> f32 {
        let Some(terms) = self.index.as_ref()
            .and_then(|index| index.floats.get(stat as usize)).and_then(Option::as_ref)
        else { return value + 0.0 };
        let mut result = (terms.set.unwrap_or(value) + terms.add) * terms.multiply;
        if let Some(upper) = terms.upper {
            result = result.min(upper);
        }
        if let Some(lower) = terms.lower {
            result = result.max(lower);
        }
        result
    }
}

/// Hooks and integer modifiers contributed by the *current* temporary effects.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct EffectCapabilities {
    pub integers: u64,
    pub warrior_woad: bool,
    pub average_damage: bool,
}

impl EffectCapabilities {
    fn include(&mut self, effect: &TemporaryEffect) {
        if let Some(index) = &effect.modifiers.index {
            for op in &index.integer_ops {
                self.integers |= op.stat.bit();
            }
        }
        self.warrior_woad |= effect.id == super::arborite::WOAD_EFFECT_ID;
        self.average_damage |= effect.average_damage_radius.is_some();
    }
}

#[derive(Clone, Debug, Default)]
pub struct ActiveEffects {
    entries: Vec<TemporaryEffect>,
    capabilities: EffectCapabilities,
    dirty: bool,
}

impl ActiveEffects {
    #[inline(never)]
    fn scan(&self) -> EffectCapabilities {
        let mut summary = EffectCapabilities::default();
        for effect in &self.entries { summary.include(effect); }
        summary
    }

    #[inline]
    pub(super) fn capabilities(&self) -> EffectCapabilities {
        if self.dirty { self.scan() } else { self.capabilities }
    }

    pub fn push(&mut self, effect: TemporaryEffect) {
        if self.dirty {
            self.capabilities = self.scan();
            self.dirty = false;
        }
        self.capabilities.include(&effect);
        self.entries.push(effect);
    }

    pub fn retain(&mut self, keep: impl FnMut(&TemporaryEffect) -> bool) {
        let old_len = self.entries.len();
        self.entries.retain(keep);
        if self.dirty || self.entries.len() != old_len {
            self.capabilities = self.scan();
            self.dirty = false;
        }
    }

    pub(super) fn tick(&mut self) {
        for effect in &mut self.entries { effect.remaining_seconds -= 1; }
        self.retain(|effect| effect.remaining_seconds > 0);
    }
}

impl<'a> IntoIterator for &'a ActiveEffects {
    type Item = &'a TemporaryEffect;
    type IntoIter = std::slice::Iter<'a, TemporaryEffect>;
    fn into_iter(self) -> Self::IntoIter { self.entries.iter() }
}

impl<'a> IntoIterator for &'a mut ActiveEffects {
    type Item = &'a mut TemporaryEffect;
    type IntoIter = std::slice::IterMut<'a, TemporaryEffect>;
    fn into_iter(self) -> Self::IntoIter {
        self.dirty = true;
        self.entries.iter_mut()
    }
}

impl std::ops::Deref for ActiveEffects {
    type Target = Vec<TemporaryEffect>;
    fn deref(&self) -> &Self::Target { &self.entries }
}

impl std::ops::DerefMut for ActiveEffects {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Direct edits can replace an entire effect, its ID, or its modifiers.
        self.dirty = true;
        &mut self.entries
    }
}

#[derive(Clone, Debug)]
pub struct TemporaryEffect {
    pub id: String,
    pub remaining_seconds: i32,
    pub consume_on_attack: bool,
    pub strength_override: Option<(i32, i32)>,
    pub strength_score_override: Option<crate::character::AbilityScore>,
    pub strength_increase: u8,
    pub movement_defense: Option<crate::core::magic::MovementDefenseRule>,
    pub average_damage_radius: Option<f32>,
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
        let id = id.into();
        let rule = crate::core::magic::spell_catalog().iter().find(|s| s.id == id).and_then(|s| s.mechanics.as_ref());
        let (movement_defense, average_damage_radius) = match rule {
            Some(crate::core::magic::EffectDefinition { values, effect: crate::core::magic::EffectRule::TimedBuff { movement_defense, average_incoming_damage }, .. }) => (movement_defense.clone(), average_incoming_damage.then_some(values.radius)),
            _ => (None, None),
        };
        Self {
            id,
            remaining_seconds: duration_seconds.max(0),
            consume_on_attack: false,
            strength_override: None,
            strength_score_override: None,
            strength_increase: 0,
            movement_defense,
            average_damage_radius,
            modifiers: ModifierStack::default(),
        }
    }
}
