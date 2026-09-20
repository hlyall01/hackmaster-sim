//! Data adapters for loading catalogs and presets from JSON.

mod armor;
mod autobattler;
mod autobattler_events;
mod fighter_presets;
mod materials;
mod npc_presets;
mod races;
mod tactical_presets;
mod talents;
mod weapons;

use crate::game_logic::{ArmorCatalog, ShieldCatalog, WeaponCatalog};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub use armor::load_armor_catalog;
pub use autobattler::load_autobattler_config;
pub use autobattler_events::load_autobattler_events;
pub use fighter_presets::{load_fighter_presets, save_fighter_presets};
pub use materials::load_materials;
pub use npc_presets::load_npc_presets;
pub use races::load_races;
pub use tactical_presets::{
    TACTICAL_PRESET_SCHEMA_VERSION, load_tactical_presets, save_tactical_presets,
};
pub use talents::load_talents;
pub use weapons::{load_shield_catalog, load_weapon_catalog};

pub const TALENTS_PATH: &str = "data/sim/talents.json";

fn mapped_data_subpath(path: &Path) -> PathBuf {
    let stripped = path.strip_prefix("data").unwrap_or(path);
    let as_str = stripped.to_string_lossy();
    if as_str.starts_with("sim/") || as_str.starts_with("autobattler/") {
        return stripped.to_path_buf();
    }
    let Some(file_name) = stripped.file_name().and_then(|name| name.to_str()) else {
        return stripped.to_path_buf();
    };
    match file_name {
        "autobattler_config.json"
        | "autobattler_quick_starts.json"
        | "events_v1.json"
        | "events_v1_handcrafted.json" => PathBuf::from("autobattler").join(file_name),
        "armor.json"
        | "fighter_presets.json"
        | "materials.json"
        | "npc_presets.json"
        | "races.json"
        | "talents.json"
        | "weapons.json" => PathBuf::from("sim").join(file_name),
        _ => stripped.to_path_buf(),
    }
}

pub fn resolve_data_path(path: &str) -> PathBuf {
    let raw = Path::new(path);
    if raw.is_absolute() {
        return raw.to_path_buf();
    }
    let stripped = raw.strip_prefix("data").unwrap_or(raw);
    let mapped = mapped_data_subpath(raw);
    let mut candidates = Vec::new();
    if let Ok(data_dir) = env::var("HACKMASTER_SIM_DATA_DIR") {
        let base = PathBuf::from(data_dir);
        candidates.push(base.join(stripped));
        if mapped != stripped {
            candidates.push(base.join(&mapped));
        }
    }
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.join("data").join(stripped));
        if mapped != stripped {
            candidates.push(cwd.join("data").join(&mapped));
        }
        candidates.push(cwd.join(raw));
    } else {
        candidates.push(raw.to_path_buf());
    }
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidates.push(exe_dir.join("data").join(stripped));
            if mapped != stripped {
                candidates.push(exe_dir.join("data").join(&mapped));
            }
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
    let mapped = mapped_data_subpath(raw);
    if let Ok(data_dir) = env::var("HACKMASTER_SIM_DATA_DIR") {
        return PathBuf::from(data_dir).join(mapped);
    }
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            return exe_dir.join("data").join(mapped);
        }
    }
    if let Ok(cwd) = env::current_dir() {
        return cwd.join("data").join(mapped);
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
    let weapons = load_weapon_catalog("data/sim/weapons.json")?;
    let armor = load_armor_catalog("data/sim/armor.json")?;
    let shields = load_shield_catalog("data/sim/weapons.json")?;
    let _materials = load_materials("data/sim/materials.json")?;
    Ok((weapons, armor, shields))
}

pub fn validate_required_data_files(paths: &[&str]) -> Result<(), Vec<String>> {
    let mut missing = Vec::new();
    for path in paths {
        let resolved = resolve_data_path(path);
        if !resolved.exists() {
            missing.push(format!("{path} (resolved: {})", resolved.display()));
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(missing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_legacy_sim_paths_into_sim_namespace() {
        let mapped = mapped_data_subpath(Path::new("data/weapons.json"));
        assert_eq!(mapped, PathBuf::from("sim").join("weapons.json"));
    }

    #[test]
    fn maps_legacy_autobattler_paths_into_autobattler_namespace() {
        let mapped = mapped_data_subpath(Path::new("data/autobattler_config.json"));
        assert_eq!(
            mapped,
            PathBuf::from("autobattler").join("autobattler_config.json")
        );
    }
}
