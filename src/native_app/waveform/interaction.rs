use radiant::gui::{
    range::NormalizedRange,
    visualization::{TimelineEditPreview, TimelineEditRamp},
};

use super::{
    SELECTION_DRAG_EPSILON, WaveformActiveDragKind, WaveformEditFadeHandle,
    WaveformEditFadeOuterGainHandle, WaveformSelectionEdge, WaveformSelectionKind,
    WaveformViewport,
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
    EditFadeOuterGain(WaveformEditFadeOuterGainDrag),
    EditGain(WaveformEditGainDrag),
    SampleSlide(WaveformSampleSlideDrag),
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
            WaveformDrag::EditFadeOuterGain(drag) => {
                WaveformActiveDragKind::EditFadeOuterGain(drag.handle)
            }
            WaveformDrag::EditGain(_) => WaveformActiveDragKind::EditGain,
            WaveformDrag::SampleSlide(_) => WaveformActiveDragKind::SampleSlide,
            WaveformDrag::Pan(_) => WaveformActiveDragKind::Pan,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct WaveformSampleSlideDrag {
    anchor_visible_ratio: f32,
    visible_frames: usize,
}

impl WaveformSampleSlideDrag {
    pub(super) fn new(anchor_visible_ratio: f32, viewport: WaveformViewport) -> Self {
        Self {
            anchor_visible_ratio: finite_or_zero(anchor_visible_ratio),
            visible_frames: viewport.visible_items(),
        }
    }

    pub(super) fn frame_offset(self, visible_ratio: f32) -> i64 {
        let visible_ratio = finite_or_zero(visible_ratio);
        ((visible_ratio - self.anchor_visible_ratio) * self.visible_frames as f32).round() as i64
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct WaveformPanDrag {
    pub(super) anchor_visible_ratio: f32,
    pub(super) viewport: WaveformViewport,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct WaveformEditGainDrag {
    anchor_y: f32,
    baseline: wavecrate::selection::SelectionRange,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct WaveformEditFadeOuterGainDrag {
    pub(super) handle: WaveformEditFadeOuterGainHandle,
}

impl WaveformEditFadeOuterGainDrag {
    pub(super) fn new(handle: WaveformEditFadeOuterGainHandle) -> Self {
        Self { handle }
    }

    pub(super) fn apply(
        self,
        selection: wavecrate::selection::SelectionRange,
        vertical_ratio: f32,
    ) -> wavecrate::selection::SelectionRange {
        let gain = outer_gain_for_vertical_ratio(vertical_ratio);
        match self.handle {
            WaveformEditFadeOuterGainHandle::In => selection.with_fade_in_outer_gain(gain),
            WaveformEditFadeOuterGainHandle::Out => selection.with_fade_out_outer_gain(gain),
        }
    }
}

fn outer_gain_for_vertical_ratio(vertical_ratio: f32) -> f32 {
    if !vertical_ratio.is_finite() {
        return 1.0;
    }
    (1.0 - (vertical_ratio / 0.5)).clamp(0.0, 1.0)
}

impl WaveformEditGainDrag {
    const BOOST_PIXELS: f32 = 120.0;
    const ATTENUATE_PIXELS: f32 = 120.0;

    pub(super) fn new(anchor_y: f32, baseline: wavecrate::selection::SelectionRange) -> Self {
        Self { anchor_y, baseline }
    }

    pub(super) fn apply(self, pointer_y: f32) -> wavecrate::selection::SelectionRange {
        self.baseline.with_gain(self.gain_for_pointer_y(pointer_y))
    }

    fn gain_for_pointer_y(self, pointer_y: f32) -> f32 {
        if !pointer_y.is_finite() || !self.anchor_y.is_finite() {
            return self.baseline.gain();
        }
        let delta_y = pointer_y - self.anchor_y;
        if delta_y < 0.0 {
            self.baseline.gain() + (-delta_y / Self::BOOST_PIXELS) * 3.0
        } else {
            self.baseline.gain() - (delta_y / Self::ATTENUATE_PIXELS)
        }
    }
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
    anchor_ratio: f32,
    current_ratio: f32,
    moved: bool,
}

impl WaveformSelectionDrag {
    pub(super) fn new(kind: WaveformSelectionKind, ratio: f32) -> Self {
        let ratio = finite_or_zero(ratio);
        Self {
            kind,
            anchor_ratio: ratio,
            current_ratio: ratio,
            moved: false,
        }
    }

    pub(super) fn update(&mut self, ratio: f32) {
        self.current_ratio = finite_or_zero(ratio);
        self.moved |= (self.current_ratio - self.anchor_ratio).abs() > SELECTION_DRAG_EPSILON;
    }

    pub(super) fn moved(self) -> bool {
        self.moved
    }

    pub(super) fn anchor_ratio(self) -> f32 {
        self.anchor_ratio
    }

    pub(super) fn range(self) -> WaveformRawSelectionRange {
        WaveformRawSelectionRange::new(self.anchor_ratio, self.current_ratio)
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct WaveformRawSelectionRange {
    start: f32,
    end: f32,
}

impl WaveformRawSelectionRange {
    fn new(start: f32, end: f32) -> Self {
        if start <= end {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct WaveformSelectionMoveDrag {
    pub(super) kind: WaveformSelectionKind,
    pub(super) anchor_ratio: f32,
    pub(super) baseline: wavecrate::selection::SelectionRange,
    allow_out_of_bounds: bool,
}

impl WaveformSelectionMoveDrag {
    pub(super) fn new(
        kind: WaveformSelectionKind,
        anchor_ratio: f32,
        baseline: wavecrate::selection::SelectionRange,
        allow_out_of_bounds: bool,
    ) -> Self {
        Self {
            kind,
            anchor_ratio,
            baseline,
            allow_out_of_bounds,
        }
    }

    pub(super) fn apply(self, ratio: f32) -> wavecrate::selection::SelectionRange {
        if self.allow_out_of_bounds {
            self.baseline.shift_unclamped(ratio - self.anchor_ratio)
        } else {
            self.baseline.shift(ratio - self.anchor_ratio)
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct WaveformSelectionResizeDrag {
    pub(super) kind: WaveformSelectionKind,
    pub(super) edge: WaveformSelectionEdge,
    pub(super) fixed_ratio: f32,
    baseline: wavecrate::selection::SelectionRange,
    allow_out_of_bounds: bool,
}

impl WaveformSelectionResizeDrag {
    pub(super) fn new(
        kind: WaveformSelectionKind,
        edge: WaveformSelectionEdge,
        selection: wavecrate::selection::SelectionRange,
        allow_out_of_bounds: bool,
    ) -> Self {
        let fixed_ratio = match edge {
            WaveformSelectionEdge::Start => selection.end(),
            WaveformSelectionEdge::End => selection.start(),
        };
        Self {
            kind,
            edge,
            fixed_ratio,
            baseline: selection,
            allow_out_of_bounds,
        }
    }

    pub(super) fn apply_with_adjusted_bounds(
        self,
        ratio: f32,
        adjust: impl FnOnce(
            wavecrate::selection::SelectionRange,
        ) -> wavecrate::selection::SelectionRange,
    ) -> wavecrate::selection::SelectionRange {
        let resized = if self.allow_out_of_bounds {
            wavecrate::selection::SelectionRange::new_unclamped(self.fixed_ratio, ratio)
        } else {
            wavecrate::selection::SelectionRange::new(self.fixed_ratio, ratio)
        };
        let resized = adjust(resized);
        match self.kind {
            WaveformSelectionKind::Play => resized,
            WaveformSelectionKind::Edit => preserve_edit_selection_effects(self.baseline, resized),
        }
    }
}

fn preserve_edit_selection_effects(
    source: wavecrate::selection::SelectionRange,
    resized: wavecrate::selection::SelectionRange,
) -> wavecrate::selection::SelectionRange {
    let source_width = source.width();
    let mut selection = resized.with_gain(source.gain());
    if let Some(fade_in) = source.fade_in() {
        selection = selection
            .with_fade_in_and_mute(
                preserved_relative_length(source_width * fade_in.length, selection.width()),
                fade_in.curve,
                preserved_fade_in_mute(source, selection),
            )
            .with_fade_in_outer_gain(fade_in.outer_gain);
    }
    if let Some(fade_out) = source.fade_out() {
        selection = selection
            .with_fade_out_and_mute(
                preserved_relative_length(source_width * fade_out.length, selection.width()),
                fade_out.curve,
                preserved_fade_out_mute(source, selection),
            )
            .with_fade_out_outer_gain(fade_out.outer_gain);
    }
    selection
}

fn preserved_relative_length(absolute_length: f32, selection_width: f32) -> f32 {
    if selection_width <= 0.0 || !selection_width.is_finite() {
        return 0.0;
    }
    (absolute_length / selection_width).clamp(0.0, 1.0)
}

fn preserved_fade_in_mute(
    source: wavecrate::selection::SelectionRange,
    resized: wavecrate::selection::SelectionRange,
) -> f32 {
    let Some(fade_in) = source.fade_in().filter(|fade| fade.mute > 0.0) else {
        return 0.0;
    };
    if resized.width() <= 0.0 {
        return 0.0;
    }
    let source_outer_start = source.start() - source.width() * fade_in.mute;
    ((resized.start() - source_outer_start) / resized.width()).max(0.0)
}

fn preserved_fade_out_mute(
    source: wavecrate::selection::SelectionRange,
    resized: wavecrate::selection::SelectionRange,
) -> f32 {
    let Some(fade_out) = source.fade_out().filter(|fade| fade.mute > 0.0) else {
        return 0.0;
    };
    if resized.width() <= 0.0 {
        return 0.0;
    }
    let source_outer_end = source.end() + source.width() * fade_out.mute;
    ((source_outer_end - resized.end()) / resized.width()).max(0.0)
}

pub(super) fn selection_from_raw_range(
    range: WaveformRawSelectionRange,
) -> wavecrate::selection::SelectionRange {
    wavecrate::selection::SelectionRange::new_unclamped(range.start, range.end)
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

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}
