use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
};

use radiant::prelude as ui;

use crate::native_app::app::WaveformCacheEntry;
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
}

impl Default for WaveformLoadState {
    fn default() -> Self {
        Self {
            progress: 0.0,
            target_progress: 0.0,
            label: None,
        }
    }
}

pub(in crate::native_app) struct WaveformCacheState {
    pub(in crate::native_app) entries: HashMap<PathBuf, WaveformCacheEntry>,
    pub(in crate::native_app) order: VecDeque<PathBuf>,
    pub(in crate::native_app) bytes: usize,
    pub(in crate::native_app) indicator_refresh_task: ui::LatestTask,
    pub(in crate::native_app) warm_pending: VecDeque<PathBuf>,
    pub(in crate::native_app) warm_task: ui::LatestTask,
    pub(in crate::native_app) active_folder_warm_delay_task: ui::LatestTask,
    pub(in crate::native_app) active_folder_warm_task: ui::LatestTask,
    pub(in crate::native_app) active_folder_warm_cancel: Option<ui::CancellationToken>,
    pub(in crate::native_app) active_folder_warm_folder_id: Option<String>,
    pub(in crate::native_app) active_folder_warm_pending: VecDeque<PathBuf>,
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
            warm_task: ui::LatestTask::new(),
            active_folder_warm_delay_task: ui::LatestTask::new(),
            active_folder_warm_task: ui::LatestTask::new(),
            active_folder_warm_cancel: None,
            active_folder_warm_folder_id: None,
            active_folder_warm_pending: Default::default(),
            cached_sample_paths: Default::default(),
        }
    }
}
