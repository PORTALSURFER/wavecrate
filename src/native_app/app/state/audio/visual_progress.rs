use std::time::{Duration, Instant};

use wavecrate::audio::PlaybackRuntimeProgress;

use super::{AudioAppState, SamplePlaybackSession};

const PLAYBACK_VISUAL_PROGRESS_ELAPSED_TOLERANCE: Duration = Duration::from_millis(8);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) struct PlaybackVisualProgress {
    pub(in crate::native_app) anchor_ratio: f32,
    pub(in crate::native_app) anchor_at: Instant,
    pub(in crate::native_app) anchor_animation_time: Option<Duration>,
    pub(in crate::native_app) span: Option<(f32, f32)>,
    pub(in crate::native_app) looping: bool,
}

impl AudioAppState {
    pub(in crate::native_app) fn set_playback_progress(
        &mut self,
        progress: PlaybackRuntimeProgress,
    ) {
        self.playback_progress = progress;
        self.sync_playback_visual_progress(false);
    }

    pub(in crate::native_app) fn set_authoritative_playback_progress(
        &mut self,
        progress: PlaybackRuntimeProgress,
    ) {
        self.playback_progress = progress;
        self.sync_playback_visual_progress(true);
    }

    pub(in crate::native_app) fn set_started_playback_progress(
        &mut self,
        progress: PlaybackRuntimeProgress,
    ) {
        self.set_authoritative_playback_progress(progress);
    }

    pub(in crate::native_app) fn clear_playback_progress(&mut self) {
        self.playback_progress = PlaybackRuntimeProgress::default();
        self.playback_visual_progress = None;
        self.pending_playback_progress_polls.clear();
    }

    pub(in crate::native_app) fn reset_playback_visual_progress(
        &mut self,
        anchor_ratio: f32,
        looping: bool,
    ) {
        let span = self.playback_visual_span();
        self.playback_visual_progress = Some(PlaybackVisualProgress {
            anchor_ratio: anchor_ratio.clamp(0.0, 1.0),
            anchor_at: Instant::now(),
            anchor_animation_time: None,
            span,
            looping,
        });
    }

    fn sync_playback_visual_progress(&mut self, force: bool) {
        if !self.playback_progress.active {
            self.playback_visual_progress = None;
            return;
        }
        let Some(progress) = self.playback_progress.progress else {
            return;
        };
        let span = self.playback_visual_span();
        let looping = self.playback_progress.looping;
        let clock_mismatch = self
            .playback_visual_progress
            .is_some_and(|clock| clock.span != span || clock.looping != looping);
        let delayed_unpainted_anchor = self.delayed_unpainted_playback_anchor_needs_refresh();
        if force
            || self.playback_visual_progress.is_none()
            || clock_mismatch
            || delayed_unpainted_anchor
        {
            self.reset_playback_visual_progress(progress, looping);
        }
    }

    fn playback_visual_span(&self) -> Option<(f32, f32)> {
        // Requested bounds can lead the audio runtime while a live retarget is
        // queued. Project the playhead through the last audible confirmation.
        self.sample_playback_session
            .as_ref()
            .filter(|session| session.request.visibility.updates_waveform_playhead())
            .map(SamplePlaybackSession::confirmed_span)
            .or(self.current_playback_span)
    }

    fn delayed_unpainted_playback_anchor_needs_refresh(&self) -> bool {
        let Some(clock) = self.playback_visual_progress else {
            return false;
        };
        if clock.anchor_animation_time.is_some() {
            return false;
        }
        let Some(runtime_elapsed) = self.playback_progress.elapsed else {
            return false;
        };
        runtime_elapsed.saturating_add(PLAYBACK_VISUAL_PROGRESS_ELAPSED_TOLERANCE)
            >= clock.anchor_at.elapsed()
    }
}
