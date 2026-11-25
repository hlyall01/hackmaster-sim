use super::*;

/// Exact small integer bins for the hot path, with sparse storage for the long tail.
#[derive(Clone, Debug, Default)]
pub(super) struct DenseHistogram {
    common: Vec<u64>,
    overflow: BTreeMap<u64, u64>,
}

impl DenseHistogram {
    pub(super) fn add(&mut self, value: u64) {
        if value >= 256 {
            *self.overflow.entry(value).or_default() += 1;
        } else {
            let i = value as usize;
            if self.common.len() <= i {
                self.common.resize(i + 1, 0);
            }
            self.common[i] += 1;
        }
    }

    pub(super) fn finish(mut self) -> BTreeMap<u64, u64> {
        self.overflow.extend(
            self.common
                .into_iter()
                .enumerate()
                .filter(|(_, n)| *n > 0)
                .map(|(i, n)| (i as u64, n)),
        );
        self.overflow
    }
}

#[derive(Clone, Debug)]
pub(super) struct BulkMetrics {
    pub(super) teams: Vec<DetailedTeamAccumulator>,
    fighter_teams: Vec<usize>,
    sources: Vec<ImpactSource>,
    // Slot and built-in ability IDs survive fight resets. Weapon swaps verify identity.
    source_ids: Vec<[Option<usize>; 5]>,
    shield_hits: Vec<DenseHistogram>,
    shield_times: Vec<DenseHistogram>,
    damage_totals: Vec<Vec<Option<u64>>>,
    pub(super) first_attack: Vec<Option<u32>>,
    last_attack: Vec<Option<u32>>,
    pub(super) trauma_inflicted: Vec<bool>,
    hp_damage: Vec<DenseHistogram>,
    pub(super) winning_hp: Vec<DenseHistogram>,
    pub(super) winning_time: Vec<DenseHistogram>,
}

impl BulkMetrics {
    pub(super) fn new(teams: Vec<DetailedTeamAccumulator>, fighter_teams: Vec<usize>) -> Self {
        let n = teams.len();
        Self {
            last_attack: vec![None; fighter_teams.len()],
            source_ids: vec![[None; 5]; fighter_teams.len()],
            shield_hits: vec![DenseHistogram::default(); n],
            shield_times: vec![DenseHistogram::default(); n],
            teams,
            fighter_teams,
            sources: Vec::new(),
            damage_totals: vec![Vec::new(); n],
            first_attack: vec![None; n],
            trauma_inflicted: vec![false; n],
            hp_damage: vec![DenseHistogram::default(); n],
            winning_hp: vec![DenseHistogram::default(); n],
            winning_time: vec![DenseHistogram::default(); n],
        }
    }

    pub(super) fn begin_fight(&mut self) {
        self.first_attack.fill(None);
        self.last_attack.fill(None);
        self.trauma_inflicted.fill(false);
    }

    pub(super) fn record_attempt(&mut self, attacker: usize, source: ImpactSource, time: u32, damage: u32) -> usize {
        let attacker_team_idx = self.fighter_teams[attacker];
        let slot = match &source {
            ImpactSource::Weapon(_, WeaponSlot::Primary) => Some(0),
            ImpactSource::Weapon(_, WeaponSlot::Secondary) => Some(1),
            ImpactSource::Eyesmite => Some(2), ImpactSource::Punch => Some(3), ImpactSource::Echo => Some(4),
            _ => None,
        };
        let cached = slot.and_then(|s| self.source_ids[attacker][s])
            .filter(|&id| self.sources[id].same_identity(&source));
        let source_id = cached.unwrap_or_else(|| {
            let id = self.sources.iter().position(|s| s.same_identity(&source)).unwrap_or_else(|| {
                let id = self.sources.len();
                self.sources.push(source);
                for totals in &mut self.damage_totals { totals.push(None); }
                id
            });
            if let Some(s) = slot { self.source_ids[attacker][s] = Some(id); }
            id
        });
        *self.damage_totals[attacker_team_idx][source_id].get_or_insert(0) +=
            u64::from(damage);
        let attacker_stats = &mut self.teams[attacker_team_idx];
        attacker_stats.attack_attempts = attacker_stats.attack_attempts.saturating_add(1);
        self.first_attack[attacker_team_idx] = Some(
            self.first_attack[attacker_team_idx]
                .map_or(time, |current| current.min(time)),
        );
        if let Some(last_time) = self.last_attack[attacker] {
            attacker_stats.attack_interval_seconds_total = attacker_stats
                .attack_interval_seconds_total
                .saturating_add(u64::from(time.saturating_sub(last_time)));
            attacker_stats.attack_intervals = attacker_stats.attack_intervals.saturating_add(1);
        }
        self.last_attack[attacker] = Some(time);

        attacker_team_idx
    }

    pub(super) fn record(&mut self, sample: AttackMetricSample) {
        let attacker_team_idx = self.record_attempt(sample.attacker_idx, sample.damage_source, sample.time, sample.hp_damage);
        let defender_team_idx = self.fighter_teams[sample.defender_idx];
        let attacker_stats = &mut self.teams[attacker_team_idx];
        if sample.direct_hit {
            attacker_stats.direct_hits = attacker_stats.direct_hits.saturating_add(1);
        }
        if sample.shield_block {
            attacker_stats.shield_blocks = attacker_stats.shield_blocks.saturating_add(1);
        }
        if sample.hp_damage > 0 {
            attacker_stats.hp_hits = attacker_stats.hp_hits.saturating_add(1);
            self.hp_damage[attacker_team_idx].add(u64::from(sample.hp_damage));
        }
        if sample.critical {
            attacker_stats.critical_hits = attacker_stats.critical_hits.saturating_add(1);
        }
        if sample.knockback_ft >= 10.0 {
            attacker_stats.knockbacks_10ft = attacker_stats.knockbacks_10ft.saturating_add(1);
        }
        if sample.killing_blow {
            attacker_stats.kills = attacker_stats.kills.saturating_add(1);
        }
        attacker_stats.total_hp_damage = attacker_stats
            .total_hp_damage
            .saturating_add(u64::from(sample.hp_damage));
        if sample.trauma_applied {
            attacker_stats.trauma_events_inflicted =
                attacker_stats.trauma_events_inflicted.saturating_add(1);
            self.trauma_inflicted[attacker_team_idx] = true;
        }
        let defender_stats = &mut self.teams[defender_team_idx];
        defender_stats.total_incoming_raw_damage = defender_stats
            .total_incoming_raw_damage
            .saturating_add(u64::from(sample.raw_damage));
        defender_stats.total_armor_prevented = defender_stats
            .total_armor_prevented
            .saturating_add(u64::from(sample.armor_prevented));
        defender_stats.total_shield_prevented = defender_stats
            .total_shield_prevented
            .saturating_add(u64::from(sample.shield_prevented));
        defender_stats.total_hp_damage_taken = defender_stats
            .total_hp_damage_taken
            .saturating_add(u64::from(sample.hp_damage));
        if sample.shield_broken {
            defender_stats.shield_breaks = defender_stats.shield_breaks.saturating_add(1);
            defender_stats.total_hits_shield_survived_before_break = defender_stats
                .total_hits_shield_survived_before_break
                .saturating_add(u64::from(sample.shield_hits_survived_before_break));
            self.shield_hits[defender_team_idx].add(u64::from(sample.shield_hits_survived_before_break));
            defender_stats.shield_break_seconds_total = defender_stats
                .shield_break_seconds_total
                .saturating_add(u64::from(sample.time));
            self.shield_times[defender_team_idx].add(u64::from(sample.time));
        }
    }

    pub(super) fn finish(self) -> Vec<DetailedTeamAccumulator> {
        let mut teams = self.teams;
        for (i, totals) in self.damage_totals.into_iter().enumerate() {
            for (source, damage) in self.sources.iter().zip(totals) {
                if let Some(damage) = damage {
                    *teams[i]
                        .damage_by_source
                        .entry(source.describe())
                        .or_default() += damage;
                }
            }
        }
        for ((team, hits), times) in teams.iter_mut().zip(self.shield_hits).zip(self.shield_times) {
            team.shield_hits_before_break_histogram = hits.finish();
            team.shield_break_time_histogram = times.finish();
        }
        for (((team, damage), hp), duration) in teams
            .iter_mut()
            .zip(self.hp_damage)
            .zip(self.winning_hp)
            .zip(self.winning_time)
        {
            team.hp_damage_histogram = damage.finish();
            team.winning_hp_histogram = hp.finish();
            team.winning_duration_seconds_histogram = duration.finish();
        }
        teams
    }
}
