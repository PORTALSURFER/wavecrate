use super::super::*;

impl From<UiAction> for runtime_contract::UiAction {
    fn from(value: UiAction) -> Self {
        let value = match super::shell_sources::product_to_generic(value) {
            Ok(action) => return action,
            Err(value) => value,
        };
        let value = match super::browser_content::product_to_generic(value) {
            Ok(action) => return action,
            Err(value) => value,
        };

        match value {
            UiAction::NormalizeFocusedBrowserSample => Self::NormalizeFocusedContentItem,
            UiAction::NormalizeWaveformSelectionOrSample => {
                Self::NormalizeWaveformSelectionOrLoadedContent
            }
            UiAction::CropWaveformSelection => Self::CropWaveformSelection,
            UiAction::CropWaveformSelectionToNewSample => {
                Self::CropWaveformSelectionToNewContentItem
            }
            UiAction::TrimWaveformSelection => Self::TrimWaveformSelection,
            UiAction::ReverseWaveformSelection => Self::ReverseWaveformSelection,
            UiAction::FadeWaveformSelectionLeftToRight => Self::FadeWaveformSelectionLeftToRight,
            UiAction::FadeWaveformSelectionRightToLeft => Self::FadeWaveformSelectionRightToLeft,
            UiAction::MuteWaveformSelection => Self::MuteWaveformSelection,
            UiAction::DeleteSelectedSliceMarkers => Self::DeleteSelectedSliceMarkers,
            UiAction::ToggleWaveformSliceSelection { index } => {
                Self::ToggleWaveformSliceSelection { index }
            }
            UiAction::AuditionWaveformDuplicateSlice { index } => {
                Self::AuditionWaveformDuplicateSlice { index }
            }
            UiAction::ToggleWaveformDuplicateSliceExemption { index } => {
                Self::ToggleWaveformDuplicateSliceExemption { index }
            }
            UiAction::MoveWaveformSliceFocus { delta } => Self::MoveWaveformSliceFocus { delta },
            UiAction::ToggleFocusedWaveformSliceExportMark => {
                Self::ToggleFocusedWaveformSliceExportMark
            }
            UiAction::AlignWaveformStartToMarker => Self::AlignWaveformStartToMarker,
            UiAction::DeleteLoadedWaveformSample => Self::DeleteLoadedWaveformContent,
            UiAction::SlideWaveformSelection { delta, fine } => {
                Self::SlideWaveformSelection { delta, fine }
            }
            UiAction::ConfirmPrompt => Self::ConfirmPrompt,
            UiAction::CancelPrompt => Self::CancelPrompt,
            UiAction::CancelProgress => Self::CancelProgress,
            UiAction::CopySelectionToClipboard => Self::CopySelectionToClipboard,
            UiAction::ToggleHotkeyOverlay => Self::ToggleHotkeyOverlay,
            UiAction::CopyStatusLog => Self::CopyStatusLog,
            UiAction::OpenFeedbackIssuePrompt => Self::OpenFeedbackIssuePrompt,
            UiAction::MoveTrashedSamplesToFolder => Self::MoveDiscardedItemsToFolder,
            UiAction::SetInputMonitoringEnabled { enabled } => {
                Self::SetInputMonitoringEnabled { enabled }
            }
            UiAction::SetAdvanceAfterRatingEnabled { enabled } => {
                Self::SetAdvanceAfterRatingEnabled { enabled }
            }
            UiAction::SetDestructiveYoloMode { enabled } => {
                Self::SetDestructiveYoloMode { enabled }
            }
            UiAction::SetInvertWaveformScroll { enabled } => {
                Self::SetInvertWaveformScroll { enabled }
            }
            UiAction::ToggleLoopPlayback => Self::ToggleLoopPlayback,
            UiAction::ToggleLoopLock => Self::ToggleLoopLock,
            UiAction::SetWaveformChannelView { stereo } => Self::SetWaveformChannelView { stereo },
            UiAction::SetNormalizedAuditionEnabled { enabled } => {
                Self::SetNormalizedAuditionEnabled { enabled }
            }
            UiAction::SetBpmSnapEnabled { enabled } => Self::SetBpmSnapEnabled { enabled },
            UiAction::SetRelativeBpmGridEnabled { enabled } => {
                Self::SetRelativeBpmGridEnabled { enabled }
            }
            UiAction::AdjustWaveformBpm { delta } => Self::AdjustWaveformBpm { delta },
            UiAction::SetWaveformBpmValue { value_tenths } => {
                Self::SetWaveformBpmValue { value_tenths }
            }
            UiAction::SetTransientSnapEnabled { enabled } => {
                Self::SetTransientSnapEnabled { enabled }
            }
            UiAction::SetTransientMarkersEnabled { enabled } => {
                Self::SetTransientMarkersEnabled { enabled }
            }
            UiAction::ToggleTransientMarkers => Self::ToggleTransientMarkers,
            UiAction::ToggleBpmSnap => Self::ToggleBpmSnap,
            UiAction::SetSliceModeEnabled { enabled } => Self::SetSliceModeEnabled { enabled },
            UiAction::SetVolume { value_milli } => Self::SetVolume { value_milli },
            UiAction::CommitVolumeSetting => Self::CommitVolumeSetting,
            UiAction::SeekWaveformPrecise { position_nanos } => {
                Self::SeekWaveformPrecise { position_nanos }
            }
            UiAction::SetWaveformCursorPrecise { position_nanos } => {
                Self::SetWaveformCursorPrecise { position_nanos }
            }
            UiAction::SeekWaveform { position_milli } => Self::SeekWaveform { position_milli },
            UiAction::SetWaveformCursor { position_milli } => {
                Self::SetWaveformCursor { position_milli }
            }
            UiAction::BeginWaveformSelectionAt { anchor_micros } => {
                Self::BeginWaveformSelectionAt { anchor_micros }
            }
            UiAction::BeginWaveformSelectionAtPrecise { anchor_nanos } => {
                Self::BeginWaveformSelectionAtPrecise { anchor_nanos }
            }
            UiAction::BeginWaveformCircularSlide { anchor_micros } => {
                Self::BeginWaveformCircularSlide { anchor_micros }
            }
            UiAction::UpdateWaveformCircularSlide { position_micros } => {
                Self::UpdateWaveformCircularSlide { position_micros }
            }
            UiAction::FinishWaveformCircularSlide => Self::FinishWaveformCircularSlide,
            UiAction::SetWaveformSelectionRange {
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
            UiAction::SetWaveformSelectionRangePrecise {
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
            UiAction::SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            } => Self::SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            },
            UiAction::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos,
                end_nanos,
            } => Self::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos,
                end_nanos,
            },
            UiAction::SetWaveformEditSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge,
            } => Self::SetWaveformEditSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge,
            },
            UiAction::SetWaveformEditSelectionRangePrecise {
                start_nanos,
                end_nanos,
                preserve_view_edge,
            } => Self::SetWaveformEditSelectionRangePrecise {
                start_nanos,
                end_nanos,
                preserve_view_edge,
            },
            UiAction::SetWaveformEditFadeInEnd { position_micros } => {
                Self::SetWaveformEditFadeInEnd { position_micros }
            }
            UiAction::SetWaveformEditFadeInMuteStart { position_micros } => {
                Self::SetWaveformEditFadeInMuteStart { position_micros }
            }
            UiAction::SetWaveformEditFadeInCurve { curve_milli } => {
                Self::SetWaveformEditFadeInCurve { curve_milli }
            }
            UiAction::SetWaveformEditFadeOutStart { position_micros } => {
                Self::SetWaveformEditFadeOutStart { position_micros }
            }
            UiAction::SetWaveformEditFadeOutMuteEnd { position_micros } => {
                Self::SetWaveformEditFadeOutMuteEnd { position_micros }
            }
            UiAction::SetWaveformEditFadeOutCurve { curve_milli } => {
                Self::SetWaveformEditFadeOutCurve { curve_milli }
            }
            UiAction::FinishWaveformEditFadeDrag => Self::FinishWaveformEditFadeDrag,
            UiAction::StartWaveformSelectionDrag {
                pointer_x,
                pointer_y,
            } => Self::StartWaveformSelectionDrag {
                pointer_x,
                pointer_y,
            },
            UiAction::UpdateWaveformSelectionDrag {
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
            UiAction::FinishWaveformSelectionDrag => Self::FinishWaveformSelectionDrag,
            UiAction::FinishWaveformSelectionRangeDrag => Self::FinishWaveformSelectionRangeDrag,
            UiAction::FinishWaveformSelectionSmartScaleDrag => {
                Self::FinishWaveformSelectionSmartScaleDrag
            }
            UiAction::BeginWaveformSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            } => Self::BeginWaveformSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            },
            UiAction::BeginWaveformSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            } => Self::BeginWaveformSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            },
            UiAction::BeginWaveformEditSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            } => Self::BeginWaveformEditSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            },
            UiAction::BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            } => Self::BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            },
            UiAction::FinishWaveformEditSelectionDrag => Self::FinishWaveformEditSelectionDrag,
            UiAction::ClearWaveformSelection => Self::ClearWaveformSelection,
            UiAction::ClearWaveformEditSelection => Self::ClearWaveformEditSelection,
            UiAction::ClearWaveformSelections => Self::ClearWaveformSelections,
            UiAction::SetWaveformViewCenter {
                center_micros,
                center_nanos,
            } => Self::SetWaveformViewCenter {
                center_micros,
                center_nanos,
            },
            UiAction::ZoomWaveform {
                zoom_in,
                steps,
                anchor_ratio_micros,
            } => Self::ZoomWaveform {
                zoom_in,
                steps,
                anchor_ratio_micros,
            },
            UiAction::ZoomWaveformToSelection => Self::ZoomWaveformToSelection,
            UiAction::ZoomWaveformFull => Self::ZoomWaveformFull,
            UiAction::Undo => Self::Undo,
            UiAction::Redo => Self::Redo,
            UiAction::CheckForUpdates => Self::CheckForUpdates,
            UiAction::OpenUpdateLink => Self::OpenUpdateLink,
            UiAction::InstallUpdate => Self::InstallUpdate,
            UiAction::DismissUpdate => Self::DismissUpdate,
            other => unreachable!(
                "shell/source action mapper must claim its domain before browser/waveform mapping: {other:?}"
            ),
        }
    }
}
