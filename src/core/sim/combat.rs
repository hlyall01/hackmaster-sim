use rand::Rng;

use crate::core::rules::{penetrating_roll, roll_damage_expr};

use super::types::{
    AttackRollBreakdown, Combatant, CombatantState, DamageBreakdown, KnockAsideRollBreakdown,
    ShieldBreakageStep, ShieldDamageBreakdown,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AttackMode {
    Normal,
    HoldAtBay,
}

pub(crate) struct AttackOutcome {
    pub(super) attacker_idx: usize,
    pub(super) defender_idx: usize,
    pub(super) knockback_ft: f32,
    pub(super) hit: bool,
    pub(super) shield_block: bool,
    pub(super) damage: i32,
    pub(super) shield_damage: i32,
    pub(super) hold_at_bay: bool,
    pub(super) use_jab: bool,
    pub(super) is_ranged: bool,
    pub(super) trauma_applied: bool,
    pub(super) trauma_seconds: Option<i32>,
    pub(super) roll: AttackRollBreakdown,
    pub(super) damage_breakdown: Option<DamageBreakdown>,
    pub(super) shield_damage_breakdown: Option<ShieldDamageBreakdown>,
    pub(super) defender_hp_after: i32,
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
) -> i32 {
    if trauma_incapacitated {
        return 8;
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

fn knockback_distance_ft(raw_damage: i32) -> f32 {
    if raw_damage <= 0 {
        0.0
    } else {
        let steps = raw_damage / 15;
        (steps * 5) as f32
    }
}

fn maybe_apply_trauma(
    combatants: &mut [Combatant; 2],
    defender_idx: usize,
    damage: i32,
    rng: &mut impl Rng,
) -> Option<i32> {
    let pain_threshold = combatants[defender_idx].sheet.vitals.threshold_of_pain;
    if damage <= pain_threshold {
        return None;
    }
    let con_half = (combatants[defender_idx].sheet.vitals.constitution as i32) / 2;
    let trauma_die_sides = combatants[defender_idx].sheet.vitals.trauma_die_sides.max(1);
    let trauma_penetrating = combatants[defender_idx].sheet.vitals.trauma_die_penetrating;
    let trauma_roll = if trauma_penetrating {
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
    combatants[defender_idx].state.next_attack_time = None;
    Some(duration as i32)
}

pub(crate) fn resolve_attack(
    combatants: &mut [Combatant; 2],
    attacker_idx: usize,
    defender_idx: usize,
    range_mod: i32,
    is_ranged: bool,
    attack_mode: AttackMode,
    now: f32,
    state_snapshot: Option<&[CombatantState; 2]>,
    rng: &mut impl Rng,
) -> AttackOutcome {
    let defender_state = state_snapshot
        .map(|snapshot| &snapshot[defender_idx])
        .unwrap_or(&combatants[defender_idx].state);
    let (
        attack_bonus,
        strength_damage,
        armor_penetration,
        use_jab,
        attacker_two_hand_grip,
        attacker_has_weapon,
        attacker_weapon_defense_always,
        attacker_uses_projectiles,
    ) = {
        let attacker = &combatants[attacker_idx];
        let weapon = &attacker.sheet.offense.weapon;
        (
            attacker.sheet.offense.attack_bonus,
            attacker.sheet.offense.strength_damage,
            weapon.armor_penetration,
            weapon.use_jab,
            weapon.two_hand_grip,
            weapon.has_weapon,
            weapon.defense_bonus_always,
            weapon.uses_projectiles,
        )
    };
    let strength_damage = if is_ranged && attacker_uses_projectiles {
        0
    } else {
        strength_damage
    };
    let (
        defense_mod,
        ranged_defense_mod,
        armor_dr,
        armor_is_heavy,
        shield_active,
        shield_defense_bonus,
        shield_cover_value,
        shield_dr,
        shield_breakage,
        trauma_incapacitated,
        defender_two_hand_grip,
        defender_has_weapon,
        defender_weapon_defense_always,
        defender_weapon_speed,
    ) = {
        let defender = &combatants[defender_idx];
        (
            defender.sheet.defense.defense_mod,
            defender.sheet.defense.ranged_defense_mod,
            defender.sheet.defense.armor_dr,
            defender.sheet.defense.armor_is_heavy,
            defender_state.shield_intact,
            defender.sheet.defense.shield_defense_bonus,
            defender.sheet.defense.shield_cover_value,
            defender.sheet.defense.shield_dr,
            defender.sheet.defense.shield_breakage,
            defender_state.trauma_remaining_seconds > 0,
            defender.sheet.offense.weapon.two_hand_grip,
            defender.sheet.offense.weapon.has_weapon,
            defender.sheet.offense.weapon.defense_bonus_always,
            defender.sheet.offense.weapon.speed,
        )
    };
    let weapon_defense_bonus = if is_ranged {
        0
    } else if defender_weapon_defense_always
        || (defender_two_hand_grip && defender_state.defense_plus_four_ready)
    {
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

    let attack_die = penetrating_roll(20, rng);
    let defense_die = penetrating_roll(
        defense_die_sides(
            is_ranged,
            defender_state.moved_last_tick,
            shield_active,
            trauma_incapacitated,
        ),
        rng,
    );
    let mut attack_roll = attack_die + attack_bonus + range_mod;
    let mut use_shield_for_ranged = false;
    let (defense_mod_used, shield_defense_bonus_used) = if is_ranged {
        let dodge_total = defense_die + ranged_defense_mod;
        let shield_total = defense_die + shield_defense_bonus;
        if ranged_defense_mod != 0 && dodge_total >= shield_total {
            (ranged_defense_mod, 0)
        } else {
            use_shield_for_ranged = shield_active;
            (0, shield_defense_bonus)
        }
    } else {
        (defense_mod, shield_defense_bonus)
    };
    if is_ranged && use_shield_for_ranged {
        if let Some(cap) = shield_cover_value {
            attack_roll = attack_roll.min(cap);
        }
    }
    let defense_roll = defense_die + defense_mod_used + weapon_defense_bonus + shield_defense_bonus_used;
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
    let mut damage = 0;
    let mut hit = false;
    let mut shield_block = false;
    let mut shield_damage = 0;
    let mut shield_broken = false;
    let mut knockback_ft = 0.0;
    let mut trauma_seconds = None;
    let mut damage_breakdown = None;
    let mut shield_damage_breakdown = None;

    if attack_roll >= defense_roll {
        hit = true;
        let (rolled_damage, halve_jab_damage) = {
            let weapon = &combatants[attacker_idx].sheet.offense.weapon;
            if use_jab {
                let cache = weapon
                    .jab_special_expr_cache
                    .as_ref()
                    .unwrap_or(&weapon.damage_expr_cache);
                (cache.roll(rng, true), weapon.jab_special_expr_cache.is_none())
            } else {
                (weapon.damage_expr_cache.roll(rng, false), false)
            }
        };
        let mut raw = rolled_damage + strength_damage;
        if halve_jab_damage {
            raw /= 2;
        }
        if raw < 0 {
            raw = 0;
        }
        if attack_mode == AttackMode::HoldAtBay && !use_jab {
            damage = 0;
            knockback_ft = 0.0;
        } else {
            let mut effective_dr = armor_dr;
            if armor_dr >= 5 || armor_is_heavy {
                effective_dr = (armor_dr - armor_penetration).max(0);
            }
            damage = (raw - effective_dr).max(0);
            combatants[defender_idx].state.hp -= damage;
            knockback_ft = knockback_distance_ft(raw);

            trauma_seconds = maybe_apply_trauma(combatants, defender_idx, damage, rng);
            damage_breakdown = Some(DamageBreakdown {
                rolled_damage,
                strength_damage,
                raw_damage: raw,
                armor_dr,
                armor_penetration,
                effective_armor_dr: effective_dr,
                final_damage: damage,
            });
        }
    } else if shield_active && !is_ranged {
        let miss_margin = defense_roll - attack_roll;
        if miss_margin < 10 {
            shield_block = true;
            let rolled_damage = {
                let weapon = &combatants[attacker_idx].sheet.offense.weapon;
                let cache = weapon
                    .shield_damage_expr_cache
                    .as_ref()
                    .unwrap_or(&weapon.damage_expr_cache);
                cache.roll(rng, false)
            };
            let mut raw = rolled_damage + strength_damage;
            if raw < 0 {
                raw = 0;
            }
            shield_damage = raw;
            let shield_after_dr = (raw - shield_dr).max(0);

            let mut effective_dr = armor_dr;
            if armor_dr >= 5 || armor_is_heavy {
                effective_dr = (armor_dr - armor_penetration).max(0);
            }
            let hp_damage = (shield_after_dr - effective_dr).max(0);
            if hp_damage > 0 {
                combatants[defender_idx].state.hp -= hp_damage;
                trauma_seconds = maybe_apply_trauma(combatants, defender_idx, hp_damage, rng);
            }

            if let Some(steps) = shield_breakage {
                if raw >= steps[3].threshold {
                    shield_broken = true;
                } else if raw >= steps[2].threshold {
                    shield_broken = breakage_roll(steps[2], rng);
                } else if raw >= steps[1].threshold {
                    shield_broken = breakage_roll(steps[1], rng);
                } else if raw >= steps[0].threshold {
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

    if hit && knockback_ft >= 10.0 {
        combatants[defender_idx].state.knockback_immobile_seconds =
            combatants[defender_idx]
                .state
                .knockback_immobile_seconds
                .max(1);
        let reset_time = now + defender_weapon_speed.max(1.0);
        combatants[defender_idx].state.next_attack_time = Some(reset_time);
    }

    if !is_ranged {
        if defender_two_hand_grip
            && combatants[defender_idx].state.defense_plus_four_ready
            && defender_has_weapon
            && !defender_weapon_defense_always
        {
            combatants[defender_idx].state.defense_plus_four_ready = false;
        }
        if attacker_two_hand_grip && attacker_has_weapon && !attacker_weapon_defense_always {
            combatants[attacker_idx].state.defense_plus_four_ready = true;
        }
    }
    let defender_hp_after = combatants[defender_idx].state.hp;
    let trauma_applied = trauma_seconds.is_some();
    AttackOutcome {
        attacker_idx,
        defender_idx,
        knockback_ft,
        hit,
        shield_block,
        damage,
        shield_damage,
        hold_at_bay: attack_mode == AttackMode::HoldAtBay,
        use_jab,
        is_ranged,
        trauma_applied,
        trauma_seconds,
        roll,
        damage_breakdown,
        shield_damage_breakdown,
        defender_hp_after,
    }
}

pub(crate) fn resolve_knock_aside(
    combatants: &mut [Combatant; 2],
    attacker_idx: usize,
    defender_idx: usize,
    state_snapshot: Option<&[CombatantState; 2]>,
    rng: &mut impl Rng,
) -> KnockAsideOutcome {
    let defender_state = state_snapshot
        .map(|snapshot| &snapshot[defender_idx])
        .unwrap_or(&combatants[defender_idx].state);
    let attacker = &combatants[attacker_idx];
    let defender = &combatants[defender_idx];
    let attack_die = penetrating_roll(20, rng);
    let attack_roll = attack_die + attacker.sheet.offense.attack_bonus;
    let defense_die = penetrating_roll(
        defense_die_sides(
            false,
            defender_state.moved_last_tick,
            false,
            defender_state.trauma_remaining_seconds > 0,
        ),
        rng,
    );
    let weapon_defense_bonus = if defender.sheet.offense.weapon.defense_bonus_always
        || (defender.sheet.offense.weapon.two_hand_grip && defender_state.defense_plus_four_ready)
    {
        4
    } else {
        0
    };
    let defense_roll = defense_die + defender.sheet.defense.defense_mod + weapon_defense_bonus;
    let success = attack_roll >= defense_roll;
    let roll = KnockAsideRollBreakdown {
        attack_die,
        defense_die,
        attack_bonus: attacker.sheet.offense.attack_bonus,
        defense_base: defender.sheet.defense.defense_mod,
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
