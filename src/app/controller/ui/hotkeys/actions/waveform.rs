use super::super::types::{HotkeyAction, HotkeyGesture, HotkeyScope};
use crate::app::state::FocusContext;
use crate::app_core::actions::NativeUiAction;
use crate::gui::input::KeyCode as Key;

const WAVEFORM: HotkeyScope = HotkeyScope::Focus(FocusContext::Waveform);

pub(super) const NORMALIZE_SELECTION: HotkeyAction = HotkeyAction {
    id: "normalize-waveform",
    label: "Normalize selection/sample",
    gesture: HotkeyGesture::new(Key::N),
    scope: WAVEFORM,
    action: NativeUiAction::NormalizeWaveformSelectionOrSample,
};
pub(super) const ALIGN_START_TO_MARKER: HotkeyAction = HotkeyAction {
    id: "align-waveform-start",
    label: "Set start to hover cursor",
    gesture: HotkeyGesture::new(Key::S),
    scope: WAVEFORM,
    action: NativeUiAction::AlignWaveformStartToMarker,
};
pub(super) const CROP_SELECTION: HotkeyAction = HotkeyAction {
    id: "crop-selection",
    label: "Crop selection",
    gesture: HotkeyGesture::new(Key::C),
    scope: WAVEFORM,
    action: NativeUiAction::CropWaveformSelection,
};
pub(super) const COPY_WAVEFORM_SELECTION: HotkeyAction = HotkeyAction {
    id: "copy-waveform-selection",
    label: "Copy selection clip",
    gesture: HotkeyGesture::with_command(Key::C),
    scope: WAVEFORM,
    action: NativeUiAction::CopySelectionToClipboard,
};
pub(super) const CROP_SELECTION_NEW_SAMPLE: HotkeyAction = HotkeyAction {
    id: "crop-selection-new-sample",
    label: "Crop selection as new sample",
    gesture: HotkeyGesture::with_shift(Key::C),
    scope: WAVEFORM,
    action: NativeUiAction::CropWaveformSelectionToNewSample,
};
pub(super) const SAVE_SELECTION_TO_BROWSER: HotkeyAction = HotkeyAction {
    id: "save-selection-to-browser",
    label: "Save selection/slices to browser",
    gesture: HotkeyGesture::new(Key::E),
    scope: WAVEFORM,
    action: NativeUiAction::SaveWaveformSelectionToBrowser,
};
pub(super) const SAVE_SELECTION_TO_BROWSER_KEEP2: HotkeyAction = HotkeyAction {
    id: "save-selection-to-browser-keep2",
    label: "Save selection/slices to browser (keep x2)",
    gesture: HotkeyGesture::with_shift(Key::E),
    scope: WAVEFORM,
    action: NativeUiAction::SaveWaveformSelectionToBrowserWithKeep2,
};
pub(super) const COMMIT_WAVEFORM_EDIT_FADES: HotkeyAction = HotkeyAction {
    id: "commit-waveform-edit-fades",
    label: "Apply edit fades",
    gesture: HotkeyGesture::new(Key::Enter),
    scope: WAVEFORM,
    action: NativeUiAction::CommitWaveformEditFades,
};
pub(super) const TOGGLE_FOCUSED_SLICE_EXPORT_MARK: HotkeyAction = HotkeyAction {
    id: "toggle-focused-slice-export-mark",
    label: "Mark focused slice for export",
    gesture: HotkeyGesture::new(Key::A),
    scope: WAVEFORM,
    action: NativeUiAction::ToggleFocusedWaveformSliceExportMark,
};
pub(super) const TRIM_SELECTION: HotkeyAction = HotkeyAction {
    id: "trim-selection",
    label: "Trim selection",
    gesture: HotkeyGesture::new(Key::T),
    scope: WAVEFORM,
    action: NativeUiAction::TrimWaveformSelection,
};
pub(super) const TOGGLE_BPM_SNAP: HotkeyAction = HotkeyAction {
    id: "toggle-bpm-snap",
    label: "Toggle BPM snap",
    gesture: HotkeyGesture::new(Key::B),
    scope: WAVEFORM,
    action: NativeUiAction::ToggleBpmSnap,
};
pub(super) const TOGGLE_TRANSIENT_MARKERS: HotkeyAction = HotkeyAction {
    id: "toggle-transients",
    label: "Show/hide transients",
    gesture: HotkeyGesture::new(Key::I),
    scope: WAVEFORM,
    action: NativeUiAction::ToggleTransientMarkers,
};
pub(super) const REVERSE_SELECTION: HotkeyAction = HotkeyAction {
    id: "reverse-selection",
    label: "Reverse selection",
    gesture: HotkeyGesture::with_shift(Key::R),
    scope: WAVEFORM,
    action: NativeUiAction::ReverseWaveformSelection,
};
pub(super) const FADE_SELECTION_LEFT_TO_RIGHT: HotkeyAction = HotkeyAction {
    id: "fade-selection-left-to-right",
    label: "Fade selection (left to right)",
    gesture: HotkeyGesture::new(Key::Backslash),
    scope: WAVEFORM,
    action: NativeUiAction::FadeWaveformSelectionLeftToRight,
};
pub(super) const FADE_SELECTION_RIGHT_TO_LEFT: HotkeyAction = HotkeyAction {
    id: "fade-selection-right-to-left",
    label: "Fade selection (right to left)",
    gesture: HotkeyGesture::new(Key::Slash),
    scope: WAVEFORM,
    action: NativeUiAction::FadeWaveformSelectionRightToLeft,
};
pub(super) const DELETE_SLICE_MARKERS: HotkeyAction = HotkeyAction {
    id: "delete-slice-markers",
    label: "Delete slice markers (Slice mode)",
    gesture: HotkeyGesture::with_shift(Key::D),
    scope: WAVEFORM,
    action: NativeUiAction::DeleteSelectedSliceMarkers,
};
pub(super) const DELETE_LOADED_SAMPLE: HotkeyAction = HotkeyAction {
    id: "delete-loaded-sample",
    label: "Delete loaded sample",
    gesture: HotkeyGesture::new(Key::D),
    scope: WAVEFORM,
    action: NativeUiAction::DeleteLoadedWaveformSample,
};
pub(super) const MUTE_SELECTION: HotkeyAction = HotkeyAction {
    id: "mute-selection",
    label: "Mute selection / Merge slices (Slice mode)",
    gesture: HotkeyGesture::new(Key::M),
    scope: WAVEFORM,
    action: NativeUiAction::MuteWaveformSelection,
};
pub(super) const ZOOM_IN_SELECTION: HotkeyAction = HotkeyAction {
    id: "zoom-in-selection",
    label: "Zoom to selection",
    gesture: HotkeyGesture::new(Key::Z),
    scope: WAVEFORM,
    action: NativeUiAction::ZoomWaveformToSelection,
};
pub(super) const ZOOM_OUT_SELECTION: HotkeyAction = HotkeyAction {
    id: "zoom-out-selection",
    label: "Zoom out",
    gesture: HotkeyGesture::new(Key::X),
    scope: WAVEFORM,
    action: NativeUiAction::ZoomWaveformFull,
};
pub(super) const SLIDE_SELECTION_LEFT: HotkeyAction = HotkeyAction {
    id: "slide-selection-left",
    label: "Previous slice / Slide selection left",
    gesture: HotkeyGesture::new(Key::ArrowLeft),
    scope: WAVEFORM,
    action: NativeUiAction::MoveWaveformSliceFocus { delta: -1 },
};
pub(super) const SLIDE_SELECTION_RIGHT: HotkeyAction = HotkeyAction {
    id: "slide-selection-right",
    label: "Next slice / Slide selection right",
    gesture: HotkeyGesture::new(Key::ArrowRight),
    scope: WAVEFORM,
    action: NativeUiAction::MoveWaveformSliceFocus { delta: 1 },
};
pub(super) const MICRO_SLIDE_SELECTION_LEFT: HotkeyAction = HotkeyAction {
    id: "micro-slide-selection-left",
    label: "Micro-slide selection left (1 sample)",
    gesture: HotkeyGesture::with_alt(Key::ArrowLeft),
    scope: WAVEFORM,
    action: NativeUiAction::SlideWaveformSelection {
        delta: -1,
        fine: true,
    },
};
pub(super) const MICRO_SLIDE_SELECTION_RIGHT: HotkeyAction = HotkeyAction {
    id: "micro-slide-selection-right",
    label: "Micro-slide selection right (1 sample)",
    gesture: HotkeyGesture::with_alt(Key::ArrowRight),
    scope: WAVEFORM,
    action: NativeUiAction::SlideWaveformSelection {
        delta: 1,
        fine: true,
    },
};
pub(super) const NUDGE_SELECTION_LEFT: HotkeyAction = HotkeyAction {
    id: "nudge-selection-left",
    label: "Nudge selection left (fine)",
    gesture: HotkeyGesture::with_shift(Key::ArrowLeft),
    scope: WAVEFORM,
    action: NativeUiAction::SlideWaveformSelection {
        delta: -1,
        fine: true,
    },
};
pub(super) const NUDGE_SELECTION_RIGHT: HotkeyAction = HotkeyAction {
    id: "nudge-selection-right",
    label: "Nudge selection right (fine)",
    gesture: HotkeyGesture::with_shift(Key::ArrowRight),
    scope: WAVEFORM,
    action: NativeUiAction::SlideWaveformSelection {
        delta: 1,
        fine: true,
    },
};
