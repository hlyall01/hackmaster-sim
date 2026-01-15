use rand::Rng;

use crate::core::rules::{clean_damage_expr, penetrating_roll, roll_damage_expr};

use super::types::{
    defense_plus_four_ready_at, AttackRollBreakdown, Combatant, CombatantState, CriticalHit,
    DamageBreakdown, KnockAsideRollBreakdown, ShieldBreakageStep, ShieldDamageBreakdown,
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
    pub(super) critical: Option<CriticalHit>,
    pub(super) counter_attack: Option<CounterAttackOutcome>,
}

pub(crate) struct CounterAttackOutcome {
    pub(super) attacker_idx: usize,
    pub(super) defender_idx: usize,
    pub(super) knockback_ft: f32,
    pub(super) hit: bool,
    pub(super) shield_block: bool,
    pub(super) damage: i32,
    pub(super) shield_damage: i32,
    pub(super) use_jab: bool,
    pub(super) is_ranged: bool,
    pub(super) trauma_applied: bool,
    pub(super) trauma_seconds: Option<i32>,
    pub(super) roll: AttackRollBreakdown,
    pub(super) damage_breakdown: Option<DamageBreakdown>,
    pub(super) shield_damage_breakdown: Option<ShieldDamageBreakdown>,
    pub(super) defender_hp_after: i32,
    pub(super) critical: Option<CriticalHit>,
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

fn knockback_distance_ft(raw_damage: i32) -> f32 {
    if raw_damage <= 0 {
        0.0
    } else {
        let steps = raw_damage / 15;
        (steps * 5) as f32
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DamageDie {
    pub sides: i32,
    pub penetrating: bool,
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

fn parse_damage_dice(expr: &str, force_nonpenetrating: bool) -> Vec<DamageDie> {
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
                    if force_nonpenetrating {
                        penetrating = false;
                    }
                    for _ in 0..count.max(1) {
                        dice.push(DamageDie { sides, penetrating });
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
                if force_nonpenetrating {
                    penetrating = false;
                }
                dice.push(DamageDie { sides, penetrating });
            }
            if i < chars.len() && chars[i] == 'p' {
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    let mut indexed: Vec<(usize, DamageDie)> = dice.into_iter().enumerate().collect();
    indexed.sort_by_key(|(idx, die)| (die.sides, *idx));
    indexed.into_iter().map(|(_, die)| die).collect()
}

pub(crate) fn extra_damage_dice_sequence(
    expr: &str,
    dice: i32,
    force_nonpenetrating: bool,
) -> Vec<DamageDie> {
    if dice <= 0 {
        return Vec::new();
    }
    let pool = parse_damage_dice(expr, force_nonpenetrating);
    if pool.is_empty() {
        return Vec::new();
    }
    let mut sequence = Vec::new();
    for idx in 0..dice {
        let die = pool[idx as usize % pool.len()];
        sequence.push(die);
    }
    sequence
}

fn roll_extra_damage(expr: &str, dice: i32, force_nonpenetrating: bool, rng: &mut impl Rng) -> i32 {
    let sequence = extra_damage_dice_sequence(expr, dice, force_nonpenetrating);
    let mut total = 0;
    for die in sequence {
        total += if die.penetrating {
            penetrating_roll(die.sides, rng)
        } else {
            roll_die(die.sides, rng)
        };
    }
    total
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

fn apply_trauma_duration(
    combatants: &mut [Combatant; 2],
    defender_idx: usize,
    duration: i32,
) -> i32 {
    let duration = duration.max(1);
    let remaining = combatants[defender_idx].state.trauma_remaining_seconds;
    let new_duration = remaining.max(duration);
    combatants[defender_idx].state.trauma_remaining_seconds = new_duration;
    combatants[defender_idx].state.next_attack_time = None;
    new_duration
}

fn resolve_counter_attack(
    combatants: &mut [Combatant; 2],
    attacker_idx: usize,
    defender_idx: usize,
    now: f32,
    use_weapon: bool,
    ignore_armor: bool,
    allow_critical: bool,
    rng: &mut impl Rng,
) -> CounterAttackOutcome {
    let attacker = &combatants[attacker_idx];
    let defender_state = combatants[defender_idx].state.clone();
    let defender = &combatants[defender_idx];
    let (attack_bonus, strength_damage, damage_expr, armor_penetration) = if use_weapon {
        (
            attacker.sheet.offense.attack_bonus,
            attacker.sheet.offense.strength_damage,
            attacker.sheet.offense.weapon.damage_expr.as_str(),
            attacker.sheet.offense.weapon.armor_penetration,
        )
    } else {
        (
            attacker.sheet.offense.attack_bonus_base,
            attacker.sheet.offense.strength_damage_base + attacker.sheet.offense.unarmed_damage_bonus,
            "(d4p-2)+(d4p-2)",
            0,
        )
    };

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
    ) = {
        (
            defender.sheet.defense.defense_mod,
            defender.sheet.defense.armor_dr,
            defender.sheet.defense.natural_dr,
            defender.sheet.defense.armor_is_heavy,
            defender_state.shield_intact,
            defender.sheet.defense.shield_defense_bonus,
            defender.sheet.defense.shield_dr,
            defender.sheet.defense.shield_breakage,
            defender_state.trauma_remaining_seconds > 0,
            defender.sheet.offense.weapon.defense_bonus_always,
            defender.sheet.offense.weapon.speed,
        )
    };
    let defense_ready =
        defense_plus_four_ready_at(&combatants[defender_idx].sheet, &defender_state, now);
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

    let (attack_die, attack_first) = penetrating_roll_with_first(20, rng);
    let (defense_die, defense_first) = penetrating_roll_with_first(
        defense_die_sides(
            false,
            defender_state.moved_last_tick,
            shield_active,
            trauma_incapacitated,
        ),
        rng,
    );
    let attack_roll = attack_die + attack_bonus;
    let defense_roll = defense_die + defense_mod + weapon_defense_bonus + shield_defense_bonus;
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

    let mut hit = attack_roll >= defense_roll;
    if attack_first == 20 {
        if defense_first == 20 && defense_roll > attack_roll {
            hit = false;
        } else {
            hit = true;
        }
    }

    let mut damage = 0;
    let mut shield_block = false;
    let mut shield_damage = 0;
    let mut shield_broken = false;
    let mut knockback_ft = 0.0;
    let mut trauma_seconds = None;
    let mut damage_breakdown = None;
    let mut shield_damage_breakdown = None;
    let mut critical = None;

    if hit {
        let rolled_damage = if use_weapon {
            attacker.sheet.offense.weapon.damage_expr_cache.roll(rng, false)
        } else {
            roll_damage_expr(damage_expr, rng, false)
        };
        let mut raw = rolled_damage + strength_damage;
        if raw < 0 {
            raw = 0;
        }
        let raw_base = raw;
        let defender_hp_before = combatants[defender_idx].state.hp;
        let mut effective_dr = if ignore_armor {
            natural_dr.max(0)
        } else if armor_dr >= 5 || armor_is_heavy {
            (armor_dr - armor_penetration).max(0)
        } else {
            armor_dr.max(0)
        };
        let mut crit_trauma_seconds = None;
        if allow_critical && attack_first == 20 {
            let severity = (attack_roll - defense_roll + raw_base - effective_dr).max(1);
            let effect = critical_effect_for(severity);
            if effect.instant_kill {
                critical = Some(CriticalHit {
                    severity: effect.severity,
                    extra_dice: effect.extra_dice,
                    extra_damage: 0,
                    speed_reset: effect.speed_reset,
                    trauma_seconds: None,
                    instant_kill: true,
                });
            } else {
                let extra_damage = roll_extra_damage(damage_expr, effect.extra_dice, false, rng);
                raw += extra_damage;
                effective_dr = if ignore_armor {
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
                    combatants[defender_idx].state.next_attack_time = Some(reset_time);
                }
                critical = Some(CriticalHit {
                    severity: effect.severity,
                    extra_dice: effect.extra_dice,
                    extra_damage: extra_damage,
                    speed_reset: effect.speed_reset,
                    trauma_seconds: crit_trauma_seconds,
                    instant_kill: false,
                });
            }
        }

        if critical.as_ref().map(|crit| crit.instant_kill).unwrap_or(false) {
            combatants[defender_idx].state.hp = 0;
            damage = defender_hp_before.max(0);
            trauma_seconds = None;
        } else {
            damage = (raw - effective_dr).max(0);
            combatants[defender_idx].state.hp -= damage;
            knockback_ft = knockback_distance_ft(raw);
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
            armor_dr: if ignore_armor { natural_dr } else { armor_dr },
            armor_penetration: if ignore_armor { 0 } else { armor_penetration },
            effective_armor_dr: effective_dr,
            final_damage: damage,
        });
    } else if shield_active {
        let miss_margin = defense_roll - attack_roll;
        if miss_margin < 10 {
            shield_block = true;
            let rolled_damage = if use_weapon {
                attacker
                    .sheet
                    .offense
                    .weapon
                    .shield_damage_expr_cache
                    .as_ref()
                    .unwrap_or(&attacker.sheet.offense.weapon.damage_expr_cache)
                    .roll(rng, false)
            } else {
                roll_damage_expr(damage_expr, rng, false)
            };
            let mut raw = rolled_damage + strength_damage;
            if raw < 0 {
                raw = 0;
            }
            shield_damage = raw;
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
                shield_dr: effective_shield_dr,
                armor_dr: if ignore_armor { natural_dr } else { armor_dr },
                armor_penetration: if ignore_armor { 0 } else { armor_penetration },
                effective_armor_dr: effective_dr,
                hp_damage,
                shield_broken,
            });
        }
    }

    let defender_hp_after = combatants[defender_idx].state.hp;
    let trauma_applied = trauma_seconds.is_some();
    CounterAttackOutcome {
        attacker_idx,
        defender_idx,
        knockback_ft,
        hit,
        shield_block,
        damage,
        shield_damage,
        use_jab: false,
        is_ranged: false,
        trauma_applied,
        trauma_seconds,
        roll,
        damage_breakdown,
        shield_damage_breakdown,
        defender_hp_after,
        critical,
    }
}

pub(crate) fn resolve_attack(
    combatants: &mut [Combatant; 2],
    attacker_idx: usize,
    defender_idx: usize,
    range_mod: i32,
    is_ranged: bool,
    distance_ft: f32,
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
        attacker_uses_projectiles,
    ) = {
        let attacker = &combatants[attacker_idx];
        let weapon = &attacker.sheet.offense.weapon;
        (
            attacker.sheet.offense.attack_bonus,
            attacker.sheet.offense.strength_damage,
            weapon.armor_penetration,
            weapon.use_jab,
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
            defender.sheet.offense.weapon.defense_bonus_always,
            defender.sheet.offense.weapon.speed,
        )
    };
    let defense_ready = if is_ranged {
        false
    } else {
        defense_plus_four_ready_at(&combatants[defender_idx].sheet, defender_state, now)
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

    let (attack_die, attack_first) = penetrating_roll_with_first(20, rng);
    let (defense_die, defense_first) = penetrating_roll_with_first(
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
    let mut critical = None;
    let natural_20 = attack_first == 20;
    let mut counter_attack = None;

    let mut attack_hits = attack_roll >= defense_roll;
    if natural_20 {
        if defense_first == 20 && defense_roll > attack_roll {
            attack_hits = false;
        } else {
            attack_hits = true;
        }
    }

    if attack_hits {
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
            let raw_base = raw;
            let defender_hp_before = combatants[defender_idx].state.hp;
            let mut crit_effect = None;
            let mut crit_extra_damage = 0;
            let mut crit_trauma_seconds = None;
            if natural_20 {
                let severity = (attack_roll - defense_roll + raw_base - effective_dr).max(1);
                let effect = critical_effect_for(severity);
                if effect.instant_kill {
                    crit_effect = Some(effect);
                } else {
                    let weapon = &combatants[attacker_idx].sheet.offense.weapon;
                    let damage_expr = if use_jab {
                        weapon
                            .jab_special_expr
                            .as_deref()
                            .unwrap_or(weapon.damage_expr.as_str())
                    } else {
                        weapon.damage_expr.as_str()
                    };
                    crit_extra_damage =
                        roll_extra_damage(damage_expr, effect.extra_dice, use_jab, rng);
                    raw += crit_extra_damage;
                    crit_effect = Some(effect);
                }
            }
            damage = (raw - effective_dr).max(0);
            combatants[defender_idx].state.hp -= damage;
            knockback_ft = knockback_distance_ft(raw);

            if let Some(effect) = crit_effect {
                if effect.instant_kill {
                    combatants[defender_idx].state.hp = 0;
                    damage = defender_hp_before.max(0);
                    critical = Some(CriticalHit {
                        severity: effect.severity,
                        extra_dice: effect.extra_dice,
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
                        combatants[defender_idx].state.next_attack_time = Some(reset_time);
                    }
                    critical = Some(CriticalHit {
                        severity: effect.severity,
                        extra_dice: effect.extra_dice,
                        extra_damage: crit_extra_damage,
                        speed_reset: effect.speed_reset,
                        trauma_seconds: crit_trauma_seconds,
                        instant_kill: false,
                    });
                }
            }

            if critical.as_ref().map(|crit| crit.instant_kill).unwrap_or(false) {
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

    if !hit
        && combatants[attacker_idx].state.hp > 0
        && combatants[defender_idx].state.hp > 0
        && defense_roll > attack_roll
    {
        let defender_reach = combatants[defender_idx]
            .sheet
            .offense
            .weapon
            .reach_ft
            .max(1.0);
        let perfect_in_reach = distance_ft <= defender_reach;
        let near_in_reach = distance_ft <= 5.0;
        if defense_first == 20 && perfect_in_reach {
            counter_attack = Some(resolve_counter_attack(
                combatants,
                defender_idx,
                attacker_idx,
                now,
                true,
                false,
                true,
                rng,
            ));
        } else if defense_first == 19 && near_in_reach {
            let defender_weapon = &combatants[defender_idx].sheet.offense.weapon;
            let use_weapon =
                defender_weapon.is_small_weapon && !defender_weapon.is_unarmed;
            counter_attack = Some(resolve_counter_attack(
                combatants,
                defender_idx,
                attacker_idx,
                now,
                use_weapon,
                !use_weapon,
                use_weapon,
                rng,
            ));
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
        critical,
        counter_attack,
    }
}

pub(crate) fn resolve_knock_aside(
    combatants: &mut [Combatant; 2],
    attacker_idx: usize,
    defender_idx: usize,
    now: f32,
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
    let defense_ready = defense_plus_four_ready_at(&defender.sheet, defender_state, now);
    let weapon_defense_bonus =
        if defender.sheet.offense.weapon.defense_bonus_always || defense_ready {
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
