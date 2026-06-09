use std::time::Instant;

use wavecrate::audio::{
    AudioDeviceSummary, AudioHostSummary, AudioOutputConfig, AudioPlayer, ResolvedOutput,
};
use wavecrate::sample_sources::config::AppSettingsCore;

use crate::native_app::app::{PendingPlaybackStart, PendingSamplePlayback};

pub(in crate::native_app) struct AudioAppState {
    pub(in crate::native_app) player: Option<AudioPlayer>,
    pub(in crate::native_app) loop_playback: bool,
    pub(in crate::native_app) volume: f32,
    pub(in crate::native_app) volume_persist_deadline: Option<Instant>,
    pub(in crate::native_app) output_config: AudioOutputConfig,
    pub(in crate::native_app) output_resolved: Option<ResolvedOutput>,
    pub(in crate::native_app) hosts: Vec<AudioHostSummary>,
    pub(in crate::native_app) devices: Vec<AudioDeviceSummary>,
    pub(in crate::native_app) sample_rates: Vec<u32>,
    pub(in crate::native_app) settings_error: Option<String>,
    pub(in crate::native_app) current_playback_span: Option<(f32, f32)>,
    pub(in crate::native_app) pending_playback_start: Option<PendingPlaybackStart>,
    pub(in crate::native_app) pending_sample_playback: Option<PendingSamplePlayback>,
    pub(in crate::native_app) early_sample_playback_path: Option<String>,
}

impl AudioAppState {
    pub(in crate::native_app) fn from_settings(settings: &AppSettingsCore) -> Self {
        Self {
            player: None,
            loop_playback: false,
            volume: settings.volume.clamp(0.0, 1.0),
            volume_persist_deadline: None,
            output_config: settings.audio_output.clone(),
            output_resolved: None,
            hosts: Vec::new(),
            devices: Vec::new(),
            sample_rates: Vec::new(),
            settings_error: None,
            current_playback_span: None,
            pending_playback_start: None,
            pending_sample_playback: None,
            early_sample_playback_path: None,
        }
    }

    #[cfg(test)]
    pub(in crate::native_app) fn for_tests() -> Self {
        Self::from_settings(&AppSettingsCore::default())
    }
}
