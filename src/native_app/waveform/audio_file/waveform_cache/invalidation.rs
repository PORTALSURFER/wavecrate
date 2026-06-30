use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex},
};

use super::identity::{
    CacheIdentity, cache_path_for_identity, playback_ready_marker_path, playback_sidecar_path,
    source_warm_marker_path,
};

static CACHE_PATH_GENERATIONS: LazyLock<Mutex<HashMap<PathBuf, u64>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(in crate::native_app) fn invalidate_persisted_waveform_cache_path(path: &Path) {
    if let Ok(mut generations) = CACHE_PATH_GENERATIONS.lock() {
        let generation = generations.entry(path.to_path_buf()).or_insert(0);
        *generation = generation.saturating_add(1);
        cleanup_current_identity_artifacts(path);
    }
}

pub(in crate::native_app) fn invalidate_persisted_waveform_cache_paths(paths: &[PathBuf]) {
    for path in paths {
        invalidate_persisted_waveform_cache_path(path);
    }
}

pub(super) fn current_path_generation(path: &Path) -> u64 {
    CACHE_PATH_GENERATIONS
        .lock()
        .ok()
        .and_then(|generations| generations.get(path).copied())
        .unwrap_or(0)
}

pub(super) fn store_job_matches_current_file(
    path: &Path,
    identity: &CacheIdentity,
    generation: u64,
) -> bool {
    generation == current_path_generation(path)
        && CacheIdentity::for_path(path).is_ok_and(|current| current == *identity)
}

pub(super) fn commit_if_store_job_current<T>(
    path: &Path,
    identity: &CacheIdentity,
    generation: u64,
    commit: impl FnOnce() -> T,
) -> Option<T> {
    let Ok(generations) = CACHE_PATH_GENERATIONS.lock() else {
        return None;
    };
    let current_generation = generations.get(path).copied().unwrap_or(0);
    if current_generation != generation
        || !CacheIdentity::for_path(path).is_ok_and(|current| current == *identity)
    {
        return None;
    }
    Some(commit())
}

fn cleanup_current_identity_artifacts(path: &Path) {
    let Ok(identity) = CacheIdentity::for_path(path) else {
        return;
    };
    let Ok(cache_path) = cache_path_for_identity(path, &identity) else {
        return;
    };
    cleanup_cache_artifacts(&cache_path);
}

pub(super) fn cleanup_cache_artifacts(cache_path: &Path) {
    remove_file_if_exists(cache_path);
    remove_file_if_exists(&playback_sidecar_path(cache_path));
    remove_file_if_exists(&playback_ready_marker_path(cache_path));
    remove_file_if_exists(&source_warm_marker_path(cache_path));
}

fn remove_file_if_exists(path: &Path) {
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {}
    }
}
