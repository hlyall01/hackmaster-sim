use super::*;
use crate::character::MasteryState;
use crate::data;

struct Fixture {
    weapons: WeaponCatalog,
    armor: ArmorCatalog,
    shields: ShieldCatalog,
    talents: TalentCatalog,
}

impl Fixture {
    fn new() -> Self {
        let (weapons, armor, shields) = data::load_catalogs().unwrap();
        Self {
            weapons,
            armor,
            shields,
            talents: data::load_talents(data::TALENTS_PATH).unwrap(),
        }
    }

    fn weapon(&self, name: &str) -> WeaponId {
        self.weapons
            .id_from_index(
                self.weapons
                    .entries()
                    .iter()
                    .position(|w| w.name == name)
                    .unwrap(),
            )
            .unwrap()
    }

    fn player(&self, weapon: &str) -> PlayerConfig {
        PlayerConfig::new("Mastery test", self.weapon(weapon))
    }

    fn build(&self, player: &PlayerConfig) -> Combatant {
        build_combatant(
            player,
            &self.weapons,
            &self.armor,
            &self.shields,
            &NpcPresetCatalog::new(vec![]),
            &self.talents,
        )
    }
}

#[test]
fn arthurs_group_masteries_follow_the_weapon_without_leaking_to_other_groups() {
    let f = Fixture::new();
    let presets = data::load_fighter_presets("data/sim/fighter_presets.json").unwrap();
    let arthur = presets
        .entries()
        .iter()
        .find(|p| p.name == "Arthur Du Randt")
        .unwrap();
    let mut player = f.player("Halberd");
    player.weapon_masteries = weapon_masteries_for_preset(arthur, &f.weapons);
    assert_eq!(
        player.mastery(WeaponGroup::Polearms),
        MasteryState {
            attack: 5,
            damage: 4,
            speed: 4,
            defense: 4
        }
    );
    assert_eq!(
        player.mastery(WeaponGroup::Spears),
        MasteryState {
            attack: 2,
            damage: 2,
            speed: 2,
            defense: 2
        }
    );
    for group in MASTERY_GROUPS {
        if !matches!(group, WeaponGroup::Polearms | WeaponGroup::Spears) {
            assert_eq!(player.mastery(group), MasteryState::default());
        }
    }
    for (name, attack, damage, speed, defense) in [
        ("Halberd", 5, 4, 4, 4),
        ("Poleaxe", 5, 4, 4, 4),
        ("Lance", 2, 2, 2, 2),
        ("Longsword", 0, 0, 0, 0),
        ("Staff", 0, 0, 0, 0),
    ] {
        player.weapon_id = f.weapon(name);
        let trained = f.build(&player);
        let neutral = f.build(&f.player(name));
        assert_eq!(
            trained.sheet.offense.attack_bonus - neutral.sheet.offense.attack_bonus,
            attack,
            "{name}"
        );
        assert_eq!(
            trained.sheet.offense.strength_damage - neutral.sheet.offense.strength_damage,
            damage,
            "{name}"
        );
        assert_eq!(
            trained.sheet.defense.defense_mod - neutral.sheet.defense.defense_mod,
            defense,
            "{name}"
        );
        let minimum = f.weapons.get(player.weapon_id).unwrap().size.min_speed();
        assert_eq!(
            trained.sheet.offense.weapon.speed,
            (neutral.sheet.offense.weapon.speed - speed as f32).max(minimum),
            "{name}"
        );
    }
}

#[test]
fn mixed_offensive_weapons_use_independent_attack_damage_and_speed_masteries() {
    let f = Fixture::new();
    let mut player = f.player("Longsword");
    player.offhand_weapon_id = Some(f.weapon("Short sword"));
    player.offensive_dualwielding = true;
    let neutral = f.build(&player);
    *player.mastery_mut(WeaponGroup::LargeSwords) = MasteryState {
        attack: 2,
        damage: 3,
        speed: 1,
        defense: 4,
    };
    *player.mastery_mut(WeaponGroup::SmallSwords) = MasteryState {
        attack: 5,
        damage: 1,
        speed: 3,
        defense: 2,
    };
    let trained = f.build(&player);
    assert_eq!(
        trained.sheet.offense.attack_bonus - neutral.sheet.offense.attack_bonus,
        2
    );
    assert_eq!(
        trained.sheet.offense.strength_damage - neutral.sheet.offense.strength_damage,
        3
    );
    assert_eq!(
        neutral.sheet.offense.weapon.speed - trained.sheet.offense.weapon.speed,
        1.0
    );
    let offhand = trained.sheet.offense.offhand.as_ref().unwrap();
    let neutral_offhand = neutral.sheet.offense.offhand.as_ref().unwrap();
    assert_eq!(offhand.attack_bonus - neutral_offhand.attack_bonus, 5);
    assert_eq!(offhand.strength_damage - neutral_offhand.strength_damage, 1);
    assert_eq!(neutral_offhand.weapon.speed - offhand.weapon.speed, 3.0);
}

#[test]
fn defensive_dualwield_sums_each_weapons_group_mastery() {
    let f = Fixture::new();
    let mut player = f.player("Longsword");
    player.offhand_weapon_id = Some(f.weapon("Short sword"));
    player.defensive_dualwielding = true;
    let neutral = f.build(&player);
    player.mastery_mut(WeaponGroup::LargeSwords).defense = 4;
    player.mastery_mut(WeaponGroup::SmallSwords).defense = 1;
    assert_eq!(
        f.build(&player).sheet.defense.defense_mod - neutral.sheet.defense.defense_mod,
        5
    );
    player.offhand_weapon_id = Some(f.weapon("Longsword"));
    assert_eq!(
        f.build(&player).sheet.defense.defense_mod - neutral.sheet.defense.defense_mod,
        8
    );
}

#[test]
fn evonia_uses_secondary_sword_mastery_independently_of_the_primary() {
    let f = Fixture::new();
    let mut player = f.player("Greatsword");
    player.offhand_weapon_id = Some(f.weapon("Short sword"));
    player.proficiencies = vec!["Greatsword".into(), "Short sword".into()];
    player.talents = vec![TalentSelection {
        id: "left_hand_of_evonia".into(),
        rank: 1,
        weapon: None,
    }];
    player.default_weapon_style_ids = Some(vec!["left_hand_of_evonia".into()]);
    let neutral = f.build(&player);
    *player.mastery_mut(WeaponGroup::LargeSwords) = MasteryState {
        attack: 5,
        defense: 4,
        ..MasteryState::default()
    };
    *player.mastery_mut(WeaponGroup::SmallSwords) = MasteryState {
        attack: 1,
        defense: 2,
        ..MasteryState::default()
    };
    let trained = f.build(&player);
    assert_eq!(
        trained.sheet.defense.defense_mod - neutral.sheet.defense.defense_mod,
        6
    );
    assert_eq!(
        trained.sheet.offense.offhand.as_ref().unwrap().attack_bonus
            - neutral.sheet.offense.offhand.as_ref().unwrap().attack_bonus,
        1
    );
    assert_eq!(
        trained.sheet.offense.attack_bonus - neutral.sheet.offense.attack_bonus,
        5
    );
}

#[test]
fn legacy_masteries_migrate_to_saved_weapon_groups_and_explicit_empty_overrides_them() {
    let f = Fixture::new();
    let presets = data::load_fighter_presets("data/sim/fighter_presets.json").unwrap();
    let mut json = serde_json::to_value(&presets.entries()[0]).unwrap();
    json.as_object_mut().unwrap().remove("weapon_masteries");
    json["weapon"] = "Longsword".into();
    json["offhand_weapon"] = "Short sword".into();
    json["masteries"] = serde_json::json!({"attack": 2, "damage": 4, "speed": 1, "defense": 3, "shield_defense": 5, "shield_speed": 2});
    let legacy: FighterPreset = serde_json::from_value(json.clone()).unwrap();
    let mut player = f.player("Lance");
    player.weapon_masteries = weapon_masteries_for_preset(&legacy, &f.weapons);
    assert_eq!(player.mastery(WeaponGroup::LargeSwords).attack, 2);
    assert_eq!(player.mastery(WeaponGroup::SmallSwords).damage, 4);
    assert_eq!(player.mastery(WeaponGroup::Shields).defense, 5);
    assert_eq!(player.mastery(WeaponGroup::Spears), MasteryState::default());
    json["weapon_masteries"] = serde_json::json!({});
    let cleared: FighterPreset = serde_json::from_value(json).unwrap();
    assert!(weapon_masteries_for_preset(&cleared, &f.weapons).is_empty());
}

#[test]
fn group_masteries_round_trip_including_unequipped_groups_and_have_no_legacy_values() {
    let f = Fixture::new();
    let presets = data::load_fighter_presets("data/sim/fighter_presets.json").unwrap();
    let preset = &presets.entries()[0];
    let json = serde_json::to_value(preset).unwrap();
    assert!(json.get("masteries").is_none());
    assert_eq!(json["weapon_masteries"]["spears"]["attack"], 2);
    let restored: FighterPreset = serde_json::from_value(json).unwrap();
    assert_eq!(
        weapon_masteries_for_preset(&restored, &f.weapons),
        weapon_masteries_for_preset(preset, &f.weapons)
    );
}
