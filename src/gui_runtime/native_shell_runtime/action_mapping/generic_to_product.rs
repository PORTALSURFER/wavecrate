use super::super::*;

impl From<runtime_contract::UiAction> for UiAction {
    fn from(value: runtime_contract::UiAction) -> Self {
        match value {
            runtime_contract::UiAction::SelectColumn { index } => Self::SelectColumn { index },
            runtime_contract::UiAction::MoveColumn { delta } => Self::MoveColumn { delta },
            runtime_contract::UiAction::ToggleTransport => Self::ToggleTransport,
            runtime_contract::UiAction::PlayCompareAnchor => Self::PlayCompareAnchor,
            runtime_contract::UiAction::PlayFromStart => Self::PlayFromStart,
            runtime_contract::UiAction::PlayFromCurrentPlayhead => Self::PlayFromCurrentPlayhead,
            runtime_contract::UiAction::PlayFromWaveformCursor => Self::PlayFromWaveformCursor,
            runtime_contract::UiAction::PlayWaveformAtPrecise { position_nanos } => {
                Self::PlayWaveformAtPrecise { position_nanos }
            }
            runtime_contract::UiAction::HandleEscape => Self::HandleEscape,
            runtime_contract::UiAction::FocusBrowserPanel => Self::FocusBrowserPanel,
            runtime_contract::UiAction::FocusSourcesPanel => Self::FocusSourcesPanel,
            runtime_contract::UiAction::FocusWaveformPanel => Self::FocusWaveformPanel,
            runtime_contract::UiAction::FocusFolderPanel => Self::FocusFolderPanel,
            runtime_contract::UiAction::FocusLoadedContentInList => {
                Self::FocusLoadedSampleInBrowser
            }
            runtime_contract::UiAction::FocusBrowserSearch => Self::FocusBrowserSearch,
            runtime_contract::UiAction::BlurBrowserSearch => Self::BlurBrowserSearch,
            runtime_contract::UiAction::OpenAddSourceDialog => Self::OpenAddSourceDialog,
            runtime_contract::UiAction::OpenOptionsMenu => Self::OpenOptionsMenu,
            runtime_contract::UiAction::CloseOptionsPanel => Self::CloseOptionsPanel,
            runtime_contract::UiAction::PickTrashFolder => Self::PickTrashFolder,
            runtime_contract::UiAction::OpenTrashFolder => Self::OpenTrashFolder,
            runtime_contract::UiAction::EditDefaultIdentifier => Self::EditDefaultIdentifier,
            runtime_contract::UiAction::ShowOptionsOverview => Self::ShowOptionsOverview,
            runtime_contract::UiAction::OpenPrimaryGroupPicker => Self::OpenAudioOutputHostPicker,
            runtime_contract::UiAction::OpenPrimaryItemPicker => Self::OpenAudioOutputDevicePicker,
            runtime_contract::UiAction::OpenPrimaryNumberPicker => {
                Self::OpenAudioOutputSampleRatePicker
            }
            runtime_contract::UiAction::OpenSecondaryGroupPicker => Self::OpenAudioInputHostPicker,
            runtime_contract::UiAction::OpenSecondaryItemPicker => Self::OpenAudioInputDevicePicker,
            runtime_contract::UiAction::OpenSecondaryNumberPicker => {
                Self::OpenAudioInputSampleRatePicker
            }
            runtime_contract::UiAction::SetPrimaryGroup { group_id } => {
                Self::SetAudioOutputHost { host_id: group_id }
            }
            runtime_contract::UiAction::SetPrimaryItem { item_name } => {
                Self::SetAudioOutputDevice {
                    device_name: item_name,
                }
            }
            runtime_contract::UiAction::SetPrimaryNumber { value } => {
                Self::SetAudioOutputSampleRate { sample_rate: value }
            }
            runtime_contract::UiAction::SetSecondaryGroup { group_id } => {
                Self::SetAudioInputHost { host_id: group_id }
            }
            runtime_contract::UiAction::SetSecondaryItem { item_name } => {
                Self::SetAudioInputDevice {
                    device_name: item_name,
                }
            }
            runtime_contract::UiAction::SetSecondaryNumber { value } => {
                Self::SetAudioInputSampleRate { sample_rate: value }
            }
            runtime_contract::UiAction::FocusFolderSearch => Self::FocusFolderSearch,
            runtime_contract::UiAction::SetFolderSearch { query } => {
                Self::SetFolderSearch { query }
            }
            runtime_contract::UiAction::ToggleShowAllFolders => Self::ToggleShowAllFolders,
            runtime_contract::UiAction::ToggleFolderFlattenedView => {
                Self::ToggleFolderFlattenedView
            }
            runtime_contract::UiAction::FocusSourceRow { index } => Self::FocusSourceRow { index },
            runtime_contract::UiAction::SelectSourceRow { index } => {
                Self::SelectSourceRow { index }
            }
            runtime_contract::UiAction::MoveSourceFocus { delta } => {
                Self::MoveSourceFocus { delta }
            }
            runtime_contract::UiAction::ReloadFocusedSourceRow => Self::ReloadFocusedSourceRow,
            runtime_contract::UiAction::HardSyncFocusedSourceRow => Self::HardSyncFocusedSourceRow,
            runtime_contract::UiAction::OpenFocusedSourceFolder => Self::OpenFocusedSourceFolder,
            runtime_contract::UiAction::RemoveFocusedSourceRow => Self::RemoveFocusedSourceRow,
            runtime_contract::UiAction::ReloadSourceRow { index } => {
                Self::ReloadSourceRow { index }
            }
            runtime_contract::UiAction::HardSyncSourceRow { index } => {
                Self::HardSyncSourceRow { index }
            }
            runtime_contract::UiAction::OpenSourceFolderRow { index } => {
                Self::OpenSourceFolderRow { index }
            }
            runtime_contract::UiAction::RemoveSourceRow { index } => {
                Self::RemoveSourceRow { index }
            }
            runtime_contract::UiAction::FocusFolderRow { index } => Self::FocusFolderRow { index },
            runtime_contract::UiAction::ActivateFolderRow { index } => {
                Self::ActivateFolderRow { index }
            }
            runtime_contract::UiAction::ToggleFolderRowExpanded { index } => {
                Self::ToggleFolderRowExpanded { index }
            }
            runtime_contract::UiAction::ExpandFocusedFolder => Self::ExpandFocusedFolder,
            runtime_contract::UiAction::CollapseFocusedFolder => Self::CollapseFocusedFolder,
            runtime_contract::UiAction::ToggleFocusedFolderSelection => {
                Self::ToggleFocusedFolderSelection
            }
            runtime_contract::UiAction::MoveFolderFocus { delta } => {
                Self::MoveFolderFocus { delta }
            }
            runtime_contract::UiAction::StartNewFolder => Self::StartNewFolder,
            runtime_contract::UiAction::StartNewFolderAtFolderRow { index } => {
                Self::StartNewFolderAtFolderRow { index }
            }
            runtime_contract::UiAction::StartNewFolderAtRoot => Self::StartNewFolderAtRoot,
            runtime_contract::UiAction::FocusFolderCreateInput => Self::FocusFolderCreateInput,
            runtime_contract::UiAction::SetFolderCreateInput { value } => {
                Self::SetFolderCreateInput { value }
            }
            runtime_contract::UiAction::ConfirmFolderCreate => Self::ConfirmFolderCreate,
            runtime_contract::UiAction::CancelFolderCreate => Self::CancelFolderCreate,
            runtime_contract::UiAction::StartFolderRename => Self::StartFolderRename,
            runtime_contract::UiAction::DeleteFocusedFolder => Self::DeleteFocusedFolder,
            runtime_contract::UiAction::RestoreRetainedFolderDeletes => {
                Self::RestoreRetainedFolderDeletes
            }
            runtime_contract::UiAction::PurgeRetainedFolderDeletes => {
                Self::PurgeRetainedFolderDeletes
            }
            runtime_contract::UiAction::ClearFolderDeleteRecoveryLog => {
                Self::ClearFolderDeleteRecoveryLog
            }
            runtime_contract::UiAction::MoveBrowserFocus { delta } => {
                Self::MoveBrowserFocus { delta }
            }
            runtime_contract::UiAction::SetBrowserViewStart { visible_row } => {
                Self::SetBrowserViewStart { visible_row }
            }
            runtime_contract::UiAction::FocusBrowserRow { visible_row } => {
                Self::FocusBrowserRow { visible_row }
            }
            runtime_contract::UiAction::SetCompareAnchorFromFocusedContent => {
                Self::SetCompareAnchorFromFocusedBrowserSample
            }
            runtime_contract::UiAction::CommitFocusedBrowserRow => Self::CommitFocusedBrowserRow,
            runtime_contract::UiAction::SaveWaveformSelectionToBrowser => {
                Self::SaveWaveformSelectionToBrowser
            }
            runtime_contract::UiAction::SaveWaveformSelectionToBrowserWithKeep2 => {
                Self::SaveWaveformSelectionToBrowserWithKeep2
            }
            runtime_contract::UiAction::CommitWaveformEditFades => Self::CommitWaveformEditFades,
            runtime_contract::UiAction::DetectWaveformSilenceSlices => {
                Self::DetectWaveformSilenceSlices
            }
            runtime_contract::UiAction::DetectWaveformExactDuplicateSlices => {
                Self::DetectWaveformExactDuplicateSlices
            }
            runtime_contract::UiAction::CleanWaveformExactDuplicateSlices => {
                Self::CleanWaveformExactDuplicateSlices
            }
            runtime_contract::UiAction::ToggleBrowserRowSelection { visible_row } => {
                Self::ToggleBrowserRowSelection { visible_row }
            }
            runtime_contract::UiAction::StartContentItemDrag {
                visible_row,
                pointer_x,
                pointer_y,
            } => Self::StartBrowserSampleDrag {
                visible_row,
                pointer_x,
                pointer_y,
            },
            runtime_contract::UiAction::UpdateContentItemDrag {
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
            runtime_contract::UiAction::FinishContentItemDrag => Self::FinishBrowserSampleDrag,
            runtime_contract::UiAction::ExtendBrowserSelectionToRow { visible_row } => {
                Self::ExtendBrowserSelectionToRow { visible_row }
            }
            runtime_contract::UiAction::AddRangeBrowserSelection { visible_row } => {
                Self::AddRangeBrowserSelection { visible_row }
            }
            runtime_contract::UiAction::ExtendBrowserSelectionFromFocus { delta } => {
                Self::ExtendBrowserSelectionFromFocus { delta }
            }
            runtime_contract::UiAction::AddRangeBrowserSelectionFromFocus { delta } => {
                Self::AddRangeBrowserSelectionFromFocus { delta }
            }
            runtime_contract::UiAction::ToggleFocusedBrowserRowSelection => {
                Self::ToggleFocusedBrowserRowSelection
            }
            runtime_contract::UiAction::SelectAllBrowserRows => Self::SelectAllBrowserRows,
            runtime_contract::UiAction::SetBrowserSearch { query } => {
                Self::SetBrowserSearch { query }
            }
            runtime_contract::UiAction::ToggleBrowserRatingFilter { level, invert } => {
                Self::ToggleBrowserRatingFilter { level, invert }
            }
            runtime_contract::UiAction::ToggleBrowserPlaybackAgeFilter { bucket, invert } => {
                Self::ToggleBrowserPlaybackAgeFilter { bucket, invert }
            }
            runtime_contract::UiAction::ToggleBrowserSidebarFilter { option, additive } => {
                Self::ToggleBrowserSidebarFilter { option, additive }
            }
            runtime_contract::UiAction::ClearBrowserSidebarFilter { facet } => {
                Self::ClearBrowserSidebarFilter { facet }
            }
            runtime_contract::UiAction::ToggleContentMark => Self::ToggleBrowserSampleMark,
            runtime_contract::UiAction::ToggleBrowserMarkedFilter => {
                Self::ToggleBrowserMarkedFilter
            }
            runtime_contract::UiAction::ToggleBrowserDerivedLabelFilter { invert } => {
                Self::ToggleBrowserTagNamedFilter { invert }
            }
            runtime_contract::UiAction::ToggleRandomNavigationMode => {
                Self::ToggleRandomNavigationMode
            }
            runtime_contract::UiAction::ToggleBrowserPillEditor => Self::ToggleBrowserTagSidebar,
            runtime_contract::UiAction::ToggleBrowserPillEditorPrimaryAction => {
                Self::ToggleBrowserTagSidebarAutoRename
            }
            runtime_contract::UiAction::ToggleBrowserDuplicateCleanupMode => {
                Self::ToggleBrowserDuplicateCleanupMode
            }
            runtime_contract::UiAction::FocusPreviousBrowserHistory => {
                Self::FocusPreviousBrowserHistory
            }
            runtime_contract::UiAction::FocusNextBrowserHistory => Self::FocusNextBrowserHistory,
            runtime_contract::UiAction::ToggleFindSimilarFocusedContent => {
                Self::ToggleFindSimilarFocusedSample
            }
            runtime_contract::UiAction::ToggleBrowserDuplicateCleanupKeep { visible_row } => {
                Self::ToggleBrowserDuplicateCleanupKeep { visible_row }
            }
            runtime_contract::UiAction::ConfirmBrowserDuplicateCleanup => {
                Self::ConfirmBrowserDuplicateCleanup
            }
            runtime_contract::UiAction::PlayRandomContentItem => Self::PlayRandomSample,
            runtime_contract::UiAction::PlayPreviousRandomContentItem => {
                Self::PlayPreviousRandomSample
            }
            runtime_contract::UiAction::AdjustSelectedBrowserRating { delta } => {
                Self::AdjustSelectedBrowserRating { delta }
            }
            runtime_contract::UiAction::SetBrowserTab { map } => Self::SetBrowserTab { map },
            runtime_contract::UiAction::FocusBrowserPillEditorInput => {
                Self::FocusBrowserTagSidebarInput
            }
            runtime_contract::UiAction::SetBrowserPillEditorInput { value } => {
                Self::SetBrowserTagSidebarInput { value }
            }
            runtime_contract::UiAction::CommitBrowserPillEditorInput => {
                Self::CommitBrowserTagSidebarInput
            }
            runtime_contract::UiAction::SetBrowserSidebarLooped { looped } => {
                Self::SetBrowserSidebarLooped { looped }
            }
            runtime_contract::UiAction::ToggleBrowserPillOption { label } => {
                Self::ToggleBrowserSidebarNormalTag { label }
            }
            runtime_contract::UiAction::FocusSpatialContentItem { content_id } => {
                Self::FocusMapSample {
                    sample_id: content_id,
                }
            }
            runtime_contract::UiAction::SetPromptInput { value } => Self::SetPromptInput { value },
            runtime_contract::UiAction::StartBrowserRename => Self::StartBrowserRename,
            runtime_contract::UiAction::ConfirmBrowserRename => Self::ConfirmBrowserRename,
            runtime_contract::UiAction::CancelBrowserRename => Self::CancelBrowserRename,
            runtime_contract::UiAction::AutoRenameBrowserSelection { visible_row } => {
                Self::AutoRenameBrowserSelection { visible_row }
            }
            runtime_contract::UiAction::SetBrowserTriageMark { target } => {
                Self::TagBrowserSelection {
                    target: target.into(),
                }
            }
            runtime_contract::UiAction::DeleteBrowserSelection => Self::DeleteBrowserSelection,
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
        }
    }
}
