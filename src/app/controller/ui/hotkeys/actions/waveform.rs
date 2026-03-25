use super::super::types::{HotkeyAction, HotkeyCommand, HotkeyGesture, HotkeyScope};
use crate::app::state::FocusContext;
use crate::gui::input::KeyCode as Key;

const WAVEFORM: HotkeyScope = HotkeyScope::Focus(FocusContext::Waveform);

pub(super) const NORMALIZE_SELECTION: HotkeyAction = HotkeyAction {
    id: "normalize-waveform",
    label: "Normalize selection/sample",
    gesture: HotkeyGesture::new(Key::N),
    scope: WAVEFORM,
    command: HotkeyCommand::NormalizeWaveform,
};

pub(super) const ALIGN_START_TO_MARKER: HotkeyAction = HotkeyAction {
    id: "align-waveform-start",
    label: "Set start to hover cursor",
    gesture: HotkeyGesture::new(Key::S),
    scope: WAVEFORM,
    command: HotkeyCommand::AlignWaveformStartToMarker,
};

pub(super) const CROP_SELECTION: HotkeyAction = HotkeyAction {
    id: "crop-selection",
    label: "Crop selection",
    gesture: HotkeyGesture::new(Key::C),
    scope: WAVEFORM,
    command: HotkeyCommand::CropSelection,
};

pub(super) const CROP_SELECTION_NEW_SAMPLE: HotkeyAction = HotkeyAction {
    id: "crop-selection-new-sample",
    label: "Crop selection as new sample",
    gesture: HotkeyGesture::with_shift(Key::C),
    scope: WAVEFORM,
    command: HotkeyCommand::CropSelectionNewSample,
};

pub(super) const SAVE_SELECTION_TO_BROWSER: HotkeyAction = HotkeyAction {
    id: "save-selection-to-browser",
    label: "Save selection/slices to browser",
    gesture: HotkeyGesture::new(Key::E),
    scope: WAVEFORM,
    command: HotkeyCommand::SaveSelectionToBrowser,
};

pub(super) const TRIM_SELECTION: HotkeyAction = HotkeyAction {
    id: "trim-selection",
    label: "Trim selection",
    gesture: HotkeyGesture::new(Key::T),
    scope: WAVEFORM,
    command: HotkeyCommand::TrimSelection,
};

pub(super) const TOGGLE_BPM_SNAP: HotkeyAction = HotkeyAction {
    id: "toggle-bpm-snap",
    label: "Toggle BPM snap",
    gesture: HotkeyGesture::new(Key::B),
    scope: WAVEFORM,
    command: HotkeyCommand::ToggleBpmSnap,
};

pub(super) const TOGGLE_TRANSIENT_MARKERS: HotkeyAction = HotkeyAction {
    id: "toggle-transients",
    label: "Show/hide transients",
    gesture: HotkeyGesture::new(Key::I),
    scope: WAVEFORM,
    command: HotkeyCommand::ToggleTransientMarkers,
};

pub(super) const REVERSE_SELECTION: HotkeyAction = HotkeyAction {
    id: "reverse-selection",
    label: "Reverse selection",
    gesture: HotkeyGesture::with_shift(Key::R),
    scope: WAVEFORM,
    command: HotkeyCommand::ReverseSelection,
};

pub(super) const FADE_SELECTION_LEFT_TO_RIGHT: HotkeyAction = HotkeyAction {
    id: "fade-selection-left-to-right",
    label: "Fade selection (left to right)",
    gesture: HotkeyGesture::new(Key::Backslash),
    scope: WAVEFORM,
    command: HotkeyCommand::FadeSelectionLeftToRight,
};

pub(super) const FADE_SELECTION_RIGHT_TO_LEFT: HotkeyAction = HotkeyAction {
    id: "fade-selection-right-to-left",
    label: "Fade selection (right to left)",
    gesture: HotkeyGesture::new(Key::Slash),
    scope: WAVEFORM,
    command: HotkeyCommand::FadeSelectionRightToLeft,
};

pub(super) const DELETE_SLICE_MARKERS: HotkeyAction = HotkeyAction {
    id: "delete-slice-markers",
    label: "Delete slice markers (Slice mode)",
    gesture: HotkeyGesture::with_shift(Key::D),
    scope: WAVEFORM,
    command: HotkeyCommand::DeleteSliceMarkers,
};

pub(super) const DELETE_LOADED_SAMPLE: HotkeyAction = HotkeyAction {
    id: "delete-loaded-sample",
    label: "Delete loaded sample",
    gesture: HotkeyGesture::new(Key::D),
    scope: WAVEFORM,
    command: HotkeyCommand::DeleteLoadedSample,
};

pub(super) const MUTE_SELECTION: HotkeyAction = HotkeyAction {
    id: "mute-selection",
    label: "Mute selection / Merge slices (Slice mode)",
    gesture: HotkeyGesture::new(Key::M),
    scope: WAVEFORM,
    command: HotkeyCommand::MuteSelection,
};

pub(super) const ZOOM_IN_SELECTION: HotkeyAction = HotkeyAction {
    id: "zoom-in-selection",
    label: "Zoom to selection",
    gesture: HotkeyGesture::new(Key::Z),
    scope: WAVEFORM,
    command: HotkeyCommand::ZoomInSelection,
};

pub(super) const ZOOM_OUT_SELECTION: HotkeyAction = HotkeyAction {
    id: "zoom-out-selection",
    label: "Zoom out",
    gesture: HotkeyGesture::new(Key::X),
    scope: WAVEFORM,
    command: HotkeyCommand::ZoomOutSelection,
};

pub(super) const SLIDE_SELECTION_LEFT: HotkeyAction = HotkeyAction {
    id: "slide-selection-left",
    label: "Slide selection left",
    gesture: HotkeyGesture::new(Key::ArrowLeft),
    scope: WAVEFORM,
    command: HotkeyCommand::SlideSelectionLeft,
};

pub(super) const SLIDE_SELECTION_RIGHT: HotkeyAction = HotkeyAction {
    id: "slide-selection-right",
    label: "Slide selection right",
    gesture: HotkeyGesture::new(Key::ArrowRight),
    scope: WAVEFORM,
    command: HotkeyCommand::SlideSelectionRight,
};

pub(super) const NUDGE_SELECTION_LEFT: HotkeyAction = HotkeyAction {
    id: "nudge-selection-left",
    label: "Nudge selection left (fine)",
    gesture: HotkeyGesture::with_shift(Key::ArrowLeft),
    scope: WAVEFORM,
    command: HotkeyCommand::NudgeSelectionLeft,
};

pub(super) const NUDGE_SELECTION_RIGHT: HotkeyAction = HotkeyAction {
    id: "nudge-selection-right",
    label: "Nudge selection right (fine)",
    gesture: HotkeyGesture::with_shift(Key::ArrowRight),
    scope: WAVEFORM,
    command: HotkeyCommand::NudgeSelectionRight,
};
