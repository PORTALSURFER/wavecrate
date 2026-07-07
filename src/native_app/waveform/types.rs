use radiant::gui::types::{Point, Vector2};
use radiant::widgets::DragHandleMessage;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) enum WaveformInteraction {
    Wheel {
        delta: Vector2,
        anchor_ratio: f32,
        expand_silence_margin: bool,
    },
    ZoomToPlaySelection,
    SlidePlaySelection {
        direction: i8,
    },
    ZoomFull,
    ScrollTo {
        offset_fraction: f32,
    },
    BeginSelection {
        kind: WaveformSelectionKind,
        visible_ratio: f32,
    },
    BeginEditFade {
        handle: WaveformEditFadeHandle,
        visible_ratio: f32,
    },
    BeginEditFadeOuterGain {
        handle: WaveformEditFadeOuterGainHandle,
        vertical_ratio: f32,
    },
    UpdateEditFadeOuterGain {
        vertical_ratio: f32,
    },
    FinishEditFadeOuterGain {
        vertical_ratio: f32,
    },
    BeginEditGain {
        pointer_y: f32,
    },
    UpdateEditGain {
        pointer_y: f32,
    },
    FinishEditGain {
        pointer_y: f32,
    },
    ClearEditFadeSilence {
        handle: WaveformEditFadeHandle,
    },
    SelectSimilarSection {
        selection: wavecrate::selection::SelectionRange,
    },
    BeginSelectionResize {
        kind: WaveformSelectionKind,
        edge: WaveformSelectionEdge,
        visible_ratio: f32,
    },
    BeginSelectionMove {
        kind: WaveformSelectionKind,
        visible_ratio: f32,
    },
    BeginSampleSlide {
        visible_ratio: f32,
    },
    UpdateSampleSlide {
        visible_ratio: f32,
    },
    FinishSampleSlide {
        visible_ratio: f32,
    },
    BeginPan {
        visible_ratio: f32,
    },
    UpdateSelection {
        visible_ratio: f32,
    },
    FinishSelection {
        visible_ratio: f32,
    },
    DragPlaySelectionExport(DragHandleMessage),
    DragLoadedSample(DragHandleMessage),
    Frame,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct WaveformContextMenu {
    pub(in crate::native_app) anchor: Point,
    pub(in crate::native_app) title: String,
    pub(in crate::native_app) extract_to_harvest_destination: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum WaveformSelectionKind {
    Play,
    Edit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum WaveformSelectionEdge {
    Start,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum WaveformEditFadeHandle {
    InEnd,
    InStart,
    InOuterStart,
    OutStart,
    OutEnd,
    OutOuterEnd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum WaveformEditFadeOuterGainHandle {
    In,
    Out,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum WaveformActiveDragKind {
    Selection(WaveformSelectionKind),
    SelectionResize(WaveformSelectionKind, WaveformSelectionEdge),
    SelectionMove(WaveformSelectionKind),
    PlaySelectionExport,
    EditFade(WaveformEditFadeHandle),
    EditFadeOuterGain(WaveformEditFadeOuterGainHandle),
    EditGain,
    SampleSlide,
    Pan,
}
