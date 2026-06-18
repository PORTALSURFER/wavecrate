use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
};

use radiant::prelude as ui;

use crate::native_app::app::{
    ActiveFolderCacheWarmPlanProgress, ActiveFolderCacheWarmProgress, ActiveFolderCacheWarmStage,
    SampleSelectionLoadState, WaveformCacheEntry,
};
use crate::native_app::waveform::WaveformState;

pub(in crate::native_app) struct WaveformAppState {
    pub(in crate::native_app) current: WaveformState,
    pub(in crate::native_app) load: WaveformLoadState,
    pub(in crate::native_app) cache: WaveformCacheState,
}

impl WaveformAppState {
    pub(in crate::native_app) fn new(current: WaveformState) -> Self {
        Self {
            current,
            load: WaveformLoadState::default(),
            cache: WaveformCacheState::default(),
        }
    }
}

pub(in crate::native_app) struct WaveformLoadState {
    pub(in crate::native_app) progress: f32,
    pub(in crate::native_app) target_progress: f32,
    pub(in crate::native_app) label: Option<String>,
    pub(in crate::native_app) selection: SampleSelectionLoadState,
}

impl Default for WaveformLoadState {
    fn default() -> Self {
        Self {
            progress: 0.0,
            target_progress: 0.0,
            label: None,
            selection: SampleSelectionLoadState::default(),
        }
    }
}

pub(in crate::native_app) struct WaveformCacheState {
    pub(in crate::native_app) entries: HashMap<PathBuf, WaveformCacheEntry>,
    pub(in crate::native_app) order: VecDeque<PathBuf>,
    pub(in crate::native_app) bytes: usize,
    pub(in crate::native_app) indicator_refresh_task: ui::LatestTask,
    pub(in crate::native_app) warm_pending: VecDeque<PathBuf>,
    pub(in crate::native_app) warm_tasks: ui::ResourceTasks,
    pub(in crate::native_app) warm_key: Option<ui::ResourceKey>,
    pub(in crate::native_app) warm_cancel: Option<ui::CancellationToken>,
    pub(in crate::native_app) active_folder_warm_plan_task: ui::LatestTask,
    pub(in crate::native_app) active_folder_warm_plan_cancel: Option<ui::CancellationToken>,
    pub(in crate::native_app) active_folder_warm_delay_task: ui::LatestTask,
    pub(in crate::native_app) active_folder_warm_tasks: ui::ResourceTasks,
    pub(in crate::native_app) active_folder_warm_key: Option<ui::ResourceKey>,
    pub(in crate::native_app) active_folder_warm_cancel: Option<ui::CancellationToken>,
    pub(in crate::native_app) active_folder_warm_folder_id: Option<String>,
    pub(in crate::native_app) active_folder_warm_pending: VecDeque<PathBuf>,
    pub(in crate::native_app) active_folder_warm_completed: usize,
    pub(in crate::native_app) active_folder_warm_total: usize,
    pub(in crate::native_app) active_folder_warm_current: Option<PathBuf>,
    pub(in crate::native_app) active_folder_warm_current_progress: f32,
    pub(in crate::native_app) active_folder_warm_current_stage: Option<ActiveFolderCacheWarmStage>,
    pub(in crate::native_app) active_folder_warm_batch_base_completed: usize,
    pub(in crate::native_app) cached_sample_paths: HashSet<String>,
}

impl Default for WaveformCacheState {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            order: Default::default(),
            bytes: 0,
            indicator_refresh_task: ui::LatestTask::new(),
            warm_pending: Default::default(),
            warm_tasks: ui::ResourceTasks::new(),
            warm_key: None,
            warm_cancel: None,
            active_folder_warm_plan_task: ui::LatestTask::new(),
            active_folder_warm_plan_cancel: None,
            active_folder_warm_delay_task: ui::LatestTask::new(),
            active_folder_warm_tasks: ui::ResourceTasks::new(),
            active_folder_warm_key: None,
            active_folder_warm_cancel: None,
            active_folder_warm_folder_id: None,
            active_folder_warm_pending: Default::default(),
            active_folder_warm_completed: 0,
            active_folder_warm_total: 0,
            active_folder_warm_current: None,
            active_folder_warm_current_progress: 0.0,
            active_folder_warm_current_stage: None,
            active_folder_warm_batch_base_completed: 0,
            cached_sample_paths: Default::default(),
        }
    }
}

impl WaveformCacheState {
    pub(in crate::native_app) fn start_active_folder_warm_plan(
        &mut self,
        folder_id: String,
        total: usize,
    ) {
        self.active_folder_warm_folder_id = Some(folder_id);
        self.active_folder_warm_pending.clear();
        self.active_folder_warm_completed = 0;
        self.active_folder_warm_total = total;
        self.active_folder_warm_current = None;
        self.active_folder_warm_current_progress = 0.0;
        self.active_folder_warm_current_stage = Some(ActiveFolderCacheWarmStage::CheckingCache);
        self.active_folder_warm_batch_base_completed = 0;
    }

    pub(in crate::native_app) fn start_active_folder_warm_decode_queue(
        &mut self,
        folder_id: String,
        pending: Vec<PathBuf>,
    ) {
        let total = pending.len();
        self.active_folder_warm_folder_id = Some(folder_id);
        self.active_folder_warm_pending = pending.into();
        self.active_folder_warm_completed = 0;
        self.active_folder_warm_total = total;
        self.clear_active_folder_warm_current();
        self.active_folder_warm_batch_base_completed = 0;
    }

    pub(in crate::native_app) fn clear_active_folder_warm_job(&mut self) {
        self.active_folder_warm_folder_id = None;
        self.active_folder_warm_pending.clear();
        self.active_folder_warm_completed = 0;
        self.active_folder_warm_total = 0;
        self.clear_active_folder_warm_current();
        self.active_folder_warm_batch_base_completed = 0;
    }

    pub(in crate::native_app) fn clear_active_folder_warm_current(&mut self) {
        self.active_folder_warm_current = None;
        self.active_folder_warm_current_progress = 0.0;
        self.active_folder_warm_current_stage = None;
    }

    pub(in crate::native_app) fn apply_active_folder_warm_plan_progress(
        &mut self,
        progress: ActiveFolderCacheWarmPlanProgress,
    ) {
        self.active_folder_warm_completed = progress.checked.min(progress.total);
        self.active_folder_warm_total = progress.total;
        self.active_folder_warm_current = Some(progress.path.clone());
        self.active_folder_warm_current_progress = if progress.total == 0 {
            1.0
        } else {
            progress.checked as f32 / progress.total as f32
        };
        self.active_folder_warm_current_stage = Some(ActiveFolderCacheWarmStage::CheckingCache);
        if progress.playback_ready {
            self.mark_sample_playback_cache_ready(&progress.path);
        }
    }

    pub(in crate::native_app) fn apply_active_folder_warm_progress(
        &mut self,
        progress: ActiveFolderCacheWarmProgress,
    ) {
        let cached_path = progress.cached.then(|| progress.path.clone());
        self.active_folder_warm_completed = self
            .active_folder_warm_batch_base_completed
            .saturating_add(progress.processed)
            .min(self.active_folder_warm_total);
        self.active_folder_warm_current = Some(progress.path);
        self.active_folder_warm_current_progress = progress.current_progress.clamp(0.0, 1.0);
        self.active_folder_warm_current_stage = Some(progress.stage);
        if let Some(path) = cached_path {
            self.mark_sample_playback_cache_ready(&path);
        }
    }

    pub(in crate::native_app) fn complete_active_folder_warm_batch(&mut self, processed: usize) {
        self.active_folder_warm_completed = self
            .active_folder_warm_batch_base_completed
            .saturating_add(processed)
            .min(self.active_folder_warm_total);
        self.clear_active_folder_warm_current();
    }

    pub(in crate::native_app) fn begin_active_folder_warm_batch(
        &mut self,
        first_path: Option<PathBuf>,
    ) {
        self.active_folder_warm_batch_base_completed = self.active_folder_warm_completed;
        self.active_folder_warm_current = first_path;
        self.active_folder_warm_current_progress = 0.0;
        self.active_folder_warm_current_stage = None;
    }

    pub(in crate::native_app) fn mark_sample_playback_cache_ready(&mut self, path: &Path) {
        self.cached_sample_paths.insert(path.display().to_string());
    }
}
