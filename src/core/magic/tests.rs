use super::*;

#[test]
fn spell_failure_matrix_and_combat_casting_reduction() {
    for (encumbrance, base) in [
        (Encumbrance::None, 0),
        (Encumbrance::Light, 25),
        (Encumbrance::Moderate, 50),
        (Encumbrance::Heavy, 75),
    ] {
        for (armor, shield, extra) in [
            (false, false, 0),
            (true, false, 25),
            (false, true, 25),
            (true, true, 25),
        ] {
            assert_eq!(
                spell_failure_percent(encumbrance, armor, shield, MagicTalents::default()),
                base + extra
            );
            assert_eq!(
                spell_failure_percent(
                    encumbrance,
                    armor,
                    shield,
                    MagicTalents {
                        combat_casting: true,
                        ..MagicTalents::default()
                    }
                ),
                (base + extra).saturating_sub(25)
            );
        }
    }
}

#[test]
fn amplification_costs_match_each_purchased_increment() {
    let base = EchoStrikeOptions::default();
    assert_eq!(
        EchoStrikeOptions {
            extra_duration_seconds: 3,
            ..base
        }
        .cost(),
        Ok(165)
    );
    assert_eq!(
        EchoStrikeOptions {
            delay_seconds: 7,
            ..base
        }
        .cost(),
        Ok(210)
    );
    assert_eq!(
        EchoStrikeOptions {
            additional_echoes: 1,
            ..base
        }
        .cost(),
        Ok(300)
    );
    assert_eq!(
        EchoStrikeOptions {
            extra_duration_seconds: 2,
            delay_seconds: 8,
            additional_echoes: 1,
            ..base
        }
        .cost(),
        Ok(350)
    );
}

#[test]
fn all_proficiency_level_limits_cover_odd_and_even_character_levels() {
    for level in 1..=20 {
        assert_eq!(EssenceProficiency::I.maximum_spell_level(level), 1);
        assert_eq!(
            EssenceProficiency::II.maximum_spell_level(level),
            (level + 1) / 2
        );
        for tier in [
            EssenceProficiency::III,
            EssenceProficiency::IV,
            EssenceProficiency::V,
        ] {
            assert_eq!(tier.maximum_spell_level(level), level);
        }
    }
}

#[test]
fn loadout_serialization_preserves_settings_and_old_presets_default_to_no_magic() {
    let defaults: MagicLoadout = serde_json::from_str("{}").unwrap();
    assert_eq!(defaults, MagicLoadout::default());
    let loadout = MagicLoadout {
        knows_echo_strike: true,
        auto_cast: AutoCast::InWeaponReach,
        echo_strike: EchoStrikeOptions {
            full_damage: true,
            ..EchoStrikeOptions::default()
        },
        essences: vec![EssencePool {
            essence_id: "test".into(),
            proficiency: EssenceProficiency::V,
            current: 450,
            capacity: 500,
        }],
        ..defaults
    };
    let encoded = serde_json::to_string(&loadout).unwrap();
    assert_eq!(
        serde_json::from_str::<MagicLoadout>(&encoded).unwrap(),
        loadout
    );
}

#[test]
fn saves_use_the_selected_save_category() {
    let bonuses = MagicSaveBonuses {
        physical: 2,
        mental: 5,
        dodge: -1,
    };
    assert_eq!(bonuses.for_kind(MagicSaveKind::Physical), 2);
    assert_eq!(bonuses.for_kind(MagicSaveKind::Mental), 5);
    assert_eq!(bonuses.for_kind(MagicSaveKind::Dodge), -1);
}

#[test]
fn seeded_save_rolls_are_reproducible_and_use_bonus_once() {
    use crate::core::rng::SimRng;
    let baseline = roll_magic_save(10, 3, &mut SimRng::from_seed(77));
    assert_eq!(baseline, roll_magic_save(10, 3, &mut SimRng::from_seed(77)));
    let boosted = roll_magic_save(14, 15, &mut SimRng::from_seed(77));
    assert_eq!(boosted.caster_total, baseline.caster_total + 4);
    assert_eq!(boosted.target_total, baseline.target_total + 12);
}

#[test]
fn malformed_or_over_budget_loadouts_fail_before_casting() {
    let pool = EssencePool {
        essence_id: "test".into(),
        proficiency: EssenceProficiency::V,
        capacity: 1000,
        current: 1000,
    };
    let mut loadout = MagicLoadout {
        knows_echo_strike: true,
        use_essence_costs: true,
        essences: vec![pool],
        ..MagicLoadout::default()
    };
    loadout.echo_strike.additional_echoes = 3;
    assert_eq!(
        loadout.validate_echo(20),
        Err(MagicError::AmplificationLimit)
    );
    loadout.echo_strike.additional_echoes = 0;
    loadout.echo_essence = 1;
    assert_eq!(loadout.validate_echo(20), Err(MagicError::InvalidEssence));
    loadout.echo_essence = 0;
    loadout.essences[0].essence_id.clear();
    assert_eq!(loadout.validate_echo(20), Err(MagicError::InvalidEssence));
}

#[test]
fn proficiency_enforces_spell_levels_and_exact_amplification_boundaries() {
    assert_eq!(EssenceProficiency::I.maximum_spell_level(20), 1);
    assert_eq!(EssenceProficiency::II.maximum_spell_level(11), 6);
    assert_eq!(EssenceProficiency::III.maximum_spell_level(11), 11);
    assert_eq!(
        EssenceProficiency::III.validate_cost(150, 151),
        Err(MagicError::InsufficientProficiency)
    );
    assert_eq!(
        EssenceProficiency::IV.validate_cost(150, 300),
        Ok(Amplification::Amplified)
    );
    assert_eq!(
        EssenceProficiency::IV.validate_cost(150, 301),
        Err(MagicError::InsufficientProficiency)
    );
    assert_eq!(
        EssenceProficiency::V.validate_cost(150, 450),
        Ok(Amplification::Overamplified)
    );
    assert_eq!(
        EssenceProficiency::V.validate_cost(150, 451),
        Err(MagicError::AmplificationLimit)
    );
    assert_eq!(
        EssenceProficiency::V.validate_cost(u32::MAX, u32::MAX),
        Ok(Amplification::None)
    );
}

#[test]
fn echo_strike_is_one_spell_and_full_damage_requires_overamplification() {
    let base = EchoStrikeOptions::default();
    assert_eq!(base.cost(), Ok(150));
    assert_eq!(base.duration_seconds(), Ok(15));
    assert_eq!(
        base.validate(9, EssenceProficiency::V),
        Err(MagicError::SpellLevelTooHigh)
    );
    let full = EchoStrikeOptions {
        full_damage: true,
        ..base
    };
    assert_eq!(full.cost(), Ok(350));
    assert_eq!(
        full.validate(10, EssenceProficiency::IV),
        Err(MagicError::InsufficientProficiency)
    );
    assert_eq!(
        full.validate(10, EssenceProficiency::V),
        Ok(Amplification::Overamplified)
    );
    assert_eq!(
        EchoStrikeOptions {
            delay_seconds: 0,
            ..base
        }
        .cost(),
        Err(MagicError::InvalidEchoDelay)
    );
    assert_eq!(
        EchoStrikeOptions {
            additional_echoes: u32::MAX,
            ..base
        }
        .cost(),
        Err(MagicError::CostOverflow)
    );
    assert_eq!(
        EchoStrikeOptions {
            delay_seconds: 1,
            full_damage: true,
            ..base
        }
        .validate(10, EssenceProficiency::V),
        Err(MagicError::AmplificationLimit)
    );
}

#[test]
fn essence_pools_reject_duplicates_excess_capacity_and_a_third_essence() {
    assert_eq!(essence_capacity(17, 15), Ok(260));
    assert_eq!(essence_capacity(17, 12), Ok(200));
    let pool = EssencePool {
        essence_id: "test".into(),
        proficiency: EssenceProficiency::III,
        capacity: 200,
        current: 150,
    };
    assert_eq!(validate_essences(std::slice::from_ref(&pool)), Ok(()));
    assert_eq!(
        validate_essences(&[pool.clone(), pool.clone()]),
        Err(MagicError::DuplicateEssence)
    );
    assert_eq!(
        validate_essences(&[pool.clone(), pool.clone(), pool.clone()]),
        Err(MagicError::TooManyEssences)
    );
    assert_eq!(
        validate_essences(&[EssencePool {
            current: 201,
            ..pool
        }]),
        Err(MagicError::EssenceExceedsCapacity)
    );
}

#[test]
fn fatigue_talents_have_distinct_effects_and_focus_upgrades_do_not_stack() {
    let mut talents = MagicTalents::default();
    assert_eq!(talents.fatigue_seconds(1), 6);
    assert_eq!(talents.fatigue_penalties().defense, -6);
    talents.apply(MagicTalent::MitigateSpellFatigue, 1);
    assert_eq!(talents.fatigue_penalties().defense, -3);
    assert!(!talents.fatigue_penalties().can_attack);
    assert!(!talents.fatigue_penalties().can_run_or_sprint);
    talents.apply(MagicTalent::DecimateSpellFatigue, 1);
    assert!(talents.fatigue_penalties().can_attack);
    assert!(!talents.fatigue_penalties().can_cast);
    talents.apply(MagicTalent::DiminishSpellFatigue, 5);
    assert_eq!(talents.fatigue_seconds(1), 1);
    talents.apply(MagicTalent::EliminateSpellFatigue, 1);
    assert_eq!(talents.fatigue_seconds(8), 0);
    talents.apply(MagicTalent::MagicMaster, 1);
    talents.apply(MagicTalent::MagicFocus, 1);
    talents.apply(MagicTalent::MagicSpecialist, 1);
    assert_eq!(talents.focus_bonus(19), 4);
}

#[test]
fn verbal_component_is_only_required_on_completion_and_talents_remove_components() {
    let components = SpellComponents {
        somatic: true,
        verbal: true,
    };
    let mut talents = MagicTalents::default();
    assert_eq!(components.validate(talents, true, false, false), Ok(()));
    assert_eq!(
        components.validate(talents, true, false, true),
        Err(MagicError::VerbalComponentBlocked)
    );
    assert_eq!(
        components.validate(talents, false, true, false),
        Err(MagicError::SomaticComponentBlocked)
    );
    talents.apply(MagicTalent::StillCasting, 1);
    talents.apply(MagicTalent::SilentCasting, 1);
    assert_eq!(components.validate(talents, false, false, true), Ok(()));
}

#[test]
fn saves_use_ties_and_natural_ones_as_specified() {
    assert!(MagicSaveRoll::from_rolls(10, 20, 15, 20).saved);
    assert!(!MagicSaveRoll::from_rolls(10, 20, 1, 30).saved);
    assert!(MagicSaveRoll::from_rolls(1, 20, 1, 30).saved);
    assert!(!MagicSaveRoll::from_rolls(1, 20, 1, 19).saved);
    let talents = MagicTalents {
        charm_resistant: true,
        illusion_resistant: true,
        sleep_resistant: true,
        ..MagicTalents::default()
    };
    assert_eq!(
        talents.resistance_bonus(SpellTags {
            charm: true,
            ..SpellTags::default()
        }),
        12
    );
    assert_eq!(
        talents.resistance_bonus(SpellTags {
            illusion: true,
            ..SpellTags::default()
        }),
        6
    );
    assert_eq!(talents.resistance_bonus(SpellTags::default()), 0);
}

#[test]
fn spell_selections_ai_and_empowerments_round_trip_without_essence_setup() {
    let mut loadout = MagicLoadout::default();
    for spell in SPELL_CATALOG {
        loadout.learn_spell(spell.id);
    }
    loadout
        .spell_ai
        .insert("echo_strike".into(), SpellCastAi::AsOftenAsPossible);
    loadout.echo_strike.additional_echoes = 3;
    loadout.echo_strike.full_damage = true;
    loadout.chronoblur_duration_ranks = 2;
    loadout.streamline_duration_ranks = 3;
    loadout.streamline_radius_ranks = 4;
    let restored: MagicLoadout =
        serde_json::from_str(&serde_json::to_string(&loadout).unwrap()).unwrap();
    assert_eq!(restored, loadout);
    assert_eq!(restored.validate_echo(10), Ok(0));
    assert!(restored.essences.is_empty());
}

#[test]
fn learning_is_idempotent_and_removing_legacy_echo_clears_knowledge_and_ai() {
    let mut loadout = MagicLoadout {
        knows_echo_strike: true,
        ..MagicLoadout::default()
    };
    loadout.learn_spell("echo_strike");
    loadout.learn_spell("echo_strike");
    assert_eq!(loadout.known_spells.len(), 1);
    loadout
        .spell_ai
        .insert("echo_strike".into(), SpellCastAi::Manual);
    loadout.forget_spell("echo_strike");
    assert!(!loadout.knows_spell("echo_strike"));
    assert!(loadout.spell_ai.is_empty());
}
