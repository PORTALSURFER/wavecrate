use std::path::Path;

use super::{WaveformActiveDragKind, WaveformDrag, WaveformSelectionKind, WaveformState};
use wavecrate::selection::SelectionRange;

impl WaveformState {
    pub(in crate::native_app) fn edit_mark_ratio(&self) -> Option<f32> {
        self.edit_mark_ratio
    }

    pub(in crate::native_app) fn play_selection_flash_frames(&self) -> u8 {
        self.play_selection_flash_frames
    }

    pub(in crate::native_app) fn edit_selection_flash_frames(&self) -> u8 {
        self.edit_selection_flash_frames
    }

    pub(in crate::native_app) fn play_selection_denied_flash_frames(&self) -> u8 {
        self.play_selection_denied_flash_frames
    }

    pub(in crate::native_app) fn edit_selection_denied_flash_frames(&self) -> u8 {
        self.edit_selection_denied_flash_frames
    }

    pub(in crate::native_app) fn copy_flash_frames(&self) -> u8 {
        self.copy_flash_frames
    }

    pub(in crate::native_app) fn protected_source_error_flash_frames(&self) -> u8 {
        self.protected_source_error_flash_frames
    }

    pub(in crate::native_app) fn play_selection_flash_active(&self) -> bool {
        self.play_selection_flash_frames > 0
    }

    pub(in crate::native_app) fn flash_play_selection(&mut self) {
        self.play_selection_flash_frames = Self::selection_flash_frame_count();
        self.play_selection_denied_flash_frames = 0;
    }

    pub(in crate::native_app) fn flash_edit_selection(&mut self) {
        self.edit_selection_flash_frames = Self::selection_flash_frame_count();
        self.edit_selection_denied_flash_frames = 0;
    }

    pub(in crate::native_app) fn flash_copied_file(&mut self) {
        self.copy_flash_frames = Self::selection_flash_frame_count();
    }

    pub(in crate::native_app) fn flash_protected_source_error(&mut self) {
        self.protected_source_error_flash_frames = Self::denied_selection_flash_frame_count();
    }

    pub(in crate::native_app) fn flash_denied_selection(&mut self, kind: WaveformSelectionKind) {
        match kind {
            WaveformSelectionKind::Play => {
                if self.play_selection.is_some() {
                    self.play_selection_flash_frames = 0;
                    self.play_selection_denied_flash_frames =
                        Self::denied_selection_flash_frame_count();
                }
            }
            WaveformSelectionKind::Edit => {
                if self.edit_selection.is_some() {
                    self.edit_selection_flash_frames = 0;
                    self.edit_selection_denied_flash_frames =
                        Self::denied_selection_flash_frame_count();
                }
            }
        }
    }

    pub(in crate::native_app) fn flash_denied_selection_matching(
        &mut self,
        selection: SelectionRange,
        fallback_kind: WaveformSelectionKind,
    ) {
        if self.play_selection == Some(selection) {
            self.flash_denied_selection(WaveformSelectionKind::Play);
            return;
        }
        if self.edit_selection == Some(selection) {
            self.flash_denied_selection(WaveformSelectionKind::Edit);
            return;
        }
        self.flash_denied_selection(fallback_kind);
    }

    pub(in crate::native_app) fn flash_play_selection_if_current(
        &mut self,
        source_path: &Path,
        selection: SelectionRange,
    ) {
        if self.file.path == source_path && self.play_selection == Some(selection) {
            self.flash_play_selection();
        }
    }

    pub(in crate::native_app) fn active_drag_kind(&self) -> Option<WaveformActiveDragKind> {
        self.active_drag.map(WaveformDrag::kind)
    }
}
