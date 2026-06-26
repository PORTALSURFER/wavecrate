use radiant::gui::types::Point;

use super::{WAVEFORM_HEIGHT, WAVEFORM_WIDTH, WaveformState, WaveformViewport};

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

    pub(in crate::native_app) fn play_selection_context_menu_anchor(&self) -> Option<Point> {
        let selection = self
            .play_selection()
            .filter(|selection| selection.width() > 0.0)?;
        if let Some(position) = self.context_menu_pointer_position {
            return Some(position);
        }
        let visible_ratio = self
            .visible_ratio_for_absolute(selection.start())
            .or_else(|| self.visible_ratio_for_absolute(selection.end()))
            .unwrap_or(0.5)
            .clamp(0.05, 0.95);
        Some(Point::new(
            visible_ratio * WAVEFORM_WIDTH as f32,
            WAVEFORM_HEIGHT as f32 * 0.5,
        ))
    }
}
