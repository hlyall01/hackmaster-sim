//! Core domain types (abilities, equipment, combatant sheet).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AbilityAdjustments {
    #[serde(default)]
    pub strength: i32,
    #[serde(default)]
    pub dexterity: i32,
    #[serde(default)]
    pub intelligence: i32,
    #[serde(default)]
    pub wisdom: i32,
    #[serde(default)]
    pub constitution: i32,
    #[serde(default)]
    pub looks: i32,
    #[serde(default)]
    pub charisma: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RaceSpec {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub category: String,
    pub base_hp: u32,
    pub size: String,
    #[serde(default)]
    pub knockback_size: Option<String>,
    #[serde(default)]
    pub ability_adjustments: AbilityAdjustments,
    #[serde(default)]
    pub pros: Vec<String>,
    #[serde(default)]
    pub cons: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AbilityKind {
    Strength,
    Dexterity,
    Intelligence,
    Wisdom,
    Constitution,
    Looks,
    Charisma,
}

impl AbilityKind {
    pub fn label(self) -> &'static str {
        match self {
            AbilityKind::Strength => "Strength",
            AbilityKind::Dexterity => "Dexterity",
            AbilityKind::Intelligence => "Intelligence",
            AbilityKind::Wisdom => "Wisdom",
            AbilityKind::Constitution => "Constitution",
            AbilityKind::Looks => "Looks",
            AbilityKind::Charisma => "Charisma",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TalentRequirement {
    MinLevel {
        level: u8,
    },
    MinStat {
        stat: AbilityKind,
        min_base: Option<u8>,
        min_percentile: Option<u8>,
    },
    RequiresTalent {
        id: String,
        min_rank: Option<u8>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TalentSpec {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub cost_bp: Option<u32>,
    #[serde(default)]
    pub cost_lp: Option<u32>,
    #[serde(default)]
    pub cost_rp: Option<u32>,
    #[serde(default = "default_talent_category")]
    pub category: String,
    #[serde(default)]
    pub race_categories: Vec<String>,
    #[serde(default)]
    pub race_ids: Vec<String>,
    #[serde(default)]
    pub requirements: Vec<TalentRequirement>,
    pub max_rank: u8,
    pub effects: Vec<TalentEffect>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct TalentSelection {
    pub id: String,
    #[serde(default = "default_talent_rank")]
    pub rank: u8,
    #[serde(default)]
    pub weapon: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TalentEffect {
    MagicTalent {
        talent: crate::core::magic::MagicTalent,
    },
    HitPointBonus {
        amount: i32,
    },
    EssenceAdvancement,
    ArmorDrBonus {
        amount: i32,
    },
    SpeedModBonus {
        amount: i32,
    },
    InitiativeModBonus {
        amount: i32,
    },
    AttackBonusWeapon {
        amount: i32,
    },
    DamageBonusWeapon {
        amount: i32,
    },
    DamageBonusWeaponGroup {
        amount: i32,
        weapon_group: String,
    },
    DefenseBonusWeapon {
        amount: i32,
    },
    InitiativeDieBonus {
        steps: i32,
    },
    Dodge {
        defense_bonus: i32,
        allow_dex_ranged: bool,
    },
    TraumaDieOverride {
        sides: i32,
        penetrating: bool,
    },
    ThresholdOfPainMultiplier {
        multiplier: f32,
    },
    ThresholdOfPainLevelBonus {
        per_level_pct: f32,
    },
    FastHealer,
    WeaponSpeedBonus {
        amount: i32,
        #[serde(default)]
        ranged_only: bool,
        #[serde(default)]
        weapon_group: Option<String>,
    },
    WeaponSpeedMultiplier {
        multiplier: f32,
        #[serde(default)]
        min_multiplier: Option<f32>,
        #[serde(default)]
        weapon_groups: Vec<String>,
        #[serde(default)]
        weapon_names: Vec<String>,
    },
    WeaponSpeedFlatBonus {
        amount: f32,
        #[serde(default)]
        min_reach_ft: Option<f32>,
        #[serde(default)]
        weapon_groups: Vec<String>,
        #[serde(default)]
        weapon_names: Vec<String>,
    },
    WeaponReachBonus {
        amount: i32,
    },
    WeaponReachMultiplier {
        multiplier: f32,
        #[serde(default)]
        weapon_groups: Vec<String>,
        #[serde(default)]
        weapon_names: Vec<String>,
    },
    WeaponReachFlatBonus {
        amount: f32,
        #[serde(default)]
        min_reach_ft: Option<f32>,
        #[serde(default)]
        weapon_groups: Vec<String>,
        #[serde(default)]
        weapon_names: Vec<String>,
    },
    CloseHitDamageExpr {
        expr: String,
        margin_less_than: i32,
        #[serde(default)]
        weapon_groups: Vec<String>,
        #[serde(default)]
        weapon_names: Vec<String>,
    },
    WeaponAttackBonus {
        amount: i32,
        #[serde(default)]
        weapon_groups: Vec<String>,
        #[serde(default)]
        weapon_names: Vec<String>,
    },
    CritMinRollWeaponGroup {
        min_roll: i32,
        #[serde(default)]
        ranged_only: bool,
    },
    CritSeverityBonusWeaponGroup {
        amount: i32,
    },
    WeaponDamageOptions {
        #[serde(default)]
        no_strength_bonus: bool,
        #[serde(default)]
        no_mastery_bonus: bool,
        #[serde(default)]
        force_nonpenetrating: bool,
        #[serde(default)]
        halve_damage: bool,
        #[serde(default)]
        ignore_all_dr: bool,
        #[serde(default)]
        internal_hemorrhage_damage: i32,
        #[serde(default)]
        melee_only: bool,
        #[serde(default)]
        hacking_or_piercing: Option<bool>,
        #[serde(default)]
        weapon_groups: Vec<String>,
        #[serde(default)]
        weapon_names: Vec<String>,
    },
    ExpandedPenetration {
        #[serde(default)]
        attack_defense_max_minus_one: bool,
        #[serde(default)]
        damage_max_minus_one: bool,
        #[serde(default)]
        weapon_groups: Vec<String>,
        #[serde(default)]
        weapon_names: Vec<String>,
    },
    OpeningEngagementExtraDamageDice {
        dice: i32,
        #[serde(default)]
        min_reach_ft: Option<f32>,
        #[serde(default)]
        weapon_groups: Vec<String>,
        #[serde(default)]
        weapon_names: Vec<String>,
    },
    AlwaysInitialEngagementIfReachAtLeastOpponent {
        #[serde(default)]
        min_reach_ft: Option<f32>,
        #[serde(default)]
        weapon_groups: Vec<String>,
        #[serde(default)]
        weapon_names: Vec<String>,
    },
    IgnoreDefenderMovementDefenseBonus {
        #[serde(default)]
        weapon_groups: Vec<String>,
        #[serde(default)]
        weapon_names: Vec<String>,
    },
    KnockbackResetsWeaponCount {
        #[serde(default)]
        weapon_groups: Vec<String>,
        #[serde(default)]
        weapon_names: Vec<String>,
    },
    HitCriticalEffectsNoExtraDice {
        #[serde(default)]
        weapon_groups: Vec<String>,
        #[serde(default)]
        weapon_names: Vec<String>,
    },
    IntAttackBonusToDamageAndDefense {
        fraction: f32,
        #[serde(default)]
        weapon_groups: Vec<String>,
        #[serde(default)]
        shield_names: Vec<String>,
    },
    ThrownFullStrengthDamage {
        #[serde(default)]
        thrown_only: bool,
        #[serde(default)]
        weapon_groups: Vec<String>,
        #[serde(default)]
        weapon_names: Vec<String>,
    },
    ConsecutiveHitsForceTraumaTwenty {
        hits: i32,
        #[serde(default)]
        weapon_groups: Vec<String>,
        #[serde(default)]
        weapon_names: Vec<String>,
    },
    ShieldDrBonusFiltered {
        amount: i32,
        #[serde(default)]
        shield_names: Vec<String>,
    },
    ShieldBreakageUsesShieldDr {
        #[serde(default)]
        shield_names: Vec<String>,
    },
    KnockbackStepBonus {
        amount: i32,
    },
    IncomingCritExtraDamageHalved,
    IncomingCritSeverityReduction {
        amount: i32,
    },
    IgnoreAncillaryCritEffects,
    IncomingCritDamageRollTwiceTakeLower,
    Precognition,
    Prescience,
    Eyesmite,
    Chronoblur,
    Streamline,
    Remarkability,
    NearPerfectDefenseMinRoll {
        roll: i32,
    },
    PerfectDefenseCounterForceCritical,
    FightDefensivelyAttackPenaltyDivisor {
        divisor: i32,
    },
    CalledShotDelayProfile {
        profile: String,
    },
    CalledShotTargetDefenseBonusDivisor {
        divisor: i32,
    },
    CalledShotSelfDefensePenalty {
        amount: i32,
    },
    CalledShotDeceptiveDefender,
    DualWieldOffhandDamagePenalty {
        amount: i32,
    },
    DualWieldPrimaryRecoveryPenalty {
        amount: f32,
    },
    DualWieldSecondaryRecoveryPenalty {
        amount: f32,
    },
    PerfectTwoWeaponFighting,
    RangeDistanceMultiplier {
        multiplier: f32,
    },
    ArmorInitiativePenaltyNegation,
    ArmorSpeedPenaltyNegation,
    ArmorDrBonusArmored {
        amount: i32,
    },
    LightArmorDefenseBonusFromDr {
        divisor: i32,
    },
    MediumArmorDrBonus {
        amount: i32,
    },
    MediumArmorDefensePenaltyReduction {
        amount: i32,
    },
    HeavyArmorDamageBonusFromDr {
        divisor: i32,
    },
    HeavyArmorDamageBonus {
        amount: i32,
    },
    ShieldDefenseBonus {
        amount: i32,
    },
    ShieldCoverValueAdjustment {
        amount: i32,
    },
    ForcedWeaponLoadout {
        weapon_name: String,
        #[serde(default)]
        min_weapon_material_tier: Option<i32>,
        #[serde(default)]
        clear_projectile_material: bool,
        #[serde(default)]
        disable_offhand: bool,
        #[serde(default)]
        force_two_hand_grip: bool,
        #[serde(default)]
        force_no_shield: bool,
        #[serde(default)]
        d6_penetration_triggers: Vec<i32>,
    },
    LargeSwordShieldStyle,
    ArmerociPoleStyle,
    CrescentMoonStyle,
    DoomrazorStyle,
    FallingSunStyle,
    FymblwngerStyle,
    HammererStyle,
    HobblerStyle,
    IthicanPrinceStyle,
    QuietRiverStyle,
    RegenstatStyle,
    ReturnerStyle,
    RhdwngFlowStyle,
    ScornOfTheDissendriStyle,
    ShieldOfBladesStyle,
    SixPathsStyle,
    StormOfBladesStyle,
    ThreeMountainsStyle,
    UnbreakableWallStyle,
    LeftHandOfEvoniaStyle,
    OnePathStyle,
    PilgrimsPathStyle,
    ReaperOfTermonStyle,
}

fn default_talent_rank() -> u8 {
    1
}

fn default_talent_category() -> String {
    "Uncategorized".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ability_kind_from_json() {
        let parsed: AbilityKind = serde_json::from_str("\"dexterity\"").unwrap();
        assert_eq!(parsed, AbilityKind::Dexterity);
    }

    #[test]
    fn parse_talent_requirements_from_json() {
        let json = r#"
        {
          "id": "tough_hide",
          "name": "Tough Hide",
          "description": "Test",
          "category": "Defense",
          "requirements": [
            { "type": "min_level", "level": 3 },
            { "type": "min_stat", "stat": "constitution", "min_base": 12 },
            { "type": "requires_talent", "id": "tough_as_nails", "min_rank": 1 }
          ],
          "max_rank": 1,
          "effects": []
        }
        "#;
        let parsed: TalentSpec = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.requirements.len(), 3);
        match &parsed.requirements[1] {
            TalentRequirement::MinStat {
                stat,
                min_base,
                min_percentile,
            } => {
                assert_eq!(*stat, AbilityKind::Constitution);
                assert_eq!(*min_base, Some(12));
                assert_eq!(*min_percentile, None);
            }
            _ => panic!("expected min_stat requirement"),
        }
    }
}
