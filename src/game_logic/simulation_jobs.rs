//! Headless simulation jobs. GUI owns scheduling; calculations remain reusable.
use super::*;
use crate::core::rng::SimRng;
use crate::core::sim::{SimConfig, SimState};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, Ordering},
};

#[derive(Clone, Default)]
pub struct JobControl {
    cancelled: Arc<AtomicBool>,
    completed: Arc<AtomicU32>,
    total: Arc<AtomicU32>,
}
impl JobControl {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
    pub fn set_total(&self, total: u32) {
        self.total.store(total, Ordering::Relaxed);
    }
    pub fn total(&self) -> u32 {
        self.total.load(Ordering::Relaxed)
    }
    pub fn completed(&self) -> u32 {
        self.completed.load(Ordering::Relaxed)
    }
    pub fn keep_running(&self, completed: u32) -> bool {
        self.completed.store(completed, Ordering::Relaxed);
        !self.cancelled.load(Ordering::Relaxed)
    }
}

#[derive(Clone, Debug)]
pub struct DpsTestResult {
    pub attacker_idx: usize,
    pub defender_idx: usize,
    pub iterations: u32,
    pub duration_seconds: u32,
    pub total_damage: u64,
    pub total_landed_damage: i64,
    pub total_rolled_damage: i64,
    pub damage_rolls: u64,
    pub attacks: u64,
    pub highest_crit_hit: i32,
    pub highest_noncrit_hit: i32,
    pub highest_shield_hit: i32,
    pub instakills: u32,
    pub dps: f64,
    pub avg_damage_per_run: f64,
    pub avg_attacks_per_run: f64,
}

#[derive(Clone, Copy)]
pub struct DpsConfig {
    pub attacker_idx: usize,
    pub iterations: u32,
    pub duration_seconds: u32,
    pub seed: u64,
}
pub fn run_dps_test(
    config: SimConfig,
    mut combatants: Vec<sim::Combatant>,
    request: DpsConfig,
    control: &JobControl,
) -> Option<DpsTestResult> {
    if combatants.len() != 2 {
        return None;
    }
    let attacker_idx = request.attacker_idx.min(1);
    let defender_idx = 1 - attacker_idx;
    let iterations = request.iterations.max(1);
    let duration_seconds = request.duration_seconds.max(1);
    control.set_total(iterations);
    let seed = request.seed;
    combatants[defender_idx].sheet.maneuvers.passive = true;
    combatants[defender_idx].sheet.vitals.infinite_hp = true;
    let mut total_damage = 0u64;
    let mut total_landed_damage = 0i64;
    let mut total_rolled_damage = 0i64;
    let mut damage_rolls = 0u64;
    let mut attacks = 0u64;
    let mut highest_crit_hit = 0i32;
    let mut highest_noncrit_hit = 0i32;
    let mut highest_shield_hit = 0i32;
    let mut instakills = 0u32;

    for run_idx in 0..iterations {
        if !control.keep_running(run_idx) {
            return None;
        }
        let run_seed = seed.wrapping_add(u64::from(run_idx));
        let mut sim = SimState::with_rng(config, SimRng::from_seed(run_seed));
        sim.log_events = false;
        sim.reset_with_combatants(combatants.clone());
        while !sim.done && sim.elapsed_seconds < duration_seconds {
            if !control.keep_running(run_idx) {
                return None;
            }
            sim.tick();
        }

        let attacker_state = &sim.combatants[attacker_idx].state;
        total_damage = total_damage.saturating_add(u64::from(attacker_state.total_hp_damage_dealt));
        total_landed_damage += attacker_state.total_damage_landed_dealt;
        total_rolled_damage += attacker_state.total_damage_rolled_dealt;
        damage_rolls = damage_rolls.saturating_add(u64::from(attacker_state.damage_rolls_dealt));
        highest_crit_hit = highest_crit_hit.max(attacker_state.max_crit_hit_dealt);
        highest_noncrit_hit = highest_noncrit_hit.max(attacker_state.max_noncrit_hit_dealt);
        highest_shield_hit = highest_shield_hit.max(attacker_state.max_shield_hit_dealt);
        instakills = instakills.saturating_add(attacker_state.total_instakills_dealt);
        attacks = attacks.saturating_add(attacker_state.attack_events);
    }

    let total_seconds = iterations as f64 * duration_seconds as f64;
    let dps = total_damage as f64 / total_seconds.max(1.0);
    if !control.keep_running(iterations) {
        return None;
    }
    Some(DpsTestResult {
        attacker_idx,
        defender_idx,
        iterations,
        duration_seconds,
        total_damage,
        total_landed_damage,
        total_rolled_damage,
        damage_rolls,
        attacks,
        highest_crit_hit,
        highest_noncrit_hit,
        highest_shield_hit,
        instakills,
        dps,
        avg_damage_per_run: total_damage as f64 / iterations as f64,
        avg_attacks_per_run: attacks as f64 / iterations as f64,
    })
}

use rand::{SeedableRng, rngs::StdRng};
#[derive(Clone, Debug)]
pub struct DamageRollLine {
    pub name: String,
    pub points: Vec<[f64; 2]>,
    pub values: Vec<f64>,
    pub average: f64,
}

#[derive(Clone, Debug)]
pub struct DamageRollPlotData {
    pub lines: Vec<DamageRollLine>,
    pub iterations: usize,
    pub x_max: usize,
    pub y_max: f64,
}

pub fn build_damage_roll_plot(
    combatant: &sim::Combatant,
    iterations: usize,
    control: &JobControl,
) -> Option<DamageRollPlotData> {
    let mut rng = StdRng::from_entropy();
    let mut entries = vec![(
        format!("Mainhand: {}", combatant.sheet.offense.weapon.name),
        combatant.sheet.offense.weapon.clone(),
        combatant.sheet.offense.strength_damage,
        0,
    )];
    if let Some(offhand) = combatant.sheet.offense.offhand.as_ref() {
        entries.push((
            format!("Offhand: {}", offhand.weapon.name),
            offhand.weapon.clone(),
            offhand.strength_damage,
            combatant.sheet.maneuvers.dualwield_offhand_damage_penalty,
        ));
    }

    let mut lines = Vec::with_capacity(entries.len());
    let mut x_max = 0usize;
    let mut y_max = 0.0f64;

    let hand_count = entries.len();
    control.set_total((hand_count * iterations) as u32);
    for (hand, (name, weapon, strength_damage, damage_penalty)) in entries.into_iter().enumerate() {
        let mut counts = Vec::<usize>::new();
        let mut total = 0i64;
        for iteration in 0..iterations {
            if iteration % 256 == 0 && !control.keep_running((hand * iterations + iteration) as u32)
            {
                return None;
            }
            let raw =
                roll_weapon_raw_damage(weapon.as_ref(), strength_damage, damage_penalty, &mut rng);
            let raw_idx = raw.max(0) as usize;
            if raw_idx >= counts.len() {
                counts.resize(raw_idx + 1, 0);
            }
            counts[raw_idx] += 1;
            total += i64::from(raw);
        }

        let mut points = Vec::with_capacity(counts.len());
        let mut values = Vec::with_capacity(counts.len());
        let denom = iterations.max(1) as f64;
        for (damage, count) in counts.into_iter().enumerate() {
            let frequency = count as f64 / denom;
            points.push([damage as f64, frequency]);
            values.push(frequency);
            y_max = y_max.max(frequency);
            x_max = x_max.max(damage);
        }

        lines.push(DamageRollLine {
            name,
            points,
            values,
            average: total as f64 / denom,
        });
    }

    if !control.keep_running((hand_count * iterations) as u32) {
        return None;
    }
    Some(DamageRollPlotData {
        lines,
        iterations,
        x_max: x_max.max(1),
        y_max: y_max.max(0.01),
    })
}

pub fn roll_weapon_raw_damage(
    weapon: &sim::WeaponProfile,
    strength_damage: i32,
    damage_penalty: i32,
    rng: &mut impl rand::Rng,
) -> i32 {
    let nonpenetrating = if weapon.use_jab {
        true
    } else {
        weapon.force_nonpenetrating_damage
    };
    let rolled_damage = weapon
        .damage_expr_cache_for_attack()
        .roll(rng, nonpenetrating);
    let mut raw = rolled_damage + strength_damage;
    if weapon.halves_damage_for_attack() {
        raw /= 2;
    }
    if weapon.halve_damage {
        raw /= 2;
    }
    raw += damage_penalty;
    raw.max(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::sim::{CombatEventKind, bulk_simulate_with_seed_controlled};

    fn combatants() -> Vec<sim::Combatant> {
        let (weapons, armor, shields) = crate::data::load_catalogs().unwrap();
        let talents = crate::data::load_talents(crate::data::TALENTS_PATH).unwrap();
        let npcs = crate::data::load_npc_presets("data/sim/npc_presets.json").unwrap();
        let mut players = [
            PlayerConfig::new("A", weapons.first_id().unwrap()),
            PlayerConfig::new("B", weapons.first_id().unwrap()),
        ];
        for player in &mut players {
            player.magic.learn_spell("echo_strike");
            player.magic.use_essence_costs = false;
            player.magic.auto_cast = crate::core::magic::AutoCast::SpellPolicies;
            player.tactical_policy.enabled = true;
        }
        build_combatants(&players, &weapons, &armor, &shields, &npcs, &talents)
    }

    #[test]
    fn dps_matches_logged_attack_counts_without_storing_events() {
        let config = SimConfig::new(5.0, 1.0);
        let mut fighters = combatants();
        let request = DpsConfig {
            attacker_idx: 0,
            iterations: 16,
            duration_seconds: 120,
            seed: 63,
        };
        let result =
            run_dps_test(config, fighters.clone(), request, &JobControl::default()).unwrap();
        fighters[1].sheet.maneuvers.passive = true;
        fighters[1].sheet.vitals.infinite_hp = true;
        let mut attacks = 0;
        let mut damage = 0;
        for run in 0..request.iterations {
            let seed = request.seed + u64::from(run);
            let mut logged = SimState::with_rng(config, SimRng::from_seed(seed));
            logged.log_events = true;
            logged.reset_with_combatants(fighters.clone());
            while !logged.done && logged.elapsed_seconds < request.duration_seconds {
                logged.tick();
            }
            let count = logged
                .combat_events
                .iter()
                .filter(|event| {
                    event.attacker_idx == 0 && matches!(event.kind, CombatEventKind::Attack(_))
                })
                .count() as u64;
            assert_eq!(logged.combatants[0].state.attack_events, count);
            attacks += count;
            damage += u64::from(logged.combatants[0].state.total_hp_damage_dealt);
        }
        assert!(attacks > 0);
        assert_eq!(result.attacks, attacks);
        assert_eq!(result.total_damage, damage);
    }

    #[test]
    fn cancelled_jobs_discard_partial_results_including_stalled_fights() {
        let control = JobControl::default();
        control.cancel();
        let config = SimConfig::new(5.0, 1.0);
        let mut fighters = combatants();
        assert!(
            run_dps_test(
                config,
                fighters.clone(),
                DpsConfig {
                    attacker_idx: 0,
                    iterations: 100,
                    duration_seconds: 60,
                    seed: 1
                },
                &control
            )
            .is_none()
        );
        assert!(build_damage_roll_plot(&fighters[0], 1000, &control).is_none());
        for fighter in &mut fighters {
            fighter.sheet.maneuvers.passive = true;
        }
        let mut calls = 0;
        assert!(
            bulk_simulate_with_seed_controlled(config, fighters, 1, u32::MAX, 1, |_| {
                calls += 1;
                calls < 3
            })
            .is_none()
        );
        assert_eq!(
            calls, 3,
            "a stalled fight must check cancellation within the fight"
        );
    }

    #[test]
    fn tactical_profiles_are_shared_but_fight_state_is_independent() {
        let fighters = combatants();
        let mut cloned = fighters[0].clone();
        assert!(!cloned.tactical_profiles.is_empty());
        assert!(Arc::ptr_eq(
            &cloned.tactical_profiles,
            &fighters[0].tactical_profiles
        ));
        cloned.state.hp -= 1;
        assert_ne!(cloned.state.hp, fighters[0].state.hp);
        assert!(cloned.activate_tactical_profile(false));
    }
}
