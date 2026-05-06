use super::*;

impl From<compat::BrowserTriageTarget> for BrowserTagTarget {
    fn from(value: compat::BrowserTriageTarget) -> Self {
        match value {
            compat::BrowserTriageTarget::Negative => Self::Trash,
            compat::BrowserTriageTarget::Neutral => Self::Neutral,
            compat::BrowserTriageTarget::Positive => Self::Keep,
        }
    }
}

impl From<BrowserTagTarget> for compat::BrowserTriageTarget {
    fn from(value: BrowserTagTarget) -> Self {
        match value {
            BrowserTagTarget::Trash => Self::Negative,
            BrowserTagTarget::Neutral => Self::Neutral,
            BrowserTagTarget::Keep => Self::Positive,
        }
    }
}

impl From<compat::UiAction> for UiAction {
    fn from(value: compat::UiAction) -> Self {
        match value {
            compat::UiAction::SelectColumn { index } => Self::SelectColumn { index },
            compat::UiAction::MoveColumn { delta } => Self::MoveColumn { delta },
            compat::UiAction::ToggleTransport => Self::ToggleTransport,
            compat::UiAction::PlayCompareAnchor => Self::PlayCompareAnchor,
            compat::UiAction::PlayFromStart => Self::PlayFromStart,
            compat::UiAction::PlayFromCurrentPlayhead => Self::PlayFromCurrentPlayhead,
            compat::UiAction::PlayFromWaveformCursor => Self::PlayFromWaveformCursor,
            compat::UiAction::PlayWaveformAtPrecise { position_nanos } => {
                Self::PlayWaveformAtPrecise { position_nanos }
            }
            compat::UiAction::HandleEscape => Self::HandleEscape,
            compat::UiAction::FocusBrowserPanel => Self::FocusBrowserPanel,
            compat::UiAction::FocusSourcesPanel => Self::FocusSourcesPanel,
            compat::UiAction::FocusWaveformPanel => Self::FocusWaveformPanel,
            compat::UiAction::FocusFolderPanel => Self::FocusFolderPanel,
            compat::UiAction::FocusLoadedContentInList => Self::FocusLoadedSampleInBrowser,
            compat::UiAction::FocusBrowserSearch => Self::FocusBrowserSearch,
            compat::UiAction::BlurBrowserSearch => Self::BlurBrowserSearch,
            compat::UiAction::OpenAddSourceDialog => Self::OpenAddSourceDialog,
            compat::UiAction::OpenOptionsMenu => Self::OpenOptionsMenu,
            compat::UiAction::CloseOptionsPanel => Self::CloseOptionsPanel,
            compat::UiAction::PickTrashFolder => Self::PickTrashFolder,
            compat::UiAction::OpenTrashFolder => Self::OpenTrashFolder,
            compat::UiAction::EditDefaultIdentifier => Self::EditDefaultIdentifier,
            compat::UiAction::ShowOptionsOverview => Self::ShowOptionsOverview,
            compat::UiAction::OpenPrimaryGroupPicker => Self::OpenAudioOutputHostPicker,
            compat::UiAction::OpenPrimaryItemPicker => Self::OpenAudioOutputDevicePicker,
            compat::UiAction::OpenPrimaryNumberPicker => Self::OpenAudioOutputSampleRatePicker,
            compat::UiAction::OpenSecondaryGroupPicker => Self::OpenAudioInputHostPicker,
            compat::UiAction::OpenSecondaryItemPicker => Self::OpenAudioInputDevicePicker,
            compat::UiAction::OpenSecondaryNumberPicker => Self::OpenAudioInputSampleRatePicker,
            compat::UiAction::SetPrimaryGroup { group_id } => {
                Self::SetAudioOutputHost { host_id: group_id }
            }
            compat::UiAction::SetPrimaryItem { item_name } => Self::SetAudioOutputDevice {
                device_name: item_name,
            },
            compat::UiAction::SetPrimaryNumber { value } => {
                Self::SetAudioOutputSampleRate { sample_rate: value }
            }
            compat::UiAction::SetSecondaryGroup { group_id } => {
                Self::SetAudioInputHost { host_id: group_id }
            }
            compat::UiAction::SetSecondaryItem { item_name } => Self::SetAudioInputDevice {
                device_name: item_name,
            },
            compat::UiAction::SetSecondaryNumber { value } => {
                Self::SetAudioInputSampleRate { sample_rate: value }
            }
            compat::UiAction::FocusFolderSearch => Self::FocusFolderSearch,
            compat::UiAction::SetFolderSearch { query } => Self::SetFolderSearch { query },
            compat::UiAction::ToggleShowAllFolders => Self::ToggleShowAllFolders,
            compat::UiAction::ToggleFolderFlattenedView => Self::ToggleFolderFlattenedView,
            compat::UiAction::FocusSourceRow { index } => Self::FocusSourceRow { index },
            compat::UiAction::SelectSourceRow { index } => Self::SelectSourceRow { index },
            compat::UiAction::MoveSourceFocus { delta } => Self::MoveSourceFocus { delta },
            compat::UiAction::ReloadFocusedSourceRow => Self::ReloadFocusedSourceRow,
            compat::UiAction::HardSyncFocusedSourceRow => Self::HardSyncFocusedSourceRow,
            compat::UiAction::OpenFocusedSourceFolder => Self::OpenFocusedSourceFolder,
            compat::UiAction::RemoveFocusedSourceRow => Self::RemoveFocusedSourceRow,
            compat::UiAction::ReloadSourceRow { index } => Self::ReloadSourceRow { index },
            compat::UiAction::HardSyncSourceRow { index } => Self::HardSyncSourceRow { index },
            compat::UiAction::OpenSourceFolderRow { index } => Self::OpenSourceFolderRow { index },
            compat::UiAction::RemoveSourceRow { index } => Self::RemoveSourceRow { index },
            compat::UiAction::FocusFolderRow { index } => Self::FocusFolderRow { index },
            compat::UiAction::ActivateFolderRow { index } => Self::ActivateFolderRow { index },
            compat::UiAction::ToggleFolderRowExpanded { index } => {
                Self::ToggleFolderRowExpanded { index }
            }
            compat::UiAction::ExpandFocusedFolder => Self::ExpandFocusedFolder,
            compat::UiAction::CollapseFocusedFolder => Self::CollapseFocusedFolder,
            compat::UiAction::ToggleFocusedFolderSelection => Self::ToggleFocusedFolderSelection,
            compat::UiAction::MoveFolderFocus { delta } => Self::MoveFolderFocus { delta },
            compat::UiAction::StartNewFolder => Self::StartNewFolder,
            compat::UiAction::StartNewFolderAtFolderRow { index } => {
                Self::StartNewFolderAtFolderRow { index }
            }
            compat::UiAction::StartNewFolderAtRoot => Self::StartNewFolderAtRoot,
            compat::UiAction::FocusFolderCreateInput => Self::FocusFolderCreateInput,
            compat::UiAction::SetFolderCreateInput { value } => {
                Self::SetFolderCreateInput { value }
            }
            compat::UiAction::ConfirmFolderCreate => Self::ConfirmFolderCreate,
            compat::UiAction::CancelFolderCreate => Self::CancelFolderCreate,
            compat::UiAction::StartFolderRename => Self::StartFolderRename,
            compat::UiAction::DeleteFocusedFolder => Self::DeleteFocusedFolder,
            compat::UiAction::RestoreRetainedFolderDeletes => Self::RestoreRetainedFolderDeletes,
            compat::UiAction::PurgeRetainedFolderDeletes => Self::PurgeRetainedFolderDeletes,
            compat::UiAction::ClearFolderDeleteRecoveryLog => Self::ClearFolderDeleteRecoveryLog,
            compat::UiAction::MoveBrowserFocus { delta } => Self::MoveBrowserFocus { delta },
            compat::UiAction::SetBrowserViewStart { visible_row } => {
                Self::SetBrowserViewStart { visible_row }
            }
            compat::UiAction::FocusBrowserRow { visible_row } => {
                Self::FocusBrowserRow { visible_row }
            }
            compat::UiAction::SetCompareAnchorFromFocusedContent => {
                Self::SetCompareAnchorFromFocusedBrowserSample
            }
            compat::UiAction::CommitFocusedBrowserRow => Self::CommitFocusedBrowserRow,
            compat::UiAction::SaveWaveformSelectionToBrowser => {
                Self::SaveWaveformSelectionToBrowser
            }
            compat::UiAction::SaveWaveformSelectionToBrowserWithKeep2 => {
                Self::SaveWaveformSelectionToBrowserWithKeep2
            }
            compat::UiAction::CommitWaveformEditFades => Self::CommitWaveformEditFades,
            compat::UiAction::DetectWaveformSilenceSlices => Self::DetectWaveformSilenceSlices,
            compat::UiAction::DetectWaveformExactDuplicateSlices => {
                Self::DetectWaveformExactDuplicateSlices
            }
            compat::UiAction::CleanWaveformExactDuplicateSlices => {
                Self::CleanWaveformExactDuplicateSlices
            }
            compat::UiAction::ToggleBrowserRowSelection { visible_row } => {
                Self::ToggleBrowserRowSelection { visible_row }
            }
            compat::UiAction::StartContentItemDrag {
                visible_row,
                pointer_x,
                pointer_y,
            } => Self::StartBrowserSampleDrag {
                visible_row,
                pointer_x,
                pointer_y,
            },
            compat::UiAction::UpdateContentItemDrag {
                pointer_x,
                pointer_y,
                hovered_folder_pane,
                hovered_folder_row,
                over_folder_panel,
                shift_down,
                alt_down,
            } => Self::UpdateBrowserSampleDrag {
                pointer_x,
                pointer_y,
                hovered_folder_pane,
                hovered_folder_row,
                over_folder_panel,
                shift_down,
                alt_down,
            },
            compat::UiAction::FinishContentItemDrag => Self::FinishBrowserSampleDrag,
            compat::UiAction::ExtendBrowserSelectionToRow { visible_row } => {
                Self::ExtendBrowserSelectionToRow { visible_row }
            }
            compat::UiAction::AddRangeBrowserSelection { visible_row } => {
                Self::AddRangeBrowserSelection { visible_row }
            }
            compat::UiAction::ExtendBrowserSelectionFromFocus { delta } => {
                Self::ExtendBrowserSelectionFromFocus { delta }
            }
            compat::UiAction::AddRangeBrowserSelectionFromFocus { delta } => {
                Self::AddRangeBrowserSelectionFromFocus { delta }
            }
            compat::UiAction::ToggleFocusedBrowserRowSelection => {
                Self::ToggleFocusedBrowserRowSelection
            }
            compat::UiAction::SelectAllBrowserRows => Self::SelectAllBrowserRows,
            compat::UiAction::SetBrowserSearch { query } => Self::SetBrowserSearch { query },
            compat::UiAction::ToggleBrowserRatingFilter { level, invert } => {
                Self::ToggleBrowserRatingFilter { level, invert }
            }
            compat::UiAction::ToggleBrowserPlaybackAgeFilter { bucket, invert } => {
                Self::ToggleBrowserPlaybackAgeFilter { bucket, invert }
            }
            compat::UiAction::ToggleBrowserSidebarFilter { option, additive } => {
                Self::ToggleBrowserSidebarFilter { option, additive }
            }
            compat::UiAction::ClearBrowserSidebarFilter { facet } => {
                Self::ClearBrowserSidebarFilter { facet }
            }
            compat::UiAction::ToggleContentMark => Self::ToggleBrowserSampleMark,
            compat::UiAction::ToggleBrowserMarkedFilter => Self::ToggleBrowserMarkedFilter,
            compat::UiAction::ToggleBrowserDerivedLabelFilter { invert } => {
                Self::ToggleBrowserTagNamedFilter { invert }
            }
            compat::UiAction::ToggleRandomNavigationMode => Self::ToggleRandomNavigationMode,
            compat::UiAction::ToggleBrowserPillEditor => Self::ToggleBrowserTagSidebar,
            compat::UiAction::ToggleBrowserPillEditorPrimaryAction => {
                Self::ToggleBrowserTagSidebarAutoRename
            }
            compat::UiAction::ToggleBrowserDuplicateCleanupMode => {
                Self::ToggleBrowserDuplicateCleanupMode
            }
            compat::UiAction::FocusPreviousBrowserHistory => Self::FocusPreviousBrowserHistory,
            compat::UiAction::FocusNextBrowserHistory => Self::FocusNextBrowserHistory,
            compat::UiAction::ToggleFindSimilarFocusedContent => {
                Self::ToggleFindSimilarFocusedSample
            }
            compat::UiAction::ToggleBrowserDuplicateCleanupKeep { visible_row } => {
                Self::ToggleBrowserDuplicateCleanupKeep { visible_row }
            }
            compat::UiAction::ConfirmBrowserDuplicateCleanup => {
                Self::ConfirmBrowserDuplicateCleanup
            }
            compat::UiAction::PlayRandomContentItem => Self::PlayRandomSample,
            compat::UiAction::PlayPreviousRandomContentItem => Self::PlayPreviousRandomSample,
            compat::UiAction::AdjustSelectedBrowserRating { delta } => {
                Self::AdjustSelectedBrowserRating { delta }
            }
            compat::UiAction::SetBrowserTab { map } => Self::SetBrowserTab { map },
            compat::UiAction::FocusBrowserPillEditorInput => Self::FocusBrowserTagSidebarInput,
            compat::UiAction::SetBrowserPillEditorInput { value } => {
                Self::SetBrowserTagSidebarInput { value }
            }
            compat::UiAction::CommitBrowserPillEditorInput => Self::CommitBrowserTagSidebarInput,
            compat::UiAction::SetBrowserSidebarLooped { looped } => {
                Self::SetBrowserSidebarLooped { looped }
            }
            compat::UiAction::ToggleBrowserPillOption { label } => {
                Self::ToggleBrowserSidebarNormalTag { label }
            }
            compat::UiAction::FocusSpatialContentItem { content_id } => Self::FocusMapSample {
                sample_id: content_id,
            },
            compat::UiAction::SetPromptInput { value } => Self::SetPromptInput { value },
            compat::UiAction::StartBrowserRename => Self::StartBrowserRename,
            compat::UiAction::ConfirmBrowserRename => Self::ConfirmBrowserRename,
            compat::UiAction::CancelBrowserRename => Self::CancelBrowserRename,
            compat::UiAction::AutoRenameBrowserSelection { visible_row } => {
                Self::AutoRenameBrowserSelection { visible_row }
            }
            compat::UiAction::SetBrowserTriageMark { target } => Self::TagBrowserSelection {
                target: target.into(),
            },
            compat::UiAction::DeleteBrowserSelection => Self::DeleteBrowserSelection,
            compat::UiAction::NormalizeFocusedContentItem => Self::NormalizeFocusedBrowserSample,
            compat::UiAction::NormalizeWaveformSelectionOrLoadedContent => {
                Self::NormalizeWaveformSelectionOrSample
            }
            compat::UiAction::CropWaveformSelection => Self::CropWaveformSelection,
            compat::UiAction::CropWaveformSelectionToNewContentItem => {
                Self::CropWaveformSelectionToNewSample
            }
            compat::UiAction::TrimWaveformSelection => Self::TrimWaveformSelection,
            compat::UiAction::ReverseWaveformSelection => Self::ReverseWaveformSelection,
            compat::UiAction::FadeWaveformSelectionLeftToRight => {
                Self::FadeWaveformSelectionLeftToRight
            }
            compat::UiAction::FadeWaveformSelectionRightToLeft => {
                Self::FadeWaveformSelectionRightToLeft
            }
            compat::UiAction::MuteWaveformSelection => Self::MuteWaveformSelection,
            compat::UiAction::DeleteSelectedSliceMarkers => Self::DeleteSelectedSliceMarkers,
            compat::UiAction::ToggleWaveformSliceSelection { index } => {
                Self::ToggleWaveformSliceSelection { index }
            }
            compat::UiAction::AuditionWaveformDuplicateSlice { index } => {
                Self::AuditionWaveformDuplicateSlice { index }
            }
            compat::UiAction::ToggleWaveformDuplicateSliceExemption { index } => {
                Self::ToggleWaveformDuplicateSliceExemption { index }
            }
            compat::UiAction::MoveWaveformSliceFocus { delta } => {
                Self::MoveWaveformSliceFocus { delta }
            }
            compat::UiAction::ToggleFocusedWaveformSliceExportMark => {
                Self::ToggleFocusedWaveformSliceExportMark
            }
            compat::UiAction::AlignWaveformStartToMarker => Self::AlignWaveformStartToMarker,
            compat::UiAction::DeleteLoadedWaveformContent => Self::DeleteLoadedWaveformSample,
            compat::UiAction::SlideWaveformSelection { delta, fine } => {
                Self::SlideWaveformSelection { delta, fine }
            }
            compat::UiAction::ConfirmPrompt => Self::ConfirmPrompt,
            compat::UiAction::CancelPrompt => Self::CancelPrompt,
            compat::UiAction::CancelProgress => Self::CancelProgress,
            compat::UiAction::CopySelectionToClipboard => Self::CopySelectionToClipboard,
            compat::UiAction::ToggleHotkeyOverlay => Self::ToggleHotkeyOverlay,
            compat::UiAction::CopyStatusLog => Self::CopyStatusLog,
            compat::UiAction::OpenFeedbackIssuePrompt => Self::OpenFeedbackIssuePrompt,
            compat::UiAction::MoveDiscardedItemsToFolder => Self::MoveTrashedSamplesToFolder,
            compat::UiAction::SetInputMonitoringEnabled { enabled } => {
                Self::SetInputMonitoringEnabled { enabled }
            }
            compat::UiAction::SetAdvanceAfterRatingEnabled { enabled } => {
                Self::SetAdvanceAfterRatingEnabled { enabled }
            }
            compat::UiAction::SetDestructiveYoloMode { enabled } => {
                Self::SetDestructiveYoloMode { enabled }
            }
            compat::UiAction::SetInvertWaveformScroll { enabled } => {
                Self::SetInvertWaveformScroll { enabled }
            }
            compat::UiAction::ToggleLoopPlayback => Self::ToggleLoopPlayback,
            compat::UiAction::ToggleLoopLock => Self::ToggleLoopLock,
            compat::UiAction::SetWaveformChannelView { stereo } => {
                Self::SetWaveformChannelView { stereo }
            }
            compat::UiAction::SetNormalizedAuditionEnabled { enabled } => {
                Self::SetNormalizedAuditionEnabled { enabled }
            }
            compat::UiAction::SetBpmSnapEnabled { enabled } => Self::SetBpmSnapEnabled { enabled },
            compat::UiAction::SetRelativeBpmGridEnabled { enabled } => {
                Self::SetRelativeBpmGridEnabled { enabled }
            }
            compat::UiAction::AdjustWaveformBpm { delta } => Self::AdjustWaveformBpm { delta },
            compat::UiAction::SetWaveformBpmValue { value_tenths } => {
                Self::SetWaveformBpmValue { value_tenths }
            }
            compat::UiAction::SetTransientSnapEnabled { enabled } => {
                Self::SetTransientSnapEnabled { enabled }
            }
            compat::UiAction::SetTransientMarkersEnabled { enabled } => {
                Self::SetTransientMarkersEnabled { enabled }
            }
            compat::UiAction::ToggleTransientMarkers => Self::ToggleTransientMarkers,
            compat::UiAction::ToggleBpmSnap => Self::ToggleBpmSnap,
            compat::UiAction::SetSliceModeEnabled { enabled } => {
                Self::SetSliceModeEnabled { enabled }
            }
            compat::UiAction::SetVolume { value_milli } => Self::SetVolume { value_milli },
            compat::UiAction::CommitVolumeSetting => Self::CommitVolumeSetting,
            compat::UiAction::SeekWaveformPrecise { position_nanos } => {
                Self::SeekWaveformPrecise { position_nanos }
            }
            compat::UiAction::SetWaveformCursorPrecise { position_nanos } => {
                Self::SetWaveformCursorPrecise { position_nanos }
            }
            compat::UiAction::SeekWaveform { position_milli } => {
                Self::SeekWaveform { position_milli }
            }
            compat::UiAction::SetWaveformCursor { position_milli } => {
                Self::SetWaveformCursor { position_milli }
            }
            compat::UiAction::BeginWaveformSelectionAt { anchor_micros } => {
                Self::BeginWaveformSelectionAt { anchor_micros }
            }
            compat::UiAction::BeginWaveformSelectionAtPrecise { anchor_nanos } => {
                Self::BeginWaveformSelectionAtPrecise { anchor_nanos }
            }
            compat::UiAction::BeginWaveformCircularSlide { anchor_micros } => {
                Self::BeginWaveformCircularSlide { anchor_micros }
            }
            compat::UiAction::UpdateWaveformCircularSlide { position_micros } => {
                Self::UpdateWaveformCircularSlide { position_micros }
            }
            compat::UiAction::FinishWaveformCircularSlide => Self::FinishWaveformCircularSlide,
            compat::UiAction::SetWaveformSelectionRange {
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
            compat::UiAction::SetWaveformSelectionRangePrecise {
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
            compat::UiAction::SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            } => Self::SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            },
            compat::UiAction::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos,
                end_nanos,
            } => Self::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos,
                end_nanos,
            },
            compat::UiAction::SetWaveformEditSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge,
            } => Self::SetWaveformEditSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge,
            },
            compat::UiAction::SetWaveformEditSelectionRangePrecise {
                start_nanos,
                end_nanos,
                preserve_view_edge,
            } => Self::SetWaveformEditSelectionRangePrecise {
                start_nanos,
                end_nanos,
                preserve_view_edge,
            },
            compat::UiAction::SetWaveformEditFadeInEnd { position_micros } => {
                Self::SetWaveformEditFadeInEnd { position_micros }
            }
            compat::UiAction::SetWaveformEditFadeInMuteStart { position_micros } => {
                Self::SetWaveformEditFadeInMuteStart { position_micros }
            }
            compat::UiAction::SetWaveformEditFadeInCurve { curve_milli } => {
                Self::SetWaveformEditFadeInCurve { curve_milli }
            }
            compat::UiAction::SetWaveformEditFadeOutStart { position_micros } => {
                Self::SetWaveformEditFadeOutStart { position_micros }
            }
            compat::UiAction::SetWaveformEditFadeOutMuteEnd { position_micros } => {
                Self::SetWaveformEditFadeOutMuteEnd { position_micros }
            }
            compat::UiAction::SetWaveformEditFadeOutCurve { curve_milli } => {
                Self::SetWaveformEditFadeOutCurve { curve_milli }
            }
            compat::UiAction::FinishWaveformEditFadeDrag => Self::FinishWaveformEditFadeDrag,
            compat::UiAction::StartWaveformSelectionDrag {
                pointer_x,
                pointer_y,
            } => Self::StartWaveformSelectionDrag {
                pointer_x,
                pointer_y,
            },
            compat::UiAction::UpdateWaveformSelectionDrag {
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
            compat::UiAction::FinishWaveformSelectionDrag => Self::FinishWaveformSelectionDrag,
            compat::UiAction::FinishWaveformSelectionRangeDrag => {
                Self::FinishWaveformSelectionRangeDrag
            }
            compat::UiAction::FinishWaveformSelectionSmartScaleDrag => {
                Self::FinishWaveformSelectionSmartScaleDrag
            }
            compat::UiAction::BeginWaveformSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            } => Self::BeginWaveformSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            },
            compat::UiAction::BeginWaveformSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            } => Self::BeginWaveformSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            },
            compat::UiAction::BeginWaveformEditSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            } => Self::BeginWaveformEditSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            },
            compat::UiAction::BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            } => Self::BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            },
            compat::UiAction::FinishWaveformEditSelectionDrag => {
                Self::FinishWaveformEditSelectionDrag
            }
            compat::UiAction::ClearWaveformSelection => Self::ClearWaveformSelection,
            compat::UiAction::ClearWaveformEditSelection => Self::ClearWaveformEditSelection,
            compat::UiAction::ClearWaveformSelections => Self::ClearWaveformSelections,
            compat::UiAction::SetWaveformViewCenter {
                center_micros,
                center_nanos,
            } => Self::SetWaveformViewCenter {
                center_micros,
                center_nanos,
            },
            compat::UiAction::ZoomWaveform {
                zoom_in,
                steps,
                anchor_ratio_micros,
            } => Self::ZoomWaveform {
                zoom_in,
                steps,
                anchor_ratio_micros,
            },
            compat::UiAction::ZoomWaveformToSelection => Self::ZoomWaveformToSelection,
            compat::UiAction::ZoomWaveformFull => Self::ZoomWaveformFull,
            compat::UiAction::Undo => Self::Undo,
            compat::UiAction::Redo => Self::Redo,
            compat::UiAction::CheckForUpdates => Self::CheckForUpdates,
            compat::UiAction::OpenUpdateLink => Self::OpenUpdateLink,
            compat::UiAction::InstallUpdate => Self::InstallUpdate,
            compat::UiAction::DismissUpdate => Self::DismissUpdate,
        }
    }
}

impl From<UiAction> for compat::UiAction {
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
