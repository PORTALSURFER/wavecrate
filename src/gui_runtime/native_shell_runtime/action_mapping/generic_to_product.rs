use super::super::*;

impl From<runtime_contract::UiAction> for UiAction {
    fn from(value: runtime_contract::UiAction) -> Self {
        let value = match super::shell_sources::generic_to_product(value) {
            Ok(action) => return action,
            Err(value) => value,
        };
        let value = match super::browser_content::generic_to_product(value) {
            Ok(action) => return action,
            Err(value) => value,
        };

        match value {
            runtime_contract::UiAction::NormalizeFocusedContentItem => {
                Self::NormalizeFocusedBrowserSample
            }
            runtime_contract::UiAction::NormalizeWaveformSelectionOrLoadedContent => {
                Self::NormalizeWaveformSelectionOrSample
            }
            runtime_contract::UiAction::CropWaveformSelection => Self::CropWaveformSelection,
            runtime_contract::UiAction::CropWaveformSelectionToNewContentItem => {
                Self::CropWaveformSelectionToNewSample
            }
            runtime_contract::UiAction::TrimWaveformSelection => Self::TrimWaveformSelection,
            runtime_contract::UiAction::ReverseWaveformSelection => Self::ReverseWaveformSelection,
            runtime_contract::UiAction::FadeWaveformSelectionLeftToRight => {
                Self::FadeWaveformSelectionLeftToRight
            }
            runtime_contract::UiAction::FadeWaveformSelectionRightToLeft => {
                Self::FadeWaveformSelectionRightToLeft
            }
            runtime_contract::UiAction::MuteWaveformSelection => Self::MuteWaveformSelection,
            runtime_contract::UiAction::DeleteSelectedSliceMarkers => {
                Self::DeleteSelectedSliceMarkers
            }
            runtime_contract::UiAction::ToggleWaveformSliceSelection { index } => {
                Self::ToggleWaveformSliceSelection { index }
            }
            runtime_contract::UiAction::AuditionWaveformDuplicateSlice { index } => {
                Self::AuditionWaveformDuplicateSlice { index }
            }
            runtime_contract::UiAction::ToggleWaveformDuplicateSliceExemption { index } => {
                Self::ToggleWaveformDuplicateSliceExemption { index }
            }
            runtime_contract::UiAction::MoveWaveformSliceFocus { delta } => {
                Self::MoveWaveformSliceFocus { delta }
            }
            runtime_contract::UiAction::ToggleFocusedWaveformSliceExportMark => {
                Self::ToggleFocusedWaveformSliceExportMark
            }
            runtime_contract::UiAction::AlignWaveformStartToMarker => {
                Self::AlignWaveformStartToMarker
            }
            runtime_contract::UiAction::DeleteLoadedWaveformContent => {
                Self::DeleteLoadedWaveformSample
            }
            runtime_contract::UiAction::SlideWaveformSelection { delta, fine } => {
                Self::SlideWaveformSelection { delta, fine }
            }
            runtime_contract::UiAction::ConfirmPrompt => Self::ConfirmPrompt,
            runtime_contract::UiAction::CancelPrompt => Self::CancelPrompt,
            runtime_contract::UiAction::CancelProgress => Self::CancelProgress,
            runtime_contract::UiAction::CopySelectionToClipboard => Self::CopySelectionToClipboard,
            runtime_contract::UiAction::ToggleHotkeyOverlay => Self::ToggleHotkeyOverlay,
            runtime_contract::UiAction::CopyStatusLog => Self::CopyStatusLog,
            runtime_contract::UiAction::OpenFeedbackIssuePrompt => Self::OpenFeedbackIssuePrompt,
            runtime_contract::UiAction::MoveDiscardedItemsToFolder => {
                Self::MoveTrashedSamplesToFolder
            }
            runtime_contract::UiAction::SetInputMonitoringEnabled { enabled } => {
                Self::SetInputMonitoringEnabled { enabled }
            }
            runtime_contract::UiAction::SetAdvanceAfterRatingEnabled { enabled } => {
                Self::SetAdvanceAfterRatingEnabled { enabled }
            }
            runtime_contract::UiAction::SetDestructiveYoloMode { enabled } => {
                Self::SetDestructiveYoloMode { enabled }
            }
            runtime_contract::UiAction::SetInvertWaveformScroll { enabled } => {
                Self::SetInvertWaveformScroll { enabled }
            }
            runtime_contract::UiAction::ToggleLoopPlayback => Self::ToggleLoopPlayback,
            runtime_contract::UiAction::ToggleLoopLock => Self::ToggleLoopLock,
            runtime_contract::UiAction::SetWaveformChannelView { stereo } => {
                Self::SetWaveformChannelView { stereo }
            }
            runtime_contract::UiAction::SetNormalizedAuditionEnabled { enabled } => {
                Self::SetNormalizedAuditionEnabled { enabled }
            }
            runtime_contract::UiAction::SetBpmSnapEnabled { enabled } => {
                Self::SetBpmSnapEnabled { enabled }
            }
            runtime_contract::UiAction::SetRelativeBpmGridEnabled { enabled } => {
                Self::SetRelativeBpmGridEnabled { enabled }
            }
            runtime_contract::UiAction::AdjustWaveformBpm { delta } => {
                Self::AdjustWaveformBpm { delta }
            }
            runtime_contract::UiAction::SetWaveformBpmValue { value_tenths } => {
                Self::SetWaveformBpmValue { value_tenths }
            }
            runtime_contract::UiAction::SetTransientSnapEnabled { enabled } => {
                Self::SetTransientSnapEnabled { enabled }
            }
            runtime_contract::UiAction::SetTransientMarkersEnabled { enabled } => {
                Self::SetTransientMarkersEnabled { enabled }
            }
            runtime_contract::UiAction::ToggleTransientMarkers => Self::ToggleTransientMarkers,
            runtime_contract::UiAction::ToggleBpmSnap => Self::ToggleBpmSnap,
            runtime_contract::UiAction::SetSliceModeEnabled { enabled } => {
                Self::SetSliceModeEnabled { enabled }
            }
            runtime_contract::UiAction::SetVolume { value_milli } => {
                Self::SetVolume { value_milli }
            }
            runtime_contract::UiAction::CommitVolumeSetting => Self::CommitVolumeSetting,
            runtime_contract::UiAction::SeekWaveformPrecise { position_nanos } => {
                Self::SeekWaveformPrecise { position_nanos }
            }
            runtime_contract::UiAction::SetWaveformCursorPrecise { position_nanos } => {
                Self::SetWaveformCursorPrecise { position_nanos }
            }
            runtime_contract::UiAction::SeekWaveform { position_milli } => {
                Self::SeekWaveform { position_milli }
            }
            runtime_contract::UiAction::SetWaveformCursor { position_milli } => {
                Self::SetWaveformCursor { position_milli }
            }
            runtime_contract::UiAction::BeginWaveformSelectionAt { anchor_micros } => {
                Self::BeginWaveformSelectionAt { anchor_micros }
            }
            runtime_contract::UiAction::BeginWaveformSelectionAtPrecise { anchor_nanos } => {
                Self::BeginWaveformSelectionAtPrecise { anchor_nanos }
            }
            runtime_contract::UiAction::BeginWaveformCircularSlide { anchor_micros } => {
                Self::BeginWaveformCircularSlide { anchor_micros }
            }
            runtime_contract::UiAction::UpdateWaveformCircularSlide { position_micros } => {
                Self::UpdateWaveformCircularSlide { position_micros }
            }
            runtime_contract::UiAction::FinishWaveformCircularSlide => {
                Self::FinishWaveformCircularSlide
            }
            runtime_contract::UiAction::SetWaveformSelectionRange {
                start_micros,
                end_micros,
                snap_override,
                preserve_view_edge,
            } => Self::SetWaveformSelectionRange {
                start_micros,
                end_micros,
                snap_override,
                preserve_view_edge,
            },
            runtime_contract::UiAction::SetWaveformSelectionRangePrecise {
                start_nanos,
                end_nanos,
                snap_override,
                preserve_view_edge,
            } => Self::SetWaveformSelectionRangePrecise {
                start_nanos,
                end_nanos,
                snap_override,
                preserve_view_edge,
            },
            runtime_contract::UiAction::SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            } => Self::SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            },
            runtime_contract::UiAction::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos,
                end_nanos,
            } => Self::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos,
                end_nanos,
            },
            runtime_contract::UiAction::SetWaveformEditSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge,
            } => Self::SetWaveformEditSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge,
            },
            runtime_contract::UiAction::SetWaveformEditSelectionRangePrecise {
                start_nanos,
                end_nanos,
                preserve_view_edge,
            } => Self::SetWaveformEditSelectionRangePrecise {
                start_nanos,
                end_nanos,
                preserve_view_edge,
            },
            runtime_contract::UiAction::SetWaveformEditFadeInEnd { position_micros } => {
                Self::SetWaveformEditFadeInEnd { position_micros }
            }
            runtime_contract::UiAction::SetWaveformEditFadeInMuteStart { position_micros } => {
                Self::SetWaveformEditFadeInMuteStart { position_micros }
            }
            runtime_contract::UiAction::SetWaveformEditFadeInCurve { curve_milli } => {
                Self::SetWaveformEditFadeInCurve { curve_milli }
            }
            runtime_contract::UiAction::SetWaveformEditFadeOutStart { position_micros } => {
                Self::SetWaveformEditFadeOutStart { position_micros }
            }
            runtime_contract::UiAction::SetWaveformEditFadeOutMuteEnd { position_micros } => {
                Self::SetWaveformEditFadeOutMuteEnd { position_micros }
            }
            runtime_contract::UiAction::SetWaveformEditFadeOutCurve { curve_milli } => {
                Self::SetWaveformEditFadeOutCurve { curve_milli }
            }
            runtime_contract::UiAction::FinishWaveformEditFadeDrag => {
                Self::FinishWaveformEditFadeDrag
            }
            runtime_contract::UiAction::StartWaveformSelectionDrag {
                pointer_x,
                pointer_y,
            } => Self::StartWaveformSelectionDrag {
                pointer_x,
                pointer_y,
            },
            runtime_contract::UiAction::UpdateWaveformSelectionDrag {
                pointer_x,
                pointer_y,
                hovered_folder_pane,
                hovered_folder_row,
                over_folder_panel,
                over_browser_list,
                shift_down,
                alt_down,
            } => Self::UpdateWaveformSelectionDrag {
                pointer_x,
                pointer_y,
                hovered_folder_pane,
                hovered_folder_row,
                over_folder_panel,
                over_browser_list,
                shift_down,
                alt_down,
            },
            runtime_contract::UiAction::FinishWaveformSelectionDrag => {
                Self::FinishWaveformSelectionDrag
            }
            runtime_contract::UiAction::FinishWaveformSelectionRangeDrag => {
                Self::FinishWaveformSelectionRangeDrag
            }
            runtime_contract::UiAction::FinishWaveformSelectionSmartScaleDrag => {
                Self::FinishWaveformSelectionSmartScaleDrag
            }
            runtime_contract::UiAction::BeginWaveformSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            } => Self::BeginWaveformSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            },
            runtime_contract::UiAction::BeginWaveformSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            } => Self::BeginWaveformSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            },
            runtime_contract::UiAction::BeginWaveformEditSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            } => Self::BeginWaveformEditSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            },
            runtime_contract::UiAction::BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            } => Self::BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            },
            runtime_contract::UiAction::FinishWaveformEditSelectionDrag => {
                Self::FinishWaveformEditSelectionDrag
            }
            runtime_contract::UiAction::ClearWaveformSelection => Self::ClearWaveformSelection,
            runtime_contract::UiAction::ClearWaveformEditSelection => {
                Self::ClearWaveformEditSelection
            }
            runtime_contract::UiAction::ClearWaveformSelections => Self::ClearWaveformSelections,
            runtime_contract::UiAction::SetWaveformViewCenter {
                center_micros,
                center_nanos,
            } => Self::SetWaveformViewCenter {
                center_micros,
                center_nanos,
            },
            runtime_contract::UiAction::ZoomWaveform {
                zoom_in,
                steps,
                anchor_ratio_micros,
            } => Self::ZoomWaveform {
                zoom_in,
                steps,
                anchor_ratio_micros,
            },
            runtime_contract::UiAction::ZoomWaveformToSelection => Self::ZoomWaveformToSelection,
            runtime_contract::UiAction::ZoomWaveformFull => Self::ZoomWaveformFull,
            runtime_contract::UiAction::Undo => Self::Undo,
            runtime_contract::UiAction::Redo => Self::Redo,
            runtime_contract::UiAction::CheckForUpdates => Self::CheckForUpdates,
            runtime_contract::UiAction::OpenUpdateLink => Self::OpenUpdateLink,
            runtime_contract::UiAction::InstallUpdate => Self::InstallUpdate,
            runtime_contract::UiAction::DismissUpdate => Self::DismissUpdate,
            other => unreachable!(
                "shell/source action mapper must claim its domain before browser/waveform mapping: {other:?}"
            ),
        }
    }
}
