use crate::core::gameplay::{EncounterTier, RunState, Wound};
use crate::core::rng::SimRng;
use crate::core::rules::roll_damage_expr;
use crate::core::skills::{self, SkillDifficulty};
use crate::core::types::TalentSelection;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventTierGate {
    Any,
    Normal,
    Elite,
    Boss,
}

impl EventTierGate {
    fn matches(self, tier: EncounterTier) -> bool {
        matches!(self, EventTierGate::Any)
            || matches!(
                (self, tier),
                (EventTierGate::Normal, EncounterTier::Normal)
                    | (EventTierGate::Elite, EncounterTier::Elite)
                    | (EventTierGate::Boss, EncounterTier::Boss)
            )
    }
}

fn default_event_tiers() -> Vec<EventTierGate> {
    vec![EventTierGate::Any]
}

fn default_event_weight() -> u32 {
    10
}

fn default_event_max_depth() -> u32 {
    u32::MAX
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventStat {
    Strength,
    Intelligence,
    Wisdom,
    Dexterity,
    Constitution,
    Looks,
    Charisma,
}

impl EventStat {
    pub fn label(self) -> &'static str {
        match self {
            EventStat::Strength => "STR",
            EventStat::Intelligence => "INT",
            EventStat::Wisdom => "WIS",
            EventStat::Dexterity => "DEX",
            EventStat::Constitution => "CON",
            EventStat::Looks => "LKS",
            EventStat::Charisma => "CHA",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventCheckDifficulty {
    Easy,
    Medium,
    Hard,
    VeryHard,
}

impl EventCheckDifficulty {
    fn as_skill(self) -> SkillDifficulty {
        match self {
            EventCheckDifficulty::Easy => SkillDifficulty::Easy,
            EventCheckDifficulty::Medium => SkillDifficulty::Medium,
            EventCheckDifficulty::Hard => SkillDifficulty::Hard,
            EventCheckDifficulty::VeryHard => SkillDifficulty::VeryHard,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventCheck {
    pub stat: EventStat,
    pub dc: u8,
    #[serde(default)]
    pub difficulty: Option<EventCheckDifficulty>,
    #[serde(default)]
    pub require_trained: bool,
    #[serde(default)]
    pub skill: Option<String>,
}

impl EventCheck {
    fn skill_difficulty(&self) -> SkillDifficulty {
        self.difficulty
            .map(EventCheckDifficulty::as_skill)
            .unwrap_or_else(|| legacy_dc_to_difficulty(self.dc))
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EventResult {
    #[serde(default)]
    pub gold_delta: i32,
    #[serde(default)]
    pub xp_delta: u32,
    #[serde(default)]
    pub bp_delta: i32,
    #[serde(default)]
    pub lp_delta: i32,
    #[serde(default)]
    pub ap_delta: i32,
    #[serde(default)]
    pub rp_delta: i32,
    #[serde(default)]
    pub honor_delta: i32,
    #[serde(default)]
    pub add_wound: u32,
    #[serde(default)]
    pub heal_wound: u32,
    #[serde(default)]
    pub add_item: Option<String>,
    #[serde(default)]
    pub add_talents: Vec<TalentSelection>,
    #[serde(default)]
    pub set_flags: Vec<String>,
    #[serde(default)]
    pub clear_flags: Vec<String>,
    #[serde(default)]
    pub trigger_fight: bool,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventChoice {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub check: Option<EventCheck>,
    #[serde(default)]
    pub success: EventResult,
    #[serde(default)]
    pub failure: EventResult,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventSpec {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default = "default_event_weight")]
    pub weight: u32,
    #[serde(default)]
    pub min_depth: u32,
    #[serde(default = "default_event_max_depth")]
    pub max_depth: u32,
    #[serde(default = "default_event_tiers")]
    pub tiers: Vec<EventTierGate>,
    #[serde(default)]
    pub requires_flags: Vec<String>,
    #[serde(default)]
    pub unique_once: bool,
    #[serde(default)]
    pub choices: Vec<EventChoice>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventCatalog {
    pub version: u32,
    pub events: Vec<EventSpec>,
}

impl Default for EventCatalog {
    fn default() -> Self {
        Self {
            version: 1,
            events: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventOutcome {
    GoldDelta(i32),
    XpDelta(u32),
    BpDelta(i32),
    LpDelta(i32),
    ApDelta(i32),
    RpDelta(i32),
    HonorDelta(i32),
    WoundAdded(u32),
    WoundHealed(u32),
    ItemGained(String),
    TalentGained(String),
    FlagSet(String),
    FlagCleared(String),
    TriggerFight,
    NoEffect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventResolution {
    pub event_id: String,
    pub event_name: String,
    pub choice_id: String,
    pub success: bool,
    pub trigger_fight: bool,
    pub outcomes: Vec<EventOutcome>,
    pub lines: Vec<String>,
}

pub fn should_spawn_event(rng: &mut SimRng) -> bool {
    rng.gen_range(0..100) < 55
}

pub fn choose_event(
    catalog: &EventCatalog,
    state: &RunState,
    tier: EncounterTier,
    rng: &mut SimRng,
) -> Option<EventSpec> {
    let mut eligible = Vec::new();
    for event in &catalog.events {
        if event.weight == 0 {
            continue;
        }
        if state.run_depth < event.min_depth || state.run_depth > event.max_depth {
            continue;
        }
        if !event.tiers.iter().any(|gate| gate.matches(tier)) {
            continue;
        }
        if event
            .requires_flags
            .iter()
            .any(|flag| !state.event_flags.iter().any(|have| have == flag))
        {
            continue;
        }
        if event.unique_once
            && state
                .seen_event_ids
                .iter()
                .any(|seen| seen.eq_ignore_ascii_case(&event.id))
        {
            continue;
        }
        eligible.push((event, event.weight));
    }
    weighted_pick_spec(&eligible, rng).cloned()
}

pub fn resolve_event_choice(
    state: &mut RunState,
    event: &EventSpec,
    choice_id: Option<&str>,
    rng: &mut SimRng,
) -> EventResolution {
    if !state
        .seen_event_ids
        .iter()
        .any(|seen| seen.eq_ignore_ascii_case(&event.id))
    {
        state.seen_event_ids.push(event.id.clone());
    }
    let fallback_choice = EventChoice {
        id: "leave".to_string(),
        text: "Leave it".to_string(),
        check: None,
        success: EventResult::default(),
        failure: EventResult::default(),
    };
    let choice = choice_id
        .and_then(|id| event.choices.iter().find(|choice| choice.id == id))
        .or_else(|| event.choices.first())
        .unwrap_or(&fallback_choice);

    let mut success = true;
    let mut lines = vec![
        format!("Event: {}", event.name),
        format!("Choice: {}", choice.text),
    ];
    if let Some(check) = &choice.check {
        let difficulty = check.skill_difficulty();
        if let Some(skill_ref) = &check.skill {
            let check_result = skills::roll_skill_check(
                &state.player,
                skill_ref,
                difficulty,
                check.require_trained,
                rng,
            );
            success = check_result.success;
            lines.push(check_result.summary_line());
        } else {
            let stat_base = stat_base_value(state, check.stat);
            let pseudo_level = i32::from(stat_base);
            let mastery = skills::mastery_tier_for_level(stat_base);
            let mastery_die = mastery.mastery_die();
            let mastery_roll = roll_damage_expr(mastery_die, rng, false);
            let ability_mod = skills::ability_mastery_modifier(stat_base);
            let target = (pseudo_level
                .saturating_add(mastery_roll)
                .saturating_add(ability_mod)
                .saturating_add(difficulty.target_shift()))
            .clamp(1, 100);
            let roll = rng.gen_range(1..=100) as i32;
            success = roll <= target;
            lines.push(format!(
                "{} {} check: d100 {} vs target {} (stat {} + {} {} + ability {} + diff {:+}) => {}",
                check.stat.label(),
                difficulty.label(),
                roll,
                target,
                stat_base,
                mastery_die,
                mastery_roll,
                ability_mod,
                difficulty.target_shift(),
                if success { "success" } else { "failure" }
            ));
        }
    } else {
        lines.push("No check required.".to_string());
    }

    let result = if success {
        &choice.success
    } else {
        &choice.failure
    };
    let mut outcomes = apply_result(state, result);
    if outcomes.is_empty() {
        outcomes.push(EventOutcome::NoEffect);
    }
    for outcome in &outcomes {
        if let Some(line) = event_outcome_line(outcome) {
            lines.push(line);
        }
    }
    lines.extend(result.notes.iter().cloned());
    if result.trigger_fight {
        lines.push("The event escalates into a fight.".to_string());
    }

    EventResolution {
        event_id: event.id.clone(),
        event_name: event.name.clone(),
        choice_id: choice.id.clone(),
        success,
        trigger_fight: result.trigger_fight,
        outcomes,
        lines,
    }
}

fn stat_base_value(state: &RunState, stat: EventStat) -> u8 {
    match stat {
        EventStat::Strength => state.player.ability_scores_full.strength.base,
        EventStat::Intelligence => state.player.ability_scores_full.intelligence.base,
        EventStat::Wisdom => state.player.ability_scores_full.wisdom.base,
        EventStat::Dexterity => state.player.ability_scores_full.dexterity.base,
        EventStat::Constitution => state.player.ability_scores_full.constitution.base,
        EventStat::Looks => state.player.ability_scores_full.looks.base,
        EventStat::Charisma => state.player.ability_scores_full.charisma.base,
    }
}

fn legacy_dc_to_difficulty(dc: u8) -> SkillDifficulty {
    match dc {
        0..=10 => SkillDifficulty::Easy,
        11..=14 => SkillDifficulty::Medium,
        15..=18 => SkillDifficulty::Hard,
        _ => SkillDifficulty::VeryHard,
    }
}

fn event_outcome_line(outcome: &EventOutcome) -> Option<String> {
    match outcome {
        EventOutcome::GoldDelta(amount) if *amount >= 0 => Some(format!("Gold +{amount}")),
        EventOutcome::GoldDelta(amount) => Some(format!("Gold {amount}")),
        EventOutcome::XpDelta(amount) => Some(format!("XP +{amount}")),
        EventOutcome::BpDelta(amount) if *amount >= 0 => Some(format!("BP +{amount}")),
        EventOutcome::BpDelta(amount) => Some(format!("BP {amount}")),
        EventOutcome::LpDelta(amount) if *amount >= 0 => Some(format!("LP +{amount}")),
        EventOutcome::LpDelta(amount) => Some(format!("LP {amount}")),
        EventOutcome::ApDelta(amount) if *amount >= 0 => Some(format!("AP +{amount}")),
        EventOutcome::ApDelta(amount) => Some(format!("AP {amount}")),
        EventOutcome::RpDelta(amount) if *amount >= 0 => Some(format!("RP +{amount}")),
        EventOutcome::RpDelta(amount) => Some(format!("RP {amount}")),
        EventOutcome::HonorDelta(amount) if *amount >= 0 => Some(format!("Honor +{amount}")),
        EventOutcome::HonorDelta(amount) => Some(format!("Honor {amount}")),
        EventOutcome::WoundAdded(amount) => Some(format!("Wound +{amount}")),
        EventOutcome::WoundHealed(amount) => Some(format!("Wound healed {amount}")),
        EventOutcome::ItemGained(item) => Some(format!("Item gained: {item}")),
        EventOutcome::TalentGained(label) => Some(format!("Talent gained: {label}")),
        EventOutcome::FlagSet(flag) => Some(format!("Flag set: {flag}")),
        EventOutcome::FlagCleared(flag) => Some(format!("Flag cleared: {flag}")),
        EventOutcome::TriggerFight => Some("This choice triggers a fight.".to_string()),
        EventOutcome::NoEffect => Some("No direct reward.".to_string()),
    }
}

fn apply_result(state: &mut RunState, result: &EventResult) -> Vec<EventOutcome> {
    let mut outcomes = Vec::new();
    if result.gold_delta != 0 {
        if result.gold_delta > 0 {
            state.inventory.gold = state
                .inventory
                .gold
                .saturating_add(result.gold_delta as u32);
        } else {
            state.inventory.gold = state
                .inventory
                .gold
                .saturating_sub(result.gold_delta.unsigned_abs());
        }
        outcomes.push(EventOutcome::GoldDelta(result.gold_delta));
    }
    if result.xp_delta > 0 {
        state.player.xp = state.player.xp.saturating_add(result.xp_delta);
        outcomes.push(EventOutcome::XpDelta(result.xp_delta));
    }
    if result.bp_delta != 0 {
        state.player.points.bp = state.player.points.bp.saturating_add(result.bp_delta);
        outcomes.push(EventOutcome::BpDelta(result.bp_delta));
    }
    if result.lp_delta != 0 {
        state.player.points.lp = state.player.points.lp.saturating_add(result.lp_delta);
        outcomes.push(EventOutcome::LpDelta(result.lp_delta));
    }
    if result.ap_delta != 0 {
        state.player.points.ap = state.player.points.ap.saturating_add(result.ap_delta);
        outcomes.push(EventOutcome::ApDelta(result.ap_delta));
    }
    if result.rp_delta != 0 {
        state.player.points.rp = state.player.points.rp.saturating_add(result.rp_delta);
        outcomes.push(EventOutcome::RpDelta(result.rp_delta));
    }
    if result.honor_delta != 0 {
        state.player.honor = state.player.honor.saturating_add(result.honor_delta);
        outcomes.push(EventOutcome::HonorDelta(result.honor_delta));
    }
    if result.add_wound > 0 {
        state.wounds.push(Wound {
            damage: result.add_wound,
            healing_progress_steps: 0,
        });
        outcomes.push(EventOutcome::WoundAdded(result.add_wound));
    }
    if result.heal_wound > 0 {
        let mut remaining = result.heal_wound;
        for wound in state.wounds.iter_mut().rev() {
            while wound.damage > 0 && remaining > 0 {
                wound.damage -= 1;
                remaining -= 1;
            }
            if remaining == 0 {
                break;
            }
        }
        state.wounds.retain(|wound| wound.damage > 0);
        outcomes.push(EventOutcome::WoundHealed(
            result.heal_wound.saturating_sub(remaining),
        ));
    }
    if let Some(item) = &result.add_item {
        if !item.trim().is_empty() {
            state.inventory.items.push(item.clone());
            outcomes.push(EventOutcome::ItemGained(item.clone()));
        }
    }
    for talent in &result.add_talents {
        if talent.id.trim().is_empty() {
            continue;
        }
        let requested_rank = talent.rank.max(1);
        if let Some(existing) = state.player.talents.iter_mut().find(|known| {
            known.id.eq_ignore_ascii_case(&talent.id)
                && known.weapon.as_ref().map(|w| w.to_ascii_lowercase())
                    == talent.weapon.as_ref().map(|w| w.to_ascii_lowercase())
        }) {
            if existing.rank < requested_rank {
                existing.rank = requested_rank;
                outcomes.push(EventOutcome::TalentGained(format!(
                    "{} rank {}",
                    talent.id, requested_rank
                )));
            }
        } else {
            state.player.talents.push(TalentSelection {
                id: talent.id.clone(),
                rank: requested_rank,
                weapon: talent.weapon.clone(),
            });
            outcomes.push(EventOutcome::TalentGained(format!(
                "{} rank {}",
                talent.id, requested_rank
            )));
        }
    }
    for flag in &result.set_flags {
        if !state.event_flags.iter().any(|current| current == flag) {
            state.event_flags.push(flag.clone());
            outcomes.push(EventOutcome::FlagSet(flag.clone()));
        }
    }
    for flag in &result.clear_flags {
        if let Some(index) = state.event_flags.iter().position(|current| current == flag) {
            state.event_flags.remove(index);
            outcomes.push(EventOutcome::FlagCleared(flag.clone()));
        }
    }
    if result.trigger_fight {
        outcomes.push(EventOutcome::TriggerFight);
    }
    outcomes
}

fn weighted_pick_spec<'a>(
    entries: &'a [(&'a EventSpec, u32)],
    rng: &mut SimRng,
) -> Option<&'a EventSpec> {
    let total: u32 = entries.iter().map(|(_, weight)| *weight).sum();
    if total == 0 {
        return None;
    }
    let mut roll = rng.gen_range(0..total);
    for (entry, weight) in entries {
        if roll < *weight {
            return Some(*entry);
        }
        roll -= *weight;
    }
    entries.last().map(|(entry, _)| *entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{Inventory, PlayerProfile};

    fn sample_catalog() -> EventCatalog {
        EventCatalog {
            version: 1,
            events: vec![
                EventSpec {
                    id: "evt_a".to_string(),
                    name: "Test A".to_string(),
                    description: "A".to_string(),
                    weight: 10,
                    min_depth: 0,
                    max_depth: 10,
                    tiers: vec![EventTierGate::Any],
                    requires_flags: vec![],
                    unique_once: true,
                    choices: vec![EventChoice {
                        id: "press".to_string(),
                        text: "Press on".to_string(),
                        check: Some(EventCheck {
                            stat: EventStat::Wisdom,
                            dc: 10,
                            difficulty: None,
                            require_trained: false,
                            skill: None,
                        }),
                        success: EventResult {
                            gold_delta: 5,
                            ..EventResult::default()
                        },
                        failure: EventResult {
                            add_wound: 1,
                            ..EventResult::default()
                        },
                    }],
                },
                EventSpec {
                    id: "evt_b".to_string(),
                    name: "Test B".to_string(),
                    description: "B".to_string(),
                    weight: 5,
                    min_depth: 0,
                    max_depth: 99,
                    tiers: vec![EventTierGate::Elite],
                    requires_flags: vec!["flag_a".to_string()],
                    unique_once: false,
                    choices: vec![],
                },
            ],
        }
    }

    #[test]
    fn event_selection_is_deterministic() {
        let state = RunState::new(PlayerProfile::default(), Inventory::default(), 7);
        let catalog = sample_catalog();
        let mut a = SimRng::from_seed(101);
        let mut b = SimRng::from_seed(101);
        let ea = choose_event(&catalog, &state, EncounterTier::Normal, &mut a).expect("event a");
        let eb = choose_event(&catalog, &state, EncounterTier::Normal, &mut b).expect("event b");
        assert_eq!(ea.id, eb.id);
    }

    #[test]
    fn prerequisites_and_unique_are_respected() {
        let mut state = RunState::new(PlayerProfile::default(), Inventory::default(), 7);
        state.seen_event_ids.push("evt_a".to_string());
        let catalog = sample_catalog();
        let mut rng = SimRng::from_seed(5);
        let selected = choose_event(&catalog, &state, EncounterTier::Normal, &mut rng);
        assert!(selected.is_none());
    }

    #[test]
    fn resolve_event_choice_applies_mutations() {
        let mut state = RunState::new(PlayerProfile::default(), Inventory::default(), 7);
        let catalog = sample_catalog();
        let event = catalog.events.first().unwrap();
        let mut rng = SimRng::from_seed(99);
        let before_gold = state.inventory.gold;
        let _ = resolve_event_choice(&mut state, event, Some("press"), &mut rng);
        assert!(state.inventory.gold != before_gold || !state.wounds.is_empty());
        assert!(state.seen_event_ids.iter().any(|id| id == "evt_a"));
    }
}
