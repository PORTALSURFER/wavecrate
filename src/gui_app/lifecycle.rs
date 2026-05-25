use super::{
    DEFAULT_FOLDER_WIDTH, FolderBrowserState, GuiAppState, GuiMessage, SampleNameViewMode,
    WaveformState,
};
use crate::gui_app::{launch::emit_gui_action, waveform::WaveformInteraction};
use radiant::prelude as ui;
use std::{collections::HashMap, sync::mpsc, time::Instant};
use wavecrate::sample_sources::config::{AppConfig, AppSettingsCore};

impl GuiAppState {
    pub(super) fn load_default() -> Result<Self, String> {
        let started_at = Instant::now();
        let config = wavecrate::sample_sources::config::load_or_default()
            .map_err(|err| format!("load app configuration: {err}"))?;
        let (worker_sender, worker_receiver) = mpsc::channel();
        let mut state = Self {
            folder_width: DEFAULT_FOLDER_WIDTH,
            folder_resize: None,
            folder_browser: FolderBrowserState::from_sample_sources(&config.sources),
            waveform: WaveformState::load_default()?,
            sample_status: String::from("Select a sample to load"),
            worker_sender,
            worker_receiver: Some(worker_receiver),
            next_task_id: 1,
            sample_load_task: ui::LatestTask::new(),
            folder_progress: None,
            progress_tick: 0.0,
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
            persisted_settings: config.core,
            audio_settings_open: false,
            audio_backend_dropdown_open: false,
            audio_output_dropdown_open: false,
            audio_sample_rate_dropdown_open: false,
            job_details_open: false,
            context_menu: None,
            waveform_loading_label: None,
            audio_settings_error: None,
            current_playback_span: None,
            native_file_drop_hover: None,
            metadata_tag_draft: String::new(),
            metadata_tag_tokens: Vec::new(),
            metadata_tags_by_file: HashMap::new(),
            sample_name_view_mode: SampleNameViewMode::DiskFilename,
        };
        state.refresh_audio_options();
        if let Err(error) = state.open_configured_audio_player() {
            state.audio_settings_error = Some(error);
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
        self.waveform.apply_interaction(WaveformInteraction::Frame);
        self.refresh_playback_progress();
        if self.folder_progress.is_some() {
            self.progress_tick = (self.progress_tick + 0.035) % 1.0;
        }
        if self.waveform_loading_label.is_some() {
            let remaining = self.waveform_loading_target_progress - self.waveform_loading_progress;
            if remaining > 0.0 {
                self.waveform_loading_progress += remaining.min(0.03);
            }
        }
        self.flush_pending_volume_persist();
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
            ..self.persisted_settings.clone()
        }
    }

    fn save_user_configuration(&self) -> Result<(), String> {
        let core = self.current_settings_core();
        wavecrate::sample_sources::config::save(&AppConfig {
            sources: self.folder_browser.configured_sample_sources(),
            core,
        })
        .map_err(|err| err.to_string())
    }
}
