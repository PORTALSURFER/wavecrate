use super::{WaveformState, WaveformViewport};

impl WaveformState {
    pub(in crate::native_app) fn viewport(&self) -> WaveformViewport {
        self.viewport
    }

    pub(in crate::native_app) fn visible_fraction(&self) -> f32 {
        self.viewport_scope().visible_fraction()
    }

    pub(in crate::native_app) fn fully_zoomed_out(&self) -> bool {
        !self.viewport_scope().is_zoomed_in()
    }

    pub(in crate::native_app) fn offset_fraction(&self) -> f32 {
        self.viewport_scope().offset_fraction()
    }

    pub(in crate::native_app) fn visible_ratio_for_absolute(&self, ratio: f32) -> Option<f32> {
        self.viewport_scope().visible_ratio_from_absolute(ratio)
    }
}
