//! Waveform playback-selection action adapters.

use super::*;
use crate::selection::SelectionEdge;
use crate::selection::SelectionRange;

impl AppController {
    /// Begin a selection drag gesture at the given position.
    pub fn start_selection_drag(&mut self, position: f32) {
        transport::start_selection_drag(self, position);
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

    /// Finish the active selection drag gesture.
    pub fn finish_selection_drag(&mut self) {
        transport::finish_selection_drag(self);
    }

    /// Set the active selection range.
    pub fn set_selection_range(&mut self, range: SelectionRange) {
        transport::set_selection_range(self, range);
    }

    /// True while a selection drag gesture is active.
    pub fn is_selection_dragging(&self) -> bool {
        transport::is_selection_dragging(self)
    }

    /// Clear the active selection.
    pub fn clear_selection(&mut self) {
        transport::clear_selection(self);
    }

    /// Toggle loop playback for the current selection.
    pub fn toggle_loop(&mut self) {
        transport::toggle_loop(self);
    }

    /// Enter or cycle the locked loop override for the current waveform session.
    pub fn toggle_loop_lock(&mut self) {
        transport::toggle_loop_lock(self);
    }

    /// Set waveform selection range using 0..=1000 milli positions from UI actions.
    pub fn set_waveform_selection_range_milli(&mut self, start_milli: u16, end_milli: u16) {
        self.set_waveform_selection_range_micros_with_drag_policy(
            micros_from_milli(start_milli),
            micros_from_milli(end_milli),
            false,
            false,
        );
    }

    /// Set waveform selection range from UI milli positions with drag-specific snap policies.
    pub(crate) fn set_waveform_selection_range_milli_with_drag_policy(
        &mut self,
        start_milli: u16,
        end_milli: u16,
        snap_override: bool,
        preserve_view_edge: bool,
    ) {
        self.set_waveform_selection_range_micros_with_drag_policy(
            micros_from_milli(start_milli),
            micros_from_milli(end_milli),
            snap_override,
            preserve_view_edge,
        );
    }

    /// Set waveform selection range from UI micro positions with drag-specific snap policies.
    pub(crate) fn set_waveform_selection_range_micros_with_drag_policy(
        &mut self,
        start_micros: u32,
        end_micros: u32,
        snap_override: bool,
        preserve_view_edge: bool,
    ) {
        // Fresh create-drags keep any old selection visible until motion begins,
        // so the first update must not be misclassified as a resize/translate of
        // that old range.
        let existing_range = if self.selection_state.range.is_creating() {
            None
        } else {
            current_playback_selection(self)
        };
        let (start_micros, end_micros) = selection_updates::snap_waveform_selection_range_micros(
            self,
            start_micros,
            end_micros,
            existing_range,
            snap_override,
            preserve_view_edge,
        );
        let next_range = selection_range_from_micros(start_micros, end_micros);
        if existing_range == Some(next_range) && waveform_focus_active(self) {
            return;
        }
        self.begin_selection_undo("Selection");
        self.set_selection_range(next_range);
        self.focus_waveform_context();
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
        if !self.preview_waveform_selection_range_micros_smart_scale(existing_range, start_micros) {
            transport::set_selection_range_with_smart_scale(
                self,
                next_range,
                SMART_SCALE_SELECTION_BEATS,
            );
        } else {
            self.update_selection_drag(micros_to_ratio(end_micros), false);
        }
        self.focus_waveform_context();
    }

    /// Clear waveform selection and keep waveform focus active.
    pub fn clear_waveform_selection_with_focus(&mut self) {
        self.clear_selection();
        self.focus_waveform();
    }

    /// Clear both waveform playback and edit selections while keeping waveform focus active.
    pub fn clear_waveform_marks_with_focus(&mut self) {
        self.clear_selection();
        self.clear_edit_selection();
        self.focus_waveform();
    }

    fn preview_waveform_selection_range_micros_smart_scale(
        &mut self,
        existing_range: Option<SelectionRange>,
        anchor_micros: u32,
    ) -> bool {
        if self.selection_state.bpm_scale_beats.is_some() {
            return true;
        }
        let Some(edge) = smart_scale_drag_edge(existing_range, anchor_micros) else {
            return false;
        };
        self.start_selection_edge_drag(edge, true)
    }
}

fn micros_to_ratio(position_micros: u32) -> f32 {
    (position_micros.min(1_000_000) as f32) / 1_000_000.0
}

fn smart_scale_drag_edge(
    range: Option<SelectionRange>,
    anchor_micros: u32,
) -> Option<SelectionEdge> {
    let range = range?;
    let (start_micros, end_micros) = range_micros(range);
    if anchor_micros == start_micros {
        Some(SelectionEdge::End)
    } else if anchor_micros == end_micros {
        Some(SelectionEdge::Start)
    } else {
        None
    }
}

fn range_micros(range: SelectionRange) -> (u32, u32) {
    let start = (range.start().clamp(0.0, 1.0) * 1_000_000.0).round() as u32;
    let end = (range.end().clamp(0.0, 1.0) * 1_000_000.0).round() as u32;
    (start, end)
}
