use super::{
    WaveformSelectionKind, WaveformState,
    interaction::{WaveformDrag, WaveformSelectionDrag},
    zero_crossing_snap::snap_selection_to_zero_crossings,
};

type SelectionRange = wavecrate::selection::SelectionRange;

impl WaveformState {
    pub(in crate::native_app) fn destructive_edit_selection(&self) -> Option<SelectionRange> {
        self.edit_selection
            .filter(|selection| selection.width() > 0.0)
            .or_else(|| {
                self.play_selection
                    .filter(|selection| selection.width() > 0.0)
            })
    }

    pub(super) fn set_selection_for_drag(&mut self, drag: WaveformSelectionDrag, snap: bool) {
        let anchor_ratio = drag.anchor_ratio();
        let range = super::interaction::selection_from_raw_range(drag.range());
        let range = self.snap_selection_if_requested(range, snap);
        let anchor_ratio = snapped_anchor_ratio_for_drag(anchor_ratio, range);
        self.set_selection_for_kind(drag.kind, anchor_ratio, range);
    }

    pub(super) fn update_active_selection_resize(&mut self, ratio: f32, snap: bool) {
        let Some(WaveformDrag::SelectionResize(drag)) = self.active_drag else {
            return;
        };
        if self.selection_for_kind(drag.kind).is_none() {
            return;
        }
        let selection = drag.apply_with_adjusted_bounds(ratio, |selection| {
            self.snap_selection_if_requested(selection, snap)
        });
        self.set_selection_for_kind(drag.kind, selection.start(), selection);
    }

    pub(super) fn update_active_selection_move(&mut self, ratio: f32, snap: bool) {
        let Some(WaveformDrag::SelectionMove(drag)) = self.active_drag else {
            return;
        };
        let selection = self.snap_selection_if_requested(drag.apply(ratio), snap);
        self.set_selection_for_kind(drag.kind, selection.start(), selection);
    }

    pub(super) fn selection_for_kind(&self, kind: WaveformSelectionKind) -> Option<SelectionRange> {
        match kind {
            WaveformSelectionKind::Play => self.play_selection,
            WaveformSelectionKind::Edit => self.edit_selection,
        }
    }

    pub(in crate::native_app) fn set_play_selection_range(&mut self, start: f32, end: f32) {
        let selection = SelectionRange::new(start, end);
        self.set_selection_for_kind(WaveformSelectionKind::Play, selection.start(), selection);
        self.record_current_play_selection_mark();
    }

    pub(in crate::native_app) fn slide_play_selection_by_width(&mut self, direction: i8) -> bool {
        let direction = direction.signum();
        let Some(selection) = self
            .play_selection
            .filter(|selection| selection.width_f64() > f64::EPSILON)
        else {
            return false;
        };
        if direction == 0 {
            return false;
        }
        let width = selection.width_f64().min(1.0);
        let max_start = (1.0 - width).max(0.0);
        let start = (selection.start_f64() + width * f64::from(direction)).clamp(0.0, max_start);
        let next = selection.with_bounds_precise(start, start + width);
        if next == selection {
            return false;
        }
        self.set_selection_for_kind(WaveformSelectionKind::Play, next.start(), next);
        self.record_current_play_selection_mark();
        self.ensure_play_selection_visible();
        true
    }

    pub(in crate::native_app) fn restore_play_selection_state(
        &mut self,
        play_mark_ratio: Option<f32>,
        play_selection: Option<SelectionRange>,
        marked_play_ranges: Vec<SelectionRange>,
    ) {
        if self.play_selection != play_selection {
            self.clear_similar_sections();
        }
        self.active_drag = None;
        self.play_mark_ratio = play_mark_ratio.filter(|ratio| ratio.is_finite());
        self.play_selection = play_selection;
        self.marked_play_ranges = marked_play_ranges;
    }

    pub(in crate::native_app) fn restore_edit_selection_state(
        &mut self,
        edit_selection: Option<SelectionRange>,
    ) {
        self.active_drag = None;
        self.edit_mark_ratio = edit_selection.map(|selection| selection.start());
        self.edit_selection = edit_selection;
    }

    pub(in crate::native_app) fn restore_play_selection_range_in_focus(
        &mut self,
        start: f32,
        end: f32,
    ) {
        self.set_play_selection_range(start, end);
        self.ensure_play_selection_visible();
    }

    pub(in crate::native_app) fn set_edit_selection_range(&mut self, selection: SelectionRange) {
        self.set_selection_for_kind(WaveformSelectionKind::Edit, selection.start(), selection);
    }

    pub(in crate::native_app) fn zero_crossing_snap_enabled(&self) -> bool {
        self.zero_crossing_snap_enabled
    }

    pub(in crate::native_app) fn toggle_zero_crossing_snap(&mut self) -> bool {
        self.zero_crossing_snap_enabled = !self.zero_crossing_snap_enabled;
        self.zero_crossing_snap_enabled
    }

    pub(super) fn record_current_play_selection_mark(&mut self) {
        let Some(selection) = self
            .play_selection
            .filter(|selection| selection.width() > 0.0)
        else {
            return;
        };
        let selection = SelectionRange::new_unclamped(selection.start(), selection.end());
        if self.marked_play_ranges.contains(&selection) {
            return;
        }
        self.marked_play_ranges.push(selection);
    }

    fn set_selection_for_kind(
        &mut self,
        kind: WaveformSelectionKind,
        mark_ratio: f32,
        selection: SelectionRange,
    ) {
        match kind {
            WaveformSelectionKind::Play => {
                if self.play_selection != Some(selection) && self.active_drag.is_none() {
                    self.clear_similar_sections();
                }
                self.play_mark_ratio = Some(mark_ratio);
                self.play_selection = Some(selection);
            }
            WaveformSelectionKind::Edit => {
                self.edit_mark_ratio = Some(mark_ratio);
                self.edit_selection = Some(selection);
            }
        }
    }

    fn snap_selection_if_enabled(&self, selection: SelectionRange) -> SelectionRange {
        if !self.zero_crossing_snap_enabled {
            return selection;
        }
        snap_selection_to_zero_crossings(selection, &self.file)
    }

    fn snap_selection_if_requested(&self, selection: SelectionRange, snap: bool) -> SelectionRange {
        if snap {
            self.snap_selection_if_enabled(selection)
        } else {
            selection
        }
    }
}

fn snapped_anchor_ratio_for_drag(anchor_ratio: f32, selection: SelectionRange) -> f32 {
    let start_distance = (anchor_ratio - selection.start()).abs();
    let end_distance = (anchor_ratio - selection.end()).abs();
    if end_distance < start_distance {
        selection.end()
    } else {
        selection.start()
    }
}
