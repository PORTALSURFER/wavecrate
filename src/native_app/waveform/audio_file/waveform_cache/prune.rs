use std::{
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use super::identity::{
    playback_descriptor_path, playback_ready_marker_path, playback_sidecar_path,
    source_warm_marker_path,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct PruneWaveformCacheOutcome {
    pub(super) read_dir_failed: bool,
    pub(super) directory_scans: usize,
    pub(super) entries_examined: usize,
    pub(super) metadata_probes: usize,
    pub(super) stale_temp_removed: usize,
    pub(super) stale_temp_remove_failed: usize,
    pub(super) orphan_sidecar_removed: usize,
    pub(super) orphan_sidecar_remove_failed: usize,
    pub(super) orphan_marker_removed: usize,
    pub(super) orphan_marker_remove_failed: usize,
    pub(super) cache_removed: usize,
    pub(super) cache_remove_failed: usize,
    pub(super) companion_remove_failed: usize,
    pub(super) bytes_before: u64,
    pub(super) bytes_after: u64,
}

pub(super) fn prune_waveform_cache_dir(
    cache_dir: &Path,
    pinned_path: Option<&Path>,
    max_bytes: u64,
) -> PruneWaveformCacheOutcome {
    let mut outcome = PruneWaveformCacheOutcome::default();
    let Ok(entries) = fs::read_dir(cache_dir) else {
        outcome.read_dir_failed = true;
        return outcome;
    };
    outcome.directory_scans = 1;
    let mut cache_entries = Vec::new();
    let mut total_bytes = 0_u64;

    for entry in entries.flatten() {
        outcome.entries_examined += 1;
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "tmp") {
            if fs::remove_file(path).is_ok() {
                outcome.stale_temp_removed += 1;
            } else {
                outcome.stale_temp_remove_failed += 1;
            }
            continue;
        }
        if path.extension().is_some_and(|extension| extension == "pcm") {
            outcome.metadata_probes += 1;
            if !path.with_extension("wfc").is_file() {
                if fs::remove_file(path).is_ok() {
                    outcome.orphan_sidecar_removed += 1;
                } else {
                    outcome.orphan_sidecar_remove_failed += 1;
                }
            }
            continue;
        }
        if path.extension().is_some_and(|extension| {
            extension == "ready" || extension == "playback" || extension == "source-ready"
        }) {
            outcome.metadata_probes += 1;
            if !path.with_extension("wfc").is_file() {
                if fs::remove_file(path).is_ok() {
                    outcome.orphan_marker_removed += 1;
                } else {
                    outcome.orphan_marker_remove_failed += 1;
                }
            }
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "wfc") {
            continue;
        }
        outcome.metadata_probes += 1;
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let sidecar_path = playback_sidecar_path(&path);
        outcome.metadata_probes += 1;
        let sidecar_len = sidecar_path
            .metadata()
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let descriptor_path = playback_descriptor_path(&path);
        outcome.metadata_probes += 1;
        let descriptor_len = descriptor_path
            .metadata()
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        let len = metadata
            .len()
            .saturating_add(sidecar_len)
            .saturating_add(descriptor_len);
        total_bytes = total_bytes.saturating_add(len);
        cache_entries.push(CacheFileForPrune {
            path,
            len,
            modified: metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        });
    }
    outcome.bytes_before = total_bytes;
    outcome.bytes_after = total_bytes;

    if total_bytes <= max_bytes {
        return outcome;
    }

    cache_entries.sort_by_key(|entry| entry.modified);
    for entry in cache_entries {
        if total_bytes <= max_bytes {
            break;
        }
        if pinned_path.is_some_and(|pinned_path| entry.path == pinned_path) {
            continue;
        }
        if fs::remove_file(&entry.path).is_ok() {
            if remove_if_exists(playback_ready_marker_path(&entry.path)).is_err() {
                outcome.companion_remove_failed += 1;
            }
            if remove_if_exists(playback_descriptor_path(&entry.path)).is_err() {
                outcome.companion_remove_failed += 1;
            }
            if remove_if_exists(playback_sidecar_path(&entry.path)).is_err() {
                outcome.companion_remove_failed += 1;
            }
            if remove_if_exists(source_warm_marker_path(&entry.path)).is_err() {
                outcome.companion_remove_failed += 1;
            }
            total_bytes = total_bytes.saturating_sub(entry.len);
            outcome.cache_removed += 1;
        } else {
            outcome.cache_remove_failed += 1;
        }
    }
    outcome.bytes_after = total_bytes;
    outcome
}

impl PruneWaveformCacheOutcome {
    pub(super) fn has_failures(&self) -> bool {
        self.read_dir_failed
            || self.stale_temp_remove_failed > 0
            || self.orphan_sidecar_remove_failed > 0
            || self.orphan_marker_remove_failed > 0
            || self.cache_remove_failed > 0
            || self.companion_remove_failed > 0
    }
}

#[derive(Debug)]
struct CacheFileForPrune {
    path: PathBuf,
    len: u64,
    modified: SystemTime,
}

fn remove_if_exists(path: PathBuf) -> Result<(), ()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(()),
    }
}
