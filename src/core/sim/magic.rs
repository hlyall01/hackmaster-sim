//! Casting transitions and delayed spell effects, shared by both combat hosts.

use super::TemporaryEffect;
use super::types::{AttackRollBreakdown, Combatant, WeaponSlot};
use crate::core::magic::{
    AutoCast, EchoStrikeOptions, EssencePool, MagicError, MagicLoadout, MagicSaveBonuses,
    MagicTalents, SpellCatalogEntry, SpellComponents, SpellKind, spell_catalog,
    spell_failure_percent,
};
use rand::Rng;

#[derive(Clone, Debug, Default)]
pub struct MagicProfile {
    pub level: u8,
    pub loadout: MagicLoadout,
    pub talents: MagicTalents,
    pub saves: MagicSaveBonuses,
    pub strength: crate::character::AbilityScore,
    pub strength_damage: i32,
    pub primary_ignores_strength: bool,
    pub secondary_ignores_strength: bool,
    pub fist_attack: i32,
    pub fist_defense: i32,
    pub fist_damage: i32,
    pub fist_speed: f32,
}

#[derive(Clone, Debug)]
pub struct CastingSpell {
    pub started_at: u32,
    pub completes_at: u32,
    pub cost: u32,
    pub essence_index: usize,
    pub definition: SpellDefinition,
    pub effect: SpellEffect,
}

/// Definitions are supplied by the spell catalog, never synthesized from UI
/// text. Echo Strike is the initial catalog entry; the same lifecycle accepts
/// other timed buffs and channelled effects.
#[derive(Clone, Debug)]
pub struct SpellDefinition {
    pub id: String,
    pub name: String,
    pub level: u8,
    pub base_cost: u32,
    pub casting_seconds: u32,
    pub components: SpellComponents,
    pub channelled: bool,
}

impl From<&SpellCatalogEntry> for SpellDefinition {
    fn from(spell: &SpellCatalogEntry) -> Self {
        Self {
            id: spell.id.clone(),
            name: spell.name.clone(),
            level: spell.level,
            base_cost: spell.base_cost,
            casting_seconds: spell.casting_seconds,
            components: spell.components,
            channelled: false,
        }
    }
}

#[derive(Clone, Debug)]
pub enum SpellEffect {
    EchoStrike(EchoStrikeOptions),
    TimedBuff(TemporaryEffect),
    Configured(super::ConfiguredEffect),
}

#[derive(Clone, Debug)]
pub struct SpellRequest {
    pub definition: SpellDefinition,
    pub effect: SpellEffect,
    pub total_cost: u32,
    pub essence_index: usize,
}

impl SpellRequest {
    pub fn from_loadout(id: &str, loadout: &MagicLoadout) -> Result<Self, MagicError> {
        let spell = spell_catalog()
            .iter()
            .find(|spell| spell.id == id)
            .ok_or(MagicError::UnknownSpell)?;
        Self::from_catalog(spell, loadout)
    }

    /// Compile a validated catalog entry; useful for custom catalogs and tests.
    pub fn from_catalog(
        spell: &SpellCatalogEntry,
        loadout: &MagicLoadout,
    ) -> Result<Self, MagicError> {
        if spell.mechanics.is_some() {
            let (effect, total_cost) = super::ConfiguredEffect::from_catalog(spell, loadout)?;
            let mut definition: SpellDefinition = spell.into();
            definition.channelled = effect.channelled();
            return Ok(Self {
                definition,
                effect: SpellEffect::Configured(effect),
                total_cost,
                essence_index: 0,
            });
        }
        if spell.kind == SpellKind::EchoStrike {
            return Self::echo_strike(loadout.echo_strike, loadout.echo_essence);
        }
        Err(MagicError::InvalidSpell)
    }

    pub fn echo_strike(
        options: EchoStrikeOptions,
        essence_index: usize,
    ) -> Result<Self, MagicError> {
        Ok(Self {
            definition: SpellKind::EchoStrike.catalog_entry().into(),
            effect: SpellEffect::EchoStrike(options),
            total_cost: options.cost()?,
            essence_index,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ArmedEcho {
    pub expires_at: u32,
    pub options: EchoStrikeOptions,
    pub cast_id: u64,
}

#[derive(Clone, Debug)]
pub struct ScheduledEcho {
    pub due_at: u32,
    pub target: usize,
    pub damage: i32,
    pub ordinal: u32,
    pub cast_id: u64,
    pub weapon_slot: WeaponSlot,
    pub is_ranged: bool,
    pub original_roll: AttackRollBreakdown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpellEventKind {
    CastStarted,
    CastCompleted,
    CastFailed,
    CastInterrupted,
    CastCancelled,
    FatigueEnded,
    BuffExpired,
    BuffDischarged,
    Dismissed,
    CastRejected,
    EchoSkipped,
    ChannelStarted,
    ChannelStopped,
}

#[derive(Clone, Debug)]
pub struct SpellEvent {
    pub time: u32,
    pub kind: SpellEventKind,
    pub essence_spent: u32,
    pub message: String,
}

#[derive(Clone, Debug, Default)]
pub struct MagicState {
    pub essences: Vec<EssencePool>,
    pub casting: Option<CastingSpell>,
    pub channeling: Option<CastingSpell>,
    /// Half-open interval; fatigue starts on the second following completion.
    pub fatigue: Option<(u32, u32)>,
    pub armed_echo: Option<ArmedEcho>,
    pub echoes: Vec<ScheduledEcho>,
    pub owned_buffs: Vec<String>,
    pub now: u32,
    pub auto_attempted: bool,
    pub started_spells: Vec<String>,
    pub last_cast_at: Option<u32>,
    pub last_instant_cast_at: Option<u32>,
    pub primary_recovery_until: f32,
    pub secondary_recovery_until: f32,
    pub events: Vec<SpellEvent>,
    /// Target candidates prepared by the arena's range/prediction policy.
    pub range_targets: std::collections::BTreeMap<String, usize>,
    pub resolved_effects: Vec<super::ConfiguredEffect>,
    pub sweeping_fields: Vec<super::spell_effects::SweepingField>,
    pub fists: Option<super::spell_effects::SummonedLimbs>,
    pub oppressive_fields: Vec<super::spell_effects::OppressiveField>,
    pub charged_objects: Vec<super::spell_effects::ChargedObject>,
    next_cast_id: u64,
}

impl MagicState {
    pub fn new(profile: &MagicProfile) -> Self {
        Self {
            essences: profile.loadout.essences.clone(),
            ..Self::default()
        }
    }

    pub fn fatigued(&self) -> bool {
        self.fatigue
            .is_some_and(|(start, end)| self.now >= start && self.now < end)
    }

    pub fn can_attack(&self, talents: MagicTalents, slot: WeaponSlot, now: f32) -> bool {
        self.casting.is_none()
            && self.channeling.is_none()
            && (!self.fatigued() || talents.fatigue_penalties().can_attack)
            && now
                >= match slot {
                    WeaponSlot::Primary => self.primary_recovery_until,
                    WeaponSlot::Secondary => self.secondary_recovery_until,
                }
    }

    pub fn defense_penalty(&self, talents: MagicTalents) -> i32 {
        if self.fatigued() {
            talents.fatigue_penalties().defense
        } else {
            0
        }
    }

    pub fn movement_multiplier(&self, talents: MagicTalents) -> f32 {
        if self.fatigued() {
            talents.fatigue_penalties().movement_multiplier
        } else {
            1.0
        }
    }

    pub fn restricted_defense(&self, talents: MagicTalents) -> bool {
        (self.casting.is_some() || self.channeling.is_some()) && !talents.combat_casting
    }

    pub fn event(&mut self, kind: SpellEventKind, spent: u32, message: impl Into<String>) {
        self.events.push(SpellEvent {
            time: self.now,
            kind,
            essence_spent: spent,
            message: message.into(),
        });
    }
}

impl Combatant {
    pub fn spell_fatigue_skill_penalty_percent(&self) -> u8 {
        if self.state.magic.fatigued() {
            self.magic.talents.fatigue_penalties().skill_penalty_percent
        } else {
            0
        }
    }

    pub fn spell_fatigue_action_seconds(&self, normal_seconds: u32) -> u32 {
        normal_seconds.saturating_mul(if self.state.magic.fatigued() {
            u32::from(
                self.magic
                    .talents
                    .fatigue_penalties()
                    .action_time_multiplier,
            )
        } else {
            1
        })
    }
    pub fn magic_blocks_running(&self) -> bool {
        self.state.magic.casting.is_some()
            || self.state.magic.channeling.is_some()
            || (self.state.magic.fatigued()
                && !self.magic.talents.fatigue_penalties().can_run_or_sprint)
    }

    pub fn magic_save_bonus(
        &self,
        kind: crate::core::magic::MagicSaveKind,
        tags: crate::core::magic::SpellTags,
    ) -> i32 {
        self.magic.saves.for_kind(kind) + self.magic.talents.resistance_bonus(tags)
            + self.intimidation_penalty()
    }

    pub fn caster_magic_save_bonus(&self, associated_ability: u8) -> i32 {
        i32::from(self.magic.level) + self.magic.talents.focus_bonus(associated_ability)
    }

    pub fn configure_magic(&mut self, profile: MagicProfile) {
        self.state.magic = MagicState::new(&profile);
        self.magic = profile;
    }

    pub fn magic_can_attack(&self, slot: WeaponSlot, now: f32) -> bool {
        self.state.magic.can_attack(self.magic.talents, slot, now)
    }

    pub fn validate_echo_cast(&self) -> Result<u32, MagicError> {
        if self.state.hp <= 0
            || self.state.trauma_remaining_seconds > 0
            || self.sheet.maneuvers.passive
        {
            return Err(MagicError::Incapacitated);
        }
        let state = &self.state.magic;
        // Include pending fatigue so the completion second cannot be used to
        // queue another spell before next-second fatigue starts.
        if state.casting.is_some() || state.channeling.is_some() || state.fatigue.is_some() {
            return Err(MagicError::Busy);
        }
        let mut loadout = self.magic.loadout.clone();
        loadout.essences = state.essences.clone();
        let cost = loadout.validate_echo(self.magic.level)?;
        SpellKind::EchoStrike.catalog_entry().components.validate(
            self.magic.talents,
            !loadout.arms_restricted,
            !loadout.silenced,
            false,
        )?;
        Ok(cost)
    }

    pub fn cast_known_spell(&mut self, id: &str, now: u32) -> Result<(), MagicError> {
        if id == "echo_strike" {
            return self.cast_echo_strike(now);
        }
        let mut request = SpellRequest::from_loadout(id, &self.magic.loadout)?;
        if let SpellEffect::Configured(effect) = &mut request.effect {
            effect.target = self.state.magic.range_targets.get(id).copied();
        }
        self.start_spell(request, now)
    }

    pub fn cast_echo_strike(&mut self, now: u32) -> Result<(), MagicError> {
        self.state.magic.now = now;
        self.expire_spell_fatigue(now);
        self.validate_echo_cast()?;
        self.start_spell(
            SpellRequest::echo_strike(
                self.magic.loadout.echo_strike,
                self.magic.loadout.echo_essence,
            )?,
            now,
        )?;
        self.state.magic.auto_attempted = true;
        Ok(())
    }

    /// Casting includes the starting second. Zero- and one-second spells
    /// resolve here; longer casts complete after duration minus one ticks.
    pub fn cast_spell(
        &mut self,
        request: SpellRequest,
        now: u32,
        rng: &mut impl Rng,
    ) -> Result<(), MagicError> {
        let completes_now = request.definition.casting_seconds <= 1;
        self.start_spell(request, now)?;
        if completes_now {
            self.advance_magic(now, rng);
        }
        Ok(())
    }

    fn start_spell(&mut self, request: SpellRequest, now: u32) -> Result<(), MagicError> {
        use crate::core::magic::validate_essences;
        let SpellRequest {
            definition,
            effect,
            total_cost: cost,
            essence_index,
        } = request;
        self.expire_spell_fatigue(now);
        if self.state.hp <= 0
            || self.state.trauma_remaining_seconds > 0
            || self.sheet.maneuvers.passive
        {
            return Err(MagicError::Incapacitated);
        }
        if self.state.magic.casting.is_some()
            || self.state.magic.channeling.is_some()
            || self.state.magic.fatigue.is_some()
        {
            return Err(MagicError::Busy);
        }
        if definition.casting_seconds == 0 && self.state.magic.last_instant_cast_at == Some(now) {
            return Err(MagicError::InstantCastLimit);
        }
        if !self.magic.loadout.knows_spell(&definition.id) {
            return Err(MagicError::UnknownSpell);
        }
        if definition.id.trim().is_empty()
            || definition.name.trim().is_empty()
            || definition.level == 0
        {
            return Err(MagicError::InvalidSpell);
        }
        if let SpellEffect::EchoStrike(options) = &effect {
            let spell = SpellKind::EchoStrike.catalog_entry();
            if definition.id != "echo_strike"
                || definition.channelled
                || options.cost()? != cost
                || definition.base_cost != spell.base_cost
                || definition.level != spell.level
                || definition.casting_seconds != spell.casting_seconds
                || definition.components != spell.components
            {
                return Err(MagicError::InvalidSpell);
            }
        } else if definition.id == "echo_strike" {
            return Err(MagicError::InvalidSpell);
        }
        if let SpellEffect::Configured(configured) = &effect {
            let catalog = configured.catalog.as_ref();
            let (mut expected, expected_cost) =
                super::ConfiguredEffect::from_catalog(catalog, &self.magic.loadout)?;
            expected.target = configured.target;
            if *configured != expected
                || cost != expected_cost
                || definition.id != catalog.id
                || definition.base_cost != catalog.base_cost
                || definition.level != catalog.level
                || definition.casting_seconds != catalog.casting_seconds
                || definition.components != catalog.components
                || definition.channelled != configured.channelled()
                || (configured.needs_target() && configured.target.is_none())
            {
                return Err(MagicError::InvalidSpell);
            }
        }
        if let SpellEffect::TimedBuff(buff) = &effect {
            if buff.remaining_seconds <= 0 || buff.id.trim().is_empty() {
                return Err(MagicError::InvalidSpell);
            }
        }
        if let SpellEffect::EchoStrike(options) = &effect {
            options.duration_seconds()?;
        }
        if self.magic.loadout.use_essence_costs {
            validate_essences(&self.state.magic.essences)?;
            let essence = self
                .state
                .magic
                .essences
                .get(essence_index)
                .ok_or(MagicError::InvalidEssence)?;
            if definition.level > essence.proficiency.maximum_spell_level(self.magic.level) {
                return Err(MagicError::SpellLevelTooHigh);
            }
            essence
                .proficiency
                .validate_cost(definition.base_cost, cost)?;
            if essence.current < cost {
                return Err(MagicError::InsufficientEssence);
            }
        }
        let cost = if self.magic.loadout.use_essence_costs {
            cost
        } else {
            0
        };
        definition.components.validate(
            self.magic.talents,
            !self.magic.loadout.arms_restricted,
            !self.magic.loadout.silenced,
            false,
        )?;
        self.state.magic.now = now;
        if definition.casting_seconds == 0 {
            self.state.magic.last_instant_cast_at = Some(now);
        }
        let message = format!(
            "Casting {} ({} seconds)",
            definition.name, definition.casting_seconds
        );
        if !self.state.magic.started_spells.contains(&definition.id) {
            self.state.magic.started_spells.push(definition.id.clone());
        }
        self.state.magic.casting = Some(CastingSpell {
            started_at: now,
            completes_at: now.saturating_add(definition.casting_seconds.saturating_sub(1)),
            cost,
            essence_index,
            definition,
            effect,
        });
        self.state.magic.last_cast_at = Some(now);
        self.state
            .magic
            .event(SpellEventKind::CastStarted, 0, message);
        Ok(())
    }

    fn begin_spell_fatigue(&mut self, now: u32, seconds: u32) {
        let talents = self.magic.talents;
        let start = now.saturating_add(1);
        let end = start.saturating_add(seconds);
        self.state.magic.fatigue = if seconds > 0 {
            Some((start, end))
        } else {
            None
        };
        let reset_at = if talents.fatigue_penalties().reset_weapon_at_completion || seconds == 0 {
            now
        } else {
            end
        };
        let primary = self
            .apply_f32(
                super::StatIdF32::WeaponSpeed,
                self.sheet.offense.weapon.speed,
            )
            .max(1.0);
        let secondary = self
            .sheet
            .offense
            .offhand
            .as_ref()
            .map(|offhand| offhand.weapon.speed)
            .unwrap_or(primary)
            .max(1.0);
        self.state.magic.primary_recovery_until = reset_at as f32 + primary;
        self.state.magic.secondary_recovery_until = reset_at as f32 + secondary;
        self.state.next_attack_time_primary = Some(self.state.magic.primary_recovery_until);
        self.state.next_attack_time_secondary = self
            .sheet
            .offense
            .offhand
            .as_ref()
            .map(|_| self.state.magic.secondary_recovery_until);
    }

    pub fn cancel_spell(&mut self, now: u32) -> Result<(), MagicError> {
        self.state.magic.now = now;
        let cast = self
            .state
            .magic
            .casting
            .as_ref()
            .ok_or(MagicError::NoActiveCast)?;
        if cast.definition.casting_seconds == 0 {
            return Err(MagicError::InstantCastCannotCancel);
        }
        let time_advanced = now > cast.started_at;
        let name = cast.definition.name.clone();
        self.state.magic.casting = None;
        // Undoing a queued cast before the clock advances has no penalty.
        // Once time has advanced, voluntary cancellation gives five seconds
        // of fatigue, independently of completed-cast fatigue reductions.
        if time_advanced {
            self.begin_spell_fatigue(now, 5);
        }
        self.state.magic.event(
            SpellEventKind::CastCancelled,
            0,
            format!("{name} cancelled"),
        );
        Ok(())
    }

    pub fn interrupt_spell(&mut self, now: u32) {
        self.state.magic.now = now;
        if self.state.magic.channeling.is_some() {
            let _ = self.stop_channeling(now);
            self.state.magic.event(
                SpellEventKind::CastInterrupted,
                0,
                "Channelling interrupted",
            );
            return;
        }
        let Some(cast) = self.state.magic.casting.take() else {
            return;
        };
        self.spend_cast_essence(&cast);
        self.begin_spell_fatigue(
            now,
            self.magic
                .talents
                .fatigue_seconds(cast.definition.casting_seconds),
        );
        self.state.magic.event(
            SpellEventKind::CastInterrupted,
            cast.cost,
            format!("{} interrupted", cast.definition.name),
        );
    }

    pub fn stop_channeling(&mut self, now: u32) -> Result<(), MagicError> {
        self.state.magic.now = now;
        let cast = self
            .state
            .magic
            .channeling
            .take()
            .ok_or(MagicError::NoActiveSpell)?;
        if let SpellEffect::TimedBuff(buff) = &cast.effect {
            self.state
                .active_effects
                .retain(|effect| effect.id != buff.id);
            self.state.magic.owned_buffs.retain(|id| id != &buff.id);
        }
        self.state.magic.fists = None;
        self.begin_spell_fatigue(
            now,
            self.magic
                .talents
                .fatigue_seconds(cast.definition.casting_seconds),
        );
        self.state.magic.event(
            SpellEventKind::ChannelStopped,
            0,
            format!("Stopped channelling {}", cast.definition.name),
        );
        Ok(())
    }

    fn spend_cast_essence(&mut self, cast: &CastingSpell) {
        // Essence is reserved while casting and spent only on discharge/failure.
        if let Some(pool) = self.state.magic.essences.get_mut(cast.essence_index) {
            pool.current = pool.current.saturating_sub(cast.cost);
        }
    }

    pub fn dismiss_echo_strike(&mut self, now: u32) -> Result<(), MagicError> {
        self.state.magic.now = now;
        if self.state.hp <= 0 || self.state.trauma_remaining_seconds > 0 {
            return Err(MagicError::Incapacitated);
        }
        if self.magic.loadout.arms_restricted {
            return Err(MagicError::SomaticComponentBlocked);
        }
        if self.state.magic.armed_echo.is_none() && self.state.magic.echoes.is_empty() {
            return Err(MagicError::NoActiveSpell);
        }
        self.state.magic.armed_echo = None;
        self.state.magic.echoes.clear();
        self.state
            .magic
            .event(SpellEventKind::Dismissed, 0, "Echo Strike dismissed");
        Ok(())
    }

    pub fn dismiss_spell(&mut self, effect_id: &str, now: u32) -> Result<(), MagicError> {
        if effect_id == "echo_strike" {
            return self.dismiss_echo_strike(now);
        }
        self.state.magic.now = now;
        if self.state.hp <= 0 || self.state.trauma_remaining_seconds > 0 {
            return Err(MagicError::Incapacitated);
        }
        if self.magic.loadout.arms_restricted {
            return Err(MagicError::SomaticComponentBlocked);
        }
        if !self
            .state
            .magic
            .owned_buffs
            .iter()
            .any(|id| id == effect_id)
        {
            return Err(MagicError::NoActiveSpell);
        }
        if self.state.magic.channeling.as_ref().is_some_and(
            |cast| matches!(&cast.effect, SpellEffect::TimedBuff(buff) if buff.id == effect_id),
        ) {
            return self.stop_channeling(now);
        }
        self.state
            .active_effects
            .retain(|effect| effect.id != effect_id);
        self.state.magic.owned_buffs.retain(|id| id != effect_id);
        self.state.magic.event(
            SpellEventKind::Dismissed,
            0,
            format!("Dismissed {effect_id}"),
        );
        Ok(())
    }

    fn expire_spell_fatigue(&mut self, now: u32) {
        self.state.magic.now = now;
        if self.state.magic.fatigue.is_some_and(|(_, end)| now >= end) {
            self.state.magic.fatigue = None;
            self.state
                .magic
                .event(SpellEventKind::FatigueEnded, 0, "Spell fatigue ended");
        }
    }

    pub fn advance_magic(&mut self, now: u32, rng: &mut impl Rng) {
        self.expire_spell_fatigue(now);
        self.state.magic.owned_buffs.retain(|id| {
            self.state
                .active_effects
                .iter()
                .any(|effect| &effect.id == id && effect.remaining_seconds > 0)
        });
        if self
            .state
            .magic
            .armed_echo
            .as_ref()
            .is_some_and(|buff| now >= buff.expires_at)
        {
            self.state.magic.armed_echo = None;
            self.state.magic.event(
                SpellEventKind::BuffExpired,
                0,
                "Echo Strike expired without discharging",
            );
        }
        if let Some(channel) = &self.state.magic.channeling {
            // Verbal components were discharged at completion; only gestures
            // need to remain possible while channelling.
            if self.state.hp <= 0
                || self.state.trauma_remaining_seconds > 0
                || channel
                    .definition
                    .components
                    .validate(
                        self.magic.talents,
                        !self.magic.loadout.arms_restricted,
                        true,
                        false,
                    )
                    .is_err()
            {
                self.interrupt_spell(now);
            } else if let SpellEffect::TimedBuff(buff) = &channel.effect {
                if let Some(active) = self
                    .state
                    .active_effects
                    .iter_mut()
                    .find(|effect| effect.id == buff.id)
                {
                    active.remaining_seconds = 2;
                }
            }
        }
        let Some(cast) = self.state.magic.casting.as_ref() else {
            return;
        };
        if self.state.hp <= 0
            || self.state.trauma_remaining_seconds > 0
            || cast
                .definition
                .components
                .validate(
                    self.magic.talents,
                    !self.magic.loadout.arms_restricted,
                    !self.magic.loadout.silenced,
                    now >= cast.completes_at,
                )
                .is_err()
        {
            self.interrupt_spell(now);
            return;
        }
        if now < cast.completes_at {
            return;
        }
        let cast = self.state.magic.casting.take().expect("active cast");
        self.spend_cast_essence(&cast);
        let failure = spell_failure_percent(
            self.magic.loadout.encumbrance,
            self.sheet.defense.armor_is_heavy,
            self.sheet.defense.shield_name.is_some() && self.state.shield_intact,
            self.magic.talents,
        );
        if failure > 0 && rng.gen_range(1..=100) <= failure {
            self.begin_spell_fatigue(
                now,
                self.magic
                    .talents
                    .fatigue_seconds(cast.definition.casting_seconds),
            );
            self.state.magic.event(
                SpellEventKind::CastFailed,
                cast.cost,
                format!(
                    "{} failed ({failure}% equipment/encumbrance failure)",
                    cast.definition.name
                ),
            );
            return;
        }
        match &cast.effect {
            SpellEffect::EchoStrike(options) => {
                self.state.magic.next_cast_id += 1;
                self.state.magic.armed_echo = Some(ArmedEcho {
                    expires_at: now
                        .saturating_add(options.duration_seconds().expect("validated options")),
                    options: *options,
                    cast_id: self.state.magic.next_cast_id,
                });
            }
            SpellEffect::Configured(effect) => {
                if let Some(buff) = effect.timed_buff() {
                    self.install_cast_buff(buff, cast.definition.channelled);
                } else {
                    self.state.magic.resolved_effects.push(effect.clone());
                }
            }
            SpellEffect::TimedBuff(buff) => {
                self.install_cast_buff(buff.clone(), cast.definition.channelled)
            }
        }

        self.state.magic.event(
            SpellEventKind::CastCompleted,
            cast.cost,
            format!("{} cast successfully", cast.definition.name),
        );
        if cast.definition.channelled {
            self.state.magic.event(
                SpellEventKind::ChannelStarted,
                0,
                format!("Channelling {}", cast.definition.name),
            );
            self.state.magic.channeling = Some(cast);
        } else {
            self.begin_spell_fatigue(
                now,
                self.magic
                    .talents
                    .fatigue_seconds(cast.definition.casting_seconds),
            );
        }
    }

    fn install_cast_buff(&mut self, mut buff: TemporaryEffect, channelled: bool) {
        self.state
            .active_effects
            .retain(|effect| effect.id != buff.id);
        if !self.state.magic.owned_buffs.contains(&buff.id) {
            self.state.magic.owned_buffs.push(buff.id.clone());
        }
        if channelled {
            buff.remaining_seconds = 2;
        }
        self.state.add_effect(buff);
    }

    fn try_spell_policies(&mut self, now: u32) {
        use crate::core::magic::SpellCastAi;
        if self.magic.loadout.known_spells.is_empty() && !self.magic.loadout.knows_echo_strike {
            return;
        }
        self.expire_spell_fatigue(now);
        if self.state.hp <= 0
            || self.state.trauma_remaining_seconds > 0
            || self.sheet.maneuvers.passive
            || self.state.magic.casting.is_some()
            || self.state.magic.channeling.is_some()
            || self.state.magic.fatigue.is_some()
        {
            return;
        }
        let eligible = |id: &str| match self.magic.loadout.ai_for(id) {
            SpellCastAi::Manual => false,
            SpellCastAi::AtFightStart => !self
                .state
                .magic
                .started_spells
                .iter()
                .any(|started| started == id),
            SpellCastAi::AsOftenAsPossible => true,
        };
        if !self
            .magic
            .loadout
            .known_spells
            .iter()
            .any(|id| eligible(id))
            && !(self.magic.loadout.knows_echo_strike && eligible("echo_strike"))
        {
            return;
        }
        // Preserve catalog order within each priority without allocating and
        // sorting a spell list on every simulated second.
        for already_started in [false, true] {
            for spell in spell_catalog().iter() {
                let started = self
                    .state
                    .magic
                    .started_spells
                    .iter()
                    .any(|id| id == &spell.id);
                if started != already_started {
                    continue;
                }
                let ai = self.magic.loadout.ai_for(&spell.id);
                if !self.magic.loadout.knows_spell(&spell.id) || ai == SpellCastAi::Manual {
                    continue;
                }
                if started && matches!(ai, SpellCastAi::AtFightStart) {
                    continue;
                }
                // Repeating Echo Strike shares Use once's opening cast. Only
                // subsequent casts wait for an enemy near weapon reach.
                let requires_range = (matches!(ai, SpellCastAi::AsOftenAsPossible)
                    && (started || spell.kind != SpellKind::EchoStrike))
                    || spell
                        .mechanics
                        .as_ref()
                        .is_some_and(|m| m.target == crate::core::magic::EffectTarget::Enemy);
                if requires_range && !self.state.magic.range_targets.contains_key(&spell.id) {
                    continue;
                }
                match spell.kind {
                    SpellKind::EchoStrike => {
                        if self.state.magic.armed_echo.is_some()
                            || self.validate_echo_cast().is_err()
                        {
                            continue;
                        }
                        if self.cast_echo_strike(now).is_ok() {
                            return;
                        }
                    }
                    _ => {
                        if self.state.has_active_effect(&spell.id) {
                            continue;
                        }
                        if self.cast_known_spell(&spell.id, now).is_ok() {
                            return;
                        }
                    }
                }
            }
        }
    }

    pub fn try_auto_cast(&mut self, now: u32, enemy_in_reach: bool) {
        self.state.magic.now = now;
        if self.magic.loadout.auto_cast == AutoCast::SpellPolicies {
            self.try_spell_policies(now);
            return;
        }
        if self.state.magic.auto_attempted || self.state.hp <= 0 {
            return;
        }
        let eligible = match self.magic.loadout.auto_cast {
            AutoCast::SpellPolicies | AutoCast::Manual => false,
            AutoCast::AtStart => true,
            AutoCast::InWeaponReach => enemy_in_reach,
        };
        if !eligible || self.state.trauma_remaining_seconds > 0 {
            return;
        }
        self.state.magic.auto_attempted = true;
        if let Err(error) = self.cast_echo_strike(now) {
            self.state.magic.event(
                SpellEventKind::CastRejected,
                0,
                format!("Echo Strike not cast: {error}"),
            );
        }
    }
}

/// Called after each primary strike or reaction resolves, before further reactions.
pub(crate) fn on_attack_resolved(
    combatants: &mut [Combatant],
    attacker: usize,
    defender: usize,
    now: f32,
    hit: bool,
    damage: i32,
    slot: WeaponSlot,
    is_ranged: bool,
    roll: &AttackRollBreakdown,
) {
    let now = now as u32;
    combatants[attacker]
        .state
        .active_effects
        .retain(|effect| !effect.consume_on_attack);
    if hit {
        combatants[attacker].state.magic.now = now;
        if let Some(buff) = combatants[attacker].state.magic.armed_echo.take() {
            if now < buff.expires_at {
                let state = &mut combatants[attacker].state.magic;
                for ordinal in 0..=buff.options.additional_echoes {
                    state.echoes.push(ScheduledEcho {
                        due_at: now
                            .saturating_add(buff.options.delay_seconds.saturating_mul(ordinal + 1)),
                        target: defender,
                        damage: if buff.options.full_damage {
                            damage.max(0)
                        } else {
                            damage.max(0)
                                / SpellKind::EchoStrike
                                    .catalog_entry()
                                    .echo_rules
                                    .as_ref()
                                    .expect("validated echo rules")
                                    .normal_damage_divisor
                        },
                        ordinal,
                        cast_id: buff.cast_id,
                        weapon_slot: slot,
                        is_ranged,
                        original_roll: roll.clone(),
                    });
                }
                state.event(
                    SpellEventKind::BuffDischarged,
                    0,
                    format!(
                        "Echo Strike discharged: {} echo(es), first in {}s",
                        buff.options.additional_echoes + 1,
                        buff.options.delay_seconds
                    ),
                );
            }
        }
    }
    if hit || damage > 0 {
        combatants[defender].interrupt_spell(now);
    }
}

pub(crate) fn take_due_echoes(
    combatants: &mut [Combatant],
    now: u32,
) -> Vec<(usize, ScheduledEcho)> {
    let mut due = Vec::new();
    for (caster, combatant) in combatants.iter_mut().enumerate() {
        for echo in combatant
            .state
            .magic
            .echoes
            .extract_if(.., |echo| echo.due_at <= now)
        {
            due.push((caster, echo));
        }
    }
    due.sort_by_key(|(caster, echo)| (echo.due_at, *caster, echo.cast_id, echo.ordinal));
    due
}

/// Do not end combat while a discharged echo can still affect a living enemy.
pub(crate) fn has_pending_echoes(combatants: &[Combatant]) -> bool {
    combatants.iter().any(|caster| {
        (!caster.magic.loadout.echo_requires_living_caster || caster.state.hp > 0)
            && caster.state.magic.echoes.iter().any(|echo| {
                combatants
                    .get(echo.target)
                    .is_some_and(|target| target.state.hp > 0)
            })
    })
}
