//! Waveform and selection action facade methods for [`AppController`].

mod edit_fades;
mod edit_selection;
mod selection_updates;
mod shared;

use super::*;
use crate::app::controller::state::selection::EditFadeDragKind;

use edit_fades::{
    update_edit_fade_in_curve_from_milli, update_edit_fade_in_end_from_micros,
    update_edit_fade_in_mute_start_from_micros, update_edit_fade_out_curve_from_milli,
    update_edit_fade_out_mute_end_from_micros, update_edit_fade_out_start_from_micros,
};
pub(super) use edit_selection::clear_edit_fade_drag;
use edit_selection::{prepare_edit_fade_drag_range, update_edit_selection_range_from_micros};
use selection_updates::snap_waveform_selection_range_micros;
use shared::bpm_matches;

/// Equality epsilon used for normalized waveform cursor no-op detection.
const WAVEFORM_CURSOR_NOOP_EPSILON: f32 = 1.0e-6;

impl AppController {
    /// Begin a selection drag gesture at the given position.
    pub fn start_selection_drag(&mut self, position: f32) {
        transport::start_selection_drag(self, position);
    }

    /// Begin a right-click edit selection drag on the waveform.
    pub fn start_edit_selection_drag(&mut self, position: f32) {
        transport::start_edit_selection_drag(self, position);
    }

    /// Begin dragging a selection edge, optionally scaling for BPM.
    pub fn start_selection_edge_drag(
        &mut self,
        edge: crate::selection::SelectionEdge,
        bpm_scale: bool,
    ) -> bool {
        transport::start_selection_edge_drag(self, edge, bpm_scale)
    }

    /// Update the active selection drag with the latest position.
    pub fn update_selection_drag(&mut self, position: f32, snap_override: bool) {
        transport::update_selection_drag(self, position, snap_override);
    }

    /// Update the in-progress edit selection drag with the latest cursor position.
    pub fn update_edit_selection_drag(&mut self, position: f32, snap_override: bool) {
        transport::update_edit_selection_drag(self, position, snap_override);
    }

    /// Finish the active selection drag gesture.
    pub fn finish_selection_drag(&mut self) {
        transport::finish_selection_drag(self);
    }

    /// Finish the edit selection drag and keep the edit selection active.
    pub fn finish_edit_selection_drag(&mut self) {
        transport::finish_edit_selection_drag(self);
    }

    /// Set the active selection range.
    pub fn set_selection_range(&mut self, range: SelectionRange) {
        transport::set_selection_range(self, range);
    }

    /// Replace the edit selection without a drag gesture.
    pub fn set_edit_selection_range(&mut self, range: SelectionRange) {
        transport::set_edit_selection_range(self, range);
    }

    /// True while a selection drag gesture is active.
    pub fn is_selection_dragging(&self) -> bool {
        transport::is_selection_dragging(self)
    }

    /// True while an edit selection drag gesture is active.
    pub fn is_edit_selection_dragging(&self) -> bool {
        transport::is_edit_selection_dragging(self)
    }

    /// Clear the active selection.
    pub fn clear_selection(&mut self) {
        transport::clear_selection(self);
    }

    /// Clear the edit selection while leaving playback selection intact.
    pub fn clear_edit_selection(&mut self) {
        transport::clear_edit_selection(self);
    }

    /// Toggle loop playback for the current selection.
    pub fn toggle_loop(&mut self) {
        transport::toggle_loop(self);
    }

    /// Seek playback to the given normalized position.
    pub fn seek_to(&mut self, position: f32) {
        transport::seek_to(self, position);
    }

    /// Seek waveform/playback using a 0..=1000 milli position from UI actions.
    pub fn seek_waveform_milli(&mut self, position_milli: u16) {
        let normalized = normalized_from_milli(position_milli);
        self.seek_to(normalized);
        self.set_waveform_cursor(normalized);
        self.focus_waveform();
    }

    /// Queue a waveform seek from UI actions and defer commit-side playback work.
    pub fn queue_waveform_seek_milli(&mut self, position_milli: u16) {
        transport::queue_waveform_seek_milli(self, position_milli);
    }

    /// Set waveform cursor using a 0..=1000 milli position from UI actions.
    pub fn set_waveform_cursor_milli(&mut self, position_milli: u16) {
        let normalized = normalized_from_milli(position_milli);
        let cursor_unchanged =
            self.ui.waveform.cursor.is_some_and(|existing| {
                (existing - normalized).abs() <= WAVEFORM_CURSOR_NOOP_EPSILON
            });
        if cursor_unchanged && waveform_focus_active(self) {
            return;
        }
        self.set_waveform_cursor(normalized);
        self.focus_waveform();
    }

    /// Set waveform selection range using 0..=1000 milli positions from UI actions.
    pub fn set_waveform_selection_range_milli(&mut self, start_milli: u16, end_milli: u16) {
        self.set_waveform_selection_range_micros_with_edge_policy(
            micros_from_milli(start_milli),
            micros_from_milli(end_milli),
            false,
        );
    }

    /// Set waveform selection range from UI milli positions with optional view-edge pinning.
    pub(crate) fn set_waveform_selection_range_milli_with_edge_policy(
        &mut self,
        start_milli: u16,
        end_milli: u16,
        preserve_view_edge: bool,
    ) {
        self.set_waveform_selection_range_micros_with_edge_policy(
            micros_from_milli(start_milli),
            micros_from_milli(end_milli),
            preserve_view_edge,
        );
    }

    /// Set waveform selection range from UI micro positions with optional view-edge pinning.
    pub(crate) fn set_waveform_selection_range_micros_with_edge_policy(
        &mut self,
        start_micros: u32,
        end_micros: u32,
        preserve_view_edge: bool,
    ) {
        let existing_range = current_playback_selection(self);
        let (start_micros, end_micros) = snap_waveform_selection_range_micros(
            self,
            start_micros,
            end_micros,
            existing_range,
            preserve_view_edge,
        );
        let next_range = selection_range_from_micros(start_micros, end_micros);
        if existing_range == Some(next_range) && waveform_focus_active(self) {
            return;
        }
        self.set_selection_range(next_range);
        self.focus_waveform();
    }

    /// Set waveform selection range without BPM snapping and recalculate BPM for a 4-beat span.
    pub fn set_waveform_selection_range_milli_smart_scale(
        &mut self,
        start_milli: u16,
        end_milli: u16,
    ) {
        self.set_waveform_selection_range_micros_smart_scale(
            micros_from_milli(start_milli),
            micros_from_milli(end_milli),
        );
    }

    /// Set waveform selection range from UI micro positions and recalculate BPM for a 4-beat span.
    pub fn set_waveform_selection_range_micros_smart_scale(
        &mut self,
        start_micros: u32,
        end_micros: u32,
    ) {
        let existing_range = current_playback_selection(self);
        let next_range = selection_range_from_micros(start_micros, end_micros);
        let next_bpm =
            transport::scaled_selection_bpm(self, SMART_SCALE_SELECTION_BEATS, next_range);
        if existing_range == Some(next_range)
            && waveform_focus_active(self)
            && bpm_matches(self.ui.waveform.bpm_value, next_bpm)
        {
            return;
        }
        transport::set_selection_range_with_smart_scale(
            self,
            next_range,
            SMART_SCALE_SELECTION_BEATS,
        );
        self.focus_waveform();
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
        let (start_micros, end_micros) = snap_waveform_selection_range_micros(
            self,
            start_micros,
            end_micros,
            existing_range,
            preserve_view_edge,
        );
        let next_range = existing_range
            .map(|existing| {
                update_edit_selection_range_from_micros(existing, start_micros, end_micros)
            })
            .unwrap_or_else(|| selection_range_from_micros(start_micros, end_micros));
        apply_edit_selection_update(self, existing_range, next_range);
    }

    /// Set waveform edit fade-in handle using a 0..=1000 milli position from UI actions.
    pub fn set_waveform_edit_fade_in_end_milli(&mut self, position_milli: u16) {
        self.set_waveform_edit_fade_in_end_micros(micros_from_milli(position_milli));
    }

    /// Set waveform edit fade-in handle using a 0..=1_000_000 micro position from UI actions.
    pub fn set_waveform_edit_fade_in_end_micros(&mut self, position_micros: u32) {
        self.update_waveform_edit_fade(EditFadeDragKind::InEnd, |range| {
            update_edit_fade_in_end_from_micros(range, position_micros)
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
            update_edit_fade_in_mute_start_from_micros(range, position_micros)
        });
    }

    /// Set waveform edit fade-in curve using a 0..=1000 milli value from UI actions.
    pub fn set_waveform_edit_fade_in_curve_milli(&mut self, curve_milli: u16) {
        self.update_waveform_edit_fade(EditFadeDragKind::InCurve, |range| {
            update_edit_fade_in_curve_from_milli(range, curve_milli)
        });
    }

    /// Set waveform edit fade-out handle using a 0..=1000 milli position from UI actions.
    pub fn set_waveform_edit_fade_out_start_milli(&mut self, position_milli: u16) {
        self.set_waveform_edit_fade_out_start_micros(micros_from_milli(position_milli));
    }

    /// Set waveform edit fade-out handle using a 0..=1_000_000 micro position from UI actions.
    pub fn set_waveform_edit_fade_out_start_micros(&mut self, position_micros: u32) {
        self.update_waveform_edit_fade(EditFadeDragKind::OutStart, |range| {
            update_edit_fade_out_start_from_micros(range, position_micros)
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
            update_edit_fade_out_mute_end_from_micros(range, position_micros)
        });
    }

    /// Set waveform edit fade-out curve using a 0..=1000 milli value from UI actions.
    pub fn set_waveform_edit_fade_out_curve_milli(&mut self, curve_milli: u16) {
        self.update_waveform_edit_fade(EditFadeDragKind::OutCurve, |range| {
            update_edit_fade_out_curve_from_milli(range, curve_milli)
        });
    }

    /// Clear any temporary edit-fade drag baseline captured for a live handle drag.
    pub fn finish_waveform_edit_fade_drag(&mut self) {
        clear_edit_fade_drag(self);
    }

    /// Clear waveform selection and keep waveform focus active.
    pub fn clear_waveform_selection_with_focus(&mut self) {
        self.clear_selection();
        self.focus_waveform();
    }

    /// Clear waveform edit selection and keep waveform focus active.
    pub fn clear_waveform_edit_selection_with_focus(&mut self) {
        self.clear_edit_selection();
        self.focus_waveform();
    }

    /// Zoom waveform from UI actions using clamped step counts and focus retention.
    pub fn zoom_waveform_steps_from_ui(&mut self, zoom_in: bool, steps: u8) {
        self.zoom_waveform_steps_from_ui_with_anchor(zoom_in, steps, None);
    }

    /// Zoom waveform from UI actions using an optional pointer anchor ratio.
    ///
    /// `anchor_ratio_micros` uses deterministic micros (`0..=1_000_000`) where
    /// `0` is the left edge and `1_000_000` is the right edge of the current view.
    pub fn zoom_waveform_steps_from_ui_with_anchor(
        &mut self,
        zoom_in: bool,
        steps: u8,
        anchor_ratio_micros: Option<u32>,
    ) {
        let before_view = self.ui.waveform.view;
        let focused_before = waveform_focus_active(self);
        let cursor_focus = if let Some(anchor_ratio_micros) = anchor_ratio_micros {
            let ratio =
                f64::from(anchor_ratio_micros.min(1_000_000)) / WAVEFORM_ANCHOR_RATIO_MICROS_SCALE;
            let focus =
                (before_view.start + (before_view.end - before_view.start) * ratio).clamp(0.0, 1.0);
            self.set_waveform_cursor_from_hover(focus as f32);
            focus
        } else if let Some(cursor) = self.ui.waveform.cursor {
            f64::from(cursor)
        } else {
            let center = ((before_view.start + before_view.end) * 0.5).clamp(0.0, 1.0);
            self.set_waveform_cursor(center as f32);
            center
        };
        self.zoom_waveform_steps_with_factor(
            zoom_in,
            zoom_steps_from_ui(steps),
            Some(cursor_focus),
            None,
            false,
            false,
        );
        if focused_before && !waveform_view_changed(before_view, self.ui.waveform.view) {
            return;
        }
        self.focus_waveform_context();
    }

    /// Zoom waveform to current selection while preserving waveform focus.
    pub fn zoom_waveform_to_selection_with_focus(&mut self) {
        self.zoom_waveform_to_selection();
        self.focus_waveform();
    }

    /// Scroll waveform viewport to a normalized center while preserving waveform focus.
    pub fn scroll_waveform_view_with_focus(&mut self, center_micros: u32) {
        self.scroll_waveform_view(
            f64::from(center_micros.min(1_000_000)) / WAVEFORM_ANCHOR_RATIO_MICROS_SCALE,
        );
        self.focus_waveform();
    }

    /// Reset waveform zoom to full range while preserving waveform focus.
    pub fn zoom_waveform_full_with_focus(&mut self) {
        self.zoom_waveform_full();
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
        let drag_range = prepare_edit_fade_drag_range(self, kind, existing_range);
        let next_range = update(drag_range);
        apply_edit_selection_update(self, Some(existing_range), next_range);
    }
}

fn current_playback_selection(controller: &AppController) -> Option<SelectionRange> {
    controller
        .selection_state
        .range
        .range()
        .or(controller.ui.waveform.selection)
}

fn current_edit_selection(controller: &AppController) -> Option<SelectionRange> {
    controller
        .selection_state
        .edit_range
        .range()
        .or(controller.ui.waveform.edit_selection)
}

fn apply_edit_selection_update(
    controller: &mut AppController,
    existing_range: Option<SelectionRange>,
    next_range: SelectionRange,
) {
    if existing_range == Some(next_range) && waveform_focus_active(controller) {
        return;
    }
    controller
        .selection_state
        .edit_range
        .set_range(Some(next_range));
    controller.apply_edit_selection(Some(next_range));
    controller.focus_waveform();
}

/// Convert one UI waveform milli value (`0..=1000`) into normalized `[0.0, 1.0]`.
pub(super) fn normalized_from_milli(value: u16) -> f32 {
    shared::normalized_from_milli(value)
}

/// Convert one UI waveform micro value (`0..=1_000_000`) back into normalized space.
pub(super) fn normalized_from_micros(value: u32) -> f32 {
    shared::normalized_from_micros(value)
}

/// Convert one normalized waveform position into UI micro space (`0..=1_000_000`).
pub(super) fn normalized_to_micros(value: f32) -> u32 {
    shared::normalized_to_micros(value)
}

/// Convert one UI waveform milli value (`0..=1000`) into micro space.
pub(super) fn micros_from_milli(value: u16) -> u32 {
    shared::micros_from_milli(value)
}

/// Build a normalized selection range from two UI waveform milli values (`0..=1000`).
pub(super) fn selection_range_from_milli(start_milli: u16, end_milli: u16) -> SelectionRange {
    shared::selection_range_from_milli(start_milli, end_milli)
}

/// Build a normalized selection range from two UI waveform micro values (`0..=1_000_000`).
pub(super) fn selection_range_from_micros(start_micros: u32, end_micros: u32) -> SelectionRange {
    shared::selection_range_from_micros(start_micros, end_micros)
}

/// Clamp UI-provided waveform zoom steps to at least one step.
pub(super) fn zoom_steps_from_ui(steps: u8) -> u32 {
    shared::zoom_steps_from_ui(steps)
}

/// Return whether waveform focus is already active.
pub(super) fn waveform_focus_active(controller: &AppController) -> bool {
    shared::waveform_focus_active(controller)
}

/// Return whether two waveform views differ enough to warrant follow-up focus work.
pub(super) fn waveform_view_changed(
    before: crate::app::state::WaveformView,
    after: crate::app::state::WaveformView,
) -> bool {
    shared::waveform_view_changed(before, after)
}
