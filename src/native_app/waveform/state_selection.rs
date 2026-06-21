use super::{
    WaveformSelectionKind, WaveformState,
    interaction::{WaveformDrag, WaveformSelectionDrag},
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

    pub(super) fn set_selection_for_drag(&mut self, drag: WaveformSelectionDrag) {
        let anchor_ratio = drag.anchor_ratio();
        let range = super::interaction::selection_from_normalized_range(drag.range());
        self.set_selection_for_kind(drag.kind, anchor_ratio, range);
    }

    pub(super) fn update_active_selection_resize(&mut self, ratio: f32) {
        let Some(WaveformDrag::SelectionResize(drag)) = self.active_drag else {
            return;
        };
        if self.selection_for_kind(drag.kind).is_none() {
            return;
        }
        let selection = drag.apply(ratio);
        self.set_selection_for_kind(drag.kind, selection.start(), selection);
    }

    pub(super) fn update_active_selection_move(&mut self, ratio: f32) {
        let Some(WaveformDrag::SelectionMove(drag)) = self.active_drag else {
            return;
        };
        let selection = drag.apply(ratio);
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

    pub(in crate::native_app) fn set_edit_selection_range(&mut self, selection: SelectionRange) {
        self.set_selection_for_kind(WaveformSelectionKind::Edit, selection.start(), selection);
    }

    pub(super) fn record_current_play_selection_mark(&mut self) {
        let Some(selection) = self
            .play_selection
            .filter(|selection| selection.width() > 0.0)
        else {
            return;
        };
        let selection = SelectionRange::new(selection.start(), selection.end());
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
                if self.play_selection != Some(selection) {
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
}
