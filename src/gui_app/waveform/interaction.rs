use radiant::gui::{
    range::{NormalizedRange, NormalizedRangeDrag, NormalizedRangeEdge},
    visualization::{TimelineEditPreview, TimelineEditRamp},
};

use super::{
    SELECTION_DRAG_EPSILON, WaveformActiveDragKind, WaveformEditFadeHandle, WaveformSelectionEdge,
    WaveformSelectionKind, WaveformViewport,
};

mod edit_fade_drag;
pub(super) use edit_fade_drag::WaveformEditFadeDrag;
mod edit_fade_resize;

#[derive(Clone, Copy, Debug)]
pub(super) enum WaveformDrag {
    Selection(WaveformSelectionDrag),
    SelectionResize(WaveformSelectionResizeDrag),
    SelectionMove(WaveformSelectionMoveDrag),
    PlaySelectionExport,
    EditFade(WaveformEditFadeDrag),
    Pan(WaveformPanDrag),
}

impl WaveformDrag {
    pub(super) fn kind(self) -> WaveformActiveDragKind {
        match self {
            WaveformDrag::Selection(drag) => WaveformActiveDragKind::Selection(drag.kind),
            WaveformDrag::SelectionResize(drag) => {
                WaveformActiveDragKind::SelectionResize(drag.kind, drag.edge)
            }
            WaveformDrag::SelectionMove(drag) => WaveformActiveDragKind::SelectionMove(drag.kind),
            WaveformDrag::PlaySelectionExport => WaveformActiveDragKind::PlaySelectionExport,
            WaveformDrag::EditFade(drag) => WaveformActiveDragKind::EditFade(drag.handle),
            WaveformDrag::Pan(_) => WaveformActiveDragKind::Pan,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct WaveformPanDrag {
    pub(super) anchor_visible_ratio: f32,
    pub(super) viewport: WaveformViewport,
}

impl WaveformPanDrag {
    pub(super) fn new(anchor_visible_ratio: f32, viewport: WaveformViewport) -> Self {
        Self {
            anchor_visible_ratio,
            viewport,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct WaveformSelectionDrag {
    pub(super) kind: WaveformSelectionKind,
    drag: NormalizedRangeDrag,
}

impl WaveformSelectionDrag {
    pub(super) fn new(kind: WaveformSelectionKind, ratio: f32) -> Self {
        Self {
            kind,
            drag: NormalizedRangeDrag::new(ratio),
        }
    }

    pub(super) fn update(&mut self, ratio: f32) {
        self.drag.update(ratio, SELECTION_DRAG_EPSILON);
    }

    pub(super) fn moved(self) -> bool {
        self.drag.moved
    }

    pub(super) fn anchor_ratio(self) -> f32 {
        self.drag.anchor_fraction
    }

    pub(super) fn range(self) -> NormalizedRange {
        self.drag.range()
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct WaveformSelectionMoveDrag {
    pub(super) kind: WaveformSelectionKind,
    pub(super) anchor_ratio: f32,
    pub(super) baseline: wavecrate::selection::SelectionRange,
}

impl WaveformSelectionMoveDrag {
    pub(super) fn new(
        kind: WaveformSelectionKind,
        anchor_ratio: f32,
        baseline: wavecrate::selection::SelectionRange,
    ) -> Self {
        Self {
            kind,
            anchor_ratio,
            baseline,
        }
    }

    pub(super) fn apply(self, ratio: f32) -> wavecrate::selection::SelectionRange {
        self.baseline.shift(ratio - self.anchor_ratio)
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct WaveformSelectionResizeDrag {
    pub(super) kind: WaveformSelectionKind,
    pub(super) edge: WaveformSelectionEdge,
    pub(super) fixed_ratio: f32,
}

impl WaveformSelectionResizeDrag {
    pub(super) fn new(
        kind: WaveformSelectionKind,
        edge: WaveformSelectionEdge,
        selection: wavecrate::selection::SelectionRange,
    ) -> Self {
        let fixed_ratio = match edge {
            WaveformSelectionEdge::Start => selection.end(),
            WaveformSelectionEdge::End => selection.start(),
        };
        Self {
            kind,
            edge,
            fixed_ratio,
        }
    }

    pub(super) fn apply(self, ratio: f32) -> wavecrate::selection::SelectionRange {
        selection_from_normalized_range(NormalizedRange::from_edge_fraction(
            normalized_range_edge(self.edge),
            self.fixed_ratio,
            ratio,
        ))
    }
}

fn normalized_range_edge(edge: WaveformSelectionEdge) -> NormalizedRangeEdge {
    match edge {
        WaveformSelectionEdge::Start => NormalizedRangeEdge::Start,
        WaveformSelectionEdge::End => NormalizedRangeEdge::End,
    }
}

pub(super) fn selection_from_normalized_range(
    range: NormalizedRange,
) -> wavecrate::selection::SelectionRange {
    wavecrate::selection::SelectionRange::new(range.start_fraction(), range.end_fraction())
}

pub(super) fn edit_preview_for_selection(
    selection: Option<wavecrate::selection::SelectionRange>,
) -> TimelineEditPreview {
    let Some(selection) = selection else {
        return TimelineEditPreview::default();
    };
    let start = selection.start();
    let end = selection.end();
    let fade_in = selection.fade_in();
    let fade_out = selection.fade_out();
    TimelineEditPreview::from_normalized_ramps(
        NormalizedRange::from_fractions(start, end),
        fade_in.map(|fade| TimelineEditRamp::new(fade.length, fade.mute, Some(fade.curve))),
        fade_out.map(|fade| TimelineEditRamp::new(fade.length, fade.mute, Some(fade.curve))),
    )
}
