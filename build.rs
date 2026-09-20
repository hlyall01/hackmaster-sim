use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

struct BinIcon {
    bin: &'static str,
    ico: &'static str,
}

const ICONS: [BinIcon; 4] = [
    BinIcon {
        bin: "sim_gui",
        ico: "assets/icon_sim_gui.ico",
    },
    BinIcon {
        bin: "hackmaster_sim",
        ico: "assets/icon_weapon_plot.ico",
    },
    BinIcon {
        bin: "sim_cli",
        ico: "assets/icon_sim_cli.ico",
    },
    BinIcon {
        bin: "autobattler",
        ico: "assets/icon_autobattler.ico",
    },
];

fn main() {
    println!("cargo:rerun-if-changed=data");
    if let Ok(entries) = fs::read_dir("data") {
        for entry in entries.flatten() {
            println!("cargo:rerun-if-changed={}", entry.path().display());
        }
    }

    if let Err(err) = sync_data_dir() {
        eprintln!("Failed to sync data directory: {err}");
    }

    for icon in ICONS.iter() {
        println!("cargo:rerun-if-changed={}", icon.ico);
    }

    let target = env::var("TARGET").unwrap_or_default();
    if !target.contains("windows") {
        return;
    }

    let mut target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_env.is_empty() {
        let lower = target.to_lowercase();
        if lower.contains("msvc") {
            target_env = "msvc".to_string();
        } else if lower.contains("gnu") || lower.contains("mingw") {
            target_env = "gnu".to_string();
        }
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let use_staging = is_unc_path(&out_dir) || is_unc_path(&manifest_dir);
    let staging_dir = if use_staging {
        let dir = env::temp_dir().join("hackmaster_sim_icons");
        if let Err(err) = fs::create_dir_all(&dir) {
            panic!(
                "Failed to create icon staging dir {}: {}",
                dir.display(),
                err
            );
        }
        dir
    } else {
        out_dir.clone()
    };

    for icon in ICONS.iter() {
        let icon_path = manifest_dir.join(icon.ico);
        if !icon_path.exists() {
            panic!("Missing icon file: {}", icon_path.display());
        }
        let icon_for_rc = if use_staging {
            let staged_icon = staging_dir.join(format!("icon_{}.ico", icon.bin));
            if let Err(err) = fs::copy(&icon_path, &staged_icon) {
                panic!("Failed to stage icon {}: {}", staged_icon.display(), err);
            }
            staged_icon
        } else {
            icon_path.clone()
        };
        let rc_path = staging_dir.join(format!("icon_{}.rc", icon.bin));
        if let Err(err) = write_rc(&rc_path, &icon_for_rc) {
            panic!("Failed to write {}: {}", rc_path.display(), err);
        }
        let obj_path = match target_env.as_str() {
            "msvc" => match compile_msvc(&rc_path, &staging_dir, icon.bin) {
                Ok(path) => path,
                Err(err) => panic!("Failed to compile {}: {}", icon.bin, err),
            },
            "gnu" => match compile_gnu(&rc_path, &staging_dir, icon.bin) {
                Ok(path) => path,
                Err(err) => panic!("Failed to compile {}: {}", icon.bin, err),
            },
            other => {
                eprintln!("Skipping Windows icon embed for unsupported target_env: {other}");
                continue;
            }
        };
        println!(
            "cargo:rustc-link-arg-bin={bin}={obj}",
            bin = icon.bin,
            obj = obj_path.display()
        );
    }
}

fn sync_data_dir() -> io::Result<()> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let source = manifest_dir.join("data");
    if !source.exists() {
        return Ok(());
    }

    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "OUT_DIR missing parents"))?;
    let target_data_dir = profile_dir.join("data");
    copy_dir_recursive(&source, &target_data_dir)?;
    Ok(())
}

fn copy_dir_recursive(source: &Path, target: &Path) -> io::Result<()> {
    if !target.exists() {
        fs::create_dir_all(target)?;
    }
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        let target_path = target.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&path, &target_path)?;
        } else if file_type.is_file() {
            fs::copy(&path, &target_path)?;
        }
    }
    Ok(())
}

fn write_rc(rc_path: &Path, icon_path: &Path) -> io::Result<()> {
    let icon = icon_path.to_string_lossy().replace('\\', "\\\\");
    let rc = format!("1 ICON \"{icon}\"\n");
    fs::write(rc_path, rc)
}

fn is_unc_path(path: &Path) -> bool {
    path.to_string_lossy().starts_with(r"\\")
}

fn compile_msvc(rc_path: &Path, out_dir: &Path, bin: &str) -> io::Result<PathBuf> {
    let res_path = out_dir.join(format!("icon_{bin}.res"));
    let args = [
        "/nologo".to_string(),
        format!("/fo{}", res_path.display()),
        rc_path.display().to_string(),
    ];
    run_tool(&["rc.exe"], &args).map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
    Ok(res_path)
}

fn compile_gnu(rc_path: &Path, out_dir: &Path, bin: &str) -> io::Result<PathBuf> {
    let obj_path = out_dir.join(format!("icon_{bin}.o"));
    let args = [
        "-i".to_string(),
        rc_path.display().to_string(),
        "-O".to_string(),
        "coff".to_string(),
        "-o".to_string(),
        obj_path.display().to_string(),
    ];
    run_tool(&["x86_64-w64-mingw32-windres", "windres"], &args)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
    Ok(obj_path)
}

fn run_tool(candidates: &[&str], args: &[String]) -> Result<(), String> {
    let mut last_err = None;
    for tool in candidates {
        match Command::new(tool).args(args).status() {
            Ok(status) => {
                if status.success() {
                    return Ok(());
                }
                return Err(format!("{} failed with status {}", tool, status));
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                last_err = Some(err);
                continue;
            }
            Err(err) => return Err(format!("Failed to run {}: {}", tool, err)),
        }
    }
    Err(format!(
        "Required tool not found (tried: {}). Last error: {:?}",
        candidates.join(", "),
        last_err
    ))
}
