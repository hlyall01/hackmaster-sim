    use super::*;
    use super::combat::{defense_die_sides, resolve_attack, AttackMode};
    use crate::core::rng::SimRng;
    use crate::core::rules::{
        clean_damage_expr, evaluate_expression_with_detail, penetrating_roll_with,
        roll_damage_expr_with_detail, DamageExprCache,
    };
    use rand::SeedableRng;
    use std::time::{Duration, Instant};

    fn combatant_basic(
        name: String,
        weapon_name: String,
        attack_bonus: i32,
        defense_mod: i32,
        armor_dr: i32,
        armor_is_heavy: bool,
        armor_penetration: i32,
        damage_expr: String,
        strength_damage: i32,
        weapon_speed: f32,
        reach_ft: f32,
        move_speed: f32,
        two_hand_grip: bool,
        use_jab: bool,
        jab_special_expr: Option<String>,
        has_weapon: bool,
        weapon_defense_always: bool,
        max_hp: i32,
    ) -> Combatant {
        let uses_projectiles = matches!(
            weapon_name.as_str(),
            "Shortbow"
                | "Recurve bow"
                | "Longbow"
                | "Warbow"
                | "Light crossbow"
                | "Heavy crossbow"
                | "Hand crossbow"
                | "Arbalest"
                | "Sling"
        );
        let damage_expr_cache = DamageExprCache::new(&damage_expr);
        let jab_special_expr_cache = jab_special_expr
            .as_deref()
            .map(DamageExprCache::new);
        let sheet = CombatantSheet {
            name,
            offense: OffenseProfile {
                attack_bonus,
                strength_damage,
                weapon: WeaponProfile {
                    name: weapon_name,
                    damage_expr,
                    damage_expr_cache,
                    shield_damage_expr: None,
                    shield_damage_expr_cache: None,
                    armor_penetration,
                    speed: weapon_speed,
                    reach_ft,
                    range_bands_feet: None,
                    range_distance_multiplier: 1.0,
                    two_hand_grip,
                    use_jab,
                    jab_special_expr,
                    jab_special_expr_cache,
                    has_weapon,
                    defense_bonus_always: weapon_defense_always,
                    uses_projectiles,
                },
            },
            defense: DefenseProfile {
                defense_mod,
                ranged_defense_mod: 0,
                armor_dr,
                armor_is_heavy,
                shield_name: None,
                shield_defense_bonus: 0,
                shield_dr: 0,
                shield_cover_value: None,
                shield_breakage: None,
            },
            mobility: MobilityProfile { move_speed },
            vitals: Vitals {
                trauma_die_sides: 20,
                trauma_die_penetrating: false,
                max_hp,
                constitution: 10,
                threshold_of_pain: 3,
            },
            maneuvers: ManeuverProfile::default(),
        };
        Combatant::new(sheet)
    }

    fn make_state(attacker: Combatant, defender: Combatant) -> SimState {
        let mut state = SimState::new(SimConfig::new(10.0, 1.0));
        state.combatants = [attacker, defender];
        state
    }

    #[test]
    fn lower_of_damage_expr_is_parsed_for_shield_damage() {
        assert_eq!(clean_damage_expr("lower of 2d6p"), "2d6p");
    }

    struct SeqRng {
        values: Vec<u32>,
        idx: usize,
    }

    impl SeqRng {
        fn new(values: Vec<u32>) -> Self {
            Self { values, idx: 0 }
        }
    }

    impl rand::RngCore for SeqRng {
        fn next_u32(&mut self) -> u32 {
            let value = self.values[self.idx % self.values.len()];
            self.idx += 1;
            value
        }

        fn next_u64(&mut self) -> u64 {
            self.next_u32() as u64
        }

        fn fill_bytes(&mut self, dest: &mut [u8]) {
            for byte in dest.iter_mut() {
                *byte = self.next_u32() as u8;
            }
        }

        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
            self.fill_bytes(dest);
            Ok(())
        }
    }

    #[test]
    fn lower_of_damage_expr_uses_lower_total() {
        let cleaned = clean_damage_expr("lower of 2d6p");
        let mut expected_rng = SeqRng::new(vec![0, 5, 1, 4]);
        let (a_total, _) = evaluate_expression_with_detail(&cleaned, &mut expected_rng);
        let (b_total, _) = evaluate_expression_with_detail(&cleaned, &mut expected_rng);
        let expected = a_total.min(b_total);

        let mut rng = SeqRng::new(vec![0, 5, 1, 4]);
        let (total, detail) = roll_damage_expr_with_detail("lower of 2d6p", &mut rng);
        assert_eq!(total, expected);
        assert!(detail.contains("lower of"));
    }

    #[test]
    fn attack_miss_does_no_damage() {
        let attacker = combatant_basic(
            "Attacker".to_string(),
            "Test Blade".to_string(),
            0,
            0,
            0,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            false,
            None,
            true,
            false,
            20,
        );
        let defender = combatant_basic(
            "Defender".to_string(),
            "Shield".to_string(),
            0,
            1000,
            0,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            false,
            None,
            true,
            false,
            20,
        );
        let mut state = make_state(attacker, defender);
        let mut rng = rand::rngs::StdRng::seed_from_u64(1);
        let _ = resolve_attack(
            &mut state.combatants,
            0,
            1,
            0,
            false,
            AttackMode::Normal,
            0.0,
            None,
            &mut rng,
        );
        assert_eq!(state.combatants[1].state.hp, 20);
    }

    #[test]
    fn bulk_sim_100k_under_one_second() {
        if cfg!(debug_assertions) {
            return;
        }
        let attacker = combatant_basic(
            "Attacker".to_string(),
            "Antler".to_string(),
            6,
            5,
            1,
            false,
            0,
            "2d6p".to_string(),
            2,
            10.0,
            3.0,
            20.0,
            false,
            false,
            None,
            true,
            false,
            18,
        );
        let defender = combatant_basic(
            "Defender".to_string(),
            "Claw".to_string(),
            5,
            6,
            1,
            false,
            0,
            "1d8p".to_string(),
            2,
            9.0,
            1.0,
            20.0,
            false,
            false,
            None,
            true,
            false,
            18,
        );
        let config = SimConfig::new(200.0, 3.0);
        let start = Instant::now();
        let _ = bulk_simulate(config, [attacker, defender], 100_000, 60);
        let elapsed = start.elapsed();
        assert!(
            elapsed <= Duration::from_secs(1),
            "bulk sim 100k took {:?}",
            elapsed
        );
    }

    #[test]
    fn mirror_match_winrate_within_two_percent() {
        if cfg!(debug_assertions) {
            return;
        }
        let combatant = combatant_basic(
            "Mirror".to_string(),
            "Test Blade".to_string(),
            4,
            4,
            2,
            false,
            0,
            "2d6p".to_string(),
            2,
            6.0,
            4.0,
            10.0,
            true,
            false,
            None,
            true,
            false,
            30,
        );
        let config = SimConfig::new(4.0, 1.0);
        let runs = 100_000u32;
        let mut sim = SimState::with_logging(config, false);
        sim.reset_with_combatants([combatant.clone(), combatant]);
        sim.set_rng(SimRng::from_seed(42));
        let mut wins = [0u32; 2];
        let mut ties = 0u32;
        for _ in 0..runs {
            sim.reset_preserve_rng();
            while !sim.done && sim.elapsed_seconds < 60 {
                sim.update(1.0);
            }
            let hp_a = sim.combatants[0].state.hp;
            let hp_b = sim.combatants[1].state.hp;
            if sim.done {
                if hp_a <= 0 && hp_b <= 0 {
                    ties += 1;
                } else if hp_a <= 0 {
                    wins[1] += 1;
                } else if hp_b <= 0 {
                    wins[0] += 1;
                } else {
                    ties += 1;
                }
            } else {
                ties += 1;
            }
        }
        let diff = if wins[0] > wins[1] {
            wins[0] - wins[1]
        } else {
            wins[1] - wins[0]
        };
        let max_diff = runs / 50;
        assert!(
            diff <= max_diff,
            "mirror winrate diff {} exceeds 2% (wins={:?}, ties={})",
            diff,
            wins,
            ties
        );
    }

    #[test]
    fn hold_at_bay_hit_without_jab_deals_no_damage() {
        let attacker = combatant_basic(
            "Attacker".to_string(),
            "Test Spear".to_string(),
            100,
            0,
            0,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            6.0,
            5.0,
            false,
            false,
            None,
            true,
            false,
            20,
        );
        let defender = combatant_basic(
            "Defender".to_string(),
            "Short Sword".to_string(),
            0,
            0,
            0,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            3.0,
            5.0,
            false,
            false,
            None,
            true,
            false,
            20,
        );
        let mut rng = SeqRng::new(vec![0]);
        let mut state = make_state(attacker, defender);
        let _ = resolve_attack(
            &mut state.combatants,
            0,
            1,
            0,
            false,
            AttackMode::HoldAtBay,
            0.0,
            None,
            &mut rng,
        );
        assert_eq!(state.combatants[1].state.hp, 20);
    }

    #[test]
    fn hold_at_bay_hit_with_jab_deals_damage() {
        let attacker = combatant_basic(
            "Attacker".to_string(),
            "Test Spear".to_string(),
            100,
            0,
            0,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            6.0,
            5.0,
            false,
            true,
            Some("1d1".to_string()),
            true,
            false,
            20,
        );
        let defender = combatant_basic(
            "Defender".to_string(),
            "Short Sword".to_string(),
            0,
            0,
            0,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            3.0,
            5.0,
            false,
            false,
            None,
            true,
            false,
            20,
        );
        let mut rng = SeqRng::new(vec![0]);
        let mut state = make_state(attacker, defender);
        let _ = resolve_attack(
            &mut state.combatants,
            0,
            1,
            0,
            false,
            AttackMode::HoldAtBay,
            0.0,
            None,
            &mut rng,
        );
        assert_eq!(state.combatants[1].state.hp, 19);
    }

    #[test]
    fn equal_reach_allows_double_ko() {
        let attacker = combatant_basic(
            "Attacker".to_string(),
            "Test Blade".to_string(),
            100,
            0,
            0,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            0.0,
            false,
            false,
            None,
            true,
            false,
            1,
        );
        let defender = combatant_basic(
            "Defender".to_string(),
            "Test Blade".to_string(),
            100,
            0,
            0,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            0.0,
            false,
            false,
            None,
            true,
            false,
            1,
        );
        let mut sim = SimState::new(SimConfig::new(1.0, 1.0));
        sim.combatants = [attacker, defender];
        sim.set_rng(SimRng::from_seed(1));
        sim.tick();
        assert!(sim.done);
        assert!(sim.combatants[0].state.hp <= 0);
        assert!(sim.combatants[1].state.hp <= 0);
    }

    #[test]
    fn equal_reach_trauma_does_not_block_simultaneous_attacks() {
        let sheet = CombatantSheet {
            name: "Test".to_string(),
            offense: OffenseProfile {
                attack_bonus: 100,
                strength_damage: 0,
                weapon: WeaponProfile {
                    name: "Test Blade".to_string(),
                    damage_expr: "1d1".to_string(),
                    damage_expr_cache: DamageExprCache::new("1d1"),
                    shield_damage_expr: None,
                    shield_damage_expr_cache: None,
                    armor_penetration: 0,
                    speed: 10.0,
                    reach_ft: 1.0,
                    range_bands_feet: None,
                    range_distance_multiplier: 1.0,
                    two_hand_grip: false,
                    use_jab: false,
                    jab_special_expr: None,
                    jab_special_expr_cache: None,
                    has_weapon: true,
                    defense_bonus_always: false,
                    uses_projectiles: false,
                },
            },
            defense: DefenseProfile {
                ranged_defense_mod: 0,
                defense_mod: 0,
                armor_dr: 0,
                armor_is_heavy: false,
                shield_name: None,
                shield_defense_bonus: 0,
                shield_dr: 0,
                shield_cover_value: None,
                shield_breakage: None,
            },
            mobility: MobilityProfile { move_speed: 0.0 },
            vitals: Vitals {
                trauma_die_sides: 20,
                trauma_die_penetrating: false,
                max_hp: 10,
                constitution: 1,
                threshold_of_pain: 0,
            },
            maneuvers: ManeuverProfile::default(),
        };
        let mut sim = SimState::new(SimConfig::new(1.0, 1.0));
        sim.combatants = [Combatant::new(sheet.clone()), Combatant::new(sheet)];
        sim.tick();
        assert!(sim.combatants[0].state.hp < sim.combatants[0].sheet.vitals.max_hp);
        assert!(sim.combatants[1].state.hp < sim.combatants[1].sheet.vitals.max_hp);
    }

    #[test]
    fn equal_reach_knockback_does_not_block_simultaneous_attacks() {
        let sheet = CombatantSheet {
            name: "Test".to_string(),
            offense: OffenseProfile {
                attack_bonus: 100,
                strength_damage: 0,
                weapon: WeaponProfile {
                    name: "Test Blade".to_string(),
                    damage_expr: "30".to_string(),
                    damage_expr_cache: DamageExprCache::new("30"),
                    shield_damage_expr: None,
                    shield_damage_expr_cache: None,
                    armor_penetration: 0,
                    speed: 10.0,
                    reach_ft: 1.0,
                    range_bands_feet: None,
                    range_distance_multiplier: 1.0,
                    two_hand_grip: false,
                    use_jab: false,
                    jab_special_expr: None,
                    jab_special_expr_cache: None,
                    has_weapon: true,
                    defense_bonus_always: false,
                    uses_projectiles: false,
                },
            },
            defense: DefenseProfile {
                ranged_defense_mod: 0,
                defense_mod: 0,
                armor_dr: 0,
                armor_is_heavy: false,
                shield_name: None,
                shield_defense_bonus: 0,
                shield_dr: 0,
                shield_cover_value: None,
                shield_breakage: None,
            },
            mobility: MobilityProfile { move_speed: 0.0 },
            vitals: Vitals {
                trauma_die_sides: 20,
                trauma_die_penetrating: false,
                max_hp: 100,
                constitution: 10,
                threshold_of_pain: 0,
            },
            maneuvers: ManeuverProfile::default(),
        };
        let mut sim = SimState::new(SimConfig::new(1.0, 1.0));
        sim.combatants = [Combatant::new(sheet.clone()), Combatant::new(sheet)];
        sim.set_rng(SimRng::from_seed(1));
        sim.tick();
        assert!(sim.combatants[0].state.hp < sim.combatants[0].sheet.vitals.max_hp);
        assert!(sim.combatants[1].state.hp < sim.combatants[1].sheet.vitals.max_hp);
    }

    #[test]
    fn damage_respects_dr_under_five() {
        let attacker = combatant_basic(
            "Attacker".to_string(),
            "Test Blade".to_string(),
            100,
            0,
            0,
            false,
            2,
            "1d1".to_string(),
            5,
            10.0,
            1.0,
            5.0,
            false,
            false,
            None,
            true,
            false,
            20,
        );
        let defender = combatant_basic(
            "Defender".to_string(),
            "Shield".to_string(),
            0,
            0,
            4,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            false,
            None,
            true,
            false,
            20,
        );
        let mut state = make_state(attacker, defender);
        let mut rng = rand::rngs::StdRng::seed_from_u64(2);
        let _ = resolve_attack(
            &mut state.combatants,
            0,
            1,
            0,
            false,
            AttackMode::Normal,
            0.0,
            None,
            &mut rng,
        );
        assert_eq!(state.combatants[1].state.hp, 18);
    }

    #[test]
    fn damage_applies_armor_penetration_when_dr_high() {
        let attacker = combatant_basic(
            "Attacker".to_string(),
            "Test Blade".to_string(),
            100,
            0,
            0,
            false,
            2,
            "1d1".to_string(),
            5,
            10.0,
            1.0,
            5.0,
            false,
            false,
            None,
            true,
            false,
            20,
        );
        let defender = combatant_basic(
            "Defender".to_string(),
            "Shield".to_string(),
            0,
            0,
            6,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            false,
            None,
            true,
            false,
            20,
        );
        let mut state = make_state(attacker, defender);
        let mut rng = rand::rngs::StdRng::seed_from_u64(3);
        let _ = resolve_attack(
            &mut state.combatants,
            0,
            1,
            0,
            false,
            AttackMode::Normal,
            0.0,
            None,
            &mut rng,
        );
        assert_eq!(state.combatants[1].state.hp, 18);
    }

    #[test]
    fn negative_penetration_increases_effective_dr() {
        let attacker = combatant_basic(
            "Attacker".to_string(),
            "Test Blade".to_string(),
            100,
            0,
            0,
            false,
            -1,
            "1d1".to_string(),
            5,
            10.0,
            1.0,
            5.0,
            false,
            false,
            None,
            true,
            false,
            20,
        );
        let defender = combatant_basic(
            "Defender".to_string(),
            "Shield".to_string(),
            0,
            0,
            6,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            false,
            None,
            true,
            false,
            20,
        );
        let mut state = make_state(attacker, defender);
        let mut rng = rand::rngs::StdRng::seed_from_u64(4);
        let _ = resolve_attack(
            &mut state.combatants,
            0,
            1,
            0,
            false,
            AttackMode::Normal,
            0.0,
            None,
            &mut rng,
        );
        assert_eq!(state.combatants[1].state.hp, 20);
    }

    #[test]
    fn damage_can_reduce_to_zero_after_dr() {
        let attacker = combatant_basic(
            "Attacker".to_string(),
            "Test Blade".to_string(),
            100,
            0,
            0,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            false,
            None,
            true,
            false,
            20,
        );
        let defender = combatant_basic(
            "Defender".to_string(),
            "Shield".to_string(),
            0,
            0,
            10,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            false,
            None,
            true,
            false,
            20,
        );
        let mut state = make_state(attacker, defender);
        let mut rng = rand::rngs::StdRng::seed_from_u64(5);
        let _ = resolve_attack(
            &mut state.combatants,
            0,
            1,
            0,
            false,
            AttackMode::Normal,
            0.0,
            None,
            &mut rng,
        );
        assert_eq!(state.combatants[1].state.hp, 20);
    }

    struct FixedRng(u64);

    impl rand::RngCore for FixedRng {
        fn next_u32(&mut self) -> u32 {
            self.0 as u32
        }

        fn next_u64(&mut self) -> u64 {
            self.0
        }

        fn fill_bytes(&mut self, dest: &mut [u8]) {
            for byte in dest.iter_mut() {
                *byte = self.0 as u8;
            }
        }

        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
            self.fill_bytes(dest);
            Ok(())
        }
    }

    #[test]
    fn two_hand_grip_bonus_ready_on_attack_timer() {
        let mut combatant = combatant_basic(
            "Attacker".to_string(),
            "Test Blade".to_string(),
            0,
            0,
            0,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            true,
            false,
            None,
            true,
            false,
            20,
        );
        assert!(combatant.state.defense_plus_four_ready);

        combatant.state.next_attack_time = Some(2.0);
        combatant
            .state
            .refresh_defense_plus_four_ready(&combatant.sheet, 1.0);
        assert!(!combatant.state.defense_plus_four_ready);

        combatant
            .state
            .refresh_defense_plus_four_ready(&combatant.sheet, 2.0);
        assert!(combatant.state.defense_plus_four_ready);
    }

    #[test]
    fn defensive_dualwielding_bonus_ready_on_attack_timer() {
        let mut combatant = combatant_basic(
            "Attacker".to_string(),
            "Test Blade".to_string(),
            0,
            0,
            0,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            false,
            None,
            true,
            false,
            20,
        );
        combatant.sheet.maneuvers.defensive_dualwielding = true;
        combatant
            .state
            .refresh_defense_plus_four_ready(&combatant.sheet, 0.0);
        assert!(combatant.state.defense_plus_four_ready);

        combatant.state.next_attack_time = Some(2.0);
        combatant
            .state
            .refresh_defense_plus_four_ready(&combatant.sheet, 1.0);
        assert!(!combatant.state.defense_plus_four_ready);

        combatant
            .state
            .refresh_defense_plus_four_ready(&combatant.sheet, 2.0);
        assert!(combatant.state.defense_plus_four_ready);
    }

    #[test]
    fn poleaxe_always_gets_defense_bonus() {
        let attacker = combatant_basic(
            "Attacker".to_string(),
            "Test Blade".to_string(),
            0,
            0,
            0,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            false,
            None,
            true,
            false,
            20,
        );
        let defender = combatant_basic(
            "Defender".to_string(),
            "Poleaxe".to_string(),
            0,
            0,
            0,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            false,
            None,
            true,
            true,
            20,
        );
        let mut state = make_state(attacker, defender);
        let mut rng = FixedRng(0);
        let _ = resolve_attack(
            &mut state.combatants,
            0,
            1,
            0,
            false,
            AttackMode::Normal,
            0.0,
            None,
            &mut rng,
        );
        assert_eq!(state.combatants[1].state.hp, 20);
    }

    #[test]
    fn ranged_stationary_uses_d12p_defense() {
        assert_eq!(defense_die_sides(true, false, false, false), 12);
    }

    #[test]
    fn ranged_moving_uses_d20p_defense() {
        assert_eq!(defense_die_sides(true, true, false, false), 20);
    }

    #[test]
    fn ranged_stationary_with_shield_uses_d20p_defense() {
        assert_eq!(defense_die_sides(true, false, true, false), 20);
    }

    #[test]
    fn moving_flag_set_when_positions_change() {
        let mut state = SimState::new(SimConfig::new(500.0, 1.0));
        let ranged = combatant_basic(
            "Archer".to_string(),
            "Longbow".to_string(),
            0,
            0,
            0,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            false,
            None,
            true,
            false,
            10,
        );
        state.reset_with_combatants([ranged.clone(), ranged]);
        state.tick();
        assert!(state.combatants[0].state.moved_last_tick);
        assert!(state.combatants[1].state.moved_last_tick);
    }

    #[test]
    fn moving_flag_clear_when_no_movement() {
        let mut state = SimState::new(SimConfig::new(20.0, 1.0));
        let melee = combatant_basic(
            "Fighter".to_string(),
            "Sword".to_string(),
            0,
            0,
            0,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            0.0,
            false,
            false,
            None,
            true,
            false,
            10,
        );
        state.reset_with_combatants([melee.clone(), melee]);
        state.tick();
        assert!(!state.combatants[0].state.moved_last_tick);
        assert!(!state.combatants[1].state.moved_last_tick);
    }

    #[test]
    fn throwing_axe_switches_to_melee_at_close_range() {
        let throwing_axe = WeaponProfile {
            name: "Throwing axe".to_string(),
            damage_expr: "1d1".to_string(),
            damage_expr_cache: DamageExprCache::new("1d1"),
            shield_damage_expr: None,
            shield_damage_expr_cache: None,
            armor_penetration: 0,
            speed: 1.0,
            reach_ft: 1.0,
            range_bands_feet: Some([20.0, 30.0, 40.0, 60.0]),
            range_distance_multiplier: 1.0,
            two_hand_grip: false,
            use_jab: false,
            jab_special_expr: None,
            jab_special_expr_cache: None,
            has_weapon: true,
            defense_bonus_always: false,
            uses_projectiles: false,
        };
        let melee_weapon = WeaponProfile {
            name: "Sword".to_string(),
            damage_expr: "1d1".to_string(),
            damage_expr_cache: DamageExprCache::new("1d1"),
            shield_damage_expr: None,
            shield_damage_expr_cache: None,
            armor_penetration: 0,
            speed: 1.0,
            reach_ft: 1.0,
            range_bands_feet: None,
            range_distance_multiplier: 1.0,
            two_hand_grip: false,
            use_jab: false,
            jab_special_expr: None,
            jab_special_expr_cache: None,
            has_weapon: true,
            defense_bonus_always: false,
            uses_projectiles: false,
        };
        let attacker = Combatant::new(CombatantSheet {
            name: "Thrower".to_string(),
            offense: OffenseProfile {
                attack_bonus: 0,
                strength_damage: 0,
                weapon: throwing_axe,
            },
            defense: DefenseProfile {
                ranged_defense_mod: 0,
                defense_mod: 0,
                armor_dr: 0,
                armor_is_heavy: false,
                shield_name: None,
                shield_defense_bonus: 0,
                shield_dr: 0,
                shield_cover_value: None,
                shield_breakage: None,
            },
            mobility: MobilityProfile { move_speed: 10.0 },
            vitals: Vitals {
                trauma_die_sides: 20,
                trauma_die_penetrating: false,
                max_hp: 1000,
                constitution: 10,
                threshold_of_pain: 0,
            },
            maneuvers: ManeuverProfile::default(),
        });
        let defender = Combatant::new(CombatantSheet {
            name: "Defender".to_string(),
            offense: OffenseProfile {
                attack_bonus: 0,
                strength_damage: 0,
                weapon: melee_weapon,
            },
            defense: DefenseProfile {
                ranged_defense_mod: 0,
                defense_mod: 0,
                armor_dr: 0,
                armor_is_heavy: false,
                shield_name: None,
                shield_defense_bonus: 0,
                shield_dr: 0,
                shield_cover_value: None,
                shield_breakage: None,
            },
            mobility: MobilityProfile { move_speed: 10.0 },
            vitals: Vitals {
                trauma_die_sides: 20,
                trauma_die_penetrating: false,
                max_hp: 1000,
                constitution: 10,
                threshold_of_pain: 0,
            },
            maneuvers: ManeuverProfile::default(),
        });

        let mut sim = SimState::new(SimConfig::new(40.0, 1.0));
        sim.reset_with_combatants([attacker, defender]);

        let mut first_ranged: Option<bool> = None;
        let mut melee_after_close: Option<bool> = None;
        let mut seen_events = 0usize;

        for _ in 0..200 {
            let distance_before = sim.distance();
            sim.tick();

            if sim.combat_events.len() > seen_events {
                for event in &sim.combat_events[seen_events..] {
                    if event.attacker_idx != 0 {
                        continue;
                    }
                    if let CombatEventKind::Attack(attack) = &event.kind {
                        if first_ranged.is_none() {
                            if distance_before > 1.0 {
                                first_ranged = Some(attack.is_ranged);
                            }
                        } else if distance_before <= 1.0 && melee_after_close.is_none() {
                            melee_after_close = Some(attack.is_ranged);
                        }
                    }
                }
                seen_events = sim.combat_events.len();
            }

            if first_ranged == Some(true) && melee_after_close == Some(false) {
                break;
            }
        }

        assert_eq!(first_ranged, Some(true));
        assert_eq!(melee_after_close, Some(false));
    }

    #[test]
    fn penetrating_roll_subtracts_one_on_extra_rolls() {
        let mut rolls = vec![6, 2].into_iter();
        let total = penetrating_roll_with(6, || rolls.next().unwrap_or(1));
        assert_eq!(total, 7);
    }

    #[test]
    fn penetrating_roll_can_chain_with_minus_one_each_time() {
        let mut rolls = vec![6, 6, 3].into_iter();
        let total = penetrating_roll_with(6, || rolls.next().unwrap_or(1));
        assert_eq!(total, 13);
    }

    #[test]
    fn one_handed_weapon_does_not_grant_defense_bonus() {
        let mut defender = combatant_basic(
            "Defender".to_string(),
            "Short Sword".to_string(),
            0,
            0,
            0,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            false,
            None,
            true,
            false,
            20,
        );
        defender
            .state
            .refresh_defense_plus_four_ready(&defender.sheet, 0.0);
        assert!(!defender.state.defense_plus_four_ready);
    }

    #[test]
    fn defense_always_applies_without_two_hand_grip() {
        let attacker = combatant_basic(
            "Attacker".to_string(),
            "Test Blade".to_string(),
            0,
            0,
            0,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            false,
            None,
            true,
            false,
            20,
        );
        let defender = combatant_basic(
            "Defender".to_string(),
            "Polehammer".to_string(),
            0,
            0,
            0,
            false,
            0,
            "1d1".to_string(),
            0,
            10.0,
            1.0,
            5.0,
            false,
            false,
            None,
            true,
            true,
            20,
        );
        let mut state = make_state(attacker, defender);
        let mut rng = FixedRng(0);
        let _ = resolve_attack(
            &mut state.combatants,
            0,
            1,
            0,
            false,
            AttackMode::Normal,
            0.0,
            None,
            &mut rng,
        );
        assert_eq!(state.combatants[1].state.hp, 20);

        let mut rng = FixedRng(0);
        let _ = resolve_attack(
            &mut state.combatants,
            0,
            1,
            0,
            false,
            AttackMode::Normal,
            0.0,
            None,
            &mut rng,
        );
        assert_eq!(state.combatants[1].state.hp, 20);
    }
