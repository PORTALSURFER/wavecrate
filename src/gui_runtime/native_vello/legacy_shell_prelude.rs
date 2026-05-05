pub(in crate::gui_runtime::native_vello) use crate::compat_app_contract::{
    AppModel, DirtySegments, NativeAppBridge, NativeMotionModel, SegmentRevisions, UiAction,
};
pub(in crate::gui_runtime::native_vello) use crate::gui::{
    frame::FrameBuildResult,
    input::KeyCode,
    input::KeyPress,
    native_shell::{
        ChromeMotionOverlayFingerprint, CursorMoveEffect, FocusOverlayFingerprint,
        HoverOverlayFingerprint, ModalOverlayFingerprint, NativeShellState, ShellLayout,
        ShellLayoutDirtyKind, ShellLayoutRuntime, ShellLayoutTreeKind, ShellNodeKind,
        StaticFrameSegment, StaticFrameSegments, StyleTokens, TextFieldVisualState,
        WaveformMotionOverlayFingerprint, compute_waveform_slice_preview_rects,
        dirty_segments_for_layout_subtree, waveform_view_window_from_bounds,
    },
    paint::{PaintFrame as NativeViewFrame, Primitive},
};
pub(in crate::gui_runtime::native_vello) use crate::gui_runtime::NativeRunReport;
pub(in crate::gui_runtime::native_vello) use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
pub(in crate::gui_runtime::native_vello) use vello::{
    kurbo::{Circle, Point as KurboPoint},
    peniko::Gradient,
};
pub(in crate::gui_runtime::native_vello) use winit::{
    event::MouseScrollDelta, keyboard::ModifiersState, window::CursorIcon,
};
