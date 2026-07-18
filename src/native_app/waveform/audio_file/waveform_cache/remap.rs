use std::{
    fs, io,
    path::{Path, PathBuf},
};

use super::{
    format::CachedWaveformFile,
    identity::{
        CacheIdentity, cache_path_for_identity, playback_descriptor_path,
        playback_ready_marker_path, playback_sidecar_path, playback_sidecar_valid,
        source_warm_marker_path,
    },
    write::{
        mark_source_warm_ready_for_cache_path, update_playback_ready_marker,
        write_playback_descriptor_sidecar,
    },
};

pub(in crate::native_app) fn remap_persisted_waveform_cache_after_move(
    old_path: &Path,
    new_path: &Path,
) -> usize {
    if new_path.is_dir() {
        return remap_directory(old_path, new_path);
    }
    if !new_path.is_file() {
        return 0;
    }
    match remap_file(old_path, new_path) {
        Ok(remapped) => usize::from(remapped),
        Err(err) => {
            tracing::debug!(
                target: "wavecrate::debug::sample_cache",
                event = "browser.sample_cache.move_remap_failed",
                old_path = %old_path.display(),
                new_path = %new_path.display(),
                error = %err,
                "Waveform cache could not be remapped after file move"
            );
            0
        }
    }
}

/// Remap an exact reverse-owned cache payload after a committed path-only move.
///
/// Unlike the legacy path-only entrypoint, this remains usable after the old source path has
/// disappeared because the caller supplies the previously persisted cache reference.
pub(in crate::native_app) fn remap_persisted_waveform_cache_ref_after_move(
    old_cache_ref: &Path,
    old_path: &Path,
    new_path: &Path,
) -> Option<PathBuf> {
    if !super::identity::cache_ref_is_managed(old_cache_ref) || !new_path.is_file() {
        return None;
    }
    match remap_file_from_cache_ref(old_cache_ref, old_path, new_path) {
        Ok(cache_ref) => cache_ref,
        Err(err) => {
            tracing::debug!(
                target: "wavecrate::debug::sample_cache",
                event = "browser.sample_cache.owned_move_remap_failed",
                old_cache_ref = %old_cache_ref.display(),
                old_path = %old_path.display(),
                new_path = %new_path.display(),
                error = %err,
                "Reverse-owned waveform cache could not be remapped after file move"
            );
            None
        }
    }
}

fn remap_directory(old_dir: &Path, new_dir: &Path) -> usize {
    let Ok(entries) = fs::read_dir(new_dir) else {
        return 0;
    };
    let mut remapped = 0;
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let new_child = entry.path();
        let old_child = old_dir.join(entry.file_name());
        if file_type.is_dir() {
            remapped += remap_directory(&old_child, &new_child);
        } else if file_type.is_file() {
            remapped += remap_persisted_waveform_cache_after_move(&old_child, &new_child);
        }
    }
    remapped
}

fn remap_file(old_path: &Path, new_path: &Path) -> Result<bool, String> {
    let identity = CacheIdentity::for_path(new_path)?;
    let old_cache_path = cache_path_for_identity(old_path, &identity)?;
    if !old_cache_path.is_file() {
        return Ok(false);
    }
    remap_file_from_cache_ref(&old_cache_path, old_path, new_path)
        .map(|cache_ref| cache_ref.is_some())
}

fn remap_file_from_cache_ref(
    old_cache_path: &Path,
    old_path: &Path,
    new_path: &Path,
) -> Result<Option<PathBuf>, String> {
    if !old_cache_path.is_file() {
        return Ok(None);
    }
    let identity = CacheIdentity::for_path(new_path)?;
    let Some(cached) = super::read::read_cached_waveform_file_at_ref(old_path, old_cache_path)
    else {
        return Ok(None);
    };
    let new_cache_path = cache_path_for_identity(new_path, &identity)?;
    let Some(mut moved_cache) = cached
        .clone()
        .into_moved_path(old_path, new_path, &identity)
    else {
        return Ok(None);
    };

    let sidecar_paths = valid_playback_sidecar_paths(&cached, &old_cache_path, &new_cache_path);
    let playback_sidecar_moved =
        sidecar_paths
            .as_ref()
            .is_some_and(|(old_sidecar, new_sidecar)| {
                move_cache_artifact(old_sidecar, new_sidecar).is_ok()
            });
    if !playback_sidecar_moved {
        moved_cache.clear_playback_cache();
    }

    if let Err(err) = write_remapped_cache_file(&new_cache_path, &moved_cache) {
        if playback_sidecar_moved && let Some((old_sidecar, new_sidecar)) = sidecar_paths {
            let _ = move_cache_artifact(&new_sidecar, &old_sidecar);
        }
        return Err(err);
    }

    mark_source_warm_ready_for_cache_path(&new_cache_path);
    if playback_sidecar_moved {
        let _ = write_playback_descriptor_sidecar(&new_cache_path, &moved_cache);
        let _ = update_playback_ready_marker(&new_cache_path, true);
    } else {
        let _ = remove_file_if_exists(&playback_descriptor_path(&new_cache_path));
        let _ = update_playback_ready_marker(&new_cache_path, false);
    }
    cleanup_old_cache_artifacts(&old_cache_path);
    Ok(Some(new_cache_path))
}

fn valid_playback_sidecar_paths(
    cached: &CachedWaveformFile,
    old_cache_path: &Path,
    new_cache_path: &Path,
) -> Option<(PathBuf, PathBuf)> {
    let playback_cache = cached.playback_cache.as_ref()?;
    let old_sidecar = playback_sidecar_path(old_cache_path);
    playback_sidecar_valid(&old_sidecar, playback_cache.sample_count).then(|| {
        let new_sidecar = playback_sidecar_path(new_cache_path);
        (old_sidecar, new_sidecar)
    })
}

fn write_remapped_cache_file(cache_path: &Path, cached: &CachedWaveformFile) -> Result<(), String> {
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let bytes = bincode::serialize(cached).map_err(|err| err.to_string())?;
    let temp_path = cache_path.with_extension("tmp");
    fs::write(&temp_path, bytes).map_err(|err| err.to_string())?;
    move_cache_artifact(&temp_path, cache_path).map_err(|err| err.to_string())
}

fn move_cache_artifact(source: &Path, destination: &Path) -> io::Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    remove_file_if_exists(destination)?;
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(rename_err) => match fs::copy(source, destination) {
            Ok(_) => {
                remove_file_if_exists(source)?;
                Ok(())
            }
            Err(_) => Err(rename_err),
        },
    }
}

fn cleanup_old_cache_artifacts(old_cache_path: &Path) {
    let _ = remove_file_if_exists(old_cache_path);
    let _ = remove_file_if_exists(&playback_descriptor_path(old_cache_path));
    let _ = remove_file_if_exists(&playback_sidecar_path(old_cache_path));
    let _ = remove_file_if_exists(&playback_ready_marker_path(old_cache_path));
    let _ = remove_file_if_exists(&source_warm_marker_path(old_cache_path));
}

fn remove_file_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}
