use super::super::{
    NativeBrowserAction, NativeCompatibilityAction, NativeHistoryUpdateAction, NativeOptionsAction,
    NativePromptEditAction, NativeShellAction, NativeSourcesFoldersAction, NativeTransportAction,
    NativeUiAction, NativeWaveformAction,
};
use super::data::gui_action_rows;
use super::{GuiActionKind, GuiActionKind as Kind};

macro_rules! build_representative_action_mapping {
    ($($kind:ident $pattern:tt => {
        id: $id:literal, surface: $surface:ident, effect: $effect:ident,
        coverage: [$($coverage:ident),+ $(,)?],
        fixtures: [$($fixture:literal),* $(,)?], sample: $sample:expr
    }),+ $(,)?) => {
        /// Return a representative action payload for the provided kind.
        pub fn representative_action_for_kind(kind: GuiActionKind) -> NativeUiAction {
            match kind {
                $(Kind::$kind => $sample,)+
            }
        }
    };
}

/// Return the payload-free kind for one concrete UI action.
pub fn action_kind(action: &NativeUiAction) -> GuiActionKind {
    match action {
        NativeUiAction::Transport(action) => transport_action_kind(action),
        NativeUiAction::HistoryAndUpdate(action) => history_update_action_kind(action),
        NativeUiAction::Shell(action) => shell_action_kind(action),
        NativeUiAction::SourcesAndFolders(action) => sources_folders_action_kind(action),
        NativeUiAction::Browser(action) => browser_action_kind(action),
        NativeUiAction::PromptsAndEdits(action) => prompt_edit_action_kind(action),
        NativeUiAction::Options(action) => options_action_kind(action),
        NativeUiAction::Waveform(action) => waveform_action_kind(action),
        NativeUiAction::Compatibility(action) => compatibility_action_kind(action),
    }
}

fn shell_action_kind(action: &NativeShellAction) -> GuiActionKind {
    match action {
        NativeShellAction::FocusBrowserPanel => Kind::FocusBrowserPanel,
        NativeShellAction::FocusSourcesPanel => Kind::FocusSourcesPanel,
        NativeShellAction::FocusWaveformPanel => Kind::FocusWaveformPanel,
        NativeShellAction::FocusFolderPanel => Kind::FocusFolderPanel,
        NativeShellAction::FocusLoadedSampleInBrowser => Kind::FocusLoadedSampleInBrowser,
        NativeShellAction::FocusBrowserSearch => Kind::FocusBrowserSearch,
        NativeShellAction::BlurBrowserSearch => Kind::BlurBrowserSearch,
        NativeShellAction::OpenAddSourceDialog => Kind::OpenAddSourceDialog,
        NativeShellAction::FocusFolderSearch => Kind::FocusFolderSearch,
        NativeShellAction::SetFolderSearch { .. } => Kind::SetFolderSearch,
        NativeShellAction::ToggleShowAllFolders => Kind::ToggleShowAllFolders,
        NativeShellAction::ToggleFolderFlattenedView => Kind::ToggleFolderFlattenedView,
    }
}

fn sources_folders_action_kind(action: &NativeSourcesFoldersAction) -> GuiActionKind {
    match action {
        NativeSourcesFoldersAction::FocusSourceRow { .. } => Kind::FocusSourceRow,
        NativeSourcesFoldersAction::SelectSourceRow { .. } => Kind::SelectSourceRow,
        NativeSourcesFoldersAction::MoveSourceFocus { .. } => Kind::MoveSourceFocus,
        NativeSourcesFoldersAction::ReloadFocusedSourceRow => Kind::ReloadFocusedSourceRow,
        NativeSourcesFoldersAction::HardSyncFocusedSourceRow => Kind::HardSyncFocusedSourceRow,
        NativeSourcesFoldersAction::OpenFocusedSourceFolder => Kind::OpenFocusedSourceFolder,
        NativeSourcesFoldersAction::RemoveFocusedSourceRow => Kind::RemoveFocusedSourceRow,
        NativeSourcesFoldersAction::ReloadSourceRow { .. } => Kind::ReloadSourceRow,
        NativeSourcesFoldersAction::HardSyncSourceRow { .. } => Kind::HardSyncSourceRow,
        NativeSourcesFoldersAction::OpenSourceFolderRow { .. } => Kind::OpenSourceFolderRow,
        NativeSourcesFoldersAction::RemoveSourceRow { .. } => Kind::RemoveSourceRow,
        NativeSourcesFoldersAction::FocusFolderRow { .. } => Kind::FocusFolderRow,
        NativeSourcesFoldersAction::ActivateFolderRow { .. } => Kind::ActivateFolderRow,
        NativeSourcesFoldersAction::ToggleFolderRowExpanded { .. } => Kind::ToggleFolderRowExpanded,
        NativeSourcesFoldersAction::ExpandFocusedFolder => Kind::ExpandFocusedFolder,
        NativeSourcesFoldersAction::CollapseFocusedFolder => Kind::CollapseFocusedFolder,
        NativeSourcesFoldersAction::ToggleFocusedFolderSelection => {
            Kind::ToggleFocusedFolderSelection
        }
        NativeSourcesFoldersAction::MoveFolderFocus { .. } => Kind::MoveFolderFocus,
        NativeSourcesFoldersAction::StartNewFolder => Kind::StartNewFolder,
        NativeSourcesFoldersAction::StartNewFolderAtFolderRow { .. } => {
            Kind::StartNewFolderAtFolderRow
        }
        NativeSourcesFoldersAction::StartNewFolderAtRoot => Kind::StartNewFolderAtRoot,
        NativeSourcesFoldersAction::FocusFolderCreateInput => Kind::FocusFolderCreateInput,
        NativeSourcesFoldersAction::SetFolderCreateInput { .. } => Kind::SetFolderCreateInput,
        NativeSourcesFoldersAction::ConfirmFolderCreate => Kind::ConfirmFolderCreate,
        NativeSourcesFoldersAction::CancelFolderCreate => Kind::CancelFolderCreate,
        NativeSourcesFoldersAction::StartFolderRename => Kind::StartFolderRename,
        NativeSourcesFoldersAction::DeleteFocusedFolder => Kind::DeleteFocusedFolder,
        NativeSourcesFoldersAction::RestoreRetainedFolderDeletes => {
            Kind::RestoreRetainedFolderDeletes
        }
        NativeSourcesFoldersAction::PurgeRetainedFolderDeletes => Kind::PurgeRetainedFolderDeletes,
        NativeSourcesFoldersAction::ClearFolderDeleteRecoveryLog => {
            Kind::ClearFolderDeleteRecoveryLog
        }
    }
}

fn browser_action_kind(action: &NativeBrowserAction) -> GuiActionKind {
    match action {
        NativeBrowserAction::MoveBrowserFocus { .. } => Kind::MoveBrowserFocus,
        NativeBrowserAction::SetBrowserViewStart { .. } => Kind::SetBrowserViewStart,
        NativeBrowserAction::FocusBrowserRow { .. } => Kind::FocusBrowserRow,
        NativeBrowserAction::SetCompareAnchorFromFocusedBrowserSample => {
            Kind::SetCompareAnchorFromFocusedBrowserSample
        }
        NativeBrowserAction::CommitFocusedBrowserRow => Kind::CommitFocusedBrowserRow,
        NativeBrowserAction::SaveWaveformSelectionToBrowser => Kind::SaveWaveformSelectionToBrowser,
        NativeBrowserAction::SaveWaveformSelectionToBrowserWithKeep2 => {
            Kind::SaveWaveformSelectionToBrowserWithKeep2
        }
        NativeBrowserAction::CommitWaveformEditFades => Kind::CommitWaveformEditFades,
        NativeBrowserAction::DetectWaveformSilenceSlices => Kind::DetectWaveformSilenceSlices,
        NativeBrowserAction::DetectWaveformExactDuplicateSlices => {
            Kind::DetectWaveformExactDuplicateSlices
        }
        NativeBrowserAction::CleanWaveformExactDuplicateSlices => {
            Kind::CleanWaveformExactDuplicateSlices
        }
        NativeBrowserAction::ToggleBrowserRowSelection { .. } => Kind::ToggleBrowserRowSelection,
        NativeBrowserAction::StartBrowserSampleDrag { .. } => Kind::StartBrowserSampleDrag,
        NativeBrowserAction::UpdateBrowserSampleDrag { .. } => Kind::UpdateBrowserSampleDrag,
        NativeBrowserAction::FinishBrowserSampleDrag => Kind::FinishBrowserSampleDrag,
        NativeBrowserAction::ExtendBrowserSelectionToRow { .. } => {
            Kind::ExtendBrowserSelectionToRow
        }
        NativeBrowserAction::AddRangeBrowserSelection { .. } => Kind::AddRangeBrowserSelection,
        NativeBrowserAction::ExtendBrowserSelectionFromFocus { .. } => {
            Kind::ExtendBrowserSelectionFromFocus
        }
        NativeBrowserAction::AddRangeBrowserSelectionFromFocus { .. } => {
            Kind::AddRangeBrowserSelectionFromFocus
        }
        NativeBrowserAction::ToggleFocusedBrowserRowSelection => {
            Kind::ToggleFocusedBrowserRowSelection
        }
        NativeBrowserAction::SelectAllBrowserRows => Kind::SelectAllBrowserRows,
        NativeBrowserAction::SetBrowserSearch { .. } => Kind::SetBrowserSearch,
        NativeBrowserAction::ToggleBrowserRatingFilter { .. } => Kind::ToggleBrowserRatingFilter,
        NativeBrowserAction::ToggleBrowserPlaybackAgeFilter { .. } => {
            Kind::ToggleBrowserPlaybackAgeFilter
        }
        NativeBrowserAction::ToggleBrowserSidebarFilter { .. } => Kind::ToggleBrowserSidebarFilter,
        NativeBrowserAction::ClearBrowserSidebarFilter { .. } => Kind::ClearBrowserSidebarFilter,
        NativeBrowserAction::ToggleBrowserSampleMark => Kind::ToggleBrowserSampleMark,
        NativeBrowserAction::ToggleBrowserMarkedFilter => Kind::ToggleBrowserMarkedFilter,
        NativeBrowserAction::ToggleBrowserTagNamedFilter { .. } => {
            Kind::ToggleBrowserTagNamedFilter
        }
        NativeBrowserAction::ToggleRandomNavigationMode => Kind::ToggleRandomNavigationMode,
        NativeBrowserAction::ToggleBrowserTagSidebar => Kind::ToggleBrowserTagSidebar,
        NativeBrowserAction::ToggleBrowserTagSidebarAutoRename => {
            Kind::ToggleBrowserTagSidebarAutoRename
        }
        NativeBrowserAction::ToggleBrowserDuplicateCleanupMode => {
            Kind::ToggleBrowserDuplicateCleanupMode
        }
        NativeBrowserAction::FocusPreviousBrowserHistory => Kind::FocusPreviousBrowserHistory,
        NativeBrowserAction::FocusNextBrowserHistory => Kind::FocusNextBrowserHistory,
        NativeBrowserAction::ToggleFindSimilarFocusedSample => Kind::ToggleFindSimilarFocusedSample,
        NativeBrowserAction::ToggleBrowserDuplicateCleanupKeep { .. } => {
            Kind::ToggleBrowserDuplicateCleanupKeep
        }
        NativeBrowserAction::ConfirmBrowserDuplicateCleanup => Kind::ConfirmBrowserDuplicateCleanup,
        NativeBrowserAction::PlayRandomSample => Kind::PlayRandomSample,
        NativeBrowserAction::PlayPreviousRandomSample => Kind::PlayPreviousRandomSample,
        NativeBrowserAction::AdjustSelectedBrowserRating { .. } => {
            Kind::AdjustSelectedBrowserRating
        }
        NativeBrowserAction::SetBrowserTab { .. } => Kind::SetBrowserTab,
        NativeBrowserAction::FocusBrowserTagSidebarInput => Kind::FocusBrowserTagSidebarInput,
        NativeBrowserAction::SetBrowserTagSidebarInput { .. } => Kind::SetBrowserTagSidebarInput,
        NativeBrowserAction::CommitBrowserTagSidebarInput => Kind::CommitBrowserTagSidebarInput,
        NativeBrowserAction::SetBrowserSidebarLooped { .. } => Kind::SetBrowserSidebarLooped,
        NativeBrowserAction::ToggleBrowserSidebarNormalTag { .. } => {
            Kind::ToggleBrowserSidebarNormalTag
        }
        NativeBrowserAction::FocusMapSample { .. } => Kind::FocusMapSample,
    }
}

fn prompt_edit_action_kind(action: &NativePromptEditAction) -> GuiActionKind {
    match action {
        NativePromptEditAction::SetPromptInput { .. } => Kind::SetPromptInput,
        NativePromptEditAction::StartBrowserRename => Kind::StartBrowserRename,
        NativePromptEditAction::ConfirmBrowserRename => Kind::ConfirmBrowserRename,
        NativePromptEditAction::CancelBrowserRename => Kind::CancelBrowserRename,
        NativePromptEditAction::AutoRenameBrowserSelection { .. } => {
            Kind::AutoRenameBrowserSelection
        }
        NativePromptEditAction::TagBrowserSelection { .. } => Kind::TagBrowserSelection,
        NativePromptEditAction::DeleteBrowserSelection => Kind::DeleteBrowserSelection,
        NativePromptEditAction::NormalizeFocusedBrowserSample => {
            Kind::NormalizeFocusedBrowserSample
        }
        NativePromptEditAction::NormalizeWaveformSelectionOrSample => {
            Kind::NormalizeWaveformSelectionOrSample
        }
        NativePromptEditAction::CropWaveformSelection => Kind::CropWaveformSelection,
        NativePromptEditAction::CropWaveformSelectionToNewSample => {
            Kind::CropWaveformSelectionToNewSample
        }
        NativePromptEditAction::TrimWaveformSelection => Kind::TrimWaveformSelection,
        NativePromptEditAction::ReverseWaveformSelection => Kind::ReverseWaveformSelection,
        NativePromptEditAction::FadeWaveformSelectionLeftToRight => {
            Kind::FadeWaveformSelectionLeftToRight
        }
        NativePromptEditAction::FadeWaveformSelectionRightToLeft => {
            Kind::FadeWaveformSelectionRightToLeft
        }
        NativePromptEditAction::MuteWaveformSelection => Kind::MuteWaveformSelection,
        NativePromptEditAction::DeleteSelectedSliceMarkers => Kind::DeleteSelectedSliceMarkers,
        NativePromptEditAction::ToggleWaveformSliceSelection { .. } => {
            Kind::ToggleWaveformSliceSelection
        }
        NativePromptEditAction::AuditionWaveformDuplicateSlice { .. } => {
            Kind::AuditionWaveformDuplicateSlice
        }
        NativePromptEditAction::ToggleWaveformDuplicateSliceExemption { .. } => {
            Kind::ToggleWaveformDuplicateSliceExemption
        }
        NativePromptEditAction::MoveWaveformSliceFocus { .. } => Kind::MoveWaveformSliceFocus,
        NativePromptEditAction::ToggleFocusedWaveformSliceExportMark => {
            Kind::ToggleFocusedWaveformSliceExportMark
        }
        NativePromptEditAction::AlignWaveformStartToMarker => Kind::AlignWaveformStartToMarker,
        NativePromptEditAction::DeleteLoadedWaveformSample => Kind::DeleteLoadedWaveformSample,
        NativePromptEditAction::SlideWaveformSelection { .. } => Kind::SlideWaveformSelection,
        NativePromptEditAction::ConfirmPrompt => Kind::ConfirmPrompt,
        NativePromptEditAction::CancelPrompt => Kind::CancelPrompt,
        NativePromptEditAction::CancelProgress => Kind::CancelProgress,
        NativePromptEditAction::CopySelectionToClipboard => Kind::CopySelectionToClipboard,
        NativePromptEditAction::ToggleHotkeyOverlay => Kind::ToggleHotkeyOverlay,
        NativePromptEditAction::CopyStatusLog => Kind::CopyStatusLog,
        NativePromptEditAction::OpenFeedbackIssuePrompt => Kind::OpenFeedbackIssuePrompt,
        NativePromptEditAction::MoveTrashedSamplesToFolder => Kind::MoveTrashedSamplesToFolder,
    }
}

fn options_action_kind(action: &NativeOptionsAction) -> GuiActionKind {
    match action {
        NativeOptionsAction::OpenOptionsMenu => Kind::OpenOptionsMenu,
        NativeOptionsAction::CloseOptionsPanel => Kind::CloseOptionsPanel,
        NativeOptionsAction::PickTrashFolder => Kind::PickTrashFolder,
        NativeOptionsAction::OpenTrashFolder => Kind::OpenTrashFolder,
        NativeOptionsAction::EditDefaultIdentifier => Kind::EditDefaultIdentifier,
        NativeOptionsAction::ShowOptionsOverview => Kind::ShowOptionsOverview,
        NativeOptionsAction::OpenAudioOutputHostPicker => Kind::OpenAudioOutputHostPicker,
        NativeOptionsAction::OpenAudioOutputDevicePicker => Kind::OpenAudioOutputDevicePicker,
        NativeOptionsAction::OpenAudioOutputSampleRatePicker => {
            Kind::OpenAudioOutputSampleRatePicker
        }
        NativeOptionsAction::OpenAudioInputHostPicker => Kind::OpenAudioInputHostPicker,
        NativeOptionsAction::OpenAudioInputDevicePicker => Kind::OpenAudioInputDevicePicker,
        NativeOptionsAction::OpenAudioInputSampleRatePicker => Kind::OpenAudioInputSampleRatePicker,
        NativeOptionsAction::SetAudioOutputHost { .. } => Kind::SetAudioOutputHost,
        NativeOptionsAction::SetAudioOutputDevice { .. } => Kind::SetAudioOutputDevice,
        NativeOptionsAction::SetAudioOutputSampleRate { .. } => Kind::SetAudioOutputSampleRate,
        NativeOptionsAction::SetAudioInputHost { .. } => Kind::SetAudioInputHost,
        NativeOptionsAction::SetAudioInputDevice { .. } => Kind::SetAudioInputDevice,
        NativeOptionsAction::SetAudioInputSampleRate { .. } => Kind::SetAudioInputSampleRate,
        NativeOptionsAction::SetInputMonitoringEnabled { .. } => Kind::SetInputMonitoringEnabled,
        NativeOptionsAction::SetAdvanceAfterRatingEnabled { .. } => {
            Kind::SetAdvanceAfterRatingEnabled
        }
        NativeOptionsAction::SetDestructiveYoloMode { .. } => Kind::SetDestructiveYoloMode,
        NativeOptionsAction::SetInvertWaveformScroll { .. } => Kind::SetInvertWaveformScroll,
        NativeOptionsAction::ToggleLoopPlayback => Kind::ToggleLoopPlayback,
        NativeOptionsAction::ToggleLoopLock => Kind::ToggleLoopLock,
        NativeOptionsAction::SetWaveformChannelView { .. } => Kind::SetWaveformChannelView,
        NativeOptionsAction::SetNormalizedAuditionEnabled { .. } => {
            Kind::SetNormalizedAuditionEnabled
        }
        NativeOptionsAction::SetBpmSnapEnabled { .. } => Kind::SetBpmSnapEnabled,
        NativeOptionsAction::SetRelativeBpmGridEnabled { .. } => Kind::SetRelativeBpmGridEnabled,
        NativeOptionsAction::AdjustWaveformBpm { .. } => Kind::AdjustWaveformBpm,
        NativeOptionsAction::SetWaveformBpmValue { .. } => Kind::SetWaveformBpmValue,
        NativeOptionsAction::SetTransientSnapEnabled { .. } => Kind::SetTransientSnapEnabled,
        NativeOptionsAction::SetTransientMarkersEnabled { .. } => Kind::SetTransientMarkersEnabled,
        NativeOptionsAction::ToggleTransientMarkers => Kind::ToggleTransientMarkers,
        NativeOptionsAction::ToggleBpmSnap => Kind::ToggleBpmSnap,
        NativeOptionsAction::SetSliceModeEnabled { .. } => Kind::SetSliceModeEnabled,
        NativeOptionsAction::SetVolume { .. } => Kind::SetVolume,
        NativeOptionsAction::CommitVolumeSetting => Kind::CommitVolumeSetting,
    }
}

fn waveform_action_kind(action: &NativeWaveformAction) -> GuiActionKind {
    match action {
        NativeWaveformAction::SeekWaveformPrecise { .. } => Kind::SeekWaveformPrecise,
        NativeWaveformAction::SetWaveformCursorPrecise { .. } => Kind::SetWaveformCursorPrecise,
        NativeWaveformAction::BeginWaveformSelectionAt { .. } => Kind::BeginWaveformSelectionAt,
        NativeWaveformAction::BeginWaveformSelectionAtPrecise { .. } => {
            Kind::BeginWaveformSelectionAtPrecise
        }
        NativeWaveformAction::BeginWaveformCircularSlide { .. } => Kind::BeginWaveformCircularSlide,
        NativeWaveformAction::UpdateWaveformCircularSlide { .. } => {
            Kind::UpdateWaveformCircularSlide
        }
        NativeWaveformAction::FinishWaveformCircularSlide => Kind::FinishWaveformCircularSlide,
        NativeWaveformAction::SetWaveformSelectionRange { .. } => Kind::SetWaveformSelectionRange,
        NativeWaveformAction::SetWaveformSelectionRangePrecise { .. } => {
            Kind::SetWaveformSelectionRangePrecise
        }
        NativeWaveformAction::SetWaveformSelectionRangeSmartScale { .. } => {
            Kind::SetWaveformSelectionRangeSmartScale
        }
        NativeWaveformAction::SetWaveformSelectionRangeSmartScalePrecise { .. } => {
            Kind::SetWaveformSelectionRangeSmartScalePrecise
        }
        NativeWaveformAction::SetWaveformEditSelectionRange { .. } => {
            Kind::SetWaveformEditSelectionRange
        }
        NativeWaveformAction::SetWaveformEditSelectionRangePrecise { .. } => {
            Kind::SetWaveformEditSelectionRangePrecise
        }
        NativeWaveformAction::SetWaveformEditFadeInEnd { .. } => Kind::SetWaveformEditFadeInEnd,
        NativeWaveformAction::SetWaveformEditFadeInMuteStart { .. } => {
            Kind::SetWaveformEditFadeInMuteStart
        }
        NativeWaveformAction::SetWaveformEditFadeInCurve { .. } => Kind::SetWaveformEditFadeInCurve,
        NativeWaveformAction::SetWaveformEditFadeOutStart { .. } => {
            Kind::SetWaveformEditFadeOutStart
        }
        NativeWaveformAction::SetWaveformEditFadeOutMuteEnd { .. } => {
            Kind::SetWaveformEditFadeOutMuteEnd
        }
        NativeWaveformAction::SetWaveformEditFadeOutCurve { .. } => {
            Kind::SetWaveformEditFadeOutCurve
        }
        NativeWaveformAction::FinishWaveformEditFadeDrag => Kind::FinishWaveformEditFadeDrag,
        NativeWaveformAction::StartWaveformSelectionDrag { .. } => Kind::StartWaveformSelectionDrag,
        NativeWaveformAction::UpdateWaveformSelectionDrag { .. } => {
            Kind::UpdateWaveformSelectionDrag
        }
        NativeWaveformAction::FinishWaveformSelectionDrag => Kind::FinishWaveformSelectionDrag,
        NativeWaveformAction::FinishWaveformSelectionRangeDrag => {
            Kind::FinishWaveformSelectionRangeDrag
        }
        NativeWaveformAction::FinishWaveformSelectionSmartScaleDrag => {
            Kind::FinishWaveformSelectionSmartScaleDrag
        }
        NativeWaveformAction::BeginWaveformSelectionShift { .. } => {
            Kind::BeginWaveformSelectionShift
        }
        NativeWaveformAction::BeginWaveformSelectionShiftPrecise { .. } => {
            Kind::BeginWaveformSelectionShiftPrecise
        }
        NativeWaveformAction::BeginWaveformEditSelectionShift { .. } => {
            Kind::BeginWaveformEditSelectionShift
        }
        NativeWaveformAction::BeginWaveformEditSelectionShiftPrecise { .. } => {
            Kind::BeginWaveformEditSelectionShiftPrecise
        }
        NativeWaveformAction::FinishWaveformEditSelectionDrag => {
            Kind::FinishWaveformEditSelectionDrag
        }
        NativeWaveformAction::ClearWaveformSelection => Kind::ClearWaveformSelection,
        NativeWaveformAction::ClearWaveformEditSelection => Kind::ClearWaveformEditSelection,
        NativeWaveformAction::ClearWaveformSelections => Kind::ClearWaveformSelections,
        NativeWaveformAction::SetWaveformViewCenter { .. } => Kind::SetWaveformViewCenter,
        NativeWaveformAction::ZoomWaveform { .. } => Kind::ZoomWaveform,
        NativeWaveformAction::ZoomWaveformToSelection => Kind::ZoomWaveformToSelection,
        NativeWaveformAction::ZoomWaveformFull => Kind::ZoomWaveformFull,
    }
}

fn transport_action_kind(action: &NativeTransportAction) -> GuiActionKind {
    match action {
        NativeTransportAction::ToggleTransport => Kind::ToggleTransport,
        NativeTransportAction::PlayCompareAnchor => Kind::PlayCompareAnchor,
        NativeTransportAction::PlayFromStart => Kind::PlayFromStart,
        NativeTransportAction::PlayFromCurrentPlayhead => Kind::PlayFromCurrentPlayhead,
        NativeTransportAction::PlayFromWaveformCursor => Kind::PlayFromWaveformCursor,
        NativeTransportAction::PlayWaveformAtPrecise { .. } => Kind::PlayWaveformAtPrecise,
        NativeTransportAction::HandleEscape => Kind::HandleEscape,
    }
}

fn history_update_action_kind(action: &NativeHistoryUpdateAction) -> GuiActionKind {
    match action {
        NativeHistoryUpdateAction::Undo => Kind::Undo,
        NativeHistoryUpdateAction::Redo => Kind::Redo,
        NativeHistoryUpdateAction::CheckForUpdates => Kind::CheckForUpdates,
        NativeHistoryUpdateAction::OpenUpdateLink => Kind::OpenUpdateLink,
        NativeHistoryUpdateAction::InstallUpdate => Kind::InstallUpdate,
        NativeHistoryUpdateAction::DismissUpdate => Kind::DismissUpdate,
    }
}

fn compatibility_action_kind(action: &NativeCompatibilityAction) -> GuiActionKind {
    match action {
        NativeCompatibilityAction::Undo => Kind::Undo,
        NativeCompatibilityAction::Redo => Kind::Redo,
        NativeCompatibilityAction::CheckForUpdates => Kind::CheckForUpdates,
        NativeCompatibilityAction::OpenUpdateLink => Kind::OpenUpdateLink,
        NativeCompatibilityAction::InstallUpdate => Kind::InstallUpdate,
        NativeCompatibilityAction::DismissUpdate => Kind::DismissUpdate,
        NativeCompatibilityAction::SelectColumn { .. } => Kind::SelectColumn,
        NativeCompatibilityAction::MoveColumn { .. } => Kind::MoveColumn,
        NativeCompatibilityAction::SeekWaveform { .. } => Kind::SeekWaveform,
        NativeCompatibilityAction::SetWaveformCursor { .. } => Kind::SetWaveformCursor,
    }
}

gui_action_rows!(build_representative_action_mapping);
