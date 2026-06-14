use super::{WaveformActiveDragKind, WaveformDrag, WaveformState};

impl WaveformState {
    pub(in crate::native_app) fn edit_mark_ratio(&self) -> Option<f32> {
        self.edit_mark_ratio
    }

    pub(in crate::native_app) fn play_selection_flash_frames(&self) -> u8 {
        self.play_selection_flash_frames
    }

    pub(in crate::native_app) fn play_selection_flash_active(&self) -> bool {
        self.play_selection_flash_frames > 0
    }

    pub(in crate::native_app) fn flash_play_selection(&mut self) {
        self.play_selection_flash_frames = Self::selection_flash_frame_count();
    }

    pub(in crate::native_app) fn active_drag_kind(&self) -> Option<WaveformActiveDragKind> {
        self.active_drag.map(WaveformDrag::kind)
    }

    pub(in crate::native_app) fn hover_cursor_ratio(&self) -> Option<f32> {
        self.hover_cursor_ratio
    }
}
