use serde::{Deserialize, Serialize};

use super::super::ui_projection_dtos::FolderPaneIdModel;

/// Waveform transport, edit, and gesture actions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaveformAction {
    SeekWaveformPrecise {
        position_nanos: u32,
    },
    SetWaveformCursorPrecise {
        position_nanos: u32,
    },
    BeginWaveformSelectionAt {
        anchor_micros: u32,
    },
    BeginWaveformSelectionAtPrecise {
        anchor_nanos: u32,
    },
    BeginWaveformCircularSlide {
        anchor_micros: u32,
    },
    UpdateWaveformCircularSlide {
        position_micros: u32,
    },
    FinishWaveformCircularSlide,
    SetWaveformSelectionRange {
        start_micros: u32,
        end_micros: u32,
        snap_override: bool,
        preserve_view_edge: bool,
    },
    SetWaveformSelectionRangePrecise {
        start_nanos: u32,
        end_nanos: u32,
        snap_override: bool,
        preserve_view_edge: bool,
    },
    SetWaveformSelectionRangeSmartScale {
        start_micros: u32,
        end_micros: u32,
    },
    SetWaveformSelectionRangeSmartScalePrecise {
        start_nanos: u32,
        end_nanos: u32,
    },
    SetWaveformEditSelectionRange {
        start_micros: u32,
        end_micros: u32,
        preserve_view_edge: bool,
    },
    SetWaveformEditSelectionRangePrecise {
        start_nanos: u32,
        end_nanos: u32,
        preserve_view_edge: bool,
    },
    SetWaveformEditFadeInEnd {
        position_micros: u32,
    },
    SetWaveformEditFadeInMuteStart {
        position_micros: u32,
    },
    SetWaveformEditFadeInCurve {
        curve_milli: u16,
    },
    SetWaveformEditFadeOutStart {
        position_micros: u32,
    },
    SetWaveformEditFadeOutMuteEnd {
        position_micros: u32,
    },
    SetWaveformEditFadeOutCurve {
        curve_milli: u16,
    },
    FinishWaveformEditFadeDrag,
    StartWaveformSelectionDrag {
        pointer_x: u16,
        pointer_y: u16,
    },
    UpdateWaveformSelectionDrag {
        pointer_x: u16,
        pointer_y: u16,
        hovered_folder_pane: Option<FolderPaneIdModel>,
        hovered_folder_row: Option<usize>,
        over_folder_panel: Option<FolderPaneIdModel>,
        over_browser_list: bool,
        shift_down: bool,
        alt_down: bool,
    },
    FinishWaveformSelectionDrag,
    FinishWaveformSelectionRangeDrag,
    FinishWaveformSelectionSmartScaleDrag,
    BeginWaveformSelectionShift {
        pointer_micros: u32,
        start_micros: u32,
        end_micros: u32,
    },
    BeginWaveformSelectionShiftPrecise {
        pointer_nanos: u32,
        start_nanos: u32,
        end_nanos: u32,
    },
    BeginWaveformEditSelectionShift {
        pointer_micros: u32,
        start_micros: u32,
        end_micros: u32,
    },
    BeginWaveformEditSelectionShiftPrecise {
        pointer_nanos: u32,
        start_nanos: u32,
        end_nanos: u32,
    },
    FinishWaveformEditSelectionDrag,
    ClearWaveformSelection,
    ClearWaveformEditSelection,
    ClearWaveformSelections,
    SetWaveformViewCenter {
        center_micros: u32,
        center_nanos: Option<u32>,
    },
    ZoomWaveform {
        zoom_in: bool,
        steps: u8,
        anchor_ratio_micros: Option<u32>,
    },
    ZoomWaveformToSelection,
    ZoomWaveformFull,
}
