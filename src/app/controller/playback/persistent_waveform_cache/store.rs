use super::key::CACHE_FILE_EXTENSION;
use std::ffi::OsStr;
use std::path::Path;

pub(super) fn read_cache_file(path: &Path) -> Option<Vec<u8>> {
    std::fs::read(path).ok()
}

pub(super) fn remove_cache_dir(dir: &Path) {
    if !dir.exists() {
        return;
    }
    if let Err(err) = std::fs::remove_dir_all(dir) {
        tracing::warn!(
            "Failed to remove waveform cache directory {}: {err}",
            dir.display()
        );
    }
}

/// Atomically write one waveform cache payload to disk using a temporary file.
pub(super) fn write_cache_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Err(format!(
            "Waveform cache path has no parent: {}",
            path.display()
        ));
    };
    std::fs::create_dir_all(parent)
        .map_err(|err| format!("Failed to create {}: {err}", parent.display()))?;
    let tmp_path = path.with_extension(format!("{CACHE_FILE_EXTENSION}.tmp"));
    std::fs::write(&tmp_path, bytes)
        .map_err(|err| format!("Failed to write {}: {err}", tmp_path.display()))?;
    if path.exists() {
        std::fs::remove_file(path)
            .map_err(|err| format!("Failed to replace {}: {err}", path.display()))?;
    }
    std::fs::rename(&tmp_path, path)
        .map_err(|err| format!("Failed to rename {}: {err}", path.display()))?;
    Ok(())
}

/// Remove older cache payloads from the same hashed directory after a successful write.
pub(super) fn cleanup_stale_cache_files(current_path: &Path) -> Result<(), String> {
    let Some(parent) = current_path.parent() else {
        return Ok(());
    };
    for entry in std::fs::read_dir(parent)
        .map_err(|err| format!("Failed to read {}: {err}", parent.display()))?
    {
        let entry = entry.map_err(|err| format!("Failed to read dir entry: {err}"))?;
        let path = entry.path();
        if path == current_path {
            continue;
        }
        if path.extension() == Some(OsStr::new(CACHE_FILE_EXTENSION))
            && let Err(err) = std::fs::remove_file(&path)
        {
            return Err(format!("Failed to remove {}: {err}", path.display()));
        }
    }
    Ok(())
}
