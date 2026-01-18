use crate::autobattler::constants::{STAT_COUNT};
use crate::autobattler::state::PointPool;
use crate::character::AbilityScore;
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
    let capped = total.clamp(1, 25 * 100);
    *score = score_from_total(capped);
}

pub fn subtract_percentile(score: &mut AbilityScore, delta: u8) {
    let total = score_total(score).saturating_sub(delta as i32);
    let capped = total.max(1);
    *score = score_from_total(capped);
}

pub fn stat_at_cap(score: &AbilityScore) -> bool {
    score_total(score) >= 25 * 100
}

pub fn bp_increment(score: &AbilityScore) -> u8 {
    if score.base < 10 {
        10
    } else if score.base >= 16 {
        3
    } else {
        5
    }
}

pub fn score_total(score: &AbilityScore) -> i32 {
    let base = score.base.max(1) as i32;
    let percentile = if score.percentile == 0 { 100 } else { score.percentile } as i32;
    (base - 1) * 100 + percentile
}

pub fn score_from_total(total: i32) -> AbilityScore {
    let total = total.max(1);
    let base = ((total - 1) / 100 + 1).min(25) as u8;
    let percentile = ((total - 1) % 100 + 1) as u8;
    AbilityScore::new(base, percentile)
}

pub fn format_percentile(value: u8) -> String {
    if value == 0 || value >= 100 {
        "00".to_string()
    } else {
        format!("{:02}", value)
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
