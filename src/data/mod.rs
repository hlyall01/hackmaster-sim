//! Data adapters for loading catalogs and presets from JSON.

mod armor;
mod atomic;
pub use atomic::atomic_write;
mod fighter_presets;
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
pub use fighter_presets::{
    load_fighter_presets, load_fighter_presets_recovering, save_fighter_presets,
};
pub use npc_presets::load_npc_presets;
pub use races::load_races;
pub use tactical_presets::{
    TACTICAL_PRESET_SCHEMA_VERSION, load_tactical_presets, load_tactical_presets_recovering,
    save_tactical_presets,
};
pub use talents::load_talents;
pub use weapons::{load_shield_catalog, load_weapon_catalog};

pub const TALENTS_PATH: &str = "data/sim/talents.json";

#[cfg(not(target_arch = "wasm32"))]
fn read_saved_presets(path: &str) -> Result<Option<String>, String> {
    match fs::read_to_string(resolve_data_path(path)) {
        Ok(data) => Ok(Some(data)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(format!("Cannot read saved presets: {err}")),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn write_presets(path: &str, data: &str) -> Result<(), String> {
    let output = resolve_writable_data_path(path);
    ensure_parent_dir(&output)?;
    atomic_write(&output, data.as_bytes()).map_err(|err| err.to_string())
}

#[cfg(target_arch = "wasm32")]
fn browser_storage() -> Result<web_sys::Storage, String> {
    web_sys::window()
        .ok_or("Preset storage requires a browser window")?
        .local_storage()
        .map_err(|err| format!("Browser storage unavailable: {err:?}"))?
        .ok_or_else(|| "Browser storage is disabled".to_owned())
}

#[cfg(target_arch = "wasm32")]
fn read_saved_presets(path: &str) -> Result<Option<String>, String> {
    browser_storage()?
        .get_item(&format!("HackmasterSim/{path}"))
        .map_err(|err| format!("Cannot read saved presets: {err:?}"))
}

#[cfg(target_arch = "wasm32")]
fn write_presets(path: &str, data: &str) -> Result<(), String> {
    browser_storage()?
        .set_item(&format!("HackmasterSim/{path}"), data)
        .map_err(|err| format!("Cannot save presets in this browser: {err:?}"))
}

fn read_presets(path: &str, bundled: &str) -> Result<String, String> {
    Ok(read_saved_presets(path)?.unwrap_or_else(|| bundled.to_owned()))
}

/// Keep the simulator usable without discarding or replacing unreadable user data.
fn recover_presets<T>(
    path: &str,
    bundled: &str,
    parse: fn(&str) -> Result<T, String>,
) -> (T, Option<String>) {
    match read_presets(path, bundled).and_then(|data| parse(&data)) {
        Ok(presets) => (presets, None),
        Err(err) => (
            parse(bundled).expect("Bundled presets must be valid"),
            Some(format!(
                "Could not load {path}: {err}. Using bundled presets; the original saved data is unchanged."
            )),
        ),
    }
}

/// A recovery save must retain malformed/unsupported data before replacing it.
/// Reuse an identical backup; never overwrite an earlier, different recovery copy.
fn preserve_invalid_presets<T>(
    path: &str,
    parse: fn(&str) -> Result<T, String>,
) -> Result<(), String> {
    let Some(data) = read_saved_presets(path)? else {
        return Ok(());
    };
    if parse(&data).is_ok() {
        return Ok(());
    }
    for index in 0..32 {
        let backup = format!("{path}.corrupt-{index}");
        match read_saved_presets(&backup)? {
            Some(existing) if existing == data => return Ok(()),
            Some(_) => continue,
            None => {
                return write_presets(&backup, &data).map_err(|err| {
                    format!("Cannot back up damaged presets; original save was not replaced: {err}")
                });
            }
        }
    }
    Err("Cannot back up damaged presets: all recovery slots are occupied. Original save was not replaced.".into())
}

fn mapped_data_subpath(path: &Path) -> PathBuf {
    let stripped = path.strip_prefix("data").unwrap_or(path);
    if stripped.starts_with("sim") {
        return stripped.to_path_buf();
    }
    let Some(file_name) = stripped.file_name().and_then(|name| name.to_str()) else {
        return stripped.to_path_buf();
    };
    match file_name {
        "armor.json"
        | "fighter_presets.json"
        | "npc_presets.json"
        | "races.json"
        | "talents.json"
        | "tactical_presets.json"
        | "weapons.json" => PathBuf::from("sim").join(file_name),
        _ => stripped.to_path_buf(),
    }
}

/// Bundled catalogs are read-only defaults. Saved overrides live outside build output.
struct DataLocations {
    override_dir: Option<PathBuf>,
    user_dir: Option<PathBuf>,
    cwd: Option<PathBuf>,
    executable_dir: Option<PathBuf>,
}
impl DataLocations {
    fn current() -> Self {
        let user_dir = if cfg!(target_os = "windows") {
            env::var_os("LOCALAPPDATA").map(PathBuf::from)
        } else {
            env::var_os("XDG_DATA_HOME").map(PathBuf::from).or_else(|| {
                env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })
        }
        .map(|base| base.join("HackmasterSim").join("data"));
        Self {
            override_dir: env::var_os("HACKMASTER_SIM_DATA_DIR").map(PathBuf::from),
            user_dir,
            cwd: env::current_dir().ok(),
            executable_dir: env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(Path::to_path_buf)),
        }
    }
    fn writable(&self, path: &str) -> PathBuf {
        let raw = Path::new(path);
        if raw.is_absolute() {
            return raw.to_path_buf();
        }
        let base = self
            .override_dir
            .clone()
            .or_else(|| self.user_dir.clone())
            .or_else(|| self.executable_dir.as_ref().map(|p| p.join("user-data")))
            .or_else(|| self.cwd.as_ref().map(|p| p.join("user-data")))
            .unwrap_or_else(|| PathBuf::from("user-data"));
        base.join(mapped_data_subpath(raw))
    }
    fn readable(&self, path: &str) -> PathBuf {
        let raw = Path::new(path);
        if raw.is_absolute() {
            return raw.to_path_buf();
        }
        let mapped = mapped_data_subpath(raw);
        let stripped = raw.strip_prefix("data").unwrap_or(raw);
        // Always prefer the same path that writes target, including legacy aliases.
        let writable = self.writable(path);
        if writable.is_file() {
            return writable;
        }
        let mut candidates = Vec::new();
        if let Some(base) = &self.override_dir {
            candidates.push(base.join(stripped));
        }
        if let Some(cwd) = &self.cwd {
            candidates.push(cwd.join("data").join(&mapped));
            candidates.push(cwd.join("data").join(stripped));
            candidates.push(cwd.join(raw));
        }
        if let Some(base) = &self.executable_dir {
            candidates.push(base.join("data").join(&mapped));
            candidates.push(base.join("data").join(stripped));
        }
        candidates
            .into_iter()
            .find(|p| p.is_file())
            .unwrap_or_else(|| raw.to_path_buf())
    }
}
pub fn resolve_data_path(path: &str) -> PathBuf {
    DataLocations::current().readable(path)
}
pub fn resolve_writable_data_path(path: &str) -> PathBuf {
    DataLocations::current().writable(path)
}

pub fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    Ok(())
}

pub fn load_catalogs() -> Result<(WeaponCatalog, ArmorCatalog, ShieldCatalog), String> {
    let (weapons, shields) = weapons::load_weapon_and_shield_catalogs("data/sim/weapons.json")?;
    let armor = load_armor_catalog("data/sim/armor.json")?;
    Ok((weapons, armor, shields))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrupt_fighter_recovery_preserves_original_and_allows_a_safe_save() {
        let dir = env::temp_dir().join(format!("hackmaster-fighter-recovery-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fighters.json");
        let path = path.to_str().unwrap();
        fs::write(path, "{broken fighter data").unwrap();

        let (presets, warning) = load_fighter_presets_recovering(path);
        assert!(warning.unwrap().contains("Using bundled presets"));
        assert!(presets.entries().iter().any(|p| p.name == "Arthur Du Randt"));
        assert_eq!(fs::read_to_string(path).unwrap(), "{broken fighter data");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);

        save_fighter_presets(path, &presets).unwrap();
        assert_eq!(fs::read_to_string(format!("{path}.corrupt-0")).unwrap(), "{broken fighter data");
        assert!(load_fighter_presets_recovering(path).1.is_none());

        // Retain older recovery copies when a different damaged document appears.
        fs::write(path, "{different broken data").unwrap();
        save_fighter_presets(path, &presets).unwrap();
        assert_eq!(fs::read_to_string(format!("{path}.corrupt-0")).unwrap(), "{broken fighter data");
        assert_eq!(fs::read_to_string(format!("{path}.corrupt-1")).unwrap(), "{different broken data");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn tactical_recovery_preserves_unsupported_schema_before_replacement() {
        let dir = env::temp_dir().join(format!("hackmaster-tactical-recovery-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tactics.json");
        let path = path.to_str().unwrap();
        let original = r#"{"schema_version":999,"presets":[]}"#;
        fs::write(path, original).unwrap();
        let (presets, warning) = load_tactical_presets_recovering(path);
        assert!(warning.unwrap().contains("Unsupported tactical preset schema"));
        assert!(!presets.is_empty());
        assert_eq!(fs::read_to_string(path).unwrap(), original);
        save_tactical_presets(path, &presets).unwrap();
        assert_eq!(fs::read_to_string(format!("{path}.corrupt-0")).unwrap(), original);
        assert_eq!(load_tactical_presets(path).unwrap(), presets);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failed_recovery_backup_does_not_replace_damaged_presets() {
        let dir = env::temp_dir().join(format!("hackmaster-recovery-failure-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fighters.json");
        let path = path.to_str().unwrap();
        fs::write(path, "{recoverable bytes").unwrap();
        // An unreadable backup destination must fail closed, without replacing it.
        fs::create_dir(format!("{path}.corrupt-0")).unwrap();
        let (presets, warning) = load_fighter_presets_recovering(path);
        assert!(warning.is_some());
        assert!(save_fighter_presets(path, &presets).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "{recoverable bytes");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn unreadable_saved_presets_are_not_treated_as_missing() {
        let dir = env::temp_dir().join(format!("hackmaster-unreadable-presets-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fighters.json");
        let path = path.to_str().unwrap();
        let original = [0xff, 0xfe];
        fs::write(path, original).unwrap();
        assert!(load_fighter_presets(path).is_err());
        let (presets, warning) = load_fighter_presets_recovering(path);
        assert!(warning.is_some());
        assert!(!presets.is_empty());
        assert!(save_fighter_presets(path, &presets).is_err());
        assert_eq!(fs::read(path).unwrap(), original);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn empty_fighter_collection_can_be_saved_and_populated() {
        let dir = env::temp_dir().join(format!("hackmaster-empty-presets-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fighters.json");
        let path = path.to_str().unwrap();
        fs::write(path, r#"{"presets":[]}"#).unwrap();
        let (mut presets, warning) = load_fighter_presets_recovering(path);
        assert!(warning.is_none());
        assert!(presets.is_empty());
        let fixture = crate::test_support::fighter_presets();
        presets.push(fixture.entries()[0].clone());
        save_fighter_presets(path, &presets).unwrap();
        assert_eq!(load_fighter_presets(path).unwrap().entries().len(), 1);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn saved_presets_override_bundled_data_across_cwd_and_rebuilds() {
        let root = env::temp_dir().join(format!("hackmaster-paths-{}", std::process::id()));
        let mut locations = DataLocations {
            override_dir: None,
            user_dir: Some(root.join("user")),
            cwd: Some(root.join("repo")),
            executable_dir: Some(root.join("build")),
        };
        let name = "data/sim/fighter_presets.json";
        let bundled = root.join("repo/data/sim/fighter_presets.json");
        atomic_write(&bundled, b"bundled").unwrap();
        assert_eq!(locations.readable(name), bundled);
        let saved = locations.writable(name);
        atomic_write(&saved, b"saved").unwrap();
        assert_eq!(locations.readable(name), saved);
        assert_eq!(locations.readable("data/fighter_presets.json"), saved);
        atomic_write(
            &root.join("build/data/sim/fighter_presets.json"),
            b"rebuilt",
        )
        .unwrap();
        locations.cwd = None;
        assert_eq!(fs::read(locations.readable(name)).unwrap(), b"saved");
        locations.override_dir = Some(root.join("override"));
        assert_eq!(
            locations.writable(name),
            root.join("override/sim/fighter_presets.json")
        );
        let absolute = root.join("explicit.json");
        assert_eq!(locations.writable(absolute.to_str().unwrap()), absolute);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn fighter_and_tactical_saves_round_trip_through_atomic_replacement() {
        let dir = env::temp_dir().join(format!(
            "hackmaster-preset-roundtrip-{}",
            std::process::id()
        ));
        let fighter_path = dir.join("fighter_presets.json");
        let tactical_path = dir.join("tactical_presets.json");
        let fighters = crate::test_support::fighter_presets();
        let tactics = crate::test_support::tactical_presets();
        for _ in 0..2 {
            save_fighter_presets(fighter_path.to_str().unwrap(), &fighters).unwrap();
            save_tactical_presets(tactical_path.to_str().unwrap(), &tactics).unwrap();
            let loaded = load_fighter_presets(fighter_path.to_str().unwrap()).unwrap();
            assert_eq!(
                serde_json::to_value(loaded.entries()).unwrap(),
                serde_json::to_value(fighters.entries()).unwrap()
            );
            assert_eq!(
                load_tactical_presets(tactical_path.to_str().unwrap()).unwrap(),
                tactics
            );
        }
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 2);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn maps_legacy_sim_paths_into_sim_namespace() {
        let mapped = mapped_data_subpath(Path::new("data/weapons.json"));
        assert_eq!(mapped, PathBuf::from("sim").join("weapons.json"));
    }
}
