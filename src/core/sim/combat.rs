use rand::Rng;
use super::damage_sources::DamageSource;

use crate::core::rules::{DamageExprCache, clean_damage_expr, penetrating_roll, roll_damage_expr};

use super::modifiers::{
    ModifierOpI32, StatIdF32, StatIdI32, TemporaryEffect,
};
use super::movement::range_modifier_for_weapon_with_scale;
use super::mounted::MountedDamagePlan;
use super::types::{
    AttackRollBreakdown, Combatant, CombatantState, CriticalHit, DamageBreakdown, DamageDie,
    KnockAsideRollBreakdown, ShieldBreakageStep, ShieldDamageBreakdown, WeaponCache, WeaponSlot,
    defense_plus_four_ready_at,
};
use std::sync::Arc;

const CURSE_OF_AXE_D6_TRIGGERS: &[i32] = &[4, 5, 6];

#[cfg(test)]
#[path = "mounted_damage_tests.rs"]
mod mounted_damage_tests;

#[cfg(test)]
#[path = "damage_source_tests.rs"]
mod damage_source_tests;

#[cfg(test)]
#[path = "knockback_tests.rs"]
mod knockback_tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AttackMode {
    Normal,
    HoldAtBay,
    Charge,
}

const CHARGE_ATTACK_BONUS: i32 = 4;
const CHARGE_DEFENSE_PENALTY_SECONDS: i32 = 5;
const CHARGE_DEFENSE_EFFECT_ID: &str = "charge_defense_penalty";
pub(crate) fn aggressive_attack_bonus(combatant: &Combatant) -> i32 {
    5 + i32::from(combatant.apply_i32(StatIdI32::FlagProjectConfidence, 0) > 0)
}

fn aggressive_attack_available(combatant: &Combatant, mode: AttackMode, ranged: bool) -> bool {
    let m = &combatant.sheet.maneuvers;
    m.aggressive_attack && mode == AttackMode::Normal && !ranged
        && !m.called_shot && !m.power_attack && !m.fight_defensively
        && !m.full_parry && !m.tactical_move && !m.fighting_withdrawal && !m.flee
        && combatant.state.tactical_next_attack_penalty == 0
}

fn aggressive_retreat_halves_damage(attacker: &Combatant, defender: &Combatant) -> bool {
    match defender.state.tactical_retreat {
        Some(true) => true,
        Some(false) => attacker.apply_i32(StatIdI32::FlagProjectConfidence, 0) == 0,
        None => false,
    }
}
const REGENSTAT_STACK_CAP: i32 = 8;
const SIX_PATHS_SHIELD_BLOCK_WINDOW: i32 = 5;
const DEFAULT_SHIELD_BLOCK_WINDOW: i32 = 10;

/// A temporal strike rolls to hit but never repeats weapon damage rolls,
/// critical effects, worn armor reduction, knockback, or on-hit triggers.
/// Natural DR still protects against each echo wound.
pub(crate) fn resolve_echo(
    combatants: &mut [Combatant],
    caster: usize,
    echo: &super::magic::ScheduledEcho,
    distance_ft: f32,
    now: u32,
    rng: &mut impl Rng,
) -> Option<(super::types::AttackEvent, bool)> {
    use super::magic::SpellEventKind;
    use super::types::{AttackEvent, AttackSource};
    let target = echo.target;
    let attacker = &combatants[caster];
    let weapon = match echo.weapon_slot {
        WeaponSlot::Primary => &attacker.sheet.offense.weapon,
        WeaponSlot::Secondary => attacker
            .sheet
            .offense
            .offhand
            .as_ref()
            .map(|offhand| &offhand.weapon)
            .unwrap_or(&attacker.sheet.offense.weapon),
    };
    let range_mod = if attacker.magic.loadout.echo_requires_weapon_range && echo.is_ranged {
        range_modifier_for_weapon_with_scale(weapon, distance_ft, weapon.range_distance_multiplier)
    } else {
        Some(echo.original_roll.range_mod)
    };
    let out_of_range = attacker.magic.loadout.echo_requires_weapon_range
        && ((!echo.is_ranged && distance_ft > weapon.reach_ft.max(attacker.melee_reach_floor_ft))
            || range_mod.is_none());
    if combatants
        .get(target)
        .is_none_or(|defender| defender.state.hp <= 0)
        || (attacker.magic.loadout.echo_requires_living_caster && attacker.state.hp <= 0)
        || out_of_range
    {
        combatants[caster].state.magic.event(SpellEventKind::EchoSkipped, 0, "Echo Strike dissipated: target unavailable or configured caster/range requirement not met");
        return None;
    }
    let attack_bonus = attack_profile_for_slot(attacker, echo.weapon_slot)
        .map(|profile| profile.attack_bonus)
        .unwrap_or(echo.original_roll.attack_bonus)
        + if regenstat_active(combatants, caster) {
            regenstat_stack_from_state(&attacker.state)
        } else {
            0
        }
        - fight_defensively_attack_penalty(attacker);
    let attack_falling_sun = attacker.apply_i32(StatIdI32::FlagFallingSunStyle, 0) > 0;
    let defender = &combatants[target];
    let shield = defender.state.shield_intact;
    let sides = defense_die_sides(
        echo.is_ranged,
        defender.state.moved_last_tick,
        shield,
        defender.state.trauma_remaining_seconds > 0
            || defender
                .state
                .magic
                .restricted_defense(defender.magic.talents),
        defender
            .sheet
            .maneuvers
            .offensive_dualwielding_defense_penalty,
    );
    let (attack_die, _) = roll_attack_or_defense_d20(attack_falling_sun, rng);
    let (defense_die, _) = if sides == 20 {
        roll_attack_or_defense_d20(
            defender.apply_i32(StatIdI32::FlagFallingSunStyle, 0) > 0,
            rng,
        )
    } else {
        penetrating_roll_with_first(sides, rng)
    };
    let stance = fight_defensively_defense_bonus(defender)
        + mounted_defense_bonus(defender)
        + if regenstat_active(combatants, target) {
            regenstat_stack_from_state(&defender.state)
        } else {
            0
        }
        - called_shot_defense_penalty(defender);
    let shield_bonus = if shield {
        defender.apply_i32(
            StatIdI32::ShieldDefenseBonus,
            defender.sheet.defense.shield_defense_bonus,
        ) + if echo.is_ranged { 0 } else { 4 }
    } else {
        0
    };
    let (defense_base, shield_bonus) = if echo.is_ranged {
        let dodge = defender.apply_i32(
            StatIdI32::RangedDefenseMod,
            defender.sheet.defense.ranged_defense_mod,
        );
        if dodge != 0 && dodge >= shield_bonus {
            (dodge + stance, 0)
        } else {
            (stance, shield_bonus)
        }
    } else {
        (
            defender.apply_i32(StatIdI32::DefenseMod, defender.sheet.defense.defense_mod)
                + stance
                + movement_melee_defense_bonus(&defender.state),
            shield_bonus,
        )
    };
    let weapon_defense_bonus = if !echo.is_ranged
        && (defender.sheet.offense.weapon.defense_bonus_always
            || defense_plus_four_ready_at(&defender.sheet, &defender.state, now as f32))
    {
        4
    } else {
        0
    };
    let defense_base = defense_base + echo.ordinal as i32 * crate::core::magic::SpellKind::EchoStrike.catalog_entry().echo_rules.as_ref().expect("validated echo rules").defense_per_echo;
    let roll = AttackRollBreakdown {
        attack_die,
        defense_die,
        attack_bonus,
        range_mod: range_mod.unwrap_or(0),
        defense_base,
        weapon_defense_bonus,
        shield_defense_bonus: shield_bonus,
        attack_total: attack_die + attack_bonus + range_mod.unwrap_or(0),
        defense_total: defense_die + defense_base + weapon_defense_bonus + shield_bonus,
    };
    let mut hit = roll.attack_total >= roll.defense_total;
    // Prescience is magical avoidance, so its save still applies.
    if hit && echo.is_ranged && defender.sheet.defense.prescience {
        let passive = penetrating_roll(20, rng)
            + defender.apply_i32(StatIdI32::DefenseMod, defender.sheet.defense.defense_mod)
            + stance;
        hit = !feat_of_agility_succeeds(defender, roll.attack_total - passive, rng);
    }
    let precognition_triggered = hit
        && !echo.is_ranged
        && defender.sheet.defense.precognition
        && defender.state.precognition_space_available
        && feat_of_agility_succeeds(defender, roll.attack_total - roll.defense_total, rng);
    let raw_damage = if !hit {
        0
    } else if precognition_triggered {
        echo.damage / 2
    } else {
        echo.damage
    };
    let natural_dr = defender.spell_damage_reduction(false);
    let damage = (raw_damage - natural_dr).max(0);
    combatants[target].state.aggressive_defense_pending = false;
    if hit {
        if !combatants[target].sheet.vitals.infinite_hp {
            combatants[target].state.hp -= damage;
        }
        combatants[target].interrupt_spell(now);
    }
    let trauma_seconds = if damage > 0 {
        maybe_apply_trauma(combatants, target, damage, rng)
    } else {
        None
    };
    Some((
        AttackEvent {
            source: AttackSource::EchoStrike,
            hit,
            shield_block: false,
            damage,
            shield_damage: 0,
            knockback_ft: 0.0,
            hold_at_bay: false,
            is_charge: false,
            is_aggressive: false,
            weapon_slot: echo.weapon_slot,
            use_jab: false,
            is_ranged: echo.is_ranged,
            trauma_applied: trauma_seconds.is_some(),
            trauma_seconds,
            roll,
            damage_breakdown: hit.then_some(DamageBreakdown {
                rolled_damage: 0,
                strength_damage: 0,
                raw_damage,
                armor_dr: natural_dr,
                armor_penetration: 0,
                effective_armor_dr: natural_dr,
                final_damage: damage,
            }),
            shield_damage_breakdown: None,
            defender_hp_after: combatants[target].state.hp,
            critical: None,
        },
        precognition_triggered,
    ))
}
const DECEPTIVE_DEFENDER_CALLED_SHOT_DEFENSE_BONUS: i32 = 4;
const CALLED_SHOT_PRECISION_BONUS_SCALE_BASE: i32 = 8;
const TWELVE_PATHS_DAMAGE_PENALTY: i32 = 3;

fn reaper_extra_dice_multiplier(
    attacker: &Combatant,
    weapon: Option<&super::types::WeaponProfile>,
) -> i32 {
    if attacker.apply_i32(StatIdI32::FlagReaperOfTermonStyle, 0) > 0
        && weapon.is_some_and(|weapon| {
            matches!(
                weapon.name.to_ascii_lowercase().as_str(),
                "scythe" | "sickle"
            )
        })
    {
        2
    } else {
        1
    }
}

fn style_armor_penetration(attacker: &Combatant, defender: &Combatant, base: i32) -> i32 {
    if attacker.apply_i32(StatIdI32::FlagOnePathCrushingStyle, 0) > 0
        && !defender.sheet.defense.armor_is_heavy
        && !(defender.apply_i32(StatIdI32::FlagNpcCombatant, 0) > 0
            && defender.apply_i32(StatIdI32::ArmorDr, defender.sheet.defense.armor_dr) >= 5)
    {
        return 0;
    }
    base
}

/// An immediate style strike does not reschedule either regular attack clock.
pub(crate) fn resolve_style_strike(
    combatants: &mut [Combatant],
    attacker_idx: usize,
    defender_idx: usize,
    slot: WeaponSlot,
    distance_ft: f32,
    now: f32,
    rng: &mut impl Rng,
) -> Option<CounterAttackOutcome> {
    let attacker = &combatants[attacker_idx];
    if !attacker.magic_can_attack(slot, now)
        || attacker.state.hp <= 0
        || attacker.state.trauma_remaining_seconds > 0
        || attacker.sheet.maneuvers.passive
        || combatants[defender_idx].state.hp <= 0
    {
        return None;
    }
    let profile = attack_profile_for_slot(attacker, slot)?;
    let reach = attacker
        .apply_f32(StatIdF32::WeaponReach, profile.weapon.reach_ft)
        .max(attacker.melee_reach_floor_ft.max(0.5));
    if distance_ft > reach {
        return None;
    }
    Some(resolve_counter_attack(
        combatants,
        attacker_idx,
        defender_idx,
        now,
        slot,
        true,
        false,
        true,
        false,
        false,
        1,
        rng,
    ))
}

pub(crate) fn resolve_style_strike_chain(
    combatants: &mut [Combatant],
    attacker_idx: usize,
    defender_idx: usize,
    slot: WeaponSlot,
    distance_ft: f32,
    now: f32,
    rng: &mut impl Rng,
) -> Vec<CounterAttackOutcome> {
    let Some(first) = resolve_style_strike(
        combatants,
        attacker_idx,
        defender_idx,
        slot,
        distance_ft,
        now,
        rng,
    ) else {
        return Vec::new();
    };
    let mut extra = resolve_evonia_counters(
        combatants,
        attacker_idx,
        defender_idx,
        !first.hit,
        None,
        distance_ft + first.knockback_ft,
        now,
        rng,
    );
    let mut results = vec![first];
    results.append(&mut extra);
    results
}

fn resolve_evonia_counters(
    combatants: &mut [Combatant],
    attacker_idx: usize,
    defender_idx: usize,
    melee_miss: bool,
    normal_counter: Option<&CounterAttackOutcome>,
    mut distance_ft: f32,
    now: f32,
    rng: &mut impl Rng,
) -> Vec<CounterAttackOutcome> {
    if combatants[attacker_idx].apply_i32(StatIdI32::FlagLeftHandOfEvoniaStyle, 0) == 0
        && combatants[defender_idx].apply_i32(StatIdI32::FlagLeftHandOfEvoniaStyle, 0) == 0
    {
        return Vec::new();
    }
    let mut pending = std::collections::VecDeque::new();
    if let Some(counter) = normal_counter {
        distance_ft += counter.knockback_ft;
        if !counter.hit && !counter.is_ranged {
            pending.push_back((counter.defender_idx, counter.attacker_idx));
        }
    }
    if melee_miss {
        pending.push_back((defender_idx, attacker_idx));
    }
    let mut outcomes = Vec::new();
    while let Some((parrier, opponent)) = pending.pop_front() {
        if combatants[parrier].apply_i32(StatIdI32::FlagLeftHandOfEvoniaStyle, 0) <= 0 {
            continue;
        }
        if let Some(counter) = resolve_style_strike(
            combatants,
            parrier,
            opponent,
            WeaponSlot::Secondary,
            distance_ft,
            now,
            rng,
        ) {
            distance_ft += counter.knockback_ft;
            if !counter.hit {
                pending.push_back((opponent, parrier));
            }
            outcomes.push(counter);
        }
    }
    outcomes
}

struct AttackProfile {
    weapon: Arc<super::types::WeaponProfile>,
    attack_bonus: i32,
    strength_damage: i32,
    armor_penetration: i32,
    use_jab: bool,
    uses_projectiles: bool,
    damage_penalty: i32,
}

fn attack_profile_for_slot(attacker: &Combatant, slot: WeaponSlot) -> Option<AttackProfile> {
    match slot {
        WeaponSlot::Primary => Some(AttackProfile {
            weapon: attacker.sheet.offense.weapon.clone(),
            attack_bonus: attacker
                .apply_i32(StatIdI32::AttackBonus, attacker.sheet.offense.attack_bonus),
            strength_damage: attacker.strength_damage_for_slot(
                WeaponSlot::Primary,
                attacker.sheet.offense.strength_damage,
            ) + if !attacker.state.shield_intact
                && attacker.apply_i32(StatIdI32::FlagLargeSwordShieldStyle, 0) > 0
            {
                TWELVE_PATHS_DAMAGE_PENALTY
            } else {
                0
            },
            armor_penetration: attacker.apply_i32(
                StatIdI32::ArmorPenetration,
                attacker.sheet.offense.weapon.armor_penetration,
            ),
            use_jab: attacker.sheet.offense.weapon.use_jab,
            uses_projectiles: attacker.sheet.offense.weapon.uses_projectiles,
            damage_penalty: 0,
        }),
        WeaponSlot::Secondary => {
            attacker
                .sheet
                .offense
                .offhand
                .as_ref()
                .map(|offhand| AttackProfile {
                    weapon: offhand.weapon.clone(),
                    attack_bonus: attacker.apply_i32(StatIdI32::AttackBonus, offhand.attack_bonus),
                    strength_damage: attacker
                        .strength_damage_for_slot(WeaponSlot::Secondary, offhand.strength_damage),
                    armor_penetration: attacker.apply_i32(
                        StatIdI32::ArmorPenetration,
                        offhand.weapon.armor_penetration,
                    ),
                    use_jab: offhand.weapon.use_jab,
                    uses_projectiles: offhand.weapon.uses_projectiles,
                    damage_penalty: if attacker.apply_i32(StatIdI32::FlagLeftHandOfEvoniaStyle, 0)
                        > 0
                    {
                        0
                    } else {
                        attacker.sheet.maneuvers.dualwield_offhand_damage_penalty
                    },
                })
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct AttackStateSnapshot {
    pub next_attack_time_primary: Option<f32>,
    pub next_attack_time_secondary: Option<f32>,
    pub trauma_remaining_seconds: i32,
    shield_intact: bool,
    moved_last_tick: bool,
    regenstat_stacks: i32,
    movement_melee_bonus: i32,
    movement_ranged_distance: f32,
    deceptive_defender_seen_attacker: bool,
}
impl AttackStateSnapshot {
    pub fn capture(combatant: &Combatant, opponent_idx: usize) -> Self {
        let state = &combatant.state;
        Self {
            next_attack_time_primary: state.next_attack_time_primary,
            next_attack_time_secondary: state.next_attack_time_secondary,
            trauma_remaining_seconds: state.trauma_remaining_seconds,
            shield_intact: state.shield_intact,
            moved_last_tick: state.moved_last_tick,
            regenstat_stacks: state.regenstat_stacks,
            movement_melee_bonus: state.movement_spell_defense().0,
            movement_ranged_distance: state.movement_spell_defense().1,
            deceptive_defender_seen_attacker: state
                .deceptive_defender_seen_attackers
                .contains(&opponent_idx),
        }
    }
    fn defense_ready(&self, sheet: &super::types::CombatantSheet, now: f32) -> bool {
        super::types::defense_plus_four_ready_with_timing(
            sheet,
            self.trauma_remaining_seconds,
            self.next_attack_time_primary,
            now,
        )
    }
    fn movement_defense_bonus(&self) -> i32 { self.movement_melee_bonus }

}
pub(crate) struct CombatRoundSnapshot([(usize, AttackStateSnapshot); 2]);
impl CombatRoundSnapshot {
    pub fn capture(combatants: &[Combatant], a: usize, b: usize) -> Self {
        Self([
            (a, AttackStateSnapshot::capture(&combatants[a], b)),
            (b, AttackStateSnapshot::capture(&combatants[b], a)),
        ])
    }
    pub fn get(&self, idx: usize) -> Option<&AttackStateSnapshot> {
        self.0
            .iter()
            .find(|(index, _)| *index == idx)
            .map(|(_, state)| state)
    }
}

fn regenstat_stack_from_state(state: &CombatantState) -> i32 {
    state.regenstat_stacks.clamp(0, REGENSTAT_STACK_CAP)
}

fn regenstat_active(combatants: &[Combatant], idx: usize) -> bool {
    combatants[idx].apply_i32(StatIdI32::FlagRegenstatStyle, 0) > 0
}

pub(crate) fn preview_attack_bonus(combatant: &Combatant, slot: WeaponSlot) -> i32 {
    attack_profile_for_slot(combatant, slot).map_or(0, |profile|
        standard_attack_bonus(combatant, &profile))
}

fn standard_attack_bonus(combatant: &Combatant, profile: &AttackProfile) -> i32 {
    profile.attack_bonus - fight_defensively_attack_penalty(combatant)
}

fn attack_damage_bonus(profile: &AttackProfile, is_ranged: bool) -> i32 {
    if is_ranged && profile.uses_projectiles { 0 } else { profile.strength_damage }
}

pub(crate) fn preview_damage_bonus(combatant: &Combatant, slot: WeaponSlot, is_ranged: bool) -> i32 {
    attack_profile_for_slot(combatant, slot).map_or(0, |profile| attack_damage_bonus(&profile, is_ranged))
}

pub(crate) fn persistent_defense_bonus(combatant: &Combatant) -> i32 {
    if combatant.apply_i32(StatIdI32::FlagPilgrimsPathStyle, 0) > 0 { 4 } else { 0 }
}

pub(crate) struct DefensePreview {
    pub melee_bonus: i32,
    pub ranged_bonus: i32,
    pub melee_die: i32,
    pub ranged_stationary_die: i32,
    pub ranged_moving_die: i32,
}

fn base_defense_modifier(combatant: &Combatant, ranged: bool) -> i32 {
    let (stat, base) = if ranged {
        (StatIdI32::RangedDefenseMod, combatant.sheet.defense.ranged_defense_mod)
    } else {
        (StatIdI32::DefenseMod, combatant.sheet.defense.defense_mod)
    };
    combatant.apply_i32(stat, base) - called_shot_defense_penalty(combatant)
}

fn ranged_defense_choice(dodge: i32, shield: i32, shield_active: bool, stance: i32) -> (i32, i32, bool) {
    if dodge != 0 && dodge >= shield {
        (dodge + stance, 0, false)
    } else {
        (stance, shield, shield_active)
    }
}

pub(crate) fn preview_defense(combatant: &Combatant) -> DefensePreview {
    let situational = fight_defensively_defense_bonus(combatant) + mounted_defense_bonus(combatant);
    let dice = |ranged, moved| defense_die_sides(ranged, moved, combatant.state.shield_intact,
        combatant.state.trauma_remaining_seconds > 0 || combatant.state.magic.restricted_defense(combatant.magic.talents),
        combatant.sheet.maneuvers.offensive_dualwielding_defense_penalty);
    DefensePreview {
        melee_bonus: base_defense_modifier(combatant, false) + situational,
        ranged_bonus: {
            let shield = if combatant.state.shield_intact {
                combatant.apply_i32(StatIdI32::ShieldDefenseBonus, combatant.sheet.defense.shield_defense_bonus)
            } else { 0 };
            let (base, shield, _) = ranged_defense_choice(base_defense_modifier(combatant, true),
                shield, combatant.state.shield_intact, situational);
            base + shield
        },
        melee_die: dice(false, false),
        ranged_stationary_die: dice(true, false),
        ranged_moving_die: dice(true, true),
    }
}

pub(crate) fn fight_defensively_attack_penalty(combatant: &Combatant) -> i32 {
    if combatant.sheet.maneuvers.fight_defensively {
        combatant
            .sheet
            .maneuvers
            .fight_defensively_attack_penalty
            .max(0)
    } else {
        0
    }
}

fn mounted_defense_bonus(combatant: &Combatant) -> i32 {
    combatant.sheet.maneuvers.mounted_combat.defense_bonus(combatant.sheet.maneuvers.mounted)
}

fn fight_defensively_defense_bonus(combatant: &Combatant) -> i32 {
    let stance_bonus = if combatant.sheet.maneuvers.fight_defensively {
        combatant
            .sheet
            .maneuvers
            .fight_defensively_defense_bonus
            .max(0)
    } else {
        0
    };
    stance_bonus
        - if combatant.state.aggressive_defense_pending { 2 } else { 0 }
        + combatant
            .state
            .magic
            .defense_penalty(combatant.magic.talents)
        + combatant.state.tactical_give_ground_defense_bonus.max(0)
        + persistent_defense_bonus(combatant)
}

fn called_shot_active(combatant: &Combatant) -> bool {
    combatant.sheet.maneuvers.called_shot
}

fn called_shot_defense_penalty(combatant: &Combatant) -> i32 {
    if called_shot_active(combatant) {
        combatant.sheet.maneuvers.called_shot_defense_penalty.max(0)
    } else {
        0
    }
}

fn called_shot_defense_bonus(combatant: &Combatant) -> i32 {
    combatant.sheet.maneuvers.called_shot_defense_bonus.max(0)
}

fn called_shot_precision_target_bonus(attacker: &Combatant, defender: &Combatant) -> i32 {
    let defender_base = defender
        .sheet
        .maneuvers
        .called_shot_target_defense_bonus_base
        .max(1);
    let attacker_scale = called_shot_defense_bonus(attacker).max(1);
    let scaled = defender_base.saturating_mul(attacker_scale);
    (scaled / CALLED_SHOT_PRECISION_BONUS_SCALE_BASE).max(1)
}

fn movement_melee_defense_bonus(state: &CombatantState) -> i32 { state.movement_spell_defense().0 }

fn update_regenstat_on_exchange(
    combatants: &mut [Combatant],
    attacker_idx: usize,
    defender_idx: usize,
    hit: bool,
) {
    let attacker_active = regenstat_active(combatants, attacker_idx);
    let defender_active = regenstat_active(combatants, defender_idx);
    if attacker_active {
        if hit {
            combatants[attacker_idx].state.regenstat_stacks =
                (combatants[attacker_idx].state.regenstat_stacks + 1).min(REGENSTAT_STACK_CAP);
        } else {
            combatants[attacker_idx].state.regenstat_stacks = 0;
        }
    }
    if defender_active {
        if hit {
            combatants[defender_idx].state.regenstat_stacks = 0;
        } else {
            combatants[defender_idx].state.regenstat_stacks =
                (combatants[defender_idx].state.regenstat_stacks + 1).min(REGENSTAT_STACK_CAP);
        }
    }
}

pub(crate) struct AttackOutcome {
    pub is_aggressive: bool,
    pub(super) damage_source: DamageSource,
    pub(super) attacker_idx: usize,
    pub(super) defender_idx: usize,
    pub(super) knockback_ft: f32,
    pub(super) hit: bool,
    pub(super) shield_block: bool,
    pub(super) damage: i32,
    pub(super) shield_damage: i32,
    pub(super) hold_at_bay: bool,
    pub(super) weapon_slot: WeaponSlot,
    pub(super) use_jab: bool,
    pub(super) is_ranged: bool,
    pub(super) trauma_applied: bool,
    pub(super) trauma_seconds: Option<i32>,
    pub(super) roll: AttackRollBreakdown,
    pub(super) damage_breakdown: Option<DamageBreakdown>,
    pub(super) shield_damage_breakdown: Option<ShieldDamageBreakdown>,
    pub(super) defender_hp_after: i32,
    pub(super) critical: Option<CriticalHit>,
    pub(super) precognition_triggered: bool,
    pub(super) counter_attack: Option<CounterAttackOutcome>,
    pub(super) additional_counters: Vec<CounterAttackOutcome>,
}

pub(crate) struct CounterAttackOutcome {
    pub(super) damage_source: DamageSource,
    pub(super) attacker_idx: usize,
    pub(super) defender_idx: usize,
    pub(super) knockback_ft: f32,
    pub(super) hit: bool,
    pub(super) shield_block: bool,
    pub(super) damage: i32,
    pub(super) shield_damage: i32,
    pub(super) weapon_slot: WeaponSlot,
    pub(super) use_jab: bool,
    pub(super) is_ranged: bool,
    pub(super) trauma_applied: bool,
    pub(super) trauma_seconds: Option<i32>,
    pub(super) roll: AttackRollBreakdown,
    pub(super) damage_breakdown: Option<DamageBreakdown>,
    pub(super) shield_damage_breakdown: Option<ShieldDamageBreakdown>,
    pub(super) defender_hp_after: i32,
    pub(super) critical: Option<CriticalHit>,
    pub(super) precognition_triggered: bool,
}

pub(crate) struct KnockAsideOutcome {
    pub(super) success: bool,
    pub(super) roll: KnockAsideRollBreakdown,
}

pub(crate) fn defense_die_sides(
    is_ranged: bool,
    defender_moved_last_tick: bool,
    has_shield: bool,
    trauma_incapacitated: bool,
    offensive_dualwielding: bool,
) -> i32 {
    if trauma_incapacitated {
        return 8;
    }
    if !is_ranged && offensive_dualwielding {
        return 10;
    }
    if is_ranged {
        if has_shield {
            20
        } else if defender_moved_last_tick {
            20
        } else {
            12
        }
    } else {
        20
    }
}

fn roll_die(sides: i32, rng: &mut impl Rng) -> i32 {
    rng.gen_range(1..=sides)
}

fn penetrating_roll_with_first(sides: i32, rng: &mut impl Rng) -> (i32, i32) {
    if sides <= 1 {
        return (sides.max(0), sides.max(0));
    }
    let first = roll_die(sides, rng);
    let mut total = first;
    if first == sides {
        loop {
            let roll = roll_die(sides, rng);
            total += roll - 1;
            if roll != sides {
                break;
            }
        }
    }
    (total, first)
}

fn penetrating_roll_with_first_max_or_one_less(sides: i32, rng: &mut impl Rng) -> (i32, i32) {
    if sides <= 1 {
        return (sides.max(0), sides.max(0));
    }
    let triggers = [(sides - 1).max(1), sides];
    let first = roll_die(sides, rng);
    let mut total = first;
    if triggers.contains(&first) {
        loop {
            let roll = roll_die(sides, rng);
            total += roll - 1;
            if !triggers.contains(&roll) {
                break;
            }
        }
    }
    (total, first)
}

fn roll_attack_or_defense_d20(falling_sun: bool, rng: &mut impl Rng) -> (i32, i32) {
    if falling_sun {
        penetrating_roll_with_first_max_or_one_less(20, rng)
    } else {
        penetrating_roll_with_first(20, rng)
    }
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CriticalEffect {
    pub severity: i32,
    pub extra_dice: i32,
    pub speed_reset: bool,
    pub auto_trauma: bool,
    pub instant_kill: bool,
}

pub(crate) fn critical_effect_for(severity: i32) -> CriticalEffect {
    let severity = severity.max(1);
    match severity {
        1..=10 => CriticalEffect {
            severity,
            extra_dice: 1,
            speed_reset: false,
            auto_trauma: false,
            instant_kill: false,
        },
        11..=20 => CriticalEffect {
            severity,
            extra_dice: 2,
            speed_reset: false,
            auto_trauma: false,
            instant_kill: false,
        },
        21..=30 => CriticalEffect {
            severity,
            extra_dice: 3,
            speed_reset: true,
            auto_trauma: false,
            instant_kill: false,
        },
        31..=40 => CriticalEffect {
            severity,
            extra_dice: 4,
            speed_reset: false,
            auto_trauma: true,
            instant_kill: false,
        },
        _ => CriticalEffect {
            severity,
            extra_dice: 0,
            speed_reset: false,
            auto_trauma: false,
            instant_kill: true,
        },
    }
}

fn apply_ancillary_critical_immunity(effect: CriticalEffect, immune: bool) -> CriticalEffect {
    if !immune {
        return effect;
    }
    CriticalEffect {
        speed_reset: false,
        auto_trauma: false,
        ..effect
    }
}

fn parse_damage_dice(
    expr: &str,
    force_nonpenetrating: bool,
    d6_penetration_triggers: Option<&[i32]>,
    penetrate_on_max_minus_one: bool,
) -> Vec<DamageDie> {
    let cleaned = clean_damage_expr(expr).to_ascii_lowercase();
    let chars: Vec<char> = cleaned.chars().collect();
    let mut dice: Vec<DamageDie> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let mut count = 0;
            while i < chars.len() && chars[i].is_ascii_digit() {
                count = count * 10 + chars[i].to_digit(10).unwrap_or(0) as i32;
                i += 1;
            }
            if i < chars.len() && chars[i] == 'd' {
                i += 1;
                let mut sides = 0;
                let mut has_sides = false;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    sides = sides * 10 + chars[i].to_digit(10).unwrap_or(0) as i32;
                    has_sides = true;
                    i += 1;
                }
                if has_sides && sides > 0 {
                    let mut penetrating = i < chars.len() && chars[i] == 'p';
                    let mut die_penetrate_on_max_minus_one =
                        penetrating && penetrate_on_max_minus_one;
                    let mut penetration_triggers = if penetrating
                        && sides == 6
                        && d6_penetration_triggers == Some(CURSE_OF_AXE_D6_TRIGGERS)
                    {
                        Some(CURSE_OF_AXE_D6_TRIGGERS)
                    } else {
                        None
                    };
                    if force_nonpenetrating {
                        penetrating = false;
                        penetration_triggers = None;
                        die_penetrate_on_max_minus_one = false;
                    }
                    for _ in 0..count.max(1) {
                        dice.push(DamageDie {
                            sides,
                            penetrating,
                            penetration_triggers,
                            penetrate_on_max_minus_one: die_penetrate_on_max_minus_one,
                        });
                    }
                }
                if i < chars.len() && chars[i] == 'p' {
                    i += 1;
                }
                continue;
            }
            continue;
        }
        if chars[i] == 'd' {
            i += 1;
            let mut sides = 0;
            let mut has_sides = false;
            while i < chars.len() && chars[i].is_ascii_digit() {
                sides = sides * 10 + chars[i].to_digit(10).unwrap_or(0) as i32;
                has_sides = true;
                i += 1;
            }
            if has_sides && sides > 0 {
                let mut penetrating = i < chars.len() && chars[i] == 'p';
                let mut die_penetrate_on_max_minus_one = penetrating && penetrate_on_max_minus_one;
                let mut penetration_triggers = if penetrating
                    && sides == 6
                    && d6_penetration_triggers == Some(CURSE_OF_AXE_D6_TRIGGERS)
                {
                    Some(CURSE_OF_AXE_D6_TRIGGERS)
                } else {
                    None
                };
                if force_nonpenetrating {
                    penetrating = false;
                    penetration_triggers = None;
                    die_penetrate_on_max_minus_one = false;
                }
                dice.push(DamageDie {
                    sides,
                    penetrating,
                    penetration_triggers,
                    penetrate_on_max_minus_one: die_penetrate_on_max_minus_one,
                });
            }
            if i < chars.len() && chars[i] == 'p' {
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    // Stable sorting preserves the original roll order among equally sized dice.
    dice.sort_by_key(|die| die.sides);
    dice
}

pub(crate) fn extra_damage_dice_sequence(
    expr: &str,
    dice: i32,
    force_nonpenetrating: bool,
) -> Vec<DamageDie> {
    if dice <= 0 {
        return Vec::new();
    }
    let pool = parse_damage_dice(expr, force_nonpenetrating, None, false);
    extra_damage_dice_sequence_from_cache(&pool, dice, force_nonpenetrating)
}

fn extra_damage_dice_sequence_from_cache(
    pool: &[DamageDie],
    dice: i32,
    force_nonpenetrating: bool,
) -> Vec<DamageDie> {
    extra_damage_dice_iter(pool, dice, force_nonpenetrating).collect()
}

fn extra_damage_dice_iter(
    pool: &[DamageDie],
    dice: i32,
    force_nonpenetrating: bool,
) -> impl Iterator<Item = DamageDie> + '_ {
    pool.iter().copied().cycle().take(dice.max(0) as usize).map(move |mut die| {
        if force_nonpenetrating {
            die.penetrating = false;
            die.penetration_triggers = None;
            die.penetrate_on_max_minus_one = false;
        }
        die
    })
}

fn average_damage_cache_rounded_down(cache: &DamageExprCache, nonpenetrating: bool) -> i32 {
    cache.expected(nonpenetrating).floor() as i32
}

fn expected_damage_die(die: DamageDie) -> f64 {
    let sides = die.sides.max(0) as f64;
    if sides <= 0.0 {
        return 0.0;
    }
    let standard = (sides + 1.0) / 2.0;
    let trigger_count = if !die.penetrating {
        0
    } else if die.penetrate_on_max_minus_one {
        2
    } else if let Some(triggers) = die.penetration_triggers {
        triggers
            .iter()
            .filter(|roll| (1..=die.sides).contains(roll))
            .count()
    } else {
        1
    };
    if trigger_count == 0 {
        return standard;
    }
    let penetration_chance = (trigger_count as f64).min(sides) / sides;
    if penetration_chance >= 1.0 {
        return standard;
    }
    let continuation_average = ((sides - 1.0) / 2.0) / (1.0 - penetration_chance);
    standard + penetration_chance * continuation_average
}

fn average_extra_damage_rounded_down(sequence: &[DamageDie]) -> i32 {
    sequence
        .iter()
        .copied()
        .map(expected_damage_die)
        .sum::<f64>()
        .floor() as i32
}

fn roll_extra_damage(
    expr: &str,
    dice: i32,
    force_nonpenetrating: bool,
    average: bool,
    rng: &mut impl Rng,
) -> i32 {
    let sequence = extra_damage_dice_sequence(expr, dice, force_nonpenetrating);
    if average {
        return average_extra_damage_rounded_down(&sequence);
    }
    let mut total = 0;
    for die in sequence {
        total += if die.penetrating && die.penetrate_on_max_minus_one {
            crate::core::rules::penetrating_roll_trigger_set(
                die.sides,
                &[(die.sides - 1).max(1), die.sides],
                rng,
            )
        } else if let Some(triggers) = die.penetration_triggers {
            crate::core::rules::penetrating_roll_trigger_set(die.sides, triggers, rng)
        } else if die.penetrating {
            penetrating_roll(die.sides, rng)
        } else {
            roll_die(die.sides, rng)
        };
    }
    total
}

fn mounted_damage_plan(
    attacker: &Combatant,
    defender: &Combatant,
    weapon: &super::WeaponProfile,
    is_ranged: bool,
) -> MountedDamagePlan {
    if !attacker.sheet.maneuvers.mounted || is_ranged || !weapon.has_weapon || weapon.is_unarmed {
        return MountedDamagePlan::default();
    }
    attacker.sheet.maneuvers.mounted_combat.damage_plan(
        &weapon.name, defender.sheet.defense.is_medium_sized,
    )
}

fn mounted_extra_dice(pool: &[DamageDie], plan: MountedDamagePlan) -> Vec<DamageDie> {
    mounted_extra_dice_iter(pool, plan).collect()
}

fn mounted_extra_dice_iter(pool: &[DamageDie], plan: MountedDamagePlan) -> impl Iterator<Item = DamageDie> + '_ {
    pool.iter().copied().take(if plan.double_base_dice { pool.len() } else { 0 })
        .chain(pool.first().copied().into_iter().cycle().take(plan.extra_smallest))
}

fn format_damage_dice(dice: &[DamageDie]) -> String {
    let mut groups: Vec<(i32, bool, usize)> = Vec::new();
    for die in dice {
        if let Some(group) = groups.iter_mut().find(|g| g.0 == die.sides && g.1 == die.penetrating) {
            group.2 += 1;
        } else {
            groups.push((die.sides, die.penetrating, 1));
        }
    }
    groups.into_iter().map(|(sides, penetrating, count)| {
        format!("{count}d{sides}{}", if penetrating { "p" } else { "" })
    }).collect::<Vec<_>>().join(" + ")
}

/// Weapon dice for the current matchup, before Strength and other flat modifiers.
/// Uses the same mounted plan and bonus dice pool as attack resolution.
pub fn weapon_damage_expression(
    attacker: &Combatant,
    defender: &Combatant,
    weapon: &super::WeaponProfile,
    is_ranged: bool,
    shield: bool,
) -> String {
    let (expr, cache) = if shield {
        match (&weapon.shield_damage_expr, &weapon.shield_damage_expr_cache) {
            (Some(expr), Some(cache)) => (expr.as_str(), cache),
            _ => return "-".into(),
        }
    } else {
        (weapon.damage_expr_for_attack(), weapon.damage_expr_cache_for_attack())
    };
    let nonpenetrating = !shield && (weapon.use_jab || weapon.force_nonpenetrating_damage);
    let plan = mounted_damage_plan(attacker, defender, weapon, is_ranged);
    let pool = parse_damage_dice(expr, nonpenetrating, cache.d6_penetration_triggers(),
        cache.penetrate_on_max_minus_one());
    let extra = mounted_extra_dice(&pool, plan);
    let base = if nonpenetrating { expr.replace(['p', 'P'], "") } else { expr.into() };
    if extra.is_empty() {
        return base;
    }

    // Combine ordinary additive dice into e.g. 7d8p, preserving flat terms once.
    // Keep special expressions (such as "lower of") intact and append their bonus dice.
    let compact = base.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    let mut constants = Vec::new();
    let simple = compact.split_inclusive(|c| c == '+' || c == '-');
    let mut sign = '+';
    let mut additive = true;
    for part in simple {
        let term = part.trim_end_matches(['+', '-']);
        if let Ok(value) = term.parse::<i32>() {
            constants.push((sign, value));
        } else if let Some((count, sides)) = term.split_once('d') {
            let sides = sides.strip_suffix('p').unwrap_or(sides);
            additive &= sign == '+' && (count.is_empty() || count.parse::<usize>().is_ok())
                && sides.parse::<i32>().is_ok();
        } else {
            additive = false;
        }
        if let Some(last @ ('+' | '-')) = part.chars().last() { sign = last; }
    }
    if !additive {
        return format!("({base}) + {}", format_damage_dice(&extra));
    }
    let mut dice = pool;
    dice.extend(extra);
    let mut expression = format_damage_dice(&dice);
    for (sign, value) in constants {
        expression.push_str(&format!(" {sign} {value}"));
    }
    expression
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WeaponDamageMode {
    #[default]
    Normal,
    CloseHit,
    OpeningEngagement,
}

#[derive(Clone, Debug)]
pub struct WeaponDamageOption {
    pub mode: WeaponDamageMode,
    pub label: String,
    pub expression: String,
}

fn opening_engagement_extra_dice(attacker: &Combatant) -> i32 {
    attacker.apply_i32(StatIdI32::OpeningEngagementExtraDamageDice,
        i32::from(attacker.apply_i32(StatIdI32::FlagArmerociPoleStyle, 0) > 0)).max(0)
}

/// Available outcomes of the resolved weapon's rules, independent of the macro UI.
pub(crate) fn weapon_roll20_damage_options(
    attacker: &Combatant, defender: &Combatant, slot: WeaponSlot, ranged: bool,
) -> Result<Vec<WeaponDamageOption>, crate::core::rules::DamageExprError> {
    let profile = attack_profile_for_slot(attacker, slot).expect("macro weapon slot");
    let weapon = &profile.weapon;
    let mut options = vec![WeaponDamageOption {
        mode: WeaponDamageMode::Normal,
        label: if weapon.use_jab { "Jab" } else { "Normal weapon damage" }.into(),
        expression: weapon_roll20_damage_expression(attacker, defender, slot, ranged, false)?,
    }];
    if weapon.close_hit_damage_cache_for_attack().is_some() {
        options.push(WeaponDamageOption {
            mode: WeaponDamageMode::CloseHit,
            label: format!("Close hit · margin <{}", weapon.use_close_hit_margin_less_than),
            expression: weapon_roll20_damage_expression(attacker, defender, slot, ranged, true)?,
        });
    }
    let extra_dice = opening_engagement_extra_dice(attacker);
    if !ranged && extra_dice > 0 {
        options.push(WeaponDamageOption {
            mode: WeaponDamageMode::OpeningEngagement,
            label: format!("Opening engagement · +{extra_dice} weapon {}", if extra_dice == 1 { "die" } else { "dice" }),
            expression: weapon_roll20_damage_with_extra_dice(attacker, defender, slot, ranged, false, extra_dice)?,
        });
    }
    Ok(options)
}

/// Roll20 form of the same weapon pool, mounted dice and flat bonuses used in combat.
pub(crate) fn weapon_roll20_damage_expression(
    attacker: &Combatant, defender: &Combatant, slot: WeaponSlot, ranged: bool, close_hit: bool,
) -> Result<String, crate::core::rules::DamageExprError> {
    weapon_roll20_damage_with_extra_dice(attacker, defender, slot, ranged, close_hit, 0)
}

/// The replacement pool and modifiers are resolved once for both combat and Roll20.
struct CloseHitDamage<'a> {
    cache: &'a DamageExprCache,
    nonpenetrating: bool,
    bonus: i32,
}

impl CloseHitDamage<'_> {
    fn roll(&self, rng: &mut impl rand::Rng, average: bool) -> i32 {
        if average {
            average_damage_cache_rounded_down(self.cache, self.nonpenetrating)
        } else {
            self.cache.roll(rng, self.nonpenetrating)
        }
    }

    fn raw_damage(&self, rolled: i32) -> i32 { (rolled + self.bonus).max(0) }

    fn roll20_expression(&self) -> Result<String, crate::core::rules::DamageExprError> {
        let mut expression = self.cache.roll20_expression(self.nonpenetrating)?;
        if self.bonus != 0 { expression.push_str(&format!("{:+}", self.bonus)); }
        Ok(expression)
    }
}

fn close_hit_damage(weapon: &super::WeaponProfile, bonus: i32) -> Option<CloseHitDamage<'_>> {
    Some(CloseHitDamage {
        cache: weapon.close_hit_damage_cache_for_attack()?,
        nonpenetrating: weapon.use_jab || weapon.force_nonpenetrating_damage,
        bonus,
    })
}

fn weapon_roll20_damage_with_extra_dice(
    attacker: &Combatant, defender: &Combatant, slot: WeaponSlot, ranged: bool, close_hit: bool, extra_dice: i32,
) -> Result<String, crate::core::rules::DamageExprError> {
    let profile = attack_profile_for_slot(attacker, slot).expect("macro weapon slot");
    let weapon = &profile.weapon;
    if let Some(damage) = close_hit_damage(weapon, attack_damage_bonus(&profile, ranged) + profile.damage_penalty)
        .filter(|_| close_hit) {
        // Replace the entire pool, including a dedicated two-handed weapon's +3.
        // The profile bonus retains Strength, mastery, material and other modifiers.
        return damage.roll20_expression();
    }
    let nonpenetrating = weapon.use_jab || weapon.force_nonpenetrating_damage;
    let cache = weapon.damage_expr_cache_for_attack();
    let pool = parse_damage_dice(weapon.damage_expr_for_attack(), nonpenetrating,
        cache.d6_penetration_triggers(), cache.penetrate_on_max_minus_one());
    let extra = extra_damage_dice_iter(&pool, extra_dice, nonpenetrating)
        .map(|die| (die.sides, die.penetrating)).collect::<Vec<_>>();
    let halved = weapon.halves_damage_for_attack() || weapon.halve_damage;
    let mut expression = cache.roll20_with_added_dice(nonpenetrating, if halved { &[] } else { &extra })?;
    for die in mounted_extra_dice_iter(&pool, mounted_damage_plan(attacker, defender, weapon, ranged)) {
        let expr = format!("1d{}{}", die.sides, if die.penetrating { "p" } else { "" });
        let cache = if die.penetrate_on_max_minus_one {
            DamageExprCache::new_with_max_minus_one_penetration(&expr)
        } else if let Some(triggers) = die.penetration_triggers {
            DamageExprCache::new_with_d6_penetration_triggers(&expr, triggers)
        } else { DamageExprCache::new(&expr) };
        expression.push_str(&format!("+{}", cache.roll20_expression(nonpenetrating)?));
    }
    let bonus = attack_damage_bonus(&profile, ranged);
    if bonus != 0 { expression.push_str(&format!("{bonus:+}")); }
    if weapon.halves_damage_for_attack() { expression = format!("floor(({expression})/2)"); }
    if weapon.halve_damage { expression = format!("floor(({expression})/2)"); }
    if profile.damage_penalty != 0 { expression.push_str(&format!("{:+}", profile.damage_penalty)); }
    if halved && !extra.is_empty() {
        // Engagement dice are added after Jab/style halving in attack resolution.
        expression.push_str(&format!("+{}", cache.roll20_extra_dice(nonpenetrating, &extra)?));
    }
    Ok(expression)
}

// Add dice, never multiply the weapon's flat term or Strength/mastery/material bonuses.
// Critical extra dice continue to use the original weapon pool.
fn roll_mounted_weapon_damage(
    weapon: &super::WeaponProfile,
    plan: MountedDamagePlan,
    use_jab: bool,
    shield: bool,
    average: bool,
    rng: &mut impl Rng,
) -> i32 {
    let (expr, cache) = if shield {
        match (&weapon.shield_damage_expr, &weapon.shield_damage_expr_cache) {
            (Some(expr), Some(cache)) => (expr.as_str(), cache),
            _ => return 0,
        }
    } else if use_jab {
        (weapon.damage_expr_for_attack(), weapon.damage_expr_cache_for_attack())
    } else {
        (weapon.damage_expr.as_str(), &weapon.damage_expr_cache)
    };
    let nonpenetrating = !shield && (use_jab || weapon.force_nonpenetrating_damage);
    if !plan.double_base_dice && plan.extra_smallest == 0 {
        return if average { average_damage_cache_rounded_down(cache, nonpenetrating) }
            else { cache.roll(rng, nonpenetrating) };
    }
    let pool = parse_damage_dice(expr, nonpenetrating, cache.d6_penetration_triggers(),
        cache.penetrate_on_max_minus_one());
    let extra = mounted_extra_dice_iter(&pool, plan);
    if average {
        return (cache.expected(nonpenetrating)
            + extra.map(expected_damage_die).sum::<f64>()).floor() as i32;
    }
    let mut result = cache.roll(rng, nonpenetrating);
    for die in extra {
        result += if die.penetrating && die.penetrate_on_max_minus_one {
            crate::core::rules::penetrating_roll_trigger_set(
                die.sides, &[(die.sides - 1).max(1), die.sides], rng)
        } else if let Some(triggers) = die.penetration_triggers {
            crate::core::rules::penetrating_roll_trigger_set(die.sides, triggers, rng)
        } else if die.penetrating {
            penetrating_roll(die.sides, rng)
        } else {
            roll_die(die.sides, rng)
        };
    }
    result
}

fn shield_block_raw_damage(
    weapon: &super::WeaponProfile,
    mounted_plan: MountedDamagePlan,
    strength_damage: i32,
    damage_penalty: i32,
    damage_multiplier: i32,
    average: bool,
    rng: &mut impl Rng,
) -> (i32, i32) {
    let Some(_) = weapon.shield_damage_expr_cache.as_ref() else {
        return (0, 0);
    };
    let rolled_damage = roll_mounted_weapon_damage(weapon, mounted_plan, false, true, average, rng);
    let mut raw = rolled_damage + strength_damage + damage_penalty;
    if raw < 0 {
        raw = 0;
    }
    raw = raw.saturating_mul(damage_multiplier.max(1));
    (rolled_damage, raw)
}

fn cached_damage_dice<'a>(
    cache: &'a mut WeaponCache,
    weapon: &super::types::WeaponProfile,
    use_jab: bool,
) -> &'a [DamageDie] {
    let slot = if use_jab {
        &mut cache.jab_damage_dice
    } else {
        &mut cache.damage_dice
    };
    if slot.is_none() {
        let expr = if use_jab {
            weapon
                .jab_special_expr
                .as_deref()
                .unwrap_or(weapon.damage_expr.as_str())
        } else {
            weapon.damage_expr.as_str()
        };
        *slot = Some(parse_damage_dice(
            expr,
            false,
            weapon.damage_expr_cache.d6_penetration_triggers(),
            weapon.damage_expr_cache.penetrate_on_max_minus_one(),
        ));
    }
    slot.as_deref().unwrap_or(&[])
}

fn roll_extra_damage_cached(
    cache: &mut WeaponCache,
    weapon: &super::types::WeaponProfile,
    use_jab: bool,
    dice: i32,
    force_nonpenetrating: bool,
    average: bool,
    rng: &mut impl Rng,
) -> i32 {
    let pool = cached_damage_dice(cache, weapon, use_jab);
    let sequence = extra_damage_dice_iter(pool, dice, force_nonpenetrating);
    if average {
        return sequence.map(expected_damage_die).sum::<f64>().floor() as i32;
    }
    let mut total = 0;
    for die in sequence {
        let value = if die.penetrating && die.penetrate_on_max_minus_one {
            crate::core::rules::penetrating_roll_trigger_set(
                die.sides,
                &[(die.sides - 1).max(1), die.sides],
                rng,
            )
        } else if let Some(triggers) = die.penetration_triggers {
            crate::core::rules::penetrating_roll_trigger_set(die.sides, triggers, rng)
        } else if die.penetrating {
            penetrating_roll(die.sides, rng)
        } else {
            roll_die(die.sides, rng)
        };
        total += value;
    }
    total
}

pub(crate) fn maybe_apply_trauma(
    combatants: &mut [Combatant],
    defender_idx: usize,
    damage: i32,
    rng: &mut impl Rng,
) -> Option<i32> {
    let pain_threshold = combatants[defender_idx].sheet.vitals.threshold_of_pain;
    if damage <= pain_threshold {
        return None;
    }
    let con_half = (combatants[defender_idx].sheet.vitals.constitution as i32) / 2;
    let trauma_die_sides = combatants[defender_idx]
        .sheet
        .vitals
        .trauma_die_sides
        .max(1);
    let trauma_penetrating = combatants[defender_idx].sheet.vitals.trauma_die_penetrating;
    let force_twenty = combatants[defender_idx].state.force_trauma_roll_20;
    if force_twenty {
        combatants[defender_idx].state.force_trauma_roll_20 = false;
    }
    let trauma_roll = if force_twenty {
        20
    } else if trauma_penetrating {
        penetrating_roll(trauma_die_sides, rng)
    } else {
        roll_die(trauma_die_sides, rng)
    };
    if trauma_roll <= con_half {
        return None;
    }
    let duration = if trauma_roll == 20 {
        roll_damage_expr("5d6p", rng, false) * 60
    } else {
        (trauma_roll - con_half) * 5
    };
    let duration = duration.max(1);
    let remaining = combatants[defender_idx].state.trauma_remaining_seconds;
    combatants[defender_idx].state.trauma_remaining_seconds = remaining.max(duration as i32);
    combatants[defender_idx].state.clear_attack_timers();
    Some(duration as i32)
}

fn apply_trauma_duration(combatants: &mut [Combatant], defender_idx: usize, duration: i32) -> i32 {
    let duration = duration.max(1);
    let remaining = combatants[defender_idx].state.trauma_remaining_seconds;
    let new_duration = remaining.max(duration);
    combatants[defender_idx].state.trauma_remaining_seconds = new_duration;
    combatants[defender_idx].state.clear_attack_timers();
    new_duration
}

fn feat_of_agility_succeeds(combatant: &Combatant, difficulty: i32, rng: &mut impl Rng) -> bool {
    let roll = penetrating_roll(20, rng);
    let total = roll + combatant.sheet.defense.feat_of_agility
        - combatant.sheet.defense.armor_feat_of_agility_penalty.max(0);
    total >= difficulty.max(0)
}

fn is_buckler_or_small_shield(name: &str) -> bool {
    let normalized = name.trim().to_ascii_lowercase();
    normalized == "buckler" || (normalized.starts_with("small ") && normalized.contains("shield"))
}

fn resolve_eyesmite(
    combatants: &mut [Combatant],
    attacker_idx: usize,
    defender_idx: usize,
    now: f32,
    rng: &mut impl Rng,
) -> CounterAttackOutcome {
    let drops_shield = combatants[attacker_idx].state.shield_intact
        && combatants[attacker_idx]
            .sheet
            .defense
            .shield_name
            .as_deref()
            .map(is_buckler_or_small_shield)
            .unwrap_or(false);
    if drops_shield {
        combatants[attacker_idx].state.shield_intact = false;
    }
    combatants[attacker_idx].state.has_attacked = true;

    let attack_bonus = combatants[attacker_idx].apply_i32(
        StatIdI32::AttackBonusBase,
        combatants[attacker_idx].sheet.offense.attack_bonus_base,
    ) + combatants[attacker_idx].apply_i32(StatIdI32::UnarmedAttackBonus, 0);
    let strength_damage = combatants[attacker_idx].apply_i32(
        StatIdI32::StrengthDamageBase,
        combatants[attacker_idx].sheet.offense.strength_damage_base,
    ) + combatants[attacker_idx].apply_i32(
        StatIdI32::UnarmedDamageBonus,
        combatants[attacker_idx].sheet.offense.unarmed_damage_bonus,
    );

    let defender_state = AttackStateSnapshot::capture(&combatants[defender_idx], attacker_idx);
    let defender = &combatants[defender_idx];
    let defender_infinite_hp = defender.sheet.vitals.infinite_hp;
    let defender_total_dr = defender.sheet.defense.armor_dr + defender.sheet.defense.natural_dr;
    let shield_active = defender_state.shield_intact;
    let defense_mod = defender.apply_i32(StatIdI32::DefenseMod, defender.sheet.defense.defense_mod)
        + mounted_defense_bonus(defender)
        + fight_defensively_defense_bonus(defender)
        - called_shot_defense_penalty(defender)
        + defender_state.movement_defense_bonus();
    let defense_ready = defender_state.defense_ready(&defender.sheet, now);
    let weapon_defense_bonus =
        if defender.sheet.offense.weapon.defense_bonus_always || defense_ready {
            4
        } else {
            0
        };
    let shield_defense_bonus = if shield_active {
        4 + defender.apply_i32(
            StatIdI32::ShieldDefenseBonus,
            defender.sheet.defense.shield_defense_bonus,
        )
    } else {
        0
    };
    let defense_sides = defense_die_sides(
        false,
        defender_state.moved_last_tick,
        shield_active,
        defender_state.trauma_remaining_seconds > 0
            || combatants[defender_idx]
                .state
                .magic
                .restricted_defense(combatants[defender_idx].magic.talents),
        defender
            .sheet
            .maneuvers
            .offensive_dualwielding_defense_penalty,
    );
    let attack_die = penetrating_roll(20, rng);
    let defense_die = penetrating_roll(defense_sides, rng);
    let attack_roll = attack_die + attack_bonus;
    let defense_roll = defense_die + defense_mod + weapon_defense_bonus + shield_defense_bonus;
    let eye_target = defense_roll
        + defender
            .sheet
            .maneuvers
            .called_shot_target_defense_bonus_base
            .max(1);
    let hit = attack_roll >= eye_target;
    let roll = AttackRollBreakdown {
        attack_die,
        defense_die,
        attack_bonus,
        range_mod: 0,
        defense_base: defense_mod,
        weapon_defense_bonus,
        shield_defense_bonus,
        attack_total: attack_roll,
        defense_total: defense_roll,
    };

    combatants[defender_idx].state.aggressive_defense_pending = false;
    let mut damage = 0;
    let mut trauma_seconds = None;
    let mut damage_breakdown = None;
    if hit {
        let rolled_damage = if combatants[defender_idx]
            .state
            .streamline_averages_incoming_damage
        {
            DamageExprCache::new("2d4p").expected(false).floor() as i32
        } else {
            roll_damage_expr("2d4p", rng, false)
        };
        let raw_damage = (rolled_damage + strength_damage).max(0);
        damage = raw_damage;
        combatants[attacker_idx].state.total_eyes_smote = combatants[attacker_idx]
            .state
            .total_eyes_smote
            .saturating_add(1);
        if !defender_infinite_hp {
            combatants[defender_idx].state.hp -= damage;
        }
        if damage > 0 {
            trauma_seconds = Some(apply_trauma_duration(
                combatants,
                defender_idx,
                damage.saturating_mul(10),
            ));
        }
        damage_breakdown = Some(DamageBreakdown {
            rolled_damage,
            strength_damage,
            raw_damage,
            armor_dr: defender_total_dr,
            armor_penetration: 0,
            effective_armor_dr: 0,
            final_damage: damage,
        });
    }
    super::magic::on_attack_resolved(
        combatants,
        attacker_idx,
        defender_idx,
        now,
        hit,
        damage,
        WeaponSlot::Primary,
        false,
        &roll,
    );
    update_regenstat_on_exchange(combatants, attacker_idx, defender_idx, hit);

    CounterAttackOutcome {
        damage_source: DamageSource::Ability { name: "Eyesmite".into() },
        attacker_idx,
        defender_idx,
        knockback_ft: 0.0,
        hit,
        shield_block: false,
        damage,
        shield_damage: 0,
        weapon_slot: WeaponSlot::Primary,
        use_jab: false,
        is_ranged: false,
        trauma_applied: trauma_seconds.is_some(),
        trauma_seconds,
        roll,
        damage_breakdown,
        shield_damage_breakdown: None,
        defender_hp_after: combatants[defender_idx].state.hp,
        critical: None,
        precognition_triggered: false,
    }
}

/// A ten-foot impact knocks the defender down, including shield hits and counters.
pub(crate) fn apply_knockback_recovery(
    defender: &mut Combatant,
    knockback_ft: f32,
    now: f32,
    weapon_speed: f32,
    hammerer: bool,
) {
    let knocked_down = knockback_ft >= 10.0;
    if knocked_down {
        defender.state.knockback_immobile_seconds =
            defender.state.knockback_immobile_seconds.max(1);
    }
    if knockback_ft > 0.0 && (knocked_down || hammerer) {
        let reset_time = now + weapon_speed.max(1.0);
        defender.state.set_next_attack_time(WeaponSlot::Primary, Some(reset_time));
        defender.state.set_next_attack_time(WeaponSlot::Secondary, Some(reset_time));
    }
}

fn resolve_counter_attack(
    combatants: &mut [Combatant],
    attacker_idx: usize,
    defender_idx: usize,
    now: f32,
    weapon_slot: WeaponSlot,
    use_weapon: bool,
    ignore_armor: bool,
    allow_critical: bool,
    force_critical: bool,
    superior_unarmed: bool,
    damage_multiplier: i32,
    rng: &mut impl Rng,
) -> CounterAttackOutcome {
    let knockback_rule = super::knockback_rule_for_attack(
        &combatants[attacker_idx], &combatants[defender_idx],
        use_weapon.then_some(weapon_slot), false, false,
    );
    combatants[attacker_idx].state.has_attacked = true;
    let tactical_attack_penalty = combatants[attacker_idx]
        .state
        .tactical_next_attack_penalty
        .max(0);
    combatants[attacker_idx].state.tactical_next_attack_penalty = 0;
    let defender_state = AttackStateSnapshot::capture(&combatants[defender_idx], attacker_idx);
    let average_damage = combatants[defender_idx]
        .state
        .streamline_averages_incoming_damage;
    let defender = &combatants[defender_idx];
    let defender_infinite_hp = defender.sheet.vitals.infinite_hp;
    let damage_multiplier = damage_multiplier.max(1);
    let attacker_hammerer = combatants[attacker_idx].apply_i32(StatIdI32::FlagHammererStyle, 0) > 0;
    let attacker_hobbler = combatants[attacker_idx].apply_i32(StatIdI32::FlagHobblerStyle, 0) > 0;
    let attacker_falling_sun =
        combatants[attacker_idx].apply_i32(StatIdI32::FlagFallingSunStyle, 0) > 0;
    let attacker_three_mountains =
        combatants[attacker_idx].apply_i32(StatIdI32::FlagThreeMountainsStyle, 0) > 0;
    let attacker_regenstat_bonus = if regenstat_active(combatants, attacker_idx) {
        regenstat_stack_from_state(&combatants[attacker_idx].state)
    } else {
        0
    };
    let defender_regenstat_bonus = if regenstat_active(combatants, defender_idx) {
        defender_state
            .regenstat_stacks
            .clamp(0, REGENSTAT_STACK_CAP)
    } else {
        0
    };
    let (
        attack_bonus,
        strength_damage,
        armor_penetration,
        crit_min_roll,
        crit_severity,
        attacker_weapon_hacking_or_piercing,
        damage_penalty,
        weapon_profile,
        unarmed_expr,
    ) = if use_weapon {
        let profile = {
            let attacker = &combatants[attacker_idx];
            attack_profile_for_slot(attacker, weapon_slot)
                .expect("weapon slot missing for counter attack")
        };
        let crit_min_roll = {
            let attacker = &combatants[attacker_idx];
            attacker.apply_i32(StatIdI32::CritMinRoll, profile.weapon.crit_min_roll)
        };
        let crit_severity = {
            let attacker = &combatants[attacker_idx];
            attacker.apply_i32(
                StatIdI32::CritSeverityBonus,
                profile.weapon.crit_severity_bonus,
            )
        };
        (
            profile.attack_bonus,
            profile.strength_damage,
            profile.armor_penetration,
            crit_min_roll,
            crit_severity,
            profile.weapon.hacking_or_piercing,
            profile.damage_penalty,
            Some(profile.weapon),
            None,
        )
    } else {
        let damage_expr = if superior_unarmed {
            "(d4p)+(d4p)"
        } else {
            "(d4p-2)+(d4p-2)"
        };
        let (attack_bonus, strength_damage_base, unarmed_damage_bonus) = {
            let attacker = &combatants[attacker_idx];
            (
                attacker.apply_i32(
                    StatIdI32::AttackBonusBase,
                    attacker.sheet.offense.attack_bonus_base,
                ) + attacker.apply_i32(StatIdI32::UnarmedAttackBonus, 0),
                attacker.apply_i32(
                    StatIdI32::StrengthDamageBase,
                    attacker.sheet.offense.strength_damage_base,
                ),
                attacker.apply_i32(
                    StatIdI32::UnarmedDamageBonus,
                    attacker.sheet.offense.unarmed_damage_bonus,
                ),
            )
        };
        (
            attack_bonus,
            strength_damage_base + unarmed_damage_bonus,
            0,
            20,
            0,
            false,
            0,
            None,
            Some(damage_expr),
        )
    };
    let damage_source = weapon_profile.as_ref().map_or_else(
        || DamageSource::Unarmed { name: "Near-perfect defense punch".into() },
        |weapon| DamageSource::weapon(weapon, weapon_slot),
    );
    let unarmed_expr = unarmed_expr.unwrap_or("d4p");
    let mounted_plan = weapon_profile.as_ref().map(|weapon| mounted_damage_plan(
        &combatants[attacker_idx], &combatants[defender_idx], weapon, false,
    )).unwrap_or_default();
    let armor_penetration = style_armor_penetration(
        &combatants[attacker_idx],
        &combatants[defender_idx],
        armor_penetration,
    );
    let attacker_fight_defensively_penalty =
        fight_defensively_attack_penalty(&combatants[attacker_idx]);
    let defender_fight_defensively_bonus = fight_defensively_defense_bonus(defender);
    let defender_called_shot_penalty = called_shot_defense_penalty(defender);

    let (
        defense_mod,
        armor_dr,
        natural_dr,
        armor_is_heavy,
        shield_active,
        shield_defense_bonus,
        shield_dr,
        shield_breakage,
        trauma_incapacitated,
        defender_weapon_defense_always,
        defender_weapon_speed,
        defender_defiant,
        defender_crit_severity_reduction,
        defender_halves_crit_extra_damage,
        defender_ignore_ancillary_crit_effects,
        defender_six_paths,
        defender_unbreakable_wall,
        defender_falling_sun,
    ) = {
        (
            defender.apply_i32(StatIdI32::DefenseMod, defender.sheet.defense.defense_mod)
                + mounted_defense_bonus(defender)
                + defender_regenstat_bonus
                + defender_fight_defensively_bonus
                - defender_called_shot_penalty
                + defender_state.movement_defense_bonus(),
            defender.apply_i32(StatIdI32::ArmorDr, defender.sheet.defense.armor_dr),
            defender.apply_i32(StatIdI32::NaturalDr, defender.sheet.defense.natural_dr),
            defender.sheet.defense.armor_is_heavy,
            defender_state.shield_intact,
            defender.apply_i32(
                StatIdI32::ShieldDefenseBonus,
                defender.sheet.defense.shield_defense_bonus,
            ),
            defender.apply_i32(StatIdI32::ShieldDr, defender.sheet.defense.shield_dr),
            defender.sheet.defense.shield_breakage,
            defender_state.trauma_remaining_seconds > 0
                || combatants[defender_idx]
                    .state
                    .magic
                    .restricted_defense(combatants[defender_idx].magic.talents),
            defender.sheet.offense.weapon.defense_bonus_always,
            defender.apply_f32(StatIdF32::WeaponSpeed, defender.sheet.offense.weapon.speed),
            defender.apply_i32(StatIdI32::FlagDefiant, 0) > 0,
            defender
                .apply_i32(StatIdI32::IncomingCritSeverityReduction, 0)
                .max(0),
            defender.apply_i32(StatIdI32::FlagIncomingCritExtraDamageHalved, 0) > 0,
            defender.apply_i32(StatIdI32::FlagIgnoreAncillaryCritEffects, 0) > 0,
            defender.apply_i32(StatIdI32::FlagSixPathsStyle, 0) > 0,
            defender.apply_i32(StatIdI32::FlagUnbreakableWallStyle, 0) > 0,
            defender.apply_i32(StatIdI32::FlagFallingSunStyle, 0) > 0,
        )
    };
    let defense_ready = defender_state.defense_ready(&combatants[defender_idx].sheet, now);
    let weapon_defense_bonus = if defender_weapon_defense_always || defense_ready {
        4
    } else {
        0
    };
    let shield_defense_bonus = if shield_active {
        4 + shield_defense_bonus
    } else {
        0
    };

    let (attack_die, attack_first) = roll_attack_or_defense_d20(attacker_falling_sun, rng);
    let defense_sides = defense_die_sides(
        false,
        defender_state.moved_last_tick,
        shield_active,
        trauma_incapacitated
            || combatants[defender_idx]
                .state
                .magic
                .restricted_defense(combatants[defender_idx].magic.talents),
        defender
            .sheet
            .maneuvers
            .offensive_dualwielding_defense_penalty,
    );
    let (defense_die, defense_first) = if defense_sides == 20 {
        roll_attack_or_defense_d20(defender_falling_sun, rng)
    } else {
        penetrating_roll_with_first(defense_sides, rng)
    };
    let attack_bonus_total = attack_bonus + attacker_regenstat_bonus
        - attacker_fight_defensively_penalty
        - tactical_attack_penalty;
    let attack_roll = attack_die + attack_bonus_total;
    let defense_roll = defense_die + defense_mod + weapon_defense_bonus + shield_defense_bonus;
    let roll = AttackRollBreakdown {
        attack_die,
        defense_die,
        attack_bonus: attack_bonus_total,
        range_mod: 0,
        defense_base: defense_mod,
        weapon_defense_bonus,
        shield_defense_bonus,
        attack_total: attack_roll,
        defense_total: defense_roll,
    };

    let mut hit = attack_roll >= defense_roll;
    if attack_first == 20 {
        if defense_first == 20 && defense_roll > attack_roll {
            hit = false;
        } else {
            hit = true;
        }
    }
    let precognition_triggered = hit
        && defender.sheet.defense.precognition
        && combatants[defender_idx].state.precognition_space_available
        && feat_of_agility_succeeds(defender, attack_roll - defense_roll, rng);

    combatants[defender_idx].state.aggressive_defense_pending = false;
    let mut damage = 0;
    let mut shield_block = false;
    let mut shield_damage = 0;
    let mut shield_broken = false;
    let mut knockback_ft = 0.0;
    let mut trauma_seconds = None;
    let mut damage_breakdown = None;
    let mut shield_damage_breakdown = None;
    let mut critical = None;
    let crit_trigger = force_critical || (allow_critical && attack_first >= crit_min_roll);
    let reaper_multiplier =
        reaper_extra_dice_multiplier(&combatants[attacker_idx], weapon_profile.as_deref());

    if hit {
        let roll = |rng: &mut _| {
            if let Some(weapon) = weapon_profile.as_ref() {
                roll_mounted_weapon_damage(weapon, mounted_plan, false, false, average_damage, rng)
            } else if average_damage {
                DamageExprCache::new(unarmed_expr).expected(false).floor() as i32
            } else {
                roll_damage_expr(unarmed_expr, rng, false)
            }
        };
        let mut rolled_damage = roll(rng);
        if !average_damage && crit_trigger && defender_defiant {
            rolled_damage = rolled_damage.min(roll(rng));
        }
        let mut raw = rolled_damage + strength_damage + damage_penalty;
        if use_weapon
            && weapon_profile
                .as_ref()
                .map(|weapon| weapon.halve_damage)
                .unwrap_or(false)
        {
            raw /= 2;
        }
        raw = raw.saturating_mul(damage_multiplier);
        if raw < 0 {
            raw = 0;
        }
        let raw_base = raw;
        let defender_hp_before = combatants[defender_idx].state.hp;
        let weapon_ignores_all_dr = use_weapon
            && weapon_profile
                .as_ref()
                .map(|weapon| weapon.ignore_all_dr)
                .unwrap_or(false);
        let mut effective_dr = if weapon_ignores_all_dr {
            0
        } else if ignore_armor {
            natural_dr.max(0)
        } else if armor_dr >= 5 || armor_is_heavy {
            (armor_dr - armor_penetration).max(0)
        } else {
            armor_dr.max(0)
        };
        let mut crit_trauma_seconds = None;
        if crit_trigger || attacker_hobbler {
            let severity = (attack_roll - defense_roll + raw_base - effective_dr + crit_severity
                - defender_crit_severity_reduction)
                .max(1);
            let mut effect = apply_ancillary_critical_immunity(
                critical_effect_for(severity),
                crit_trigger
                    && attacker_weapon_hacking_or_piercing
                    && defender_ignore_ancillary_crit_effects,
            );
            effect.extra_dice *= reaper_multiplier;
            if effect.instant_kill {
                critical = Some(CriticalHit {
                    severity: effect.severity,
                    extra_dice: if attacker_hobbler {
                        0
                    } else {
                        effect.extra_dice
                    },
                    extra_damage: 0,
                    speed_reset: effect.speed_reset,
                    trauma_seconds: None,
                    instant_kill: true,
                });
            } else {
                let extra_dice = if attacker_hobbler {
                    0
                } else {
                    effect.extra_dice
                };
                let extra_damage = if extra_dice > 0 {
                    if let Some(weapon_profile) = weapon_profile.as_ref() {
                        let cache = combatants[attacker_idx].state.weapon_cache_mut(weapon_slot);
                        let rolled = roll_extra_damage_cached(
                            cache,
                            weapon_profile.as_ref(),
                            false,
                            extra_dice,
                            weapon_profile.force_nonpenetrating_damage,
                            average_damage,
                            rng,
                        );
                        if defender_halves_crit_extra_damage {
                            rolled / 2
                        } else {
                            rolled
                        }
                    } else {
                        let rolled =
                            roll_extra_damage(unarmed_expr, extra_dice, false, average_damage, rng);
                        if defender_halves_crit_extra_damage {
                            rolled / 2
                        } else {
                            rolled
                        }
                    }
                } else {
                    0
                };
                raw += extra_damage;
                effective_dr = if weapon_ignores_all_dr {
                    0
                } else if ignore_armor {
                    natural_dr.max(0)
                } else if armor_dr >= 5 || armor_is_heavy {
                    (armor_dr - armor_penetration).max(0)
                } else {
                    armor_dr.max(0)
                };
                if effect.auto_trauma {
                    let forced = roll_damage_expr("5d6p", rng, false) * 60;
                    let applied = apply_trauma_duration(combatants, defender_idx, forced);
                    crit_trauma_seconds = Some(applied);
                }
                if effect.speed_reset && crit_trauma_seconds.is_none() {
                    let reset_time = now + defender_weapon_speed.max(1.0);
                    combatants[defender_idx]
                        .state
                        .set_next_attack_time(WeaponSlot::Primary, Some(reset_time));
                    combatants[defender_idx]
                        .state
                        .set_next_attack_time(WeaponSlot::Secondary, Some(reset_time));
                }
                critical = Some(CriticalHit {
                    severity: effect.severity,
                    extra_dice: extra_dice,
                    extra_damage: extra_damage,
                    speed_reset: effect.speed_reset,
                    trauma_seconds: crit_trauma_seconds,
                    instant_kill: false,
                });
            }
        }

        if critical
            .as_ref()
            .map(|crit| crit.instant_kill)
            .unwrap_or(false)
            && !defender_infinite_hp
        {
            combatants[defender_idx].state.hp = 0;
            damage = defender_hp_before.max(0);
            trauma_seconds = None;
        } else {
            damage = (raw - effective_dr).max(0);
            if damage > 0 {
                damage += weapon_profile
                    .as_ref()
                    .map(|weapon| weapon.internal_hemorrhage_damage.max(0))
                    .unwrap_or(0);
            }
            if precognition_triggered {
                damage /= 2;
            }
            if !defender_infinite_hp {
                combatants[defender_idx].state.hp -= damage;
            }
            knockback_ft = knockback_rule.distance_ft(raw);
            trauma_seconds = maybe_apply_trauma(combatants, defender_idx, damage, rng);
            if let Some(crit) = critical.as_mut() {
                if let Some(crit_seconds) = crit.trauma_seconds {
                    trauma_seconds =
                        Some(trauma_seconds.map_or(crit_seconds, |base| base.max(crit_seconds)));
                    crit.trauma_seconds = trauma_seconds;
                }
            }
        }
        damage_breakdown = Some(DamageBreakdown {
            rolled_damage,
            strength_damage,
            raw_damage: raw,
            armor_dr: if weapon_ignores_all_dr {
                0
            } else if ignore_armor {
                natural_dr
            } else {
                armor_dr
            },
            armor_penetration: if ignore_armor || weapon_ignores_all_dr {
                0
            } else {
                armor_penetration
            },
            effective_armor_dr: effective_dr,
            final_damage: damage,
        });
    } else if shield_active {
        let miss_margin = defense_roll - attack_roll;
        let shield_block_window = if defender_six_paths {
            SIX_PATHS_SHIELD_BLOCK_WINDOW
        } else {
            DEFAULT_SHIELD_BLOCK_WINDOW
        };
        if miss_margin < shield_block_window {
            shield_block = true;
            let (rolled_damage, raw) = if use_weapon {
                let weapon = weapon_profile.as_ref().expect("weapon profile missing");
                shield_block_raw_damage(
                    weapon,
                    mounted_plan,
                    strength_damage,
                    damage_penalty,
                    damage_multiplier,
                    average_damage,
                    rng,
                )
            } else {
                // Unarmed has no dedicated shield-damage expression, so shield hits do zero.
                (0, 0)
            };
            shield_damage = raw;
            knockback_ft = knockback_rule.distance_ft(raw);
            let effective_shield_dr = if ignore_armor { 0 } else { shield_dr };
            let shield_after_dr = (raw - effective_shield_dr).max(0);
            let effective_dr = if ignore_armor {
                natural_dr.max(0)
            } else if armor_dr >= 5 || armor_is_heavy {
                (armor_dr - armor_penetration).max(0)
            } else {
                armor_dr.max(0)
            };
            let hp_damage = (shield_after_dr - effective_dr).max(0);
            if hp_damage > 0 {
                if !defender_infinite_hp {
                    combatants[defender_idx].state.hp -= hp_damage;
                }
                trauma_seconds = maybe_apply_trauma(combatants, defender_idx, hp_damage, rng);
            }
            let breakage_raw = if defender_unbreakable_wall {
                (raw - effective_shield_dr).max(0)
            } else {
                raw
            };
            if let Some(steps) = shield_breakage {
                if breakage_raw >= steps[3].threshold {
                    shield_broken = true;
                } else if breakage_raw >= steps[2].threshold {
                    shield_broken = breakage_roll(steps[2], rng);
                } else if breakage_raw >= steps[1].threshold {
                    shield_broken = breakage_roll(steps[1], rng);
                } else if breakage_raw >= steps[0].threshold {
                    shield_broken = breakage_roll(steps[0], rng);
                }
            }
            if shield_broken {
                combatants[defender_idx].state.shield_intact = false;
            }
            damage = hp_damage;
            shield_damage_breakdown = Some(ShieldDamageBreakdown {
                rolled_damage,
                strength_damage,
                raw_damage: raw,
                shield_dr: effective_shield_dr,
                armor_dr: if ignore_armor { natural_dr } else { armor_dr },
                armor_penetration: if ignore_armor { 0 } else { armor_penetration },
                effective_armor_dr: effective_dr,
                hp_damage,
                shield_broken,
            });
        }
    }

    super::magic::on_attack_resolved(
        combatants,
        attacker_idx,
        defender_idx,
        now,
        hit,
        damage,
        weapon_slot,
        false,
        &roll,
    );
    let defender_hp_after = combatants[defender_idx].state.hp;
    update_regenstat_on_exchange(combatants, attacker_idx, defender_idx, hit);
    if hit && attacker_three_mountains {
        combatants[attacker_idx].state.three_mountains_hit_streak += 1;
        if combatants[attacker_idx].state.three_mountains_hit_streak >= 3 {
            combatants[defender_idx].state.force_trauma_roll_20 = true;
        }
    } else if attacker_three_mountains {
        combatants[attacker_idx].state.three_mountains_hit_streak = 0;
    }
    if hit || shield_block {
        apply_knockback_recovery(
            &mut combatants[defender_idx],
            knockback_ft,
            now,
            defender_weapon_speed,
            attacker_hammerer,
        );
    }
    let trauma_applied = trauma_seconds.is_some();
    CounterAttackOutcome {
        damage_source,
        attacker_idx,
        defender_idx,
        knockback_ft,
        hit,
        shield_block,
        damage,
        shield_damage,
        weapon_slot,
        use_jab: false,
        is_ranged: false,
        trauma_applied,
        trauma_seconds,
        roll,
        damage_breakdown,
        shield_damage_breakdown,
        defender_hp_after,
        critical,
        precognition_triggered,
    }
}

pub(crate) fn resolve_attack(
    combatants: &mut [Combatant],
    attacker_idx: usize,
    defender_idx: usize,
    range_mod: i32,
    is_ranged: bool,
    distance_ft: f32,
    attack_mode: AttackMode,
    weapon_slot: WeaponSlot,
    now: f32,
    state_snapshot: Option<&CombatRoundSnapshot>,
    rng: &mut impl Rng,
) -> AttackOutcome {
    let is_aggressive = aggressive_attack_available(&combatants[attacker_idx], attack_mode, is_ranged)
        && attack_profile_for_slot(&combatants[attacker_idx], weapon_slot)
            .is_some_and(|profile| !profile.use_jab);
    let halve_aggressive_damage = is_aggressive
        && aggressive_retreat_halves_damage(&combatants[attacker_idx], &combatants[defender_idx]);
    let tactical_attack_penalty = combatants[attacker_idx]
        .state
        .tactical_next_attack_penalty
        .max(0);
    combatants[attacker_idx].state.tactical_next_attack_penalty = 0;
    let defender_state = state_snapshot
        .and_then(|snapshot| snapshot.get(defender_idx))
        .copied()
        .unwrap_or_else(|| AttackStateSnapshot::capture(&combatants[defender_idx], attacker_idx));
    let attacker_state = state_snapshot
        .and_then(|snapshot| snapshot.get(attacker_idx))
        .copied()
        .unwrap_or_else(|| AttackStateSnapshot::capture(&combatants[attacker_idx], defender_idx));
    let average_damage = combatants[defender_idx]
        .state
        .streamline_averages_incoming_damage;
    let defender_infinite_hp = combatants[defender_idx].sheet.vitals.infinite_hp;
    let attack_profile = {
        let attacker = &combatants[attacker_idx];
        attack_profile_for_slot(attacker, weapon_slot).expect("weapon slot missing for attack")
    };
    let damage_source = DamageSource::weapon(&attack_profile.weapon, weapon_slot);
    let knockback_rule = super::knockback_rule_for_attack(
        &combatants[attacker_idx], &combatants[defender_idx], Some(weapon_slot),
        is_ranged, attack_mode == AttackMode::Charge,
    );
    let attacker_hammerer = combatants[attacker_idx].apply_i32(StatIdI32::FlagHammererStyle, 0) > 0;
    let attacker_hobbler = combatants[attacker_idx].apply_i32(StatIdI32::FlagHobblerStyle, 0) > 0;
    let attacker_falling_sun =
        combatants[attacker_idx].apply_i32(StatIdI32::FlagFallingSunStyle, 0) > 0;
    let attacker_three_mountains =
        combatants[attacker_idx].apply_i32(StatIdI32::FlagThreeMountainsStyle, 0) > 0;
    let attacker_returner = combatants[attacker_idx].apply_i32(StatIdI32::FlagReturnerStyle, 0) > 0;
    let defender_returner = combatants[defender_idx].apply_i32(StatIdI32::FlagReturnerStyle, 0) > 0;
    let attacker_armeroci =
        combatants[attacker_idx].apply_i32(StatIdI32::FlagArmerociPoleStyle, 0) > 0;
    let attacker_regenstat_bonus = if regenstat_active(combatants, attacker_idx) {
        attacker_state
            .regenstat_stacks
            .clamp(0, REGENSTAT_STACK_CAP)
    } else {
        0
    };
    let defender_regenstat_bonus = if regenstat_active(combatants, defender_idx) {
        defender_state
            .regenstat_stacks
            .clamp(0, REGENSTAT_STACK_CAP)
    } else {
        0
    };
    if attacker_returner {
        combatants[attacker_idx].state.returner_counter_available = true;
    }

    let forgo_opening_returner = attacker_returner
        && !is_ranged
        && attack_mode == AttackMode::Normal
        && weapon_slot == WeaponSlot::Primary
        && combatants[attacker_idx].state.returner_skip_opening_attack;
    if forgo_opening_returner {
        combatants[attacker_idx].state.returner_skip_opening_attack = false;
        combatants[attacker_idx].state.returner_double_counter_ready = true;
        if attacker_three_mountains {
            combatants[attacker_idx].state.three_mountains_hit_streak = 0;
        }
        if regenstat_active(combatants, attacker_idx) {
            combatants[attacker_idx].state.regenstat_stacks = 0;
        }
        let roll = AttackRollBreakdown {
            attack_die: 0,
            defense_die: 0,
            attack_bonus: attack_profile.attack_bonus + attacker_regenstat_bonus,
            range_mod,
            defense_base: 0,
            weapon_defense_bonus: 0,
            shield_defense_bonus: 0,
            attack_total: 0,
            defense_total: 0,
        };
        return AttackOutcome {
            is_aggressive: false,
            damage_source,
            attacker_idx,
            defender_idx,
            knockback_ft: 0.0,
            hit: false,
            shield_block: false,
            damage: 0,
            shield_damage: 0,
            hold_at_bay: attack_mode == AttackMode::HoldAtBay,
            weapon_slot,
            use_jab: attack_profile.use_jab,
            is_ranged,
            trauma_applied: false,
            trauma_seconds: None,
            roll,
            damage_breakdown: None,
            shield_damage_breakdown: None,
            defender_hp_after: combatants[defender_idx].state.hp,
            critical: None,
            precognition_triggered: false,
            counter_attack: None,
            additional_counters: Vec::new(),
        };
    }
    combatants[attacker_idx].state.has_attacked = true;
    combatants[attacker_idx].state.aggressive_maneuvers_locked = is_aggressive;
    if is_aggressive {
        combatants[attacker_idx].state.aggressive_defense_pending = true;
    }
    let defender_initial_attack_bonus = if combatants[defender_idx]
        .sheet
        .maneuvers
        .called_shot_deceptive_defender
    {
        let seen_in_snapshot = defender_state.deceptive_defender_seen_attacker;
        let seen_in_live = combatants[defender_idx]
            .state
            .deceptive_defender_seen_attackers
            .contains(&attacker_idx);
        if seen_in_snapshot || seen_in_live {
            0
        } else {
            1
        }
    } else {
        0
    };
    if combatants[defender_idx]
        .sheet
        .maneuvers
        .called_shot_deceptive_defender
        && !combatants[defender_idx]
            .state
            .deceptive_defender_seen_attackers
            .contains(&attacker_idx)
    {
        combatants[defender_idx]
            .state
            .deceptive_defender_seen_attackers
            .push(attacker_idx);
    }
    let defender_fight_defensively_bonus =
        fight_defensively_defense_bonus(&combatants[defender_idx]);
    let attacker_called_shot = called_shot_active(&combatants[attacker_idx]);
    let attacker_called_shot_precision_bonus =
        called_shot_precision_target_bonus(&combatants[attacker_idx], &combatants[defender_idx]);
    let defender_called_shot_bonus = if attacker_called_shot
        && combatants[defender_idx]
            .sheet
            .maneuvers
            .called_shot_deceptive_defender
    {
        DECEPTIVE_DEFENDER_CALLED_SHOT_DEFENSE_BONUS
    } else {
        0
    };
    let mut attack_bonus = standard_attack_bonus(&combatants[attacker_idx], &attack_profile);
    if is_aggressive {
        attack_bonus += aggressive_attack_bonus(&combatants[attacker_idx]);
    }
    if attack_mode == AttackMode::Charge && !combatants[attacker_idx].sheet.maneuvers.mounted {
        attack_bonus += CHARGE_ATTACK_BONUS;
    }
    attack_bonus += attacker_regenstat_bonus;
    attack_bonus -= tactical_attack_penalty;
    let strength_damage = attack_damage_bonus(&attack_profile, is_ranged);
    let armor_penetration = style_armor_penetration(
        &combatants[attacker_idx],
        &combatants[defender_idx],
        attack_profile.armor_penetration,
    );
    let use_jab = attack_profile.use_jab;
    let damage_penalty = attack_profile.damage_penalty;
    let weapon = attack_profile.weapon;
    let mounted_plan = mounted_damage_plan(
        &combatants[attacker_idx],
        &combatants[defender_idx],
        &weapon,
        is_ranged,
    );
    let reaper_multiplier = reaper_extra_dice_multiplier(&combatants[attacker_idx], Some(&weapon));
    let movement_ranged_distance = defender_state.movement_ranged_distance;
    let chronoblur_melee_bonus = if is_ranged {
        0
    } else {
        defender_state.movement_defense_bonus()
    };
    let (range_mod, chronoblur_out_of_range) = if is_ranged && movement_ranged_distance > 0.0 {
        let range_scale = combatants[attacker_idx].apply_f32(
            StatIdF32::RangeDistanceMultiplier,
            weapon.range_distance_multiplier,
        );
        match range_modifier_for_weapon_with_scale(
            weapon.as_ref(),
            distance_ft + movement_ranged_distance,
            range_scale,
        ) {
            Some(modifier) => (modifier, false),
            None => (range_mod.min(-8), true),
        }
    } else {
        (range_mod, false)
    };
    let (
        defense_mod,
        ranged_defense_mod,
        armor_dr,
        natural_dr,
        armor_is_heavy,
        shield_active,
        shield_defense_bonus,
        shield_cover_value,
        shield_dr,
        shield_breakage,
        trauma_incapacitated,
        defender_weapon_defense_always,
        defender_weapon_speed,
        defender_defiant,
        defender_crit_severity_reduction,
        defender_halves_crit_extra_damage,
        defender_ignore_ancillary_crit_effects,
        defender_superior_defense,
        defender_edge_counter,
        defender_six_paths,
        defender_unbreakable_wall,
        defender_falling_sun,
    ) = {
        let defender = &combatants[defender_idx];
        (
            base_defense_modifier(defender, false)
                + defender_regenstat_bonus
                + defender_initial_attack_bonus
                + chronoblur_melee_bonus,
            base_defense_modifier(defender, true) + defender_regenstat_bonus
                + defender_initial_attack_bonus,
            defender.apply_i32(StatIdI32::ArmorDr, defender.sheet.defense.armor_dr),
            defender.apply_i32(StatIdI32::NaturalDr, defender.sheet.defense.natural_dr),
            defender.sheet.defense.armor_is_heavy,
            defender_state.shield_intact,
            defender.apply_i32(
                StatIdI32::ShieldDefenseBonus,
                defender.sheet.defense.shield_defense_bonus,
            ),
            defender
                .sheet
                .defense
                .shield_cover_value
                .map(|value| defender.apply_i32(StatIdI32::ShieldCoverValue, value)),
            defender.apply_i32(StatIdI32::ShieldDr, defender.sheet.defense.shield_dr),
            defender.sheet.defense.shield_breakage,
            defender_state.trauma_remaining_seconds > 0
                || combatants[defender_idx]
                    .state
                    .magic
                    .restricted_defense(combatants[defender_idx].magic.talents),
            defender.sheet.offense.weapon.defense_bonus_always,
            defender.apply_f32(StatIdF32::WeaponSpeed, defender.sheet.offense.weapon.speed),
            defender.apply_i32(StatIdI32::FlagDefiant, 0) > 0,
            defender
                .apply_i32(StatIdI32::IncomingCritSeverityReduction, 0)
                .max(0),
            defender.apply_i32(StatIdI32::FlagIncomingCritExtraDamageHalved, 0) > 0,
            defender.apply_i32(StatIdI32::FlagIgnoreAncillaryCritEffects, 0) > 0,
            defender.apply_i32(StatIdI32::FlagSuperiorDefense, 0) > 0,
            defender.apply_i32(StatIdI32::FlagEdgeCounter, 0) > 0,
            defender.apply_i32(StatIdI32::FlagSixPathsStyle, 0) > 0,
            defender.apply_i32(StatIdI32::FlagUnbreakableWallStyle, 0) > 0,
            defender.apply_i32(StatIdI32::FlagFallingSunStyle, 0) > 0,
        )
    };
    let defense_ready = if is_ranged {
        false
    } else {
        defender_state.defense_ready(&combatants[defender_idx].sheet, now)
    };
    let weapon_defense_bonus = if is_ranged {
        0
    } else if defender_weapon_defense_always || defense_ready {
        4
    } else {
        0
    };
    let shield_defense_bonus = if shield_active {
        let base = if is_ranged { 0 } else { 4 };
        base + shield_defense_bonus
    } else {
        0
    };

    let (attack_die, attack_first) = roll_attack_or_defense_d20(attacker_falling_sun, rng);
    let defense_sides = defense_die_sides(
        is_ranged,
        defender_state.moved_last_tick,
        shield_active,
        trauma_incapacitated
            || combatants[defender_idx]
                .state
                .magic
                .restricted_defense(combatants[defender_idx].magic.talents),
        combatants[defender_idx]
            .sheet
            .maneuvers
            .offensive_dualwielding_defense_penalty,
    );
    let (defense_die, defense_first) = if defense_sides == 20 {
        roll_attack_or_defense_d20(defender_falling_sun, rng)
    } else {
        penetrating_roll_with_first(defense_sides, rng)
    };
    let mut attack_roll = attack_die + attack_bonus + range_mod;
    let mut use_shield_for_ranged = false;
    let (defense_mod_used, shield_defense_bonus_used) = if is_ranged {
        let (base, shield, use_shield) = ranged_defense_choice(ranged_defense_mod,
            shield_defense_bonus, shield_active, defender_fight_defensively_bonus);
        use_shield_for_ranged = use_shield;
        (base, shield)
    } else {
        (
            defense_mod + defender_fight_defensively_bonus,
            shield_defense_bonus,
        )
    };
    // Mount movement protects the rider with either ranged defense option and in melee.
    let defense_mod_used = defense_mod_used + mounted_defense_bonus(&combatants[defender_idx]);
    if is_ranged && use_shield_for_ranged {
        if let Some(cap) = shield_cover_value {
            attack_roll = attack_roll.min(cap);
        }
    }
    let defense_roll = defense_die
        + defense_mod_used
        + weapon_defense_bonus
        + shield_defense_bonus_used
        + defender_called_shot_bonus;
    let called_shot_precision_target = if attacker_called_shot {
        defense_roll + attacker_called_shot_precision_bonus
    } else {
        defense_roll
    };
    let roll = AttackRollBreakdown {
        attack_die,
        defense_die,
        attack_bonus,
        range_mod,
        defense_base: defense_mod_used,
        weapon_defense_bonus,
        shield_defense_bonus: shield_defense_bonus_used,
        attack_total: attack_roll,
        defense_total: defense_roll,
    };
    combatants[defender_idx].state.aggressive_defense_pending = false;
    let mut damage = 0;
    let mut hit = false;
    let mut shield_block = false;
    let mut shield_damage = 0;
    let mut shield_broken = false;
    let mut knockback_ft = 0.0;
    let mut trauma_seconds = None;
    let mut damage_breakdown = None;
    let mut shield_damage_breakdown = None;
    let mut critical = None;
    let mut crit_min_roll = {
        let attacker = &combatants[attacker_idx];
        attacker.apply_i32(StatIdI32::CritMinRoll, weapon.crit_min_roll)
    };
    if is_ranged {
        if let Some(ranged_min) = weapon.crit_min_roll_ranged {
            let ranged_min = {
                let attacker = &combatants[attacker_idx];
                attacker.apply_i32(StatIdI32::CritMinRoll, ranged_min)
            };
            crit_min_roll = crit_min_roll.min(ranged_min);
        }
    }
    let crit_trigger = attack_first >= crit_min_roll;
    let mut counter_attack = None;

    let mut attack_hits = if attacker_called_shot {
        attack_roll > defense_roll
    } else {
        attack_roll >= defense_roll
    };
    let mut called_shot_precise_hit =
        attacker_called_shot && attack_roll >= called_shot_precision_target;
    if crit_trigger && defense_first == 20 {
        if defense_roll > attack_roll {
            attack_hits = false;
            called_shot_precise_hit = false;
        } else if attack_roll > defense_roll {
            attack_hits = true;
            called_shot_precise_hit =
                attacker_called_shot && attack_roll >= called_shot_precision_target;
        }
    }
    if chronoblur_out_of_range {
        attack_hits = false;
        called_shot_precise_hit = false;
    }
    if attack_hits && is_ranged && combatants[defender_idx].sheet.defense.prescience {
        let passive_defense_roll =
            penetrating_roll(20, rng) + defense_mod + defender_fight_defensively_bonus;
        let difficulty = attack_roll - passive_defense_roll;
        if feat_of_agility_succeeds(&combatants[defender_idx], difficulty, rng) {
            attack_hits = false;
            called_shot_precise_hit = false;
        }
    }
    let crit_trigger = crit_trigger
        || (called_shot_precise_hit
            && combatants[attacker_idx].apply_i32(StatIdI32::FlagOnePathPiercingStyle, 0) > 0);
    let precognition_triggered = attack_hits
        && !is_ranged
        && combatants[defender_idx].sheet.defense.precognition
        && combatants[defender_idx].state.precognition_space_available
        && feat_of_agility_succeeds(&combatants[defender_idx], attack_roll - defense_roll, rng);
    let armeroci_opening = attacker_armeroci
        && !is_ranged
        && combatants[attacker_idx]
            .state
            .armeroci_opening_strike_available;
    if attacker_armeroci && !is_ranged {
        combatants[attacker_idx]
            .state
            .armeroci_opening_strike_available = false;
    }

    if attack_hits {
        hit = true;
        let mut rolled_damage = roll_mounted_weapon_damage(
            &weapon, mounted_plan, use_jab, false, average_damage, rng);
        if !average_damage && crit_trigger && defender_defiant {
            rolled_damage = rolled_damage.min(roll_mounted_weapon_damage(
                &weapon, mounted_plan, use_jab, false, average_damage, rng));
        }
        let halve_jab_damage = use_jab && weapon.halves_damage_for_attack();
        let mut raw = rolled_damage + strength_damage;
        if halve_jab_damage {
            raw /= 2;
        }
        if weapon.halve_damage {
            raw /= 2;
        }
        raw += damage_penalty;
        if raw < 0 {
            raw = 0;
        }
        if armeroci_opening {
            let extra_damage = {
                let extra_dice = opening_engagement_extra_dice(&combatants[attacker_idx]);
                let cache = combatants[attacker_idx].state.weapon_cache_mut(weapon_slot);
                roll_extra_damage_cached(
                    cache,
                    weapon.as_ref(),
                    use_jab,
                    extra_dice,
                    use_jab || weapon.force_nonpenetrating_damage,
                    average_damage,
                    rng,
                )
            };
            raw += extra_damage;
        }
        if attack_mode == AttackMode::HoldAtBay && !use_jab {
            damage = 0;
            knockback_ft = 0.0;
        } else {
            let armor_ignored = called_shot_precise_hit || weapon.ignore_all_dr;
            let effective_dr = if armor_ignored {
                if weapon.ignore_all_dr {
                    0
                } else {
                    natural_dr.max(0)
                }
            } else if armor_dr >= 5 || armor_is_heavy {
                (armor_dr - armor_penetration).max(0)
            } else {
                armor_dr
            };
            let close_damage =
                close_hit_damage(&weapon, strength_damage + damage_penalty).filter(|_| {
                    attack_roll - defense_roll < weapon.use_close_hit_margin_less_than
                });
            if let Some(damage) = close_damage {
                // Replace the entire weapon pool, including its built-in flat term.
                rolled_damage = damage.roll(rng, average_damage);
                raw = damage.raw_damage(rolled_damage);
            }
            let raw_base = raw;
            let defender_hp_before = combatants[defender_idx].state.hp;
            let mut crit_effect = None;
            let mut crit_extra_damage = 0;
            let mut crit_trauma_seconds = None;
            if crit_trigger || attacker_hobbler {
                let severity_defense_roll = if armor_ignored {
                    called_shot_precision_target
                } else {
                    defense_roll
                };
                let severity_damage = if halve_aggressive_damage { raw_base / 2 } else { raw_base };
                let severity = (attack_roll - severity_defense_roll + severity_damage - effective_dr + {
                    let attacker = &combatants[attacker_idx];
                    attacker.apply_i32(StatIdI32::CritSeverityBonus, weapon.crit_severity_bonus)
                } - defender_crit_severity_reduction)
                    .max(1);
                let mut effect = apply_ancillary_critical_immunity(
                    critical_effect_for(severity),
                    crit_trigger
                        && weapon.hacking_or_piercing
                        && defender_ignore_ancillary_crit_effects,
                );
                effect.extra_dice *= reaper_multiplier;
                if effect.instant_kill {
                    crit_effect = Some(effect);
                } else {
                    let extra_dice = if attacker_hobbler {
                        0
                    } else {
                        effect.extra_dice
                    };
                    crit_extra_damage = if extra_dice > 0 {
                        let cache = combatants[attacker_idx].state.weapon_cache_mut(weapon_slot);
                        let rolled = roll_extra_damage_cached(
                            cache,
                            weapon.as_ref(),
                            use_jab,
                            extra_dice,
                            use_jab || weapon.force_nonpenetrating_damage,
                            average_damage,
                            rng,
                        );
                        if defender_halves_crit_extra_damage {
                            rolled / 2
                        } else {
                            rolled
                        }
                    } else {
                        0
                    };
                    raw += crit_extra_damage;
                    crit_effect = Some(effect);
                }
            }
            if halve_aggressive_damage {
                raw /= 2;
            }
            damage = (raw - effective_dr).max(0);
            if damage > 0 {
                damage += weapon.internal_hemorrhage_damage.max(0);
            }
            if precognition_triggered {
                damage /= 2;
            }
            if !defender_infinite_hp {
                combatants[defender_idx].state.hp -= damage;
            }
            knockback_ft = knockback_rule.distance_ft(raw);

            if let Some(effect) = crit_effect {
                if effect.instant_kill {
                    if !defender_infinite_hp {
                        combatants[defender_idx].state.hp = 0;
                        damage = defender_hp_before.max(0);
                    }
                    critical = Some(CriticalHit {
                        severity: effect.severity,
                        extra_dice: if attacker_hobbler {
                            0
                        } else {
                            effect.extra_dice
                        },
                        extra_damage: crit_extra_damage,
                        speed_reset: effect.speed_reset,
                        trauma_seconds: None,
                        instant_kill: true,
                    });
                } else {
                    if effect.auto_trauma {
                        let forced = roll_damage_expr("5d6p", rng, false) * 60;
                        let applied = apply_trauma_duration(combatants, defender_idx, forced);
                        crit_trauma_seconds = Some(applied);
                    }
                    if effect.speed_reset && crit_trauma_seconds.is_none() {
                        let reset_time = now + defender_weapon_speed.max(1.0);
                        combatants[defender_idx]
                            .state
                            .set_next_attack_time(WeaponSlot::Primary, Some(reset_time));
                        combatants[defender_idx]
                            .state
                            .set_next_attack_time(WeaponSlot::Secondary, Some(reset_time));
                    }
                    critical = Some(CriticalHit {
                        severity: effect.severity,
                        extra_dice: if attacker_hobbler {
                            0
                        } else {
                            effect.extra_dice
                        },
                        extra_damage: crit_extra_damage,
                        speed_reset: effect.speed_reset,
                        trauma_seconds: crit_trauma_seconds,
                        instant_kill: false,
                    });
                }
            }

            if critical
                .as_ref()
                .map(|crit| crit.instant_kill)
                .unwrap_or(false)
            {
                trauma_seconds = None;
            } else {
                trauma_seconds = maybe_apply_trauma(combatants, defender_idx, damage, rng);
                if let Some(crit) = critical.as_mut() {
                    if let Some(crit_seconds) = crit.trauma_seconds {
                        trauma_seconds = Some(
                            trauma_seconds.map_or(crit_seconds, |base| base.max(crit_seconds)),
                        );
                        crit.trauma_seconds = trauma_seconds;
                    }
                }
            }
            damage_breakdown = Some(DamageBreakdown {
                rolled_damage,
                strength_damage,
                raw_damage: raw,
                armor_dr: if weapon.ignore_all_dr {
                    0
                } else if armor_ignored {
                    natural_dr
                } else {
                    armor_dr
                },
                armor_penetration: if armor_ignored || weapon.ignore_all_dr {
                    0
                } else {
                    armor_penetration
                },
                effective_armor_dr: effective_dr,
                final_damage: damage,
            });
        }
    } else if shield_active && !is_ranged {
        let miss_margin = defense_roll - attack_roll;
        let shield_block_window = if defender_six_paths {
            SIX_PATHS_SHIELD_BLOCK_WINDOW
        } else {
            DEFAULT_SHIELD_BLOCK_WINDOW
        };
        if miss_margin < shield_block_window {
            shield_block = true;
            let (rolled_damage, raw) = shield_block_raw_damage(
                &weapon,
                mounted_plan,
                strength_damage,
                damage_penalty,
                1,
                average_damage,
                rng,
            );
            let raw = if halve_aggressive_damage { raw / 2 } else { raw };
            shield_damage = raw;
            knockback_ft = knockback_rule.distance_ft(raw);
            let shield_after_dr = (raw - shield_dr).max(0);

            let mut effective_dr = armor_dr;
            if armor_dr >= 5 || armor_is_heavy {
                effective_dr = (armor_dr - armor_penetration).max(0);
            }
            let hp_damage = (shield_after_dr - effective_dr).max(0);
            if hp_damage > 0 {
                if !defender_infinite_hp {
                    combatants[defender_idx].state.hp -= hp_damage;
                }
                trauma_seconds = maybe_apply_trauma(combatants, defender_idx, hp_damage, rng);
            }

            let breakage_raw = if defender_unbreakable_wall {
                (raw - shield_dr).max(0)
            } else {
                raw
            };
            if let Some(steps) = shield_breakage {
                if breakage_raw >= steps[3].threshold {
                    shield_broken = true;
                } else if breakage_raw >= steps[2].threshold {
                    shield_broken = breakage_roll(steps[2], rng);
                } else if breakage_raw >= steps[1].threshold {
                    shield_broken = breakage_roll(steps[1], rng);
                } else if breakage_raw >= steps[0].threshold {
                    shield_broken = breakage_roll(steps[0], rng);
                }
            }
            if shield_broken {
                combatants[defender_idx].state.shield_intact = false;
            }
            damage = hp_damage;
            shield_damage_breakdown = Some(ShieldDamageBreakdown {
                rolled_damage,
                strength_damage,
                raw_damage: raw,
                shield_dr,
                armor_dr,
                armor_penetration,
                effective_armor_dr: effective_dr,
                hp_damage,
                shield_broken,
            });
        }
    }

    super::magic::on_attack_resolved(
        combatants,
        attacker_idx,
        defender_idx,
        now,
        hit,
        damage,
        weapon_slot,
        is_ranged,
        &roll,
    );

    if attack_mode == AttackMode::Charge {
        apply_charge_defense_penalty(combatants, attacker_idx);
    }
    update_regenstat_on_exchange(combatants, attacker_idx, defender_idx, hit);
    if attacker_three_mountains {
        if hit {
            combatants[attacker_idx].state.three_mountains_hit_streak += 1;
            if combatants[attacker_idx].state.three_mountains_hit_streak >= 3 {
                combatants[defender_idx].state.force_trauma_roll_20 = true;
            }
        } else {
            combatants[attacker_idx].state.three_mountains_hit_streak = 0;
        }
    }

    if !hit
        && knockback_ft == 0.0
        && combatants[defender_idx].magic_can_attack(WeaponSlot::Primary, now)
        && combatants[attacker_idx].state.hp > 0
        && combatants[defender_idx].state.hp > 0
        && !combatants[defender_idx].sheet.maneuvers.passive
        && defense_roll > attack_roll
    {
        let defender_reach = combatants[defender_idx]
            .apply_f32(
                StatIdF32::WeaponReach,
                combatants[defender_idx].sheet.offense.weapon.reach_ft,
            )
            .max(1.0);
        let perfect_in_reach = distance_ft <= defender_reach;
        let near_in_reach = distance_ft <= 5.0;
        let near_perfect_min = if defender_superior_defense { 18 } else { 19 };
        if defense_first == 20 && perfect_in_reach {
            counter_attack = Some(resolve_counter_attack(
                combatants,
                defender_idx,
                attacker_idx,
                now,
                WeaponSlot::Primary,
                true,
                false,
                true,
                defender_edge_counter,
                false,
                1,
                rng,
            ));
        } else if defense_first >= near_perfect_min && defense_first < 20 && near_in_reach {
            if combatants[defender_idx].sheet.defense.eyesmite {
                counter_attack = Some(resolve_eyesmite(
                    combatants,
                    defender_idx,
                    attacker_idx,
                    now,
                    rng,
                ));
            } else {
                let defender_weapon = &combatants[defender_idx].sheet.offense.weapon;
                let offhand_small = combatants[defender_idx]
                    .sheet
                    .maneuvers
                    .offensive_dualwielding
                    && combatants[defender_idx]
                        .sheet
                        .offense
                        .offhand
                        .as_ref()
                        .map(|offhand| offhand.weapon.is_small_weapon && !offhand.weapon.is_unarmed)
                        .unwrap_or(false);
                let (use_weapon, weapon_slot) = if offhand_small {
                    (true, WeaponSlot::Secondary)
                } else {
                    (
                        defender_weapon.is_small_weapon && !defender_weapon.is_unarmed,
                        WeaponSlot::Primary,
                    )
                };
                let superior_unarmed = defender_superior_defense && !use_weapon;
                counter_attack = Some(resolve_counter_attack(
                    combatants,
                    defender_idx,
                    attacker_idx,
                    now,
                    weapon_slot,
                    use_weapon,
                    !use_weapon,
                    use_weapon,
                    false,
                    superior_unarmed,
                    1,
                    rng,
                ));
            }
        }
    }
    if counter_attack.is_none()
        && combatants[defender_idx].magic_can_attack(WeaponSlot::Primary, now)
        && defender_returner
        && combatants[defender_idx].state.returner_counter_available
        && combatants[defender_idx].state.trauma_remaining_seconds <= 0
        && combatants[attacker_idx].state.hp > 0
        && combatants[defender_idx].state.hp > 0
        && !combatants[defender_idx].sheet.maneuvers.passive
    {
        let defender_reach = combatants[defender_idx]
            .apply_f32(
                StatIdF32::WeaponReach,
                combatants[defender_idx].sheet.offense.weapon.reach_ft,
            )
            .max(1.0);
        if !is_ranged && distance_ft <= defender_reach {
            let returner_multiplier =
                if combatants[defender_idx].state.returner_double_counter_ready {
                    2
                } else {
                    1
                };
            counter_attack = Some(resolve_counter_attack(
                combatants,
                defender_idx,
                attacker_idx,
                now,
                WeaponSlot::Primary,
                true,
                false,
                true,
                false,
                false,
                returner_multiplier,
                rng,
            ));
            combatants[defender_idx].state.returner_counter_available = false;
            combatants[defender_idx].state.returner_double_counter_ready = false;
        }
    }

    if hit || shield_block {
        apply_knockback_recovery(
            &mut combatants[defender_idx],
            knockback_ft,
            now,
            defender_weapon_speed,
            attacker_hammerer,
        );
    }

    if !is_ranged {
        combatants[attacker_idx].state.charge_distance_ft = 0.0;
        combatants[attacker_idx]
            .state
            .charge_threshold_started_within_20ft = false;
    }

    let defender_hp_after = combatants[defender_idx].state.hp;
    let additional_counters = resolve_evonia_counters(
        combatants,
        attacker_idx,
        defender_idx,
        !hit && !is_ranged,
        counter_attack.as_ref(),
        distance_ft,
        now,
        rng,
    );
    let trauma_applied = trauma_seconds.is_some();
    AttackOutcome {
        is_aggressive,
        damage_source,
        attacker_idx,
        defender_idx,
        knockback_ft,
        hit,
        shield_block,
        damage,
        shield_damage,
        hold_at_bay: attack_mode == AttackMode::HoldAtBay,
        weapon_slot,
        use_jab,
        is_ranged,
        trauma_applied,
        trauma_seconds,
        roll,
        damage_breakdown,
        shield_damage_breakdown,
        defender_hp_after,
        critical,
        precognition_triggered,
        counter_attack,
        additional_counters,
    }
}

fn apply_charge_defense_penalty(combatants: &mut [Combatant], attacker_idx: usize) {
    let dex_bonus = combatants[attacker_idx]
        .sheet
        .defense
        .dex_defense_bonus
        .max(0);
    let state = &mut combatants[attacker_idx].state;
    state
        .active_effects
        .retain(|effect| effect.id != CHARGE_DEFENSE_EFFECT_ID);
    if dex_bonus == 0 {
        return;
    }
    let mut effect = TemporaryEffect::new(CHARGE_DEFENSE_EFFECT_ID, CHARGE_DEFENSE_PENALTY_SECONDS);
    effect
        .modifiers
        .add_i32(StatIdI32::DefenseMod, ModifierOpI32::Add(-dex_bonus));
    state.add_effect(effect);
}

pub(crate) fn resolve_knock_aside(
    combatants: &mut [Combatant],
    attacker_idx: usize,
    defender_idx: usize,
    now: f32,
    state_snapshot: Option<&CombatRoundSnapshot>,
    rng: &mut impl Rng,
) -> KnockAsideOutcome {
    let tactical_attack_penalty = combatants[attacker_idx]
        .state
        .tactical_next_attack_penalty
        .max(0);
    combatants[attacker_idx].state.tactical_next_attack_penalty = 0;
    let defender_state = state_snapshot
        .and_then(|snapshot| snapshot.get(defender_idx))
        .copied()
        .unwrap_or_else(|| AttackStateSnapshot::capture(&combatants[defender_idx], attacker_idx));
    let attacker = &combatants[attacker_idx];
    let defender = &combatants[defender_idx];
    let attacker_fight_defensively_penalty = fight_defensively_attack_penalty(attacker);
    let defender_fight_defensively_bonus = fight_defensively_defense_bonus(defender);
    let defender_called_shot_penalty = called_shot_defense_penalty(defender);
    let attack_die = penetrating_roll(20, rng);
    let attack_bonus = attacker.sheet.offense.attack_bonus
        + attacker.intimidation_penalty()
        - attacker_fight_defensively_penalty
        - tactical_attack_penalty;
    let attack_roll = attack_die + attack_bonus;
    let defense_die = penetrating_roll(
        defense_die_sides(
            false,
            defender_state.moved_last_tick,
            false,
            defender_state.trauma_remaining_seconds > 0
                || combatants[defender_idx]
                    .state
                    .magic
                    .restricted_defense(combatants[defender_idx].magic.talents),
            defender
                .sheet
                .maneuvers
                .offensive_dualwielding_defense_penalty,
        ),
        rng,
    );
    let defense_ready = defender_state.defense_ready(&defender.sheet, now);
    let weapon_defense_bonus =
        if defender.sheet.offense.weapon.defense_bonus_always || defense_ready {
            4
        } else {
            0
        };
    let defense_base = defender
        .apply_i32(StatIdI32::DefenseMod, defender.sheet.defense.defense_mod)
        + defender_fight_defensively_bonus
        - defender_called_shot_penalty
        + defender_state.movement_defense_bonus();
    let defense_roll = defense_die + defense_base + weapon_defense_bonus;
    combatants[defender_idx].state.aggressive_defense_pending = false;
    let success = attack_roll >= defense_roll;
    let roll = KnockAsideRollBreakdown {
        attack_die,
        defense_die,
        attack_bonus,
        defense_base,
        weapon_defense_bonus,
        attack_total: attack_roll,
        defense_total: defense_roll,
    };
    KnockAsideOutcome { success, roll }
}

fn breakage_roll(step: ShieldBreakageStep, rng: &mut impl Rng) -> bool {
    if let Some(modifier) = step.save_mod {
        let attacker_roll = penetrating_roll(20, rng);
        let defender_roll = penetrating_roll(20, rng) + modifier;
        attacker_roll >= defender_roll
    } else {
        true
    }
}
