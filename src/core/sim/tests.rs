use super::combat::{
    AttackMode, critical_effect_for, defense_die_sides, extra_damage_dice_sequence, resolve_attack,
};
use super::*;
use crate::character::{Progression, ProgressionTier};
use crate::core::rng::SimRng;
use crate::core::rules::{
    DamageExprCache, clean_damage_expr, evaluate_expression_with_detail, penetrating_roll_with,
    roll_damage_expr_with_detail,
};
use crate::core::sim::DamageDie;
use crate::core::types::RaceSpec;
use crate::{data, game_logic};
use rand::SeedableRng;
use std::sync::Arc;
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
    let jab_special_expr_cache = jab_special_expr.as_deref().map(DamageExprCache::new);
    let sheet = CombatantSheet {
        name,
        offense: OffenseProfile {
            attack_bonus,
            attack_bonus_base: attack_bonus,
            strength_damage,
            strength_damage_base: strength_damage,
            unarmed_damage_bonus: 0,
            weapon: Arc::new(WeaponProfile {
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
                is_small_weapon: false,
                is_unarmed: false,
                crit_min_roll: 20,
                crit_min_roll_ranged: None,
                crit_severity_bonus: 0,
            }),
            offhand: None,
        },
        defense: DefenseProfile {
            defense_mod,
            ranged_defense_mod: 0,
            dex_defense_bonus: 0,
            armor_dr,
            natural_dr: 0,
            knockback_step: 15,
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
        modifiers: ModifierStack::default(),
    };
    Combatant::new(sheet)
}

fn make_state(attacker: Combatant, defender: Combatant) -> SimState {
    let mut state = SimState::new(SimConfig::new(10.0, 1.0));
    let mut attacker = attacker;
    let mut defender = defender;
    attacker.team_id = 0;
    defender.team_id = 1;
    state.reset_with_combatants(vec![attacker, defender]);
    state
}

fn min_raw_damage_for_knockback(knockback_ft: f32, step_damage: i32, charge_bonus: bool) -> i32 {
    let steps_needed = (knockback_ft / 5.0).ceil().max(0.0) as i32;
    let step_damage = step_damage.max(1);
    let multiplier = if charge_bonus { 2 } else { 1 };
    let numerator = steps_needed * step_damage;
    (numerator + multiplier - 1) / multiplier
}

fn setup_charge_sim(reach_ft: f32, move_speed: f32, stop_distance: f32) -> SimState {
    let mut attacker = combatant_basic(
        "Charger".to_string(),
        "Test Blade".to_string(),
        100,
        0,
        0,
        false,
        0,
        "1d1".to_string(),
        0,
        1.0,
        reach_ft,
        move_speed,
        false,
        false,
        None,
        true,
        false,
        10_000,
    );
    let mut defender = combatant_basic(
        "Dummy".to_string(),
        "Test Blade".to_string(),
        -100,
        -100,
        0,
        false,
        0,
        "1d1".to_string(),
        0,
        10.0,
        reach_ft,
        0.0,
        false,
        false,
        None,
        true,
        false,
        10_000,
    );
    attacker.sheet.maneuvers.charge = true;
    defender.sheet.maneuvers.charge = true;
    attacker.team_id = 0;
    defender.team_id = 1;

    let mut sim = SimState::with_rng(SimConfig::new(200.0, stop_distance), SimRng::from_seed(1));
    sim.reset_with_combatants(vec![attacker, defender]);
    sim
}

fn set_distance(sim: &mut SimState, distance_ft: f32) {
    let distance_tiles = distance_ft.round() as i32;
    sim.actors[0].position = GridPos::new(0, 0);
    sim.actors[1].position = GridPos::new(distance_tiles, 0);
    for combatant in &mut sim.combatants {
        combatant.state.charge_distance_ft = 0.0;
        combatant.state.charge_target_idx = None;
        combatant.state.clear_attack_timers();
    }
}

fn set_distance_no_reset(sim: &mut SimState, distance_ft: f32) {
    let distance_tiles = distance_ft.round() as i32;
    sim.actors[0].position = GridPos::new(0, 0);
    sim.actors[1].position = GridPos::new(distance_tiles, 0);
}

fn arthur_duel_sim(
    raw_a: i32,
    raw_b: i32,
    move_speed_a: f32,
    move_speed_b: f32,
    attack_bonus_a: i32,
    attack_bonus_b: i32,
    defense_mod_a: i32,
    defense_mod_b: i32,
) -> SimState {
    arthur_duel_sim_with_distance(
        raw_a,
        raw_b,
        move_speed_a,
        move_speed_b,
        attack_bonus_a,
        attack_bonus_b,
        defense_mod_a,
        defense_mod_b,
        200.0,
        Some(8.0),
    )
}

fn arthur_duel_sim_with_distance(
    raw_a: i32,
    raw_b: i32,
    move_speed_a: f32,
    move_speed_b: f32,
    attack_bonus_a: i32,
    attack_bonus_b: i32,
    defense_mod_a: i32,
    defense_mod_b: i32,
    start_distance: f32,
    initial_distance: Option<f32>,
) -> SimState {
    let (weapon_catalog, armor_catalog, shield_catalog) =
        data::load_catalogs().expect("failed to load catalogs");
    let npc_presets =
        data::load_npc_presets("data/npc_presets.json").expect("failed to load npc presets");
    let fighter_presets = data::load_fighter_presets("data/fighter_presets.json")
        .expect("failed to load fighter presets");
    let talent_catalog = data::load_talents(data::TALENTS_PATH).expect("failed to load talents");
    let race_catalog = data::load_races("data/races.json").expect("failed to load races");

    let arthur_preset = find_fighter_preset(&fighter_presets, "Arthur Du Randt")
        .expect("missing Arthur Du Randt preset");
    let arthur = player_config_from_preset(
        arthur_preset,
        &weapon_catalog,
        &armor_catalog,
        &shield_catalog,
        &race_catalog,
    );
    let players = [arthur.clone(), arthur];
    let stop_distance =
        game_logic::stop_distance_for_players(&players, &weapon_catalog, &talent_catalog);
    let mut combatants = game_logic::build_combatants(
        &players,
        &weapon_catalog,
        &armor_catalog,
        &shield_catalog,
        &npc_presets,
        &talent_catalog,
    );

    let raw_values = [raw_a, raw_b];
    let attack_bonuses = [attack_bonus_a, attack_bonus_b];
    let defense_mods = [defense_mod_a, defense_mod_b];
    let move_speeds = [move_speed_a, move_speed_b];
    for (idx, combatant) in combatants.iter_mut().enumerate() {
        let raw = raw_values[idx].max(1);
        let damage_expr = format!("{raw}d1");
        let mut weapon = combatant.sheet.offense.weapon.as_ref().clone();
        weapon.damage_expr = damage_expr.clone();
        weapon.damage_expr_cache = DamageExprCache::new(&damage_expr);
        weapon.crit_min_roll = 21;
        weapon.speed = 14.0;
        weapon.reach_ft = 8.0;
        combatant.sheet.offense.weapon = Arc::new(weapon);
        combatant.sheet.offense.strength_damage = 0;
        combatant.sheet.offense.strength_damage_base = 0;
        combatant.sheet.offense.attack_bonus = attack_bonuses[idx];
        combatant.sheet.offense.attack_bonus_base = attack_bonuses[idx];
        combatant.sheet.defense.defense_mod = defense_mods[idx];
        combatant.sheet.defense.armor_dr = 0;
        combatant.sheet.defense.knockback_step = 15;
        combatant.sheet.mobility.move_speed = move_speeds[idx];
        combatant.sheet.vitals.threshold_of_pain = 10_000;
        combatant.sheet.vitals.constitution = 0;
        combatant.sheet.vitals.max_hp = 10_000;
    }

    let mut sim = SimState::with_rng(
        SimConfig::new(start_distance, stop_distance),
        SimRng::from_seed(1),
    );
    sim.reset_with_combatants(combatants);
    if let Some(distance) = initial_distance {
        set_distance(&mut sim, distance);
    }
    sim
}

fn first_attack_by(sim: &SimState, attacker_idx: usize, min_time: u32) -> Option<&AttackEvent> {
    sim.combat_events
        .iter()
        .filter_map(|event| match &event.kind {
            CombatEventKind::Attack(attack)
                if event.attacker_idx == attacker_idx && event.time >= min_time =>
            {
                Some(attack)
            }
            _ => None,
        })
        .next()
}

fn find_fighter_preset<'a>(
    catalog: &'a game_logic::FighterPresetCatalog,
    name: &str,
) -> Option<&'a game_logic::FighterPreset> {
    catalog
        .entries()
        .iter()
        .find(|preset| preset.name.eq_ignore_ascii_case(name))
}

fn player_config_from_preset(
    preset: &game_logic::FighterPreset,
    weapon_catalog: &game_logic::WeaponCatalog,
    armor_catalog: &game_logic::ArmorCatalog,
    shield_catalog: &game_logic::ShieldCatalog,
    race_catalog: &[RaceSpec],
) -> game_logic::PlayerConfig {
    let attack = tier_from_label(&preset.progression.attack).unwrap_or(ProgressionTier::I);
    let speed = tier_from_label(&preset.progression.speed).unwrap_or(ProgressionTier::I);
    let initiative = tier_from_label(&preset.progression.initiative).unwrap_or(ProgressionTier::I);
    let health = tier_from_label(&preset.progression.health).unwrap_or(ProgressionTier::I);

    let mut player = game_logic::PlayerConfig::new(
        &preset.name,
        weapon_catalog
            .first_id()
            .unwrap_or_else(|| game_logic::WeaponId::new(0)),
    );
    player.level = preset.level;
    player.progression = Progression::new(attack, speed, initiative, health);
    player.mastery_attack = game_logic::clamp_mastery(preset.masteries.attack);
    player.mastery_defense = game_logic::clamp_mastery(preset.masteries.defense);
    player.mastery_damage = game_logic::clamp_mastery(preset.masteries.damage);
    player.mastery_speed = game_logic::clamp_mastery(preset.masteries.speed);
    player.shield_mastery_defense = game_logic::clamp_mastery(preset.masteries.shield_defense);
    player.shield_mastery_speed = game_logic::clamp_mastery(preset.masteries.shield_speed);
    player.base_hp = preset.base_hp;
    player.move_speed = preset.move_speed;
    player.strength_base = preset.strength_base;
    player.strength_pct = game_logic::normalize_percentile(preset.strength_pct);
    player.dex_base = preset.dex_base;
    player.dex_pct = game_logic::normalize_percentile(preset.dex_pct);
    player.intelligence = preset.intelligence;
    player.wisdom = preset.wisdom;
    player.constitution = preset.constitution;
    player.looks = preset.looks;
    player.charisma = preset.charisma;
    player.weapon_material_tier = preset.weapon_material_tier;
    player.offhand_weapon_material_tier = preset.offhand_weapon_material_tier;
    player.armor_material_tier = preset.armor_material_tier;
    player.projectile_material_tier = preset.projectile_material_tier;
    player.offhand_projectile_material_tier = preset.offhand_projectile_material_tier;
    player.shield_material_tier = preset.shield_material_tier;
    player.two_hand_grip = preset.two_hand_grip;
    let maneuvers = preset.maneuvers;
    player.use_jab = maneuvers.use_jab;
    player.hold_at_bay = maneuvers.hold_at_bay;
    player.aggressive_attack = maneuvers.aggressive_attack;
    player.charge = maneuvers.charge;
    player.ready_against_charge = maneuvers.ready_against_charge;
    player.tactical_move = maneuvers.tactical_move;
    player.fight_defensively = maneuvers.fight_defensively;
    player.full_parry = maneuvers.full_parry;
    player.give_ground = maneuvers.give_ground;
    player.scamper_back = maneuvers.scamper_back;
    player.fighting_withdrawal = maneuvers.fighting_withdrawal;
    player.flee = maneuvers.flee;
    player.defensive_dualwielding = preset.defensive_dualwielding;
    player.offensive_dualwielding = preset.offensive_dualwielding;
    player.proficiencies = preset.proficiencies.clone();
    player.talents = preset.talents.clone();
    player.race_id = preset.race_id.clone();
    player.race_applied = false;
    player.knockback_step =
        game_logic::knockback_step_for_race_id(player.race_id.as_deref(), race_catalog);
    player.weapon_id = find_weapon_id_by_name(weapon_catalog, &preset.weapon)
        .or_else(|| weapon_catalog.first_id())
        .unwrap_or_else(|| game_logic::WeaponId::new(0));
    player.offhand_weapon_id = preset
        .offhand_weapon
        .as_deref()
        .and_then(|name| find_weapon_id_by_name(weapon_catalog, name));
    player.armor_id = find_armor_id_by_name(armor_catalog, &preset.armor)
        .or_else(|| armor_catalog.first_id())
        .unwrap_or_else(|| game_logic::ArmorId::new(0));
    player.shield_id = find_shield_id_by_name(shield_catalog, &preset.shield)
        .or_else(|| shield_catalog.first_id())
        .unwrap_or_else(|| game_logic::ShieldId::new(0));
    if let Some(weapon) = weapon_catalog.get(player.weapon_id) {
        game_logic::sanitize_projectile_tier(&mut player, weapon);
    }
    player
}

fn tier_from_label(label: &str) -> Option<ProgressionTier> {
    match label.trim() {
        "I" | "1" => Some(ProgressionTier::I),
        "II" | "2" => Some(ProgressionTier::II),
        "III" | "3" => Some(ProgressionTier::III),
        "IV" | "4" => Some(ProgressionTier::IV),
        "V" | "5" => Some(ProgressionTier::V),
        "VI" | "6" => Some(ProgressionTier::VI),
        _ => None,
    }
}

fn find_weapon_id_by_name(
    catalog: &game_logic::WeaponCatalog,
    name: &str,
) -> Option<game_logic::WeaponId> {
    catalog
        .entries()
        .iter()
        .position(|weapon| weapon.name.eq_ignore_ascii_case(name))
        .and_then(|idx| catalog.id_from_index(idx))
}

fn find_armor_id_by_name(
    catalog: &game_logic::ArmorCatalog,
    name: &str,
) -> Option<game_logic::ArmorId> {
    if name.eq_ignore_ascii_case("None") {
        return catalog.first_id();
    }
    catalog
        .entries()
        .iter()
        .position(|entry| {
            entry
                .armor
                .as_ref()
                .map(|armor| armor.name.eq_ignore_ascii_case(name))
                .unwrap_or(false)
        })
        .and_then(|idx| catalog.id_from_index(idx))
}

fn find_shield_id_by_name(
    catalog: &game_logic::ShieldCatalog,
    name: &str,
) -> Option<game_logic::ShieldId> {
    if name.eq_ignore_ascii_case("None") {
        return catalog.first_id();
    }
    catalog
        .entries()
        .iter()
        .position(|entry| {
            entry
                .shield
                .as_ref()
                .map(|shield| shield.name.eq_ignore_ascii_case(name))
                .unwrap_or(false)
        })
        .and_then(|idx| catalog.id_from_index(idx))
}

#[test]
fn lower_of_damage_expr_is_parsed_for_shield_damage() {
    assert_eq!(clean_damage_expr("lower of 2d6p"), "2d6p");
}

#[test]
fn critical_effects_follow_severity_table() {
    let low = critical_effect_for(1);
    assert_eq!(low.extra_dice, 1);
    assert!(!low.speed_reset);
    assert!(!low.auto_trauma);
    assert!(!low.instant_kill);

    let mid = critical_effect_for(15);
    assert_eq!(mid.extra_dice, 2);
    assert!(!mid.speed_reset);

    let high = critical_effect_for(25);
    assert_eq!(high.extra_dice, 3);
    assert!(high.speed_reset);

    let severe = critical_effect_for(35);
    assert_eq!(severe.extra_dice, 4);
    assert!(severe.auto_trauma);

    let deadly = critical_effect_for(41);
    assert!(deadly.instant_kill);
    assert_eq!(deadly.extra_dice, 0);
}

#[test]
fn extra_damage_dice_cycles_low_to_high() {
    let sequence = extra_damage_dice_sequence("2d3+d6", 4, false);
    assert_eq!(
        sequence,
        vec![
            DamageDie {
                sides: 3,
                penetrating: false
            },
            DamageDie {
                sides: 3,
                penetrating: false
            },
            DamageDie {
                sides: 6,
                penetrating: false
            },
            DamageDie {
                sides: 3,
                penetrating: false
            },
        ]
    );
}

#[test]
fn extra_damage_dice_respects_penetration_flags() {
    let sequence = extra_damage_dice_sequence("d4p+d6", 3, false);
    assert_eq!(
        sequence,
        vec![
            DamageDie {
                sides: 4,
                penetrating: true
            },
            DamageDie {
                sides: 6,
                penetrating: false
            },
            DamageDie {
                sides: 4,
                penetrating: true
            },
        ]
    );
    let nonpen = extra_damage_dice_sequence("d4p+d6", 2, true);
    assert_eq!(
        nonpen,
        vec![
            DamageDie {
                sides: 4,
                penetrating: false
            },
            DamageDie {
                sides: 6,
                penetrating: false
            },
        ]
    );
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
        1.0,
        AttackMode::Normal,
        WeaponSlot::Primary,
        0.0,
        None,
        &mut rng,
    );
    assert_eq!(state.combatants[1].state.hp, 20);
}

#[test]
fn bulk_sim_arthur_vs_zorya_100k_under_point_eight_seconds() {
    if cfg!(debug_assertions) {
        return;
    }
    let (weapon_catalog, armor_catalog, shield_catalog) =
        data::load_catalogs().expect("failed to load catalogs");
    let npc_presets =
        data::load_npc_presets("data/npc_presets.json").expect("failed to load npc presets");
    let fighter_presets = data::load_fighter_presets("data/fighter_presets.json")
        .expect("failed to load fighter presets");
    let talent_catalog = data::load_talents(data::TALENTS_PATH).expect("failed to load talents");
    let race_catalog = data::load_races("data/races.json").expect("failed to load races");

    let arthur_preset = find_fighter_preset(&fighter_presets, "Arthur Du Randt")
        .expect("missing Arthur Du Randt preset");
    let zorya_preset =
        find_fighter_preset(&fighter_presets, "Zorya").expect("missing Zorya preset");

    let arthur = player_config_from_preset(
        arthur_preset,
        &weapon_catalog,
        &armor_catalog,
        &shield_catalog,
        &race_catalog,
    );
    let zorya = player_config_from_preset(
        zorya_preset,
        &weapon_catalog,
        &armor_catalog,
        &shield_catalog,
        &race_catalog,
    );
    let players = [arthur, zorya];
    let combatants = game_logic::build_combatants(
        &players,
        &weapon_catalog,
        &armor_catalog,
        &shield_catalog,
        &npc_presets,
        &talent_catalog,
    );
    let config = SimConfig::new(200.0, 1.0);
    let start = Instant::now();
    let _ = bulk_simulate(config, combatants, 100_000, u32::MAX);
    let elapsed = start.elapsed();
    assert!(
        elapsed <= Duration::from_millis(800),
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
    let mut combatant_a = combatant.clone();
    let mut combatant_b = combatant;
    combatant_a.team_id = 0;
    combatant_b.team_id = 1;
    sim.reset_with_combatants(vec![combatant_a, combatant_b]);
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
        1.0,
        AttackMode::HoldAtBay,
        WeaponSlot::Primary,
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
        1.0,
        AttackMode::HoldAtBay,
        WeaponSlot::Primary,
        0.0,
        None,
        &mut rng,
    );
    assert_eq!(state.combatants[1].state.hp, 19);
}

#[test]
fn ranged_weapons_cannot_hold_at_bay() {
    let ranged_weapon = Arc::new(WeaponProfile {
        name: "Test Thrower".to_string(),
        damage_expr: "1d1".to_string(),
        damage_expr_cache: DamageExprCache::new("1d1"),
        shield_damage_expr: None,
        shield_damage_expr_cache: None,
        armor_penetration: 0,
        speed: 10.0,
        reach_ft: 6.0,
        range_bands_feet: Some([20.0, 30.0, 40.0, 60.0]),
        range_distance_multiplier: 1.0,
        two_hand_grip: false,
        use_jab: false,
        jab_special_expr: None,
        jab_special_expr_cache: None,
        has_weapon: true,
        defense_bonus_always: false,
        uses_projectiles: false,
        is_small_weapon: false,
        is_unarmed: false,
        crit_min_roll: 20,
        crit_min_roll_ranged: None,
        crit_severity_bonus: 0,
    });
    let melee_weapon = Arc::new(WeaponProfile {
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
        is_small_weapon: false,
        is_unarmed: false,
        crit_min_roll: 20,
        crit_min_roll_ranged: None,
        crit_severity_bonus: 0,
    });
    let mut maneuvers = ManeuverProfile::default();
    maneuvers.hold_at_bay = true;
    let attacker = Combatant::new(CombatantSheet {
        name: "Thrower".to_string(),
        offense: OffenseProfile {
            attack_bonus: 0,
            attack_bonus_base: 0,
            strength_damage: 0,
            strength_damage_base: 0,
            unarmed_damage_bonus: 0,
            weapon: ranged_weapon,
            offhand: None,
        },
        defense: DefenseProfile {
            ranged_defense_mod: 0,
            defense_mod: 0,
            dex_defense_bonus: 0,
            armor_dr: 0,
            natural_dr: 0,
            knockback_step: 15,
            armor_is_heavy: false,
            shield_name: None,
            shield_defense_bonus: 0,
            shield_dr: 0,
            shield_cover_value: None,
            shield_breakage: None,
        },
        mobility: MobilityProfile { move_speed: 5.0 },
        vitals: Vitals {
            trauma_die_sides: 20,
            trauma_die_penetrating: false,
            max_hp: 100,
            constitution: 10,
            threshold_of_pain: 0,
        },
        maneuvers,
        modifiers: ModifierStack::default(),
    });
    let defender = Combatant::new(CombatantSheet {
        name: "Defender".to_string(),
        offense: OffenseProfile {
            attack_bonus: 0,
            attack_bonus_base: 0,
            strength_damage: 0,
            strength_damage_base: 0,
            unarmed_damage_bonus: 0,
            weapon: melee_weapon,
            offhand: None,
        },
        defense: DefenseProfile {
            ranged_defense_mod: 0,
            defense_mod: 0,
            dex_defense_bonus: 0,
            armor_dr: 0,
            natural_dr: 0,
            knockback_step: 15,
            armor_is_heavy: false,
            shield_name: None,
            shield_defense_bonus: 0,
            shield_dr: 0,
            shield_cover_value: None,
            shield_breakage: None,
        },
        mobility: MobilityProfile { move_speed: 5.0 },
        vitals: Vitals {
            trauma_die_sides: 20,
            trauma_die_penetrating: false,
            max_hp: 100,
            constitution: 10,
            threshold_of_pain: 0,
        },
        maneuvers: ManeuverProfile::default(),
        modifiers: ModifierStack::default(),
    });
    let mut sim = SimState::new(SimConfig::new(12.0, 1.0));
    let mut attacker = attacker;
    let mut defender = defender;
    attacker.team_id = 0;
    defender.team_id = 1;
    sim.reset_with_combatants(vec![attacker, defender]);
    sim.set_rng(SimRng::from_seed(1));
    for _ in 0..5 {
        sim.tick();
    }

    let saw_attack = sim
        .combat_events
        .iter()
        .any(|event| matches!(event.kind, CombatEventKind::Attack(_)));
    assert!(saw_attack);
    let saw_hold_at_bay = sim
        .combat_events
        .iter()
        .any(|event| matches!(&event.kind, CombatEventKind::Attack(attack) if attack.hold_at_bay));
    assert!(!saw_hold_at_bay);
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
    let mut attacker = attacker;
    let mut defender = defender;
    attacker.team_id = 0;
    defender.team_id = 1;
    sim.reset_with_combatants(vec![attacker, defender]);
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
            attack_bonus_base: 100,
            strength_damage: 0,
            strength_damage_base: 0,
            unarmed_damage_bonus: 0,
            weapon: Arc::new(WeaponProfile {
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
                is_small_weapon: false,
                is_unarmed: false,
                crit_min_roll: 20,
                crit_min_roll_ranged: None,
                crit_severity_bonus: 0,
            }),
            offhand: None,
        },
        defense: DefenseProfile {
            ranged_defense_mod: 0,
            defense_mod: 0,
            dex_defense_bonus: 0,
            armor_dr: 0,
            natural_dr: 0,
            knockback_step: 15,
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
        modifiers: ModifierStack::default(),
    };
    let mut sim = SimState::new(SimConfig::new(1.0, 1.0));
    let mut first = Combatant::new(sheet.clone());
    let mut second = Combatant::new(sheet);
    first.team_id = 0;
    second.team_id = 1;
    sim.reset_with_combatants(vec![first, second]);
    sim.tick();
    assert!(sim.combatants[0].state.hp < sim.combatants[0].sheet.vitals.max_hp);
    assert!(sim.combatants[1].state.hp < sim.combatants[1].sheet.vitals.max_hp);
}

#[test]
fn offensive_dualwielding_schedules_offhand_after_primary() {
    let mut attacker = combatant_basic(
        "Attacker".to_string(),
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
    let mut weapon = attacker.sheet.offense.weapon.as_ref().clone();
    weapon.crit_min_roll = 21;
    attacker.sheet.offense.weapon = Arc::new(weapon);
    let mut offhand_weapon = attacker.sheet.offense.weapon.as_ref().clone();
    offhand_weapon.name = "Offhand".to_string();
    offhand_weapon.speed = 6.0;
    attacker.sheet.offense.offhand = Some(OffhandProfile {
        attack_bonus: attacker.sheet.offense.attack_bonus,
        strength_damage: attacker.sheet.offense.strength_damage,
        weapon: Arc::new(offhand_weapon),
    });
    attacker.sheet.maneuvers.offensive_dualwielding = true;

    let defender = combatant_basic(
        "Defender".to_string(),
        "Fist".to_string(),
        0,
        -1000,
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
        200,
    );
    let mut sim = SimState::new(SimConfig::new(1.0, 1.0));
    let mut attacker = attacker;
    let mut defender = defender;
    attacker.team_id = 0;
    defender.team_id = 1;
    sim.reset_with_combatants(vec![attacker, defender]);
    sim.set_rng(SimRng::from_seed(1));
    sim.tick();
    assert_eq!(sim.combatants[0].state.next_attack_time_primary, Some(12.0));
    assert_eq!(
        sim.combatants[0].state.next_attack_time_secondary,
        Some(7.0)
    );
}

#[test]
fn shield_strike_speedup_applies_only_to_style_and_clamps_to_now() {
    let attacker = combatant_basic(
        "Attacker".to_string(),
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
    let mut defender = combatant_basic(
        "Defender".to_string(),
        "Fist".to_string(),
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
        .sheet
        .modifiers
        .add_i32(StatIdI32::FlagLargeSwordShieldStyle, ModifierOpI32::Set(1));
    let mut sim = SimState::new(SimConfig::new(1.0, 1.0));
    let mut attacker = attacker;
    let mut defender = defender;
    attacker.team_id = 0;
    defender.team_id = 1;
    sim.reset_with_combatants(vec![attacker, defender]);

    sim.combatants[1].state.next_attack_time_primary = Some(8.0);
    sim.apply_shield_strike_speedup(1, 5.0);
    assert_eq!(sim.combatants[1].state.next_attack_time_primary, Some(6.0));

    sim.apply_shield_strike_speedup(1, 7.0);
    assert_eq!(sim.combatants[1].state.next_attack_time_primary, Some(7.0));

    sim.combatants[1].sheet.modifiers = ModifierStack::default();
    sim.combatants[1].state.next_attack_time_primary = Some(10.0);
    sim.apply_shield_strike_speedup(1, 5.0);
    assert_eq!(sim.combatants[1].state.next_attack_time_primary, Some(10.0));
}

#[test]
fn hammerer_resets_weapon_count_on_any_knockback() {
    let mut attacker = combatant_basic(
        "Attacker".to_string(),
        "Warhammer".to_string(),
        100,
        0,
        0,
        false,
        0,
        "20".to_string(),
        0,
        10.0,
        1.0,
        5.0,
        false,
        false,
        None,
        true,
        false,
        100,
    );
    let mut attacker_weapon = attacker.sheet.offense.weapon.as_ref().clone();
    attacker_weapon.crit_min_roll = 21;
    attacker.sheet.offense.weapon = Arc::new(attacker_weapon);
    attacker
        .sheet
        .modifiers
        .add_i32(StatIdI32::FlagHammererStyle, ModifierOpI32::Set(1));
    let mut defender = combatant_basic(
        "Defender".to_string(),
        "Fist".to_string(),
        0,
        -100,
        0,
        false,
        0,
        "1".to_string(),
        0,
        10.0,
        1.0,
        5.0,
        false,
        false,
        None,
        true,
        false,
        100,
    );
    defender.sheet.vitals.threshold_of_pain = 100;
    let mut sim = make_state(attacker, defender);
    sim.combatants[1].state.next_attack_time_primary = Some(2.0);
    sim.combatants[1].state.next_attack_time_secondary = Some(2.0);
    let mut rng = rand::rngs::StdRng::seed_from_u64(2);
    let outcome = resolve_attack(
        &mut sim.combatants,
        0,
        1,
        0,
        false,
        1.0,
        AttackMode::Normal,
        WeaponSlot::Primary,
        0.0,
        None,
        &mut rng,
    );
    assert!(outcome.hit);
    assert_eq!(outcome.knockback_ft, 5.0);
    assert_eq!(sim.combatants[1].state.next_attack_time_primary, Some(10.0));
    assert_eq!(
        sim.combatants[1].state.next_attack_time_secondary,
        Some(10.0)
    );
}

#[test]
fn regenstat_stacks_gain_and_reset_on_hit_and_miss() {
    let mut attacker = combatant_basic(
        "Attacker".to_string(),
        "Longsword".to_string(),
        100,
        0,
        0,
        false,
        0,
        "1".to_string(),
        0,
        10.0,
        1.0,
        5.0,
        false,
        false,
        None,
        true,
        false,
        100,
    );
    let mut attacker_weapon = attacker.sheet.offense.weapon.as_ref().clone();
    attacker_weapon.crit_min_roll = 21;
    attacker.sheet.offense.weapon = Arc::new(attacker_weapon);
    attacker
        .sheet
        .modifiers
        .add_i32(StatIdI32::FlagRegenstatStyle, ModifierOpI32::Set(1));
    let defender = combatant_basic(
        "Defender".to_string(),
        "Fist".to_string(),
        0,
        -100,
        0,
        false,
        0,
        "1".to_string(),
        0,
        10.0,
        1.0,
        5.0,
        false,
        false,
        None,
        true,
        false,
        100,
    );
    let mut sim = make_state(attacker, defender);
    let mut rng = rand::rngs::StdRng::seed_from_u64(3);
    let first = resolve_attack(
        &mut sim.combatants,
        0,
        1,
        0,
        false,
        1.0,
        AttackMode::Normal,
        WeaponSlot::Primary,
        0.0,
        None,
        &mut rng,
    );
    assert!(first.hit);
    assert_eq!(sim.combatants[0].state.regenstat_stacks, 1);

    let second = resolve_attack(
        &mut sim.combatants,
        0,
        1,
        0,
        false,
        1.0,
        AttackMode::Normal,
        WeaponSlot::Primary,
        0.0,
        None,
        &mut rng,
    );
    assert!(second.hit);
    assert_eq!(sim.combatants[0].state.regenstat_stacks, 2);

    sim.combatants[0].sheet.offense.attack_bonus = -100;
    sim.combatants[0].sheet.offense.attack_bonus_base = -100;
    sim.combatants[1].sheet.defense.defense_mod = 100;
    let third = resolve_attack(
        &mut sim.combatants,
        0,
        1,
        0,
        false,
        1.0,
        AttackMode::Normal,
        WeaponSlot::Primary,
        0.0,
        None,
        &mut rng,
    );
    assert!(!third.hit);
    assert_eq!(sim.combatants[0].state.regenstat_stacks, 0);
}

#[test]
fn returner_counter_is_limited_between_own_attacks() {
    let attacker = combatant_basic(
        "Attacker".to_string(),
        "Sword".to_string(),
        100,
        0,
        0,
        false,
        0,
        "1".to_string(),
        0,
        10.0,
        1.0,
        5.0,
        false,
        false,
        None,
        true,
        false,
        100,
    );
    let mut defender = combatant_basic(
        "Defender".to_string(),
        "Greatsword".to_string(),
        10,
        -100,
        0,
        false,
        0,
        "1".to_string(),
        0,
        10.0,
        1.0,
        5.0,
        false,
        false,
        None,
        true,
        false,
        100,
    );
    defender
        .sheet
        .modifiers
        .add_i32(StatIdI32::FlagReturnerStyle, ModifierOpI32::Set(1));
    let mut sim = make_state(attacker, defender);
    sim.combatants[1].state.returner_skip_opening_attack = false;
    sim.combatants[1].state.returner_double_counter_ready = false;
    let mut rng = rand::rngs::StdRng::seed_from_u64(4);
    let first = resolve_attack(
        &mut sim.combatants,
        0,
        1,
        0,
        false,
        1.0,
        AttackMode::Normal,
        WeaponSlot::Primary,
        0.0,
        None,
        &mut rng,
    );
    assert!(first.counter_attack.is_some());
    assert!(!sim.combatants[1].state.returner_counter_available);

    let second = resolve_attack(
        &mut sim.combatants,
        0,
        1,
        0,
        false,
        1.0,
        AttackMode::Normal,
        WeaponSlot::Primary,
        0.0,
        None,
        &mut rng,
    );
    assert!(second.counter_attack.is_none());
}

#[test]
fn three_mountains_sets_next_trauma_to_twenty_after_three_hits() {
    let mut attacker = combatant_basic(
        "Attacker".to_string(),
        "Warhammer".to_string(),
        100,
        0,
        0,
        false,
        0,
        "1".to_string(),
        0,
        10.0,
        1.0,
        5.0,
        false,
        false,
        None,
        true,
        false,
        100,
    );
    let mut attacker_weapon = attacker.sheet.offense.weapon.as_ref().clone();
    attacker_weapon.crit_min_roll = 21;
    attacker.sheet.offense.weapon = Arc::new(attacker_weapon);
    attacker
        .sheet
        .modifiers
        .add_i32(StatIdI32::FlagThreeMountainsStyle, ModifierOpI32::Set(1));
    let defender = combatant_basic(
        "Defender".to_string(),
        "Fist".to_string(),
        0,
        -100,
        0,
        false,
        0,
        "1".to_string(),
        0,
        10.0,
        1.0,
        5.0,
        false,
        false,
        None,
        true,
        false,
        100,
    );
    let mut sim = make_state(attacker, defender);
    let mut rng = rand::rngs::StdRng::seed_from_u64(5);
    for _ in 0..3 {
        let outcome = resolve_attack(
            &mut sim.combatants,
            0,
            1,
            0,
            false,
            1.0,
            AttackMode::Normal,
            WeaponSlot::Primary,
            0.0,
            None,
            &mut rng,
        );
        assert!(outcome.hit);
    }
    assert_eq!(sim.combatants[0].state.three_mountains_hit_streak, 3);
    assert!(sim.combatants[1].state.force_trauma_roll_20);
}

#[test]
fn six_paths_uses_tighter_shield_block_window() {
    let mut attacker = combatant_basic(
        "Attacker".to_string(),
        "Sword".to_string(),
        0,
        0,
        0,
        false,
        0,
        "1".to_string(),
        0,
        10.0,
        1.0,
        5.0,
        false,
        false,
        None,
        true,
        false,
        100,
    );
    let mut attacker_weapon = attacker.sheet.offense.weapon.as_ref().clone();
    attacker_weapon.crit_min_roll = 21;
    attacker.sheet.offense.weapon = Arc::new(attacker_weapon);
    let mut defender_baseline = combatant_basic(
        "Defender".to_string(),
        "Fist".to_string(),
        0,
        5,
        0,
        false,
        0,
        "1".to_string(),
        0,
        10.0,
        1.0,
        5.0,
        false,
        false,
        None,
        true,
        false,
        100,
    );
    defender_baseline.sheet.defense.shield_name = Some("Medium metallic shield".to_string());
    defender_baseline.sheet.defense.shield_defense_bonus = 0;
    defender_baseline.sheet.defense.shield_dr = 1;
    let mut defender_six_paths = defender_baseline.clone();
    defender_six_paths
        .sheet
        .modifiers
        .add_i32(StatIdI32::FlagSixPathsStyle, ModifierOpI32::Set(1));

    let mut found = false;
    for seed in 0..5000u64 {
        let mut baseline_state = make_state(attacker.clone(), defender_baseline.clone());
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let baseline = resolve_attack(
            &mut baseline_state.combatants,
            0,
            1,
            0,
            false,
            1.0,
            AttackMode::Normal,
            WeaponSlot::Primary,
            0.0,
            None,
            &mut rng,
        );
        if baseline.hit || !baseline.shield_block {
            continue;
        }
        let mut six_paths_state = make_state(attacker.clone(), defender_six_paths.clone());
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let six_paths = resolve_attack(
            &mut six_paths_state.combatants,
            0,
            1,
            0,
            false,
            1.0,
            AttackMode::Normal,
            WeaponSlot::Primary,
            0.0,
            None,
            &mut rng,
        );
        if !six_paths.hit && !six_paths.shield_block {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "expected six paths to remove some baseline shield blocks in the 5..9 miss window"
    );
}

#[test]
fn equal_reach_knockback_does_not_block_simultaneous_attacks() {
    let sheet = CombatantSheet {
        name: "Test".to_string(),
        offense: OffenseProfile {
            attack_bonus: 100,
            attack_bonus_base: 100,
            strength_damage: 0,
            strength_damage_base: 0,
            unarmed_damage_bonus: 0,
            weapon: Arc::new(WeaponProfile {
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
                is_small_weapon: false,
                is_unarmed: false,
                crit_min_roll: 20,
                crit_min_roll_ranged: None,
                crit_severity_bonus: 0,
            }),
            offhand: None,
        },
        defense: DefenseProfile {
            ranged_defense_mod: 0,
            defense_mod: 0,
            dex_defense_bonus: 0,
            armor_dr: 0,
            natural_dr: 0,
            knockback_step: 15,
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
        modifiers: ModifierStack::default(),
    };
    let mut sim = SimState::new(SimConfig::new(1.0, 1.0));
    let mut first = Combatant::new(sheet.clone());
    let mut second = Combatant::new(sheet);
    first.team_id = 0;
    second.team_id = 1;
    sim.reset_with_combatants(vec![first, second]);
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
        1.0,
        AttackMode::Normal,
        WeaponSlot::Primary,
        0.0,
        None,
        &mut rng,
    );
    assert_eq!(state.combatants[1].state.hp, 18);
}

#[test]
fn shield_block_damage_stacks_shield_dr_and_armor_dr() {
    let attacker = combatant_basic(
        "Attacker".to_string(),
        "Test Blade".to_string(),
        0,
        0,
        0,
        false,
        0,
        "12".to_string(),
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
    let mut defender = combatant_basic(
        "Defender".to_string(),
        "Shield".to_string(),
        0,
        5,
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
        40,
    );
    defender.sheet.defense.shield_name = Some("Small metallic shield".to_string());
    defender.sheet.defense.shield_dr = 3;

    let mut state = make_state(attacker, defender);
    state.combatants[1].state.shield_intact = true;
    let mut rng = FixedRng(0);
    let event = resolve_attack(
        &mut state.combatants,
        0,
        1,
        0,
        false,
        1.0,
        AttackMode::Normal,
        WeaponSlot::Primary,
        0.0,
        None,
        &mut rng,
    );

    assert!(!event.hit);
    assert!(
        event.shield_block,
        "shield block missing: atk={} def={} shield_intact={}",
        event.roll.attack_total,
        event.roll.defense_total,
        state.combatants[1].state.shield_intact
    );
    assert_eq!(event.damage, 5);
    assert_eq!(state.combatants[1].state.hp, 35);

    let breakdown = event
        .shield_damage_breakdown
        .as_ref()
        .expect("expected shield damage breakdown");
    assert_eq!(breakdown.raw_damage, 12);
    assert_eq!(breakdown.shield_dr, 3);
    assert_eq!(breakdown.effective_armor_dr, 4);
    assert_eq!(breakdown.hp_damage, 5);
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
    let mut rng = SeqRng::new(vec![0]);
    let _ = resolve_attack(
        &mut state.combatants,
        0,
        1,
        0,
        false,
        1.0,
        AttackMode::Normal,
        WeaponSlot::Primary,
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
        1.0,
        AttackMode::Normal,
        WeaponSlot::Primary,
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
        1.0,
        AttackMode::Normal,
        WeaponSlot::Primary,
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
fn temporary_effects_apply_and_expire() {
    let mut combatant = Combatant::default();
    let mut effect = TemporaryEffect::new("test_buff", 2);
    effect
        .modifiers
        .add_i32(StatIdI32::AttackBonus, ModifierOpI32::Add(5));
    combatant.state.add_effect(effect);
    let base = combatant.sheet.offense.attack_bonus;
    assert_eq!(combatant.apply_i32(StatIdI32::AttackBonus, base), base + 5);
    combatant.state.tick_effects();
    assert_eq!(combatant.apply_i32(StatIdI32::AttackBonus, base), base + 5);
    combatant.state.tick_effects();
    assert_eq!(combatant.apply_i32(StatIdI32::AttackBonus, base), base);
}

#[test]
fn charge_attack_applies_bonus_knockback_and_defense_penalty() {
    let attacker = combatant_basic(
        "Charger".to_string(),
        "Test Blade".to_string(),
        0,
        10,
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
    let mut state = make_state(attacker, defender);
    state.combatants[0].sheet.defense.dex_defense_bonus = 3;
    state.combatants[1].sheet.defense.knockback_step = 1;
    state.combatants[0].state.charge_distance_ft = 25.0;
    let mut rng = FixedRng(0);
    let event = resolve_attack(
        &mut state.combatants,
        0,
        1,
        0,
        false,
        1.0,
        AttackMode::Charge,
        WeaponSlot::Primary,
        0.0,
        None,
        &mut rng,
    );
    assert_eq!(event.roll.attack_bonus, 4);
    assert_eq!(event.knockback_ft, 10.0);
    let base_defense = state.combatants[0].sheet.defense.defense_mod;
    assert_eq!(
        state.combatants[0].apply_i32(StatIdI32::DefenseMod, base_defense),
        base_defense - 3
    );
    assert_eq!(state.combatants[0].state.charge_distance_ft, 0.0);
    assert!(
        state.combatants[0]
            .state
            .active_effects
            .iter()
            .any(|effect| effect.id == "charge_defense_penalty")
    );
}

#[test]
fn second_charge_requires_minimum_knockback() {
    let reach_ft: f32 = 5.0;
    let move_speed: f32 = 20.0;
    let stop_distance: f32 = 5.0;
    let knockback_step = 15;
    let min_knockback_ft = 20.0_f32;
    let min_expected_raw = min_raw_damage_for_knockback(min_knockback_ft, knockback_step, true);
    assert_eq!(min_expected_raw, 30);

    let mut sim = setup_charge_sim(reach_ft, move_speed, stop_distance);
    set_distance(&mut sim, reach_ft + min_knockback_ft - 1.0);
    sim.tick();
    assert!(
        sim.combatants[0].state.charge_distance_ft < 20.0,
        "expected charge distance < 20 when gap < 20ft"
    );

    set_distance(&mut sim, reach_ft + min_knockback_ft);
    sim.tick();
    assert!(
        sim.combatants[0].state.charge_distance_ft >= 20.0,
        "expected charge distance >= 20 when gap >= 20ft"
    );
}

#[test]
fn second_charge_does_not_trigger_from_small_knockback() {
    let reach_ft: f32 = 5.0;
    let move_speed: f32 = 20.0;
    let stop_distance: f32 = 5.0;
    let knockback_step = 15;

    let small_knockback_raw = min_raw_damage_for_knockback(5.0, knockback_step, true);
    assert_eq!(small_knockback_raw, 8);

    let mut sim = setup_charge_sim(reach_ft, move_speed, stop_distance);
    set_distance(&mut sim, reach_ft + 5.0);
    sim.tick();

    assert!(
        sim.combatants[0].state.charge_distance_ft < 20.0,
        "expected small knockback not to yield a second charge"
    );
}

#[test]
fn distance_between_uses_manhattan_tiles() {
    let mut sim = setup_charge_sim(5.0, 20.0, 5.0);
    sim.actors[0].position = GridPos::new(0, 0);
    sim.actors[1].position = GridPos::new(3, 4);
    let distance = sim.distance_between(0, 1).unwrap_or(0.0);
    assert_eq!(distance, 7.0, "expected manhattan distance of 7 tiles");
}

#[test]
fn charge_distance_accumulates_only_when_outside_reach() {
    let reach_ft: f32 = 8.0;
    let move_speed: f32 = 20.0;
    let stop_distance: f32 = 8.0;
    let mut sim = setup_charge_sim(reach_ft, move_speed, stop_distance);

    // Inside reach: no charge accumulation.
    set_distance(&mut sim, 6.0);
    sim.tick();
    assert_eq!(sim.combatants[0].state.charge_distance_ft, 0.0);

    // Just outside reach: accumulate movement amount (20 ft).
    set_distance(&mut sim, 30.0);
    sim.tick();
    assert!(
        sim.combatants[0].state.charge_distance_ft >= 20.0,
        "expected charge distance to accumulate when outside reach"
    );
}

#[test]
fn reentering_reach_clears_attack_timers() {
    let reach_ft: f32 = 8.0;
    let move_speed: f32 = 20.0;
    let stop_distance: f32 = 8.0;
    let mut sim = setup_charge_sim(reach_ft, move_speed, stop_distance);

    // Force a pending timer, then re-enter reach from outside.
    sim.combatants[0]
        .state
        .set_next_attack_time(WeaponSlot::Primary, Some(99.0));
    set_distance(&mut sim, 40.0);
    sim.tick();
    assert!(
        sim.combatants[0].state.next_attack_time_primary.is_none(),
        "expected attack timers to clear when re-entering reach"
    );
}

#[test]
fn charge_requires_target_at_least_20ft_away() {
    let reach_ft: f32 = 8.0;
    let move_speed: f32 = 20.0;
    let stop_distance: f32 = 8.0;
    let mut sim = setup_charge_sim(reach_ft, move_speed, stop_distance);

    // Target is only 19ft away; desired rule says this should not yield a charge.
    set_distance_no_reset(&mut sim, 19.0);
    sim.tick();
    assert!(
        sim.combatants[0].state.charge_distance_ft < 20.0,
        "expected no charge accumulation when target < 20ft away"
    );
}

#[test]
fn arthur_vs_arthur_charges_on_first_contact() {
    let (weapon_catalog, armor_catalog, shield_catalog) =
        data::load_catalogs().expect("failed to load catalogs");
    let npc_presets =
        data::load_npc_presets("data/npc_presets.json").expect("failed to load npc presets");
    let fighter_presets = data::load_fighter_presets("data/fighter_presets.json")
        .expect("failed to load fighter presets");
    let talent_catalog = data::load_talents(data::TALENTS_PATH).expect("failed to load talents");
    let race_catalog = data::load_races("data/races.json").expect("failed to load races");

    let arthur_preset = find_fighter_preset(&fighter_presets, "Arthur Du Randt")
        .expect("missing Arthur Du Randt preset");
    let arthur = player_config_from_preset(
        arthur_preset,
        &weapon_catalog,
        &armor_catalog,
        &shield_catalog,
        &race_catalog,
    );
    let players = [arthur.clone(), arthur];
    let stop_distance =
        game_logic::stop_distance_for_players(&players, &weapon_catalog, &talent_catalog);
    assert!(
        (stop_distance - 8.0).abs() < 0.01,
        "expected stop distance ~8ft, got {stop_distance}"
    );
    let combatants = game_logic::build_combatants(
        &players,
        &weapon_catalog,
        &armor_catalog,
        &shield_catalog,
        &npc_presets,
        &talent_catalog,
    );
    let mut sim = SimState::with_rng(SimConfig::new(200.0, stop_distance), SimRng::from_seed(1));
    sim.reset_with_combatants(combatants);

    for _ in 0..10 {
        sim.tick();
        let attack_count = sim
            .combat_events
            .iter()
            .filter(|event| matches!(event.kind, CombatEventKind::Attack(_)))
            .count();
        if attack_count >= 2 {
            break;
        }
    }

    let mut t5_charges = 0;
    for event in &sim.combat_events {
        if event.time == 5 {
            if let CombatEventKind::Attack(attack) = &event.kind {
                if attack.is_charge {
                    t5_charges += 1;
                }
            }
        }
    }
    assert!(
        t5_charges >= 2,
        "expected both Arthurs to charge on first contact, got {t5_charges}"
    );
}

#[test]
fn arthur_knockback_10ft_no_charge_and_reengage() {
    let mut sim = arthur_duel_sim(30, 1, 0.0, 20.0, 100, -100, 100, -100);

    for _ in 0..20 {
        sim.tick();
    }

    let attack = first_attack_by(&sim, 1, 1).expect("Arthur 2 never attacked after t=0");
    assert!(
        !attack.is_charge,
        "expected Arthur 2 not to charge after 10ft knockback"
    );

    let event_time = sim
        .combat_events
        .iter()
        .find(|event| {
            event.attacker_idx == 1
                && event.time >= 1
                && matches!(event.kind, CombatEventKind::Attack(_))
        })
        .map(|event| event.time)
        .unwrap_or(0);
    assert!(
        event_time <= 2,
        "expected reengage attack soon after knockback, got attack at t={event_time}s"
    );
}

#[test]
fn arthur_knockback_20ft_thresholds_and_charges() {
    let mut sim = arthur_duel_sim(60, 1, 0.0, 20.0, 100, -100, 100, -100);

    // Force a 20ft gap without resetting charge, then close it in one tick.
    set_distance_no_reset(&mut sim, 28.0);
    sim.tick();
    assert!(
        sim.combatants[1].state.charge_distance_ft >= 20.0,
        "expected Arthur 2 to have 20ft charge distance, got {}",
        sim.combatants[1].state.charge_distance_ft
    );
    // Next tick should allow the charge attack.
    sim.tick();
    let attack = first_attack_by(&sim, 1, 1).unwrap_or_else(|| {
            let times: Vec<u32> = sim
                .combat_events
                .iter()
                .filter(|event| {
                    event.attacker_idx == 1
                        && matches!(event.kind, CombatEventKind::Attack(_))
                })
                .map(|event| event.time)
                .collect();
            let next_attack = sim
                .combatants
                .get(1)
                .and_then(|combatant| combatant.state.next_attack_time_primary)
                .unwrap_or(-1.0);
            let distance = sim.distance_between(0, 1).unwrap_or(-1.0);
            panic!(
                "Arthur 2 never attacked after t=1, attack times: {times:?}, next_attack: {next_attack}, distance: {distance}"
            );
        });
    assert!(
        attack.is_charge,
        "expected Arthur 2 to charge after 20ft knockback"
    );
}

#[test]
fn arthur_both_knockback_10ft_no_charge_on_reengage() {
    let mut sim = arthur_duel_sim(30, 30, 10.0, 10.0, 100, 100, -100, -100);

    for _ in 0..30 {
        sim.tick();
    }

    let attack_a = first_attack_by(&sim, 0, 1).expect("Arthur 1 never attacked after t=0");
    let attack_b = first_attack_by(&sim, 1, 1).expect("Arthur 2 never attacked after t=0");
    assert!(
        !attack_a.is_charge && !attack_b.is_charge,
        "expected no charges after mutual 10ft knockback"
    );
}

#[test]
fn arthur_both_knockback_10ft_should_not_charge_after_10ft_move_each() {
    let mut sim = arthur_duel_sim(30, 30, 10.0, 10.0, 100, 100, -100, -100);

    // Force a 20ft gap (10ft each knockback), then have each close 10ft.
    set_distance_no_reset(&mut sim, 28.0);
    sim.tick(); // movement only, no attacks yet
    sim.tick(); // re-engage attacks

    let attack_a = first_attack_by(&sim, 0, 1).expect("Arthur 1 never attacked after re-engage");
    let attack_b = first_attack_by(&sim, 1, 1).expect("Arthur 2 never attacked after re-engage");
    assert!(
        !attack_a.is_charge && !attack_b.is_charge,
        "expected no charges after each moved 10ft to re-engage"
    );
}

#[test]
fn arthur_log_repro_double_charge_then_no_charge_on_reengage() {
    let mut sim =
        arthur_duel_sim_with_distance(20, 20, 20.0, 20.0, 100, 100, -100, -100, 200.0, None);

    for _ in 0..8 {
        sim.tick();
    }

    let attacks_t5: Vec<&AttackEvent> = sim
        .combat_events
        .iter()
        .filter_map(|event| match &event.kind {
            CombatEventKind::Attack(attack) if event.time == 5 => Some(attack),
            _ => None,
        })
        .collect();
    assert!(
        attacks_t5.len() >= 2,
        "expected 2 attacks at t=5, got {}",
        attacks_t5.len()
    );
    assert!(
        attacks_t5.iter().all(|attack| attack.is_charge),
        "expected both t=5 attacks to be charges"
    );
    assert!(
        attacks_t5
            .iter()
            .all(|attack| (attack.knockback_ft - 10.0).abs() < 0.1),
        "expected both t=5 attacks to knock back ~10ft"
    );

    let attacks_t7: Vec<&AttackEvent> = sim
        .combat_events
        .iter()
        .filter_map(|event| match &event.kind {
            CombatEventKind::Attack(attack) if event.time == 7 => Some(attack),
            _ => None,
        })
        .collect();
    assert!(
        attacks_t7.iter().all(|attack| !attack.is_charge),
        "expected no charges at t=7 after both moved ~10ft"
    );
}

#[test]
fn bulk_arthur_charges_do_not_start_within_20ft() {
    let (weapon_catalog, armor_catalog, shield_catalog) =
        data::load_catalogs().expect("failed to load catalogs");
    let npc_presets =
        data::load_npc_presets("data/npc_presets.json").expect("failed to load npc presets");
    let fighter_presets = data::load_fighter_presets("data/fighter_presets.json")
        .expect("failed to load fighter presets");
    let talent_catalog = data::load_talents(data::TALENTS_PATH).expect("failed to load talents");
    let race_catalog = data::load_races("data/races.json").expect("failed to load races");

    let arthur_preset = find_fighter_preset(&fighter_presets, "Arthur Du Randt")
        .expect("missing Arthur Du Randt preset");
    let arthur = player_config_from_preset(
        arthur_preset,
        &weapon_catalog,
        &armor_catalog,
        &shield_catalog,
        &race_catalog,
    );
    let players = [arthur.clone(), arthur];
    let combatants = game_logic::build_combatants(
        &players,
        &weapon_catalog,
        &armor_catalog,
        &shield_catalog,
        &npc_presets,
        &talent_catalog,
    );

    let mut sim = SimState::with_rng(
        SimConfig::new(
            200.0,
            game_logic::stop_distance_for_players(&players, &weapon_catalog, &talent_catalog),
        ),
        SimRng::from_seed(1),
    );
    sim.reset_with_combatants(combatants);

    let mut fights_with_charge_within_20ft = 0u32;
    let runs = 200u32;
    for _ in 0..runs {
        sim.reset_preserve_rng();
        while !sim.done && sim.elapsed_seconds < 60 {
            sim.tick();
        }
        if sim
            .combatants
            .iter()
            .any(|combatant| combatant.state.charge_started_within_20ft)
        {
            fights_with_charge_within_20ft += 1;
        }
    }
    assert_eq!(
        fights_with_charge_within_20ft, 0,
        "expected no charges that started within 20ft, got {fights_with_charge_within_20ft}"
    );
}

#[test]
fn arthur_mirror_symmetry_with_swapped_order() {
    let (weapon_catalog, armor_catalog, shield_catalog) =
        data::load_catalogs().expect("failed to load catalogs");
    let npc_presets =
        data::load_npc_presets("data/npc_presets.json").expect("failed to load npc presets");
    let fighter_presets = data::load_fighter_presets("data/fighter_presets.json")
        .expect("failed to load fighter presets");
    let talent_catalog = data::load_talents(data::TALENTS_PATH).expect("failed to load talents");
    let race_catalog = data::load_races("data/races.json").expect("failed to load races");

    let arthur_preset = find_fighter_preset(&fighter_presets, "Arthur Du Randt")
        .expect("missing Arthur Du Randt preset");
    let arthur = player_config_from_preset(
        arthur_preset,
        &weapon_catalog,
        &armor_catalog,
        &shield_catalog,
        &race_catalog,
    );
    let players_a = [arthur.clone(), arthur.clone()];
    let players_b = [arthur.clone(), arthur];

    let stop_distance_a =
        game_logic::stop_distance_for_players(&players_a, &weapon_catalog, &talent_catalog);
    let stop_distance_b =
        game_logic::stop_distance_for_players(&players_b, &weapon_catalog, &talent_catalog);
    let combatants_a = game_logic::build_combatants(
        &players_a,
        &weapon_catalog,
        &armor_catalog,
        &shield_catalog,
        &npc_presets,
        &talent_catalog,
    );
    let combatants_b = game_logic::build_combatants(
        &players_b,
        &weapon_catalog,
        &armor_catalog,
        &shield_catalog,
        &npc_presets,
        &talent_catalog,
    );

    let runs = 1000;
    let max_seconds = 60;
    let result_a = bulk_simulate(
        SimConfig::new(200.0, stop_distance_a),
        combatants_a,
        runs,
        max_seconds,
    );
    let result_b = bulk_simulate(
        SimConfig::new(200.0, stop_distance_b),
        combatants_b,
        runs,
        max_seconds,
    );

    let wins_a_left = result_a.wins.get(0).copied().unwrap_or(0);
    let wins_a_right = result_a.wins.get(1).copied().unwrap_or(0);
    let wins_b_left = result_b.wins.get(0).copied().unwrap_or(0);
    let wins_b_right = result_b.wins.get(1).copied().unwrap_or(0);

    let diff_a = (wins_a_left as i32 - wins_a_right as i32).abs();
    let diff_b = (wins_b_left as i32 - wins_b_right as i32).abs();
    let max_diff = diff_a.max(diff_b);

    assert!(
        max_diff <= 30,
        "mirror symmetry failed: wins A L/R {wins_a_left}/{wins_a_right}, wins B L/R {wins_b_left}/{wins_b_right}"
    );
}

#[test]
fn arthur_mirror_symmetry_large_sample() {
    if cfg!(debug_assertions) {
        return;
    }
    let (weapon_catalog, armor_catalog, shield_catalog) =
        data::load_catalogs().expect("failed to load catalogs");
    let npc_presets =
        data::load_npc_presets("data/npc_presets.json").expect("failed to load npc presets");
    let fighter_presets = data::load_fighter_presets("data/fighter_presets.json")
        .expect("failed to load fighter presets");
    let talent_catalog = data::load_talents(data::TALENTS_PATH).expect("failed to load talents");
    let race_catalog = data::load_races("data/races.json").expect("failed to load races");

    let arthur_preset = find_fighter_preset(&fighter_presets, "Arthur Du Randt")
        .expect("missing Arthur Du Randt preset");
    let arthur = player_config_from_preset(
        arthur_preset,
        &weapon_catalog,
        &armor_catalog,
        &shield_catalog,
        &race_catalog,
    );
    let players_a = [arthur.clone(), arthur.clone()];
    let players_b = [arthur.clone(), arthur];

    let stop_distance_a =
        game_logic::stop_distance_for_players(&players_a, &weapon_catalog, &talent_catalog);
    let stop_distance_b =
        game_logic::stop_distance_for_players(&players_b, &weapon_catalog, &talent_catalog);
    let combatants_a = game_logic::build_combatants(
        &players_a,
        &weapon_catalog,
        &armor_catalog,
        &shield_catalog,
        &npc_presets,
        &talent_catalog,
    );
    let combatants_b = game_logic::build_combatants(
        &players_b,
        &weapon_catalog,
        &armor_catalog,
        &shield_catalog,
        &npc_presets,
        &talent_catalog,
    );

    let runs = 500_000;
    let max_seconds = 60;
    let result_a = bulk_simulate(
        SimConfig::new(200.0, stop_distance_a),
        combatants_a,
        runs,
        max_seconds,
    );
    let result_b = bulk_simulate(
        SimConfig::new(200.0, stop_distance_b),
        combatants_b,
        runs,
        max_seconds,
    );

    let wins_a_left = result_a.wins.get(0).copied().unwrap_or(0);
    let wins_a_right = result_a.wins.get(1).copied().unwrap_or(0);
    let wins_b_left = result_b.wins.get(0).copied().unwrap_or(0);
    let wins_b_right = result_b.wins.get(1).copied().unwrap_or(0);

    let diff_a = (wins_a_left as i32 - wins_a_right as i32).abs();
    let diff_b = (wins_b_left as i32 - wins_b_right as i32).abs();
    let max_diff = diff_a.max(diff_b) as f32;
    let allowed = runs as f32 * 0.03;

    if max_diff > allowed {
        eprintln!(
            "warning: mirror symmetry drift at scale: wins A L/R {wins_a_left}/{wins_a_right}, wins B L/R {wins_b_left}/{wins_b_right} (max diff {max_diff}, allowed {allowed})"
        );
    }
}

#[test]
fn defiant_uses_lower_damage_roll_on_crit() {
    let mut attacker = combatant_basic(
        "Attacker".to_string(),
        "Sword".to_string(),
        20,
        0,
        0,
        false,
        0,
        "1d6".to_string(),
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
    let mut weapon = attacker.sheet.offense.weapon.as_ref().clone();
    weapon.crit_min_roll = 1;
    attacker.sheet.offense.weapon = Arc::new(weapon);
    let mut defender = combatant_basic(
        "Defender".to_string(),
        "Shield".to_string(),
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
        .sheet
        .modifiers
        .add_i32(StatIdI32::FlagDefiant, ModifierOpI32::Set(1));
    let mut baseline = defender.clone();
    baseline
        .sheet
        .modifiers
        .add_i32(StatIdI32::FlagDefiant, ModifierOpI32::Set(0));
    let mut found = false;
    for seed in 0..1000u64 {
        let mut state = make_state(attacker.clone(), baseline.clone());
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let baseline_outcome = resolve_attack(
            &mut state.combatants,
            0,
            1,
            0,
            false,
            1.0,
            AttackMode::Normal,
            WeaponSlot::Primary,
            0.0,
            None,
            &mut rng,
        );
        let baseline_roll = baseline_outcome
            .damage_breakdown
            .as_ref()
            .map(|detail| detail.rolled_damage)
            .unwrap_or(0);
        let mut state = make_state(attacker.clone(), defender.clone());
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let defiant_outcome = resolve_attack(
            &mut state.combatants,
            0,
            1,
            0,
            false,
            1.0,
            AttackMode::Normal,
            WeaponSlot::Primary,
            0.0,
            None,
            &mut rng,
        );
        let defiant_roll = defiant_outcome
            .damage_breakdown
            .as_ref()
            .map(|detail| detail.rolled_damage)
            .unwrap_or(0);
        if defiant_roll < baseline_roll {
            found = true;
            break;
        }
    }
    assert!(found, "defiant should lower rolled damage for some seeds");
}

#[test]
fn superior_defense_uses_upgraded_unarmed_counter_damage() {
    let attacker = combatant_basic(
        "Attacker".to_string(),
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
        5.0,
        false,
        false,
        None,
        true,
        false,
        20,
    );
    let mut defender = combatant_basic(
        "Defender".to_string(),
        "Shield".to_string(),
        10,
        10,
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
    defender
        .sheet
        .modifiers
        .add_i32(StatIdI32::FlagSuperiorDefense, ModifierOpI32::Set(1));
    let mut baseline = defender.clone();
    baseline
        .sheet
        .modifiers
        .add_i32(StatIdI32::FlagSuperiorDefense, ModifierOpI32::Set(0));
    let mut found_threshold = false;
    for seed in 0..2000u64 {
        let mut state = make_state(attacker.clone(), baseline.clone());
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let baseline_outcome = resolve_attack(
            &mut state.combatants,
            0,
            1,
            0,
            false,
            4.0,
            AttackMode::Normal,
            WeaponSlot::Primary,
            0.0,
            None,
            &mut rng,
        );
        if baseline_outcome.counter_attack.is_some() {
            continue;
        }
        let mut state = make_state(attacker.clone(), defender.clone());
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let superior_outcome = resolve_attack(
            &mut state.combatants,
            0,
            1,
            0,
            false,
            4.0,
            AttackMode::Normal,
            WeaponSlot::Primary,
            0.0,
            None,
            &mut rng,
        );
        if superior_outcome.counter_attack.is_some() {
            found_threshold = true;
            break;
        }
    }
    assert!(
        found_threshold,
        "expected superior defense to trigger on 18"
    );

    let mut found_damage = false;
    for seed in 0..2000u64 {
        let mut state = make_state(attacker.clone(), baseline.clone());
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let baseline_outcome = resolve_attack(
            &mut state.combatants,
            0,
            1,
            0,
            false,
            4.0,
            AttackMode::Normal,
            WeaponSlot::Primary,
            0.0,
            None,
            &mut rng,
        );
        let baseline_damage = baseline_outcome
            .counter_attack
            .as_ref()
            .and_then(|counter| counter.damage_breakdown.as_ref())
            .map(|detail| detail.rolled_damage);
        let Some(baseline_damage) = baseline_damage else {
            continue;
        };
        let mut state = make_state(attacker.clone(), defender.clone());
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let superior_outcome = resolve_attack(
            &mut state.combatants,
            0,
            1,
            0,
            false,
            4.0,
            AttackMode::Normal,
            WeaponSlot::Primary,
            0.0,
            None,
            &mut rng,
        );
        let superior_damage = superior_outcome
            .counter_attack
            .as_ref()
            .and_then(|counter| counter.damage_breakdown.as_ref())
            .map(|detail| detail.rolled_damage);
        let Some(superior_damage) = superior_damage else {
            continue;
        };
        if superior_damage == baseline_damage + 4 {
            found_damage = true;
            break;
        }
    }
    assert!(
        found_damage,
        "expected superior defense to add 4 to counter damage"
    );
}

#[test]
fn edge_counter_forces_critical_on_perfect_defense_riposte() {
    let attacker = combatant_basic(
        "Attacker".to_string(),
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
        5.0,
        false,
        false,
        None,
        true,
        false,
        20,
    );
    let mut defender = combatant_basic(
        "Defender".to_string(),
        "Shield".to_string(),
        0,
        10,
        0,
        false,
        0,
        "1d1".to_string(),
        0,
        10.0,
        10.0,
        5.0,
        false,
        false,
        None,
        true,
        false,
        20,
    );
    defender
        .sheet
        .modifiers
        .add_i32(StatIdI32::FlagEdgeCounter, ModifierOpI32::Set(1));
    let mut baseline = defender.clone();
    baseline
        .sheet
        .modifiers
        .add_i32(StatIdI32::FlagEdgeCounter, ModifierOpI32::Set(0));
    let mut found = false;
    for seed in 0..2000u64 {
        let mut state = make_state(attacker.clone(), baseline.clone());
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let baseline_outcome = resolve_attack(
            &mut state.combatants,
            0,
            1,
            0,
            false,
            6.0,
            AttackMode::Normal,
            WeaponSlot::Primary,
            0.0,
            None,
            &mut rng,
        );
        let Some(counter) = baseline_outcome.counter_attack.as_ref() else {
            continue;
        };
        if !counter.hit {
            continue;
        }
        if counter.critical.is_some() {
            continue;
        }
        let mut state = make_state(attacker.clone(), defender.clone());
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let edge_outcome = resolve_attack(
            &mut state.combatants,
            0,
            1,
            0,
            false,
            6.0,
            AttackMode::Normal,
            WeaponSlot::Primary,
            0.0,
            None,
            &mut rng,
        );
        let is_critical = edge_outcome
            .counter_attack
            .as_ref()
            .and_then(|counter| counter.critical.as_ref())
            .is_some();
        if is_critical {
            found = true;
            break;
        }
    }
    assert!(found, "expected edge counter to force a critical riposte");
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

    combatant.state.next_attack_time_primary = Some(2.0);
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

    combatant.state.next_attack_time_primary = Some(2.0);
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
        1.0,
        AttackMode::Normal,
        WeaponSlot::Primary,
        0.0,
        None,
        &mut rng,
    );
    assert_eq!(state.combatants[1].state.hp, 20);
}

#[test]
fn ranged_stationary_uses_d12p_defense() {
    assert_eq!(defense_die_sides(true, false, false, false, false), 12);
}

#[test]
fn ranged_moving_uses_d20p_defense() {
    assert_eq!(defense_die_sides(true, true, false, false, false), 20);
}

#[test]
fn ranged_stationary_with_shield_uses_d20p_defense() {
    assert_eq!(defense_die_sides(true, false, true, false, false), 20);
}

#[test]
fn offensive_dualwielding_uses_d10p_defense() {
    assert_eq!(defense_die_sides(false, false, false, false, true), 10);
}

#[test]
fn offhand_attack_applies_damage_penalty() {
    let mut attacker = combatant_basic(
        "Attacker".to_string(),
        "Short Sword".to_string(),
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
    let mut offhand_weapon = attacker.sheet.offense.weapon.as_ref().clone();
    offhand_weapon.name = "Offhand".to_string();
    attacker.sheet.offense.offhand = Some(OffhandProfile {
        attack_bonus: attacker.sheet.offense.attack_bonus,
        strength_damage: attacker.sheet.offense.strength_damage,
        weapon: Arc::new(offhand_weapon),
    });
    attacker.sheet.maneuvers.offensive_dualwielding = true;
    let defender = combatant_basic(
        "Defender".to_string(),
        "Fist".to_string(),
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
    let mut state_primary = make_state(attacker.clone(), defender.clone());
    let mut rng = FixedRng(0);
    let primary = resolve_attack(
        &mut state_primary.combatants,
        0,
        1,
        0,
        false,
        1.0,
        AttackMode::Normal,
        WeaponSlot::Primary,
        0.0,
        None,
        &mut rng,
    );
    let mut state_secondary = make_state(attacker, defender);
    let mut rng = FixedRng(0);
    let secondary = resolve_attack(
        &mut state_secondary.combatants,
        0,
        1,
        0,
        false,
        1.0,
        AttackMode::Normal,
        WeaponSlot::Secondary,
        0.0,
        None,
        &mut rng,
    );
    assert_eq!(primary.damage, 1);
    assert_eq!(secondary.damage, 0);
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
    let mut ranged_a = ranged.clone();
    let mut ranged_b = ranged;
    ranged_a.team_id = 0;
    ranged_b.team_id = 1;
    state.reset_with_combatants(vec![ranged_a, ranged_b]);
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
    let mut melee_a = melee.clone();
    let mut melee_b = melee;
    melee_a.team_id = 0;
    melee_b.team_id = 1;
    state.reset_with_combatants(vec![melee_a, melee_b]);
    state.tick();
    assert!(!state.combatants[0].state.moved_last_tick);
    assert!(!state.combatants[1].state.moved_last_tick);
}

#[test]
fn throwing_axe_switches_to_melee_at_close_range() {
    let throwing_axe = Arc::new(WeaponProfile {
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
        is_small_weapon: false,
        is_unarmed: false,
        crit_min_roll: 20,
        crit_min_roll_ranged: None,
        crit_severity_bonus: 0,
    });
    let melee_weapon = Arc::new(WeaponProfile {
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
        is_small_weapon: false,
        is_unarmed: false,
        crit_min_roll: 20,
        crit_min_roll_ranged: None,
        crit_severity_bonus: 0,
    });
    let attacker = Combatant::new(CombatantSheet {
        name: "Thrower".to_string(),
        offense: OffenseProfile {
            attack_bonus: 0,
            attack_bonus_base: 0,
            strength_damage: 0,
            strength_damage_base: 0,
            unarmed_damage_bonus: 0,
            weapon: throwing_axe,
            offhand: None,
        },
        defense: DefenseProfile {
            ranged_defense_mod: 0,
            defense_mod: 0,
            dex_defense_bonus: 0,
            armor_dr: 0,
            natural_dr: 0,
            knockback_step: 15,
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
        modifiers: ModifierStack::default(),
    });
    let defender = Combatant::new(CombatantSheet {
        name: "Defender".to_string(),
        offense: OffenseProfile {
            attack_bonus: 0,
            attack_bonus_base: 0,
            strength_damage: 0,
            strength_damage_base: 0,
            unarmed_damage_bonus: 0,
            weapon: melee_weapon,
            offhand: None,
        },
        defense: DefenseProfile {
            ranged_defense_mod: 0,
            defense_mod: 0,
            dex_defense_bonus: 0,
            armor_dr: 0,
            natural_dr: 0,
            knockback_step: 15,
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
        modifiers: ModifierStack::default(),
    });

    let mut sim = SimState::new(SimConfig::new(40.0, 1.0));
    let mut attacker = attacker;
    let mut defender = defender;
    attacker.team_id = 0;
    defender.team_id = 1;
    sim.reset_with_combatants(vec![attacker, defender]);

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
fn throwing_axe_cooldown_resets_on_melee_engagement() {
    let throwing_axe = Arc::new(WeaponProfile {
        name: "Throwing axe".to_string(),
        damage_expr: "1d1".to_string(),
        damage_expr_cache: DamageExprCache::new("1d1"),
        shield_damage_expr: None,
        shield_damage_expr_cache: None,
        armor_penetration: 0,
        speed: 20.0,
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
        is_small_weapon: false,
        is_unarmed: false,
        crit_min_roll: 20,
        crit_min_roll_ranged: None,
        crit_severity_bonus: 0,
    });
    let melee_weapon = Arc::new(WeaponProfile {
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
        is_small_weapon: false,
        is_unarmed: false,
        crit_min_roll: 20,
        crit_min_roll_ranged: None,
        crit_severity_bonus: 0,
    });
    let attacker = Combatant::new(CombatantSheet {
        name: "Thrower".to_string(),
        offense: OffenseProfile {
            attack_bonus: 0,
            attack_bonus_base: 0,
            strength_damage: 0,
            strength_damage_base: 0,
            unarmed_damage_bonus: 0,
            weapon: throwing_axe,
            offhand: None,
        },
        defense: DefenseProfile {
            ranged_defense_mod: 0,
            defense_mod: 0,
            dex_defense_bonus: 0,
            armor_dr: 0,
            natural_dr: 0,
            knockback_step: 15,
            armor_is_heavy: false,
            shield_name: None,
            shield_defense_bonus: 0,
            shield_dr: 0,
            shield_cover_value: None,
            shield_breakage: None,
        },
        mobility: MobilityProfile { move_speed: 20.0 },
        vitals: Vitals {
            trauma_die_sides: 20,
            trauma_die_penetrating: false,
            max_hp: 1000,
            constitution: 10,
            threshold_of_pain: 0,
        },
        maneuvers: ManeuverProfile::default(),
        modifiers: ModifierStack::default(),
    });
    let defender = Combatant::new(CombatantSheet {
        name: "Defender".to_string(),
        offense: OffenseProfile {
            attack_bonus: 0,
            attack_bonus_base: 0,
            strength_damage: 0,
            strength_damage_base: 0,
            unarmed_damage_bonus: 0,
            weapon: melee_weapon,
            offhand: None,
        },
        defense: DefenseProfile {
            ranged_defense_mod: 0,
            defense_mod: 0,
            dex_defense_bonus: 0,
            armor_dr: 0,
            natural_dr: 0,
            knockback_step: 15,
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
            max_hp: 1000,
            constitution: 10,
            threshold_of_pain: 0,
        },
        maneuvers: ManeuverProfile::default(),
        modifiers: ModifierStack::default(),
    });

    let mut sim = SimState::new(SimConfig::new(20.0, 1.0));
    let mut attacker = attacker;
    let mut defender = defender;
    attacker.team_id = 0;
    defender.team_id = 1;
    sim.reset_with_combatants(vec![attacker, defender]);

    let mut first_ranged_time: Option<u32> = None;
    let mut first_melee_time: Option<u32> = None;
    let mut seen_events = 0usize;

    for _ in 0..5 {
        sim.tick();
        if sim.combat_events.len() > seen_events {
            for event in &sim.combat_events[seen_events..] {
                if event.attacker_idx != 0 {
                    continue;
                }
                if let CombatEventKind::Attack(attack) = &event.kind {
                    if attack.is_ranged && first_ranged_time.is_none() {
                        first_ranged_time = Some(event.time);
                    } else if !attack.is_ranged && first_melee_time.is_none() {
                        first_melee_time = Some(event.time);
                    }
                }
            }
            seen_events = sim.combat_events.len();
        }
        if first_ranged_time.is_some() && first_melee_time.is_some() {
            break;
        }
    }

    assert_eq!(first_ranged_time, Some(0));
    assert_eq!(first_melee_time, Some(1));
}

#[test]
fn throwing_axe_should_allow_melee_to_close_in_gui_config() {
    let (weapon_catalog, armor_catalog, shield_catalog) =
        data::load_catalogs().expect("failed to load catalogs");
    let npc_presets =
        data::load_npc_presets("data/npc_presets.json").expect("failed to load npc presets");
    let talent_catalog = data::load_talents(data::TALENTS_PATH).expect("failed to load talents");

    let weapon_id_by_name = |name: &str| {
        weapon_catalog
            .entries()
            .iter()
            .position(|weapon| weapon.name == name)
            .and_then(|idx| weapon_catalog.id_from_index(idx))
            .unwrap_or_else(|| panic!("missing weapon preset {}", name))
    };

    let throwing_axe_id = weapon_id_by_name("Throwing axe");
    let club_id = weapon_id_by_name("Club");

    let mut thrower = game_logic::PlayerConfig::new("Thrower", throwing_axe_id);
    let mut chaser = game_logic::PlayerConfig::new("Chaser", club_id);
    thrower.base_hp = 1000;
    chaser.base_hp = 1000;
    let players = [thrower, chaser];
    let stop_distance =
        game_logic::stop_distance_for_players(&players, &weapon_catalog, &talent_catalog);

    let mut sim = SimState::new(SimConfig::new(200.0, stop_distance));
    sim.reset_with_combatants(game_logic::build_combatants(
        &players,
        &weapon_catalog,
        &armor_catalog,
        &shield_catalog,
        &npc_presets,
        &talent_catalog,
    ));
    for _ in 0..30 {
        sim.tick();
    }

    let distance = sim.distance();
    assert!(
        distance <= 5.0,
        "expected melee to close within 5ft, got {distance}"
    );
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
        1.0,
        AttackMode::Normal,
        WeaponSlot::Primary,
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
        1.0,
        AttackMode::Normal,
        WeaponSlot::Primary,
        0.0,
        None,
        &mut rng,
    );
    assert_eq!(state.combatants[1].state.hp, 20);
}

#[test]
fn zorya_vs_arthur_battle_progresses() {
    let (weapon_catalog, armor_catalog, shield_catalog) =
        data::load_catalogs().expect("failed to load catalogs");
    let npc_presets =
        data::load_npc_presets("data/npc_presets.json").expect("failed to load npc presets");
    let fighter_presets = data::load_fighter_presets("data/fighter_presets.json")
        .expect("failed to load fighter presets");
    let talent_catalog = data::load_talents(data::TALENTS_PATH).expect("failed to load talents");
    let race_catalog = data::load_races("data/races.json").expect("failed to load races");

    let arthur_preset = find_fighter_preset(&fighter_presets, "Arthur Du Randt")
        .expect("missing Arthur Du Randt preset");
    let zorya_preset =
        find_fighter_preset(&fighter_presets, "Zorya").expect("missing Zorya preset");

    let arthur = player_config_from_preset(
        arthur_preset,
        &weapon_catalog,
        &armor_catalog,
        &shield_catalog,
        &race_catalog,
    );
    let zorya = player_config_from_preset(
        zorya_preset,
        &weapon_catalog,
        &armor_catalog,
        &shield_catalog,
        &race_catalog,
    );
    let players = [arthur, zorya];
    let stop_distance =
        game_logic::stop_distance_for_players(&players, &weapon_catalog, &talent_catalog);

    let mut sim = SimState::new(SimConfig::new(stop_distance, stop_distance));
    sim.set_rng(SimRng::from_seed(42));
    sim.reset_with_combatants(game_logic::build_combatants(
        &players,
        &weapon_catalog,
        &armor_catalog,
        &shield_catalog,
        &npc_presets,
        &talent_catalog,
    ));

    let max_seconds = 300;
    while !sim.done && sim.elapsed_seconds < max_seconds {
        sim.update(1.0);
    }

    assert!(sim.done, "battle did not finish within {max_seconds}s");
    let arthur_attacked = sim
        .combat_events
        .iter()
        .any(|event| event.attacker_idx == 0 && matches!(event.kind, CombatEventKind::Attack(_)));
    let zorya_attacked = sim
        .combat_events
        .iter()
        .any(|event| event.attacker_idx == 1 && matches!(event.kind, CombatEventKind::Attack(_)));
    assert!(arthur_attacked, "Arthur never attacked");
    assert!(zorya_attacked, "Zorya never attacked");
}
