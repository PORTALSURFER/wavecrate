//! Controller logic shared by the active native runtime.
//! This module now delegates responsibilities into focused submodules to
//! keep files small and behavior easy to reason about.

/// Shared controller-level formatting helpers.
mod formatting;
/// Deterministic GUI fixture builders used by GUI test scenarios.
mod gui_fixtures;
mod history;
mod library;
mod playback;
mod source_watcher;
mod ui;

mod config;
pub(crate) mod controller_state;
pub(crate) mod jobs;
mod performance;
/// Controller-side synchronization for projection revision counters.
mod revision_bus;
/// Derived-state graph access helpers for runtime projection paths.
mod runtime_graph;
pub(crate) mod state;
pub(crate) mod undo;
mod undo_jobs;
pub(crate) mod updates;

pub(crate) use crate::app_core::state::StatusTone;
use crate::{
    app::state::UiState,
    app::view_model,
    audio::AudioPlayer,
    sample_sources::{SampleSource, SourceDatabase, SourceDbError, SourceId, WavEntry},
    selection::SelectionRange,
    waveform::WaveformRenderer,
};
pub(crate) use controller_state::*;
pub(crate) use gui_fixtures::build_named_gui_fixture_controller;
#[cfg(test)]
pub(crate) use history::catalog_history_handler_supported;
pub(in crate::app::controller) use library::analysis_jobs::AnalysisJobMessage;
use library::analysis_jobs::AnalysisWorkerPool;
use open;
use playback::audio_loader::{AudioLoadError, AudioLoadJob, AudioLoadOutcome};
use rfd::FileDialog;
use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

pub(crate) use ui::map_view::UmapPointQuery;
pub(crate) use ui::status_message::StatusMessage;

pub(crate) const MIN_SELECTION_WIDTH: f32 = 0.001;
pub(crate) const BPM_MIN_SELECTION_DIVISOR: f32 = 16.0;
pub(crate) const SMART_SCALE_SELECTION_BEATS: f32 = 4.0;
pub(crate) const AUDIO_CACHE_CAPACITY: usize = 12;
pub(crate) const AUDIO_HISTORY_LIMIT: usize = 8;
pub(crate) const RANDOM_HISTORY_LIMIT: usize = 20;
pub(crate) const FOCUS_HISTORY_LIMIT: usize = 100;
pub(crate) const UNDO_LIMIT: usize = 20;
pub(crate) const STATUS_LOG_LIMIT: usize = 200;

/// Retained browser-row projection fields keyed by absolute entry index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProjectedBrowserRowCacheEntry {
    /// Stable row-identity hash derived from the live entry relative path.
    pub row_identity_hash: u64,
    /// Relative sample path used for metadata preloads and label fallback.
    pub relative_path: PathBuf,
    /// Stable rendered row label for the browser list.
    pub row_label: String,
    /// Triage column index (`0..=2`) for this row.
    pub column_index: usize,
    /// Signed keep/trash rating level for this row (`-3..=3`).
    pub rating_level: i8,
    /// Playback-age bucket projected for row-aging visuals.
    pub playback_age_bucket: crate::app::state::PlaybackAgeBucket,
    /// Stable rendered inline metadata label for the browser list row.
    pub bucket_label: String,
    /// Whether the backing sample file is currently marked missing.
    pub missing: bool,
    /// Whether the backing sample is marked looped.
    pub looped: bool,
    /// Whether the backing sample is marked as a confirmed keep lock.
    pub locked: bool,
    /// Whether the backing sample is session-marked for later review.
    pub marked: bool,
    /// Cached BPM bits used to detect metadata changes without rebuilding label text.
    pub bpm_value_bits: Option<u32>,
    /// Whether the backing sample currently carries the long-sample marker.
    pub long_sample_mark: bool,
    /// Monotonic usage tick used for bounded least-recently-used eviction.
    pub last_used_tick: u64,
}

/// Visible browser window metadata retained for incremental BPM preloads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProjectedBrowserPreloadWindow {
    /// Selected source associated with the last preload window.
    pub source_id: Option<SourceId>,
    /// Visible-row revision associated with the last preload window.
    pub visible_rows_revision: u64,
    /// First visible row index covered by the last preload window.
    pub window_start: usize,
    /// Number of rows covered by the last preload window.
    pub window_len: usize,
}

/// Cache key for retained map-point projection payloads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProjectedMapPointsCacheKey {
    /// Stable hash of the active source identifier.
    pub source_id_hash: u64,
    /// Stable hash of the active UMAP version.
    pub umap_version_hash: u64,
    /// Monotonic revision for cached map points.
    pub points_revision: u64,
    /// Bitwise query minimum X bound.
    pub query_min_x_bits: u32,
    /// Bitwise query maximum X bound.
    pub query_max_x_bits: u32,
    /// Bitwise query minimum Y bound.
    pub query_min_y_bits: u32,
    /// Bitwise query maximum Y bound.
    pub query_max_y_bits: u32,
}

/// Retained immutable map-point payload reused across native map projections.
pub(crate) type ProjectedMapPointCacheEntry = radiant::app::MapPointModel;

/// Retained selected-row lookup representation for browser projections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ProjectedSelectedPathsLookup {
    /// Fast path for the common single-selection case.
    Single(usize),
    /// Dense lookup used for larger multi-selections.
    Dense(Vec<bool>),
}

/// Maintains app state and bridges core logic to the active GUI runtime.
pub struct AppController {
    /// Mutable UI state shared with native rendering.
    pub ui: UiState,
    audio: ControllerAudioState,
    sample_view: ControllerSampleViewState,
    library: LibraryState,
    cache: LibraryCacheState,
    ui_cache: ControllerUiCacheState,
    /// Cached native projection payload for the currently rendered waveform image.
    pub(crate) projected_waveform_image_signature: Option<u64>,
    /// Cached native projection payload for the currently rendered waveform image.
    pub(crate) projected_waveform_image: Option<Arc<crate::gui::types::ImageRgba>>,
    /// Selected source associated with the retained browser row projection cache.
    pub(crate) projected_browser_rows_source_id: Option<SourceId>,
    /// Static browser-row projection fields keyed by absolute entry index.
    pub(crate) projected_browser_rows: HashMap<usize, ProjectedBrowserRowCacheEntry>,
    /// Monotonic usage clock for bounded browser-row cache eviction.
    pub(crate) projected_browser_row_cache_clock: u64,
    /// Last visible browser window used to diff BPM preload requests.
    pub(crate) projected_browser_preload_window: Option<ProjectedBrowserPreloadWindow>,
    /// Selected-path revision for the retained browser selected-path lookup cache.
    pub(crate) projected_selected_paths_revision: Option<u64>,
    /// Selected absolute-index lookup reused across native browser projections.
    pub(crate) projected_selected_paths_lookup: Option<ProjectedSelectedPathsLookup>,
    /// Retained key for normalized map-point projection payloads.
    pub(crate) projected_map_points_key: Option<ProjectedMapPointsCacheKey>,
    /// Retained map points normalized into render-ready milli-units.
    pub(crate) projected_map_points: Arc<[ProjectedMapPointCacheEntry]>,
    /// Retained unique cluster count aligned with `projected_map_points`.
    pub(crate) projected_map_cluster_count: usize,
    wav_entries: WavEntriesState,
    selection_state: ControllerSelectionState,
    pub(crate) settings: AppSettingsState,
    runtime: ControllerRuntimeState,
    history: ControllerHistoryState,
    #[cfg(target_os = "windows")]
    drag_hwnd: Option<windows::Win32::Foundation::HWND>,
}

impl AppController {
    /// Create a controller with shared renderer and optional audio player.
    pub fn new(renderer: WaveformRenderer, player: Option<Rc<RefCell<AudioPlayer>>>) -> Self {
        let default_capacity = crate::sample_sources::config::AppSettingsCore::default()
            .job_message_queue_capacity as usize;
        Self::new_with_job_message_queue_capacity(renderer, player, default_capacity)
    }

    /// Create a controller with a bounded job message queue capacity override.
    pub fn new_with_job_message_queue_capacity(
        renderer: WaveformRenderer,
        player: Option<Rc<RefCell<AudioPlayer>>>,
        job_message_queue_capacity: usize,
    ) -> Self {
        let (wav_job_tx, wav_job_rx, wav_loader) = library::wav_entries_loader::spawn_wav_loader();
        let (audio_job_tx, audio_job_rx, audio_loader) =
            playback::audio_loader::spawn_audio_loader(renderer.clone());
        let (recording_waveform_job_tx, recording_waveform_job_rx, recording_waveform_loader) =
            playback::recording::waveform_loader::spawn_recording_waveform_loader();
        let (search_job_tx, search_job_rx, search_worker) =
            library::wavs::browser_search_worker::spawn_search_worker();
        let jobs = jobs::ControllerJobs::new(jobs::ControllerJobsInit {
            wav_job_tx,
            wav_job_rx,
            wav_loader,
            audio_job_tx,
            audio_job_rx,
            audio_loader,
            recording_waveform_job_tx,
            recording_waveform_job_rx,
            recording_waveform_loader,
            search_job_tx,
            search_job_rx,
            search_worker,
            job_message_queue_capacity,
        });
        let analysis = AnalysisWorkerPool::new();
        Self {
            ui: UiState::default(),
            audio: ControllerAudioState::new(player, AUDIO_CACHE_CAPACITY, AUDIO_HISTORY_LIMIT),
            sample_view: ControllerSampleViewState::new(renderer),
            library: LibraryState::new(),
            cache: LibraryCacheState::new(),
            ui_cache: ControllerUiCacheState::new(),
            projected_waveform_image_signature: None,
            projected_waveform_image: None,
            projected_browser_rows_source_id: None,
            projected_browser_rows: HashMap::new(),
            projected_browser_row_cache_clock: 0,
            projected_browser_preload_window: None,
            projected_selected_paths_revision: None,
            projected_selected_paths_lookup: None,
            projected_map_points_key: None,
            projected_map_points: Arc::default(),
            projected_map_cluster_count: 0,
            wav_entries: WavEntriesState::new(0, 1024),
            selection_state: ControllerSelectionState::new(),
            settings: AppSettingsState::new(),
            runtime: ControllerRuntimeState::new(jobs, analysis),
            history: ControllerHistoryState::new(UNDO_LIMIT),
            #[cfg(target_os = "windows")]
            drag_hwnd: None,
        }
    }

    #[cfg(target_os = "windows")]
    /// Store the HWND used for initiating external drag-and-drop operations on Windows.
    /// This is populated from the active host frame when available.
    pub fn set_drag_hwnd(&mut self, hwnd: Option<windows::Win32::Foundation::HWND>) {
        self.drag_hwnd = hwnd;
    }

    pub(crate) fn browser(&mut self) -> library::browser_controller::BrowserController<'_> {
        library::browser_controller::BrowserController::new(self)
    }

    pub(crate) fn waveform(&mut self) -> ui::waveform_controller::WaveformController<'_> {
        ui::waveform_controller::WaveformController::new(self)
    }

    pub(crate) fn drag_drop(&mut self) -> ui::drag_drop_controller::DragDropController<'_> {
        ui::drag_drop_controller::DragDropController::new(self)
    }

    pub(crate) fn hotkeys_ctrl(&mut self) -> ui::hotkeys_controller::HotkeysController<'_> {
        ui::hotkeys_controller::HotkeysController::new(self)
    }
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
