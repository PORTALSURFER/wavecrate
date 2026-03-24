//! Stable payload-free GUI action identities used across host tooling.

use serde::Serialize;

/// Stable payload-free identity for one GUI action variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuiActionKind {
    /// Select one top-level shell column directly.
    SelectColumn,
    /// Move the focused shell column by a relative delta.
    MoveColumn,
    /// Toggle transport playback on or off.
    ToggleTransport,
    /// Start playback from the beginning of the current sample or loop.
    PlayFromStart,
    /// Start playback from the current playhead position.
    PlayFromCurrentPlayhead,
    /// Apply the shell-wide escape-key behavior.
    HandleEscape,
    /// Move focus into the browser panel.
    FocusBrowserPanel,
    /// Move focus into the sources panel.
    FocusSourcesPanel,
    /// Move focus into the waveform panel.
    FocusWaveformPanel,
    /// Move focus into the folder-browser subsection of the sources panel.
    FocusFolderPanel,
    /// Focus the currently loaded sample inside the browser list.
    FocusLoadedSampleInBrowser,
    /// Focus the browser search field.
    FocusBrowserSearch,
    /// Remove focus from the browser search field.
    BlurBrowserSearch,
    /// Open the add-source dialog.
    OpenAddSourceDialog,
    /// Open the options menu or panel.
    OpenOptionsMenu,
    /// Close the options panel.
    CloseOptionsPanel,
    /// Open the trash-folder picker flow.
    PickTrashFolder,
    /// Open the configured trash folder in the host shell.
    OpenTrashFolder,
    /// Focus the folder-search field in the sources panel.
    FocusFolderSearch,
    /// Set the sources folder-search query.
    SetFolderSearch,
    /// Focus one source row directly and activate the sources list section.
    FocusSourceRow,
    /// Select one source row directly.
    SelectSourceRow,
    /// Move source focus by a relative delta.
    MoveSourceFocus,
    /// Reload the currently focused source row.
    ReloadFocusedSourceRow,
    /// Force a hard sync on the currently focused source row.
    HardSyncFocusedSourceRow,
    /// Open the currently focused source folder in the host shell.
    OpenFocusedSourceFolder,
    /// Remove the currently focused source row from the library list.
    RemoveFocusedSourceRow,
    /// Remove dead links associated with the currently focused source row.
    RemoveDeadLinksForFocusedSourceRow,
    /// Reload the focused source row.
    ReloadSourceRow,
    /// Force a hard sync on the focused source row.
    HardSyncSourceRow,
    /// Open the focused source folder row in the host shell.
    OpenSourceFolderRow,
    /// Remove the focused source row from the library list.
    RemoveSourceRow,
    /// Remove dead links associated with the focused source row.
    RemoveDeadLinksForSourceRow,
    /// Focus one folder row directly.
    FocusFolderRow,
    /// Toggle selection for the currently focused folder row.
    ToggleFocusedFolderSelection,
    /// Move folder focus by a relative delta.
    MoveFolderFocus,
    /// Start creating a new folder under the current parent.
    StartNewFolder,
    /// Start creating a new folder at the source root.
    StartNewFolderAtRoot,
    /// Start renaming the focused folder.
    StartFolderRename,
    /// Delete the focused folder.
    DeleteFocusedFolder,
    /// Clear the folder-delete recovery log.
    ClearFolderDeleteRecoveryLog,
    /// Move browser focus by a relative row delta.
    MoveBrowserFocus,
    /// Set the top visible browser row explicitly.
    SetBrowserViewStart,
    /// Focus one browser row directly.
    FocusBrowserRow,
    /// Commit the currently focused browser row.
    CommitFocusedBrowserRow,
    /// Save the current waveform selection back to the browser/sample metadata.
    SaveWaveformSelectionToBrowser,
    /// Detect silence-separated waveform slices and preview them for export.
    DetectWaveformSilenceSlices,
    /// Toggle selection on one browser row.
    ToggleBrowserRowSelection,
    /// Extend browser selection through one row.
    ExtendBrowserSelectionToRow,
    /// Add a contiguous browser-row range to the current selection.
    AddRangeBrowserSelection,
    /// Extend browser selection from the focused anchor row.
    ExtendBrowserSelectionFromFocus,
    /// Add a browser-row range from the focused anchor without clearing existing selection.
    AddRangeBrowserSelectionFromFocus,
    /// Toggle selection on the currently focused browser row.
    ToggleFocusedBrowserRowSelection,
    /// Select every visible browser row.
    SelectAllBrowserRows,
    /// Set the browser search query.
    SetBrowserSearch,
    /// Toggle one browser rating-filter chip.
    ToggleBrowserRatingFilter,
    /// Toggle random browser-navigation mode.
    ToggleRandomNavigationMode,
    /// Focus the previous sample from browser history.
    FocusPreviousBrowserHistory,
    /// Focus the next sample from browser history.
    FocusNextBrowserHistory,
    /// Toggle find-similar mode for the focused browser sample.
    ToggleFindSimilarFocusedSample,
    /// Play a random visible browser sample.
    PlayRandomSample,
    /// Replay the previous random-visible browser sample.
    PlayPreviousRandomSample,
    /// Adjust the rating for selected browser rows by a signed delta.
    AdjustSelectedBrowserRating,
    /// Switch the browser between samples and map tabs.
    SetBrowserTab,
    /// Focus one sample in the map view.
    FocusMapSample,
    /// Set the active prompt input text.
    SetPromptInput,
    /// Start the browser rename flow for the focused item.
    StartBrowserRename,
    /// Confirm the active browser rename prompt.
    ConfirmBrowserRename,
    /// Cancel the active browser rename prompt.
    CancelBrowserRename,
    /// Apply a rating/tag to the current browser selection.
    TagBrowserSelection,
    /// Delete the current browser selection.
    DeleteBrowserSelection,
    /// Normalize the currently focused browser sample.
    NormalizeFocusedBrowserSample,
    /// Normalize the waveform selection or whole sample.
    NormalizeWaveformSelectionOrSample,
    /// Crop the current sample to the active waveform selection.
    CropWaveformSelection,
    /// Crop the waveform selection into a newly created sample.
    CropWaveformSelectionToNewSample,
    /// Trim away audio outside the active waveform selection.
    TrimWaveformSelection,
    /// Reverse the active waveform selection.
    ReverseWaveformSelection,
    /// Fade the active waveform selection from left to right.
    FadeWaveformSelectionLeftToRight,
    /// Fade the active waveform selection from right to left.
    FadeWaveformSelectionRightToLeft,
    /// Mute the active waveform selection or merge selected slices.
    MuteWaveformSelection,
    /// Delete the selected slice markers.
    DeleteSelectedSliceMarkers,
    /// Align waveform start to the latest hover marker.
    AlignWaveformStartToMarker,
    /// Delete the currently loaded waveform sample.
    DeleteLoadedWaveformSample,
    /// Slide or nudge the active waveform selection.
    SlideWaveformSelection,
    /// Confirm the active prompt dialog.
    ConfirmPrompt,
    /// Cancel the active prompt dialog.
    CancelPrompt,
    /// Cancel the active progress operation.
    CancelProgress,
    /// Toggle the hotkey overlay.
    ToggleHotkeyOverlay,
    /// Copy the status log to the clipboard.
    CopyStatusLog,
    /// Open the feedback issue prompt flow.
    OpenFeedbackIssuePrompt,
    /// Move trashed browser samples into the configured trash folder.
    MoveTrashedSamplesToFolder,
    /// Enable or disable live input monitoring.
    SetInputMonitoringEnabled,
    /// Enable or disable automatic advance after rating.
    SetAdvanceAfterRatingEnabled,
    /// Enable or disable destructive YOLO mode.
    SetDestructiveYoloMode,
    /// Enable or disable inverted waveform-scroll behavior.
    SetInvertWaveformScroll,
    /// Toggle loop playback for the active sample or selection.
    ToggleLoopPlayback,
    /// Toggle whether loop state stays locked across sample changes.
    ToggleLoopLock,
    /// Switch the waveform channel-view mode.
    SetWaveformChannelView,
    /// Enable or disable normalized audition playback.
    SetNormalizedAuditionEnabled,
    /// Enable or disable BPM snap behavior.
    SetBpmSnapEnabled,
    /// Adjust BPM by a relative amount.
    AdjustWaveformBpm,
    /// Set BPM to an explicit value.
    SetWaveformBpmValue,
    /// Enable or disable transient snapping.
    SetTransientSnapEnabled,
    /// Enable or disable transient marker visibility.
    SetTransientMarkersEnabled,
    /// Toggle transient marker visibility.
    ToggleTransientMarkers,
    /// Toggle BPM snap behavior.
    ToggleBpmSnap,
    /// Enable or disable waveform slice mode.
    SetSliceModeEnabled,
    /// Toggle selection for one previewed waveform slice.
    ToggleWaveformSliceSelection,
    /// Set transport volume.
    SetVolume,
    /// Commit the current volume setting after an interactive edit.
    CommitVolumeSetting,
    /// Seek playback to one waveform position using nanounit precision.
    SeekWaveformPrecise,
    /// Set the waveform cursor to one position using nanounit precision.
    SetWaveformCursorPrecise,
    /// Seek playback to one waveform position.
    SeekWaveform,
    /// Set the waveform cursor to one position.
    SetWaveformCursor,
    /// Begin a new waveform selection from one exact anchor point.
    BeginWaveformSelectionAt,
    /// Set the playback selection range directly.
    SetWaveformSelectionRange,
    /// Set the playback selection range while applying BPM smart-scale behavior.
    SetWaveformSelectionRangeSmartScale,
    /// Set the edit selection range directly.
    SetWaveformEditSelectionRange,
    /// Set the edit fade-in end handle.
    SetWaveformEditFadeInEnd,
    /// Set the edit fade-in mute start handle.
    SetWaveformEditFadeInMuteStart,
    /// Set the edit fade-in curve shape.
    SetWaveformEditFadeInCurve,
    /// Set the edit fade-out start handle.
    SetWaveformEditFadeOutStart,
    /// Set the edit fade-out mute end handle.
    SetWaveformEditFadeOutMuteEnd,
    /// Set the edit fade-out curve shape.
    SetWaveformEditFadeOutCurve,
    /// Finish an interactive edit-fade drag.
    FinishWaveformEditFadeDrag,
    /// Start a playback-selection drag gesture.
    StartWaveformSelectionDrag,
    /// Update an in-progress playback-selection drag gesture.
    UpdateWaveformSelectionDrag,
    /// Finish an interactive playback-selection drag.
    FinishWaveformSelectionDrag,
    /// Finish an interactive playback-selection range drag.
    FinishWaveformSelectionRangeDrag,
    /// Finish an interactive smart-scale playback-selection drag.
    FinishWaveformSelectionSmartScaleDrag,
    /// Begin shifting the playback selection without resizing it.
    BeginWaveformSelectionShift,
    /// Begin shifting the edit selection without resizing it.
    BeginWaveformEditSelectionShift,
    /// Finish an interactive edit-selection range drag.
    FinishWaveformEditSelectionDrag,
    /// Clear the active playback selection.
    ClearWaveformSelection,
    /// Clear the active edit selection.
    ClearWaveformEditSelection,
    /// Clear both playback and edit selections together.
    ClearWaveformSelections,
    /// Center the waveform viewport on one position.
    SetWaveformViewCenter,
    /// Zoom the waveform viewport by a relative amount.
    ZoomWaveform,
    /// Zoom the waveform viewport to the active selection.
    ZoomWaveformToSelection,
    /// Reset the waveform viewport to the full sample.
    ZoomWaveformFull,
    /// Undo the last reversible user action.
    Undo,
    /// Redo the last undone user action.
    Redo,
    /// Start the check-for-updates flow.
    CheckForUpdates,
    /// Open the selected update or release link.
    OpenUpdateLink,
    /// Start installing the selected update.
    InstallUpdate,
    /// Dismiss the active update prompt or panel.
    DismissUpdate,
}

impl GuiActionKind {
    /// All currently cataloged action kinds in stable declaration order.
    pub const ALL: [Self; 146] = [
        Self::SelectColumn,
        Self::MoveColumn,
        Self::ToggleTransport,
        Self::PlayFromStart,
        Self::PlayFromCurrentPlayhead,
        Self::HandleEscape,
        Self::FocusBrowserPanel,
        Self::FocusSourcesPanel,
        Self::FocusWaveformPanel,
        Self::FocusFolderPanel,
        Self::FocusLoadedSampleInBrowser,
        Self::FocusBrowserSearch,
        Self::BlurBrowserSearch,
        Self::OpenAddSourceDialog,
        Self::OpenOptionsMenu,
        Self::CloseOptionsPanel,
        Self::PickTrashFolder,
        Self::OpenTrashFolder,
        Self::FocusFolderSearch,
        Self::SetFolderSearch,
        Self::FocusSourceRow,
        Self::SelectSourceRow,
        Self::MoveSourceFocus,
        Self::ReloadFocusedSourceRow,
        Self::HardSyncFocusedSourceRow,
        Self::OpenFocusedSourceFolder,
        Self::RemoveFocusedSourceRow,
        Self::RemoveDeadLinksForFocusedSourceRow,
        Self::ReloadSourceRow,
        Self::HardSyncSourceRow,
        Self::OpenSourceFolderRow,
        Self::RemoveSourceRow,
        Self::RemoveDeadLinksForSourceRow,
        Self::FocusFolderRow,
        Self::ToggleFocusedFolderSelection,
        Self::MoveFolderFocus,
        Self::StartNewFolder,
        Self::StartNewFolderAtRoot,
        Self::StartFolderRename,
        Self::DeleteFocusedFolder,
        Self::ClearFolderDeleteRecoveryLog,
        Self::MoveBrowserFocus,
        Self::SetBrowserViewStart,
        Self::FocusBrowserRow,
        Self::CommitFocusedBrowserRow,
        Self::SaveWaveformSelectionToBrowser,
        Self::DetectWaveformSilenceSlices,
        Self::ToggleBrowserRowSelection,
        Self::ExtendBrowserSelectionToRow,
        Self::AddRangeBrowserSelection,
        Self::ExtendBrowserSelectionFromFocus,
        Self::AddRangeBrowserSelectionFromFocus,
        Self::ToggleFocusedBrowserRowSelection,
        Self::SelectAllBrowserRows,
        Self::SetBrowserSearch,
        Self::ToggleBrowserRatingFilter,
        Self::ToggleRandomNavigationMode,
        Self::FocusPreviousBrowserHistory,
        Self::FocusNextBrowserHistory,
        Self::ToggleFindSimilarFocusedSample,
        Self::PlayRandomSample,
        Self::PlayPreviousRandomSample,
        Self::AdjustSelectedBrowserRating,
        Self::SetBrowserTab,
        Self::FocusMapSample,
        Self::SetPromptInput,
        Self::StartBrowserRename,
        Self::ConfirmBrowserRename,
        Self::CancelBrowserRename,
        Self::TagBrowserSelection,
        Self::DeleteBrowserSelection,
        Self::NormalizeFocusedBrowserSample,
        Self::NormalizeWaveformSelectionOrSample,
        Self::CropWaveformSelection,
        Self::CropWaveformSelectionToNewSample,
        Self::TrimWaveformSelection,
        Self::ReverseWaveformSelection,
        Self::FadeWaveformSelectionLeftToRight,
        Self::FadeWaveformSelectionRightToLeft,
        Self::MuteWaveformSelection,
        Self::DeleteSelectedSliceMarkers,
        Self::AlignWaveformStartToMarker,
        Self::DeleteLoadedWaveformSample,
        Self::SlideWaveformSelection,
        Self::ConfirmPrompt,
        Self::CancelPrompt,
        Self::CancelProgress,
        Self::ToggleHotkeyOverlay,
        Self::CopyStatusLog,
        Self::OpenFeedbackIssuePrompt,
        Self::MoveTrashedSamplesToFolder,
        Self::SetInputMonitoringEnabled,
        Self::SetAdvanceAfterRatingEnabled,
        Self::SetDestructiveYoloMode,
        Self::SetInvertWaveformScroll,
        Self::ToggleLoopPlayback,
        Self::ToggleLoopLock,
        Self::SetWaveformChannelView,
        Self::SetNormalizedAuditionEnabled,
        Self::SetBpmSnapEnabled,
        Self::AdjustWaveformBpm,
        Self::SetWaveformBpmValue,
        Self::SetTransientSnapEnabled,
        Self::SetTransientMarkersEnabled,
        Self::ToggleTransientMarkers,
        Self::ToggleBpmSnap,
        Self::SetSliceModeEnabled,
        Self::ToggleWaveformSliceSelection,
        Self::SetVolume,
        Self::CommitVolumeSetting,
        Self::SeekWaveformPrecise,
        Self::SetWaveformCursorPrecise,
        Self::SeekWaveform,
        Self::SetWaveformCursor,
        Self::BeginWaveformSelectionAt,
        Self::SetWaveformSelectionRange,
        Self::SetWaveformSelectionRangeSmartScale,
        Self::SetWaveformEditSelectionRange,
        Self::SetWaveformEditFadeInEnd,
        Self::SetWaveformEditFadeInMuteStart,
        Self::SetWaveformEditFadeInCurve,
        Self::SetWaveformEditFadeOutStart,
        Self::SetWaveformEditFadeOutMuteEnd,
        Self::SetWaveformEditFadeOutCurve,
        Self::FinishWaveformEditFadeDrag,
        Self::StartWaveformSelectionDrag,
        Self::UpdateWaveformSelectionDrag,
        Self::FinishWaveformSelectionDrag,
        Self::FinishWaveformSelectionRangeDrag,
        Self::FinishWaveformSelectionSmartScaleDrag,
        Self::BeginWaveformSelectionShift,
        Self::BeginWaveformEditSelectionShift,
        Self::FinishWaveformEditSelectionDrag,
        Self::ClearWaveformSelection,
        Self::ClearWaveformEditSelection,
        Self::ClearWaveformSelections,
        Self::SetWaveformViewCenter,
        Self::ZoomWaveform,
        Self::ZoomWaveformToSelection,
        Self::ZoomWaveformFull,
        Self::Undo,
        Self::Redo,
        Self::CheckForUpdates,
        Self::OpenUpdateLink,
        Self::InstallUpdate,
        Self::DismissUpdate,
    ];
}
