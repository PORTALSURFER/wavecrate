//! Controller logic shared by the active native runtime.
//! This module now delegates responsibilities into focused submodules to
//! keep files small and behavior easy to reason about.

/// Shared controller-level formatting helpers.
mod formatting;
mod library;
mod playback;
mod source_watcher;
mod ui;

mod config;
pub(crate) mod controller_state;
pub(crate) mod jobs;
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
    gui::repaint::RepaintSignal,
    sample_sources::{SampleSource, SourceDatabase, SourceDbError, SourceId, WavEntry},
    selection::SelectionRange,
    waveform::WaveformRenderer,
};
pub(crate) use controller_state::*;
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

#[cfg(test)]
/// Re-export hotkey definitions for controller tests that use the bare `hotkeys`
/// module path.
pub(crate) use ui::hotkeys;
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
    /// Stable rendered inline metadata label for the browser list row.
    pub bucket_label: String,
    /// Whether the backing sample file is currently marked missing.
    pub missing: bool,
    /// Whether the backing sample is marked looped.
    pub looped: bool,
    /// Whether the backing sample is marked as a confirmed keep lock.
    pub locked: bool,
    /// Cached BPM bits used to detect metadata changes without rebuilding label text.
    pub bpm_value_bits: Option<u32>,
    /// Whether the backing sample currently carries the long-sample marker.
    pub long_sample_mark: bool,
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

    fn observe_frame_timing_for_fps(&mut self, now: Instant, user_active: bool) {
        const SLOW_FRAME_THRESHOLD: Duration = Duration::from_millis(40);
        if let Some(last_frame) = self.runtime.performance.last_frame_at {
            let frame_delta = now.saturating_duration_since(last_frame);
            self.runtime.performance.observe_frame_interval(frame_delta);
            if frame_delta >= SLOW_FRAME_THRESHOLD {
                self.runtime.performance.last_slow_frame_at = Some(now);
            }
        }
        self.runtime.performance.last_frame_at = Some(now);
        if user_active {
            self.runtime.performance.last_user_activity_at = Some(now);
        }
    }

    pub(crate) fn update_performance_governor(&mut self, user_active: bool) {
        const ACTIVE_WINDOW: Duration = Duration::from_millis(300);
        const IDLE_WINDOW: Duration = Duration::from_secs(2);
        let now = Instant::now();
        self.observe_frame_timing_for_fps(now, user_active);
        let recent_input = self
            .runtime
            .performance
            .last_user_activity_at
            .is_some_and(|time| now.saturating_duration_since(time) <= ACTIVE_WINDOW);
        let recent_slow_frame = self
            .runtime
            .performance
            .last_slow_frame_at
            .is_some_and(|time| now.saturating_duration_since(time) <= ACTIVE_WINDOW);
        let busy = self.is_playing() || recent_input || recent_slow_frame;
        let analysis_active = self
            .ui
            .progress
            .analysis
            .as_ref()
            .is_some_and(|snapshot| snapshot.pending > 0 || snapshot.running > 0);
        let pause_claiming = (self.is_playing() || recent_input) && !analysis_active;
        let last_activity_at = match (
            self.runtime.performance.last_user_activity_at,
            self.runtime.performance.last_slow_frame_at,
        ) {
            (Some(input), Some(slow)) => Some(input.max(slow)),
            (Some(input), None) => Some(input),
            (None, Some(slow)) => Some(slow),
            (None, None) => None,
        };
        let idle = !self.is_playing()
            && last_activity_at
                .is_some_and(|time| now.saturating_duration_since(time) >= IDLE_WINDOW);
        let base_worker_count = if self.settings.analysis.analysis_worker_count == 0 {
            crate::app::controller::library::analysis_jobs::default_worker_count()
        } else {
            self.settings.analysis.analysis_worker_count
        };
        let idle_target = self
            .runtime
            .performance
            .idle_worker_override
            .unwrap_or(base_worker_count);
        let target = if busy || !idle { 1 } else { idle_target };
        if pause_claiming {
            self.runtime.analysis.pause_claiming();
        } else {
            self.runtime.analysis.resume_claiming();
        }
        if self.runtime.performance.last_worker_count != Some(target) {
            self.runtime.analysis.set_worker_count(target);
            self.runtime.performance.last_worker_count = Some(target);
        }
    }

    /// Record the latest inter-frame timing sample used by the FPS counter.
    pub(crate) fn record_frame_timing_for_fps(&mut self) {
        let now = Instant::now();
        self.observe_frame_timing_for_fps(now, false);
    }

    #[cfg(target_os = "windows")]
    /// Store the HWND used for initiating external drag-and-drop operations on Windows.
    /// This is populated from the active host frame when available.
    pub fn set_drag_hwnd(&mut self, hwnd: Option<windows::Win32::Foundation::HWND>) {
        self.drag_hwnd = hwnd;
    }

    pub(crate) fn set_status(&mut self, text: impl Into<String>, tone: StatusTone) {
        let text = text.into();
        let status_changed = self.ui.status.text != text || self.ui.status.status_tone != tone;
        self.ui.status.text = text.clone();
        self.ui.status.status_tone = tone;
        if status_changed {
            self.mark_status_projection_revision_dirty();
        }
        let entry = format!("[{}] {}", status_prefix(tone), text);
        if self.ui.status.log.last().is_some_and(|last| last == &entry) {
            return;
        }
        self.ui.status.log.push(entry);
        if self.ui.status.log.len() > STATUS_LOG_LIMIT {
            let overflow = self.ui.status.log.len() - STATUS_LOG_LIMIT;
            self.ui.status.log.drain(0..overflow);
        }
        log_status_entry(tone, self.ui.status.log.last().expect("just pushed"));
    }

    pub(crate) fn set_error_status(&mut self, text: impl Into<String>) {
        self.set_status(text, StatusTone::Error);
    }

    pub(crate) fn set_status_message(&mut self, message: StatusMessage) {
        let (text, tone) = message.into_text_and_tone();
        self.set_status(text, tone);
    }

    /// Current exponentially weighted average FPS estimated from recent frame intervals.
    pub(crate) fn average_fps(&self) -> Option<f64> {
        self.runtime.performance.average_fps()
    }

    pub(crate) fn set_repaint_signal(&mut self, signal: Arc<dyn RepaintSignal>) {
        self.runtime.jobs.set_repaint_signal(signal.clone());
        self.runtime.analysis.set_repaint_signal(signal);
    }

    /// Shut down background workers owned by the controller.
    pub(crate) fn shutdown(&mut self) {
        self.runtime.jobs.shutdown();
        self.runtime.analysis.shutdown();
    }

    pub(crate) fn undo(&mut self) {
        if self.history.pending_undo.is_some() {
            self.set_status("Undo already in progress", StatusTone::Warning);
            return;
        }
        if self.runtime.jobs.file_ops_in_progress() {
            self.set_status("File operation already in progress", StatusTone::Warning);
            return;
        }
        let mut stack = std::mem::replace(
            &mut self.history.undo_stack,
            undo::UndoStack::new(UNDO_LIMIT),
        );
        let result = stack.undo(self);
        self.history.undo_stack = stack;
        match result {
            Ok(undo::UndoOutcome::Applied(label)) => {
                self.set_status(format!("Undid {label}"), StatusTone::Info);
            }
            Ok(undo::UndoOutcome::Empty) => self.set_status("Nothing to undo", StatusTone::Info),
            Ok(undo::UndoOutcome::Deferred(pending)) => {
                self.begin_deferred_undo_job(*pending);
            }
            Err(err) => self.set_status(format!("Undo failed: {err}"), StatusTone::Error),
        }
    }

    pub(crate) fn redo(&mut self) {
        if self.history.pending_undo.is_some() {
            self.set_status("Redo already in progress", StatusTone::Warning);
            return;
        }
        if self.runtime.jobs.file_ops_in_progress() {
            self.set_status("File operation already in progress", StatusTone::Warning);
            return;
        }
        let mut stack = std::mem::replace(
            &mut self.history.undo_stack,
            undo::UndoStack::new(UNDO_LIMIT),
        );
        let result = stack.redo(self);
        self.history.undo_stack = stack;
        match result {
            Ok(undo::UndoOutcome::Applied(label)) => {
                self.set_status(format!("Redid {label}"), StatusTone::Info);
            }
            Ok(undo::UndoOutcome::Empty) => self.set_status("Nothing to redo", StatusTone::Info),
            Ok(undo::UndoOutcome::Deferred(pending)) => {
                self.begin_deferred_undo_job(*pending);
            }
            Err(err) => self.set_status(format!("Redo failed: {err}"), StatusTone::Error),
        }
    }

    pub(crate) fn push_undo_entry(&mut self, entry: undo::UndoEntry<AppController>) {
        self.history.undo_stack.push(entry);
    }

    pub(crate) fn begin_selection_undo(&mut self, label: impl Into<String>) {
        if self.selection_state.pending_undo.is_some() {
            return;
        }
        let before = self
            .selection_state
            .range
            .range()
            .or(self.ui.waveform.selection);
        self.selection_state.pending_undo = Some(SelectionUndoState {
            label: label.into(),
            before,
        });
    }

    pub(crate) fn commit_selection_undo(&mut self) {
        let Some(pending) = self.selection_state.pending_undo.take() else {
            return;
        };
        let after = self
            .selection_state
            .range
            .range()
            .or(self.ui.waveform.selection);
        self.push_selection_undo(pending.label, pending.before, after);
    }

    pub(crate) fn push_selection_undo(
        &mut self,
        label: impl Into<String>,
        before: Option<SelectionRange>,
        after: Option<SelectionRange>,
    ) {
        if before == after {
            return;
        }
        let label = label.into();
        self.push_undo_entry(undo::UndoEntry::<AppController>::new(
            label,
            move |controller| {
                controller.selection_state.range.set_range(before);
                controller.apply_selection(before);
                Ok(undo::UndoExecution::Applied)
            },
            move |controller| {
                controller.selection_state.range.set_range(after);
                controller.apply_selection(after);
                Ok(undo::UndoExecution::Applied)
            },
        ));
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

    /// Returns the duration in seconds for the currently loaded audio, if any.
    pub(crate) fn loaded_audio_duration_seconds(&self) -> Option<f32> {
        self.sample_view
            .wav
            .loaded_audio
            .as_ref()
            .map(|audio| audio.duration_seconds)
    }

    pub(crate) fn is_issue_gateway_poll_in_progress(&self) -> bool {
        self.runtime.jobs.issue_gateway_poll_in_progress()
    }
}

fn log_status_entry(tone: StatusTone, entry: &str) {
    match tone {
        StatusTone::Warning => tracing::warn!("{entry}"),
        StatusTone::Error => tracing::error!("{entry}"),
        StatusTone::Info | StatusTone::Busy | StatusTone::Idle => tracing::info!("{entry}"),
    }
}

/// Return the status badge prefix text for a status tone.
fn status_prefix(tone: StatusTone) -> &'static str {
    match tone {
        StatusTone::Idle => "Idle",
        StatusTone::Busy => "Working",
        StatusTone::Info => "Info",
        StatusTone::Warning => "Warning",
        StatusTone::Error => "Error",
    }
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
