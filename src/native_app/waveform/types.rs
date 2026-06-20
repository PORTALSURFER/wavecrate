use radiant::gui::types::Vector2;
use radiant::widgets::DragHandleMessage;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) enum WaveformInteraction {
    Wheel {
        delta: Vector2,
        anchor_ratio: f32,
    },
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
    BeginSelectionResize {
        kind: WaveformSelectionKind,
        edge: WaveformSelectionEdge,
        visible_ratio: f32,
    },
    BeginSelectionMove {
        kind: WaveformSelectionKind,
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
    Frame,
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
    Pan,
}
