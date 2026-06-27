use radiant::prelude as ui;
use std::{
    cell::RefCell,
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use crate::native_app::{
    app::{
        ActiveFolderCacheWarmPlanProgress, ActiveFolderCacheWarmPlanResult,
        ActiveFolderCacheWarmProgress, ActiveFolderCacheWarmRequest, ActiveFolderCacheWarmResult,
        ActiveFolderCacheWarmStage, WaveformCacheIndicatorRefreshResult, WaveformCacheWarmResult,
        WaveformState,
    },
    audio::sample_load_actions::cache::ACTIVE_FOLDER_CACHE_WARM_HYDRATE_MAX_FILES,
    waveform::{
        cached_waveform_file_audition_ready_exists, cached_waveform_file_exists,
        load_cached_waveform_file_for_playback, mark_cached_waveform_file_source_warm_attempted,
    },
};

const ACTIVE_FOLDER_CACHE_PROGRESS_MIN_INTERVAL: Duration = Duration::from_millis(50);
const ACTIVE_FOLDER_CACHE_PROGRESS_MIN_DELTA: f32 = 0.01;
const ACTIVE_FOLDER_CACHE_PLAN_PROGRESS_MIN_INTERVAL: Duration = Duration::from_millis(50);
const ACTIVE_FOLDER_CACHE_PLAN_PROGRESS_MIN_DELTA: f32 = 0.005;
const ACTIVE_FOLDER_CACHE_LOADING_PROGRESS: f32 = 0.18;
const ACTIVE_FOLDER_CACHE_DECODE_PROGRESS_START: f32 = 0.2;
const ACTIVE_FOLDER_CACHE_DECODE_PROGRESS_RANGE: f32 = 0.795;

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

#[cfg(test)]
pub(in crate::native_app) fn warm_active_folder_waveform_cache(
    request: ActiveFolderCacheWarmRequest,
    is_cancelled: impl Fn() -> bool,
) -> ActiveFolderCacheWarmResult {
    warm_active_folder_waveform_cache_with_progress(request, is_cancelled, |_| {})
}

#[cfg(test)]
pub(in crate::native_app) fn plan_active_folder_waveform_cache_warm(
    request: ActiveFolderCacheWarmRequest,
    is_cancelled: impl Fn() -> bool,
) -> ActiveFolderCacheWarmPlanResult {
    plan_active_folder_waveform_cache_warm_with_progress(request, is_cancelled, |_| {})
}

pub(in crate::native_app) fn plan_active_folder_waveform_cache_warm_with_progress(
    request: ActiveFolderCacheWarmRequest,
    is_cancelled: impl Fn() -> bool,
    progress: impl Fn(ActiveFolderCacheWarmPlanProgress),
) -> ActiveFolderCacheWarmPlanResult {
    let ActiveFolderCacheWarmRequest { folder_id, paths } = request;
    let total = paths.len();
    let mut progress_gate = ui::ProgressUpdateGate::new(
        ACTIVE_FOLDER_CACHE_PLAN_PROGRESS_MIN_INTERVAL,
        ACTIVE_FOLDER_CACHE_PLAN_PROGRESS_MIN_DELTA,
    );
    let playback_ready = Vec::new();
    let mut pending = Vec::new();
    for (index, path) in paths.into_iter().enumerate() {
        let checked = index.saturating_add(1);
        if is_cancelled() {
            pending.push(path);
            return ActiveFolderCacheWarmPlanResult {
                folder_id,
                playback_ready,
                pending,
                cancelled: true,
            };
        }
        report_active_folder_cache_plan_progress(
            &folder_id,
            &path,
            checked,
            total,
            false,
            &mut progress_gate,
            &progress,
        );
        pending.push(path);
    }
    ActiveFolderCacheWarmPlanResult {
        folder_id,
        playback_ready,
        pending,
        cancelled: false,
    }
}

fn report_active_folder_cache_plan_progress(
    folder_id: &str,
    path: &Path,
    checked: usize,
    total: usize,
    playback_ready: bool,
    progress_gate: &mut ui::ProgressUpdateGate,
    progress: &impl Fn(ActiveFolderCacheWarmPlanProgress),
) {
    let fraction = if total == 0 {
        1.0
    } else {
        checked as f32 / total as f32
    };
    if progress_gate.accept(fraction).is_none() {
        return;
    }
    progress(ActiveFolderCacheWarmPlanProgress {
        folder_id: folder_id.to_owned(),
        path: path.to_path_buf(),
        checked,
        total,
        playback_ready,
    });
}

pub(super) fn warm_active_folder_waveform_cache_with_progress(
    request: ActiveFolderCacheWarmRequest,
    is_cancelled: impl Fn() -> bool,
    progress: impl Fn(ActiveFolderCacheWarmProgress),
) -> ActiveFolderCacheWarmResult {
    let ActiveFolderCacheWarmRequest { folder_id, paths } = request;
    let mut paths = paths.into_iter();
    let mut loaded = Vec::new();
    let playback_ready = Vec::new();
    let mut deferred = Vec::new();
    let mut processed = 0;
    let mut decoded_source = false;
    let mut hydrated_cache_files = 0;
    while let Some(path) = paths.next() {
        if is_cancelled() {
            deferred.push(path);
            break;
        }
        report_active_folder_cache_progress(
            &folder_id,
            &path,
            processed,
            0.0,
            ActiveFolderCacheWarmStage::CheckingCache,
            false,
            &progress,
        );
        report_active_folder_cache_progress(
            &folder_id,
            &path,
            processed,
            ACTIVE_FOLDER_CACHE_LOADING_PROGRESS,
            ActiveFolderCacheWarmStage::LoadingCache,
            false,
            &progress,
        );
        if let Some(file) = load_cached_waveform_file_for_playback(path.clone()) {
            processed += 1;
            report_active_folder_cache_progress(
                &folder_id,
                &path,
                processed,
                1.0,
                ActiveFolderCacheWarmStage::Ready,
                true,
                &progress,
            );
            loaded.push((path, Arc::new(file)));
            hydrated_cache_files += 1;
            if hydrated_cache_files >= ACTIVE_FOLDER_CACHE_WARM_HYDRATE_MAX_FILES {
                deferred.extend(paths);
                break;
            }
            continue;
        }
        let progress_gate = RefCell::new(ui::ProgressUpdateGate::new(
            ACTIVE_FOLDER_CACHE_PROGRESS_MIN_INTERVAL,
            ACTIVE_FOLDER_CACHE_PROGRESS_MIN_DELTA,
        ));
        let decode_path = path.clone();
        match WaveformState::load_path_with_progress_and_cancel(
            path.clone(),
            |fraction| {
                let Some(fraction) = progress_gate.borrow_mut().accept(fraction) else {
                    return;
                };
                let file_progress = ACTIVE_FOLDER_CACHE_DECODE_PROGRESS_START
                    + fraction * ACTIVE_FOLDER_CACHE_DECODE_PROGRESS_RANGE;
                report_active_folder_cache_progress(
                    &folder_id,
                    &decode_path,
                    processed,
                    file_progress,
                    ActiveFolderCacheWarmStage::Decoding,
                    false,
                    &progress,
                );
            },
            &is_cancelled,
        ) {
            Ok(waveform) => {
                mark_cached_waveform_file_source_warm_attempted(&path);
                loaded.push((path.clone(), waveform.file()));
                processed += 1;
                report_active_folder_cache_progress(
                    &folder_id,
                    &decode_path,
                    processed,
                    1.0,
                    ActiveFolderCacheWarmStage::Ready,
                    true,
                    &progress,
                );
            }
            Err(_) if is_cancelled() => {
                deferred.push(path.clone());
            }
            Err(_) => {
                mark_cached_waveform_file_source_warm_attempted(&path);
                processed += 1;
            }
        }
        decoded_source = true;
        deferred.extend(paths);
        break;
    }
    ActiveFolderCacheWarmResult {
        folder_id,
        loaded,
        playback_ready,
        deferred,
        processed,
        decoded_source,
        cancelled: is_cancelled(),
    }
}

fn report_active_folder_cache_progress(
    folder_id: &str,
    path: &Path,
    processed: usize,
    current_progress: f32,
    stage: ActiveFolderCacheWarmStage,
    cached: bool,
    progress: &impl Fn(ActiveFolderCacheWarmProgress),
) {
    progress(ActiveFolderCacheWarmProgress {
        folder_id: folder_id.to_owned(),
        path: path.to_path_buf(),
        processed,
        current_progress: current_progress.clamp(0.0, 1.0),
        stage,
        cached,
    });
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
        if cached_waveform_file_audition_ready_exists(path) {
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
