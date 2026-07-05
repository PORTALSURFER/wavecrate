use std::{sync::mpsc::Receiver, time::Instant};

use wavecrate::audio::{
    AudioDeviceSummary, AudioHostSummary, AudioOutputConfig, AudioPlayer, PlaybackRequestId,
    PlaybackRuntimeEvent, PlaybackRuntimeHandle, PlaybackRuntimeProgress, ResolvedOutput,
};
use wavecrate::sample_sources::config::AppSettingsCore;

use crate::native_app::{
    app::PendingSamplePlayback,
    audio::{playback::PlaybackIntent, playback_history::PlaybackNavigationHistory},
};

pub(in crate::native_app) struct AudioAppState {
    pub(in crate::native_app) player: Option<AudioPlayer>,
    pub(in crate::native_app) loop_playback: bool,
    pub(in crate::native_app) loop_playback_manual_override_path: Option<String>,
    pub(in crate::native_app) metronome_enabled: bool,
    pub(in crate::native_app) normalized_audition_enabled: bool,
    pub(in crate::native_app) volume: f32,
    pub(in crate::native_app) volume_persist_deadline: Option<Instant>,
    pub(in crate::native_app) volume_persist_inflight: bool,
    pub(in crate::native_app) last_played_persist_task: radiant::prelude::LatestTask,
    pub(in crate::native_app) output_config: AudioOutputConfig,
    pub(in crate::native_app) output_resolved: Option<ResolvedOutput>,
    pub(in crate::native_app) hosts: Vec<AudioHostSummary>,
    pub(in crate::native_app) devices: Vec<AudioDeviceSummary>,
    pub(in crate::native_app) sample_rates: Vec<u32>,
    pub(in crate::native_app) settings_error: Option<String>,
    pub(in crate::native_app) current_playback_span: Option<(f32, f32)>,
    pub(in crate::native_app) pending_playback_start: Option<PendingPlaybackStart>,
    pub(in crate::native_app) pending_sample_playback: Option<PendingSamplePlayback>,
    pub(in crate::native_app) playback_history: PlaybackNavigationHistory,
    pub(in crate::native_app) early_sample_playback_path: Option<String>,
    pub(in crate::native_app) early_sample_playback_kind: Option<EarlySamplePlaybackKind>,
    pub(in crate::native_app) playback_runtime: Option<PlaybackRuntimeHandle>,
    pub(in crate::native_app) playback_events: Option<Receiver<PlaybackRuntimeEvent>>,
    pub(in crate::native_app) playback_progress: PlaybackRuntimeProgress,
    pub(in crate::native_app) pending_runtime_start: Option<PendingRuntimePlaybackStart>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_app) enum EarlySamplePlaybackKind {
    PreviewSlice,
    FullSample,
}

pub(in crate::native_app) struct PendingRuntimePlaybackStart {
    pub(in crate::native_app) id: PlaybackRequestId,
    pub(in crate::native_app) path: String,
    pub(in crate::native_app) span: (f32, f32),
    pub(in crate::native_app) show_start_marker: bool,
    pub(in crate::native_app) submitted_at: Instant,
    pub(in crate::native_app) origin: &'static str,
    pub(in crate::native_app) source_kind: &'static str,
}

impl PendingRuntimePlaybackStart {
    pub(in crate::native_app) fn new(
        id: PlaybackRequestId,
        path: String,
        span: (f32, f32),
        show_start_marker: bool,
        origin: &'static str,
        source_kind: &'static str,
    ) -> Self {
        Self {
            id,
            path,
            span,
            show_start_marker,
            submitted_at: Instant::now(),
            origin,
            source_kind,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) struct PendingPlaybackStart {
    pub(in crate::native_app) intent: PlaybackIntent,
    pub(in crate::native_app) record_history: bool,
}

impl PendingPlaybackStart {
    pub(in crate::native_app) fn record(intent: PlaybackIntent) -> Self {
        Self {
            intent,
            record_history: true,
        }
    }

    pub(in crate::native_app) fn skip_history(intent: PlaybackIntent) -> Self {
        Self {
            intent,
            record_history: false,
        }
    }
}

impl AudioAppState {
    pub(in crate::native_app) fn from_settings(settings: &AppSettingsCore) -> Self {
        Self {
            player: None,
            loop_playback: false,
            loop_playback_manual_override_path: None,
            metronome_enabled: false,
            normalized_audition_enabled: settings.controls.normalized_audition_enabled,
            volume: settings.volume.clamp(0.0, 1.0),
            volume_persist_deadline: None,
            volume_persist_inflight: false,
            last_played_persist_task: radiant::prelude::LatestTask::new(),
            output_config: settings.audio_output.clone(),
            output_resolved: None,
            hosts: Vec::new(),
            devices: Vec::new(),
            sample_rates: Vec::new(),
            settings_error: None,
            current_playback_span: None,
            pending_playback_start: None,
            pending_sample_playback: None,
            playback_history: PlaybackNavigationHistory::default(),
            early_sample_playback_path: None,
            early_sample_playback_kind: None,
            playback_runtime: None,
            playback_events: None,
            playback_progress: PlaybackRuntimeProgress::default(),
            pending_runtime_start: None,
        }
    }

    #[cfg(test)]
    pub(in crate::native_app) fn for_tests() -> Self {
        Self::from_settings(&AppSettingsCore::default())
    }
}
