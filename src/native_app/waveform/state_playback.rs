use super::WaveformState;

impl WaveformState {
    pub(in crate::native_app) fn is_playing(&self) -> bool {
        self.playing
    }

    pub(in crate::native_app) fn playhead_ratio(&self) -> Option<f32> {
        self.playhead_ratio
    }

    pub(in crate::native_app) fn play_mark_ratio(&self) -> Option<f32> {
        self.play_mark_ratio
    }

    pub(in crate::native_app) fn take_pending_playback_start(&mut self) -> Option<f32> {
        self.pending_playback_start.take()
    }

    pub(in crate::native_app) fn take_pending_sample_slide_frame_offset(&mut self) -> Option<i64> {
        self.pending_sample_slide_frame_offset.take()
    }

    pub(in crate::native_app) fn start_playback(&mut self, ratio: f32) {
        self.start_playback_with_marker(ratio, true);
    }

    pub(in crate::native_app) fn start_playback_without_marker(&mut self, ratio: f32) {
        self.start_playback_with_marker(ratio, false);
    }

    fn start_playback_with_marker(&mut self, ratio: f32, show_marker: bool) {
        let ratio = ratio.clamp(0.0, 1.0);
        self.playing = true;
        self.play_mark_ratio = show_marker.then_some(ratio);
        self.playhead_ratio = Some(ratio);
        self.zoom_anchor_ratio = ratio;
    }

    pub(in crate::native_app) fn set_playhead_ratio(&mut self, ratio: f32) {
        let ratio = ratio.clamp(0.0, 1.0);
        self.playhead_ratio = Some(ratio);
        self.zoom_anchor_ratio = ratio;
    }

    pub(in crate::native_app) fn stop_playback(&mut self) {
        self.playing = false;
        self.playhead_ratio = None;
    }
}
