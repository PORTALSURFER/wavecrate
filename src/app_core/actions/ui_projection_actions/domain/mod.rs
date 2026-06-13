//! Domain-family classification for Wavecrate UI actions.
//!
//! This module names action families explicitly so dispatch, catalog,
//! invalidation, and tests can migrate by domain without treating the root
//! action enum as one undifferentiated API.

use super::{CompatibilityAction, OptionsAction, UiAction};

/// Stable domain family for a Wavecrate UI action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UiActionDomain {
    /// Column and triage actions.
    ColumnTriage,
    /// Transport actions.
    Transport,
    /// Shell actions.
    Shell,
    /// Source and folder-tree actions.
    SourcesAndFolders,
    /// Browser actions.
    Browser,
    /// Prompt, rename, file-edit, and confirmation actions.
    PromptsAndEdits,
    /// Options actions.
    Options,
    /// Waveform actions.
    Waveform,
    /// History and update actions.
    HistoryAndUpdates,
}

impl UiAction {
    /// Return the domain family that owns this action's behavior.
    pub fn domain(&self) -> UiActionDomain {
        match self {
            // Transport and global playback actions.
            UiAction::Transport(
                crate::app_core::actions::NativeTransportAction::ToggleTransport,
            ) => UiActionDomain::Transport,
            UiAction::Transport(
                crate::app_core::actions::NativeTransportAction::PlayCompareAnchor,
            ) => UiActionDomain::Transport,
            UiAction::Transport(crate::app_core::actions::NativeTransportAction::PlayFromStart) => {
                UiActionDomain::Transport
            }
            UiAction::Transport(
                crate::app_core::actions::NativeTransportAction::PlayFromCurrentPlayhead,
            ) => UiActionDomain::Transport,
            UiAction::Transport(
                crate::app_core::actions::NativeTransportAction::PlayFromWaveformCursor,
            ) => UiActionDomain::Transport,
            UiAction::Transport(
                crate::app_core::actions::NativeTransportAction::PlayWaveformAtPrecise { .. },
            ) => UiActionDomain::Transport,
            UiAction::Transport(crate::app_core::actions::NativeTransportAction::HandleEscape) => {
                UiActionDomain::Transport
            }
            UiAction::HistoryAndUpdate(_) => UiActionDomain::HistoryAndUpdates,
            // Focus and shell-surface actions.
            UiAction::FocusBrowserPanel => UiActionDomain::Shell,
            UiAction::FocusSourcesPanel => UiActionDomain::Shell,
            UiAction::FocusWaveformPanel => UiActionDomain::Shell,
            UiAction::FocusFolderPanel => UiActionDomain::Shell,
            UiAction::FocusLoadedSampleInBrowser => UiActionDomain::Shell,
            UiAction::FocusBrowserSearch => UiActionDomain::Shell,
            UiAction::BlurBrowserSearch => UiActionDomain::Shell,
            UiAction::OpenAddSourceDialog => UiActionDomain::Shell,
            UiAction::Options(OptionsAction::OpenOptionsMenu)
            | UiAction::Options(OptionsAction::CloseOptionsPanel)
            | UiAction::Options(OptionsAction::PickTrashFolder)
            | UiAction::Options(OptionsAction::OpenTrashFolder)
            | UiAction::Options(OptionsAction::EditDefaultIdentifier)
            | UiAction::Options(OptionsAction::ShowOptionsOverview)
            | UiAction::Options(OptionsAction::OpenAudioOutputHostPicker)
            | UiAction::Options(OptionsAction::OpenAudioOutputDevicePicker)
            | UiAction::Options(OptionsAction::OpenAudioOutputSampleRatePicker)
            | UiAction::Options(OptionsAction::OpenAudioInputHostPicker)
            | UiAction::Options(OptionsAction::OpenAudioInputDevicePicker)
            | UiAction::Options(OptionsAction::OpenAudioInputSampleRatePicker)
            | UiAction::Options(OptionsAction::SetAudioOutputHost { .. })
            | UiAction::Options(OptionsAction::SetAudioOutputDevice { .. })
            | UiAction::Options(OptionsAction::SetAudioOutputSampleRate { .. })
            | UiAction::Options(OptionsAction::SetAudioInputHost { .. })
            | UiAction::Options(OptionsAction::SetAudioInputDevice { .. })
            | UiAction::Options(OptionsAction::SetAudioInputSampleRate { .. }) => {
                UiActionDomain::Options
            }
            UiAction::FocusFolderSearch => UiActionDomain::Shell,
            UiAction::SetFolderSearch { .. } => UiActionDomain::Shell,
            UiAction::ToggleShowAllFolders => UiActionDomain::Shell,
            UiAction::ToggleFolderFlattenedView => UiActionDomain::Shell,
            // Sources and folder tree actions.
            UiAction::FocusSourceRow { .. } => UiActionDomain::SourcesAndFolders,
            UiAction::SelectSourceRow { .. } => UiActionDomain::SourcesAndFolders,
            UiAction::MoveSourceFocus { .. } => UiActionDomain::SourcesAndFolders,
            UiAction::ReloadFocusedSourceRow => UiActionDomain::SourcesAndFolders,
            UiAction::HardSyncFocusedSourceRow => UiActionDomain::SourcesAndFolders,
            UiAction::OpenFocusedSourceFolder => UiActionDomain::SourcesAndFolders,
            UiAction::RemoveFocusedSourceRow => UiActionDomain::SourcesAndFolders,
            UiAction::ReloadSourceRow { .. } => UiActionDomain::SourcesAndFolders,
            UiAction::HardSyncSourceRow { .. } => UiActionDomain::SourcesAndFolders,
            UiAction::OpenSourceFolderRow { .. } => UiActionDomain::SourcesAndFolders,
            UiAction::RemoveSourceRow { .. } => UiActionDomain::SourcesAndFolders,
            UiAction::FocusFolderRow { .. } => UiActionDomain::SourcesAndFolders,
            UiAction::ActivateFolderRow { .. } => UiActionDomain::SourcesAndFolders,
            UiAction::ToggleFolderRowExpanded { .. } => UiActionDomain::SourcesAndFolders,
            UiAction::ExpandFocusedFolder => UiActionDomain::SourcesAndFolders,
            UiAction::CollapseFocusedFolder => UiActionDomain::SourcesAndFolders,
            UiAction::ToggleFocusedFolderSelection => UiActionDomain::SourcesAndFolders,
            UiAction::MoveFolderFocus { .. } => UiActionDomain::SourcesAndFolders,
            UiAction::StartNewFolder => UiActionDomain::SourcesAndFolders,
            UiAction::StartNewFolderAtFolderRow { .. } => UiActionDomain::SourcesAndFolders,
            UiAction::StartNewFolderAtRoot => UiActionDomain::SourcesAndFolders,
            UiAction::FocusFolderCreateInput => UiActionDomain::SourcesAndFolders,
            UiAction::SetFolderCreateInput { .. } => UiActionDomain::SourcesAndFolders,
            UiAction::ConfirmFolderCreate => UiActionDomain::SourcesAndFolders,
            UiAction::CancelFolderCreate => UiActionDomain::SourcesAndFolders,
            UiAction::StartFolderRename => UiActionDomain::SourcesAndFolders,
            UiAction::DeleteFocusedFolder => UiActionDomain::SourcesAndFolders,
            UiAction::RestoreRetainedFolderDeletes => UiActionDomain::SourcesAndFolders,
            UiAction::PurgeRetainedFolderDeletes => UiActionDomain::SourcesAndFolders,
            UiAction::ClearFolderDeleteRecoveryLog => UiActionDomain::SourcesAndFolders,
            // Browser navigation, selection, search, and map actions.
            UiAction::MoveBrowserFocus { .. } => UiActionDomain::Browser,
            UiAction::SetBrowserViewStart { .. } => UiActionDomain::Browser,
            UiAction::FocusBrowserRow { .. } => UiActionDomain::Browser,
            UiAction::SetCompareAnchorFromFocusedBrowserSample => UiActionDomain::Browser,
            UiAction::CommitFocusedBrowserRow => UiActionDomain::Browser,
            UiAction::SaveWaveformSelectionToBrowser => UiActionDomain::Browser,
            UiAction::SaveWaveformSelectionToBrowserWithKeep2 => UiActionDomain::Browser,
            UiAction::CommitWaveformEditFades => UiActionDomain::Browser,
            UiAction::DetectWaveformSilenceSlices => UiActionDomain::Browser,
            UiAction::DetectWaveformExactDuplicateSlices => UiActionDomain::Browser,
            UiAction::CleanWaveformExactDuplicateSlices => UiActionDomain::Browser,
            UiAction::ToggleBrowserRowSelection { .. } => UiActionDomain::Browser,
            UiAction::StartBrowserSampleDrag { .. } => UiActionDomain::Browser,
            UiAction::UpdateBrowserSampleDrag { .. } => UiActionDomain::Browser,
            UiAction::FinishBrowserSampleDrag => UiActionDomain::Browser,
            UiAction::ExtendBrowserSelectionToRow { .. } => UiActionDomain::Browser,
            UiAction::AddRangeBrowserSelection { .. } => UiActionDomain::Browser,
            UiAction::ExtendBrowserSelectionFromFocus { .. } => UiActionDomain::Browser,
            UiAction::AddRangeBrowserSelectionFromFocus { .. } => UiActionDomain::Browser,
            UiAction::ToggleFocusedBrowserRowSelection => UiActionDomain::Browser,
            UiAction::SelectAllBrowserRows => UiActionDomain::Browser,
            UiAction::SetBrowserSearch { .. } => UiActionDomain::Browser,
            UiAction::ToggleBrowserRatingFilter { .. } => UiActionDomain::Browser,
            UiAction::ToggleBrowserPlaybackAgeFilter { .. } => UiActionDomain::Browser,
            UiAction::ToggleBrowserSidebarFilter { .. } => UiActionDomain::Browser,
            UiAction::ClearBrowserSidebarFilter { .. } => UiActionDomain::Browser,
            UiAction::ToggleBrowserSampleMark => UiActionDomain::Browser,
            UiAction::ToggleBrowserMarkedFilter => UiActionDomain::Browser,
            UiAction::ToggleBrowserTagNamedFilter { .. } => UiActionDomain::Browser,
            UiAction::ToggleRandomNavigationMode => UiActionDomain::Browser,
            UiAction::ToggleBrowserTagSidebar => UiActionDomain::Browser,
            UiAction::ToggleBrowserTagSidebarAutoRename => UiActionDomain::Browser,
            UiAction::ToggleBrowserDuplicateCleanupMode => UiActionDomain::Browser,
            UiAction::FocusPreviousBrowserHistory => UiActionDomain::Browser,
            UiAction::FocusNextBrowserHistory => UiActionDomain::Browser,
            UiAction::ToggleFindSimilarFocusedSample => UiActionDomain::Browser,
            UiAction::ToggleBrowserDuplicateCleanupKeep { .. } => UiActionDomain::Browser,
            UiAction::ConfirmBrowserDuplicateCleanup => UiActionDomain::Browser,
            UiAction::PlayRandomSample => UiActionDomain::Browser,
            UiAction::PlayPreviousRandomSample => UiActionDomain::Browser,
            UiAction::AdjustSelectedBrowserRating { .. } => UiActionDomain::Browser,
            UiAction::SetBrowserTab { .. } => UiActionDomain::Browser,
            UiAction::FocusBrowserTagSidebarInput => UiActionDomain::Browser,
            UiAction::SetBrowserTagSidebarInput { .. } => UiActionDomain::Browser,
            UiAction::CommitBrowserTagSidebarInput => UiActionDomain::Browser,
            UiAction::SetBrowserSidebarLooped { .. } => UiActionDomain::Browser,
            UiAction::ToggleBrowserSidebarNormalTag { .. } => UiActionDomain::Browser,
            UiAction::FocusMapSample { .. } => UiActionDomain::Browser,
            // Prompt, rename, and confirmation actions.
            UiAction::SetPromptInput { .. } => UiActionDomain::PromptsAndEdits,
            UiAction::StartBrowserRename => UiActionDomain::PromptsAndEdits,
            UiAction::ConfirmBrowserRename => UiActionDomain::PromptsAndEdits,
            UiAction::CancelBrowserRename => UiActionDomain::PromptsAndEdits,
            UiAction::AutoRenameBrowserSelection { .. } => UiActionDomain::PromptsAndEdits,
            UiAction::TagBrowserSelection { .. } => UiActionDomain::PromptsAndEdits,
            UiAction::DeleteBrowserSelection => UiActionDomain::PromptsAndEdits,
            UiAction::NormalizeFocusedBrowserSample => UiActionDomain::PromptsAndEdits,
            UiAction::NormalizeWaveformSelectionOrSample => UiActionDomain::PromptsAndEdits,
            UiAction::CropWaveformSelection => UiActionDomain::PromptsAndEdits,
            UiAction::CropWaveformSelectionToNewSample => UiActionDomain::PromptsAndEdits,
            UiAction::TrimWaveformSelection => UiActionDomain::PromptsAndEdits,
            UiAction::ReverseWaveformSelection => UiActionDomain::PromptsAndEdits,
            UiAction::FadeWaveformSelectionLeftToRight => UiActionDomain::PromptsAndEdits,
            UiAction::FadeWaveformSelectionRightToLeft => UiActionDomain::PromptsAndEdits,
            UiAction::MuteWaveformSelection => UiActionDomain::PromptsAndEdits,
            UiAction::DeleteSelectedSliceMarkers => UiActionDomain::PromptsAndEdits,
            UiAction::ToggleWaveformSliceSelection { .. } => UiActionDomain::PromptsAndEdits,
            UiAction::AuditionWaveformDuplicateSlice { .. } => UiActionDomain::PromptsAndEdits,
            UiAction::ToggleWaveformDuplicateSliceExemption { .. } => {
                UiActionDomain::PromptsAndEdits
            }
            UiAction::MoveWaveformSliceFocus { .. } => UiActionDomain::PromptsAndEdits,
            UiAction::ToggleFocusedWaveformSliceExportMark => UiActionDomain::PromptsAndEdits,
            UiAction::AlignWaveformStartToMarker => UiActionDomain::PromptsAndEdits,
            UiAction::DeleteLoadedWaveformSample => UiActionDomain::PromptsAndEdits,
            UiAction::SlideWaveformSelection { .. } => UiActionDomain::PromptsAndEdits,
            UiAction::ConfirmPrompt => UiActionDomain::PromptsAndEdits,
            UiAction::CancelPrompt => UiActionDomain::PromptsAndEdits,
            UiAction::CancelProgress => UiActionDomain::PromptsAndEdits,
            UiAction::CopySelectionToClipboard => UiActionDomain::PromptsAndEdits,
            UiAction::ToggleHotkeyOverlay => UiActionDomain::PromptsAndEdits,
            UiAction::CopyStatusLog => UiActionDomain::PromptsAndEdits,
            UiAction::OpenFeedbackIssuePrompt => UiActionDomain::PromptsAndEdits,
            UiAction::MoveTrashedSamplesToFolder => UiActionDomain::PromptsAndEdits,
            // Options and persistent interaction toggles.
            UiAction::Options(OptionsAction::SetInputMonitoringEnabled { .. }) => {
                UiActionDomain::Options
            }
            UiAction::Options(OptionsAction::SetAdvanceAfterRatingEnabled { .. }) => {
                UiActionDomain::Options
            }
            UiAction::Options(OptionsAction::SetDestructiveYoloMode { .. }) => {
                UiActionDomain::Options
            }
            UiAction::Options(OptionsAction::SetInvertWaveformScroll { .. }) => {
                UiActionDomain::Options
            }
            UiAction::ToggleLoopPlayback => UiActionDomain::Options,
            UiAction::ToggleLoopLock => UiActionDomain::Options,
            UiAction::SetWaveformChannelView { .. } => UiActionDomain::Options,
            UiAction::SetNormalizedAuditionEnabled { .. } => UiActionDomain::Options,
            UiAction::SetBpmSnapEnabled { .. } => UiActionDomain::Options,
            UiAction::SetRelativeBpmGridEnabled { .. } => UiActionDomain::Options,
            UiAction::AdjustWaveformBpm { .. } => UiActionDomain::Options,
            UiAction::SetWaveformBpmValue { .. } => UiActionDomain::Options,
            UiAction::SetTransientSnapEnabled { .. } => UiActionDomain::Options,
            UiAction::SetTransientMarkersEnabled { .. } => UiActionDomain::Options,
            UiAction::ToggleTransientMarkers => UiActionDomain::Options,
            UiAction::ToggleBpmSnap => UiActionDomain::Options,
            UiAction::SetSliceModeEnabled { .. } => UiActionDomain::Options,
            UiAction::Options(OptionsAction::SetVolume { .. }) => UiActionDomain::Options,
            UiAction::Options(OptionsAction::CommitVolumeSetting) => UiActionDomain::Options,
            // Waveform transport, edit, and gesture actions.
            UiAction::SeekWaveformPrecise { .. } => UiActionDomain::Waveform,
            UiAction::SetWaveformCursorPrecise { .. } => UiActionDomain::Waveform,
            UiAction::BeginWaveformSelectionAt { .. } => UiActionDomain::Waveform,
            UiAction::BeginWaveformSelectionAtPrecise { .. } => UiActionDomain::Waveform,
            UiAction::BeginWaveformCircularSlide { .. } => UiActionDomain::Waveform,
            UiAction::UpdateWaveformCircularSlide { .. } => UiActionDomain::Waveform,
            UiAction::FinishWaveformCircularSlide => UiActionDomain::Waveform,
            UiAction::SetWaveformSelectionRange { .. } => UiActionDomain::Waveform,
            UiAction::SetWaveformSelectionRangePrecise { .. } => UiActionDomain::Waveform,
            UiAction::SetWaveformSelectionRangeSmartScale { .. } => UiActionDomain::Waveform,
            UiAction::SetWaveformSelectionRangeSmartScalePrecise { .. } => UiActionDomain::Waveform,
            UiAction::SetWaveformEditSelectionRange { .. } => UiActionDomain::Waveform,
            UiAction::SetWaveformEditSelectionRangePrecise { .. } => UiActionDomain::Waveform,
            UiAction::SetWaveformEditFadeInEnd { .. } => UiActionDomain::Waveform,
            UiAction::SetWaveformEditFadeInMuteStart { .. } => UiActionDomain::Waveform,
            UiAction::SetWaveformEditFadeInCurve { .. } => UiActionDomain::Waveform,
            UiAction::SetWaveformEditFadeOutStart { .. } => UiActionDomain::Waveform,
            UiAction::SetWaveformEditFadeOutMuteEnd { .. } => UiActionDomain::Waveform,
            UiAction::SetWaveformEditFadeOutCurve { .. } => UiActionDomain::Waveform,
            UiAction::FinishWaveformEditFadeDrag => UiActionDomain::Waveform,
            UiAction::StartWaveformSelectionDrag { .. } => UiActionDomain::Waveform,
            UiAction::UpdateWaveformSelectionDrag { .. } => UiActionDomain::Waveform,
            UiAction::FinishWaveformSelectionDrag => UiActionDomain::Waveform,
            UiAction::FinishWaveformSelectionRangeDrag => UiActionDomain::Waveform,
            UiAction::FinishWaveformSelectionSmartScaleDrag => UiActionDomain::Waveform,
            UiAction::BeginWaveformSelectionShift { .. } => UiActionDomain::Waveform,
            UiAction::BeginWaveformSelectionShiftPrecise { .. } => UiActionDomain::Waveform,
            UiAction::BeginWaveformEditSelectionShift { .. } => UiActionDomain::Waveform,
            UiAction::BeginWaveformEditSelectionShiftPrecise { .. } => UiActionDomain::Waveform,
            UiAction::FinishWaveformEditSelectionDrag => UiActionDomain::Waveform,
            UiAction::ClearWaveformSelection => UiActionDomain::Waveform,
            UiAction::ClearWaveformEditSelection => UiActionDomain::Waveform,
            UiAction::ClearWaveformSelections => UiActionDomain::Waveform,
            UiAction::SetWaveformViewCenter { .. } => UiActionDomain::Waveform,
            UiAction::ZoomWaveform { .. } => UiActionDomain::Waveform,
            UiAction::ZoomWaveformToSelection => UiActionDomain::Waveform,
            UiAction::ZoomWaveformFull => UiActionDomain::Waveform,
            // Retained flat compatibility inputs for history and update actions.
            UiAction::Compatibility(
                CompatibilityAction::Undo
                | CompatibilityAction::Redo
                | CompatibilityAction::CheckForUpdates
                | CompatibilityAction::OpenUpdateLink
                | CompatibilityAction::InstallUpdate
                | CompatibilityAction::DismissUpdate,
            ) => UiActionDomain::HistoryAndUpdates,
            UiAction::Compatibility(
                CompatibilityAction::SelectColumn { .. } | CompatibilityAction::MoveColumn { .. },
            ) => UiActionDomain::ColumnTriage,
            UiAction::Compatibility(
                CompatibilityAction::SeekWaveform { .. }
                | CompatibilityAction::SetWaveformCursor { .. },
            ) => UiActionDomain::Waveform,
        }
    }
}

#[cfg(test)]
mod tests;
