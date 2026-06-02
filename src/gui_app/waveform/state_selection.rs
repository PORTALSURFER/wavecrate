use super::{
    WaveformSelectionKind, WaveformState,
    interaction::{WaveformDrag, WaveformSelectionDrag},
};

type SelectionRange = wavecrate::selection::SelectionRange;

impl WaveformState {
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

    fn set_selection_for_kind(
        &mut self,
        kind: WaveformSelectionKind,
        mark_ratio: f32,
        selection: SelectionRange,
    ) {
        match kind {
            WaveformSelectionKind::Play => {
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
