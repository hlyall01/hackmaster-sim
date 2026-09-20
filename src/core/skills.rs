use crate::character::{AbilityScore, AbilitySetFull};
use crate::core::rng::SimRng;
use crate::core::rules::roll_damage_expr;
use crate::core::types::{PlayerProfile, SkillProgress};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillAbility {
    Strength,
    Intelligence,
    Wisdom,
    Dexterity,
    Constitution,
    Looks,
    Charisma,
}

impl SkillAbility {
    pub fn short_label(self) -> &'static str {
        match self {
            SkillAbility::Strength => "STR",
            SkillAbility::Intelligence => "INT",
            SkillAbility::Wisdom => "WIS",
            SkillAbility::Dexterity => "DEX",
            SkillAbility::Constitution => "CON",
            SkillAbility::Looks => "LKS",
            SkillAbility::Charisma => "CHA",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkillPrerequisite {
    pub skill_id: &'static str,
    pub min_level: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkillSpec {
    pub id: &'static str,
    pub name: &'static str,
    pub relevant: &'static [SkillAbility],
    pub lp_cost: i32,
    pub universal: bool,
    pub prerequisites: &'static [SkillPrerequisite],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillMasteryTier {
    Unskilled,
    Novice,
    Average,
    Advanced,
    Expert,
    Master,
}

impl SkillMasteryTier {
    pub fn label(self) -> &'static str {
        match self {
            SkillMasteryTier::Unskilled => "Unskilled",
            SkillMasteryTier::Novice => "Novice",
            SkillMasteryTier::Average => "Average",
            SkillMasteryTier::Advanced => "Advanced",
            SkillMasteryTier::Expert => "Expert",
            SkillMasteryTier::Master => "Master",
        }
    }

    pub fn mastery_die(self) -> &'static str {
        match self {
            SkillMasteryTier::Unskilled | SkillMasteryTier::Novice => "d12p",
            SkillMasteryTier::Average => "d8p",
            SkillMasteryTier::Advanced => "d6p",
            SkillMasteryTier::Expert => "d4p",
            SkillMasteryTier::Master => "d3p",
        }
    }

    pub fn required_character_level(self) -> u8 {
        match self {
            SkillMasteryTier::Unskilled | SkillMasteryTier::Novice => 1,
            SkillMasteryTier::Average => 3,
            SkillMasteryTier::Advanced => 5,
            SkillMasteryTier::Expert => 7,
            SkillMasteryTier::Master => 9,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillDifficulty {
    Easy,
    Medium,
    Hard,
    VeryHard,
}

impl SkillDifficulty {
    pub fn label(self) -> &'static str {
        match self {
            SkillDifficulty::Easy => "Easy",
            SkillDifficulty::Medium => "Medium",
            SkillDifficulty::Hard => "Hard",
            SkillDifficulty::VeryHard => "Very Hard",
        }
    }

    pub fn target_shift(self) -> i32 {
        match self {
            SkillDifficulty::Easy => 30,
            SkillDifficulty::Medium => 15,
            SkillDifficulty::Hard => 0,
            SkillDifficulty::VeryHard => -15,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillCheckResult {
    pub skill_id: String,
    pub skill_name: String,
    pub difficulty: SkillDifficulty,
    pub success: bool,
    pub trained: bool,
    pub level: u8,
    pub mastery: SkillMasteryTier,
    pub mastery_die: &'static str,
    pub mastery_roll: i32,
    pub relevant_ability: u8,
    pub ability_modifier: i32,
    pub target: i32,
    pub roll: i32,
    pub reason: Option<String>,
}

impl SkillCheckResult {
    pub fn summary_line(&self) -> String {
        if let Some(reason) = &self.reason {
            return reason.clone();
        }
        format!(
            "{} {} check: d100 {} vs target {} (skill {} + {} {} + ability {} + diff {:+}) => {}",
            self.skill_name,
            self.difficulty.label(),
            self.roll,
            self.target,
            self.level,
            self.mastery_die,
            self.mastery_roll,
            self.ability_modifier,
            self.difficulty.target_shift(),
            if self.success { "success" } else { "failure" }
        )
    }
}

macro_rules! req {
    ($skill:literal, $min:expr) => {
        SkillPrerequisite {
            skill_id: $skill,
            min_level: $min,
        }
    };
}

macro_rules! skill {
    (
        $id:literal,
        $name:literal,
        [$($ability:ident),*],
        $lp:expr,
        $universal:expr,
        [$($prereq:expr),*]
    ) => {
        SkillSpec {
            id: $id,
            name: $name,
            relevant: &[$(SkillAbility::$ability),*],
            lp_cost: $lp,
            universal: $universal,
            prerequisites: &[$($prereq),*],
        }
    };
}

const SKILLS: &[SkillSpec] = &[
    skill!("acting", "Acting", [Looks, Charisma], 2, true, []),
    skill!(
        "administration",
        "Administration",
        [Intelligence, Wisdom, Charisma],
        2,
        false,
        []
    ),
    skill!("agriculture", "Agriculture", [Wisdom], 1, false, []),
    skill!(
        "animal_empathy",
        "Animal Empathy",
        [Wisdom, Charisma],
        1,
        false,
        []
    ),
    skill!("animal_herding", "Animal Herding", [Wisdom], 1, false, []),
    skill!(
        "animal_husbandry",
        "Animal Husbandry",
        [Wisdom],
        1,
        true,
        []
    ),
    skill!("animal_mimicry", "Animal Mimicry", [Wisdom], 1, true, []),
    skill!(
        "animal_training",
        "Animal Training",
        [Intelligence, Wisdom],
        2,
        false,
        [req!("animal_empathy", 30)]
    ),
    skill!("appraisal", "Appraisal", [Intelligence], 1, false, []),
    skill!(
        "armorsmithing",
        "Armorsmithing",
        [Strength, Intelligence],
        2,
        false,
        [req!("blacksmithing", 25)]
    ),
    skill!(
        "artistry",
        "Artistry",
        [Wisdom],
        1,
        false,
        [req!("literacy", 35)]
    ),
    skill!("astrology", "Astrology", [Intelligence], 2, false, []),
    skill!(
        "blacksmithing",
        "Blacksmithing",
        [Strength, Intelligence],
        1,
        false,
        []
    ),
    skill!("boating", "Boating", [Wisdom], 1, true, []),
    skill!("botany", "Botany", [Intelligence], 1, false, []),
    skill!(
        "butchery",
        "Butchery",
        [Strength, Intelligence],
        1,
        true,
        []
    ),
    skill!("carpentry", "Carpentry", [Intelligence], 1, false, []),
    skill!("cartography", "Cartography", [Intelligence], 2, true, []),
    skill!("climbing", "Climbing", [Strength, Dexterity], 2, true, []),
    skill!("cooking", "Cooking", [Intelligence, Wisdom], 1, false, []),
    skill!("craft", "Craft", [Wisdom, Dexterity], 1, false, []),
    skill!("current_affairs", "Current Affairs", [Wisdom], 1, true, []),
    skill!("direction_sense", "Direction Sense", [Wisdom], 1, false, []),
    skill!(
        "disarm_trap",
        "Disarm Trap",
        [Intelligence, Dexterity],
        3,
        false,
        []
    ),
    skill!(
        "disguise",
        "Disguise",
        [Intelligence, Charisma],
        2,
        true,
        []
    ),
    skill!("distraction", "Distraction", [Charisma], 1, true, []),
    skill!(
        "engineering",
        "Engineering",
        [Intelligence],
        3,
        false,
        [req!("literacy", 26), req!("mathematics", 1)]
    ),
    skill!(
        "escape_artist",
        "Escape Artist",
        [Intelligence, Dexterity],
        2,
        true,
        []
    ),
    skill!("fast_talking", "Fast Talking", [Charisma], 1, false, []),
    skill!("fire_building", "Fire Building", [Wisdom], 1, true, []),
    skill!("first_aid", "First Aid", [Wisdom], 2, false, []),
    skill!("forestry", "Forestry", [Intelligence], 1, false, []),
    skill!(
        "forgery",
        "Forgery",
        [Intelligence, Dexterity],
        3,
        false,
        [req!("literacy", 35)]
    ),
    skill!("gambling", "Gambling", [Wisdom, Charisma], 1, false, []),
    skill!("geology", "Geology", [Intelligence], 1, false, []),
    skill!(
        "glean_information",
        "Glean Information",
        [Intelligence, Wisdom, Charisma],
        1,
        true,
        []
    ),
    skill!("hiding", "Hiding", [Intelligence, Dexterity], 2, true, []),
    skill!("history", "History", [Intelligence], 1, false, []),
    skill!("hunting", "Hunting", [Wisdom], 2, false, []),
    skill!("identify_trap", "Identify Trap", [Wisdom], 3, false, []),
    skill!(
        "intimidation",
        "Intimidation",
        [Strength, Charisma],
        2,
        true,
        []
    ),
    skill!("jumping", "Jumping", [Strength], 1, true, []),
    skill!("language", "Language", [Intelligence], 1, false, []),
    skill!("law", "Law", [Intelligence], 2, true, []),
    skill!("leadership", "Leadership", [Charisma], 3, false, []),
    skill!(
        "leatherworking",
        "Leatherworking",
        [Intelligence, Dexterity],
        1,
        false,
        []
    ),
    skill!("listening", "Listening", [Wisdom], 2, true, []),
    skill!("literacy", "Literacy", [Intelligence], 2, false, []),
    skill!(
        "lock_picking",
        "Lock Picking",
        [Intelligence, Dexterity],
        3,
        false,
        []
    ),
    skill!(
        "mathematics",
        "Mathematics",
        [Intelligence],
        2,
        false,
        [req!("literacy", 30)]
    ),
    skill!("mining", "Mining", [Strength, Intelligence], 1, false, []),
    skill!("monster_lore", "Monster Lore", [Intelligence], 2, false, []),
    skill!("musician", "Musician", [Wisdom], 1, false, []),
    skill!("observation", "Observation", [Wisdom], 2, true, []),
    skill!("oration", "Oration", [Charisma], 1, true, []),
    skill!("persuasion", "Persuasion", [Charisma], 2, true, []),
    skill!("pick_pocket", "Pick Pocket", [Dexterity], 3, true, []),
    skill!("pottery", "Pottery", [Wisdom, Dexterity], 1, false, []),
    skill!("reading_lips", "Reading Lips", [Intelligence], 1, true, []),
    skill!("recruiting", "Recruiting", [Charisma], 1, true, []),
    skill!("religion", "Religion", [Wisdom], 1, false, []),
    skill!("riding", "Riding", [Wisdom, Dexterity], 2, false, []),
    skill!("rope_use", "Rope Use", [Dexterity], 1, true, []),
    skill!(
        "salesmanship",
        "Salesmanship",
        [Intelligence, Wisdom, Charisma],
        3,
        true,
        []
    ),
    skill!("scrutiny", "Scrutiny", [Wisdom], 2, true, []),
    skill!("seduction", "Seduction", [Looks, Charisma], 2, true, []),
    skill!("skilled_liar", "Skilled Liar", [Charisma], 2, true, []),
    skill!("sneaking", "Sneaking", [Dexterity], 3, true, []),
    skill!("survival", "Survival", [Wisdom, Constitution], 2, true, []),
    skill!(
        "survival_urban",
        "Survival, Urban",
        [Wisdom, Charisma],
        1,
        false,
        []
    ),
    skill!(
        "swimming",
        "Swimming",
        [Strength, Constitution],
        1,
        false,
        []
    ),
    skill!("torture", "Torture", [Intelligence], 2, true, []),
    skill!("tracking", "Tracking", [Wisdom], 3, true, []),
    skill!(
        "trap_design",
        "Trap Design",
        [Intelligence, Dexterity],
        3,
        false,
        []
    ),
    skill!("weather_sense", "Weather Sense", [Wisdom], 1, false, []),
    skill!(
        "weaponsmithing",
        "Weaponsmithing",
        [Strength, Intelligence],
        2,
        false,
        [req!("blacksmithing", 25)]
    ),
];

pub fn all_skill_specs() -> &'static [SkillSpec] {
    SKILLS
}

pub fn mastery_tier_for_level(level: u8) -> SkillMasteryTier {
    match level {
        0 => SkillMasteryTier::Unskilled,
        1..=25 => SkillMasteryTier::Novice,
        26..=50 => SkillMasteryTier::Average,
        51..=75 => SkillMasteryTier::Advanced,
        76..=87 => SkillMasteryTier::Expert,
        _ => SkillMasteryTier::Master,
    }
}

pub fn event_skill_bonus_for_level(level: u8) -> i32 {
    match mastery_tier_for_level(level) {
        SkillMasteryTier::Unskilled => 0,
        SkillMasteryTier::Novice => 1,
        SkillMasteryTier::Average => 2,
        SkillMasteryTier::Advanced => 3,
        SkillMasteryTier::Expert => 4,
        SkillMasteryTier::Master => 5,
    }
}

pub fn ability_mastery_modifier(score: u8) -> i32 {
    match score {
        0 | 1 => -5,
        2..=3 => -4,
        4..=5 => -3,
        6..=7 => -2,
        8..=9 => -1,
        10..=11 => 0,
        12..=13 => 1,
        14..=15 => 2,
        16..=17 => 3,
        18..=19 => 4,
        20..=22 => 5,
        _ => 6,
    }
}

pub fn max_skill_for_character_level(level: u8) -> u8 {
    if level >= 9 {
        100
    } else if level >= 7 {
        87
    } else if level >= 5 {
        75
    } else if level >= 3 {
        50
    } else {
        25
    }
}

pub fn normalize_skill_ref(input: &str) -> String {
    let mut out = String::new();
    let mut last_sep = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_sep = false;
        } else if !last_sep {
            out.push('_');
            last_sep = true;
        }
    }
    out.trim_matches('_').to_string()
}

pub fn skill_spec(skill_ref: &str) -> Option<&'static SkillSpec> {
    let key = normalize_skill_ref(skill_ref);
    if key.is_empty() {
        return None;
    }
    SKILLS
        .iter()
        .find(|spec| spec.id == key || normalize_skill_ref(spec.name) == key)
}

pub fn relevant_ability_lowest(abilities: &AbilitySetFull, spec: &SkillSpec) -> u8 {
    let mut lowest = u8::MAX;
    for ability in spec.relevant {
        let value = ability_score_for(abilities, *ability).base;
        if value < lowest {
            lowest = value;
        }
    }
    if lowest == u8::MAX { 1 } else { lowest.max(1) }
}

fn ability_score_for(abilities: &AbilitySetFull, ability: SkillAbility) -> AbilityScore {
    match ability {
        SkillAbility::Strength => abilities.strength,
        SkillAbility::Intelligence => abilities.intelligence,
        SkillAbility::Wisdom => abilities.wisdom,
        SkillAbility::Dexterity => abilities.dexterity,
        SkillAbility::Constitution => abilities.constitution,
        SkillAbility::Looks => abilities.looks,
        SkillAbility::Charisma => abilities.charisma,
    }
}

pub fn starting_skill_level(abilities: &AbilitySetFull, spec: &SkillSpec) -> u8 {
    relevant_ability_lowest(abilities, spec).clamp(1, 25)
}

pub fn skill_level_in(skills: &[SkillProgress], skill_ref: &str) -> u8 {
    let Some(spec) = skill_spec(skill_ref) else {
        return 0;
    };
    skills
        .iter()
        .find(|entry| normalize_skill_ref(&entry.id) == spec.id)
        .map(|entry| entry.level.min(100))
        .unwrap_or(0)
}

fn set_skill_level(skills: &mut Vec<SkillProgress>, skill_id: &str, level: u8) {
    if let Some(existing) = skills
        .iter_mut()
        .find(|entry| normalize_skill_ref(&entry.id) == skill_id)
    {
        existing.level = level.min(100);
        existing.id = skill_id.to_string();
        return;
    }
    skills.push(SkillProgress {
        id: skill_id.to_string(),
        level: level.min(100),
    });
}

pub fn remove_skill(skills: &mut Vec<SkillProgress>, skill_ref: &str) -> bool {
    let Some(spec) = skill_spec(skill_ref) else {
        return false;
    };
    if let Some(index) = skills
        .iter()
        .position(|entry| normalize_skill_ref(&entry.id) == spec.id)
    {
        skills.remove(index);
        return true;
    }
    false
}

fn prerequisite_failures(skill: &SkillSpec, known_skills: &[SkillProgress]) -> Vec<String> {
    let mut failures = Vec::new();
    for prereq in skill.prerequisites {
        let level = skill_level_in(known_skills, prereq.skill_id);
        if level < prereq.min_level {
            let prereq_label = skill_spec(prereq.skill_id)
                .map(|spec| spec.name)
                .unwrap_or(prereq.skill_id);
            failures.push(format!(
                "Requires {prereq_label} {}+, currently {level}",
                prereq.min_level
            ));
        }
    }
    failures
}

pub fn advancement_cap(skill_ref: &str, character_level: u8, known_skills: &[SkillProgress]) -> u8 {
    let Some(skill) = skill_spec(skill_ref) else {
        return 0;
    };
    let mut cap = max_skill_for_character_level(character_level);

    if !prerequisite_failures(skill, known_skills).is_empty() {
        return 0;
    }

    let literacy = skill_level_in(known_skills, "literacy");
    let mathematics = skill_level_in(known_skills, "mathematics");
    let cartography = skill_level_in(known_skills, "cartography");
    let disarm_trap = skill_level_in(known_skills, "disarm_trap");

    match skill.id {
        "engineering" => {
            if literacy < 26 {
                cap = cap.min(25);
            }
            if mathematics == 0 {
                return 0;
            }
            cap = cap.min(mathematics);
            if cap > 50 && cartography < 26 {
                cap = 50;
            }
        }
        "history" => {
            if cap > 50 && literacy < 20 {
                cap = 50;
            }
        }
        "law" => {
            if cap > 25 && literacy < 26 {
                cap = 25;
            }
            if cap > 50 && literacy < 51 {
                cap = 50;
            }
        }
        "trap_design" => {
            cap = cap.min(disarm_trap);
        }
        _ => {}
    }
    cap
}

pub fn can_learn_skill(
    known_skills: &[SkillProgress],
    character_level: u8,
    skill_ref: &str,
) -> Result<&'static SkillSpec, String> {
    let Some(skill) = skill_spec(skill_ref) else {
        return Err(format!("Unknown skill: {skill_ref}"));
    };
    if skill_level_in(known_skills, skill.id) > 0 {
        return Err(format!("{} is already learned", skill.name));
    }
    let failures = prerequisite_failures(skill, known_skills);
    if !failures.is_empty() {
        return Err(failures.join("; "));
    }
    if advancement_cap(skill.id, character_level, known_skills) == 0 {
        return Err(format!("{} cannot currently be learned", skill.name));
    }
    Ok(skill)
}

pub fn learn_skill(
    known_skills: &mut Vec<SkillProgress>,
    abilities: &AbilitySetFull,
    character_level: u8,
    skill_ref: &str,
) -> Result<SkillProgress, String> {
    let skill = can_learn_skill(known_skills, character_level, skill_ref)?;
    let cap = advancement_cap(skill.id, character_level, known_skills);
    let start_level = starting_skill_level(abilities, skill).min(cap);
    if start_level == 0 {
        return Err(format!(
            "{} cannot be learned at the current cap",
            skill.name
        ));
    }
    set_skill_level(known_skills, skill.id, start_level);
    Ok(SkillProgress {
        id: skill.id.to_string(),
        level: start_level,
    })
}

pub fn ensure_skill(
    known_skills: &mut Vec<SkillProgress>,
    abilities: &AbilitySetFull,
    character_level: u8,
    skill_ref: &str,
) -> Result<SkillProgress, String> {
    if let Some(spec) = skill_spec(skill_ref) {
        let existing = skill_level_in(known_skills, spec.id);
        if existing > 0 {
            return Ok(SkillProgress {
                id: spec.id.to_string(),
                level: existing,
            });
        }
    }
    learn_skill(known_skills, abilities, character_level, skill_ref)
}

pub fn total_lp_cost(skills: &[SkillProgress]) -> i32 {
    let mut total = 0;
    for entry in skills {
        if entry.level == 0 {
            continue;
        }
        if let Some(spec) = skill_spec(&entry.id) {
            total += spec.lp_cost;
        }
    }
    total
}

pub fn legacy_skill_names(skills: &[SkillProgress]) -> Vec<String> {
    let mut names: Vec<String> = skills
        .iter()
        .filter(|entry| entry.level > 0)
        .map(|entry| {
            skill_spec(&entry.id)
                .map(|spec| spec.name.to_string())
                .unwrap_or_else(|| entry.id.clone())
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

pub fn derive_skill_levels_from_legacy(
    names: &[String],
    abilities: &AbilitySetFull,
) -> Vec<SkillProgress> {
    let mut out = Vec::new();
    for name in names {
        let Some(spec) = skill_spec(name) else {
            continue;
        };
        if out
            .iter()
            .any(|entry: &SkillProgress| normalize_skill_ref(&entry.id) == spec.id)
        {
            continue;
        }
        out.push(SkillProgress {
            id: spec.id.to_string(),
            level: starting_skill_level(abilities, spec),
        });
    }
    out
}

pub fn player_skill_level(player: &PlayerProfile, skill_ref: &str) -> u8 {
    let Some(spec) = skill_spec(skill_ref) else {
        return 0;
    };
    let level = skill_level_in(&player.skill_levels, spec.id);
    if level > 0 {
        return level;
    }
    if player
        .skills
        .iter()
        .any(|name| normalize_skill_ref(name) == spec.id)
    {
        return starting_skill_level(&player.ability_scores_full, spec);
    }
    0
}

pub fn event_skill_bonus_for_player(player: &PlayerProfile, skill_ref: &str) -> i32 {
    event_skill_bonus_for_level(player_skill_level(player, skill_ref))
}

pub fn roll_skill_check(
    player: &PlayerProfile,
    skill_ref: &str,
    difficulty: SkillDifficulty,
    require_trained: bool,
    rng: &mut SimRng,
) -> SkillCheckResult {
    let Some(spec) = skill_spec(skill_ref) else {
        return SkillCheckResult {
            skill_id: normalize_skill_ref(skill_ref),
            skill_name: skill_ref.to_string(),
            difficulty,
            success: false,
            trained: false,
            level: 0,
            mastery: SkillMasteryTier::Unskilled,
            mastery_die: SkillMasteryTier::Unskilled.mastery_die(),
            mastery_roll: 0,
            relevant_ability: 0,
            ability_modifier: 0,
            target: 0,
            roll: 0,
            reason: Some(format!("Unknown skill: {skill_ref}")),
        };
    };

    let level = player_skill_level(player, spec.id);
    if require_trained && level == 0 {
        return SkillCheckResult {
            skill_id: spec.id.to_string(),
            skill_name: spec.name.to_string(),
            difficulty,
            success: false,
            trained: false,
            level,
            mastery: SkillMasteryTier::Unskilled,
            mastery_die: SkillMasteryTier::Unskilled.mastery_die(),
            mastery_roll: 0,
            relevant_ability: relevant_ability_lowest(&player.ability_scores_full, spec),
            ability_modifier: 0,
            target: 0,
            roll: 0,
            reason: Some(format!("{} requires trained mastery", spec.name)),
        };
    }

    let mastery = mastery_tier_for_level(level);
    let mastery_die = mastery.mastery_die();
    let mastery_roll = roll_damage_expr(mastery_die, rng, false);
    let relevant_ability = relevant_ability_lowest(&player.ability_scores_full, spec);
    let ability_modifier = ability_mastery_modifier(relevant_ability);
    let mut target = i32::from(level)
        .saturating_add(mastery_roll)
        .saturating_add(ability_modifier)
        .saturating_add(difficulty.target_shift());
    target = target.clamp(1, 100);
    let roll = roll_damage_expr("d100", rng, false).clamp(1, 100);
    SkillCheckResult {
        skill_id: spec.id.to_string(),
        skill_name: spec.name.to_string(),
        difficulty,
        success: roll <= target,
        trained: level > 0,
        level,
        mastery,
        mastery_die,
        mastery_roll,
        relevant_ability,
        ability_modifier,
        target,
        roll,
        reason: None,
    }
}

pub fn describe_relevant_abilities(spec: &SkillSpec) -> String {
    spec.relevant
        .iter()
        .map(|ability| ability.short_label())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::character::AbilityScore;

    fn test_abilities() -> AbilitySetFull {
        AbilitySetFull {
            strength: AbilityScore::new(14, 1),
            intelligence: AbilityScore::new(12, 1),
            wisdom: AbilityScore::new(10, 1),
            dexterity: AbilityScore::new(16, 1),
            constitution: AbilityScore::new(13, 1),
            looks: AbilityScore::new(9, 1),
            charisma: AbilityScore::new(11, 1),
        }
    }

    #[test]
    fn level_to_mastery_tier_matches_table() {
        assert_eq!(mastery_tier_for_level(0), SkillMasteryTier::Unskilled);
        assert_eq!(mastery_tier_for_level(1), SkillMasteryTier::Novice);
        assert_eq!(mastery_tier_for_level(25), SkillMasteryTier::Novice);
        assert_eq!(mastery_tier_for_level(26), SkillMasteryTier::Average);
        assert_eq!(mastery_tier_for_level(51), SkillMasteryTier::Advanced);
        assert_eq!(mastery_tier_for_level(76), SkillMasteryTier::Expert);
        assert_eq!(mastery_tier_for_level(88), SkillMasteryTier::Master);
    }

    #[test]
    fn starting_level_uses_lowest_relevant_ability() {
        let spec = skill_spec("climbing").expect("climbing spec");
        let level = starting_skill_level(&test_abilities(), spec);
        assert_eq!(level, 14);
    }

    #[test]
    fn learning_enforces_prerequisites() {
        let mut skills = Vec::<SkillProgress>::new();
        let err = learn_skill(&mut skills, &test_abilities(), 1, "animal_training")
            .expect_err("missing prereq should fail");
        assert!(err.contains("Animal Empathy"));
    }

    #[test]
    fn legacy_names_are_resolved_to_specs() {
        let names = vec!["Climbing".to_string(), "First Aid".to_string()];
        let levels = derive_skill_levels_from_legacy(&names, &test_abilities());
        assert_eq!(levels.len(), 2);
        assert_eq!(skill_level_in(&levels, "climbing"), 14);
        assert_eq!(skill_level_in(&levels, "first_aid"), 10);
    }
}
