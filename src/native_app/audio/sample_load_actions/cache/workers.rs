use std::{collections::HashSet, path::PathBuf, sync::Arc};

use crate::native_app::{
    app::{WaveformCacheIndicatorRefreshResult, WaveformCacheWarmResult, WaveformState},
    waveform::{
        WaveformFile, cached_waveform_file_exists, cached_waveform_file_playback_ready_exists,
        load_cached_waveform_file_for_playback,
    },
};

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
    paths: Vec<PathBuf>,
    is_cancelled: impl Fn() -> bool,
) -> Vec<(PathBuf, Arc<WaveformFile>)> {
    paths
        .into_iter()
        .filter_map(|path| {
            if is_cancelled() {
                return None;
            }
            if let Some(file) = load_cached_waveform_file_for_playback(path.clone()) {
                return Some((path, Arc::new(file)));
            }
            let waveform = WaveformState::load_path_with_progress_and_cancel(
                path.clone(),
                |_| {},
                &is_cancelled,
            )
            .ok()?;
            Some((path, waveform.file()))
        })
        .collect()
}

pub(super) fn probe_persisted_waveform_cache_indicators(
    paths: Vec<PathBuf>,
) -> WaveformCacheIndicatorRefreshResult {
    let mut playback_ready_paths = HashSet::new();
    let mut warm_candidate_paths = HashSet::new();
    for path in &paths {
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
