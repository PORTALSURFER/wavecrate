//! Shared waveform drag-mode types and hit-test constants.

/// Drag-mode state carried across waveform pointer interactions.
#[cfg_attr(not(test), derive(PartialEq, Eq))]
#[derive(Clone, Copy, Debug)]
pub(in crate::gui_runtime::native_vello) enum WaveformPointerDragMode {
    /// Drag updates seek/playhead position.
    Seek,
    /// Drag updates cursor position.
    Cursor,
    /// Drag updates circular waveform-slide preview and commit state.
    CircularSlide {
        /// Fixed anchor micro position captured at drag start.
        anchor_micros: u32,
    },
    /// Drag extends playback selection from a fixed anchor micro value.
    Selection {
        /// Fixed anchor nanounit position captured at drag start.
        anchor_micros: u32,
        /// Optional stable clamp captured while the pointer remains off-plot.
        boundary_lock: Option<WaveformSelectionBoundaryLock>,
    },
    /// Drag resizes a playback selection without snapping and recomputes BPM from a 4-beat span.
    SelectionSmartScale {
        /// Fixed anchor nanounit position captured at drag start.
        anchor_micros: u32,
        /// Optional stable clamp captured while the pointer remains off-plot.
        boundary_lock: Option<WaveformSelectionBoundaryLock>,
    },
    /// Drag shifts the playback selection while preserving its width.
    SelectionShift {
        /// Pointer nanounit position captured at drag start.
        pointer_micros: u32,
        /// Original playback-selection start nanounit position.
        start_micros: u32,
        /// Original playback-selection end nanounit position.
        end_micros: u32,
    },
    /// Drag extends edit selection from a fixed anchor micro value.
    EditSelection {
        /// Fixed anchor nanounit position captured at drag start.
        anchor_micros: u32,
        /// Optional stable clamp captured while the pointer remains off-plot.
        boundary_lock: Option<WaveformSelectionBoundaryLock>,
    },
    /// Drag shifts the edit selection while preserving its width.
    EditSelectionShift {
        /// Pointer nanounit position captured at drag start.
        pointer_micros: u32,
        /// Original edit-selection start nanounit position.
        start_micros: u32,
        /// Original edit-selection end nanounit position.
        end_micros: u32,
    },
    /// Drag updates the edit fade-in end handle.
    EditFadeInEnd,
    /// Drag updates the edit fade-in mute-start handle.
    EditFadeInMuteStart,
    /// Drag updates the edit fade-in curve.
    EditFadeInCurve,
    /// Drag updates the edit fade-out start handle.
    EditFadeOutStart,
    /// Drag updates the edit fade-out mute-end handle.
    EditFadeOutMuteEnd,
    /// Drag updates the edit fade-out curve.
    EditFadeOutCurve,
}

#[cfg(test)]
impl PartialEq for WaveformPointerDragMode {
    fn eq(&self, other: &Self) -> bool {
        use WaveformPointerDragMode::*;

        match (*self, *other) {
            (Seek, Seek)
            | (Cursor, Cursor)
            | (EditFadeInEnd, EditFadeInEnd)
            | (EditFadeInMuteStart, EditFadeInMuteStart)
            | (EditFadeInCurve, EditFadeInCurve)
            | (EditFadeOutStart, EditFadeOutStart)
            | (EditFadeOutMuteEnd, EditFadeOutMuteEnd)
            | (EditFadeOutCurve, EditFadeOutCurve) => true,
            (
                CircularSlide {
                    anchor_micros: left,
                },
                CircularSlide {
                    anchor_micros: right,
                },
            ) => left == right,
            (
                Selection {
                    anchor_micros: left,
                    boundary_lock: left_lock,
                },
                Selection {
                    anchor_micros: right,
                    boundary_lock: right_lock,
                },
            )
            | (
                SelectionSmartScale {
                    anchor_micros: left,
                    boundary_lock: left_lock,
                },
                SelectionSmartScale {
                    anchor_micros: right,
                    boundary_lock: right_lock,
                },
            )
            | (
                EditSelection {
                    anchor_micros: left,
                    boundary_lock: left_lock,
                },
                EditSelection {
                    anchor_micros: right,
                    boundary_lock: right_lock,
                },
            ) => waveform_drag_position_equivalent(left, right) && left_lock == right_lock,
            (
                SelectionShift {
                    pointer_micros: left_pointer,
                    start_micros: left_start,
                    end_micros: left_end,
                },
                SelectionShift {
                    pointer_micros: right_pointer,
                    start_micros: right_start,
                    end_micros: right_end,
                },
            )
            | (
                EditSelectionShift {
                    pointer_micros: left_pointer,
                    start_micros: left_start,
                    end_micros: left_end,
                },
                EditSelectionShift {
                    pointer_micros: right_pointer,
                    start_micros: right_start,
                    end_micros: right_end,
                },
            ) => {
                waveform_drag_position_equivalent(left_pointer, right_pointer)
                    && waveform_drag_position_equivalent(left_start, right_start)
                    && waveform_drag_position_equivalent(left_end, right_end)
            }
            _ => false,
        }
    }
}

#[cfg(test)]
impl Eq for WaveformPointerDragMode {}

#[cfg(test)]
fn waveform_drag_position_equivalent(left: u32, right: u32) -> bool {
    left == right || left.checked_mul(1000) == Some(right) || right.checked_mul(1000) == Some(left)
}

/// Horizontal waveform plot edge used by out-of-bounds drag locks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) enum WaveformOutsidePlotSide {
    /// Pointer sits left of the waveform plot.
    Left,
    /// Pointer sits right of the waveform plot.
    Right,
}

/// Stable absolute clamp for anchor-based selection drags outside the waveform plot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct WaveformSelectionBoundaryLock {
    /// Horizontal plot edge the pointer is currently beyond.
    pub(in crate::gui_runtime::native_vello) side: WaveformOutsidePlotSide,
    /// Absolute waveform nanounit position captured for the drag.
    pub(in crate::gui_runtime::native_vello) position_nanos: u32,
}

/// Half-width in pixels used for fade-handle hit testing.
pub(in crate::gui_runtime::native_vello) const WAVEFORM_EDIT_FADE_HANDLE_HIT_HALF_WIDTH: f32 = 7.0;
pub(in crate::gui_runtime::native_vello) const WAVEFORM_EDIT_FADE_TOP_TAB_SIZE: f32 = 10.0;
/// Horizontal drag distance required before a new playback selection counts as intentional.
pub(in crate::gui_runtime::native_vello) const WAVEFORM_SELECTION_CLICK_SLOP_PX: f32 = 3.0;
/// Half-width in pixels used for waveform edge-resize hit testing.
pub(in crate::gui_runtime::native_vello) const WAVEFORM_RESIZE_EDGE_HIT_HALF_WIDTH: f32 = 7.0;
/// Fraction of waveform height used by centered resize-edge hit regions.
pub(in crate::gui_runtime::native_vello) const WAVEFORM_RESIZE_EDGE_HEIGHT_RATIO: f32 = 0.34;
/// Width/height in logical pixels for the playback-selection drag handle.
pub(in crate::gui_runtime::native_vello) const WAVEFORM_SELECTION_DRAG_HANDLE_SIZE: f32 = 12.0;
/// Extra hit slop around the playback-selection drag handle.
pub(in crate::gui_runtime::native_vello) const WAVEFORM_SELECTION_DRAG_HANDLE_HIT_INSET: f32 = 4.0;
/// Width in logical pixels for bottom-center selection shift handles.
pub(in crate::gui_runtime::native_vello) const WAVEFORM_SELECTION_SHIFT_HANDLE_WIDTH: f32 = 14.0;
/// Height in logical pixels for bottom-center selection shift handles.
pub(in crate::gui_runtime::native_vello) const WAVEFORM_SELECTION_SHIFT_HANDLE_HEIGHT: f32 = 7.0;
/// Extra hit slop around bottom-center selection shift handles.
pub(in crate::gui_runtime::native_vello) const WAVEFORM_SELECTION_SHIFT_HANDLE_HIT_INSET: f32 = 4.0;
/// Pixel-delta normalization factor for wheel-driven waveform zoom steps.
pub(in crate::gui_runtime::native_vello) const WAVEFORM_WHEEL_ZOOM_PIXEL_STEP: f32 = 48.0;
/// Integer precision used by pointer-anchored zoom ratios (`0..=1_000_000`).
pub(in crate::gui_runtime::native_vello) const WAVEFORM_ANCHOR_RATIO_MICROS_SCALE: u32 = 1_000_000;
