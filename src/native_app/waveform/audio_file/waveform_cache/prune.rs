use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use super::identity::{playback_ready_marker_path, playback_sidecar_path};

pub(super) fn prune_waveform_cache_dir(pinned_path: &Path, max_bytes: u64) {
    let Some(cache_dir) = pinned_path.parent() else {
        return;
    };
    let Ok(entries) = fs::read_dir(cache_dir) else {
        return;
    };
    let mut cache_entries = Vec::new();
    let mut total_bytes = 0_u64;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "tmp") {
            let _ = fs::remove_file(path);
            continue;
        }
        if path.extension().is_some_and(|extension| extension == "pcm") {
            if !path.with_extension("wfc").is_file() {
                let _ = fs::remove_file(path);
            }
            continue;
        }
        if path
            .extension()
            .is_some_and(|extension| extension == "ready")
        {
            if !path.with_extension("wfc").is_file() {
                let _ = fs::remove_file(path);
            }
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "wfc") {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let sidecar_path = playback_sidecar_path(&path);
        let sidecar_len = sidecar_path
            .metadata()
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let len = metadata.len().saturating_add(sidecar_len);
        total_bytes = total_bytes.saturating_add(len);
        cache_entries.push(CacheFileForPrune {
            path,
            len,
            modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        });
    }

    if total_bytes <= max_bytes {
        return;
    }

    cache_entries.sort_by_key(|entry| entry.modified);
    for entry in cache_entries {
        if total_bytes <= max_bytes {
            break;
        }
        if entry.path == pinned_path {
            continue;
        }
        if fs::remove_file(&entry.path).is_ok() {
            let _ = fs::remove_file(playback_ready_marker_path(&entry.path));
            let _ = fs::remove_file(playback_sidecar_path(&entry.path));
            total_bytes = total_bytes.saturating_sub(entry.len);
        }
    }
}

#[derive(Debug)]
struct CacheFileForPrune {
    path: PathBuf,
    len: u64,
    modified: SystemTime,
}
