use std::collections::{HashMap, HashSet};

use crate::character::{MasteryAspect, WeaponGroup, mastery_threshold};
use crate::core::rng::SimRng;
use crate::core::rules::roll_damage_expr;
use crate::core::sim::{CombatEvent, CombatEventKind, WeaponSlot};
use crate::core::types::{PlayerProfile, TalentSelection, WeaponMasteryProgress};
use crate::game_logic::{PlayerConfig, ShieldCatalog, WeaponCatalog, is_ranged_weapon};

const DEFAULT_MAX_TIER: i32 = 5;
const SUPREMACY_MAX_TIER: i32 = 6;
const PROFICIENCY_REFUND_BP: i32 = 1;

#[derive(Clone, Debug)]
pub struct WeaponMasteryRow {
    pub group: WeaponGroup,
    pub label: &'static str,
    pub experience: u32,
    pub threshold: u32,
    pub unspent_points: u32,
    pub proficient: bool,
    pub at_max_tier: bool,
    pub max_tier: i32,
    pub completed_tiers: i32,
    pub attack: i32,
    pub defense: i32,
    pub damage: i32,
    pub speed: i32,
}

pub fn weapon_group_label(group: WeaponGroup) -> &'static str {
    match group {
        WeaponGroup::Unarmed => "Unarmed",
        WeaponGroup::Axes => "Axes",
        WeaponGroup::Basic => "Basic",
        WeaponGroup::Blunt => "Blunt",
        WeaponGroup::Bows => "Bows",
        WeaponGroup::Crossbows => "Crossbows",
        WeaponGroup::Double => "Double",
        WeaponGroup::Ensnaring => "Ensnaring",
        WeaponGroup::Lashes => "Lashes",
        WeaponGroup::LargeSwords => "Large swords",
        WeaponGroup::SmallSwords => "Small swords",
        WeaponGroup::Polearms => "Polearms",
        WeaponGroup::Spears => "Spears",
        WeaponGroup::Shields => "Shields",
    }
}

pub fn weapon_group_from_label(value: &str) -> Option<WeaponGroup> {
    let normalized = normalize_token(value);
    match normalized.as_str() {
        "unarmed" => Some(WeaponGroup::Unarmed),
        "axes" => Some(WeaponGroup::Axes),
        "basic" => Some(WeaponGroup::Basic),
        "blunt" => Some(WeaponGroup::Blunt),
        "bows" => Some(WeaponGroup::Bows),
        "crossbows" => Some(WeaponGroup::Crossbows),
        "double" => Some(WeaponGroup::Double),
        "ensnaring" => Some(WeaponGroup::Ensnaring),
        "lashes" => Some(WeaponGroup::Lashes),
        "large swords" | "large sword" => Some(WeaponGroup::LargeSwords),
        "small swords" | "small sword" => Some(WeaponGroup::SmallSwords),
        "polearms" | "polearm" => Some(WeaponGroup::Polearms),
        "spears" | "spear" => Some(WeaponGroup::Spears),
        "shields" | "shield" => Some(WeaponGroup::Shields),
        _ => None,
    }
}

pub fn total_unspent_mastery_points(profile: &PlayerProfile) -> u32 {
    profile
        .weapon_masteries
        .iter()
        .map(|entry| entry.unspent_points)
        .sum()
}

pub fn mastery_rows(
    profile: &PlayerProfile,
    weapon_catalog: &WeaponCatalog,
) -> Vec<WeaponMasteryRow> {
    let mut rows: Vec<WeaponMasteryRow> = profile
        .weapon_masteries
        .iter()
        .filter_map(|entry| {
            let group = weapon_group_from_label(&entry.group)?;
            let threshold = threshold_for_group(
                profile,
                group,
                default_ranged_usage_for_group(group),
                weapon_catalog,
            );
            let max_tier = max_tier_for_group(profile, group, weapon_catalog);
            let completed = completed_tiers(entry, group == WeaponGroup::Shields);
            let at_max_tier = group_at_max_tier(entry, group, max_tier);
            Some(WeaponMasteryRow {
                group,
                label: weapon_group_label(group),
                experience: entry.experience,
                threshold,
                unspent_points: entry.unspent_points,
                proficient: is_proficient_with_group(profile, group, weapon_catalog),
                at_max_tier,
                max_tier,
                completed_tiers: completed,
                attack: entry.attack,
                defense: entry.defense,
                damage: entry.damage,
                speed: entry.speed,
            })
        })
        .collect();
    rows.sort_by(|a, b| a.label.cmp(b.label));
    rows
}

pub fn seed_profile_masteries_from_config(
    profile: &mut PlayerProfile,
    config: &PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    shield_catalog: &ShieldCatalog,
) {
    if let Some(weapon) = weapon_catalog.get(config.weapon_id) {
        let idx = ensure_progress_index(profile, weapon.group);
        let entry = &mut profile.weapon_masteries[idx];
        entry.attack = entry.attack.max(config.mastery_attack.max(0));
        entry.defense = entry.defense.max(config.mastery_defense.max(0));
        entry.damage = entry.damage.max(config.mastery_damage.max(0));
        entry.speed = entry.speed.max(config.mastery_speed.max(0));
        let tiers = completed_tiers(entry, false);
        entry.free_proficiency_tiers_claimed = entry.free_proficiency_tiers_claimed.max(tiers);
    }

    let shield_equipped = shield_catalog
        .get(config.shield_id)
        .and_then(|entry| entry.shield.as_ref())
        .is_some();
    if shield_equipped {
        let idx = ensure_progress_index(profile, WeaponGroup::Shields);
        let entry = &mut profile.weapon_masteries[idx];
        entry.defense = entry.defense.max(config.shield_mastery_defense.max(0));
        entry.speed = entry.speed.max(config.shield_mastery_speed.max(0));
        let tiers = completed_tiers(entry, true);
        entry.free_proficiency_tiers_claimed = entry.free_proficiency_tiers_claimed.max(tiers);
    }
}

pub fn apply_profile_masteries_to_config(
    profile: &PlayerProfile,
    config: &mut PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    shield_catalog: &ShieldCatalog,
) {
    config.mastery_attack = 0;
    config.mastery_defense = 0;
    config.mastery_damage = 0;
    config.mastery_speed = 0;
    config.shield_mastery_defense = 0;
    config.shield_mastery_speed = 0;

    if let Some(weapon) = weapon_catalog.get(config.weapon_id) {
        if let Some(progress) = progress_for_group(profile, weapon.group) {
            config.mastery_attack = progress.attack.max(0);
            config.mastery_defense = progress.defense.max(0);
            config.mastery_damage = progress.damage.max(0);
            config.mastery_speed = progress.speed.max(0);
        }
    }

    let shield_equipped = shield_catalog
        .get(config.shield_id)
        .and_then(|entry| entry.shield.as_ref())
        .is_some();
    if shield_equipped {
        if let Some(progress) = progress_for_group(profile, WeaponGroup::Shields) {
            config.shield_mastery_defense = progress.defense.max(0);
            config.shield_mastery_speed = progress.speed.max(0);
        }
    }
}

pub fn spend_mastery_point(
    profile: &mut PlayerProfile,
    group: WeaponGroup,
    aspect: MasteryAspect,
    weapon_catalog: &WeaponCatalog,
) -> Result<Vec<String>, String> {
    if !is_proficient_with_group(profile, group, weapon_catalog) {
        return Err(format!(
            "Cannot spend {} mastery points without a matching proficiency.",
            weapon_group_label(group)
        ));
    }
    if group == WeaponGroup::Shields
        && matches!(aspect, MasteryAspect::Attack | MasteryAspect::Damage)
    {
        return Err("Shields only have Defense and Speed mastery.".to_string());
    }

    let max_tier = max_tier_for_group(profile, group, weapon_catalog);
    let idx = ensure_progress_index(profile, group);
    let mut lines = Vec::new();

    let (current, gate_floor) = {
        let entry = &profile.weapon_masteries[idx];
        let current = match aspect {
            MasteryAspect::Attack => entry.attack,
            MasteryAspect::Defense => entry.defense,
            MasteryAspect::Damage => entry.damage,
            MasteryAspect::Speed => entry.speed,
        };
        let floor = if group == WeaponGroup::Shields {
            entry.defense.min(entry.speed)
        } else {
            entry
                .attack
                .min(entry.defense)
                .min(entry.damage)
                .min(entry.speed)
        };
        (current, floor)
    };
    if current >= max_tier {
        return Err(format!(
            "{} is already at max tier (+{}).",
            weapon_group_label(group),
            max_tier
        ));
    }
    if current > 0 && gate_floor < current {
        return Err(format!(
            "Finish tier +{} in all {} aspects before taking +{} in {}.",
            current,
            if group == WeaponGroup::Shields {
                "shield"
            } else {
                "weapon"
            },
            current + 1,
            aspect_label(aspect)
        ));
    }

    {
        let entry = &mut profile.weapon_masteries[idx];
        if entry.unspent_points == 0 {
            return Err(format!(
                "No unspent {} mastery points.",
                weapon_group_label(group)
            ));
        }
        entry.unspent_points = entry.unspent_points.saturating_sub(1);
        match aspect {
            MasteryAspect::Attack => entry.attack += 1,
            MasteryAspect::Defense => entry.defense += 1,
            MasteryAspect::Damage => entry.damage += 1,
            MasteryAspect::Speed => entry.speed += 1,
        }
    }

    lines.push(format!(
        "{} {} mastery increased to +{}.",
        weapon_group_label(group),
        aspect_label(aspect),
        current + 1
    ));

    let (tiers_after, tiers_claimed) = {
        let entry = &profile.weapon_masteries[idx];
        (
            completed_tiers(entry, group == WeaponGroup::Shields),
            entry.free_proficiency_tiers_claimed,
        )
    };
    if tiers_after > tiers_claimed {
        for _tier in (tiers_claimed + 1)..=tiers_after {
            if group != WeaponGroup::Shields {
                match grant_group_proficiency_or_refund(profile, group, weapon_catalog) {
                    GroupReward::Proficiency(name) => {
                        lines.push(format!(
                            "Free proficiency gained in {}: {}.",
                            weapon_group_label(group),
                            name
                        ));
                    }
                    GroupReward::RefundBp(amount) => {
                        lines.push(format!(
                            "{} already fully proficient; BP +{} refunded.",
                            weapon_group_label(group),
                            amount
                        ));
                    }
                }
            }
        }
        if let Some(entry) = profile.weapon_masteries.get_mut(idx) {
            entry.free_proficiency_tiers_claimed = tiers_after;
        }
    }

    Ok(lines)
}

pub fn apply_weapon_experience_from_fight(
    profile: &mut PlayerProfile,
    player_config: &PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    shield_catalog: &ShieldCatalog,
    events: &[CombatEvent],
    _opponent_level: u8,
    rng: &mut SimRng,
) -> Vec<String> {
    let used_groups = collect_used_groups(player_config, weapon_catalog, shield_catalog, events);
    let mut filtered: Vec<(WeaponGroup, bool)> = used_groups
        .into_iter()
        .filter(|(group, _used_ranged)| !group_is_maxed(profile, *group, weapon_catalog))
        .collect();
    filtered.sort_by(|a, b| weapon_group_label(a.0).cmp(weapon_group_label(b.0)));

    if filtered.is_empty() {
        return vec!["No weapon experience gained (no eligible weapon groups used).".to_string()];
    }

    let base_dice = base_wexp_dice_for_group_count(filtered.len());
    let mut lines = Vec::new();
    for (group, used_ranged) in filtered {
        let bonus_dice = wexp_bonus_dice(profile, group, used_ranged);
        let total_dice = (base_dice + bonus_dice).max(1);
        let expr = format!("{total_dice}d6p");
        let gained = roll_damage_expr(&expr, rng, false).max(0) as u32;
        let threshold = threshold_for_group(profile, group, used_ranged, weapon_catalog);
        let idx = ensure_progress_index(profile, group);
        let entry = &mut profile.weapon_masteries[idx];
        entry.experience = entry.experience.saturating_add(gained);
        let mut points_gained = 0u32;
        while entry.experience >= threshold {
            entry.experience = entry.experience.saturating_sub(threshold);
            entry.unspent_points = entry.unspent_points.saturating_add(1);
            points_gained = points_gained.saturating_add(1);
        }

        let mut line = format!(
            "{} wexp +{} ({}; carry {}/{}).",
            weapon_group_label(group),
            gained,
            expr,
            entry.experience,
            threshold
        );
        if points_gained > 0 {
            line.push_str(&format!(
                " Mastery points +{} (unspent {}).",
                points_gained, entry.unspent_points
            ));
            if !is_proficient_with_group(profile, group, weapon_catalog) {
                line.push_str(" Spend blocked until proficient.");
            }
        }
        lines.push(line);
    }
    lines
}

fn aspect_label(aspect: MasteryAspect) -> &'static str {
    match aspect {
        MasteryAspect::Attack => "Attack",
        MasteryAspect::Defense => "Defense",
        MasteryAspect::Damage => "Damage",
        MasteryAspect::Speed => "Speed",
    }
}

fn base_wexp_dice_for_group_count(group_count: usize) -> i32 {
    match group_count {
        0 => 0,
        1 => 8,
        2 => 6,
        3 => 5,
        4 => 4,
        _ => 3,
    }
}

fn wexp_bonus_dice(profile: &PlayerProfile, group: WeaponGroup, used_ranged: bool) -> i32 {
    let mut bonus = 0;
    bonus += 2 * i32::from(talent_rank_for_group(
        profile,
        "weapon_specialization",
        group,
    ));
    if used_ranged {
        bonus += 2 * i32::from(talent_rank_for_group(
            profile,
            "ranged_weapon_specialization",
            group,
        ));
    }
    bonus
}

fn threshold_for_group(
    profile: &PlayerProfile,
    group: WeaponGroup,
    used_ranged: bool,
    _weapon_catalog: &WeaponCatalog,
) -> u32 {
    let base = if group == WeaponGroup::Shields {
        200.0
    } else if used_ranged || default_ranged_usage_for_group(group) {
        150.0
    } else {
        100.0
    };
    let completed = progress_for_group(profile, group)
        .map(|entry| completed_tiers(entry, group == WeaponGroup::Shields))
        .unwrap_or(0);
    let mut threshold = mastery_threshold(base, profile.base_stats.intelligence, completed);
    let focus_rank = talent_rank_for_group(profile, "weapon_focus", group) as i32;
    if focus_rank > 0 {
        threshold *= 0.8_f32.powi(focus_rank);
    }
    threshold.ceil().max(1.0) as u32
}

fn default_ranged_usage_for_group(group: WeaponGroup) -> bool {
    matches!(
        group,
        WeaponGroup::Bows | WeaponGroup::Crossbows | WeaponGroup::Ensnaring
    )
}

fn max_tier_for_group(
    profile: &PlayerProfile,
    group: WeaponGroup,
    weapon_catalog: &WeaponCatalog,
) -> i32 {
    let mut cap = DEFAULT_MAX_TIER;
    if talent_rank_for_group(profile, "weapon_supremacy", group) > 0 {
        cap = SUPREMACY_MAX_TIER;
    }
    if group_contains_ranged_weapon(group, weapon_catalog)
        && talent_rank_for_group(profile, "ranged_weapon_supremacy", group) > 0
    {
        cap = SUPREMACY_MAX_TIER;
    }
    cap
}

fn group_contains_ranged_weapon(group: WeaponGroup, weapon_catalog: &WeaponCatalog) -> bool {
    weapon_catalog
        .entries()
        .iter()
        .any(|weapon| weapon.group == group && is_ranged_weapon(weapon))
}

fn group_is_maxed(
    profile: &PlayerProfile,
    group: WeaponGroup,
    weapon_catalog: &WeaponCatalog,
) -> bool {
    let Some(entry) = progress_for_group(profile, group) else {
        return false;
    };
    let max_tier = max_tier_for_group(profile, group, weapon_catalog);
    group_at_max_tier(entry, group, max_tier)
}

fn group_at_max_tier(entry: &WeaponMasteryProgress, group: WeaponGroup, max_tier: i32) -> bool {
    if group == WeaponGroup::Shields {
        entry.defense >= max_tier && entry.speed >= max_tier
    } else {
        entry.attack >= max_tier
            && entry.defense >= max_tier
            && entry.damage >= max_tier
            && entry.speed >= max_tier
    }
}

fn completed_tiers(entry: &WeaponMasteryProgress, shield_group: bool) -> i32 {
    if shield_group {
        entry.defense.min(entry.speed).max(0)
    } else {
        entry
            .attack
            .min(entry.defense)
            .min(entry.damage)
            .min(entry.speed)
            .max(0)
    }
}

fn collect_used_groups(
    player_config: &PlayerConfig,
    weapon_catalog: &WeaponCatalog,
    shield_catalog: &ShieldCatalog,
    events: &[CombatEvent],
) -> HashMap<WeaponGroup, bool> {
    let mut used: HashMap<WeaponGroup, bool> = HashMap::new();
    let primary_group = weapon_catalog
        .get(player_config.weapon_id)
        .map(|weapon| weapon.group);
    let offhand_group = player_config
        .offhand_weapon_id
        .and_then(|id| weapon_catalog.get(id))
        .map(|weapon| weapon.group);
    for event in events {
        if event.attacker_idx != 0 {
            continue;
        }
        let CombatEventKind::Attack(attack) = &event.kind else {
            continue;
        };
        let group = match attack.weapon_slot {
            WeaponSlot::Primary => primary_group,
            WeaponSlot::Secondary => offhand_group.or(primary_group),
        };
        let Some(group) = group else {
            continue;
        };
        used.entry(group)
            .and_modify(|is_ranged| *is_ranged |= attack.is_ranged)
            .or_insert(attack.is_ranged);
    }

    let shield_equipped = shield_catalog
        .get(player_config.shield_id)
        .and_then(|entry| entry.shield.as_ref())
        .is_some();
    if shield_equipped {
        let shield_used = events.iter().any(|event| {
            event.defender_idx == 0 && matches!(event.kind, CombatEventKind::Attack(_))
        });
        if shield_used {
            used.entry(WeaponGroup::Shields).or_insert(false);
        }
    }
    used
}

fn talent_rank_for_group(profile: &PlayerProfile, talent_id: &str, group: WeaponGroup) -> u8 {
    profile
        .talents
        .iter()
        .filter(|selection| selection.id.eq_ignore_ascii_case(talent_id))
        .filter(|selection| selection_matches_group(selection, group))
        .map(|selection| selection.rank.max(1))
        .max()
        .unwrap_or(0)
}

fn selection_matches_group(selection: &TalentSelection, group: WeaponGroup) -> bool {
    selection
        .weapon
        .as_deref()
        .and_then(weapon_group_from_label)
        .map(|selected_group| selected_group == group)
        .unwrap_or(false)
}

fn progress_for_group(
    profile: &PlayerProfile,
    group: WeaponGroup,
) -> Option<&WeaponMasteryProgress> {
    profile
        .weapon_masteries
        .iter()
        .find(|entry| weapon_group_from_label(&entry.group) == Some(group))
}

fn ensure_progress_index(profile: &mut PlayerProfile, group: WeaponGroup) -> usize {
    if let Some((idx, _)) = profile
        .weapon_masteries
        .iter()
        .enumerate()
        .find(|(_, entry)| weapon_group_from_label(&entry.group) == Some(group))
    {
        idx
    } else {
        profile.weapon_masteries.push(WeaponMasteryProgress {
            group: weapon_group_label(group).to_string(),
            ..WeaponMasteryProgress::default()
        });
        profile.weapon_masteries.len().saturating_sub(1)
    }
}

fn is_proficient_with_group(
    profile: &PlayerProfile,
    group: WeaponGroup,
    weapon_catalog: &WeaponCatalog,
) -> bool {
    let group_key = normalize_token(weapon_group_label(group));
    let mut weapon_keys: HashSet<String> = HashSet::new();
    for weapon in weapon_catalog
        .entries()
        .iter()
        .filter(|weapon| weapon.group == group)
    {
        weapon_keys.insert(normalize_token(&weapon.name));
    }
    profile.proficiencies.iter().any(|entry| {
        let token = normalize_proficiency(entry);
        token == group_key || weapon_keys.contains(&token)
    })
}

fn grant_group_proficiency_or_refund(
    profile: &mut PlayerProfile,
    group: WeaponGroup,
    weapon_catalog: &WeaponCatalog,
) -> GroupReward {
    let mut weapon_names: Vec<String> = weapon_catalog
        .entries()
        .iter()
        .filter(|weapon| weapon.group == group)
        .map(|weapon| weapon.name.clone())
        .collect();
    weapon_names.sort();
    for weapon_name in weapon_names {
        if !is_proficient_with_weapon_name(profile, &weapon_name) {
            profile.proficiencies.push(weapon_name.clone());
            return GroupReward::Proficiency(weapon_name);
        }
    }
    profile.points.bp = profile.points.bp.saturating_add(PROFICIENCY_REFUND_BP);
    GroupReward::RefundBp(PROFICIENCY_REFUND_BP)
}

fn is_proficient_with_weapon_name(profile: &PlayerProfile, weapon_name: &str) -> bool {
    let target = normalize_token(weapon_name);
    profile
        .proficiencies
        .iter()
        .any(|entry| normalize_proficiency(entry) == target)
}

fn normalize_proficiency(value: &str) -> String {
    let lowered = value.to_ascii_lowercase().replace("proficiency", " ");
    normalize_token(&lowered)
}

fn normalize_token(value: &str) -> String {
    let mut out = String::new();
    let mut last_space = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_space = false;
        } else if !last_space {
            out.push(' ');
            last_space = true;
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

enum GroupReward {
    Proficiency(String),
    RefundBp(i32),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::sim::{
        AttackEvent, AttackRollBreakdown, CombatEvent, DamageBreakdown, ShieldDamageBreakdown,
    };
    use crate::game_logic::{PlayerConfig, ShieldId, WeaponId};

    fn sample_catalogs() -> (WeaponCatalog, ShieldCatalog) {
        let (weapons, _armor, shields) = crate::data::load_catalogs().expect("catalogs");
        (weapons, shields)
    }

    fn attack_event(slot: WeaponSlot, is_ranged: bool) -> CombatEvent {
        CombatEvent {
            time: 0,
            attacker_idx: 0,
            defender_idx: 1,
            kind: CombatEventKind::Attack(AttackEvent {
                hit: true,
                shield_block: false,
                damage: 1,
                shield_damage: 0,
                knockback_ft: 0.0,
                hold_at_bay: false,
                is_charge: false,
                weapon_slot: slot,
                use_jab: false,
                is_ranged,
                trauma_applied: false,
                trauma_seconds: None,
                roll: AttackRollBreakdown {
                    attack_die: 1,
                    defense_die: 1,
                    attack_bonus: 0,
                    range_mod: 0,
                    defense_base: 0,
                    weapon_defense_bonus: 0,
                    shield_defense_bonus: 0,
                    attack_total: 1,
                    defense_total: 1,
                },
                damage_breakdown: Some(DamageBreakdown {
                    rolled_damage: 1,
                    strength_damage: 0,
                    raw_damage: 1,
                    armor_dr: 0,
                    armor_penetration: 0,
                    effective_armor_dr: 0,
                    final_damage: 1,
                }),
                shield_damage_breakdown: Some(ShieldDamageBreakdown {
                    rolled_damage: 0,
                    strength_damage: 0,
                    raw_damage: 0,
                    shield_dr: 0,
                    armor_dr: 0,
                    armor_penetration: 0,
                    effective_armor_dr: 0,
                    hp_damage: 0,
                    shield_broken: false,
                }),
                defender_hp_after: 0,
                critical: None,
            }),
        }
    }

    fn player_with_weapon(weapon_id: WeaponId) -> PlayerConfig {
        let mut config = PlayerConfig::new("Test", weapon_id);
        config.level = 1;
        config
    }

    #[test]
    fn spend_requires_matching_proficiency() {
        let (weapons, _shields) = sample_catalogs();
        let mut profile = PlayerProfile::default();
        let group = WeaponGroup::Polearms;
        let idx = ensure_progress_index(&mut profile, group);
        profile.weapon_masteries[idx].unspent_points = 1;
        let err = spend_mastery_point(&mut profile, group, MasteryAspect::Attack, &weapons)
            .expect_err("should require proficiency");
        assert!(err.contains("without a matching proficiency"));
    }

    #[test]
    fn damage_tier_cannot_advance_before_other_aspects() {
        let (weapons, _shields) = sample_catalogs();
        let mut profile = PlayerProfile::default();
        profile.proficiencies.push("Glaive".to_string());
        let group = WeaponGroup::Polearms;
        let idx = ensure_progress_index(&mut profile, group);
        profile.weapon_masteries[idx].unspent_points = 1;
        profile.weapon_masteries[idx].damage = 1;
        let err = spend_mastery_point(&mut profile, group, MasteryAspect::Damage, &weapons)
            .expect_err("should enforce tier gate");
        assert!(err.contains("Finish tier +1"));
    }

    #[test]
    fn encounter_wexp_grants_points_with_carry() {
        let (weapons, shields) = sample_catalogs();
        let primary = weapons
            .entries()
            .iter()
            .enumerate()
            .find(|(_, w)| w.group == WeaponGroup::Polearms)
            .and_then(|(idx, _)| weapons.id_from_index(idx))
            .expect("polearm");
        let mut profile = PlayerProfile::default();
        profile.base_stats.intelligence = 10;
        let mut config = player_with_weapon(primary);
        config.shield_id = ShieldId::new(0);
        let mut rng = SimRng::from_seed(42);
        let lines = apply_weapon_experience_from_fight(
            &mut profile,
            &config,
            &weapons,
            &shields,
            &[attack_event(WeaponSlot::Primary, false)],
            1,
            &mut rng,
        );
        assert!(!lines.is_empty());
        let entry = progress_for_group(&profile, WeaponGroup::Polearms).expect("mastery entry");
        assert!(entry.unspent_points >= 1 || entry.experience > 0);
    }

    #[test]
    fn apply_profile_masteries_sets_equipped_values() {
        let (weapons, shields) = sample_catalogs();
        let primary = weapons
            .entries()
            .iter()
            .enumerate()
            .find(|(_, w)| w.group == WeaponGroup::Polearms)
            .and_then(|(idx, _)| weapons.id_from_index(idx))
            .expect("polearm");
        let mut profile = PlayerProfile::default();
        let idx = ensure_progress_index(&mut profile, WeaponGroup::Polearms);
        profile.weapon_masteries[idx].attack = 2;
        profile.weapon_masteries[idx].defense = 3;
        profile.weapon_masteries[idx].damage = 4;
        profile.weapon_masteries[idx].speed = 1;

        let mut cfg = PlayerConfig::new("Test", primary);
        cfg.shield_id = ShieldId::new(0);
        apply_profile_masteries_to_config(&profile, &mut cfg, &weapons, &shields);
        assert_eq!(cfg.mastery_attack, 2);
        assert_eq!(cfg.mastery_defense, 3);
        assert_eq!(cfg.mastery_damage, 4);
        assert_eq!(cfg.mastery_speed, 1);
    }
}
