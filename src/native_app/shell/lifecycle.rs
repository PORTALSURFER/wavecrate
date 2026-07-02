use crate::native_app::app::{
    AudioAppState, BackgroundTaskState, ChromeUiState, FolderBrowserState, GuiMessage,
    LibraryAppState, MetadataAppState, NativeAppState, SettingsAppState, StartupState, StatusState,
    UiAppState, WaveformAppState, WaveformState, sample_path_label,
};
use crate::native_app::app::{WaveformInteraction, emit_gui_action};
use crate::native_app::sample_library::folder_browser::view_contract::DEFAULT_FOLDER_WIDTH;
use radiant::prelude as ui;
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
use wavecrate::sample_sources::config::{AppConfig, AppSettingsCore};

const UI_FRAME_TARGET_FPS: u32 = 60;
const UI_FRAME_TARGET: Duration = Duration::from_micros(16_667);
const UI_FRAME_CADENCE: ui::FrameCadenceConfig =
    ui::FrameCadenceConfig::new(Duration::from_millis(25), Duration::from_millis(100), 60);

impl NativeAppState {
    pub(in crate::native_app) fn load_default() -> Result<Self, String> {
        let started_at = Instant::now();
        let config = wavecrate::sample_sources::config::load_or_default()
            .map_err(|err| format!("load app configuration: {err}"))?;
        let has_configured_sources = !config.sources.is_empty();
        let mut folder_browser = FolderBrowserState::from_sample_sources_deferred(&config.sources);
        folder_browser.set_locked_folder_paths(&config.core.folder_locks);
        folder_browser.apply_collection_names(&config.core.collection_names);
        folder_browser.set_similarity_controls(config.core.similarity.clone());
        let startup_source_scan_pending =
            has_configured_sources && !folder_browser.selected_source_loaded();
        let startup_folder_verify_pending =
            has_configured_sources && folder_browser.selected_source_loaded();
        let (worker_sender, worker_receiver) = mpsc::channel();
        let source_watcher = has_configured_sources.then(|| {
            crate::native_app::sample_library::source_watcher::GuiSourceWatcherHandle::spawn(
                config.sources.clone(),
                worker_sender.clone(),
            )
        });
        let background = BackgroundTaskState::new(worker_sender, Some(worker_receiver));
        let audio = AudioAppState::from_settings(&config.core);
        let startup = StartupState::new(
            startup_source_scan_pending,
            startup_folder_verify_pending,
            has_configured_sources,
        );
        #[cfg(test)]
        let startup = {
            let mut startup = startup;
            startup.app_icon_install_pending = false;
            startup.release_update_check_pending = false;
            startup
        };
        let state = Self {
            ui: UiAppState::new(
                ChromeUiState::new(DEFAULT_FOLDER_WIDTH),
                StatusState::new("Select a sample to load"),
                SettingsAppState::new(config.core.clone()),
                startup,
            ),
            library: LibraryAppState::new(folder_browser, source_watcher),
            waveform: WaveformAppState::new(WaveformState::load_default()?),
            background,
            audio,
            transactions: Default::default(),
            metadata: MetadataAppState::from_settings(&config.core),
        };
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

    pub(in crate::native_app) fn sync_source_watcher(&mut self) {
        let sources = self.library.folder_browser.configured_sample_sources();
        if sources.is_empty() {
            self.library.source_watcher = None;
            return;
        }
        match &self.library.source_watcher {
            Some(watcher) => watcher.replace_sources(sources),
            None => {
                self.library.source_watcher = Some(
                    crate::native_app::sample_library::source_watcher::GuiSourceWatcherHandle::spawn(
                        sources,
                        self.background.worker_sender.clone(),
                    ),
                );
            }
        }
    }

    pub(in crate::native_app) fn persist_user_configuration(
        &mut self,
        action: &'static str,
        started_at: Instant,
    ) {
        match self.save_user_configuration() {
            Ok(()) => {
                self.ui.settings.persisted = self.current_settings_core();
                self.audio.volume_persist_deadline = None;
            }
            Err(error) => {
                self.ui.status.sample = format!("Settings not saved: {error}");
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

    pub(in crate::native_app) fn advance_frame(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let frame_update_started_at = Instant::now();
        self.record_frame_timing();
        let waveform_started_at = Instant::now();
        self.waveform
            .current
            .apply_interaction(WaveformInteraction::Frame);
        self.library.folder_browser.advance_copy_flash_frame();
        self.library
            .folder_browser
            .advance_protected_source_error_flash_frame();
        self.library
            .folder_browser
            .advance_drag_hover_folder_auto_expand();
        log_slow_frame_phase("ui.frame.update.waveform_interaction", waveform_started_at);
        let playback_events_started_at = Instant::now();
        self.drain_playback_runtime_events();
        log_slow_frame_phase(
            "ui.frame.update.playback_runtime_events",
            playback_events_started_at,
        );
        let playback_started_at = Instant::now();
        self.refresh_playback_progress();
        log_slow_frame_phase("ui.frame.update.playback_progress", playback_started_at);
        if self.library.folder_scan_active()
            || self.background.file_move_progress.is_some()
            || self.waveform.cache.active_folder_warm_folder_id.is_some()
        {
            self.background.progress_tick = (self.background.progress_tick + 0.035) % 1.0;
        }
        if self.waveform.load.label.is_some() {
            let remaining = self.waveform.load.target_progress - self.waveform.load.progress;
            if remaining > 0.0 {
                self.waveform.load.progress += remaining.min(0.03);
            }
        }
        let persist_started_at = Instant::now();
        self.flush_pending_volume_persist(context);
        self.flush_pending_similarity_settings_persist(context);
        log_slow_frame_phase("ui.frame.update.persist_settings", persist_started_at);
        log_slow_frame_phase("ui.frame.update.total", frame_update_started_at);
    }

    fn record_frame_timing(&mut self) {
        let report = self.background.frame_cadence.record_now(UI_FRAME_CADENCE);
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
        let sample_loading = self.active_sample_load_task().is_some();
        let audio_opening = self.background.audio_open.active().is_some();
        let folder_scanning = self.library.folder_scan_active();
        let normalizing = self.background.normalization_progress.is_some();
        let moving_files = self.background.file_move_progress.is_some();
        let waveform_loading = self.waveform_sample_load_active();
        let playing = self.waveform.current.is_playing();
        let pending_playback = self.audio.pending_playback_start.is_some();
        let interaction_active = sample_loading
            || audio_opening
            || folder_scanning
            || normalizing
            || moving_files
            || waveform_loading
            || pending_playback;
        let cadence_context = if interaction_active {
            "interaction"
        } else if playing {
            "playback"
        } else {
            "idle"
        };
        let selected = self
            .library
            .folder_browser
            .selected_file_id()
            .map(sample_path_label)
            .unwrap_or_default();

        match report.kind {
            ui::FrameCadenceKind::ErrorSpike | ui::FrameCadenceKind::WarnSpike => {
                tracing::warn!(
                    target: "wavecrate::debug::ui_frame",
                    event = "ui.frame.deviation",
                    severity = report.kind.severity().unwrap_or("warn"),
                    frame = report.frame_index,
                    target_fps = UI_FRAME_TARGET_FPS,
                    target_ms = duration_ms(UI_FRAME_TARGET),
                    delta_ms,
                    max_delta_ms,
                    sample_loading,
                    audio_opening,
                    folder_scanning,
                    normalizing,
                    waveform_loading,
                    playing,
                    pending_playback,
                    cadence_context,
                    selected = selected.as_str(),
                    "UI frame cadence deviated from 60Hz target"
                );
            }
            ui::FrameCadenceKind::Periodic => {
                tracing::debug!(
                    target: "wavecrate::debug::ui_frame",
                    event = "ui.frame",
                    frame = report.frame_index,
                    target_fps = UI_FRAME_TARGET_FPS,
                    target_ms = duration_ms(UI_FRAME_TARGET),
                    delta_ms,
                    max_delta_ms,
                    sample_loading,
                    audio_opening,
                    folder_scanning,
                    normalizing,
                    waveform_loading,
                    playing,
                    pending_playback,
                    cadence_context,
                    selected = selected.as_str(),
                    "UI frame timing"
                );
            }
            ui::FrameCadenceKind::Started | ui::FrameCadenceKind::Normal => {}
        }
    }

    pub(in crate::native_app) fn maybe_auto_load_startup_sample(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if !self.ui.startup.auto_load_pending {
            return;
        }
        if self.library.folder_browser.selected_file_id().is_some() {
            self.ui.startup.auto_load_pending = false;
            return;
        }
        let Some(path) = self.library.folder_browser.first_audio_file_path() else {
            if self.library.folder_browser.selected_source_loaded() {
                self.ui.startup.auto_load_pending = false;
            }
            return;
        };
        self.ui.startup.auto_load_pending = false;
        self.library.folder_browser.focus_file_across_sources(&path);
        self.load_sample_without_autoplay(path.display().to_string(), context);
    }

    pub(in crate::native_app) fn worker_subscription(&mut self) -> ui::Subscription<GuiMessage> {
        self.background
            .worker_receiver
            .take()
            .map(|receiver| ui::Subscription::worker("gui-workers", receiver))
            .unwrap_or_else(ui::Subscription::none)
    }

    pub(in crate::native_app) fn current_settings_core(&self) -> AppSettingsCore {
        let mut controls = self.ui.settings.persisted.controls.clone();
        controls.normalized_audition_enabled = self.audio.normalized_audition_enabled;
        AppSettingsCore {
            audio_output: self.audio.output_config.clone(),
            volume: self.audio.volume,
            controls,
            similarity: self.library.folder_browser.similarity_controls().clone(),
            collection_names: self.library.folder_browser.custom_collection_names(),
            folder_locks: self.library.folder_browser.locked_folder_paths(),
            tag_dictionary: self.metadata.tag_dictionary.clone(),
            ..self.ui.settings.persisted.clone()
        }
    }

    fn save_user_configuration(&self) -> Result<(), String> {
        let core = self.current_settings_core();
        wavecrate::sample_sources::config::save(&AppConfig {
            sources: self.library.folder_browser.configured_sample_sources(),
            core,
        })
        .map_err(|err| err.to_string())?;
        self.library.folder_browser.save_source_scan_cache()
    }

    pub(in crate::native_app) fn shutdown(&mut self) -> Option<serde_json::Value> {
        let started_at = Instant::now();
        crate::native_app::waveform::flush_background_waveform_cache_stores_for_shutdown();
        let elapsed = started_at.elapsed();
        Some(serde_json::json!({
            "waveform_cache_shutdown_flush_ms": duration_ms(elapsed),
        }))
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
