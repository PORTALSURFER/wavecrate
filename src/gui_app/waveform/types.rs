use radiant::gui::types::Vector2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_app) enum WaveformInteraction {
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
    Frame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_app) enum WaveformSelectionKind {
    Play,
    Edit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_app) enum WaveformSelectionEdge {
    Start,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_app) enum WaveformEditFadeHandle {
    FadeInEnd,
    FadeInStart,
    FadeInOuterStart,
    FadeOutStart,
    FadeOutEnd,
    FadeOutOuterEnd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_app) enum WaveformActiveDragKind {
    Selection(WaveformSelectionKind),
    SelectionResize(WaveformSelectionKind, WaveformSelectionEdge),
    SelectionMove(WaveformSelectionKind),
    EditFade(WaveformEditFadeHandle),
    Pan,
}
