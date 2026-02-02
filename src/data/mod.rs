//! Data adapters for loading catalogs and presets from JSON.

mod armor;
mod autobattler;
mod fighter_presets;
mod materials;
mod npc_presets;
mod races;
mod talents;
mod weapons;

use crate::game_logic::{ArmorCatalog, ShieldCatalog, WeaponCatalog};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub use armor::load_armor_catalog;
pub use autobattler::load_autobattler_config;
pub use fighter_presets::{load_fighter_presets, save_fighter_presets};
pub use materials::load_materials;
pub use npc_presets::load_npc_presets;
pub use races::load_races;
pub use talents::load_talents;
pub use weapons::{load_shield_catalog, load_weapon_catalog};

pub fn resolve_data_path(path: &str) -> PathBuf {
    let raw = Path::new(path);
    if raw.is_absolute() {
        return raw.to_path_buf();
    }
    let stripped = raw.strip_prefix("data").unwrap_or(raw);
    let mut candidates = Vec::new();
    if let Ok(data_dir) = env::var("HACKMASTER_SIM_DATA_DIR") {
        candidates.push(PathBuf::from(data_dir).join(stripped));
    }
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.join("data").join(stripped));
        candidates.push(cwd.join(raw));
    } else {
        candidates.push(raw.to_path_buf());
    }
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.push(exe_dir.join("data").join(stripped));
        }
    }
    for candidate in candidates {
        if candidate.exists() {
            return candidate;
        }
    }
    raw.to_path_buf()
}

pub fn resolve_writable_data_path(path: &str) -> PathBuf {
    let raw = Path::new(path);
    if raw.is_absolute() {
        return raw.to_path_buf();
    }
    let stripped = raw.strip_prefix("data").unwrap_or(raw);
    if let Ok(data_dir) = env::var("HACKMASTER_SIM_DATA_DIR") {
        return PathBuf::from(data_dir).join(stripped);
    }
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            return exe_dir.join("data").join(stripped);
        }
    }
    if let Ok(cwd) = env::current_dir() {
        return cwd.join("data").join(stripped);
    }
    raw.to_path_buf()
}

pub fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    Ok(())
}

pub fn load_catalogs() -> Result<(WeaponCatalog, ArmorCatalog, ShieldCatalog), String> {
    let weapons = load_weapon_catalog("data/weapons.json")?;
    let armor = load_armor_catalog("data/armor.json")?;
    let shields = load_shield_catalog("data/weapons.json")?;
    let _materials = load_materials("data/materials.json")?;
    Ok((weapons, armor, shields))
}
