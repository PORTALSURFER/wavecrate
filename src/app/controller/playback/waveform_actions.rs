//! Waveform and selection action facade methods for [`AppController`].

use super::*;

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
        let existing_range = self
            .selection_state
            .range
            .range()
            .or(self.ui.waveform.selection);
        let (start_milli, end_milli) =
            snap_waveform_selection_resize_milli(self, start_milli, end_milli, existing_range);
        let next_range = selection_range_from_milli(start_milli, end_milli);
        if existing_range == Some(next_range) && waveform_focus_active(self) {
            return;
        }
        self.set_selection_range(next_range);
        self.focus_waveform();
    }

    /// Set waveform edit selection range using 0..=1000 milli positions from UI actions.
    pub fn set_waveform_edit_selection_range_milli(&mut self, start_milli: u16, end_milli: u16) {
        let next_range = selection_range_from_milli(start_milli, end_milli);
        let existing_range = self
            .selection_state
            .edit_range
            .range()
            .or(self.ui.waveform.edit_selection);
        if existing_range == Some(next_range) && waveform_focus_active(self) {
            return;
        }
        self.set_edit_selection_range(next_range);
        self.focus_waveform();
    }

    /// Set waveform edit fade-in handle using a 0..=1000 milli position from UI actions.
    pub fn set_waveform_edit_fade_in_end_milli(&mut self, position_milli: u16) {
        let Some(existing_range) = self
            .selection_state
            .edit_range
            .range()
            .or(self.ui.waveform.edit_selection)
        else {
            return;
        };
        let next_range = update_edit_fade_in_end_from_milli(existing_range, position_milli);
        if existing_range == next_range && waveform_focus_active(self) {
            return;
        }
        self.selection_state.edit_range.set_range(Some(next_range));
        self.apply_edit_selection(Some(next_range));
        self.focus_waveform();
    }

    /// Set waveform edit fade-out handle using a 0..=1000 milli position from UI actions.
    pub fn set_waveform_edit_fade_out_start_milli(&mut self, position_milli: u16) {
        let Some(existing_range) = self
            .selection_state
            .edit_range
            .range()
            .or(self.ui.waveform.edit_selection)
        else {
            return;
        };
        let next_range = update_edit_fade_out_start_from_milli(existing_range, position_milli);
        if existing_range == next_range && waveform_focus_active(self) {
            return;
        }
        self.selection_state.edit_range.set_range(Some(next_range));
        self.apply_edit_selection(Some(next_range));
        self.focus_waveform();
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

    /// Reset waveform zoom to full range while preserving waveform focus.
    pub fn zoom_waveform_full_with_focus(&mut self) {
        self.zoom_waveform_full();
        self.focus_waveform();
    }
}

/// Convert one UI waveform milli value (`0..=1000`) into normalized `[0.0, 1.0]`.
pub(super) fn normalized_from_milli(value: u16) -> f32 {
    (value.min(1000) as f32) / 1000.0
}

/// Convert one normalized waveform position into UI milli space (`0..=1000`).
pub(super) fn normalized_to_milli(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

/// Build a normalized selection range from two UI waveform milli values (`0..=1000`).
pub(super) fn selection_range_from_milli(start_milli: u16, end_milli: u16) -> SelectionRange {
    SelectionRange::new(
        normalized_from_milli(start_milli),
        normalized_from_milli(end_milli),
    )
}

/// Snap waveform selection-resize milli values to BPM steps for edge-resize gestures.
fn snap_waveform_selection_resize_milli(
    controller: &AppController,
    start_milli: u16,
    end_milli: u16,
    existing_range: Option<SelectionRange>,
) -> (u16, u16) {
    let mut start = start_milli.min(1000);
    let mut end = end_milli.min(1000);
    let Some(step) = waveform_bpm_snap_step(controller) else {
        return (start, end);
    };
    let Some(existing) = existing_range else {
        return (start, end);
    };
    let existing_start = normalized_to_milli(existing.start());
    let existing_end = normalized_to_milli(existing.end());
    if start == existing_end {
        end = snap_milli_to_bpm_step(end, step);
    } else if end == existing_start {
        start = snap_milli_to_bpm_step(start, step);
    } else if start == existing_start {
        end = snap_milli_to_bpm_step(end, step);
    } else if end == existing_end {
        start = snap_milli_to_bpm_step(start, step);
    }
    (start, end)
}

/// Resolve the normalized BPM snap step used for waveform selection gestures.
fn waveform_bpm_snap_step(controller: &AppController) -> Option<f32> {
    if !controller.ui.waveform.bpm_snap_enabled {
        return None;
    }
    let bpm = controller.ui.waveform.bpm_value?;
    if !bpm.is_finite() || bpm <= 0.0 {
        return None;
    }
    let duration = controller
        .sample_view
        .wav
        .loaded_audio
        .as_ref()
        .map(|audio| audio.duration_seconds)?;
    if !duration.is_finite() || duration <= 0.0 {
        return None;
    }
    let step = 60.0 / bpm / duration;
    (step.is_finite() && step > 0.0).then_some(step)
}

/// Snap one normalized milli position to the closest BPM step.
fn snap_milli_to_bpm_step(value_milli: u16, step: f32) -> u16 {
    if !step.is_finite() || step <= 0.0 {
        return value_milli.min(1000);
    }
    let normalized = normalized_from_milli(value_milli);
    let snapped = (normalized / step).round() * step;
    normalized_to_milli(snapped)
}

/// Update edit fade-in length from one absolute waveform milli handle position.
pub(super) fn update_edit_fade_in_end_from_milli(
    range: SelectionRange,
    position_milli: u16,
) -> SelectionRange {
    let width = range.width();
    if width <= 0.0 {
        return range;
    }
    let start = range.start();
    let end = range.end();
    let clamped_position = normalized_from_milli(position_milli).clamp(start, end);
    let length = ((clamped_position - start) / width).clamp(0.0, 1.0);
    let curve = range.fade_in().map(|fade| fade.curve).unwrap_or(0.5);
    range.with_fade_in(length, curve)
}

/// Update edit fade-out length from one absolute waveform milli handle position.
pub(super) fn update_edit_fade_out_start_from_milli(
    range: SelectionRange,
    position_milli: u16,
) -> SelectionRange {
    let width = range.width();
    if width <= 0.0 {
        return range;
    }
    let start = range.start();
    let end = range.end();
    let clamped_position = normalized_from_milli(position_milli).clamp(start, end);
    let length = ((end - clamped_position) / width).clamp(0.0, 1.0);
    let curve = range.fade_out().map(|fade| fade.curve).unwrap_or(0.5);
    range.with_fade_out(length, curve)
}

/// Clamp UI-provided waveform zoom steps to at least one step.
pub(super) fn zoom_steps_from_ui(steps: u8) -> u32 {
    u32::from(steps.max(1))
}

/// Return whether waveform focus is already active.
pub(super) fn waveform_focus_active(controller: &AppController) -> bool {
    controller.ui.focus.context == crate::app::state::FocusContext::Waveform
}

/// Return whether two waveform views differ enough to warrant follow-up focus work.
pub(super) fn waveform_view_changed(
    before: crate::app::state::WaveformView,
    after: crate::app::state::WaveformView,
) -> bool {
    (before.start - after.start).abs() > WAVEFORM_VIEW_NOOP_EPSILON
        || (before.end - after.end).abs() > WAVEFORM_VIEW_NOOP_EPSILON
}
