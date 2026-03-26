//! Waveform edit-selection and edit-fade action adapters.

use super::*;
use crate::app::controller::state::selection::EditFadeDragKind;
use crate::selection::SelectionRange;

impl AppController {
    /// Begin a right-click edit selection drag on the waveform.
    pub fn start_edit_selection_drag(&mut self, position: f32) {
        transport::start_edit_selection_drag(self, position);
    }

    /// Update the in-progress edit selection drag with the latest cursor position.
    pub fn update_edit_selection_drag(&mut self, position: f32, snap_override: bool) {
        transport::update_edit_selection_drag(self, position, snap_override);
    }

    /// Finish the edit selection drag and keep the edit selection active.
    pub fn finish_edit_selection_drag(&mut self) {
        transport::finish_edit_selection_drag(self);
    }

    /// Replace the edit selection without a drag gesture.
    pub fn set_edit_selection_range(&mut self, range: SelectionRange) {
        transport::set_edit_selection_range(self, range);
    }

    /// True while an edit selection drag gesture is active.
    pub fn is_edit_selection_dragging(&self) -> bool {
        transport::is_edit_selection_dragging(self)
    }

    /// Clear the edit selection while leaving playback selection intact.
    pub fn clear_edit_selection(&mut self) {
        transport::clear_edit_selection(self);
    }

    /// Set waveform edit selection range using 0..=1000 milli positions from UI actions.
    ///
    /// Edge-resize gestures preserve existing edit fades and keep them attached to the
    /// moved selection edge until the resized span becomes too small to fit them.
    pub fn set_waveform_edit_selection_range_milli(&mut self, start_milli: u16, end_milli: u16) {
        self.set_waveform_edit_selection_range_micros_with_edge_policy(
            micros_from_milli(start_milli),
            micros_from_milli(end_milli),
            false,
        );
    }

    /// Set waveform edit selection range with optional view-edge pinning for out-of-bounds drags.
    pub(crate) fn set_waveform_edit_selection_range_milli_with_edge_policy(
        &mut self,
        start_milli: u16,
        end_milli: u16,
        preserve_view_edge: bool,
    ) {
        self.set_waveform_edit_selection_range_micros_with_edge_policy(
            micros_from_milli(start_milli),
            micros_from_milli(end_milli),
            preserve_view_edge,
        );
    }

    /// Set waveform edit selection range with optional view-edge pinning for out-of-bounds drags.
    pub(crate) fn set_waveform_edit_selection_range_micros_with_edge_policy(
        &mut self,
        start_micros: u32,
        end_micros: u32,
        preserve_view_edge: bool,
    ) {
        let existing_range = current_edit_selection(self);
        let (start_micros, end_micros) =
            selection_updates::snap_edit_selection_range_micros(
                self,
                start_micros,
                end_micros,
                existing_range,
                preserve_view_edge,
            );
        let next_range = existing_range
            .map(|existing| {
                edit_selection::update_edit_selection_range_from_micros(
                    existing,
                    start_micros,
                    end_micros,
                )
            })
            .unwrap_or_else(|| selection_range_from_micros(start_micros, end_micros));
        if existing_range != Some(next_range) {
            self.begin_edit_selection_undo("Edit selection");
        }
        apply_edit_selection_update(self, existing_range, next_range);
    }

    /// Set waveform edit fade-in handle using a 0..=1000 milli position from UI actions.
    pub fn set_waveform_edit_fade_in_end_milli(&mut self, position_milli: u16) {
        self.set_waveform_edit_fade_in_end_micros(micros_from_milli(position_milli));
    }

    /// Set waveform edit fade-in handle using a 0..=1_000_000 micro position from UI actions.
    pub fn set_waveform_edit_fade_in_end_micros(&mut self, position_micros: u32) {
        self.update_waveform_edit_fade(EditFadeDragKind::InEnd, |range| {
            edit_fades::update_edit_fade_in_end_from_micros(range, position_micros)
        });
    }

    /// Set the waveform edit fade-in bottom handle using a 0..=1000 milli position.
    ///
    /// The UI action name still refers to the legacy mute-start handle, but the bottom
    /// handle now resizes the edit-selection start while keeping the fade-in end fixed.
    pub fn set_waveform_edit_fade_in_mute_start_milli(&mut self, position_milli: u16) {
        self.set_waveform_edit_fade_in_mute_start_micros(micros_from_milli(position_milli));
    }

    /// Set the waveform edit fade-in bottom handle using a 0..=1_000_000 micro position.
    pub fn set_waveform_edit_fade_in_mute_start_micros(&mut self, position_micros: u32) {
        self.update_waveform_edit_fade(EditFadeDragKind::InMuteStart, |range| {
            edit_fades::update_edit_fade_in_mute_start_from_micros(range, position_micros)
        });
    }

    /// Set waveform edit fade-in curve using a 0..=1000 milli value from UI actions.
    pub fn set_waveform_edit_fade_in_curve_milli(&mut self, curve_milli: u16) {
        self.update_waveform_edit_fade(EditFadeDragKind::InCurve, |range| {
            edit_fades::update_edit_fade_in_curve_from_milli(range, curve_milli)
        });
    }

    /// Set waveform edit fade-out handle using a 0..=1000 milli position from UI actions.
    pub fn set_waveform_edit_fade_out_start_milli(&mut self, position_milli: u16) {
        self.set_waveform_edit_fade_out_start_micros(micros_from_milli(position_milli));
    }

    /// Set waveform edit fade-out handle using a 0..=1_000_000 micro position from UI actions.
    pub fn set_waveform_edit_fade_out_start_micros(&mut self, position_micros: u32) {
        self.update_waveform_edit_fade(EditFadeDragKind::OutStart, |range| {
            edit_fades::update_edit_fade_out_start_from_micros(range, position_micros)
        });
    }

    /// Set the waveform edit fade-out bottom handle using a 0..=1000 milli position.
    ///
    /// The UI action name still refers to the legacy mute-end handle, but the bottom
    /// handle now resizes the edit-selection end while keeping the fade-out start fixed.
    pub fn set_waveform_edit_fade_out_mute_end_milli(&mut self, position_milli: u16) {
        self.set_waveform_edit_fade_out_mute_end_micros(micros_from_milli(position_milli));
    }

    /// Set the waveform edit fade-out bottom handle using a 0..=1_000_000 micro position.
    pub fn set_waveform_edit_fade_out_mute_end_micros(&mut self, position_micros: u32) {
        self.update_waveform_edit_fade(EditFadeDragKind::OutMuteEnd, |range| {
            edit_fades::update_edit_fade_out_mute_end_from_micros(range, position_micros)
        });
    }

    /// Set waveform edit fade-out curve using a 0..=1000 milli value from UI actions.
    pub fn set_waveform_edit_fade_out_curve_milli(&mut self, curve_milli: u16) {
        self.update_waveform_edit_fade(EditFadeDragKind::OutCurve, |range| {
            edit_fades::update_edit_fade_out_curve_from_milli(range, curve_milli)
        });
    }

    /// Clear any temporary edit-fade drag baseline captured for a live handle drag.
    pub fn finish_waveform_edit_fade_drag(&mut self) {
        clear_edit_fade_drag(self);
        self.commit_edit_selection_undo();
    }

    /// Clear waveform edit selection and keep waveform focus active.
    pub fn clear_waveform_edit_selection_with_focus(&mut self) {
        self.clear_edit_selection();
        self.focus_waveform();
    }

    fn update_waveform_edit_fade(
        &mut self,
        kind: EditFadeDragKind,
        update: impl FnOnce(SelectionRange) -> SelectionRange,
    ) {
        let Some(existing_range) = current_edit_selection(self) else {
            return;
        };
        let drag_range = edit_selection::prepare_edit_fade_drag_range(self, kind, existing_range);
        let next_range = update(drag_range);
        if existing_range != next_range {
            self.begin_edit_selection_undo("Edit selection");
        }
        apply_edit_selection_update(self, Some(existing_range), next_range);
    }
}
