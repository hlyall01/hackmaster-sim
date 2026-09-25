//! Generic catalog effect execution, targeting, and movement prediction.
use super::*;
use crate::core::magic::*;
use crate::core::rules::{expected_damage_expr, penetrating_roll};
use crate::core::sim::spell_effects::{
    ChargedObject, OppressiveField, SummonedLimbs, SweepingField,
};
use crate::core::sim::{
    ConfiguredEffect, SpellEffect, SpellEventKind, SpellRequest, TemporaryEffect,
};
use rand::Rng;

#[cfg(test)]
#[path = "spell_effect_tests.rs"]
mod tests;

impl SimState {
    fn nearest_spell_enemy(&self, caster: usize, range: f32) -> Option<usize> {
        self.combatants
            .iter()
            .enumerate()
            .filter(|(i, c)| {
                *i != caster && c.team_id != self.combatants[caster].team_id && c.state.hp > 0
            })
            .filter_map(|(i, _)| {
                self.distance_between(caster, i)
                    .filter(|d| *d <= range)
                    .map(|d| (i, d))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
            .map(|(i, _)| i)
    }

    /// Run the actual arena movement on a private copy, with attacks and spell
    /// damage disabled. Existing casting/fatigue timers still advance. This includes
    /// grid rounding, boundaries, immobilization,
    /// casting movement limits, ranged kiting, and the arena's active pair.
    pub(super) fn predict_spell_target(
        &self,
        caster: usize,
        request: &SpellRequest,
        range: f32,
    ) -> Option<usize> {
        let ticks = request.definition.casting_seconds.saturating_sub(1);
        if ticks == 0 {
            return self.nearest_spell_enemy(caster, range);
        }
        let mut forecast = Self {
            actors: self.actors.clone(),
            combatants: self.combatants.clone(),
            elapsed_seconds: self.elapsed_seconds,
            hold_at_bay: self.hold_at_bay.clone(),
            previous_positions: self.previous_positions.clone(),
            predicting_movement: true,
            ..Self::with_rng_and_log(self.config, self.rng.clone(), false)
        };
        forecast.combatants[caster].state.magic.casting =
            Some(crate::core::sim::magic::CastingSpell {
                started_at: self.elapsed_seconds,
                completes_at: self
                    .elapsed_seconds
                    .saturating_add(request.definition.casting_seconds.saturating_sub(1)),
                cost: 0,
                essence_index: 0,
                definition: request.definition.clone(),
                effect: request.effect.clone(),
            });
        for _ in 0..ticks {
            forecast.tick();
        }
        forecast.nearest_spell_enemy(caster, range)
    }

    pub(super) fn prepare_spell_targets(&mut self, caster: usize) {
        self.combatants[caster].state.magic.range_targets.clear();
        let actor = &self.combatants[caster];
        if actor.magic.loadout.known_spells.is_empty() && !actor.magic.loadout.knows_echo_strike {
            return;
        }
        if actor.state.hp <= 0
            || actor.state.trauma_remaining_seconds > 0
            || actor.state.magic.casting.is_some()
            || actor.state.magic.channeling.is_some()
            || actor.state.magic.fatigue.is_some()
        {
            return;
        }
        let reach = actor
            .apply_f32(StatIdF32::WeaponReach, actor.sheet.offense.weapon.reach_ft)
            .max(1.0);
        for spell in spell_catalog() {
            let actor = &self.combatants[caster];
            if !actor.magic.loadout.knows_spell(&spell.id) {
                continue;
            }
            let ai = actor.magic.loadout.ai_for(&spell.id);
            if ai == SpellCastAi::Manual
                || (matches!(ai, SpellCastAi::AtFightStart)
                    && actor.state.magic.started_spells.contains(&spell.id))
            {
                continue;
            }
            if actor.state.has_active_effect(&spell.id)
                || (spell.kind == SpellKind::EchoStrike && actor.state.magic.armed_echo.is_some())
                || actor
                    .state
                    .magic
                    .sweeping_fields
                    .iter()
                    .any(|f| f.spell.id == spell.id)
                || actor
                    .state
                    .magic
                    .oppressive_fields
                    .iter()
                    .any(|f| f.spell.id == spell.id)
                || actor
                    .state
                    .magic
                    .charged_objects
                    .iter()
                    .any(|f| f.spell.id == spell.id)
            {
                continue;
            }
            let Ok(request) = SpellRequest::from_loadout(&spell.id, &actor.magic.loadout) else {
                continue;
            };
            let range = match &request.effect {
                SpellEffect::Configured(effect) => effect.targeting_range(reach),
                _ => reach,
            };
            let target = self.predict_spell_target(caster, &request, range);
            if let Some(target) = target {
                self.combatants[caster]
                    .state
                    .magic
                    .range_targets
                    .insert(spell.id.clone(), target);
            }
        }
    }

    pub(super) fn target_manual_spell(
        &self,
        caster: usize,
        request: &mut SpellRequest,
    ) -> Result<(), MagicError> {
        let Some(actor) = self.combatants.get(caster) else {
            return Err(MagicError::Incapacitated);
        };
        if actor.state.hp <= 0 {
            return Err(MagicError::Incapacitated);
        }
        if let SpellEffect::Configured(effect) = &mut request.effect {
            if effect.needs_target() {
                effect.target = self.nearest_spell_enemy(caster, f32::INFINITY);
                if effect.target.is_none() {
                    return Err(MagicError::InvalidSpell);
                }
            }
        }
        Ok(())
    }

    fn effect_message(&mut self, caster: usize, message: String) {
        self.combatants[caster]
            .state
            .magic
            .event(SpellEventKind::BuffDischarged, 0, message);
    }

    fn record_spell_miss(&mut self, caster: usize, target: usize, spell: &ConfiguredEffect) {
        self.record_attack_metrics(RecordedAttackMetrics {
            damage_source: DamageSource::Spell {
                name: spell.name.clone(),
            },
            attacker_idx: caster,
            defender_idx: target,
            hp_damage: 0,
            damage_rolled: None,
            damage_landed: Some(0),
            highest_hit_bucket: None,
            instant_kill: false,
            shield_block: false,
            shield_broken: false,
            shield_damage: 0,
            knockback_ft: 0.0,
            attempted: true,
            direct_hit: false,
            critical: false,
            trauma_applied: false,
            defender_hp_after: self.combatants[target].state.hp,
            armor_prevented: 0,
            shield_prevented: 0,
        });
    }

    fn spell_saved(
        &mut self,
        caster: usize,
        target: usize,
        spell: &ConfiguredEffect,
        rule: SaveRule,
    ) -> bool {
        self.refresh_intimidation();
        let a = &self.combatants[caster];
        let kind = rule.kind;
        let ability = if spell.definition.casting_ability == CastingAbility::Constitution {
            a.sheet.vitals.constitution
        } else {
            a.magic.strength.base
        };
        let attack = a.caster_magic_save_bonus(ability);
        let defense = self.combatants[target].magic_save_bonus(kind, SpellTags::default());
        let save = roll_magic_save(attack, defense, &mut self.rng);
        self.effect_message(
            caster,
            format!(
                "{}: {:?} save {} vs {} — {}",
                self.combatants[target].sheet.name,
                kind,
                save.target_total,
                save.caster_total,
                if save.saved { "saved" } else { "failed" }
            ),
        );
        save.saved
    }

    fn damage_average(spell: &ConfiguredEffect, count: u32, extra: i32) -> i32 {
        let d = spell.damage_rule();
        let suffix = if d.penetrating { "p" } else { "" };
        (expected_damage_expr(&format!("{count}d{}{suffix}", d.sides)).floor() as i32
            + d.bonus_per_die * count as i32
            + d.flat_bonus
            + extra)
            .max(0)
    }

    fn roll_damage_pool(&mut self, spell: &ConfiguredEffect, count: u32) -> Vec<i32> {
        let d = spell.damage_rule();
        (0..count)
            .map(|_| {
                (if d.penetrating {
                    penetrating_roll(d.sides, &mut self.rng)
                } else {
                    self.rng.gen_range(1..=d.sides)
                }) + d.bonus_per_die
            })
            .collect()
    }

    fn spell_damage_roll(
        &mut self,
        target: usize,
        spell: &ConfiguredEffect,
        count: u32,
        extra: i32,
    ) -> i32 {
        if self.combatants[target]
            .state
            .streamline_averages_incoming_damage
        {
            Self::damage_average(spell, count, extra)
        } else {
            (self.roll_damage_pool(spell, count).iter().sum::<i32>()
                + spell.damage_rule().flat_bonus
                + extra)
                .max(0)
        }
    }

    fn resolve_damage_hit(
        &mut self,
        caster: usize,
        target: usize,
        spell: &ConfiguredEffect,
        mut raw: i32,
    ) {
        if let EffectRule::Damage { attack, save, .. } = &spell.definition.effect {
            if let Some(attack) = attack {
                let roll = penetrating_roll(attack.attack_die, &mut self.rng)
                    + self.combatants[caster].apply_i32(
                        StatIdI32::AttackBonusBase,
                        self.combatants[caster].sheet.offense.attack_bonus_base,
                    )
                    + spell.bonus;
                let defense =
                    penetrating_roll(attack.defense_die, &mut self.rng) + attack.defense_bonus;
                if roll <= defense {
                    self.record_spell_miss(caster, target, spell);
                    self.effect_message(
                        caster,
                        format!(
                            "{} misses {} (attack {roll}, defense {defense})",
                            spell.name, self.combatants[target].sheet.name
                        ),
                    );
                    return;
                }
            }
            if let Some(save) = save {
                if self.spell_saved(caster, target, spell, *save) {
                    match save.outcome {
                        SaveOutcome::Negate => return,
                        SaveOutcome::Half => raw /= 2,
                    }
                }
            }
        }
        self.deal_spell_wound(caster, target, spell, raw);
    }

    fn deal_spell_wound(
        &mut self,
        caster: usize,
        target: usize,
        spell: &ConfiguredEffect,
        raw: i32,
    ) {
        let damage_rule = spell.damage_rule();
        let phantom = damage_rule.phantom;
        let dr = self.combatants[target].spell_damage_reduction(damage_rule.armor_reduction);
        let wound = (raw - dr).max(0);
        let damage = if phantom { 0 } else { wound };
        if !self.combatants[target].sheet.vitals.infinite_hp {
            self.combatants[target].state.hp =
                self.combatants[target].state.hp.saturating_sub(damage);
        }
        let trauma = crate::core::sim::combat::maybe_apply_trauma(
            &mut self.combatants,
            target,
            wound,
            &mut self.rng,
        );
        if !phantom || trauma.is_some() {
            self.combatants[target].interrupt_spell(self.elapsed_seconds);
        }
        let push = if damage_rule.knockback {
            crate::core::sim::knockback_rule_for_attack(
                &self.combatants[caster],
                &self.combatants[target],
                None,
                true,
                false,
            )
            .distance_ft(raw)
        } else {
            0.0
        };
        if push > 0.0 {
            self.apply_knockback(caster, target, push);
            let speed = self.combatants[target].sheet.offense.weapon.speed;
            crate::core::sim::combat::apply_knockback_recovery(
                &mut self.combatants[target],
                push,
                self.elapsed_seconds as f32,
                speed,
                false,
            );
        }
        self.record_attack_metrics(RecordedAttackMetrics {
            damage_source: DamageSource::Spell {
                name: spell.name.clone(),
            },
            attacker_idx: caster,
            defender_idx: target,
            hp_damage: damage,
            damage_rolled: Some(if phantom { 0 } else { raw }),
            damage_landed: Some(damage),
            highest_hit_bucket: Some(false),
            instant_kill: false,
            shield_block: false,
            shield_broken: false,
            shield_damage: 0,
            knockback_ft: push,
            attempted: true,
            direct_hit: true,
            critical: false,
            trauma_applied: trauma.is_some(),
            defender_hp_after: self.combatants[target].state.hp,
            armor_prevented: if phantom {
                0
            } else {
                raw.min(dr).max(0) as u32
            },
            shield_prevented: 0,
        });
        self.effect_message(
            caster,
            format!(
                "{} hits {}: {} {}damage{}",
                spell.name,
                self.combatants[target].sheet.name,
                if phantom { wound } else { damage },
                if phantom { "phantom " } else { "" },
                if trauma.is_some() { "; trauma" } else { "" }
            ),
        );
    }

    fn push_spell_target(&mut self, caster: usize, target: usize, feet: f32) {
        let tiles = (feet / self.config.tile_size_ft.max(0.01)).floor() as i32;
        self.move_away(target, caster, tiles);
        self.combatants[target].state.moved_last_tick |= tiles > 0;
    }

    fn apply_strength_spell(&mut self, caster: usize, spell: &ConfiguredEffect) {
        let EffectRule::Strength {
            score,
            maximum_base,
            priority,
            consume_on_attack,
        } = &spell.definition.effect
        else {
            return;
        };
        let actor = &self.combatants[caster];
        let fractional = match score {
            StrengthRule::Set { fractional_score } => *fractional_score,
            StrengthRule::AddRoll {
                sides,
                fractional_units_per_point,
            } => {
                u32::from(actor.magic.strength.base) * 100
                    + u32::from(actor.magic.strength.percentile)
                    + self.rng.gen_range(1..=*sides) * fractional_units_per_point
            }
        }
        .saturating_add(spell.strength);
        let score = crate::character::AbilityScore::new(
            (fractional / 100).min(u32::from(*maximum_base)) as u8,
            (fractional % 100) as u8,
        );
        let damage = crate::character::lookup_strength(&crate::character::AbilityScore::new(
            score.base,
            if score.percentile > 50 { 51 } else { 1 },
        ))
        .damage;
        let mut buff = TemporaryEffect::new(&spell.id, spell.duration as i32);
        buff.strength_override = Some((damage, *priority));
        buff.consume_on_attack = *consume_on_attack;
        self.combatants[caster].state.add_effect(buff);
        self.effect_message(
            caster,
            format!(
                "{}: Strength {}/{:02} for {}s",
                spell.name, score.base, score.percentile, spell.duration
            ),
        );
    }

    pub(super) fn resolve_configured_effect(&mut self, caster: usize, spell: ConfiguredEffect) {
        self.refresh_intimidation();
        let id = spell.id.clone();
        self.combatants[caster]
            .state
            .active_effects
            .retain(|b| b.id != id);
        self.resolve_effect_rule(caster, spell);
        if self.combatants[caster].state.has_active_effect(&id)
            && !self.combatants[caster]
                .state
                .magic
                .owned_buffs
                .contains(&id)
        {
            self.combatants[caster].state.magic.owned_buffs.push(id);
        }
    }

    fn resolve_effect_rule(&mut self, caster: usize, spell: ConfiguredEffect) {
        if let Some(buff) = spell.timed_buff() {
            self.combatants[caster].state.add_effect(buff);
            return;
        }
        if let EffectRule::Sequence { effects } = &spell.definition.effect {
            for rule in effects {
                let mut part = spell.clone();
                part.definition.effect = rule.clone();
                self.resolve_effect_rule(caster, part);
            }
            return;
        }
        if matches!(spell.definition.effect, EffectRule::Strength { .. }) {
            self.apply_strength_spell(caster, &spell);
            return;
        }
        let target = if spell.needs_target() {
            spell.target
        } else {
            Some(caster)
        };
        let Some(target) =
            target.filter(|&t| t < self.combatants.len() && self.combatants[t].state.hp > 0)
        else {
            self.effect_message(
                caster,
                format!("{} dissipates: target unavailable", spell.name),
            );
            if spell.channelled() {
                let _ = self.combatants[caster].stop_channeling(self.elapsed_seconds);
            }
            return;
        };
        if spell.range > 0.0
            && self
                .distance_between(caster, target)
                .is_none_or(|d| d > spell.range)
        {
            self.effect_message(
                caster,
                format!("{} misses: target out of range", spell.name),
            );
            if spell.channelled() {
                let _ = self.combatants[caster].stop_channeling(self.elapsed_seconds);
            }
            return;
        }
        match &spell.definition.effect {
            EffectRule::Damage {
                delivery: DamageDelivery::DelayedRadius { .. },
                ..
            } => {
                let center = self.actors[target].position;
                if spell.duration == 0 {
                    self.resolve_radius_damage(caster, &spell, center);
                } else {
                    self.combatants[caster]
                        .state
                        .magic
                        .charged_objects
                        .push(ChargedObject {
                            center: spell.needs_target().then_some(center),
                            detonates_at: self.elapsed_seconds.saturating_add(spell.duration),
                            spell,
                        });
                }
            }
            EffectRule::RecoveryField { .. } => {
                self.combatants[caster]
                    .state
                    .magic
                    .oppressive_fields
                    .push(OppressiveField {
                        center: self.actors[target].position,
                        radius: spell.radius,
                        expires_at: self.elapsed_seconds.saturating_add(spell.duration),
                        delayed: Vec::new(),
                        spell,
                    });
            }
            EffectRule::Displace {
                subject,
                distance,
                direction,
                recovery,
                stop_distance,
                save,
            } => {
                let moved = if *subject == EffectTarget::Caster {
                    caster
                } else {
                    target
                };
                if save.is_some_and(|save| self.spell_saved(caster, moved, &spell, save)) {
                    return;
                }
                let feet = match distance {
                    DisplacementDistance::ForceOverWeight => {
                        let weight = self.combatants[moved].magic.loadout.body_weight_lbs;
                        spell.force as f32 / if weight == 0 { 200.0 } else { weight as f32 }
                    }
                    DisplacementDistance::KnockbackImpact => {
                        crate::core::sim::knockback_rule_for_attack(
                            &self.combatants[caster],
                            &self.combatants[moved],
                            None,
                            true,
                            false,
                        )
                        .distance_ft(spell.force as i32)
                    }
                };
                match direction {
                    DisplacementDirection::AwayFromCaster => {
                        self.push_spell_target(caster, moved, feet)
                    }
                    DisplacementDirection::TowardTarget => self.move_toward(
                        moved,
                        target,
                        (feet / self.config.tile_size_ft.max(0.01)).floor() as i32,
                        *stop_distance,
                    ),
                }
                self.combatants[moved].state.moved_last_tick = true;
                if *recovery {
                    let speed = self.combatants[moved].sheet.offense.weapon.speed;
                    crate::core::sim::combat::apply_knockback_recovery(
                        &mut self.combatants[moved],
                        feet,
                        self.elapsed_seconds as f32,
                        speed,
                        false,
                    );
                }
                self.effect_message(
                    caster,
                    format!(
                        "{} moves {} up to {feet:.2} feet (grid limited)",
                        spell.name, self.combatants[moved].sheet.name
                    ),
                );
            }
            EffectRule::Damage {
                delivery: DamageDelivery::Volley,
                ..
            } => {
                for _ in 0..spell.projectiles {
                    let raw = self.spell_damage_roll(target, &spell, spell.dice, 0);
                    self.resolve_damage_hit(caster, target, &spell, raw);
                }
            }
            EffectRule::Damage {
                delivery: DamageDelivery::Chain { .. },
                ..
            } => self.resolve_chain_damage(caster, target, &spell),
            EffectRule::Damage {
                delivery: DamageDelivery::MovingLine { caster_immune, .. },
                ..
            } => {
                let origin = self.actors[caster].position;
                let position = self.actors[target].position;
                let dx = (position.x - origin.x) as f32;
                let dy = (position.y - origin.y) as f32;
                let length = dx.hypot(dy).max(1.0);
                self.combatants[caster]
                    .state
                    .magic
                    .sweeping_fields
                    .push(SweepingField {
                        origin,
                        direction: (dx / length, dy / length),
                        started_at: self.elapsed_seconds,
                        touched: if *caster_immune { vec![caster] } else { vec![] },
                        spell,
                    });
            }
            EffectRule::ExtendTrauma { save } => {
                let center = self.actors[target].position;
                let affected: Vec<_> = self
                    .combatants
                    .iter()
                    .enumerate()
                    .filter(|(i, c)| {
                        c.state.hp > 0
                            && c.state.trauma_remaining_seconds > 0
                            && center.manhattan_distance(self.actors[*i].position) as f32
                                * self.config.tile_size_ft
                                <= spell.radius
                    })
                    .map(|(i, _)| i)
                    .collect();
                for i in affected {
                    if !save.is_some_and(|save| self.spell_saved(caster, i, &spell, save)) {
                        self.combatants[i].state.trauma_remaining_seconds = self.combatants[i]
                            .state
                            .trauma_remaining_seconds
                            .saturating_add(spell.duration as i32);
                    }
                }
            }
            EffectRule::SummonMelee {
                count, hp_percent, ..
            } => {
                let hp = ((i64::from(self.combatants[caster].sheet.vitals.max_hp)
                    * i64::from(*hp_percent)
                    + 99)
                    / 100) as i32;
                self.combatants[caster].state.magic.fists = Some(SummonedLimbs {
                    expires_at: self.elapsed_seconds.saturating_add(spell.duration),
                    next_attack: self.elapsed_seconds,
                    hp: vec![hp; *count as usize],
                    spell,
                });
            }
            _ => unreachable!("effect handled before target resolution"),
        }
    }

    fn resolve_chain_damage(&mut self, caster: usize, first: usize, spell: &ConfiguredEffect) {
        let EffectRule::Damage {
            delivery:
                DamageDelivery::Chain {
                    turn_degrees,
                    dice_lost_per_target,
                },
            ..
        } = &spell.definition.effect
        else {
            return;
        };
        let cosine = turn_degrees.to_radians().cos();
        let mut previous = self.actors[caster].position;
        let mut target = first;
        let mut remaining = spell.range;
        let mut struck = vec![caster];
        let wounds = self.roll_damage_pool(spell, spell.dice);
        for dice in (1..=spell.dice)
            .rev()
            .step_by(*dice_lost_per_target as usize)
        {
            let position = self.actors[target].position;
            remaining -= previous.manhattan_distance(position) as f32 * self.config.tile_size_ft;
            if remaining < 0.0 {
                break;
            }
            let raw = if self.combatants[target]
                .state
                .streamline_averages_incoming_damage
            {
                Self::damage_average(spell, dice, 0)
            } else {
                wounds[..dice as usize].iter().sum::<i32>() + spell.damage_rule().flat_bonus
            };
            self.resolve_damage_hit(caster, target, spell, raw);
            struck.push(target);
            let direction = (
                (position.x - previous.x) as f32,
                (position.y - previous.y) as f32,
            );
            let next = self
                .combatants
                .iter()
                .enumerate()
                .filter(|(i, c)| c.state.hp > 0 && !struck.contains(i))
                .filter(|(_, c)| c.team_id != self.combatants[caster].team_id)
                .filter_map(|(i, _)| {
                    let p = self.actors[i].position;
                    let distance = position.manhattan_distance(p) as f32 * self.config.tile_size_ft;
                    let v = ((p.x - position.x) as f32, (p.y - position.y) as f32);
                    let dot = direction.0 * v.0 + direction.1 * v.1;
                    let in_arc = dot > 0.0
                        && dot + 0.0001 >= cosine * direction.0.hypot(direction.1) * v.0.hypot(v.1);
                    (distance <= remaining && in_arc).then_some((i, distance))
                })
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
                .map(|(i, _)| i);
            let Some(next) = next else {
                break;
            };
            previous = position;
            target = next;
        }
    }

    fn resolve_radius_damage(&mut self, caster: usize, spell: &ConfiguredEffect, center: GridPos) {
        let blast = self.roll_damage_pool(spell, spell.dice).iter().sum::<i32>()
            + spell.damage_rule().flat_bonus;
        let immune = matches!(
            spell.definition.effect,
            EffectRule::Damage {
                delivery: DamageDelivery::DelayedRadius {
                    caster_immune: true
                },
                ..
            }
        );
        for target in 0..self.combatants.len() {
            if self.combatants[target].state.hp > 0
                && !(immune && target == caster)
                && center.manhattan_distance(self.actors[target].position) as f32
                    * self.config.tile_size_ft
                    <= spell.radius
            {
                let raw = if self.combatants[target]
                    .state
                    .streamline_averages_incoming_damage
                {
                    Self::damage_average(spell, spell.dice, 0)
                } else {
                    blast
                };
                self.resolve_damage_hit(caster, target, spell, raw);
            }
        }
    }

    pub(super) fn advance_effect_fields(&mut self) {
        for caster in 0..self.combatants.len() {
            let objects = std::mem::take(&mut self.combatants[caster].state.magic.charged_objects);
            for object in objects {
                if self.elapsed_seconds < object.detonates_at {
                    self.combatants[caster]
                        .state
                        .magic
                        .charged_objects
                        .push(object);
                    continue;
                }
                let center = object.center.unwrap_or(self.actors[caster].position);
                self.resolve_radius_damage(caster, &object.spell, center);
            }
            let zones = std::mem::take(&mut self.combatants[caster].state.magic.oppressive_fields);
            for mut zone in zones {
                if self.elapsed_seconds >= zone.expires_at {
                    continue;
                }
                let EffectRule::RecoveryField {
                    extra_seconds,
                    save,
                } = zone.spell.definition.effect
                else {
                    continue;
                };
                for target in 0..self.combatants.len() {
                    let standing = self.combatants[target].state.knockback_immobile_seconds;
                    let in_area = zone.center.manhattan_distance(self.actors[target].position)
                        as f32
                        * self.config.tile_size_ft
                        <= zone.radius;
                    if standing <= 0 || !in_area {
                        zone.delayed.retain(|i| *i != target);
                        continue;
                    }
                    if !zone.delayed.contains(&target) {
                        self.combatants[target].state.knockback_immobile_seconds += extra_seconds;
                        zone.delayed.push(target);
                    } else if standing == 1 && !self.spell_saved(caster, target, &zone.spell, save)
                    {
                        self.combatants[target].state.knockback_immobile_seconds = 2;
                        self.combatants[target].state.set_next_attack_time(
                            WeaponSlot::Primary,
                            Some(self.elapsed_seconds as f32 + 1.0),
                        );
                        self.combatants[target].state.set_next_attack_time(
                            WeaponSlot::Secondary,
                            Some(self.elapsed_seconds as f32 + 1.0),
                        );
                    }
                }
                self.combatants[caster]
                    .state
                    .magic
                    .oppressive_fields
                    .push(zone);
            }
            let fields = std::mem::take(&mut self.combatants[caster].state.magic.sweeping_fields);
            for mut field in fields {
                let age = self.elapsed_seconds.saturating_sub(field.started_at);
                if age == 0 {
                    self.combatants[caster]
                        .state
                        .magic
                        .sweeping_fields
                        .push(field);
                    continue;
                }
                let near = (age - 1) as f32 * field.spell.speed;
                let far = age as f32 * field.spell.speed;
                for target in 0..self.combatants.len() {
                    if field.touched.contains(&target) || self.combatants[target].state.hp <= 0 {
                        continue;
                    }
                    let pos = self.actors[target].position;
                    let x = (pos.x - field.origin.x) as f32 * self.config.tile_size_ft;
                    let y = (pos.y - field.origin.y) as f32 * self.config.tile_size_ft;
                    let along = x * field.direction.0 + y * field.direction.1;
                    let across = (x * field.direction.1 - y * field.direction.0).abs();
                    let previous = self.previous_positions.get(target).copied().unwrap_or(pos);
                    let px = (previous.x - field.origin.x) as f32 * self.config.tile_size_ft;
                    let py = (previous.y - field.origin.y) as f32 * self.config.tile_size_ft;
                    let before = px * field.direction.0 + py * field.direction.1 - near;
                    let after = along - far;
                    let crossing = before * after <= 0.0;
                    let fraction = if (before - after).abs() > f32::EPSILON {
                        (before / (before - after)).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                    let crossing_width = ((px + (x - px) * fraction) * field.direction.1
                        - (py + (y - py) * fraction) * field.direction.0)
                        .abs();
                    if (along >= near && along <= far && across <= field.spell.width / 2.0)
                        || (crossing && crossing_width <= field.spell.width / 2.0)
                    {
                        field.touched.push(target);
                        let ignores_trauma = matches!(
                            field.spell.definition.effect,
                            EffectRule::Damage {
                                delivery: DamageDelivery::MovingLine {
                                    ignore_traumatized: true,
                                    ..
                                },
                                ..
                            }
                        );
                        if !ignores_trauma
                            || self.combatants[target].state.trauma_remaining_seconds <= 0
                        {
                            let raw =
                                self.spell_damage_roll(target, &field.spell, field.spell.dice, 0);
                            self.resolve_damage_hit(caster, target, &field.spell, raw);
                        }
                    }
                }
                if age < field.spell.duration {
                    self.combatants[caster]
                        .state
                        .magic
                        .sweeping_fields
                        .push(field);
                }
            }
            let Some(mut fists) = self.combatants[caster].state.magic.fists.take() else {
                continue;
            };
            if self.combatants[caster].state.magic.channeling.is_none() {
                continue;
            }
            if self.elapsed_seconds >= fists.expires_at || fists.hp.iter().all(|hp| *hp <= 0) {
                let _ = self.combatants[caster].stop_channeling(self.elapsed_seconds);
                continue;
            }
            if self.elapsed_seconds >= fists.next_attack {
                self.refresh_intimidation();
                if let Some(target) = self.nearest_spell_enemy(caster, fists.spell.range) {
                    fists.spell.target = Some(target);
                    let attack = penetrating_roll(20, &mut self.rng)
                        + self.combatants[caster].magic.fist_attack
                        + self.combatants[caster].intimidation_penalty();
                    let preview =
                        crate::core::sim::combat::preview_defense(&self.combatants[target]);
                    let shield = if self.combatants[target].state.shield_intact {
                        4 + self.combatants[target].apply_i32(
                            StatIdI32::ShieldDefenseBonus,
                            self.combatants[target].sheet.defense.shield_defense_bonus,
                        )
                    } else {
                        0
                    };
                    let defense = penetrating_roll(preview.melee_die, &mut self.rng)
                        + preview.melee_bonus
                        + shield
                        + self.combatants[target]
                            .state
                            .magic
                            .defense_penalty(self.combatants[target].magic.talents);
                    if attack > defense {
                        let count = fists.hp.iter().filter(|hp| **hp > 0).count() as u32;
                        let EffectRule::SummonMelee {
                            virtual_strength,
                            strength_damage,
                            maximum_strength_base,
                            ..
                        } = fists.spell.definition.effect
                        else {
                            continue;
                        };
                        let strength_bonus = |fractional: u32| {
                            crate::character::lookup_strength(&crate::character::AbilityScore::new(
                                (fractional / 100).min(u32::from(maximum_strength_base)) as u8,
                                if fractional % 100 > 50 { 51 } else { 1 },
                            ))
                            .damage
                        };
                        let bonus = strength_damage
                            + strength_bonus(virtual_strength + fists.spell.strength)
                            - strength_bonus(virtual_strength);
                        let bonus = bonus * count as i32 / fists.hp.len() as i32;
                        let raw = self.spell_damage_roll(
                            target,
                            &fists.spell,
                            count,
                            bonus + self.combatants[caster].magic.fist_damage,
                        );
                        self.deal_spell_wound(caster, target, &fists.spell, raw);
                    } else {
                        self.record_spell_miss(caster, target, &fists.spell);
                        self.effect_message(
                            caster,
                            format!(
                                "{} misses {}",
                                fists.spell.name, self.combatants[target].sheet.name
                            ),
                        );
                    }
                    fists.next_attack = self.elapsed_seconds.saturating_add(
                        self.combatants[caster].magic.fist_speed.max(1.0).ceil() as u32,
                    );
                }
            }
            self.combatants[caster].state.magic.fists = Some(fists);
            self.retaliate_against_fists(caster);
        }
    }

    /// An engaged melee opponent attacks the nearby fists if the caster is out
    /// of weapon reach. When the caster is reachable, ordinary arena combat wins.
    fn retaliate_against_fists(&mut self, caster: usize) {
        let Some(fists) = self.combatants[caster].state.magic.fists.as_ref() else {
            return;
        };
        let Some(attacker) = fists.spell.target else {
            return;
        };
        let actor = &self.combatants[attacker];
        let now = self.elapsed_seconds as f32;
        if actor.state.hp <= 0
            || actor.state.trauma_remaining_seconds > 0
            || actor.sheet.maneuvers.passive
            || !actor.magic_can_attack(WeaponSlot::Primary, now)
            || actor.sheet.offense.weapon.range_bands_feet.is_some()
            || self
                .distance_between(attacker, caster)
                .is_some_and(|d| d <= actor.sheet.offense.weapon.reach_ft)
            || self
                .distance_between(attacker, caster)
                .is_none_or(|d| d > fists.spell.range)
        {
            return;
        }
        let speed = actor
            .apply_f32(StatIdF32::WeaponSpeed, actor.sheet.offense.weapon.speed)
            .max(1.0);
        let next = actor.state.next_attack_time_primary;
        if next.is_none() {
            self.combatants[attacker]
                .state
                .set_next_attack_time(WeaponSlot::Primary, Some(now + speed));
            return;
        }
        if next.is_some_and(|t| now < t) {
            return;
        }
        let attack = penetrating_roll(20, &mut self.rng)
            + crate::core::sim::combat::preview_attack_bonus(actor, WeaponSlot::Primary);
        let EffectRule::SummonMelee {
            defense_die,
            defense_bonus,
            ..
        } = fists.spell.definition.effect
        else {
            return;
        };
        let spell_name = fists.spell.name.clone();
        let defense = penetrating_roll(defense_die, &mut self.rng)
            + defense_bonus
            + self.combatants[caster].magic.fist_defense;
        let raw = if attack > defense {
            roll_damage_expr(
                actor.sheet.offense.weapon.damage_expr_for_attack(),
                &mut self.rng,
                actor.sheet.offense.weapon.force_nonpenetrating_damage,
            ) + crate::core::sim::combat::preview_damage_bonus(actor, WeaponSlot::Primary, false)
        } else {
            0
        };
        self.combatants[attacker]
            .state
            .set_next_attack_time(WeaponSlot::Primary, Some(now + speed));
        self.combatants[attacker]
            .state
            .active_effects
            .retain(|effect| !effect.consume_on_attack);
        if attack <= defense {
            self.effect_message(
                caster,
                format!(
                    "{} misses a summoned limb",
                    self.combatants[attacker].sheet.name
                ),
            );
            return;
        }
        let fists = self.combatants[caster]
            .state
            .magic
            .fists
            .as_mut()
            .expect("active fists");
        let Some(hand) = fists.hp.iter().position(|hp| *hp > 0) else {
            return;
        };
        let damage = raw.max(0).min(fists.hp[hand]);
        fists.hp[hand] -= damage;
        let destroyed = fists.hp.iter().all(|hp| *hp <= 0);
        if !self.combatants[caster].sheet.vitals.infinite_hp {
            self.combatants[caster].state.hp -= damage;
        }
        let trauma = crate::core::sim::combat::maybe_apply_trauma(
            &mut self.combatants,
            caster,
            damage,
            &mut self.rng,
        );
        self.effect_message(
            caster,
            format!(
                "{spell_name} limb {} takes {damage} damage; caster suffers the same wound",
                hand + 1
            ),
        );
        self.record_attack_metrics(RecordedAttackMetrics {
            damage_source: DamageSource::weapon(
                &self.combatants[attacker].sheet.offense.weapon,
                WeaponSlot::Primary,
            ),
            attacker_idx: attacker,
            defender_idx: caster,
            hp_damage: damage,
            damage_rolled: Some(raw),
            damage_landed: Some(damage),
            highest_hit_bucket: Some(false),
            instant_kill: false,
            shield_block: false,
            shield_broken: false,
            shield_damage: 0,
            knockback_ft: 0.0,
            attempted: true,
            direct_hit: true,
            critical: false,
            trauma_applied: trauma.is_some(),
            defender_hp_after: self.combatants[caster].state.hp,
            armor_prevented: 0,
            shield_prevented: 0,
        });
        if destroyed || self.combatants[caster].state.hp <= 0 || trauma.is_some() {
            let _ = self.combatants[caster].stop_channeling(self.elapsed_seconds);
        }
    }
}
