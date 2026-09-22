//! Magic rules shared by character configuration and combat.
//!
//! Essence-specific generation and volatility are deliberately not inferred here:
//! those require the rules for the caster's essence.

use rand::Rng;
use serde::{Deserialize, Serialize};

mod catalog;
pub use catalog::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EssenceProficiency {
    I,
    II,
    III,
    IV,
    V,
}

impl EssenceProficiency {
    pub fn maximum_spell_level(self, character_level: u8) -> u8 {
        match self {
            Self::I => 1,
            Self::II => character_level.div_ceil(2),
            Self::III | Self::IV | Self::V => character_level,
        }
    }

    pub fn validate_cost(self, base: u32, total: u32) -> Result<Amplification, MagicError> {
        if base == 0 || total < base {
            return Err(MagicError::InvalidCost);
        }
        let base = u64::from(base);
        let total = u64::from(total);
        if total > base * 3 {
            return Err(MagicError::AmplificationLimit);
        }
        if total == base {
            return Ok(Amplification::None);
        }
        match self {
            Self::V if total > base * 2 => Ok(Amplification::Overamplified),
            Self::IV | Self::V if total <= base * 2 => Ok(Amplification::Amplified),
            _ => Err(MagicError::InsufficientProficiency),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Amplification {
    None,
    Amplified,
    Overamplified,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MagicError {
    InvalidCost,
    AmplificationLimit,
    InsufficientProficiency,
    SpellLevelTooHigh,
    InvalidEchoDelay,
    CostOverflow,
    TooManyEssences,
    DuplicateEssence,
    InvalidEssence,
    EssenceExceedsCapacity,
    SomaticComponentBlocked,
    VerbalComponentBlocked,
    UnknownSpell,
    InsufficientEssence,
    Busy,
    Incapacitated,
    NoActiveCast,
    InstantCastCannotCancel,
    NoActiveSpell,
    InvalidSpell,
    InstantCastLimit,
}

impl std::fmt::Display for MagicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidCost => "Invalid essence cost",
            Self::AmplificationLimit => "Total cost exceeds 300% of base cost",
            Self::InsufficientProficiency => {
                "Essence proficiency is too low for this amplification"
            }
            Self::SpellLevelTooHigh => {
                "Essence proficiency or character level is too low for this spell"
            }
            Self::InvalidEchoDelay => "Echo delay must be 1–10 seconds",
            Self::CostOverflow => "Spell settings exceed supported limits",
            Self::TooManyEssences => "A character can have at most two essences",
            Self::DuplicateEssence => "Each essence must be distinct",
            Self::InvalidEssence => "Configure an essence for this spell",
            Self::EssenceExceedsCapacity => "Starting essence exceeds capacity",
            Self::SomaticComponentBlocked => "Somatic gestures are blocked",
            Self::VerbalComponentBlocked => "The verbal component is blocked",
            Self::UnknownSpell => "This character does not know this spell",
            Self::InsufficientEssence => "Not enough essence",
            Self::Busy => "Already casting, channeling, or recovering from spell fatigue",
            Self::Incapacitated => "Cannot cast while dead, incapacitated, or passive",
            Self::NoActiveCast => "No spell is being cast",
            Self::InstantCastCannotCancel => "An instant spell cannot be cancelled",
            Self::NoActiveSpell => "No active spell to dismiss",
            Self::InvalidSpell => "Invalid spell definition or effect settings",
            Self::InstantCastLimit => "Only one zero-second spell may be cast per second",
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Encumbrance {
    #[default]
    None,
    Light,
    Moderate,
    Heavy,
}

impl Encumbrance {
    pub fn failure_percent(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Light => 25,
            Self::Moderate => 50,
            Self::Heavy => 75,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoCast {
    #[default]
    SpellPolicies,
    Manual,
    AtStart,
    InWeaponReach,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpellCastAi {
    AsOftenAsPossible,
    #[serde(alias = "when_useful")]
    AtFightStart,
    Manual,
}
impl SpellCastAi {
    pub const ALL: [Self; 3] = [Self::AtFightStart, Self::Manual, Self::AsOftenAsPossible];

    pub fn label(self) -> &'static str {
        match self {
            Self::AtFightStart => "Use once",
            Self::AsOftenAsPossible => "Use as much as possible",
            Self::Manual => "Manual",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MagicLoadout {
    /// Spell IDs selected from the catalog.
    pub known_spells: Vec<String>,
    pub spell_ai: std::collections::BTreeMap<String, SpellCastAi>,
    /// Optional rules-engine compatibility; the character editor ignores EP.
    pub use_essence_costs: bool,
    pub essences: Vec<EssencePool>,
    pub knows_echo_strike: bool,
    pub echo_essence: usize,
    pub echo_strike: EchoStrikeOptions,
    /// Each rank adds 30 seconds.
    pub chronoblur_duration_ranks: u32,
    /// Each rank adds one minute / ten feet, respectively.
    pub streamline_duration_ranks: u32,
    pub streamline_radius_ranks: u32,
    /// Automatic casting attempts once per fight; failed casts do not loop.
    pub auto_cast: AutoCast,
    pub encumbrance: Encumbrance,
    pub arms_restricted: bool,
    pub silenced: bool,
    pub echo_requires_living_caster: bool,
    pub echo_requires_weapon_range: bool,
}

impl MagicLoadout {
    pub fn streamline_radius_feet(&self) -> u64 {
        30 + 10 * u64::from(self.streamline_radius_ranks)
    }

    pub fn ai_for(&self, id: &str) -> SpellCastAi {
        self.spell_ai
            .get(id)
            .copied()
            .unwrap_or(SpellCastAi::AtFightStart)
    }

    pub fn knows_spell(&self, id: &str) -> bool {
        (id == "echo_strike" && self.knows_echo_strike)
            || self.known_spells.iter().any(|known| known == id)
    }

    pub fn learn_spell(&mut self, id: &str) {
        if !self.known_spells.iter().any(|known| known == id) {
            self.known_spells.push(id.to_owned());
        }
        self.auto_cast = AutoCast::SpellPolicies;
        self.use_essence_costs = false;
    }

    pub fn forget_spell(&mut self, id: &str) {
        self.known_spells.retain(|known| known != id);
        self.spell_ai.remove(id);
        if id == "echo_strike" {
            self.knows_echo_strike = false;
        }
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn validate_echo(&self, level: u8) -> Result<u32, MagicError> {
        if !self.knows_spell("echo_strike") {
            return Err(MagicError::UnknownSpell);
        }
        if !self.use_essence_costs {
            self.echo_strike.cost()?;
            self.echo_strike.duration_seconds()?;
            return Ok(0);
        }
        validate_essences(&self.essences)?;
        let essence = self
            .essences
            .get(self.echo_essence)
            .ok_or(MagicError::InvalidEssence)?;
        self.echo_strike.validate(level, essence.proficiency)?;
        let cost = self.echo_strike.cost()?;
        if essence.current < cost {
            return Err(MagicError::InsufficientEssence);
        }
        Ok(cost)
    }
}

/// Equipment adds one 25-point penalty when either heavy armor or a shield is
/// used. Combat Casting subtracts 25 percentage points before the final clamp.
pub fn spell_failure_percent(
    encumbrance: Encumbrance,
    heavy_armor: bool,
    shield: bool,
    talents: MagicTalents,
) -> u8 {
    let chance = encumbrance.failure_percent() + if heavy_armor || shield { 25 } else { 0 };
    chance
        .saturating_sub(if talents.combat_casting { 25 } else { 0 })
        .min(100)
}

/// Ability scores and advancement are explicit inputs until essence definitions
/// supply their associated abilities and advancement tables.
pub fn essence_capacity(highest_relevant_ability: u8, advancement: u32) -> Result<u32, MagicError> {
    let product = u64::from(highest_relevant_ability) * u64::from(advancement);
    u32::try_from(((product + 5) / 10) * 10).map_err(|_| MagicError::CostOverflow)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EssencePool {
    pub essence_id: String,
    pub proficiency: EssenceProficiency,
    pub capacity: u32,
    pub current: u32,
}

pub fn validate_essences(pools: &[EssencePool]) -> Result<(), MagicError> {
    if pools.len() > 2 {
        return Err(MagicError::TooManyEssences);
    }
    for (index, pool) in pools.iter().enumerate() {
        if pool.essence_id.trim().is_empty() {
            return Err(MagicError::InvalidEssence);
        }
        if pool.current > pool.capacity {
            return Err(MagicError::EssenceExceedsCapacity);
        }
        if pools[..index]
            .iter()
            .any(|other| other.essence_id == pool.essence_id)
        {
            return Err(MagicError::DuplicateEssence);
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EchoStrikeOptions {
    pub extra_duration_seconds: u32,
    pub delay_seconds: u32,
    pub additional_echoes: u32,
    pub full_damage: bool,
}

impl Default for EchoStrikeOptions {
    fn default() -> Self {
        Self {
            extra_duration_seconds: 0,
            delay_seconds: 10,
            additional_echoes: 0,
            full_damage: false,
        }
    }
}

impl EchoStrikeOptions {
    pub fn cost(self) -> Result<u32, MagicError> {
        if !(1..=10).contains(&self.delay_seconds) {
            return Err(MagicError::InvalidEchoDelay);
        }
        let total = u64::from(SpellKind::EchoStrike.catalog_entry().base_cost)
            + 5 * u64::from(self.extra_duration_seconds)
            + 20 * u64::from(10 - self.delay_seconds)
            + 150 * u64::from(self.additional_echoes)
            + if self.full_damage { 200 } else { 0 };
        u32::try_from(total).map_err(|_| MagicError::CostOverflow)
    }

    pub fn validate(
        self,
        level: u8,
        proficiency: EssenceProficiency,
    ) -> Result<Amplification, MagicError> {
        let spell = SpellKind::EchoStrike.catalog_entry();
        if proficiency.maximum_spell_level(level) < spell.level {
            return Err(MagicError::SpellLevelTooHigh);
        }
        proficiency.validate_cost(spell.base_cost, self.cost()?)
    }

    pub fn duration_seconds(self) -> Result<u32, MagicError> {
        15_u32
            .checked_add(self.extra_duration_seconds)
            .ok_or(MagicError::CostOverflow)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MagicTalent {
    CharmResistant,
    CombatCasting,
    DiminishSpellFatigue,
    MitigateSpellFatigue,
    DecimateSpellFatigue,
    EliminateSpellFatigue,
    IllusionResistant,
    MagicFocus,
    MagicSpecialist,
    MagicMaster,
    SilentCasting,
    StillCasting,
    SleepResistant,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MagicTalents {
    pub charm_resistant: bool,
    pub combat_casting: bool,
    pub diminish_spell_fatigue: u8,
    pub mitigate_spell_fatigue: bool,
    pub decimate_spell_fatigue: bool,
    pub eliminate_spell_fatigue: bool,
    pub illusion_resistant: bool,
    pub focus_tier: u8,
    pub silent_casting: bool,
    pub still_casting: bool,
    pub sleep_resistant: bool,
}

impl MagicTalents {
    pub fn apply(&mut self, talent: MagicTalent, rank: u8) {
        if rank == 0 {
            return;
        }
        match talent {
            MagicTalent::CharmResistant => self.charm_resistant = true,
            MagicTalent::CombatCasting => self.combat_casting = true,
            MagicTalent::DiminishSpellFatigue => {
                self.diminish_spell_fatigue = self.diminish_spell_fatigue.max(rank.min(5))
            }
            MagicTalent::MitigateSpellFatigue => self.mitigate_spell_fatigue = true,
            MagicTalent::DecimateSpellFatigue => self.decimate_spell_fatigue = true,
            MagicTalent::EliminateSpellFatigue => self.eliminate_spell_fatigue = true,
            MagicTalent::IllusionResistant => self.illusion_resistant = true,
            MagicTalent::MagicFocus => self.focus_tier = self.focus_tier.max(1),
            MagicTalent::MagicSpecialist => self.focus_tier = self.focus_tier.max(2),
            MagicTalent::MagicMaster => self.focus_tier = self.focus_tier.max(3),
            MagicTalent::SilentCasting => self.silent_casting = true,
            MagicTalent::StillCasting => self.still_casting = true,
            MagicTalent::SleepResistant => self.sleep_resistant = true,
        }
    }

    pub fn focus_bonus(self, associated_ability: u8) -> i32 {
        match self.focus_tier {
            0 => 0,
            1 => i32::from(associated_ability / 6),
            2 => i32::from(associated_ability / 5),
            _ => i32::from(associated_ability / 4),
        }
    }

    pub fn fatigue_seconds(self, casting_seconds: u32) -> u32 {
        if self.eliminate_spell_fatigue {
            0
        } else {
            casting_seconds
                .saturating_add(5)
                .saturating_sub(u32::from(self.diminish_spell_fatigue.min(5)))
        }
    }

    pub fn fatigue_penalties(self) -> FatiguePenalties {
        let decimated = self.decimate_spell_fatigue || self.eliminate_spell_fatigue;
        let mitigated = self.mitigate_spell_fatigue || decimated;
        FatiguePenalties {
            defense: if decimated {
                0
            } else if mitigated {
                -3
            } else {
                -6
            },
            movement_multiplier: if mitigated { 1.0 } else { 0.5 },
            can_run_or_sprint: decimated,
            can_attack: decimated,
            can_cast: self.eliminate_spell_fatigue,
            skill_penalty_percent: if decimated { 0 } else { 30 },
            action_time_multiplier: if decimated { 1 } else { 2 },
            reset_weapon_at_completion: decimated,
        }
    }

    pub fn resistance_bonus(self, tags: SpellTags) -> i32 {
        i32::from(self.charm_resistant && tags.charm) * 12
            + i32::from(self.illusion_resistant && tags.illusion) * 6
            + i32::from(self.sleep_resistant && tags.sleep) * 12
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FatiguePenalties {
    pub defense: i32,
    pub movement_multiplier: f32,
    pub can_run_or_sprint: bool,
    pub can_attack: bool,
    pub can_cast: bool,
    pub skill_penalty_percent: u8,
    pub action_time_multiplier: u8,
    pub reset_weapon_at_completion: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellTags {
    pub charm: bool,
    pub illusion: bool,
    pub sleep: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellComponents {
    pub somatic: bool,
    pub verbal: bool,
}

impl SpellComponents {
    /// Somatic gestures are checked throughout a cast; speech only at completion.
    pub fn validate(
        self,
        talents: MagicTalents,
        arms_free: bool,
        can_speak: bool,
        completing: bool,
    ) -> Result<(), MagicError> {
        if self.somatic && !talents.still_casting && !arms_free {
            return Err(MagicError::SomaticComponentBlocked);
        }
        if completing && self.verbal && !talents.silent_casting && !can_speak {
            return Err(MagicError::VerbalComponentBlocked);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MagicSaveKind {
    Physical,
    Mental,
    Dodge,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MagicSaveBonuses {
    pub physical: i32,
    pub mental: i32,
    pub dodge: i32,
}

impl MagicSaveBonuses {
    pub fn for_kind(self, kind: MagicSaveKind) -> i32 {
        match kind {
            MagicSaveKind::Physical => self.physical,
            MagicSaveKind::Mental => self.mental,
            MagicSaveKind::Dodge => self.dodge,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MagicSaveRoll {
    pub caster_first_die: i32,
    pub caster_total: i32,
    pub target_first_die: i32,
    pub target_total: i32,
    pub saved: bool,
}

impl MagicSaveRoll {
    pub fn from_rolls(
        caster_first_die: i32,
        caster_total: i32,
        target_first_die: i32,
        target_total: i32,
    ) -> Self {
        let saved =
            (target_first_die != 1 || caster_first_die == 1) && target_total >= caster_total;
        Self {
            caster_first_die,
            caster_total,
            target_first_die,
            target_total,
            saved,
        }
    }
}

pub fn roll_magic_save(caster_bonus: i32, target_bonus: i32, rng: &mut impl Rng) -> MagicSaveRoll {
    fn roll(rng: &mut impl Rng) -> (i32, i32) {
        let first = rng.gen_range(1..=20);
        let mut total = first;
        if first == 20 {
            loop {
                let next = rng.gen_range(1..=20);
                total += next - 1;
                if next != 20 {
                    break;
                }
            }
        }
        (first, total)
    }
    let (caster_first, caster_roll) = roll(rng);
    let (target_first, target_roll) = roll(rng);
    MagicSaveRoll::from_rolls(
        caster_first,
        caster_roll + caster_bonus,
        target_first,
        target_roll + target_bonus,
    )
}

#[cfg(test)]
mod tests;
