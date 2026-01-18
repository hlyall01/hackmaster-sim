use std::fs;
use std::path::{Path, PathBuf};

use crate::autobattler::constants::{
    CHARACTER_SAVE_DIR, CHARACTER_SAVE_EXTENSION, RUN_SAVE_DIR, RUN_SAVE_EXTENSION,
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
    let contents = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&contents).map_err(|err| err.to_string())
}

pub fn read_run_save(path: &Path) -> Result<RunSave, String> {
    let contents = fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&contents).map_err(|err| err.to_string())
}
