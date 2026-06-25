use super::{WaveformState, WaveformViewport};

impl WaveformState {
    pub(in crate::native_app) fn viewport(&self) -> WaveformViewport {
        self.viewport
    }

    pub(in crate::native_app) fn visible_fraction(&self) -> f32 {
        self.viewport.visible_fraction(self.file.frames)
    }

    pub(in crate::native_app) fn fully_zoomed_out(&self) -> bool {
        !self.viewport.is_zoomed_in(self.file.frames)
            && !self.viewport.extends_beyond_audio(self.file.frames)
    }

    pub(in crate::native_app) fn offset_fraction(&self) -> f32 {
        self.viewport.offset_fraction(self.file.frames)
    }

    pub(in crate::native_app) fn visible_ratio_for_absolute(&self, ratio: f32) -> Option<f32> {
        self.viewport
            .visible_ratio_from_absolute(self.file.frames, ratio)
    }
}
