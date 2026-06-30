use std::time::{Duration, Instant};

use radiant::prelude as ui;
use wavecrate::sample_sources::config::{AppConfig, AppSettingsCore};

use crate::native_app::app::{
    GuiMessage, NativeAppState, VolumeSettingsPersistResult, emit_gui_action,
};

pub(in crate::native_app) const VOLUME_PERSIST_DEBOUNCE: Duration = Duration::from_millis(250);

impl NativeAppState {
    pub(in crate::native_app) fn set_volume(&mut self, volume: f32) {
        let started_at = Instant::now();
        let previous = volume_milli(self.audio.volume);
        self.audio.volume = volume.clamp(0.0, 1.0);
        if let Some(runtime) = self.audio.playback_runtime.as_ref() {
            let _ = runtime.try_set_volume(self.audio.volume);
        } else if let Some(player) = self.audio.player.as_mut() {
            player.set_volume(self.audio.volume);
        }
        if volume_milli(self.audio.volume) == previous {
            return;
        }
        self.audio.volume_persist_deadline = Some(started_at + VOLUME_PERSIST_DEBOUNCE);
    }

    pub(in crate::native_app) fn set_normalized_audition_enabled(
        &mut self,
        enabled: bool,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        if self.audio.normalized_audition_enabled == enabled
            && self
                .ui
                .settings
                .persisted
                .controls
                .normalized_audition_enabled
                == enabled
        {
            return;
        }
        self.audio.normalized_audition_enabled = enabled;
        self.ui
            .settings
            .persisted
            .controls
            .normalized_audition_enabled = enabled;
        let current_span = self
            .audio
            .current_playback_span
            .or_else(|| {
                self.waveform
                    .current
                    .play_selection()
                    .filter(|selection| selection.width() > 0.0)
                    .map(|selection| (selection.start(), selection.end()))
            })
            .unwrap_or((0.0, 1.0));
        let playback_gain_normalization =
            self.playback_gain_normalization_for_span(current_span.0, current_span.1);
        if let Some(runtime) = self.audio.playback_runtime.as_ref() {
            let _ =
                runtime.try_set_playback_gain_with_normalization(1.0, playback_gain_normalization);
        } else {
            let gain = self.normalized_audition_gain_for_current_span();
            if let Some(player) = self.audio.player.as_mut() {
                player.set_playback_gain(gain);
            }
        }
        self.persist_top_bar_audio_settings(context);
    }

    fn persist_top_bar_audio_settings(&mut self, context: &mut ui::UiUpdateContext<GuiMessage>) {
        self.audio.volume_persist_deadline = None;
        self.audio.volume_persist_inflight = true;
        let persisted = self.current_settings_core();
        let sources = self.library.folder_browser.configured_sample_sources();
        context
            .business()
            .blocking_io("gui-top-bar-audio-settings-persist")
            .run(
                move |_| persist_volume_settings(sources, persisted),
                GuiMessage::VolumeSettingsPersisted,
            );
    }

    pub(in crate::native_app) fn flush_pending_volume_persist(
        &mut self,
        context: &mut ui::UiUpdateContext<GuiMessage>,
    ) {
        let Some(deadline) = self.audio.volume_persist_deadline else {
            return;
        };
        if Instant::now() < deadline || self.audio.volume_persist_inflight {
            return;
        }
        self.audio.volume_persist_deadline = None;
        self.audio.volume_persist_inflight = true;

        let persisted = self.current_settings_core();
        let sources = self.library.folder_browser.configured_sample_sources();
        context
            .business()
            .blocking_io("gui-volume-settings-persist")
            .run(
                move |_| persist_volume_settings(sources, persisted),
                GuiMessage::VolumeSettingsPersisted,
            );
    }

    pub(in crate::native_app) fn finish_volume_settings_persist(
        &mut self,
        result: VolumeSettingsPersistResult,
    ) {
        let started_at = Instant::now();
        self.audio.volume_persist_inflight = false;
        match result.result {
            Ok(()) => {
                self.ui.settings.persisted = result.persisted;
                emit_gui_action(
                    "playback.volume.set",
                    Some("transport"),
                    None,
                    "success",
                    started_at,
                    None,
                );
            }
            Err(error) => {
                self.ui.status.sample = format!("Settings not saved: {error}");
                emit_gui_action(
                    "playback.volume.persist",
                    Some("settings"),
                    None,
                    "persist_error",
                    started_at,
                    Some(&error),
                );
            }
        }
    }
}

fn persist_volume_settings(
    sources: Vec<wavecrate::sample_sources::SampleSource>,
    persisted: AppSettingsCore,
) -> VolumeSettingsPersistResult {
    let result = wavecrate::sample_sources::config::save(&AppConfig {
        sources,
        core: persisted.clone(),
    })
    .map_err(|err| err.to_string());
    VolumeSettingsPersistResult { persisted, result }
}

fn volume_milli(volume: f32) -> u16 {
    (volume.clamp(0.0, 1.0) * 1000.0).round() as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_milli_clamps_and_rounds_to_persist_threshold() {
        assert_eq!(volume_milli(-1.0), 0);
        assert_eq!(volume_milli(0.0004), 0);
        assert_eq!(volume_milli(0.0005), 1);
        assert_eq!(volume_milli(0.5), 500);
        assert_eq!(volume_milli(2.0), 1000);
    }
}
