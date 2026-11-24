//! Lossless batch statistics for parallel bulk jobs. Final rates and percentiles
//! are computed once, after merging raw counters and histograms.
use super::*;

// Mix the two inputs separately: seed + index would cause consecutive GUI
// batches to reuse all but one of their fights when the batch seed increments.
pub(super) fn fight_seed(seed: u64, index: u32) -> u64 {
    fn mix(mut value: u64) -> u64 {
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^ (value >> 31)
    }
    mix(seed ^ mix(u64::from(index).wrapping_add(0x9e3779b97f4a7c15)))
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BulkSimBatch {
    pub(super) runs: u32,
    pub(super) wins: Vec<u32>,
    pub(super) ties: u32,
    pub(super) shields_present: bool,
    pub(super) shield_breaks: u32,
    pub(super) total_hits_shield_survived: u64,
    pub(super) total_seconds: u64,
    pub(super) shortest_duration: u32,
    pub(super) longest_duration: u32,
    pub(super) fights_with_second_charge: u32,
    pub(super) fights_with_trauma: u32,
    pub(super) fights_with_trauma_first_exchange: u32,
    pub(super) fights_with_knockback_20ft: u32,
    pub(super) fights_with_charge_within_20ft: u32,
    pub(super) instakills: u32,
    pub(super) highest_single_crit_hit: i32,
    pub(super) highest_single_noncrit_hit: i32,
    pub(super) highest_single_shield_hit: i32,
    pub(super) highest_single_crit_hit_by_team: Vec<i32>,
    pub(super) highest_single_noncrit_hit_by_team: Vec<i32>,
    pub(super) highest_single_shield_hit_by_team: Vec<i32>,
    pub(super) instakills_by_team: Vec<u32>,
    pub(super) total_damage_dealt_by_team: Vec<u64>,
    pub(super) total_damage_taken_by_team: Vec<u64>,
    pub(super) total_damage_rolled_dealt_by_team: Vec<i64>,
    pub(super) total_damage_landed_dealt_by_team: Vec<i64>,
    pub(super) damage_rolls_dealt_by_team: Vec<u64>,
    pub(super) total_remaining_hp_by_team: Vec<u64>,
    pub(super) max_total_knockback_one_side_ft: f32,
    pub(super) total_max_knockback_one_side_ft: f64,
    pub(super) duration_histogram: BTreeMap<u64, u64>,
    pub(super) detailed_accumulators: Vec<DetailedTeamAccumulator>,
}

impl BulkSimBatch {
    pub fn runs(&self) -> u32 {
        self.runs
    }

    pub fn finish(self) -> BulkSimResult {
        if self.runs == 0 {
            return BulkSimResult::default();
        }
        let Self {
            runs,
            wins,
            ties,
            shields_present,
            shield_breaks,
            total_hits_shield_survived,
            total_seconds,
            shortest_duration,
            longest_duration,
            fights_with_second_charge,
            fights_with_trauma,
            fights_with_trauma_first_exchange,
            fights_with_knockback_20ft,
            fights_with_charge_within_20ft,
            instakills,
            highest_single_crit_hit,
            highest_single_noncrit_hit,
            highest_single_shield_hit,
            highest_single_crit_hit_by_team,
            highest_single_noncrit_hit_by_team,
            highest_single_shield_hit_by_team,
            instakills_by_team,
            total_damage_dealt_by_team,
            total_damage_taken_by_team,
            total_damage_rolled_dealt_by_team,
            total_damage_landed_dealt_by_team,
            damage_rolls_dealt_by_team,
            total_remaining_hp_by_team,
            max_total_knockback_one_side_ft,
            total_max_knockback_one_side_ft,
            duration_histogram,
            detailed_accumulators,
        } = self;
        let avg_duration = total_seconds as f32 / runs as f32;
        let avg_damage_dealt_by_team = total_damage_dealt_by_team
            .iter()
            .copied()
            .map(|value| value as f32 / runs as f32)
            .collect();
        let avg_damage_taken_by_team = total_damage_taken_by_team
            .into_iter()
            .map(|value| value as f32 / runs as f32)
            .collect();
        let avg_damage_landed_by_team = total_damage_landed_dealt_by_team
            .into_iter()
            .zip(damage_rolls_dealt_by_team.iter().copied())
            .map(|(total, count)| {
                if count > 0 {
                    total as f32 / count as f32
                } else {
                    0.0
                }
            })
            .collect();
        let avg_damage_rolled_by_team = total_damage_rolled_dealt_by_team
            .into_iter()
            .zip(damage_rolls_dealt_by_team)
            .map(|(total, count)| {
                if count > 0 {
                    total as f32 / count as f32
                } else {
                    0.0
                }
            })
            .collect();
        let avg_remaining_hp_by_team = total_remaining_hp_by_team
            .into_iter()
            .map(|value| value as f32 / runs as f32)
            .collect();
        let avg_hits_shield_survived = if shield_breaks > 0 {
            total_hits_shield_survived as f32 / shield_breaks as f32
        } else {
            0.0
        };
        let detailed = DetailedSimStats {
            duration_p10: histogram_percentile(&duration_histogram, 0.10) as u32,
            duration_p50: histogram_percentile(&duration_histogram, 0.50) as u32,
            duration_p90: histogram_percentile(&duration_histogram, 0.90) as u32,
            duration_p99: histogram_percentile(&duration_histogram, 0.99) as u32,
            teams: detailed_accumulators
                .into_iter()
                .map(|accumulator| accumulator.finish(runs, total_seconds))
                .collect(),
        };
        BulkSimResult {
            wins,
            ties,
            shields_present,
            shield_breaks,
            avg_hits_shield_survived,
            avg_duration,
            shortest_duration: if shortest_duration == u32::MAX {
                0
            } else {
                shortest_duration
            },
            longest_duration,
            fights_with_second_charge,
            fights_with_trauma,
            fights_with_trauma_first_exchange,
            fights_with_knockback_20ft,
            fights_with_charge_within_20ft,
            instakills,
            instakills_by_team,
            highest_single_crit_hit,
            highest_single_noncrit_hit,
            highest_single_shield_hit,
            highest_single_crit_hit_by_team,
            highest_single_noncrit_hit_by_team,
            highest_single_shield_hit_by_team,
            avg_damage_dealt_by_team,
            avg_damage_taken_by_team,
            avg_damage_rolled_by_team,
            avg_damage_landed_by_team,
            avg_remaining_hp_by_team,
            max_total_knockback_one_side_ft,
            avg_max_knockback_one_side_ft: total_max_knockback_one_side_ft as f32 / runs as f32,
            detailed,
        }
    }

    pub fn merge(&mut self, other: Self) {
        if other.runs == 0 {
            return;
        }
        if self.runs == 0 {
            *self = other;
            return;
        }
        assert_eq!(self.wins.len(), other.wins.len());
        self.runs += other.runs;
        for (a, b) in self.wins.iter_mut().zip(other.wins) {
            *a += b;
        }
        self.ties += other.ties;
        self.shields_present |= other.shields_present;
        self.shield_breaks += other.shield_breaks;
        self.total_hits_shield_survived += other.total_hits_shield_survived;
        self.total_seconds += other.total_seconds;
        self.shortest_duration = self.shortest_duration.min(other.shortest_duration);
        self.longest_duration = self.longest_duration.max(other.longest_duration);
        self.fights_with_second_charge += other.fights_with_second_charge;
        self.fights_with_trauma += other.fights_with_trauma;
        self.fights_with_trauma_first_exchange += other.fights_with_trauma_first_exchange;
        self.fights_with_knockback_20ft += other.fights_with_knockback_20ft;
        self.fights_with_charge_within_20ft += other.fights_with_charge_within_20ft;
        self.instakills += other.instakills;
        self.highest_single_crit_hit = self
            .highest_single_crit_hit
            .max(other.highest_single_crit_hit);
        self.highest_single_noncrit_hit = self
            .highest_single_noncrit_hit
            .max(other.highest_single_noncrit_hit);
        self.highest_single_shield_hit = self
            .highest_single_shield_hit
            .max(other.highest_single_shield_hit);
        for (a, b) in self
            .highest_single_crit_hit_by_team
            .iter_mut()
            .zip(other.highest_single_crit_hit_by_team)
        {
            *a = (*a).max(b);
        }
        for (a, b) in self
            .highest_single_noncrit_hit_by_team
            .iter_mut()
            .zip(other.highest_single_noncrit_hit_by_team)
        {
            *a = (*a).max(b);
        }
        for (a, b) in self
            .highest_single_shield_hit_by_team
            .iter_mut()
            .zip(other.highest_single_shield_hit_by_team)
        {
            *a = (*a).max(b);
        }
        for (a, b) in self
            .instakills_by_team
            .iter_mut()
            .zip(other.instakills_by_team)
        {
            *a += b;
        }
        for (a, b) in self
            .total_damage_dealt_by_team
            .iter_mut()
            .zip(other.total_damage_dealt_by_team)
        {
            *a += b;
        }
        for (a, b) in self
            .total_damage_taken_by_team
            .iter_mut()
            .zip(other.total_damage_taken_by_team)
        {
            *a += b;
        }
        for (a, b) in self
            .total_damage_rolled_dealt_by_team
            .iter_mut()
            .zip(other.total_damage_rolled_dealt_by_team)
        {
            *a += b;
        }
        for (a, b) in self
            .total_damage_landed_dealt_by_team
            .iter_mut()
            .zip(other.total_damage_landed_dealt_by_team)
        {
            *a += b;
        }
        for (a, b) in self
            .damage_rolls_dealt_by_team
            .iter_mut()
            .zip(other.damage_rolls_dealt_by_team)
        {
            *a += b;
        }
        for (a, b) in self
            .total_remaining_hp_by_team
            .iter_mut()
            .zip(other.total_remaining_hp_by_team)
        {
            *a += b;
        }
        self.max_total_knockback_one_side_ft = self
            .max_total_knockback_one_side_ft
            .max(other.max_total_knockback_one_side_ft);
        self.total_max_knockback_one_side_ft += other.total_max_knockback_one_side_ft;
        merge_histogram(&mut self.duration_histogram, other.duration_histogram);
        for (a, b) in self
            .detailed_accumulators
            .iter_mut()
            .zip(other.detailed_accumulators)
        {
            a.merge(b);
        }
    }
}

impl DetailedTeamAccumulator {
    fn merge(&mut self, other: Self) {
        assert_eq!(self.team_id, other.team_id);
        merge_histogram(&mut self.damage_by_source, other.damage_by_source);
        self.wins += other.wins;
        self.attack_attempts += other.attack_attempts;
        self.direct_hits += other.direct_hits;
        self.shield_blocks += other.shield_blocks;
        self.hp_hits += other.hp_hits;
        self.knockbacks_10ft += other.knockbacks_10ft;
        self.critical_hits += other.critical_hits;
        self.kills += other.kills;
        self.eyesmite_available |= other.eyesmite_available;
        self.eyes_smote += other.eyes_smote;
        self.total_hp_damage += other.total_hp_damage;
        self.total_incoming_raw_damage += other.total_incoming_raw_damage;
        self.total_armor_prevented += other.total_armor_prevented;
        self.total_shield_prevented += other.total_shield_prevented;
        self.total_hp_damage_taken += other.total_hp_damage_taken;
        self.first_attack_seconds_total += other.first_attack_seconds_total;
        self.fights_with_attack += other.fights_with_attack;
        self.attack_interval_seconds_total += other.attack_interval_seconds_total;
        self.attack_intervals += other.attack_intervals;
        self.fights_with_trauma_inflicted += other.fights_with_trauma_inflicted;
        self.trauma_events_inflicted += other.trauma_events_inflicted;
        self.total_trauma_downtime_seconds += other.total_trauma_downtime_seconds;
        self.shield_fights += other.shield_fights;
        self.fights_with_shield_break += other.fights_with_shield_break;
        self.total_hits_shield_survived_before_break +=
            other.total_hits_shield_survived_before_break;
        self.shield_breaks += other.shield_breaks;
        merge_histogram(
            &mut self.shield_hits_before_break_histogram,
            other.shield_hits_before_break_histogram,
        );
        self.shield_break_seconds_total += other.shield_break_seconds_total;
        merge_histogram(
            &mut self.shield_break_time_histogram,
            other.shield_break_time_histogram,
        );
        self.winning_hp_total += other.winning_hp_total;
        merge_histogram(&mut self.winning_hp_histogram, other.winning_hp_histogram);
        self.winning_duration_seconds_total += other.winning_duration_seconds_total;
        merge_histogram(
            &mut self.winning_duration_seconds_histogram,
            other.winning_duration_seconds_histogram,
        );
        merge_histogram(&mut self.hp_damage_histogram, other.hp_damage_histogram);
    }
}

fn merge_histogram<K: Ord>(target: &mut BTreeMap<K, u64>, source: BTreeMap<K, u64>) {
    for (key, count) in source {
        *target.entry(key).or_default() += count;
    }
}

/// Each fight owns its random stream, independent of scheduling and worker count.
/// The legacy bulk API retains its original continuous stream for saved seeds.
pub fn bulk_simulate_batch(
    config: SimConfig,
    combatants: Vec<Combatant>,
    runs: u32,
    max_seconds: u32,
    seed: u64,
    first_run: u32,
    keep_running: impl FnMut(u32) -> bool,
) -> Option<BulkSimBatch> {
    bulk_simulate_batch_inner(
        config,
        combatants,
        runs,
        max_seconds,
        seed,
        Some(first_run),
        keep_running,
    )
}

// JSON object keys cannot represent structured damage sources.
pub(super) mod source_totals {
    use super::*;
    use serde::{Deserialize, Serialize};
    pub fn serialize<S: serde::Serializer>(
        map: &BTreeMap<DamageSource, u64>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        map.iter().collect::<Vec<_>>().serialize(serializer)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<BTreeMap<DamageSource, u64>, D::Error> {
        Ok(Vec::<(DamageSource, u64)>::deserialize(deserializer)?
            .into_iter()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consecutive_batch_seeds_do_not_reuse_the_same_fights() {
        for seed in [0, 1, 9876, u64::MAX] {
            let current: HashSet<_> = (0..10_000).map(|index| fight_seed(seed, index)).collect();
            let next: HashSet<_> = (0..10_000)
                .map(|index| fight_seed(seed.wrapping_add(1), index))
                .collect();
            assert_eq!(current.len(), 10_000);
            assert!(current.is_disjoint(&next));
        }
    }

    #[test]
    fn merging_keeps_weighted_medians_and_damage_percentiles() {
        let make = |runs, duration, hp, damage| BulkSimBatch {
            runs,
            wins: vec![runs],
            total_seconds: u64::from(runs) * duration,
            duration_histogram: BTreeMap::from([(duration, u64::from(runs))]),
            detailed_accumulators: vec![DetailedTeamAccumulator {
                wins: runs,
                winning_hp_total: u64::from(runs) * hp,
                winning_hp_histogram: BTreeMap::from([(hp, u64::from(runs))]),
                hp_damage_histogram: BTreeMap::from([(damage, u64::from(runs))]),
                ..Default::default()
            }],
            ..Default::default()
        };
        let mut merged = make(9, 10, 20, 3);
        merged.merge(make(1, 100, 200, 99));
        let result = merged.finish();
        assert_eq!(result.avg_duration, 19.0);
        assert_eq!(result.detailed.duration_p50, 10);
        assert_eq!(result.detailed.duration_p90, 10);
        assert_eq!(result.detailed.duration_p99, 100);
        let team = &result.detailed.teams[0];
        assert_eq!(team.avg_winning_hp, Some(38.0));
        assert_eq!(team.median_winning_hp, Some(20));
        assert_eq!(team.hp_damage_p50, 3);
        assert_eq!(team.hp_damage_p90, 3);
        assert_eq!(team.hp_damage_p99, 99);
    }

    #[test]
    fn large_knockback_totals_do_not_depend_on_partition_rounding() {
        let batch = |runs, total| BulkSimBatch {
            runs,
            wins: vec![runs],
            total_max_knockback_one_side_ft: total,
            ..Default::default()
        };
        let total = 16_777_216.0 + 5.0 + 5.0;
        let serial = batch(3, total).finish();
        let mut merged = batch(1, 16_777_216.0);
        merged.merge(batch(1, 5.0));
        merged.merge(batch(1, 5.0));
        assert_eq!(merged.total_max_knockback_one_side_ft, total);
        assert_eq!(
            merged.finish().avg_max_knockback_one_side_ft,
            serial.avg_max_knockback_one_side_ft
        );
    }
}
