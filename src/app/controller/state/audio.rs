//! Audio playback and loading state for the controller.

use crate::app::controller::playback::audio_cache::AudioCache;
use crate::audio::AudioPlayer;
use crate::audio::{AudioRecorder, InputMonitor};
use crate::sample_sources::SourceId;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

pub(crate) struct ControllerAudioState {
    pub(crate) player: Option<Rc<RefCell<AudioPlayer>>>,
    pub(crate) cache: AudioCache,
    pub(crate) pending_loop_disable_at: Option<Instant>,
    pub(crate) pending_loop_retarget: Option<PendingLoopRetarget>,
    pub(crate) recorder: Option<AudioRecorder>,
    pub(crate) recording_target: Option<RecordingTarget>,
    pub(crate) input_monitor: Option<InputMonitor>,
    pub(crate) pending_age_update: Option<PendingAgeUpdate>,
}

impl ControllerAudioState {
    pub(crate) fn new(
        player: Option<Rc<RefCell<AudioPlayer>>>,
        cache_capacity: usize,
        history_limit: usize,
    ) -> Self {
        Self {
            player,
            cache: AudioCache::new(cache_capacity, history_limit),
            pending_loop_disable_at: None,
            pending_loop_retarget: None,
            recorder: None,
            recording_target: None,
            input_monitor: None,
            pending_age_update: None,
        }
    }

    /// Clear any deferred loop retarget scheduled for the active player.
    pub(crate) fn clear_pending_loop_retarget(&mut self) {
        self.pending_loop_retarget = None;
    }

    /// Schedule one loop retarget to apply at the provided cycle boundary.
    pub(crate) fn schedule_loop_retarget(&mut self, deadline: Instant, start_override: f64) {
        self.pending_loop_retarget = Some(PendingLoopRetarget {
            deadline,
            start_override: start_override.clamp(0.0, 1.0),
        });
    }

    /// Consume one deferred loop retarget once it becomes due.
    ///
    /// The retarget is cleared eagerly when playback or loop mode is no longer
    /// active so later selection edits cannot accidentally revive stale state.
    pub(crate) fn take_due_loop_retarget(
        &mut self,
        now: Instant,
        is_playing: bool,
        is_looping: bool,
    ) -> Option<f64> {
        let pending = self.pending_loop_retarget?;
        if !is_playing || !is_looping {
            self.pending_loop_retarget = None;
            return None;
        }
        if now < pending.deadline {
            return None;
        }
        self.pending_loop_retarget = None;
        Some(pending.start_override)
    }
}

#[derive(Clone)]
pub(crate) struct PendingAgeUpdate {
    pub(crate) source_id: SourceId,
    pub(crate) root: PathBuf,
    pub(crate) relative_path: PathBuf,
    pub(crate) played_at: i64,
}

#[derive(Clone)]
pub(crate) struct RecordingTarget {
    pub(crate) source_id: SourceId,
    pub(crate) relative_path: PathBuf,
    pub(crate) absolute_path: PathBuf,
    pub(crate) last_refresh_at: Option<Instant>,
    pub(crate) last_file_len: u64,
    pub(crate) loaded_once: bool,
}

/// Tracks the latest in-flight recording waveform refresh.
#[derive(Clone)]
pub(crate) struct PendingRecordingWaveform {
    pub(crate) request_id: u64,
    pub(crate) source_id: SourceId,
    pub(crate) relative_path: PathBuf,
    pub(crate) absolute_path: PathBuf,
}

#[derive(Clone)]
pub(crate) struct PendingAudio {
    pub(crate) request_id: u64,
    pub(crate) source_id: SourceId,
    pub(crate) root: PathBuf,
    pub(crate) relative_path: PathBuf,
    pub(crate) intent: AudioLoadIntent,
}

#[derive(Clone, Debug)]
pub(crate) struct PendingPlayback {
    pub(crate) source_id: SourceId,
    pub(crate) relative_path: PathBuf,
    pub(crate) looped: bool,
    pub(crate) start_override: Option<f64>,
    /// When true, playback should use the freshly loaded audio even if browser focus points elsewhere.
    pub(crate) force_loaded_audio: bool,
}

/// Deferred loop-span retarget applied once the current loop cycle completes.
#[derive(Clone, Copy)]
pub(crate) struct PendingLoopRetarget {
    pub(crate) deadline: Instant,
    pub(crate) start_override: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum AudioLoadIntent {
    Selection,
}

#[derive(Clone)]
pub(crate) struct LoadedAudio {
    pub(crate) source_id: SourceId,
    pub(crate) root: PathBuf,
    pub(crate) relative_path: PathBuf,
    pub(crate) bytes: Arc<[u8]>,
    pub(crate) duration_seconds: f32,
    pub(crate) sample_rate: u32,
}
