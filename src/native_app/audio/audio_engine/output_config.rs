use std::time::Instant;

use radiant::prelude as ui;

use crate::native_app::app::{
    AudioOutputPersistResult, GuiMessage, NativeAppState, PendingPlaybackStart, emit_gui_action,
};
use crate::native_app::audio::playback::PlaybackIntent;
use wavecrate::sample_sources::config::AppConfig;

impl NativeAppState {
    pub(in crate::native_app) fn set_audio_output_host(
        &mut self,
        host: Option<String>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        self.close_audio_settings_dropdowns();
        self.audio.output_config.host = host;
        self.audio.output_config.device = None;
        self.audio.output_config.sample_rate = None;
        self.apply_audio_output_config_change(started_at, "audio.output.host.set", context);
    }

    pub(in crate::native_app) fn set_audio_output_device(
        &mut self,
        device: Option<String>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        self.close_audio_settings_dropdowns();
        self.audio.output_config.device = device;
        self.audio.output_config.sample_rate = None;
        self.apply_audio_output_config_change(started_at, "audio.output.device.set", context);
    }

    pub(in crate::native_app) fn set_audio_output_sample_rate(
        &mut self,
        sample_rate: Option<u32>,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let started_at = Instant::now();
        self.close_audio_settings_dropdowns();
        self.audio.output_config.sample_rate = sample_rate;
        self.apply_audio_output_config_change(started_at, "audio.output.sample_rate.set", context);
    }

    pub(in crate::native_app) fn apply_audio_output_config_change(
        &mut self,
        started_at: Instant,
        action: &'static str,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let restart_span = self
            .waveform
            .current
            .is_playing()
            .then_some(self.audio.current_playback_span)
            .flatten();
        self.stop_audio_output_playback();
        if let Some(runtime) = self.audio.playback_runtime.take() {
            let _ = runtime.try_shutdown();
        }
        self.background.audio_open.cancel();
        self.audio.player = None;
        self.audio.playback_events = None;
        self.audio.clear_sample_playback_session();
        self.audio.output_resolved = None;
        self.audio.pending_playback_start = restart_span.map(|(start, end)| {
            PendingPlaybackStart::skip_history(PlaybackIntent::new(start, end))
        });
        self.audio.settings_error = None;
        self.ui.status.sample = String::from("Audio output change pending");
        self.queue_audio_options_refresh(context);
        self.queue_configured_audio_player_open(context, true);
        emit_gui_action(
            action,
            Some("audio_settings"),
            None,
            "queued",
            started_at,
            None,
        );
    }

    pub(in crate::native_app) fn queue_audio_output_persist(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let mut persisted = self.current_settings_core();
        persisted.audio_output = self.audio.output_config.clone();
        let sources = self.library.folder_browser.configured_sample_sources();
        context
            .business()
            .blocking_io("gui-audio-output-persist")
            .latest(&mut self.background.audio_output_persist_task)
            .run(
                move |_| persist_audio_output_settings(sources, persisted),
                GuiMessage::AudioOutputPersisted,
            );
    }

    pub(in crate::native_app) fn finish_audio_output_persist(
        &mut self,
        completion: ui::TaskCompletion<AudioOutputPersistResult>,
    ) {
        let Some(result) = self
            .background
            .audio_output_persist_task
            .finish_completion(completion)
        else {
            return;
        };
        match result.result {
            Ok(()) if result.persisted.audio_output == self.audio.output_config => {
                self.ui.settings.persisted.audio_output = result.persisted.audio_output;
                self.audio.output_config_persist_pending = false;
            }
            Ok(()) => {}
            Err(error) => {
                self.ui.status.sample = format!("Settings not saved: {error}");
                emit_gui_action(
                    "audio.output.persist",
                    Some("settings"),
                    None,
                    "persist_error",
                    Instant::now(),
                    Some(&error),
                );
            }
        }
    }
}

fn persist_audio_output_settings(
    sources: Vec<wavecrate::sample_sources::SampleSource>,
    persisted: wavecrate::sample_sources::config::AppSettingsCore,
) -> AudioOutputPersistResult {
    let result = wavecrate::sample_sources::config::save(&AppConfig {
        sources,
        core: persisted.clone(),
    })
    .map_err(|err| err.to_string());
    AudioOutputPersistResult { persisted, result }
}
