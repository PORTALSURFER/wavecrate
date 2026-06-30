use crate::native_app::app::NativeAppState;
use wavecrate::audio::PlaybackRuntimeGainNormalization;

impl NativeAppState {
    pub(in crate::native_app) fn normalized_audition_gain_for_current_span(&self) -> f32 {
        if let Some((start, end)) = self.audio.current_playback_span {
            return self.normalized_audition_gain_for_span(start, end);
        }
        let (start, end) = self
            .waveform
            .current
            .play_selection()
            .filter(|selection| selection.width() > 0.0)
            .map(|selection| (selection.start(), selection.end()))
            .unwrap_or((0.0, 1.0));
        self.normalized_audition_gain_for_span(start, end)
    }

    pub(in crate::native_app) fn normalized_audition_gain_for_span(
        &self,
        start: f32,
        end: f32,
    ) -> f32 {
        if !self.audio.normalized_audition_enabled {
            return 1.0;
        }
        self.waveform
            .current
            .normalized_audition_gain_for_span(start, end)
    }

    pub(in crate::native_app) fn playback_gain_normalization_for_span(
        &self,
        start: f32,
        end: f32,
    ) -> Option<PlaybackRuntimeGainNormalization> {
        self.audio
            .normalized_audition_enabled
            .then_some(PlaybackRuntimeGainNormalization::new(start, end))
    }
}
