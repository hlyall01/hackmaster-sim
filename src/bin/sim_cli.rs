use character::{
    AbilityScore, AbilitySet, ArmorRegion, Character, Equipment, MaterialKind, Progression,
    ProgressionTier, Weapon, WeaponGroup, WeaponMastery,
};
use hackmaster_sim::core::rules::DamageExprCache;
use hackmaster_sim::{character, data, game_logic, sim};
use sim::{
    Combatant, CombatantSheet, DefenseProfile, MobilityProfile, OffenseProfile, SimConfig,
    SimState, Vitals, WeaponProfile,
};
use std::sync::Arc;

fn main() {
    let abilities = AbilitySet {
        strength: AbilityScore::new(15, 1), // 15/01
        intelligence: 12,
        wisdom: 11,
        dexterity: AbilityScore::new(13, 1),
        constitution: 14,
        looks: 10,
        charisma: 10,
    };

    let weapon = Weapon {
        name: "Longsword".to_string(),
        group: WeaponGroup::LargeSwords,
        speed: 10.0,
        damage_expr: "2d8p".to_string(),
        reach_ft: 3.5,
        armor_pen: 0,
        defense_bonus_always: false,
    };
    let reach_ft = weapon.reach_ft;

    let mastery = WeaponMastery {
        group: WeaponGroup::LargeSwords,
        points: Default::default(),
        base_threshold: 100.0,
    };

    let armor_catalog =
        data::load_armor_catalog("data/armor.json").expect("Failed to load armor catalog");
    let armor = armor_catalog.entries().iter().find_map(|entry| {
        entry
            .armor
            .as_ref()
            .filter(|armor| armor.name == "Chainmail" && armor.region == ArmorRegion::Northern)
            .cloned()
    });
    let materials = data::load_materials("data/materials.json").expect("Failed to load materials");

    let equipment = Equipment {
        weapon: Some(weapon.clone()),
        shield: None,
        armor,
        weapon_material: materials
            .iter()
            .find(|material| material.kind == MaterialKind::Metal && material.name == "Steel")
            .cloned(),
        armor_material: None,
        shield_material: None,
    };

    let character = Character::builder("Example Duelist")
        .level(
            5,
            Progression::new(
                ProgressionTier::III,
                ProgressionTier::III,
                ProgressionTier::III,
                ProgressionTier::III,
            ),
        )
        .base_hp(10)
        .abilities(abilities)
        .weapon_mastery(mastery)
        .equipment(equipment)
        .build();

    let derived = character.derived();
    let dex_defense_bonus = character.ability_mods.dexterity.defense;

    println!("Character: {}", character.name);
    println!("Level: {} ({:?})", character.level, character.progression);
    println!("Attack bonus: {}", derived.attack_bonus);
    println!("Speed mod: {}", derived.speed_mod);
    println!("Initiative mod: {}", derived.initiative_mod);
    println!("Initiative die: {:?}", derived.initiative_die);
    println!(
        "Hit points: {} (x{:.1})",
        derived.hit_points, derived.health_mult
    );
    println!("Drain resistance: {}", derived.drain_resistance);
    println!("Base DV: {}", derived.base_dv);
    println!("Armor DR: {}", derived.armor_dr);
    println!(
        "Carry capacity (none/light/medium/heavy): {:?}",
        derived.carry_capacity
    );
    println!("Load category: {}", derived.load_category);

    let weapon_catalog =
        data::load_weapon_catalog("data/weapons.json").expect("Failed to load weapon catalog");
    let weapon_preset = weapon_catalog
        .entries()
        .iter()
        .find(|preset| preset.name == weapon.name);
    let strength_damage = weapon_preset
        .map(|preset| {
            game_logic::strength_damage_for_weapon(preset, character.ability_mods.strength.damage)
        })
        .unwrap_or(character.ability_mods.strength.damage);
    let weapon_name = character
        .equipment
        .weapon
        .as_ref()
        .map(|weapon| weapon.name.clone())
        .unwrap_or_else(|| "Unarmed".to_string());
    let weapon_damage_expr = character
        .equipment
        .weapon
        .as_ref()
        .map(|weapon| weapon.damage_expr.clone())
        .unwrap_or_else(|| "d4p".to_string());
    let weapon_damage_cache = DamageExprCache::new(&weapon_damage_expr);
    let weapon_speed = character
        .equipment
        .weapon
        .as_ref()
        .map(|weapon| weapon.speed)
        .unwrap_or(10.0);
    let weapon_reach = character
        .equipment
        .weapon
        .as_ref()
        .map(|weapon| weapon.reach_ft)
        .unwrap_or(1.0);
    let armor_penetration = character
        .equipment
        .weapon
        .as_ref()
        .map(|weapon| weapon.armor_pen)
        .unwrap_or(0);
    let weapon_defense_always = character
        .equipment
        .weapon
        .as_ref()
        .map(|weapon| weapon.defense_bonus_always)
        .unwrap_or(false);
    let has_weapon = character.equipment.weapon.is_some();
    let range_bands_feet = weapon_preset.and_then(|preset| preset.range_bands_feet);
    let uses_projectiles = weapon_preset
        .map(|preset| game_logic::weapon_uses_projectiles(preset))
        .unwrap_or(false);
    let hacking_or_piercing = weapon_preset
        .map(|preset| preset.hacking_or_piercing)
        .unwrap_or(false);
    let armor_is_heavy = character
        .equipment
        .armor
        .as_ref()
        .map(|armor| matches!(armor.armor_type, character::ArmorType::Heavy))
        .unwrap_or(false);
    let mut sim = SimState::new(SimConfig::new(20.0, reach_ft));
    let sheet = CombatantSheet {
        name: character.name.clone(),
        offense: OffenseProfile {
            attack_bonus: derived.attack_bonus,
            attack_bonus_base: derived.attack_bonus,
            strength_damage,
            strength_damage_base: character.ability_mods.strength.damage,
            unarmed_damage_bonus: 0,
            weapon: Arc::new(WeaponProfile {
                name: weapon_name,
                damage_expr: weapon_damage_expr,
                damage_expr_cache: weapon_damage_cache,
                shield_damage_expr: None,
                shield_damage_expr_cache: None,
                armor_penetration,
                speed: weapon_speed,
                reach_ft: weapon_reach,
                range_bands_feet,
                range_distance_multiplier: 1.0,
                two_hand_grip: false,
                use_jab: false,
                jab_special_expr: None,
                jab_special_expr_cache: None,
                has_weapon,
                defense_bonus_always: weapon_defense_always,
                uses_projectiles,
                is_small_weapon: false,
                is_unarmed: weapon.group == WeaponGroup::Unarmed,
                hacking_or_piercing,
                force_nonpenetrating_damage: false,
                halve_damage: false,
                ignore_all_dr: false,
                internal_hemorrhage_damage: 0,
                use_close_hit_damage_expr: None,
                use_close_hit_damage_expr_cache: None,
                use_close_hit_margin_less_than: 0,
                crit_min_roll: 20,
                crit_min_roll_ranged: None,
                crit_severity_bonus: 0,
                defender_knockback_step_adjustment: 0,
            }),
            offhand: None,
        },
        defense: DefenseProfile {
            defense_mod: derived.base_dv,
            ranged_defense_mod: 0,
            dex_defense_bonus,
            armor_dr: derived.armor_dr,
            natural_dr: 0,
            knockback_step: game_logic::DEFAULT_KNOCKBACK_STEP,
            armor_is_heavy,
            shield_name: None,
            shield_defense_bonus: 0,
            shield_dr: 0,
            shield_cover_value: None,
            shield_breakage: None,
        },
        mobility: MobilityProfile { move_speed: 5.0 },
        vitals: Vitals {
            max_hp: derived.hit_points as i32,
            constitution: character.abilities.constitution,
            drain_resistance: derived.drain_resistance,
            threshold_of_pain: game_logic::threshold_of_pain(
                derived.hit_points as i32,
                character.level,
            ),
            trauma_die_sides: 20,
            trauma_die_penetrating: false,
        },
        maneuvers: sim::ManeuverProfile {
            hold_at_bay: false,
            called_shot: false,
            called_shot_defense_bonus: 8,
            called_shot_defense_penalty: 4,
            called_shot_delay_profile: sim::CalledShotDelayProfile::Standard,
            called_shot_deceptive_defender: false,
            called_shot_target_defense_bonus_base: 8,
            power_attack: false,
            aggressive_attack: false,
            charge: false,
            ready_against_charge: false,
            tactical_move: false,
            fight_defensively: false,
            fight_defensively_attack_penalty: 0,
            fight_defensively_defense_bonus: 0,
            full_parry: false,
            give_ground: false,
            scamper_back: false,
            fighting_withdrawal: false,
            flee: false,
            mounted: false,
            defensive_dualwielding: false,
            offensive_dualwielding: false,
            offensive_dualwielding_defense_penalty: false,
            dualwield_offhand_damage_penalty: -2,
            dualwield_primary_recovery_penalty: 2.0,
            dualwield_secondary_recovery_penalty: 2.0,
        },
        modifiers: sim::ModifierStack::default(),
    };
    let combatant = Combatant::new(sheet);
    let mut combatant_a = combatant.clone();
    let mut combatant_b = combatant;
    combatant_a.team_id = 0;
    combatant_b.team_id = 1;
    sim.reset_with_combatants(vec![combatant_a, combatant_b]);
    println!("--- Simulation (1s ticks) ---");
    let mut printed_events = 0usize;
    while !sim.done {
        sim.update(1.0);
        println!(
            "t={}s | distance={:.1} ft",
            sim.elapsed_seconds,
            sim.distance()
        );
        if printed_events < sim.combat_events.len() {
            for event in &sim.combat_events[printed_events..] {
                println!("{}", sim::format_combat_event_line(event, &sim.combatants));
            }
            printed_events = sim.combat_events.len();
        }
        if sim.elapsed_seconds > 120 {
            println!("Stopping after 120s (safety cutoff).");
            break;
        }
    }
}
