//! Waveform and selection action facade methods for [`AppController`].

use super::*;
use crate::app::controller::state::selection::{EditFadeDragKind, EditFadeDragState};

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
            snap_waveform_selection_range_milli(self, start_milli, end_milli, existing_range);
        let next_range = selection_range_from_milli(start_milli, end_milli);
        if existing_range == Some(next_range) && waveform_focus_active(self) {
            return;
        }
        self.set_selection_range(next_range);
        self.focus_waveform();
    }

    /// Set waveform edit selection range using 0..=1000 milli positions from UI actions.
    ///
    /// Edge-resize gestures preserve existing edit fades and keep them attached to the
    /// moved selection edge until the resized span becomes too small to fit them.
    pub fn set_waveform_edit_selection_range_milli(&mut self, start_milli: u16, end_milli: u16) {
        let existing_range = self
            .selection_state
            .edit_range
            .range()
            .or(self.ui.waveform.edit_selection);
        let (start_milli, end_milli) =
            snap_waveform_selection_range_milli(self, start_milli, end_milli, existing_range);
        let next_range = existing_range
            .map(|existing| {
                update_edit_selection_range_from_milli(existing, start_milli, end_milli)
            })
            .unwrap_or_else(|| selection_range_from_milli(start_milli, end_milli));
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
        let drag_range =
            prepare_edit_fade_drag_range(self, EditFadeDragKind::InEnd, existing_range);
        let next_range = update_edit_fade_in_end_from_milli(drag_range, position_milli);
        if existing_range == next_range && waveform_focus_active(self) {
            return;
        }
        self.selection_state.edit_range.set_range(Some(next_range));
        self.apply_edit_selection(Some(next_range));
        self.focus_waveform();
    }

    /// Set the waveform edit fade-in bottom handle using a 0..=1000 milli position.
    ///
    /// The UI action name still refers to the legacy mute-start handle, but the bottom
    /// handle now resizes the edit-selection start while keeping the fade-in end fixed.
    pub fn set_waveform_edit_fade_in_mute_start_milli(&mut self, position_milli: u16) {
        let Some(existing_range) = self
            .selection_state
            .edit_range
            .range()
            .or(self.ui.waveform.edit_selection)
        else {
            return;
        };
        let drag_range =
            prepare_edit_fade_drag_range(self, EditFadeDragKind::InMuteStart, existing_range);
        let next_range = update_edit_fade_in_mute_start_from_milli(drag_range, position_milli);
        if existing_range == next_range && waveform_focus_active(self) {
            return;
        }
        self.selection_state.edit_range.set_range(Some(next_range));
        self.apply_edit_selection(Some(next_range));
        self.focus_waveform();
    }

    /// Set waveform edit fade-in curve using a 0..=1000 milli value from UI actions.
    pub fn set_waveform_edit_fade_in_curve_milli(&mut self, curve_milli: u16) {
        let Some(existing_range) = self
            .selection_state
            .edit_range
            .range()
            .or(self.ui.waveform.edit_selection)
        else {
            return;
        };
        let drag_range =
            prepare_edit_fade_drag_range(self, EditFadeDragKind::InCurve, existing_range);
        let next_range = update_edit_fade_in_curve_from_milli(drag_range, curve_milli);
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
        let drag_range =
            prepare_edit_fade_drag_range(self, EditFadeDragKind::OutStart, existing_range);
        let next_range = update_edit_fade_out_start_from_milli(drag_range, position_milli);
        if existing_range == next_range && waveform_focus_active(self) {
            return;
        }
        self.selection_state.edit_range.set_range(Some(next_range));
        self.apply_edit_selection(Some(next_range));
        self.focus_waveform();
    }

    /// Set the waveform edit fade-out bottom handle using a 0..=1000 milli position.
    ///
    /// The UI action name still refers to the legacy mute-end handle, but the bottom
    /// handle now resizes the edit-selection end while keeping the fade-out start fixed.
    pub fn set_waveform_edit_fade_out_mute_end_milli(&mut self, position_milli: u16) {
        let Some(existing_range) = self
            .selection_state
            .edit_range
            .range()
            .or(self.ui.waveform.edit_selection)
        else {
            return;
        };
        let drag_range =
            prepare_edit_fade_drag_range(self, EditFadeDragKind::OutMuteEnd, existing_range);
        let next_range = update_edit_fade_out_mute_end_from_milli(drag_range, position_milli);
        if existing_range == next_range && waveform_focus_active(self) {
            return;
        }
        self.selection_state.edit_range.set_range(Some(next_range));
        self.apply_edit_selection(Some(next_range));
        self.focus_waveform();
    }

    /// Set waveform edit fade-out curve using a 0..=1000 milli value from UI actions.
    pub fn set_waveform_edit_fade_out_curve_milli(&mut self, curve_milli: u16) {
        let Some(existing_range) = self
            .selection_state
            .edit_range
            .range()
            .or(self.ui.waveform.edit_selection)
        else {
            return;
        };
        let drag_range =
            prepare_edit_fade_drag_range(self, EditFadeDragKind::OutCurve, existing_range);
        let next_range = update_edit_fade_out_curve_from_milli(drag_range, curve_milli);
        if existing_range == next_range && waveform_focus_active(self) {
            return;
        }
        self.selection_state.edit_range.set_range(Some(next_range));
        self.apply_edit_selection(Some(next_range));
        self.focus_waveform();
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

/// Update an edit-selection range from UI milli values while preserving live fade state.
pub(super) fn update_edit_selection_range_from_milli(
    existing: SelectionRange,
    start_milli: u16,
    end_milli: u16,
) -> SelectionRange {
    let next = selection_range_from_milli(start_milli, end_milli);
    preserve_edit_selection_effects(
        existing,
        next,
        resized_edit_selection_edge(existing, start_milli, end_milli),
    )
}

/// Reuse the range captured when one edit-fade drag started until that drag ends.
fn prepare_edit_fade_drag_range(
    controller: &mut AppController,
    kind: EditFadeDragKind,
    existing_range: SelectionRange,
) -> SelectionRange {
    match controller.selection_state.edit_fade_drag {
        Some(state) if state.kind == kind => state.baseline,
        _ => {
            controller.selection_state.edit_fade_drag = Some(EditFadeDragState {
                kind,
                baseline: existing_range,
            });
            existing_range
        }
    }
}

/// Drop any retained edit-fade drag snapshot when the gesture is no longer active.
pub(super) fn clear_edit_fade_drag(controller: &mut AppController) {
    controller.selection_state.edit_fade_drag = None;
}

/// Return which edit-selection edge a raw UI range update is dragging, if any.
fn resized_edit_selection_edge(
    existing: SelectionRange,
    start_milli: u16,
    end_milli: u16,
) -> Option<crate::selection::SelectionEdge> {
    let existing_start = normalized_to_milli(existing.start());
    let existing_end = normalized_to_milli(existing.end());
    if start_milli == existing_end || end_milli == existing_end {
        Some(crate::selection::SelectionEdge::Start)
    } else if start_milli == existing_start || end_milli == existing_start {
        Some(crate::selection::SelectionEdge::End)
    } else {
        None
    }
}

/// Rebuild a resized edit-selection range while keeping existing fades where possible.
fn preserve_edit_selection_effects(
    existing: SelectionRange,
    next: SelectionRange,
    moved_edge: Option<crate::selection::SelectionEdge>,
) -> SelectionRange {
    let mut rebuilt = SelectionRange::new(next.start(), next.end()).with_gain(existing.gain());
    let next_width = rebuilt.width();
    if next_width <= f32::EPSILON {
        return rebuilt;
    }
    let existing_width = existing.width();
    if existing_width <= f32::EPSILON {
        return rebuilt;
    }
    let fade_in = existing.fade_in();
    let fade_out = existing.fade_out();
    let fade_in_abs = fade_in
        .map(|fade| existing_width * fade.length)
        .unwrap_or(0.0);
    let fade_out_abs = fade_out
        .map(|fade| existing_width * fade.length)
        .unwrap_or(0.0);
    let (next_fade_in_abs, next_fade_out_abs) =
        clamped_preserved_edit_fade_lengths(next_width, fade_in_abs, fade_out_abs, moved_edge);
    if let Some(fade) = fade_in {
        rebuilt = rebuilt.with_fade_in(next_fade_in_abs / next_width, fade.curve);
        if fade.mute > 0.0 {
            rebuilt = rebuilt.with_fade_in_mute((existing_width * fade.mute) / next_width);
        }
    }
    if let Some(fade) = fade_out {
        rebuilt = rebuilt.with_fade_out(next_fade_out_abs / next_width, fade.curve);
        if fade.mute > 0.0 {
            rebuilt = rebuilt.with_fade_out_mute((existing_width * fade.mute) / next_width);
        }
    }
    rebuilt
}

/// Clamp preserved fade lengths for one edit-selection resize, prioritizing the fixed edge.
fn clamped_preserved_edit_fade_lengths(
    next_width: f32,
    fade_in_abs: f32,
    fade_out_abs: f32,
    moved_edge: Option<crate::selection::SelectionEdge>,
) -> (f32, f32) {
    if next_width <= f32::EPSILON {
        return (0.0, 0.0);
    }
    match moved_edge {
        Some(crate::selection::SelectionEdge::Start) => {
            let keep_out = fade_out_abs.min(next_width);
            let keep_in = fade_in_abs.min((next_width - keep_out).max(0.0));
            (keep_in, keep_out)
        }
        Some(crate::selection::SelectionEdge::End) => {
            let keep_in = fade_in_abs.min(next_width);
            let keep_out = fade_out_abs.min((next_width - keep_in).max(0.0));
            (keep_in, keep_out)
        }
        None => {
            let total = fade_in_abs + fade_out_abs;
            if total <= next_width || total <= f32::EPSILON {
                (fade_in_abs.min(next_width), fade_out_abs.min(next_width))
            } else {
                let scale = next_width / total;
                (fade_in_abs * scale, fade_out_abs * scale)
            }
        }
    }
}

/// Snap waveform selection milli values to BPM steps for translated or resized ranges.
fn snap_waveform_selection_range_milli(
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
    if translated_waveform_selection_range(start, end, existing_start, existing_end) {
        let width = existing_end.saturating_sub(existing_start);
        let snapped_start = snap_milli_to_bpm_step(start, step).min(1000u16.saturating_sub(width));
        let snapped_end = snapped_start.saturating_add(width).min(1000);
        return (snapped_start, snapped_end);
    }
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

/// Return whether the proposed range is a pure translation of the existing range.
fn translated_waveform_selection_range(
    start_milli: u16,
    end_milli: u16,
    existing_start_milli: u16,
    existing_end_milli: u16,
) -> bool {
    let width = end_milli.abs_diff(start_milli);
    let existing_width = existing_end_milli.abs_diff(existing_start_milli);
    width == existing_width
        && start_milli != existing_start_milli
        && end_milli != existing_end_milli
        && start_milli != end_milli
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

/// Rebuild an edit range while preserving gain and any surviving fade parameters.
fn rebuild_edit_range(
    range: SelectionRange,
    start: f32,
    end: f32,
    fade_in: Option<crate::selection::FadeParams>,
    fade_out: Option<crate::selection::FadeParams>,
) -> SelectionRange {
    let mut next = SelectionRange::new(start, end).with_gain(range.gain());
    if let Some(fade) = fade_in {
        next = next.with_fade_in(fade.length, fade.curve);
        if fade.mute > 0.0 {
            next = next.with_fade_in_mute(fade.mute);
        }
    }
    if let Some(fade) = fade_out {
        next = next.with_fade_out(fade.length, fade.curve);
        if fade.mute > 0.0 {
            next = next.with_fade_out_mute(fade.mute);
        }
    }
    next
}

/// Update the edit-selection start from the fade-in bottom handle position.
pub(super) fn update_edit_fade_in_mute_start_from_milli(
    range: SelectionRange,
    position_milli: u16,
) -> SelectionRange {
    let Some(fade_in) = range.fade_in() else {
        return range;
    };
    let width = range.width();
    if width <= f32::EPSILON {
        return range;
    }
    let fade_in_end = range.start() + (width * fade_in.length);
    let new_start = normalized_from_milli(position_milli).clamp(0.0, fade_in_end);
    let new_width = (range.end() - new_start).max(0.0);
    let new_length = if new_width <= f32::EPSILON {
        0.0
    } else {
        ((fade_in_end - new_start) / new_width).clamp(0.0, 1.0)
    };
    let next_fade_in = crate::selection::FadeParams::with_curve(new_length, fade_in.curve);
    rebuild_edit_range(
        range,
        new_start,
        range.end(),
        Some(next_fade_in),
        range.fade_out(),
    )
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

/// Update the edit-selection end from the fade-out bottom handle position.
pub(super) fn update_edit_fade_out_mute_end_from_milli(
    range: SelectionRange,
    position_milli: u16,
) -> SelectionRange {
    let Some(fade_out) = range.fade_out() else {
        return range;
    };
    let width = range.width();
    if width <= f32::EPSILON {
        return range;
    }
    let fade_out_start = range.end() - (width * fade_out.length);
    let new_end = normalized_from_milli(position_milli).clamp(fade_out_start, 1.0);
    let new_width = (new_end - range.start()).max(0.0);
    let new_length = if new_width <= f32::EPSILON {
        0.0
    } else {
        ((new_end - fade_out_start) / new_width).clamp(0.0, 1.0)
    };
    let next_fade_out = crate::selection::FadeParams::with_curve(new_length, fade_out.curve);
    rebuild_edit_range(
        range,
        range.start(),
        new_end,
        range.fade_in(),
        Some(next_fade_out),
    )
}

/// Update edit fade-in curve from one UI milli curve value.
pub(super) fn update_edit_fade_in_curve_from_milli(
    range: SelectionRange,
    curve_milli: u16,
) -> SelectionRange {
    let Some(fade_in) = range.fade_in() else {
        return range;
    };
    range.with_fade_in(fade_in.length, normalized_from_milli(curve_milli))
}

/// Update edit fade-out curve from one UI milli curve value.
pub(super) fn update_edit_fade_out_curve_from_milli(
    range: SelectionRange,
    curve_milli: u16,
) -> SelectionRange {
    let Some(fade_out) = range.fade_out() else {
        return range;
    };
    range.with_fade_out(fade_out.length, normalized_from_milli(curve_milli))
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
