use super::{
    DEFAULT_FOLDER_WIDTH, FolderBrowserState, GuiAppState, GuiMessage, SampleNameViewMode,
    WaveformState, sample_path_label,
};
use crate::gui_app::{launch::emit_gui_action, waveform::WaveformInteraction};
use radiant::prelude as ui;
use std::{
    collections::HashMap,
    sync::mpsc,
    time::{Duration, Instant},
};
use wavecrate::sample_sources::config::{AppConfig, AppSettingsCore};

const UI_FRAME_CADENCE: ui::FrameCadenceConfig =
    ui::FrameCadenceConfig::new(Duration::from_millis(34), Duration::from_millis(100), 120);

impl GuiAppState {
    pub(super) fn load_default() -> Result<Self, String> {
        let started_at = Instant::now();
        let config = wavecrate::sample_sources::config::load_or_default()
            .map_err(|err| format!("load app configuration: {err}"))?;
        let has_configured_sources = !config.sources.is_empty();
        let folder_browser = FolderBrowserState::from_sample_sources_deferred(&config.sources);
        let startup_source_scan_pending =
            has_configured_sources && !folder_browser.selected_source_loaded();
        let (worker_sender, worker_receiver) = mpsc::channel();
        let mut state = Self {
            folder_width: DEFAULT_FOLDER_WIDTH,
            folder_resize: None,
            folder_browser,
            waveform: WaveformState::load_default()?,
            sample_status: String::from("Select a sample to load"),
            worker_sender,
            worker_receiver: Some(worker_receiver),
            next_task_id: 1,
            deferred_sample_load_task: ui::LatestTask::new(),
            sample_load_task: ui::LatestTask::new(),
            sample_load_cancel: None,
            audio_open_task: ui::LatestTask::new(),
            audio_open_results: Default::default(),
            folder_progress: None,
            normalization_progress: None,
            progress_tick: 0.0,
            frame_cadence: ui::FrameCadenceMonitor::new(),
            waveform_loading_progress: 0.0,
            waveform_loading_target_progress: 0.0,
            audio_player: None,
            loop_playback: false,
            volume: config.core.volume.clamp(0.0, 1.0),
            volume_persist_deadline: None,
            audio_output_config: config.core.audio_output.clone(),
            audio_output_resolved: None,
            audio_hosts: Vec::new(),
            audio_devices: Vec::new(),
            audio_sample_rates: Vec::new(),
            persisted_settings: config.core.clone(),
            audio_settings_open: false,
            audio_settings_dropdown: ui::ExclusiveOpen::new(),
            job_details_open: false,
            context_menu: None,
            waveform_loading_label: None,
            audio_settings_error: None,
            current_playback_span: None,
            pending_playback_start: None,
            pending_sample_playback: None,
            native_file_drop_hover: None,
            metadata_tag_draft: String::new(),
            metadata_tag_tokens: Vec::new(),
            metadata_tag_input_mode: Default::default(),
            metadata_tag_completion_cycle: ui::CyclicListSelectionCycle::new(),
            metadata_tag_dictionary: config.core.tag_dictionary.clone(),
            metadata_tag_library_open: false,
            metadata_tag_drag: None,
            metadata_tag_drop_hover: None,
            selected_metadata_tag: None,
            collapsed_metadata_tag_categories: Default::default(),
            metadata_tags_by_file: HashMap::new(),
            sample_name_view_mode: SampleNameViewMode::DiskFilename,
            startup_source_scan_pending,
            startup_auto_load_pending: has_configured_sources,
            waveform_cache: HashMap::new(),
            waveform_cache_order: Default::default(),
            waveform_cache_bytes: 0,
            waveform_cache_warm_pending: Default::default(),
            waveform_cache_warm_task: ui::LatestTask::new(),
            waveform_cache_warm_results: Default::default(),
            cached_sample_paths: Default::default(),
        };
        if state.folder_browser.selected_source_loaded() {
            state.refresh_persisted_waveform_cache_indicators();
        }
        emit_gui_action(
            "runtime.startup.load_default_state",
            Some("background"),
            Some("assets"),
            "success",
            started_at,
            None,
        );
        Ok(state)
    }

    pub(super) fn persist_user_configuration(&mut self, action: &'static str, started_at: Instant) {
        match self.save_user_configuration() {
            Ok(()) => {
                self.persisted_settings = self.current_settings_core();
                self.volume_persist_deadline = None;
            }
            Err(error) => {
                self.sample_status = format!("Settings not saved: {error}");
                emit_gui_action(
                    action,
                    Some("settings"),
                    None,
                    "persist_error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }

    pub(super) fn advance_frame(&mut self) {
        let frame_update_started_at = Instant::now();
        self.record_frame_timing();
        let waveform_started_at = Instant::now();
        self.waveform.apply_interaction(WaveformInteraction::Frame);
        log_slow_frame_phase("ui.frame.update.waveform_interaction", waveform_started_at);
        let playback_started_at = Instant::now();
        self.refresh_playback_progress();
        log_slow_frame_phase("ui.frame.update.playback_progress", playback_started_at);
        if self.folder_progress.is_some() || self.normalization_progress.is_some() {
            self.progress_tick = (self.progress_tick + 0.035) % 1.0;
        }
        if self.waveform_loading_label.is_some() {
            let remaining = self.waveform_loading_target_progress - self.waveform_loading_progress;
            if remaining > 0.0 {
                self.waveform_loading_progress += remaining.min(0.03);
            }
        }
        let persist_started_at = Instant::now();
        self.flush_pending_volume_persist();
        log_slow_frame_phase("ui.frame.update.persist_volume", persist_started_at);
        log_slow_frame_phase("ui.frame.update.total", frame_update_started_at);
    }

    fn record_frame_timing(&mut self) {
        let report = self.frame_cadence.record_now(UI_FRAME_CADENCE);
        let Some(delta) = report.delta else {
            tracing::debug!(
                target: "wavecrate::debug::ui_frame",
                event = "ui.frame",
                frame = report.frame_index,
                "UI frame timing started"
            );
            return;
        };
        let delta_ms = duration_ms(delta);
        let max_delta_ms = duration_ms(report.max_delta);
        let sample_loading = self.sample_load_task.active().is_some();
        let audio_opening = self.audio_open_task.active().is_some();
        let folder_scanning = self.folder_progress.is_some();
        let normalizing = self.normalization_progress.is_some();
        let waveform_loading = self.waveform_loading_label.is_some();
        let playing = self.waveform.is_playing();
        let pending_playback = self.pending_playback_start.is_some();
        let selected = self
            .folder_browser
            .selected_file_id()
            .map(sample_path_label)
            .unwrap_or_default();

        match report.kind {
            ui::FrameCadenceKind::ErrorSpike | ui::FrameCadenceKind::WarnSpike => {
                tracing::warn!(
                    target: "wavecrate::debug::ui_frame",
                    event = "ui.frame.spike",
                    severity = report.kind.severity().unwrap_or("warn"),
                    frame = report.frame_index,
                    delta_ms,
                    max_delta_ms,
                    sample_loading,
                    audio_opening,
                    folder_scanning,
                    normalizing,
                    waveform_loading,
                    playing,
                    pending_playback,
                    selected = selected.as_str(),
                    "UI frame spike"
                );
            }
            ui::FrameCadenceKind::Periodic => {
                tracing::debug!(
                    target: "wavecrate::debug::ui_frame",
                    event = "ui.frame",
                    frame = report.frame_index,
                    delta_ms,
                    max_delta_ms,
                    sample_loading,
                    audio_opening,
                    folder_scanning,
                    normalizing,
                    waveform_loading,
                    playing,
                    pending_playback,
                    selected = selected.as_str(),
                    "UI frame timing"
                );
            }
            ui::FrameCadenceKind::Started | ui::FrameCadenceKind::Normal => {}
        }
    }

    pub(super) fn maybe_auto_load_startup_sample(
        &mut self,
        context: &mut ui::UpdateContext<GuiMessage>,
    ) {
        if !self.startup_auto_load_pending {
            return;
        }
        if self.folder_browser.selected_file_id().is_some() {
            self.startup_auto_load_pending = false;
            return;
        }
        let Some(path) = self.folder_browser.first_audio_file_path() else {
            if self.folder_browser.selected_source_loaded() {
                self.startup_auto_load_pending = false;
            }
            return;
        };
        self.startup_auto_load_pending = false;
        self.folder_browser.focus_file_across_sources(&path);
        self.load_sample_without_autoplay(path.display().to_string(), context);
    }

    pub(super) fn worker_subscription(&mut self) -> ui::Subscription<GuiMessage> {
        self.worker_receiver
            .take()
            .map(|receiver| ui::Subscription::worker("gui-workers", receiver))
            .unwrap_or_else(ui::Subscription::none)
    }

    pub(super) fn current_settings_core(&self) -> AppSettingsCore {
        AppSettingsCore {
            audio_output: self.audio_output_config.clone(),
            volume: self.volume,
            tag_dictionary: self.metadata_tag_dictionary.clone(),
            ..self.persisted_settings.clone()
        }
    }

    fn save_user_configuration(&self) -> Result<(), String> {
        let core = self.current_settings_core();
        wavecrate::sample_sources::config::save(&AppConfig {
            sources: self.folder_browser.configured_sample_sources(),
            core,
        })
        .map_err(|err| err.to_string())?;
        self.folder_browser.save_source_scan_cache()
    }
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn log_slow_frame_phase(event: &'static str, started_at: Instant) {
    let elapsed = started_at.elapsed();
    if elapsed < Duration::from_millis(4) {
        return;
    }
    tracing::warn!(
        target: "wavecrate::debug::ui_frame",
        event,
        elapsed_ms = duration_ms(elapsed),
        "Slow UI frame update phase"
    );
}
