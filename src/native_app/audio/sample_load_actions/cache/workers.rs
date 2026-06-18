use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::native_app::{
    app::{
        ActiveFolderCacheWarmResult, WaveformCacheIndicatorRefreshResult, WaveformCacheWarmResult,
        WaveformState,
    },
    waveform::{
        cached_waveform_file_exists, cached_waveform_file_playback_ready_exists,
        load_cached_waveform_file_for_playback,
    },
};

use super::ACTIVE_FOLDER_CACHE_WARM_MAX_SOURCE_FILE_BYTES;

pub(in crate::native_app) fn warm_persisted_waveform_cache(
    paths: Vec<PathBuf>,
    is_cancelled: impl Fn() -> bool,
) -> WaveformCacheWarmResult {
    let loaded = paths
        .into_iter()
        .filter_map(|path| {
            if is_cancelled() {
                return None;
            }
            load_cached_waveform_file_for_playback(path.clone())
                .map(Arc::new)
                .map(|file| (path, file))
        })
        .collect();
    WaveformCacheWarmResult { loaded }
}

pub(in crate::native_app) fn warm_active_folder_waveform_cache(
    folder_id: String,
    paths: Vec<PathBuf>,
    is_cancelled: impl Fn() -> bool,
) -> ActiveFolderCacheWarmResult {
    let mut paths = paths.into_iter();
    let mut loaded = Vec::new();
    let mut deferred = Vec::new();
    let mut processed = 0;
    let mut decoded_source = false;
    while let Some(path) = paths.next() {
        if is_cancelled() {
            deferred.push(path);
            break;
        }
        processed += 1;
        if !active_folder_cache_auto_warm_allowed(&path) {
            continue;
        }
        if cached_waveform_file_playback_ready_exists(&path) {
            if let Some(file) = load_cached_waveform_file_for_playback(path.clone()) {
                loaded.push((path, Arc::new(file)));
            }
            continue;
        }
        if let Some(file) = load_cached_waveform_file_for_playback(path.clone()) {
            loaded.push((path, Arc::new(file)));
            decoded_source = true;
            deferred.extend(paths);
            break;
        }
        if let Ok(waveform) =
            WaveformState::load_path_with_progress_and_cancel(path.clone(), |_| {}, &is_cancelled)
        {
            loaded.push((path, waveform.file()));
        }
        decoded_source = true;
        deferred.extend(paths);
        break;
    }
    ActiveFolderCacheWarmResult {
        folder_id,
        loaded,
        deferred,
        processed,
        decoded_source,
        cancelled: is_cancelled(),
    }
}

fn active_folder_cache_auto_warm_allowed(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|metadata| metadata.len() <= ACTIVE_FOLDER_CACHE_WARM_MAX_SOURCE_FILE_BYTES)
        .unwrap_or(false)
}

pub(super) fn probe_persisted_waveform_cache_indicators(
    paths: Vec<PathBuf>,
    is_cancelled: impl Fn() -> bool,
) -> WaveformCacheIndicatorRefreshResult {
    let mut playback_ready_paths = HashSet::new();
    let mut warm_candidate_paths = HashSet::new();
    for path in &paths {
        if is_cancelled() {
            break;
        }
        if cached_waveform_file_playback_ready_exists(path) {
            playback_ready_paths.insert(path.clone());
        } else if cached_waveform_file_exists(path) {
            warm_candidate_paths.insert(path.clone());
        }
    }
    WaveformCacheIndicatorRefreshResult {
        probed_paths: paths,
        playback_ready_paths,
        warm_candidate_paths,
    }
}
