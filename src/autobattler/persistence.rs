use std::fs;
use std::path::{Path, PathBuf};

use crate::autobattler::constants::{
    CHARACTER_SAVE_DIR, CHARACTER_SAVE_EXTENSION, RUN_SAVE_DIR, RUN_SAVE_EXTENSION,
    RUN_SAVE_VERSION, SAVE_VERSION,
};
use crate::autobattler::state::{CharacterSave, RunSave, SaveEntry};
use crate::data;

pub fn sanitize_filename(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == ' ' || ch == '-' || ch == '_' {
            if !out.ends_with('_') {
                out.push('_');
            }
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "character".to_string()
    } else {
        trimmed
    }
}

pub fn save_path_for(file_name: &str) -> PathBuf {
    let dir = data::resolve_writable_data_path(CHARACTER_SAVE_DIR);
    dir.join(file_name)
}

pub fn run_save_path_for(file_name: &str) -> PathBuf {
    let dir = data::resolve_writable_data_path(RUN_SAVE_DIR);
    dir.join(file_name)
}

pub fn scan_save_entries() -> Vec<SaveEntry> {
    let dir = data::resolve_writable_data_path(CHARACTER_SAVE_DIR);
    let mut entries = Vec::new();
    let Ok(read_dir) = fs::read_dir(&dir) else {
        return entries;
    };
    for item in read_dir.flatten() {
        let path = item.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some(CHARACTER_SAVE_EXTENSION) {
            continue;
        }
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        let display_name = read_character_save(&path)
            .map(|save| save.name)
            .unwrap_or_else(|_| file_name.clone());
        entries.push(SaveEntry {
            file_name,
            display_name,
        });
    }
    entries.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    entries
}

pub fn scan_run_save_entries() -> Vec<SaveEntry> {
    let dir = data::resolve_writable_data_path(RUN_SAVE_DIR);
    let mut entries = Vec::new();
    let Ok(read_dir) = fs::read_dir(&dir) else {
        return entries;
    };
    for item in read_dir.flatten() {
        let path = item.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some(RUN_SAVE_EXTENSION) {
            continue;
        }
        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        let display_name = read_run_save(&path)
            .map(|save| save.name)
            .unwrap_or_else(|_| file_name.clone());
        entries.push(SaveEntry {
            file_name,
            display_name,
        });
    }
    entries.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    entries
}

pub fn write_character_save(path: &Path, save: &CharacterSave) -> Result<(), String> {
    data::ensure_parent_dir(path)?;
    let json = serde_json::to_string_pretty(save).map_err(|err| err.to_string())?;
    fs::write(path, json).map_err(|err| err.to_string())
}

pub fn write_run_save(path: &Path, save: &RunSave) -> Result<(), String> {
    data::ensure_parent_dir(path)?;
    let json = serde_json::to_string_pretty(save).map_err(|err| err.to_string())?;
    fs::write(path, json).map_err(|err| err.to_string())
}

pub fn read_character_save(path: &Path) -> Result<CharacterSave, String> {
    let contents = fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))?;
    let save: CharacterSave =
        serde_json::from_str(&contents).map_err(|err| format!("{}: {err}", path.display()))?;
    if save.version > SAVE_VERSION {
        return Err(format!(
            "{}: unsupported character save version {} (max supported: {})",
            path.display(),
            save.version,
            SAVE_VERSION
        ));
    }
    Ok(save)
}

pub fn read_run_save(path: &Path) -> Result<RunSave, String> {
    let contents = fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))?;
    let save: RunSave =
        serde_json::from_str(&contents).map_err(|err| format!("{}: {err}", path.display()))?;
    if save.version > RUN_SAVE_VERSION {
        return Err(format!(
            "{}: unsupported run save version {} (max supported: {})",
            path.display(),
            save.version,
            RUN_SAVE_VERSION
        ));
    }
    Ok(save)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autobattler::state::{
        AbilityScoreSave, CharacterSave, DowntimeActivity, LevelUpCheckpoint, RunAction, RunSave,
        RunStateSave, WoundSave,
    };
    use crate::core::gameplay::{DepthBand, EncounterTier};
    use crate::core::types::TalentSelection;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_json_path(prefix: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}_{stamp}.json"))
    }

    #[test]
    fn read_character_save_accepts_missing_version() {
        let path = temp_json_path("character_save_missing_version");
        let json = r#"{
            "name":"Arthur",
            "stats":[],
            "race_id":null,
            "talents":[],
            "bp_history":[]
        }"#;
        fs::write(&path, json).expect("write temp save");
        let save = read_character_save(&path).expect("load migrated save");
        assert_eq!(save.version, SAVE_VERSION);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_character_save_rejects_future_version() {
        let path = temp_json_path("character_save_future_version");
        let json = format!(
            r#"{{
                "version":{},
                "name":"Arthur",
                "stats":[],
                "race_id":null,
                "talents":[],
                "bp_history":[]
            }}"#,
            SAVE_VERSION + 99
        );
        fs::write(&path, json).expect("write temp save");
        let err = read_character_save(&path).expect_err("expected version error");
        assert!(err.contains("unsupported character save version"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn read_run_save_accepts_missing_version() {
        let path = temp_json_path("run_save_missing_version");
        let json = r#"{
            "name":"Run",
            "character":{"name":"Arthur","stats":[],"race_id":null,"talents":[],"bp_history":[]},
            "run_state":{
                "player":{"name":"Arthur","level":1,"xp":0,"base_stats":{"strength":{"base":10,"percentile":0},"intelligence":10,"wisdom":10,"dexterity":{"base":10,"percentile":0},"constitution":10,"looks":10,"charisma":10},"talents":[]},
                "inventory":{"gold":0,"items":[]},
                "run_depth":1,
                "seed":7,
                "encounter_index":0,
                "wounds":[]
            },
            "days_elapsed":0,
            "training_days":0,
            "run_over":false,
            "last_action":null,
            "selected_activity":"Acrobatics",
            "last_log":[]
        }"#;
        fs::write(&path, json).expect("write temp run save");
        let save = read_run_save(&path).expect("load migrated run save");
        assert_eq!(save.version, RUN_SAVE_VERSION);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn run_save_roundtrip_preserves_levelup_checkpoint() {
        let path = temp_json_path("run_save_roundtrip");
        let character = CharacterSave {
            version: SAVE_VERSION,
            name: "Arthur".to_string(),
            stats: vec![AbilityScoreSave {
                base: 10,
                percentile: 0,
            }],
            race_id: None,
            talents: vec![TalentSelection {
                id: "fast_healer".to_string(),
                rank: 1,
                weapon: None,
            }],
            bp_history: vec![],
            weapon_name: String::new(),
            armor_label: String::new(),
            shield_name: String::new(),
            alignment: String::new(),
            honor: 0,
            background: String::new(),
            height: String::new(),
            weight: String::new(),
            age: String::new(),
            handedness: String::new(),
            quirks: vec![],
            flaws: vec![],
            skills: vec![],
            skill_levels: vec![],
            proficiencies: vec![],
            starting_money: 0,
            money_rolled: false,
        };
        let run_state = RunStateSave {
            player: crate::autobattler::state::PlayerProfileSave {
                name: "Arthur".to_string(),
                level: 2,
                xp: 60,
                base_stats: crate::autobattler::state::AbilitySetSave {
                    strength: AbilityScoreSave {
                        base: 10,
                        percentile: 0,
                    },
                    intelligence: 10,
                    wisdom: 10,
                    dexterity: AbilityScoreSave {
                        base: 10,
                        percentile: 0,
                    },
                    constitution: 10,
                    looks: 10,
                    charisma: 10,
                },
                ability_scores_full: None,
                progression: Default::default(),
                points: Default::default(),
                banked_points: Default::default(),
                honor: 0,
                alignment: None,
                race_id: None,
                background: None,
                quirks: vec![],
                flaws: vec![],
                skills: vec![],
                skill_levels: vec![],
                proficiencies: vec![],
                weapon_masteries: vec![],
                talents: vec![],
            },
            inventory: crate::autobattler::state::InventorySave {
                gold: 12,
                items: vec![],
            },
            run_depth: 3,
            seed: 99,
            encounter_index: 6,
            last_encounter_tier: EncounterTier::Elite,
            last_encounter_band: DepthBand::Veteran,
            event_flags: vec!["chain_1_started".to_string()],
            seen_event_ids: vec!["evt_001".to_string()],
            wounds: vec![WoundSave {
                damage: 2,
                healing_progress_steps: 1,
            }],
        };
        let save = RunSave {
            version: RUN_SAVE_VERSION,
            name: "Run".to_string(),
            character,
            run_state,
            days_elapsed: 16,
            training_days: 8,
            run_over: false,
            awaiting_downtime_choice: false,
            pending_levelup: Some(LevelUpCheckpoint {
                levels_gained: 1,
                total_slots: 4,
                bp_slots: 4,
                lp_slots: 0,
                ap_slots: 0,
                rp_slots: 0,
            }),
            last_action: Some(RunAction::Activity),
            selected_activity: DowntimeActivity::WeaponDrills,
            last_log: vec!["ok".to_string()],
        };
        write_run_save(&path, &save).expect("write run save");
        let loaded = read_run_save(&path).expect("read run save");
        assert_eq!(loaded.pending_levelup.as_ref().map(|p| p.bp_slots), Some(4));
        assert_eq!(loaded.run_state.last_encounter_tier, EncounterTier::Elite);
        let _ = fs::remove_file(path);
    }
}
