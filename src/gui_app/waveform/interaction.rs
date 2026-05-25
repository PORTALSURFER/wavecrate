use radiant::gui::{
    range::NormalizedRange,
    visualization::{TimelineEditPreview, TimelineEditPreviewParts},
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
    pub(super) anchor_ratio: f32,
    pub(super) current_ratio: f32,
    pub(super) moved: bool,
}

impl WaveformSelectionDrag {
    pub(super) fn new(kind: WaveformSelectionKind, ratio: f32) -> Self {
        Self {
            kind,
            anchor_ratio: ratio,
            current_ratio: ratio,
            moved: false,
        }
    }

    pub(super) fn update(&mut self, ratio: f32) {
        self.current_ratio = ratio;
        self.moved |= (self.current_ratio - self.anchor_ratio).abs() > SELECTION_DRAG_EPSILON;
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

    pub(super) fn apply(
        self,
        _selection: wavecrate::selection::SelectionRange,
        ratio: f32,
    ) -> wavecrate::selection::SelectionRange {
        let ratio = ratio.clamp(0.0, 1.0);
        match self.edge {
            WaveformSelectionEdge::Start => {
                wavecrate::selection::SelectionRange::new(ratio, self.fixed_ratio)
            }
            WaveformSelectionEdge::End => {
                wavecrate::selection::SelectionRange::new(self.fixed_ratio, ratio)
            }
        }
    }
}

pub(super) fn edit_preview_for_selection(
    selection: Option<wavecrate::selection::SelectionRange>,
) -> TimelineEditPreview {
    let Some(selection) = selection else {
        return TimelineEditPreview::default();
    };
    let start = selection.start();
    let end = selection.end();
    let width = selection.width();
    let fade_in = selection.fade_in();
    let fade_out = selection.fade_out();
    TimelineEditPreview::from_parts(TimelineEditPreviewParts {
        selection: Some(NormalizedRange::from_micros(
            normalized_to_micros(start),
            normalized_to_micros(end),
        )),
        leading_end_milli: fade_in.map(|fade| normalized_to_milli(start + width * fade.length)),
        leading_end_micros: fade_in.map(|fade| normalized_to_micros(start + width * fade.length)),
        leading_inner_start_milli: fade_in
            .map(|fade| normalized_to_milli(start - width * fade.mute)),
        leading_inner_start_micros: fade_in
            .map(|fade| normalized_to_micros(start - width * fade.mute)),
        leading_curve_milli: fade_in.map(|fade| normalized_to_milli(fade.curve)),
        trailing_start_milli: fade_out.map(|fade| normalized_to_milli(end - width * fade.length)),
        trailing_start_micros: fade_out.map(|fade| normalized_to_micros(end - width * fade.length)),
        trailing_inner_end_milli: fade_out.map(|fade| normalized_to_milli(end + width * fade.mute)),
        trailing_inner_end_micros: fade_out
            .map(|fade| normalized_to_micros(end + width * fade.mute)),
        trailing_curve_milli: fade_out.map(|fade| normalized_to_milli(fade.curve)),
    })
}

fn normalized_to_milli(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

fn normalized_to_micros(value: f32) -> u32 {
    (value.clamp(0.0, 1.0) * 1_000_000.0).round() as u32
}
