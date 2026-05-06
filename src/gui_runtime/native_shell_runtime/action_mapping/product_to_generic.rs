use super::super::*;

impl From<UiAction> for runtime_contract::UiAction {
    fn from(value: UiAction) -> Self {
        match value {
            UiAction::SelectColumn { index } => Self::SelectColumn { index },
            UiAction::MoveColumn { delta } => Self::MoveColumn { delta },
            UiAction::ToggleTransport => Self::ToggleTransport,
            UiAction::PlayCompareAnchor => Self::PlayCompareAnchor,
            UiAction::PlayFromStart => Self::PlayFromStart,
            UiAction::PlayFromCurrentPlayhead => Self::PlayFromCurrentPlayhead,
            UiAction::PlayFromWaveformCursor => Self::PlayFromWaveformCursor,
            UiAction::PlayWaveformAtPrecise { position_nanos } => {
                Self::PlayWaveformAtPrecise { position_nanos }
            }
            UiAction::HandleEscape => Self::HandleEscape,
            UiAction::FocusBrowserPanel => Self::FocusBrowserPanel,
            UiAction::FocusSourcesPanel => Self::FocusSourcesPanel,
            UiAction::FocusWaveformPanel => Self::FocusWaveformPanel,
            UiAction::FocusFolderPanel => Self::FocusFolderPanel,
            UiAction::FocusLoadedSampleInBrowser => Self::FocusLoadedContentInList,
            UiAction::FocusBrowserSearch => Self::FocusBrowserSearch,
            UiAction::BlurBrowserSearch => Self::BlurBrowserSearch,
            UiAction::OpenAddSourceDialog => Self::OpenAddSourceDialog,
            UiAction::OpenOptionsMenu => Self::OpenOptionsMenu,
            UiAction::CloseOptionsPanel => Self::CloseOptionsPanel,
            UiAction::PickTrashFolder => Self::PickTrashFolder,
            UiAction::OpenTrashFolder => Self::OpenTrashFolder,
            UiAction::EditDefaultIdentifier => Self::EditDefaultIdentifier,
            UiAction::ShowOptionsOverview => Self::ShowOptionsOverview,
            UiAction::OpenAudioOutputHostPicker => Self::OpenPrimaryGroupPicker,
            UiAction::OpenAudioOutputDevicePicker => Self::OpenPrimaryItemPicker,
            UiAction::OpenAudioOutputSampleRatePicker => Self::OpenPrimaryNumberPicker,
            UiAction::OpenAudioInputHostPicker => Self::OpenSecondaryGroupPicker,
            UiAction::OpenAudioInputDevicePicker => Self::OpenSecondaryItemPicker,
            UiAction::OpenAudioInputSampleRatePicker => Self::OpenSecondaryNumberPicker,
            UiAction::SetAudioOutputHost { host_id } => Self::SetPrimaryGroup { group_id: host_id },
            UiAction::SetAudioOutputDevice { device_name } => Self::SetPrimaryItem {
                item_name: device_name,
            },
            UiAction::SetAudioOutputSampleRate { sample_rate } => {
                Self::SetPrimaryNumber { value: sample_rate }
            }
            UiAction::SetAudioInputHost { host_id } => {
                Self::SetSecondaryGroup { group_id: host_id }
            }
            UiAction::SetAudioInputDevice { device_name } => Self::SetSecondaryItem {
                item_name: device_name,
            },
            UiAction::SetAudioInputSampleRate { sample_rate } => {
                Self::SetSecondaryNumber { value: sample_rate }
            }
            UiAction::FocusFolderSearch => Self::FocusFolderSearch,
            UiAction::SetFolderSearch { query } => Self::SetFolderSearch { query },
            UiAction::ToggleShowAllFolders => Self::ToggleShowAllFolders,
            UiAction::ToggleFolderFlattenedView => Self::ToggleFolderFlattenedView,
            UiAction::FocusSourceRow { index } => Self::FocusSourceRow { index },
            UiAction::SelectSourceRow { index } => Self::SelectSourceRow { index },
            UiAction::MoveSourceFocus { delta } => Self::MoveSourceFocus { delta },
            UiAction::ReloadFocusedSourceRow => Self::ReloadFocusedSourceRow,
            UiAction::HardSyncFocusedSourceRow => Self::HardSyncFocusedSourceRow,
            UiAction::OpenFocusedSourceFolder => Self::OpenFocusedSourceFolder,
            UiAction::RemoveFocusedSourceRow => Self::RemoveFocusedSourceRow,
            UiAction::ReloadSourceRow { index } => Self::ReloadSourceRow { index },
            UiAction::HardSyncSourceRow { index } => Self::HardSyncSourceRow { index },
            UiAction::OpenSourceFolderRow { index } => Self::OpenSourceFolderRow { index },
            UiAction::RemoveSourceRow { index } => Self::RemoveSourceRow { index },
            UiAction::FocusFolderRow { index } => Self::FocusFolderRow { index },
            UiAction::ActivateFolderRow { index } => Self::ActivateFolderRow { index },
            UiAction::ToggleFolderRowExpanded { index } => Self::ToggleFolderRowExpanded { index },
            UiAction::ExpandFocusedFolder => Self::ExpandFocusedFolder,
            UiAction::CollapseFocusedFolder => Self::CollapseFocusedFolder,
            UiAction::ToggleFocusedFolderSelection => Self::ToggleFocusedFolderSelection,
            UiAction::MoveFolderFocus { delta } => Self::MoveFolderFocus { delta },
            UiAction::StartNewFolder => Self::StartNewFolder,
            UiAction::StartNewFolderAtFolderRow { index } => {
                Self::StartNewFolderAtFolderRow { index }
            }
            UiAction::StartNewFolderAtRoot => Self::StartNewFolderAtRoot,
            UiAction::FocusFolderCreateInput => Self::FocusFolderCreateInput,
            UiAction::SetFolderCreateInput { value } => Self::SetFolderCreateInput { value },
            UiAction::ConfirmFolderCreate => Self::ConfirmFolderCreate,
            UiAction::CancelFolderCreate => Self::CancelFolderCreate,
            UiAction::StartFolderRename => Self::StartFolderRename,
            UiAction::DeleteFocusedFolder => Self::DeleteFocusedFolder,
            UiAction::RestoreRetainedFolderDeletes => Self::RestoreRetainedFolderDeletes,
            UiAction::PurgeRetainedFolderDeletes => Self::PurgeRetainedFolderDeletes,
            UiAction::ClearFolderDeleteRecoveryLog => Self::ClearFolderDeleteRecoveryLog,
            UiAction::MoveBrowserFocus { delta } => Self::MoveBrowserFocus { delta },
            UiAction::SetBrowserViewStart { visible_row } => {
                Self::SetBrowserViewStart { visible_row }
            }
            UiAction::FocusBrowserRow { visible_row } => Self::FocusBrowserRow { visible_row },
            UiAction::SetCompareAnchorFromFocusedBrowserSample => {
                Self::SetCompareAnchorFromFocusedContent
            }
            UiAction::CommitFocusedBrowserRow => Self::CommitFocusedBrowserRow,
            UiAction::SaveWaveformSelectionToBrowser => Self::SaveWaveformSelectionToBrowser,
            UiAction::SaveWaveformSelectionToBrowserWithKeep2 => {
                Self::SaveWaveformSelectionToBrowserWithKeep2
            }
            UiAction::CommitWaveformEditFades => Self::CommitWaveformEditFades,
            UiAction::DetectWaveformSilenceSlices => Self::DetectWaveformSilenceSlices,
            UiAction::DetectWaveformExactDuplicateSlices => {
                Self::DetectWaveformExactDuplicateSlices
            }
            UiAction::CleanWaveformExactDuplicateSlices => Self::CleanWaveformExactDuplicateSlices,
            UiAction::ToggleBrowserRowSelection { visible_row } => {
                Self::ToggleBrowserRowSelection { visible_row }
            }
            UiAction::StartBrowserSampleDrag {
                visible_row,
                pointer_x,
                pointer_y,
            } => Self::StartContentItemDrag {
                visible_row,
                pointer_x,
                pointer_y,
            },
            UiAction::UpdateBrowserSampleDrag {
                pointer_x,
                pointer_y,
                hovered_folder_pane,
                hovered_folder_row,
                over_folder_panel,
                shift_down,
                alt_down,
            } => Self::UpdateContentItemDrag {
                pointer_x,
                pointer_y,
                hovered_folder_pane,
                hovered_folder_row,
                over_folder_panel,
                shift_down,
                alt_down,
            },
            UiAction::FinishBrowserSampleDrag => Self::FinishContentItemDrag,
            UiAction::ExtendBrowserSelectionToRow { visible_row } => {
                Self::ExtendBrowserSelectionToRow { visible_row }
            }
            UiAction::AddRangeBrowserSelection { visible_row } => {
                Self::AddRangeBrowserSelection { visible_row }
            }
            UiAction::ExtendBrowserSelectionFromFocus { delta } => {
                Self::ExtendBrowserSelectionFromFocus { delta }
            }
            UiAction::AddRangeBrowserSelectionFromFocus { delta } => {
                Self::AddRangeBrowserSelectionFromFocus { delta }
            }
            UiAction::ToggleFocusedBrowserRowSelection => Self::ToggleFocusedBrowserRowSelection,
            UiAction::SelectAllBrowserRows => Self::SelectAllBrowserRows,
            UiAction::SetBrowserSearch { query } => Self::SetBrowserSearch { query },
            UiAction::ToggleBrowserRatingFilter { level, invert } => {
                Self::ToggleBrowserRatingFilter { level, invert }
            }
            UiAction::ToggleBrowserPlaybackAgeFilter { bucket, invert } => {
                Self::ToggleBrowserPlaybackAgeFilter { bucket, invert }
            }
            UiAction::ToggleBrowserSidebarFilter { option, additive } => {
                Self::ToggleBrowserSidebarFilter { option, additive }
            }
            UiAction::ClearBrowserSidebarFilter { facet } => {
                Self::ClearBrowserSidebarFilter { facet }
            }
            UiAction::ToggleBrowserSampleMark => Self::ToggleContentMark,
            UiAction::ToggleBrowserMarkedFilter => Self::ToggleBrowserMarkedFilter,
            UiAction::ToggleBrowserTagNamedFilter { invert } => {
                Self::ToggleBrowserDerivedLabelFilter { invert }
            }
            UiAction::ToggleRandomNavigationMode => Self::ToggleRandomNavigationMode,
            UiAction::ToggleBrowserTagSidebar => Self::ToggleBrowserPillEditor,
            UiAction::ToggleBrowserTagSidebarAutoRename => {
                Self::ToggleBrowserPillEditorPrimaryAction
            }
            UiAction::ToggleBrowserDuplicateCleanupMode => Self::ToggleBrowserDuplicateCleanupMode,
            UiAction::FocusPreviousBrowserHistory => Self::FocusPreviousBrowserHistory,
            UiAction::FocusNextBrowserHistory => Self::FocusNextBrowserHistory,
            UiAction::ToggleFindSimilarFocusedSample => Self::ToggleFindSimilarFocusedContent,
            UiAction::ToggleBrowserDuplicateCleanupKeep { visible_row } => {
                Self::ToggleBrowserDuplicateCleanupKeep { visible_row }
            }
            UiAction::ConfirmBrowserDuplicateCleanup => Self::ConfirmBrowserDuplicateCleanup,
            UiAction::PlayRandomSample => Self::PlayRandomContentItem,
            UiAction::PlayPreviousRandomSample => Self::PlayPreviousRandomContentItem,
            UiAction::AdjustSelectedBrowserRating { delta } => {
                Self::AdjustSelectedBrowserRating { delta }
            }
            UiAction::SetBrowserTab { map } => Self::SetBrowserTab { map },
            UiAction::FocusBrowserTagSidebarInput => Self::FocusBrowserPillEditorInput,
            UiAction::SetBrowserTagSidebarInput { value } => {
                Self::SetBrowserPillEditorInput { value }
            }
            UiAction::CommitBrowserTagSidebarInput => Self::CommitBrowserPillEditorInput,
            UiAction::SetBrowserSidebarLooped { looped } => {
                Self::SetBrowserSidebarLooped { looped }
            }
            UiAction::ToggleBrowserSidebarNormalTag { label } => {
                Self::ToggleBrowserPillOption { label }
            }
            UiAction::FocusMapSample { sample_id } => Self::FocusSpatialContentItem {
                content_id: sample_id,
            },
            UiAction::SetPromptInput { value } => Self::SetPromptInput { value },
            UiAction::StartBrowserRename => Self::StartBrowserRename,
            UiAction::ConfirmBrowserRename => Self::ConfirmBrowserRename,
            UiAction::CancelBrowserRename => Self::CancelBrowserRename,
            UiAction::AutoRenameBrowserSelection { visible_row } => {
                Self::AutoRenameBrowserSelection { visible_row }
            }
            UiAction::TagBrowserSelection { target } => Self::SetBrowserTriageMark {
                target: target.into(),
            },
            UiAction::DeleteBrowserSelection => Self::DeleteBrowserSelection,
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
        }
    }
}
