use crate::autobattler::constants::STAT_COUNT;
use crate::autobattler::state::PointPool;
use crate::character::{AbilityScore, charisma_honor_adjustment, looks_honor_adjustment};
use crate::core::types::{RaceSpec, TalentSelection, TalentSpec};
use crate::game_logic::TalentCatalog;

pub fn clamp_stat_adjustment(base: u8, delta: i32) -> u8 {
    let adjusted = base as i32 + delta;
    adjusted.clamp(1, 25) as u8
}

pub fn apply_stat_adjustment(score: &mut AbilityScore, delta: i32) {
    score.base = clamp_stat_adjustment(score.base, delta);
}

pub fn apply_percentile(score: &mut AbilityScore, delta: u8) {
    let total = score_total(score).saturating_add(delta as i32);
    let capped = total.clamp(0, max_stat_total());
    *score = score_from_total(capped);
}

pub fn subtract_percentile(score: &mut AbilityScore, delta: u8) {
    let total = score_total(score).saturating_sub(delta as i32);
    let capped = total.max(0);
    *score = score_from_total(capped);
}

pub fn stat_at_cap(score: &AbilityScore) -> bool {
    score_total(score) >= max_stat_total()
}

pub fn bp_increment(score: &AbilityScore) -> u8 {
    let total = score_total(score);
    if total < stat_total_for(10, 1) {
        10
    } else if total < stat_total_for(16, 1) {
        5
    } else {
        3
    }
}

pub fn score_total(score: &AbilityScore) -> i32 {
    let base = score.base.max(1) as i32;
    let percentile = (score.percentile % 100) as i32;
    (base - 1) * 100 + percentile
}

pub fn score_from_total(total: i32) -> AbilityScore {
    let total = total.clamp(0, max_stat_total());
    let base = (total / 100 + 1).min(25) as u8;
    let percentile = (total % 100) as u8;
    AbilityScore::new(base, percentile)
}

pub fn format_percentile(value: u8) -> String {
    let value = value % 100;
    if value == 0 {
        "00".to_string()
    } else {
        format!("{:02}", value)
    }
}

fn max_stat_total() -> i32 {
    (25 - 1) * 100 + 99
}

fn stat_total_for(base: u8, percentile: u8) -> i32 {
    let base = base.max(1);
    let percentile = percentile % 100;
    (base as i32 - 1) * 100 + percentile as i32
}

pub struct HonorBreakdown {
    pub base: i32,
    pub looks_mod: i32,
    pub cha_mod: i32,
    pub total: i32,
}

pub fn starting_honor(
    stats: &[AbilityScore; STAT_COUNT],
    effective_charisma: u8,
) -> HonorBreakdown {
    let mut total = 0.0;
    for (idx, score) in stats.iter().enumerate() {
        let base = if idx == 6 {
            effective_charisma as f32
        } else {
            score.base as f32
        };
        let percentile = (score.percentile % 100) as f32 / 100.0;
        total += base + percentile;
    }
    let base = (total / STAT_COUNT as f32).floor() as i32;
    let looks_mod = looks_honor_adjustment(stats[5].base);
    let cha_mod = charisma_honor_adjustment(effective_charisma);
    let total = base + looks_mod + cha_mod;
    HonorBreakdown {
        base,
        looks_mod,
        cha_mod,
        total,
    }
}

pub fn format_score(score: AbilityScore) -> String {
    format!("{}/{}", score.base, format_percentile(score.percentile))
}

pub fn talent_display_label(selection: &TalentSelection, talent_catalog: &TalentCatalog) -> String {
    let talent_name = talent_catalog
        .entries()
        .iter()
        .find(|talent| talent.id == selection.id)
        .map(|talent| talent.name.as_str())
        .unwrap_or(selection.id.as_str());
    let mut details: Vec<String> = Vec::new();
    if let Some(weapon) = selection.weapon.as_ref() {
        details.push(weapon.clone());
    }
    if selection.rank > 1 {
        details.push(format!("rank {}", selection.rank));
    }
    if details.is_empty() {
        talent_name.to_string()
    } else {
        format!("{talent_name} ({})", details.join(", "))
    }
}

pub fn race_adjustment_summary(race: &RaceSpec) -> String {
    let mut parts = Vec::new();
    let adj = &race.ability_adjustments;
    if adj.strength != 0 {
        parts.push(format!("STR {:+}", adj.strength));
    }
    if adj.dexterity != 0 {
        parts.push(format!("DEX {:+}", adj.dexterity));
    }
    if adj.intelligence != 0 {
        parts.push(format!("INT {:+}", adj.intelligence));
    }
    if adj.wisdom != 0 {
        parts.push(format!("WIS {:+}", adj.wisdom));
    }
    if adj.constitution != 0 {
        parts.push(format!("CON {:+}", adj.constitution));
    }
    if adj.looks != 0 {
        parts.push(format!("LKS {:+}", adj.looks));
    }
    if adj.charisma != 0 {
        parts.push(format!("CHA {:+}", adj.charisma));
    }
    if parts.is_empty() {
        "No stat adjustments".to_string()
    } else {
        parts.join(", ")
    }
}

pub fn total_talent_costs(
    selections: &[TalentSelection],
    talent_catalog: &TalentCatalog,
) -> PointPool {
    let mut total = PointPool::default();
    for selection in selections {
        let Some(spec) = find_talent(talent_catalog, &selection.id) else {
            continue;
        };
        let cost = talent_cost_for_rank(spec, selection.rank.max(1));
        total = total.add(cost);
    }
    total
}

pub fn find_talent<'a>(talent_catalog: &'a TalentCatalog, id: &str) -> Option<&'a TalentSpec> {
    talent_catalog
        .entries()
        .iter()
        .find(|talent| talent.id == id)
}

pub fn talent_cost_for_rank(spec: &TalentSpec, rank: u8) -> PointPool {
    let rank = rank.max(1) as i32;
    PointPool {
        bp: spec.cost_bp.unwrap_or(0) as i32 * rank,
        lp: spec.cost_lp.unwrap_or(0) as i32 * rank,
        ap: 0,
        rp: spec.cost_rp.unwrap_or(0) as i32 * rank,
    }
}

pub fn max_affordable_rank(spec: &TalentSpec, budget: PointPool) -> u8 {
    let max_rank = spec.max_rank.max(1);
    for rank in (1..=max_rank).rev() {
        let cost = talent_cost_for_rank(spec, rank);
        if budget.can_afford(cost) {
            return rank;
        }
    }
    1
}

pub fn scaled_enemy_level(player_level: u8, run_depth: u32) -> u8 {
    let depth_bonus = (run_depth / 2) as u8;
    player_level.saturating_add(depth_bonus)
}

pub fn hobgoblin_level(name: &str) -> Option<u8> {
    let lower = name.to_ascii_lowercase();
    if lower == "hobgoblin" {
        Some(1)
    } else if let Some(rest) = lower.strip_prefix("hobgoblin ") {
        rest.trim().parse::<u8>().ok()
    } else {
        None
    }
}

pub fn stat_label(idx: usize) -> &'static str {
    if idx < STAT_COUNT {
        crate::autobattler::constants::STAT_LABELS[idx]
    } else {
        "?"
    }
}
