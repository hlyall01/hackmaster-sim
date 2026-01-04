#[path = "../character.rs"]
mod character;
#[path = "../sim.rs"]
mod sim;
#[path = "../game_logic.rs"]
#[allow(dead_code)]
mod game_logic;

use character::{
    AbilityScore, AbilitySet, ArmorRegion, Character, Equipment, MaterialKind, Progression,
    ProgressionTier, Weapon, WeaponGroup, WeaponMastery,
};
use sim::{
    Combatant, CombatantSheet, DefenseProfile, MobilityProfile, OffenseProfile, SimConfig,
    SimState, Vitals, WeaponProfile,
};

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

    let armor = character::ARMOR
        .iter()
        .find(|a| a.name == "Chainmail" && a.region == ArmorRegion::Northern)
        .cloned();

    let equipment = Equipment {
        weapon: Some(weapon.clone()),
        shield: None,
        armor,
        weapon_material: character::MATERIALS
            .iter()
            .find(|m| m.kind == MaterialKind::Metal && m.name == "Steel")
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
    println!("Base DV: {}", derived.base_dv);
    println!("Armor DR: {}", derived.armor_dr);
    println!(
        "Carry capacity (none/light/medium/heavy): {:?}",
        derived.carry_capacity
    );
    println!("Load category: {}", derived.load_category);

    let mut sim = SimState::new(SimConfig::new(20.0, reach_ft));
    let strength_damage = game_logic::strength_damage_for_weapon(
        &weapon.name,
        character.ability_mods.strength.damage,
    );
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
    let armor_is_heavy = character
        .equipment
        .armor
        .as_ref()
        .map(|armor| matches!(armor.armor_type, character::ArmorType::Heavy))
        .unwrap_or(false);
    let sheet = CombatantSheet {
        name: character.name.clone(),
        offense: OffenseProfile {
            attack_bonus: derived.attack_bonus,
            strength_damage,
            weapon: WeaponProfile {
                name: weapon_name,
                damage_expr: weapon_damage_expr,
                shield_damage_expr: None,
                armor_penetration,
                speed: weapon_speed,
                reach_ft: weapon_reach,
                two_hand_grip: false,
                use_jab: false,
                jab_special_expr: None,
                has_weapon,
                defense_bonus_always: weapon_defense_always,
            },
        },
        defense: DefenseProfile {
            defense_mod: derived.base_dv,
            armor_dr: derived.armor_dr,
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
            threshold_of_pain: game_logic::threshold_of_pain(
                derived.hit_points as i32,
                character.level,
            ),
        },
    };
    let combatant = Combatant::new(sheet);
    sim.reset_with_combatants([combatant.clone(), combatant]);
    println!("--- Simulation (1s ticks) ---");
    while !sim.done {
        sim.update(1.0);
        println!(
            "t={}s | distance={:.1} ft",
            sim.elapsed_seconds,
            sim.distance()
        );
        if let Some(event) = &sim.last_event {
            println!("{event}");
        }
        if sim.elapsed_seconds > 120 {
            println!("Stopping after 120s (safety cutoff).");
            break;
        }
    }
}
