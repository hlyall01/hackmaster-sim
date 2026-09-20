use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

/// Replace a save only after its complete contents have reached the temporary file.
/// The temporary sibling keeps the rename on the same filesystem (also on Windows).
pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    atomic_write_with(path, |file| file.write_all(bytes))
}
fn atomic_write_with(
    path: &Path,
    write: impl FnOnce(&mut fs::File) -> io::Result<()>,
) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "save path needs a filename"))?;
    let mut attempts = 0;
    let (temp, mut file) = loop {
        let id = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let mut temp_name = name.to_os_string();
        temp_name.push(format!(".{}.{}.tmp", std::process::id(), id));
        let temp = parent.join(temp_name);
        match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => break (temp, file),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists && attempts < 16 => {
                attempts += 1;
            }
            Err(err) => return Err(err),
        }
    };
    let result = (|| {
        write(&mut file)?;
        file.sync_all()?;
        if let Ok(metadata) = fs::metadata(path) {
            fs::set_permissions(&temp, metadata.permissions())?;
        }
        drop(file);
        fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_save_keeps_original_and_cleans_temporary_file() {
        let dir = std::env::temp_dir().join(format!(
            "hackmaster-atomic-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("preset.json");
        atomic_write(&path, b"original").unwrap();
        let failure = atomic_write_with(&path, |file| {
            file.write_all(b"partial")?;
            Err(io::Error::other("injected failure"))
        });
        assert!(failure.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        atomic_write(&path, b"replacement").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"replacement");
        fs::remove_dir_all(dir).unwrap();
    }
}
