use hackmaster_sim::character::{AbilityScore, AbilitySet, Progression, ProgressionTier};
use hackmaster_sim::core::gameplay::{
    run_next_fight, CombatantBuilder, EnemySpawnEntry, EnemySpawner, LootTable, RunState, Wound,
};
use hackmaster_sim::core::ids::NpcPresetId;
use hackmaster_sim::core::rng::SimRng;
use hackmaster_sim::core::sim::{CombatEvent, CombatEventKind, SimConfig};
use hackmaster_sim::core::types::{EnemyProfile, Inventory, PlayerProfile};
use hackmaster_sim::data;
use hackmaster_sim::game_logic::{
    self, ArmorCatalog, ArmorId, FighterPreset, FighterPresetCatalog, NpcPresetCatalog,
    PlayerConfig, ShieldCatalog, ShieldId, TalentCatalog, WeaponCatalog, WeaponId,
};

const FIGHTER_PRESETS_PATH: &str = "data/fighter_presets.json";
const NPC_PRESETS_PATH: &str = "data/npc_presets.json";
const TALENTS_PATH: &str = "data/talents.json";
const FIGHTS_TO_RUN: u32 = 5;
const MAX_FIGHT_SECONDS: u32 = 120;
const REST_DAYS_BETWEEN_ENCOUNTERS: u32 = 8;
const DEFAULT_ENEMY_WEAPON: &str = "Battle axe";

struct AutobattlerBuilder<'a> {
    player_base: PlayerConfig,
    enemy_weapon_id: WeaponId,
    weapon_catalog: &'a WeaponCatalog,
    armor_catalog: &'a ArmorCatalog,
    shield_catalog: &'a ShieldCatalog,
    npc_presets: &'a NpcPresetCatalog,
    talent_catalog: &'a TalentCatalog,
}

impl CombatantBuilder for AutobattlerBuilder<'_> {
    fn build_player(&self, state: &RunState) -> hackmaster_sim::core::sim::Combatant {
        let mut player = self.player_base.clone();
        player.name = state.player.name.clone();
        player.level = state.player.level;
        player.strength_base = state.player.base_stats.strength.base;
        player.strength_pct = state.player.base_stats.strength.percentile;
        player.dex_base = state.player.base_stats.dexterity.base;
        player.dex_pct = state.player.base_stats.dexterity.percentile;
        player.intelligence = state.player.base_stats.intelligence;
        player.wisdom = state.player.base_stats.wisdom;
        player.constitution = state.player.base_stats.constitution;
        player.looks = state.player.base_stats.looks;
        player.charisma = state.player.base_stats.charisma;
        let mut combatant = game_logic::build_combatant(
            &player,
            self.weapon_catalog,
            self.armor_catalog,
            self.shield_catalog,
            self.npc_presets,
            self.talent_catalog,
        );
        let wound_total = state.total_wound_damage();
        if wound_total > 0 {
            let wound_total = i32::try_from(wound_total).unwrap_or(i32::MAX);
            let adjusted_hp = (combatant.sheet.vitals.max_hp - wound_total).max(0);
            combatant.state.hp = adjusted_hp;
        }
        combatant
    }

    fn build_enemy(&self, enemy: &EnemyProfile) -> hackmaster_sim::core::sim::Combatant {
        let mut npc = PlayerConfig::new("Hobgoblin", self.enemy_weapon_id);
        npc.level = enemy.level;
        npc.npc_preset = Some(enemy.preset_id);
        game_logic::build_combatant(
            &npc,
            self.weapon_catalog,
            self.armor_catalog,
            self.shield_catalog,
            self.npc_presets,
            self.talent_catalog,
        )
    }
}

fn main() {
    let (weapon_catalog, armor_catalog, shield_catalog) = data::load_catalogs()
        .unwrap_or_else(|err| panic!("Failed to load JSON catalogs: {err}"));
    let npc_presets =
        data::load_npc_presets(NPC_PRESETS_PATH).expect("Failed to load NPC presets");
    let fighter_presets =
        data::load_fighter_presets(FIGHTER_PRESETS_PATH).expect("Failed to load fighter presets");
    let talent_catalog = data::load_talents(TALENTS_PATH).expect("Failed to load talents");

    let arthur_preset = find_fighter_preset(&fighter_presets, "Arthur Du Randt")
        .or_else(|| fighter_presets.entries().first())
        .expect("No fighter presets found");
    let player_config = player_config_from_preset(
        arthur_preset,
        &weapon_catalog,
        &armor_catalog,
        &shield_catalog,
    );
    let player_profile = player_profile_from_config(&player_config);
    let mut run_state = RunState::new(player_profile, Inventory::default());

    let spawner = hobgoblin_spawner(&npc_presets);
    let loot_table = LootTable {
        gold_range: 8..=16,
        xp_per_level: 0,
        item_table: Vec::new(),
    };
    let sim_config = SimConfig::new(20.0, 1.0);
    let mut rng = SimRng::from_seed(7);

    let enemy_weapon_id = find_weapon_id_by_name(&weapon_catalog, DEFAULT_ENEMY_WEAPON)
        .or_else(|| weapon_catalog.first_id())
        .unwrap_or(WeaponId::new(0));
    let builder = AutobattlerBuilder {
        player_base: player_config,
        enemy_weapon_id,
        weapon_catalog: &weapon_catalog,
        armor_catalog: &armor_catalog,
        shield_catalog: &shield_catalog,
        npc_presets: &npc_presets,
        talent_catalog: &talent_catalog,
    };

    println!("Autobattler run start: {}", run_state.player.name);
    for fight_index in 1..=FIGHTS_TO_RUN {
        let outcome = run_next_fight(
            run_state,
            &spawner,
            &loot_table,
            None,
            sim_config,
            MAX_FIGHT_SECONDS,
            REST_DAYS_BETWEEN_ENCOUNTERS,
            &builder,
            &mut rng,
        );
        let enemy_name = outcome
            .enemy
            .and_then(|enemy| npc_presets.get(enemy.preset_id))
            .map(|preset| preset.name.as_str())
            .unwrap_or("Unknown");

        let status = if outcome.fight.won { "WIN" } else { "LOSS" };
        let reward_gold = outcome.reward.as_ref().map(|r| r.gold).unwrap_or(0);
        let wound_total = outcome.state.total_wound_damage();
        let wound_list = if outcome.state.wounds.is_empty() {
            "none".to_string()
        } else {
            outcome
                .state
                .wounds
                .iter()
                .map(|wound| wound.damage.to_string())
                .collect::<Vec<_>>()
                .join(",")
        };
        let wound_tracker = format_wound_tracker(&outcome.state.wounds);
        let hits_dealt = format_hit_list(&outcome.fight.events, 0, 1);
        let hits_taken = format_hit_list(&outcome.fight.events, 1, 0);
        println!(
            "Fight {fight_index}: vs {enemy_name} -> {status} | hp={} | wounds=[{}] total_wounds={} | +{}g | total_g={}",
            outcome.fight.remaining_hp,
            wound_list,
            wound_total,
            reward_gold,
            outcome.state.inventory.gold
        );
        println!("  Wound tracker (hd progress/need): {wound_tracker}");
        println!("  Hits (hp damage): dealt=[{hits_dealt}] taken=[{hits_taken}]");

        run_state = outcome.state;
        if !outcome.fight.won {
            println!("Run ended at depth {}.", run_state.run_depth);
            break;
        }
    }
}

fn format_wound_tracker(wounds: &[Wound]) -> String {
    if wounds.is_empty() {
        return "none".to_string();
    }
    wounds
        .iter()
        .map(|wound| {
            let damage = wound.damage;
            let progress = wound.healing_progress_half_days.min(damage);
            format!("{damage}({progress}/{damage} hd)")
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_hit_list(events: &[CombatEvent], attacker_idx: usize, defender_idx: usize) -> String {
    let hits = events
        .iter()
        .filter_map(|event| {
            if event.attacker_idx != attacker_idx || event.defender_idx != defender_idx {
                return None;
            }
            let CombatEventKind::Attack(attack) = &event.kind else {
                return None;
            };
            if attack.damage <= 0 {
                return None;
            }
            Some(attack.damage.to_string())
        })
        .collect::<Vec<_>>();

    if hits.is_empty() {
        return "none".to_string();
    }

    hits.join(",")
}

fn find_fighter_preset<'a>(
    catalog: &'a FighterPresetCatalog,
    name: &str,
) -> Option<&'a FighterPreset> {
    catalog
        .entries()
        .iter()
        .find(|preset| preset.name.eq_ignore_ascii_case(name))
}

fn player_config_from_preset(
    preset: &FighterPreset,
    weapon_catalog: &WeaponCatalog,
    armor_catalog: &ArmorCatalog,
    shield_catalog: &ShieldCatalog,
) -> PlayerConfig {
    let attack = tier_from_label(&preset.progression.attack).unwrap_or(ProgressionTier::I);
    let speed = tier_from_label(&preset.progression.speed).unwrap_or(ProgressionTier::I);
    let initiative = tier_from_label(&preset.progression.initiative).unwrap_or(ProgressionTier::I);
    let health = tier_from_label(&preset.progression.health).unwrap_or(ProgressionTier::I);

    let mut player = PlayerConfig::new(
        &preset.name,
        weapon_catalog
            .first_id()
            .unwrap_or_else(|| WeaponId::new(0)),
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
    player.armor_material_tier = preset.armor_material_tier;
    player.projectile_material_tier = preset.projectile_material_tier;
    player.shield_material_tier = preset.shield_material_tier;
    player.two_hand_grip = preset.two_hand_grip;
    player.use_jab = preset.use_jab;
    player.hold_at_bay = preset.hold_at_bay;
    player.defensive_dualwielding = preset.defensive_dualwielding;
    player.talents = preset.talents.clone();
    player.weapon_id = find_weapon_id_by_name(weapon_catalog, &preset.weapon)
        .or_else(|| weapon_catalog.first_id())
        .unwrap_or_else(|| WeaponId::new(0));
    player.armor_id = find_armor_id_by_name(armor_catalog, &preset.armor)
        .or_else(|| armor_catalog.first_id())
        .unwrap_or_else(|| ArmorId::new(0));
    player.shield_id = find_shield_id_by_name(shield_catalog, &preset.shield)
        .or_else(|| shield_catalog.first_id())
        .unwrap_or_else(|| ShieldId::new(0));
    if let Some(weapon) = weapon_catalog.get(player.weapon_id) {
        game_logic::sanitize_projectile_tier(&mut player, weapon);
    }
    player
}

fn player_profile_from_config(config: &PlayerConfig) -> PlayerProfile {
    PlayerProfile {
        name: config.name.clone(),
        level: config.level,
        xp: 0,
        base_stats: AbilitySet {
            strength: AbilityScore::new(config.strength_base, config.strength_pct),
            intelligence: config.intelligence,
            wisdom: config.wisdom,
            dexterity: AbilityScore::new(config.dex_base, config.dex_pct),
            constitution: config.constitution,
            looks: config.looks,
            charisma: config.charisma,
        },
    }
}

fn hobgoblin_spawner(npc_presets: &NpcPresetCatalog) -> EnemySpawner {
    let mut spawner = EnemySpawner::default();
    for (index, preset) in npc_presets.entries().iter().enumerate() {
        if let Some(level) = hobgoblin_level(&preset.name) {
            spawner.push(EnemySpawnEntry {
                preset_id: NpcPresetId::new(index),
                min_level: level,
                max_level: u8::MAX,
                weight: 1,
            });
        }
    }
    spawner
}

fn hobgoblin_level(name: &str) -> Option<u8> {
    let lower = name.to_ascii_lowercase();
    if lower == "hobgoblin" {
        Some(1)
    } else if let Some(rest) = lower.strip_prefix("hobgoblin ") {
        rest.trim().parse::<u8>().ok()
    } else {
        None
    }
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

fn find_weapon_id_by_name(catalog: &WeaponCatalog, name: &str) -> Option<WeaponId> {
    catalog
        .entries()
        .iter()
        .position(|weapon| weapon.name.eq_ignore_ascii_case(name))
        .and_then(|idx| catalog.id_from_index(idx))
}

fn find_armor_id_by_name(catalog: &ArmorCatalog, name: &str) -> Option<ArmorId> {
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

fn find_shield_id_by_name(catalog: &ShieldCatalog, name: &str) -> Option<ShieldId> {
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
