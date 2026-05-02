//! Sempal-owned native runtime action DTOs.
//!
//! These actions describe Sempal user intent inside app-core. Radiant still
//! emits and consumes a compatibility copy at the runtime boundary, so this
//! module keeps narrow adapters without making Radiant the owner of Sempal
//! dispatch payloads.

use super::native_shell_dtos::{FolderPaneIdModel, PlaybackAgeFilterChip};
use radiant::compat::legacy_shell as compat;
use serde::{Deserialize, Serialize};

/// Triage targets used by native browser action surfaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrowserTagTarget {
    /// Move selected/focused rows to trash.
    Trash,
    /// Set selected/focused rows to neutral.
    Neutral,
    /// Mark selected/focused rows as keep.
    Keep,
}

/// Action emitted by the native runtime input layer.
#[cfg_attr(not(test), derive(PartialEq, Eq))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UiAction {
    // Column / triage compatibility actions.
    /// Select a target triage/browser column.
    SelectColumn {
        /// Target column index in the visible triage column set.
        index: usize,
    },
    /// Move column focus left/right.
    MoveColumn {
        /// Signed column delta (`-1` for left, `+1` for right).
        delta: i8,
    },

    // Transport and global playback actions.
    /// Toggle transport playback state.
    ToggleTransport,
    /// Replay the stored compare-anchor sample without changing browser focus.
    PlayCompareAnchor,
    /// Start playback from the beginning of the active sample.
    PlayFromStart,
    /// Start playback from the current playhead or cursor position.
    PlayFromCurrentPlayhead,
    /// Start playback from the current waveform cursor position.
    ///
    /// Plain waveform click-release uses this action so playback starts from
    /// the exact clicked point instead of reusing any older visible playhead
    /// position.
    PlayFromWaveformCursor,
    /// Start playback immediately from one exact waveform position.
    ///
    /// Plain waveform click-release uses this direct action so the host can
    /// seek and start playback from the clicked point in one step without
    /// inferring intent from the cursor or visible playhead state.
    PlayWaveformAtPrecise {
        /// Normalized nanounit playback target (`0..=1_000_000_000`).
        position_nanos: u32,
    },
    /// Handle Escape key behavior for playback, selection, and cursor cleanup.
    HandleEscape,

    // Focus and shell-surface actions.
    /// Focus the browser/list panel.
    FocusBrowserPanel,
    /// Focus the sources panel.
    FocusSourcesPanel,
    /// Focus the waveform panel.
    FocusWaveformPanel,
    /// Focus the folder browser section inside the sources panel.
    FocusFolderPanel {
        /// Pane that should become active, or `None` for the current active pane.
        pane: Option<FolderPaneIdModel>,
    },
    /// Focus the currently loaded sample in the browser.
    FocusLoadedSampleInBrowser,
    /// Focus the browser search field.
    FocusBrowserSearch,
    /// Clear browser-search focus while preserving the current query text.
    BlurBrowserSearch,
    /// Open the source-add file dialog.
    OpenAddSourceDialog,
    /// Open the native options menu.
    OpenOptionsMenu,
    /// Close the native options panel.
    CloseOptionsPanel,
    /// Open a folder picker for the configured trash destination.
    PickTrashFolder,
    /// Open the configured trash folder in the OS file explorer.
    OpenTrashFolder,
    /// Open the default-identifier prompt inside the options panel.
    EditDefaultIdentifier,
    /// Return from one audio picker to the main options overview.
    ShowOptionsOverview,
    /// Expand the output-host picker inside the options panel.
    OpenAudioOutputHostPicker,
    /// Expand the output-device picker inside the options panel.
    OpenAudioOutputDevicePicker,
    /// Expand the output sample-rate picker inside the options panel.
    OpenAudioOutputSampleRatePicker,
    /// Expand the input-host picker inside the options panel.
    OpenAudioInputHostPicker,
    /// Expand the input-device picker inside the options panel.
    OpenAudioInputDevicePicker,
    /// Expand the input sample-rate picker inside the options panel.
    OpenAudioInputSampleRatePicker,
    /// Apply one output-host selection.
    SetAudioOutputHost {
        /// Selected host identifier, or `None` for the system default.
        host_id: Option<String>,
    },
    /// Apply one output-device selection.
    SetAudioOutputDevice {
        /// Selected device name, or `None` for the host default.
        device_name: Option<String>,
    },
    /// Apply one output sample-rate selection.
    SetAudioOutputSampleRate {
        /// Selected sample rate in Hz, or `None` for the device default.
        sample_rate: Option<u32>,
    },
    /// Apply one input-host selection.
    SetAudioInputHost {
        /// Selected host identifier, or `None` for the system default.
        host_id: Option<String>,
    },
    /// Apply one input-device selection.
    SetAudioInputDevice {
        /// Selected device name, or `None` for the host default.
        device_name: Option<String>,
    },
    /// Apply one input sample-rate selection.
    SetAudioInputSampleRate {
        /// Selected sample rate in Hz, or `None` for the device default.
        sample_rate: Option<u32>,
    },
    /// Focus the source-folder search field.
    FocusFolderSearch {
        /// Pane whose folder-search field should receive focus, or `None` for the active pane.
        pane: Option<FolderPaneIdModel>,
    },
    /// Set folder search query.
    SetFolderSearch {
        /// Pane whose folder-search query changed, or `None` for the active pane.
        pane: Option<FolderPaneIdModel>,
        /// Full folder-search query text.
        query: String,
    },
    /// Toggle whether the folder tree shows disk folders without WAV-backed samples.
    ToggleShowAllFolders {
        /// Pane whose folder-visibility toggle was activated, or `None` for the active pane.
        pane: Option<FolderPaneIdModel>,
    },
    /// Toggle whether folder filtering includes descendant files.
    ToggleFolderFlattenedView {
        /// Pane whose flattened-view toggle was activated, or `None` for the active pane.
        pane: Option<FolderPaneIdModel>,
    },

    // Sources and folder tree actions.
    /// Focus a source row by index and make the sources list the active section.
    FocusSourceRow {
        /// Pane containing the target source selector, or `None` for the active pane.
        pane: Option<FolderPaneIdModel>,
        /// Target source row index.
        index: usize,
    },
    /// Select a source row by index.
    SelectSourceRow {
        /// Pane containing the target source selector, or `None` for the active pane.
        pane: Option<FolderPaneIdModel>,
        /// Target source row index.
        index: usize,
    },
    /// Move focused source selection by row delta.
    MoveSourceFocus {
        /// Signed row delta applied to the focused source selection.
        delta: i8,
    },
    /// Reload wav entries for the focused source row.
    ReloadFocusedSourceRow,
    /// Run a hard sync/rescan for the focused source row.
    HardSyncFocusedSourceRow,
    /// Open the focused source folder in the system file manager.
    OpenFocusedSourceFolder,
    /// Remove the currently focused source row.
    RemoveFocusedSourceRow,
    /// Reload wav entries for one source row.
    ReloadSourceRow {
        /// Pane containing the target source selector, or `None` for the active pane.
        pane: Option<FolderPaneIdModel>,
        /// Target source row index.
        index: usize,
    },
    /// Run a hard sync/rescan for one source row.
    HardSyncSourceRow {
        /// Pane containing the target source selector, or `None` for the active pane.
        pane: Option<FolderPaneIdModel>,
        /// Target source row index.
        index: usize,
    },
    /// Open one source row folder in the system file manager.
    OpenSourceFolderRow {
        /// Pane containing the target source selector, or `None` for the active pane.
        pane: Option<FolderPaneIdModel>,
        /// Target source row index.
        index: usize,
    },
    /// Remove one configured source row.
    RemoveSourceRow {
        /// Pane containing the target source selector, or `None` for the active pane.
        pane: Option<FolderPaneIdModel>,
        /// Target source row index.
        index: usize,
    },
    /// Focus a folder row by index.
    FocusFolderRow {
        /// Pane containing the target folder row, or `None` for the active pane.
        pane: Option<FolderPaneIdModel>,
        /// Target folder row index.
        index: usize,
    },
    /// Activate one folder row using the default row-click behavior.
    ///
    /// Hosts use this combined action for pointer clicks that should keep the
    /// existing folder-filter selection behavior while also toggling expansion
    /// for expandable non-root rows outside folder-search mode.
    ActivateFolderRow {
        /// Pane containing the target folder row, or `None` for the active pane.
        pane: Option<FolderPaneIdModel>,
        /// Target folder row index.
        index: usize,
    },
    /// Toggle expansion for one folder row without changing selection semantics.
    ToggleFolderRowExpanded {
        /// Pane containing the target folder row, or `None` for the active pane.
        pane: Option<FolderPaneIdModel>,
        /// Target folder row index.
        index: usize,
    },
    /// Expand the currently focused folder row when it has children.
    ExpandFocusedFolder,
    /// Collapse the currently focused folder row or focus its parent.
    CollapseFocusedFolder,
    /// Toggle selection for the currently focused folder row.
    ToggleFocusedFolderSelection,
    /// Move folder focus by row delta.
    MoveFolderFocus {
        /// Signed row delta applied to focused folder selection.
        delta: i8,
    },
    /// Create a folder relative to the focused folder.
    StartNewFolder,
    /// Create a folder relative to one specific projected folder row.
    StartNewFolderAtFolderRow {
        /// Pane containing the target folder row, or `None` for the active pane.
        pane: Option<FolderPaneIdModel>,
        /// Backing controller folder row index.
        index: usize,
    },
    /// Create a folder at the source root.
    StartNewFolderAtRoot,
    /// Focus the active inline folder-create input.
    FocusFolderCreateInput,
    /// Update the active inline folder-create input text.
    SetFolderCreateInput {
        /// Folder-create input text after the latest edit.
        value: String,
    },
    /// Confirm the active inline folder-create draft.
    ConfirmFolderCreate,
    /// Cancel the active inline folder-create draft.
    CancelFolderCreate,
    /// Start folder rename flow for the focused folder.
    StartFolderRename,
    /// Delete the currently focused folder.
    DeleteFocusedFolder,
    /// Open the explicit restore flow for retained folder deletes.
    RestoreRetainedFolderDeletes,
    /// Open the explicit purge flow for retained folder deletes.
    PurgeRetainedFolderDeletes,
    /// Clear staged delete recovery log entries.
    ClearFolderDeleteRecoveryLog,

    // Browser navigation, selection, search, and map actions.
    /// Move browser focus by a row delta in the visible list.
    ///
    /// Hosts should treat this as lightweight preview navigation so held-arrow
    /// or wheel stepping can stay responsive across large browser lists.
    MoveBrowserFocus {
        /// Signed visible-row delta for browser focus movement.
        delta: i8,
    },
    /// Scroll the browser viewport to a specific visible-row start without changing selection.
    SetBrowserViewStart {
        /// Target top visible row index for the browser viewport.
        visible_row: usize,
    },
    /// Focus a browser row by visible index.
    FocusBrowserRow {
        /// Target visible row index in the browser list.
        visible_row: usize,
    },
    /// Store the focused browser sample as the compare-anchor reference.
    SetCompareAnchorFromFocusedBrowserSample,
    /// Commit the currently focused browser row as the active loaded sample.
    CommitFocusedBrowserRow,
    /// Save the current waveform selection or slices into the browser as a new sample.
    SaveWaveformSelectionToBrowser,
    /// Save the current waveform selection or slices and mark exported clips keep-2.
    SaveWaveformSelectionToBrowserWithKeep2,
    /// Commit preview fades for the active waveform edit selection.
    CommitWaveformEditFades,
    /// Detect silence-split waveform slices for the loaded sample.
    DetectWaveformSilenceSlices,
    /// Detect near-duplicate windows for the loaded sample using the current selection size.
    DetectWaveformExactDuplicateSlices,
    /// Clean near-duplicate windows while keeping the first occurrence.
    CleanWaveformExactDuplicateSlices,
    /// Toggle browser-row selection by visible index.
    ToggleBrowserRowSelection {
        /// Target visible row index in the browser list.
        visible_row: usize,
    },
    /// Start dragging one browser sample or the active browser multi-selection.
    ///
    /// The runtime emits this only after a browser-row press exceeds drag slop,
    /// so plain clicks can still resolve into the existing focus/selection
    /// actions on release without changing preview behavior.
    StartBrowserSampleDrag {
        /// Target visible row index that armed the drag session.
        visible_row: usize,
        /// Pointer x-position in logical UI coordinates.
        pointer_x: u16,
        /// Pointer y-position in logical UI coordinates.
        pointer_y: u16,
    },
    /// Update the active browser-sample drag with the latest pointer position.
    UpdateBrowserSampleDrag {
        /// Pointer x-position in logical UI coordinates.
        pointer_x: u16,
        /// Pointer y-position in logical UI coordinates.
        pointer_y: u16,
        /// Folder pane currently hovered, when the pointer is over a folder pane.
        hovered_folder_pane: Option<FolderPaneIdModel>,
        /// Backing controller folder-row index currently hovered, when any.
        hovered_folder_row: Option<usize>,
        /// Folder pane currently hovered by the pointer background, when any.
        over_folder_panel: Option<FolderPaneIdModel>,
        /// Whether Shift is currently held.
        shift_down: bool,
        /// Whether Alt is currently held.
        alt_down: bool,
    },
    /// Finish the active browser-sample drag gesture.
    FinishBrowserSampleDrag,
    /// Extend selection from the anchor to the target visible row.
    ExtendBrowserSelectionToRow {
        /// Target visible row index used as selection endpoint.
        visible_row: usize,
    },
    /// Extend selection additively from the anchor to the target visible row.
    AddRangeBrowserSelection {
        /// Target visible row index used as additive selection endpoint.
        visible_row: usize,
    },
    /// Move browser focus and extend selection by a visible-row delta.
    ExtendBrowserSelectionFromFocus {
        /// Signed visible-row delta from current focus.
        delta: i8,
    },
    /// Move browser focus and extend selection additively by a visible-row delta.
    AddRangeBrowserSelectionFromFocus {
        /// Signed visible-row delta from current focus.
        delta: i8,
    },
    /// Toggle selection state for the currently focused browser row.
    ToggleFocusedBrowserRowSelection,
    /// Select every row in the current visible browser list.
    SelectAllBrowserRows,
    /// Set browser search query.
    SetBrowserSearch {
        /// Full browser-search query text.
        query: String,
    },
    /// Toggle one browser rating-filter chip for level `-3..=3`, or `4` for locked keeps.
    ToggleBrowserRatingFilter {
        /// Signed rating level associated with the clicked filter chip.
        level: i8,
        /// Whether the click should activate every filter chip except the clicked one.
        invert: bool,
    },
    /// Toggle one browser playback-age filter chip.
    ToggleBrowserPlaybackAgeFilter {
        /// Playback-age chip associated with the clicked filter chip.
        bucket: PlaybackAgeFilterChip,
        /// Whether the click should activate every playback-age chip except the clicked one.
        invert: bool,
    },
    /// Toggle the session mark for the focused browser row or current multi-selection.
    ToggleBrowserSampleMark,
    /// Toggle whether the browser shows only session-marked samples.
    ToggleBrowserMarkedFilter,
    /// Toggle whether the browser shows samples already named from tags.
    ToggleBrowserTagNamedFilter {
        /// Whether the click should show samples not yet named from tags.
        invert: bool,
    },
    /// Toggle sticky random navigation mode for browser next/previous stepping.
    ToggleRandomNavigationMode,
    /// Toggle the browser-local metadata tag sidebar.
    ToggleBrowserTagSidebar,
    /// Toggle auto-rename for browser metadata sidebar edits.
    ToggleBrowserTagSidebarAutoRename,
    /// Toggle browser duplicate-cleanup mode for the focused browser sample.
    ToggleBrowserDuplicateCleanupMode,
    /// Focus the previous browser sample from focus history.
    FocusPreviousBrowserHistory,
    /// Focus the next browser sample from focus history.
    FocusNextBrowserHistory,
    /// Toggle find-similar mode for the focused browser sample.
    ToggleFindSimilarFocusedSample,
    /// Toggle whether one duplicate-cleanup browser row should be kept.
    ToggleBrowserDuplicateCleanupKeep {
        /// Target visible row index in the browser list.
        visible_row: usize,
    },
    /// Confirm duplicate cleanup and trash every unkept duplicate.
    ConfirmBrowserDuplicateCleanup,
    /// Play a random visible sample.
    PlayRandomSample,
    /// Replay the previous random-visible sample.
    PlayPreviousRandomSample,
    /// Adjust the rating for selected browser rows by a signed delta.
    AdjustSelectedBrowserRating {
        /// Signed rating delta applied to selected rows.
        delta: i8,
    },
    /// Set active browser tab (`map = true` selects map; otherwise list).
    SetBrowserTab {
        /// Whether to switch to map tab (`true`) or list tab (`false`).
        map: bool,
    },
    /// Focus the browser metadata tag input field.
    FocusBrowserTagSidebarInput,
    /// Set the browser metadata tag input value.
    SetBrowserTagSidebarInput {
        /// Full tag input text.
        value: String,
    },
    /// Commit the browser metadata tag input value.
    CommitBrowserTagSidebarInput,
    /// Apply one playback-type value to the browser selection.
    SetBrowserSidebarLooped {
        /// Playback type to apply.
        looped: bool,
    },
    /// Toggle one normal tag candidate for the browser selection.
    ToggleBrowserSidebarNormalTag {
        /// Normal tag label to assign or remove.
        label: String,
    },
    /// Focus a specific map sample by stable sample id.
    FocusMapSample {
        /// Stable sample identifier used by map hit-testing.
        sample_id: String,
    },

    // Prompt, rename, and confirmation actions.
    /// Set editable text for the active prompt input field.
    SetPromptInput {
        /// Prompt input text after edit.
        value: String,
    },
    /// Start inline rename flow for the focused browser row.
    StartBrowserRename,
    /// Confirm the currently pending browser rename prompt.
    ConfirmBrowserRename,
    /// Cancel the currently pending browser rename prompt.
    CancelBrowserRename,
    /// Run deterministic auto rename for the active browser selection snapshot.
    AutoRenameBrowserSelection {
        /// Optional visible row that should join the current multi-selection.
        visible_row: Option<usize>,
    },
    /// Apply a triage tag to focused/selected browser rows.
    TagBrowserSelection {
        /// Triage bucket applied to focused/selected browser rows.
        target: BrowserTagTarget,
    },
    /// Delete focused/selected browser rows.
    DeleteBrowserSelection,
    /// Normalize the focused browser sample in-place.
    NormalizeFocusedBrowserSample,
    /// Normalize the waveform selection, or the loaded sample when no selection is active.
    NormalizeWaveformSelectionOrSample,
    /// Crop the waveform file down to the active selection.
    CropWaveformSelection,
    /// Write the active waveform selection to a new sibling sample file.
    CropWaveformSelectionToNewSample,
    /// Trim the active waveform selection out of the loaded file.
    TrimWaveformSelection,
    /// Reverse the active waveform selection.
    ReverseWaveformSelection,
    /// Fade the active waveform selection from left to right.
    FadeWaveformSelectionLeftToRight,
    /// Fade the active waveform selection from right to left.
    FadeWaveformSelectionRightToLeft,
    /// Mute the active waveform selection or merge selected slices in slice mode.
    MuteWaveformSelection,
    /// Delete the selected slice markers.
    DeleteSelectedSliceMarkers,
    /// Toggle selection for one detected silence-split waveform slice.
    ToggleWaveformSliceSelection {
        /// Zero-based slice index within the current preview batch.
        index: usize,
    },
    /// Focus and audition one duplicate-cleanup preview slice.
    AuditionWaveformDuplicateSlice {
        /// Zero-based duplicate preview index within the current cleanup batch.
        index: usize,
    },
    /// Toggle whether one duplicate-cleanup preview should be kept.
    ToggleWaveformDuplicateSliceExemption {
        /// Zero-based duplicate preview index within the current cleanup batch.
        index: usize,
    },
    /// Move the focused review slice by one signed step.
    MoveWaveformSliceFocus {
        /// Signed slice delta (`-1` for previous, `+1` for next).
        delta: i8,
    },
    /// Toggle export marking for the currently focused review slice.
    ToggleFocusedWaveformSliceExportMark,
    /// Align the waveform start marker to the latest hover marker.
    AlignWaveformStartToMarker,
    /// Delete the currently loaded sample and navigate to the next candidate.
    DeleteLoadedWaveformSample,
    /// Slide the active waveform selection by one coarse or fine step.
    SlideWaveformSelection {
        /// Signed selection slide delta (`-1` for left, `+1` for right).
        delta: i8,
        /// Whether the slide should use the fine nudge step.
        fine: bool,
    },
    /// Confirm the currently visible modal prompt.
    ConfirmPrompt,
    /// Cancel the currently visible modal prompt.
    CancelPrompt,
    /// Request cancellation of the active progress operation.
    CancelProgress,
    /// Copy the current browser sample file(s) or waveform selection clip to the clipboard.
    ///
    /// Hosts keep this action context-sensitive:
    /// - browser focus copies the selected or focused source file paths
    /// - waveform focus copies the current exported selection clip, when any
    CopySelectionToClipboard,
    /// Toggle the hotkey/help overlay.
    ToggleHotkeyOverlay,
    /// Copy the status log to the clipboard.
    CopyStatusLog,
    /// Open the feedback-issue prompt flow.
    OpenFeedbackIssuePrompt,
    /// Move all trashed samples into the configured trash folder.
    MoveTrashedSamplesToFolder,

    // Options and persistent interaction toggles.
    /// Enable/disable input monitoring.
    SetInputMonitoringEnabled {
        /// Target enabled state.
        enabled: bool,
    },
    /// Enable/disable rating-based browser auto-advance.
    SetAdvanceAfterRatingEnabled {
        /// Target enabled state.
        enabled: bool,
    },
    /// Enable/disable destructive edit confirmations.
    SetDestructiveYoloMode {
        /// Target enabled state.
        enabled: bool,
    },
    /// Enable/disable inverted waveform scrolling.
    SetInvertWaveformScroll {
        /// Target enabled state.
        enabled: bool,
    },
    /// Toggle loop-playback state.
    ToggleLoopPlayback,
    /// Enter or cycle the locked loop override across sample changes.
    ToggleLoopLock,
    /// Set waveform channel view mode.
    SetWaveformChannelView {
        /// When true, uses split stereo mode; otherwise mono mode.
        stereo: bool,
    },
    /// Enable/disable normalized audition playback.
    SetNormalizedAuditionEnabled {
        /// Target enabled state.
        enabled: bool,
    },
    /// Enable/disable BPM snapping for waveform edits.
    SetBpmSnapEnabled {
        /// Target enabled state.
        enabled: bool,
    },
    /// Enable/disable selection-relative BPM grid anchoring.
    SetRelativeBpmGridEnabled {
        /// Target enabled state.
        enabled: bool,
    },
    /// Adjust waveform BPM by a signed whole-number delta.
    AdjustWaveformBpm {
        /// Signed BPM delta applied to the current value.
        delta: i8,
    },
    /// Set waveform BPM to an explicit positive numeric value.
    SetWaveformBpmValue {
        /// Absolute BPM value in tenths (`1200` = `120.0 BPM`).
        value_tenths: u16,
    },
    /// Enable/disable transient snapping for waveform edits.
    SetTransientSnapEnabled {
        /// Target enabled state.
        enabled: bool,
    },
    /// Enable/disable transient marker visibility.
    SetTransientMarkersEnabled {
        /// Target enabled state.
        enabled: bool,
    },
    /// Toggle transient marker visibility.
    ToggleTransientMarkers,
    /// Toggle BPM snapping for waveform edits.
    ToggleBpmSnap,
    /// Enable/disable slice mode.
    SetSliceModeEnabled {
        /// Target enabled state.
        enabled: bool,
    },
    /// Set output volume to a normalized milli value (`0..=1000`).
    SetVolume {
        /// Normalized milli volume value (`0..=1000`).
        value_milli: u16,
    },
    /// Persist the current volume setting after a drag/continuous edit.
    CommitVolumeSetting,

    // Waveform transport, edit, and gesture actions.
    /// Seek waveform/playhead to a normalized nanounit position (`0..=1_000_000_000`).
    SeekWaveformPrecise {
        /// Normalized nanounit target position (`0..=1_000_000_000`).
        position_nanos: u32,
    },
    /// Set waveform cursor to a normalized nanounit position (`0..=1_000_000_000`).
    SetWaveformCursorPrecise {
        /// Normalized nanounit cursor position (`0..=1_000_000_000`).
        position_nanos: u32,
    },
    /// Seek waveform/playhead to a normalized milli position (`0..=1000`).
    ///
    /// This compatibility action is retained for older callers and is upgraded
    /// to the precise nanounit path at the host boundary.
    SeekWaveform {
        /// Normalized milli target position (`0..=1000`).
        position_milli: u16,
    },
    /// Set waveform cursor to a normalized milli position (`0..=1000`).
    ///
    /// This compatibility action is retained for older callers and is upgraded
    /// to the precise nanounit path at the host boundary.
    SetWaveformCursor {
        /// Normalized milli cursor position (`0..=1000`).
        position_milli: u16,
    },
    /// Arm a new playback-selection drag from one exact anchor point.
    ///
    /// The runtime routes plain waveform press through this action first, but
    /// only commits the selection once the pointer moves far enough to exceed
    /// click slop. This preserves the initial click anchor exactly, even when
    /// BPM snapping or an older selection is active.
    BeginWaveformSelectionAt {
        /// Exact anchor position in normalized micro-units.
        anchor_micros: u32,
    },
    /// Arm a new playback-selection drag from one exact nanounit anchor point.
    BeginWaveformSelectionAtPrecise {
        /// Exact anchor position in normalized nanounits.
        anchor_nanos: u32,
    },
    /// Begin one circular waveform-slide gesture from an exact anchor point.
    ///
    /// Hosts use this for wrap-drag sample rotation: while the gesture is
    /// active, pointer motion rotates the waveform preview in a wrapping
    /// manner, and release commits the rotated sample to disk.
    BeginWaveformCircularSlide {
        /// Exact anchor position in normalized micro-units.
        anchor_micros: u32,
    },
    /// Update an active circular waveform-slide gesture.
    UpdateWaveformCircularSlide {
        /// Current pointer position in normalized micro-units.
        position_micros: u32,
    },
    /// Finish an active circular waveform-slide gesture.
    FinishWaveformCircularSlide,
    /// Set waveform selection bounds in normalized micro space (`0..=1_000_000`).
    SetWaveformSelectionRange {
        /// Selection start position in normalized micro-units.
        start_micros: u32,
        /// Selection end position in normalized micro-units.
        end_micros: u32,
        /// When true, bypass BPM snapping for this playback drag update.
        ///
        /// Native waveform drags set this while Alt is held so the active
        /// playback selection can move freely until Alt is released again.
        snap_override: bool,
        /// When true, keep an out-of-bounds drag clamped to the current viewport edge
        /// instead of BPM-snapping that edge back inward.
        preserve_view_edge: bool,
    },
    /// Set waveform selection bounds in normalized nano space (`0..=1_000_000_000`).
    SetWaveformSelectionRangePrecise {
        /// Selection start position in normalized nanounits.
        start_nanos: u32,
        /// Selection end position in normalized nanounits.
        end_nanos: u32,
        /// When true, bypass BPM snapping for this playback drag update.
        snap_override: bool,
        /// When true, keep an out-of-bounds drag clamped to the current viewport edge.
        preserve_view_edge: bool,
    },
    /// Set waveform selection bounds without BPM snapping and recalculate BPM for a 4-beat span.
    SetWaveformSelectionRangeSmartScale {
        /// Selection anchor/start position in normalized micro-units.
        start_micros: u32,
        /// Selection dragged edge position in normalized micro-units.
        end_micros: u32,
    },
    /// Set waveform selection bounds with nano precision and smart-scale BPM behavior.
    SetWaveformSelectionRangeSmartScalePrecise {
        /// Selection anchor/start position in normalized nanounits.
        start_nanos: u32,
        /// Selection dragged edge position in normalized nanounits.
        end_nanos: u32,
    },
    /// Set waveform edit-selection bounds in normalized micro space (`0..=1_000_000`).
    SetWaveformEditSelectionRange {
        /// Edit-selection start position in normalized micro-units.
        start_micros: u32,
        /// Edit-selection end position in normalized micro-units.
        end_micros: u32,
        /// When true, keep an out-of-bounds drag clamped to the current viewport edge
        /// instead of BPM-snapping that edge back inward.
        preserve_view_edge: bool,
    },
    /// Set waveform edit-selection bounds in normalized nano space (`0..=1_000_000_000`).
    SetWaveformEditSelectionRangePrecise {
        /// Edit-selection start position in normalized nanounits.
        start_nanos: u32,
        /// Edit-selection end position in normalized nanounits.
        end_nanos: u32,
        /// When true, keep an out-of-bounds drag clamped to the current viewport edge.
        preserve_view_edge: bool,
    },
    /// Set the edit fade-in end handle in normalized micro space (`0..=1_000_000`).
    SetWaveformEditFadeInEnd {
        /// Fade-in end handle position in normalized micro-units.
        position_micros: u32,
    },
    /// Set the edit fade-in mute start handle in normalized micro space (`0..=1_000_000`).
    SetWaveformEditFadeInMuteStart {
        /// Fade-in mute-start handle position in normalized micro-units.
        position_micros: u32,
    },
    /// Set the edit fade-in curve tension in normalized milli space (`0..=1000`).
    SetWaveformEditFadeInCurve {
        /// Fade-in curve value in normalized milli-units.
        curve_milli: u16,
    },
    /// Set the edit fade-out start handle in normalized micro space (`0..=1_000_000`).
    SetWaveformEditFadeOutStart {
        /// Fade-out start handle position in normalized micro-units.
        position_micros: u32,
    },
    /// Set the edit fade-out mute end handle in normalized micro space (`0..=1_000_000`).
    SetWaveformEditFadeOutMuteEnd {
        /// Fade-out mute-end handle position in normalized micro-units.
        position_micros: u32,
    },
    /// Set the edit fade-out curve tension in normalized milli space (`0..=1000`).
    SetWaveformEditFadeOutCurve {
        /// Fade-out curve value in normalized milli-units.
        curve_milli: u16,
    },
    /// Finish an active waveform edit-fade drag gesture.
    FinishWaveformEditFadeDrag,
    /// Start dragging the current waveform playback selection from its drag handle.
    StartWaveformSelectionDrag {
        /// Pointer x-position in logical UI coordinates.
        pointer_x: u16,
        /// Pointer y-position in logical UI coordinates.
        pointer_y: u16,
    },
    /// Update the active waveform-selection drag with the latest pointer position.
    UpdateWaveformSelectionDrag {
        /// Pointer x-position in logical UI coordinates.
        pointer_x: u16,
        /// Pointer y-position in logical UI coordinates.
        pointer_y: u16,
        /// Folder pane currently hovered, when the pointer is over a folder pane.
        hovered_folder_pane: Option<FolderPaneIdModel>,
        /// Backing controller folder-row index currently hovered, when any.
        hovered_folder_row: Option<usize>,
        /// Folder pane currently hovered by the pointer background, when any.
        over_folder_panel: Option<FolderPaneIdModel>,
        /// Whether the pointer currently hovers the sample browser list.
        over_browser_list: bool,
        /// Whether Shift is currently held.
        shift_down: bool,
        /// Whether Alt is currently held.
        alt_down: bool,
    },
    /// Finish the active waveform-selection drag gesture.
    FinishWaveformSelectionDrag,
    /// Finish an active playback-selection range drag gesture.
    ///
    /// This covers plain create, resize, and shift gestures that mutate the
    /// playback-selection range directly instead of using the export drag flow.
    FinishWaveformSelectionRangeDrag,
    /// Finish the active alt-resize smart-scale gesture and commit the inferred BPM.
    FinishWaveformSelectionSmartScaleDrag,
    /// Arm a playback-selection translate gesture from the bottom-center handle.
    BeginWaveformSelectionShift {
        /// Pointer micro position captured at press time.
        pointer_micros: u32,
        /// Selection start preserved across the translate gesture.
        start_micros: u32,
        /// Selection end preserved across the translate gesture.
        end_micros: u32,
    },
    /// Arm a playback-selection translate gesture from the bottom-center handle with nano precision.
    BeginWaveformSelectionShiftPrecise {
        /// Pointer nanounit position captured at press time.
        pointer_nanos: u32,
        /// Selection start preserved across the translate gesture.
        start_nanos: u32,
        /// Selection end preserved across the translate gesture.
        end_nanos: u32,
    },
    /// Arm an edit-selection translate gesture from the bottom-center handle.
    BeginWaveformEditSelectionShift {
        /// Pointer micro position captured at press time.
        pointer_micros: u32,
        /// Edit-selection start preserved across the translate gesture.
        start_micros: u32,
        /// Edit-selection end preserved across the translate gesture.
        end_micros: u32,
    },
    /// Arm an edit-selection translate gesture from the bottom-center handle with nano precision.
    BeginWaveformEditSelectionShiftPrecise {
        /// Pointer nanounit position captured at press time.
        pointer_nanos: u32,
        /// Edit-selection start preserved across the translate gesture.
        start_nanos: u32,
        /// Edit-selection end preserved across the translate gesture.
        end_nanos: u32,
    },
    /// Finish an active edit-selection range drag gesture.
    ///
    /// This covers plain create, resize, and shift gestures that mutate the
    /// edit-selection range directly.
    FinishWaveformEditSelectionDrag,
    /// Clear active waveform selection.
    ClearWaveformSelection,
    /// Clear active waveform edit selection.
    ClearWaveformEditSelection,
    /// Clear both active waveform selection types from one pointer gesture.
    ClearWaveformSelections,
    /// Scroll the waveform viewport to a normalized center position in micros.
    SetWaveformViewCenter {
        /// Target center point within the full waveform (`0..=1_000_000`).
        center_micros: u32,
        /// Optional exact center point within the full waveform (`0..=1_000_000_000`).
        ///
        /// Native input supplies this at deep zoom so viewport gestures keep
        /// sub-micro precision instead of collapsing to the nearest micro.
        center_nanos: Option<u32>,
    },
    /// Zoom waveform view by discrete steps.
    ZoomWaveform {
        /// When true, zooms in; otherwise zooms out.
        zoom_in: bool,
        /// Number of discrete zoom steps to apply.
        steps: u8,
        /// Optional high-precision hover anchor ratio within current waveform view.
        ///
        /// Values are stored in micros (`0..=1_000_000`) to preserve deterministic
        /// equality semantics while keeping pointer-anchored zoom stable at deep zoom.
        anchor_ratio_micros: Option<u32>,
    },
    /// Fit waveform view to the active selection.
    ZoomWaveformToSelection,
    /// Reset waveform view to full-range (`0..=1000`).
    ZoomWaveformFull,

    // Global history and update actions.
    /// Trigger undo.
    Undo,
    /// Trigger redo.
    Redo,
    /// Trigger an explicit update check.
    CheckForUpdates,
    /// Open the available update URL.
    OpenUpdateLink,
    /// Install update and exit where supported.
    InstallUpdate,
    /// Dismiss current update notification.
    DismissUpdate,
}

#[cfg(test)]
impl PartialEq for UiAction {
    fn eq(&self, other: &Self) -> bool {
        waveform_precision_equivalent(self, other)
            || serde_json::to_value(self).ok() == serde_json::to_value(other).ok()
    }
}

#[cfg(test)]
impl Eq for UiAction {}

#[cfg(test)]
fn waveform_precision_equivalent(left: &UiAction, right: &UiAction) -> bool {
    use UiAction::*;

    match (left, right) {
        (
            BeginWaveformSelectionAt { anchor_micros },
            BeginWaveformSelectionAtPrecise { anchor_nanos },
        )
        | (
            BeginWaveformSelectionAtPrecise { anchor_nanos },
            BeginWaveformSelectionAt { anchor_micros },
        ) => nanos_match_micros(*anchor_nanos, *anchor_micros),
        (
            SetWaveformSelectionRange {
                start_micros,
                end_micros,
                snap_override,
                preserve_view_edge,
            },
            SetWaveformSelectionRangePrecise {
                start_nanos,
                end_nanos,
                snap_override: precise_snap_override,
                preserve_view_edge: precise_preserve_view_edge,
            },
        )
        | (
            SetWaveformSelectionRangePrecise {
                start_nanos,
                end_nanos,
                snap_override: precise_snap_override,
                preserve_view_edge: precise_preserve_view_edge,
            },
            SetWaveformSelectionRange {
                start_micros,
                end_micros,
                snap_override,
                preserve_view_edge,
            },
        ) => {
            snap_override == precise_snap_override
                && preserve_view_edge == precise_preserve_view_edge
                && nanos_match_micros(*start_nanos, *start_micros)
                && nanos_match_micros(*end_nanos, *end_micros)
        }
        (
            SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            },
            SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos,
                end_nanos,
            },
        )
        | (
            SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos,
                end_nanos,
            },
            SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            },
        ) => {
            nanos_match_micros(*start_nanos, *start_micros)
                && nanos_match_micros(*end_nanos, *end_micros)
        }
        (
            BeginWaveformSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            },
            BeginWaveformSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            },
        )
        | (
            BeginWaveformSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            },
            BeginWaveformSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            },
        ) => {
            nanos_match_micros(*pointer_nanos, *pointer_micros)
                && nanos_match_micros(*start_nanos, *start_micros)
                && nanos_match_micros(*end_nanos, *end_micros)
        }
        (
            SetWaveformEditSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge,
            },
            SetWaveformEditSelectionRangePrecise {
                start_nanos,
                end_nanos,
                preserve_view_edge: precise_preserve_view_edge,
            },
        )
        | (
            SetWaveformEditSelectionRangePrecise {
                start_nanos,
                end_nanos,
                preserve_view_edge: precise_preserve_view_edge,
            },
            SetWaveformEditSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge,
            },
        ) => {
            preserve_view_edge == precise_preserve_view_edge
                && nanos_match_micros(*start_nanos, *start_micros)
                && nanos_match_micros(*end_nanos, *end_micros)
        }
        (
            BeginWaveformEditSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            },
            BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            },
        )
        | (
            BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            },
            BeginWaveformEditSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            },
        ) => {
            nanos_match_micros(*pointer_nanos, *pointer_micros)
                && nanos_match_micros(*start_nanos, *start_micros)
                && nanos_match_micros(*end_nanos, *end_micros)
        }
        _ => false,
    }
}

#[cfg(test)]
fn nanos_match_micros(nanos: u32, micros: u32) -> bool {
    nanos == micros
        || ((nanos.min(1_000_000_000) + 500) / 1000).min(1_000_000) == micros.min(1_000_000)
}

impl From<compat::BrowserTagTarget> for BrowserTagTarget {
    fn from(value: compat::BrowserTagTarget) -> Self {
        match value {
            compat::BrowserTagTarget::Negative => Self::Trash,
            compat::BrowserTagTarget::Neutral => Self::Neutral,
            compat::BrowserTagTarget::Positive => Self::Keep,
        }
    }
}

impl From<BrowserTagTarget> for compat::BrowserTagTarget {
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
            compat::UiAction::SelectColumn { index } => Self::SelectColumn { index: index },
            compat::UiAction::MoveColumn { delta } => Self::MoveColumn { delta: delta },
            compat::UiAction::ToggleTransport => Self::ToggleTransport,
            compat::UiAction::PlayCompareAnchor => Self::PlayCompareAnchor,
            compat::UiAction::PlayFromStart => Self::PlayFromStart,
            compat::UiAction::PlayFromCurrentPlayhead => Self::PlayFromCurrentPlayhead,
            compat::UiAction::PlayFromWaveformCursor => Self::PlayFromWaveformCursor,
            compat::UiAction::PlayWaveformAtPrecise { position_nanos } => {
                Self::PlayWaveformAtPrecise {
                    position_nanos: position_nanos,
                }
            }
            compat::UiAction::HandleEscape => Self::HandleEscape,
            compat::UiAction::FocusBrowserPanel => Self::FocusBrowserPanel,
            compat::UiAction::FocusSourcesPanel => Self::FocusSourcesPanel,
            compat::UiAction::FocusWaveformPanel => Self::FocusWaveformPanel,
            compat::UiAction::FocusFolderPanel { pane } => Self::FocusFolderPanel {
                pane: pane.map(Into::into),
            },
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
            compat::UiAction::OpenAudioOutputHostPicker => Self::OpenAudioOutputHostPicker,
            compat::UiAction::OpenAudioOutputDevicePicker => Self::OpenAudioOutputDevicePicker,
            compat::UiAction::OpenAudioOutputSampleRatePicker => {
                Self::OpenAudioOutputSampleRatePicker
            }
            compat::UiAction::OpenAudioInputHostPicker => Self::OpenAudioInputHostPicker,
            compat::UiAction::OpenAudioInputDevicePicker => Self::OpenAudioInputDevicePicker,
            compat::UiAction::OpenAudioInputSampleRatePicker => {
                Self::OpenAudioInputSampleRatePicker
            }
            compat::UiAction::SetAudioOutputHost { host_id } => {
                Self::SetAudioOutputHost { host_id: host_id }
            }
            compat::UiAction::SetAudioOutputDevice { device_name } => Self::SetAudioOutputDevice {
                device_name: device_name,
            },
            compat::UiAction::SetAudioOutputSampleRate { sample_rate } => {
                Self::SetAudioOutputSampleRate {
                    sample_rate: sample_rate,
                }
            }
            compat::UiAction::SetAudioInputHost { host_id } => {
                Self::SetAudioInputHost { host_id: host_id }
            }
            compat::UiAction::SetAudioInputDevice { device_name } => Self::SetAudioInputDevice {
                device_name: device_name,
            },
            compat::UiAction::SetAudioInputSampleRate { sample_rate } => {
                Self::SetAudioInputSampleRate {
                    sample_rate: sample_rate,
                }
            }
            compat::UiAction::FocusFolderSearch { pane } => Self::FocusFolderSearch {
                pane: pane.map(Into::into),
            },
            compat::UiAction::SetFolderSearch { pane, query } => Self::SetFolderSearch {
                pane: pane.map(Into::into),
                query: query,
            },
            compat::UiAction::ToggleShowAllFolders { pane } => Self::ToggleShowAllFolders {
                pane: pane.map(Into::into),
            },
            compat::UiAction::ToggleFolderFlattenedView { pane } => {
                Self::ToggleFolderFlattenedView {
                    pane: pane.map(Into::into),
                }
            }
            compat::UiAction::FocusSourceRow { pane, index } => Self::FocusSourceRow {
                pane: pane.map(Into::into),
                index: index,
            },
            compat::UiAction::SelectSourceRow { pane, index } => Self::SelectSourceRow {
                pane: pane.map(Into::into),
                index: index,
            },
            compat::UiAction::MoveSourceFocus { delta } => Self::MoveSourceFocus { delta: delta },
            compat::UiAction::ReloadFocusedSourceRow => Self::ReloadFocusedSourceRow,
            compat::UiAction::HardSyncFocusedSourceRow => Self::HardSyncFocusedSourceRow,
            compat::UiAction::OpenFocusedSourceFolder => Self::OpenFocusedSourceFolder,
            compat::UiAction::RemoveFocusedSourceRow => Self::RemoveFocusedSourceRow,
            compat::UiAction::ReloadSourceRow { pane, index } => Self::ReloadSourceRow {
                pane: pane.map(Into::into),
                index: index,
            },
            compat::UiAction::HardSyncSourceRow { pane, index } => Self::HardSyncSourceRow {
                pane: pane.map(Into::into),
                index: index,
            },
            compat::UiAction::OpenSourceFolderRow { pane, index } => Self::OpenSourceFolderRow {
                pane: pane.map(Into::into),
                index: index,
            },
            compat::UiAction::RemoveSourceRow { pane, index } => Self::RemoveSourceRow {
                pane: pane.map(Into::into),
                index: index,
            },
            compat::UiAction::FocusFolderRow { pane, index } => Self::FocusFolderRow {
                pane: pane.map(Into::into),
                index: index,
            },
            compat::UiAction::ActivateFolderRow { pane, index } => Self::ActivateFolderRow {
                pane: pane.map(Into::into),
                index: index,
            },
            compat::UiAction::ToggleFolderRowExpanded { pane, index } => {
                Self::ToggleFolderRowExpanded {
                    pane: pane.map(Into::into),
                    index: index,
                }
            }
            compat::UiAction::ExpandFocusedFolder => Self::ExpandFocusedFolder,
            compat::UiAction::CollapseFocusedFolder => Self::CollapseFocusedFolder,
            compat::UiAction::ToggleFocusedFolderSelection => Self::ToggleFocusedFolderSelection,
            compat::UiAction::MoveFolderFocus { delta } => Self::MoveFolderFocus { delta: delta },
            compat::UiAction::StartNewFolder => Self::StartNewFolder,
            compat::UiAction::StartNewFolderAtFolderRow { pane, index } => {
                Self::StartNewFolderAtFolderRow {
                    pane: pane.map(Into::into),
                    index: index,
                }
            }
            compat::UiAction::StartNewFolderAtRoot => Self::StartNewFolderAtRoot,
            compat::UiAction::FocusFolderCreateInput => Self::FocusFolderCreateInput,
            compat::UiAction::SetFolderCreateInput { value } => {
                Self::SetFolderCreateInput { value: value }
            }
            compat::UiAction::ConfirmFolderCreate => Self::ConfirmFolderCreate,
            compat::UiAction::CancelFolderCreate => Self::CancelFolderCreate,
            compat::UiAction::StartFolderRename => Self::StartFolderRename,
            compat::UiAction::DeleteFocusedFolder => Self::DeleteFocusedFolder,
            compat::UiAction::RestoreRetainedFolderDeletes => Self::RestoreRetainedFolderDeletes,
            compat::UiAction::PurgeRetainedFolderDeletes => Self::PurgeRetainedFolderDeletes,
            compat::UiAction::ClearFolderDeleteRecoveryLog => Self::ClearFolderDeleteRecoveryLog,
            compat::UiAction::MoveBrowserFocus { delta } => Self::MoveBrowserFocus { delta: delta },
            compat::UiAction::SetBrowserViewStart { visible_row } => Self::SetBrowserViewStart {
                visible_row: visible_row,
            },
            compat::UiAction::FocusBrowserRow { visible_row } => Self::FocusBrowserRow {
                visible_row: visible_row,
            },
            compat::UiAction::SetCompareAnchorFromFocusedBrowserSample => {
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
                Self::ToggleBrowserRowSelection {
                    visible_row: visible_row,
                }
            }
            compat::UiAction::StartBrowserSampleDrag {
                visible_row,
                pointer_x,
                pointer_y,
            } => Self::StartBrowserSampleDrag {
                visible_row: visible_row,
                pointer_x: pointer_x,
                pointer_y: pointer_y,
            },
            compat::UiAction::UpdateBrowserSampleDrag {
                pointer_x,
                pointer_y,
                hovered_folder_pane,
                hovered_folder_row,
                over_folder_panel,
                shift_down,
                alt_down,
            } => Self::UpdateBrowserSampleDrag {
                pointer_x: pointer_x,
                pointer_y: pointer_y,
                hovered_folder_pane: hovered_folder_pane.map(Into::into),
                hovered_folder_row: hovered_folder_row,
                over_folder_panel: over_folder_panel.map(Into::into),
                shift_down: shift_down,
                alt_down: alt_down,
            },
            compat::UiAction::FinishBrowserSampleDrag => Self::FinishBrowserSampleDrag,
            compat::UiAction::ExtendBrowserSelectionToRow { visible_row } => {
                Self::ExtendBrowserSelectionToRow {
                    visible_row: visible_row,
                }
            }
            compat::UiAction::AddRangeBrowserSelection { visible_row } => {
                Self::AddRangeBrowserSelection {
                    visible_row: visible_row,
                }
            }
            compat::UiAction::ExtendBrowserSelectionFromFocus { delta } => {
                Self::ExtendBrowserSelectionFromFocus { delta: delta }
            }
            compat::UiAction::AddRangeBrowserSelectionFromFocus { delta } => {
                Self::AddRangeBrowserSelectionFromFocus { delta: delta }
            }
            compat::UiAction::ToggleFocusedBrowserRowSelection => {
                Self::ToggleFocusedBrowserRowSelection
            }
            compat::UiAction::SelectAllBrowserRows => Self::SelectAllBrowserRows,
            compat::UiAction::SetBrowserSearch { query } => Self::SetBrowserSearch { query: query },
            compat::UiAction::ToggleBrowserRatingFilter { level, invert } => {
                Self::ToggleBrowserRatingFilter {
                    level: level,
                    invert: invert,
                }
            }
            compat::UiAction::ToggleBrowserPlaybackAgeFilter { bucket, invert } => {
                Self::ToggleBrowserPlaybackAgeFilter {
                    bucket: bucket.into(),
                    invert: invert,
                }
            }
            compat::UiAction::ToggleBrowserSampleMark => Self::ToggleBrowserSampleMark,
            compat::UiAction::ToggleBrowserMarkedFilter => Self::ToggleBrowserMarkedFilter,
            compat::UiAction::ToggleBrowserTagNamedFilter { invert } => {
                Self::ToggleBrowserTagNamedFilter { invert: invert }
            }
            compat::UiAction::ToggleRandomNavigationMode => Self::ToggleRandomNavigationMode,
            compat::UiAction::ToggleBrowserTagSidebar => Self::ToggleBrowserTagSidebar,
            compat::UiAction::ToggleBrowserTagSidebarAutoRename => {
                Self::ToggleBrowserTagSidebarAutoRename
            }
            compat::UiAction::ToggleBrowserDuplicateCleanupMode => {
                Self::ToggleBrowserDuplicateCleanupMode
            }
            compat::UiAction::FocusPreviousBrowserHistory => Self::FocusPreviousBrowserHistory,
            compat::UiAction::FocusNextBrowserHistory => Self::FocusNextBrowserHistory,
            compat::UiAction::ToggleFindSimilarFocusedSample => {
                Self::ToggleFindSimilarFocusedSample
            }
            compat::UiAction::ToggleBrowserDuplicateCleanupKeep { visible_row } => {
                Self::ToggleBrowserDuplicateCleanupKeep {
                    visible_row: visible_row,
                }
            }
            compat::UiAction::ConfirmBrowserDuplicateCleanup => {
                Self::ConfirmBrowserDuplicateCleanup
            }
            compat::UiAction::PlayRandomSample => Self::PlayRandomSample,
            compat::UiAction::PlayPreviousRandomSample => Self::PlayPreviousRandomSample,
            compat::UiAction::AdjustSelectedBrowserRating { delta } => {
                Self::AdjustSelectedBrowserRating { delta: delta }
            }
            compat::UiAction::SetBrowserTab { map } => Self::SetBrowserTab { map: map },
            compat::UiAction::FocusBrowserTagSidebarInput => Self::FocusBrowserTagSidebarInput,
            compat::UiAction::SetBrowserTagSidebarInput { value } => {
                Self::SetBrowserTagSidebarInput { value: value }
            }
            compat::UiAction::CommitBrowserTagSidebarInput => Self::CommitBrowserTagSidebarInput,
            compat::UiAction::SetBrowserSidebarLooped { looped } => {
                Self::SetBrowserSidebarLooped { looped: looped }
            }
            compat::UiAction::ToggleBrowserSidebarNormalTag { label } => {
                Self::ToggleBrowserSidebarNormalTag { label: label }
            }
            compat::UiAction::FocusMapSample { sample_id } => Self::FocusMapSample {
                sample_id: sample_id,
            },
            compat::UiAction::SetPromptInput { value } => Self::SetPromptInput { value: value },
            compat::UiAction::StartBrowserRename => Self::StartBrowserRename,
            compat::UiAction::ConfirmBrowserRename => Self::ConfirmBrowserRename,
            compat::UiAction::CancelBrowserRename => Self::CancelBrowserRename,
            compat::UiAction::AutoRenameBrowserSelection { visible_row } => {
                Self::AutoRenameBrowserSelection {
                    visible_row: visible_row,
                }
            }
            compat::UiAction::TagBrowserSelection { target } => Self::TagBrowserSelection {
                target: target.into(),
            },
            compat::UiAction::DeleteBrowserSelection => Self::DeleteBrowserSelection,
            compat::UiAction::NormalizeFocusedBrowserSample => Self::NormalizeFocusedBrowserSample,
            compat::UiAction::NormalizeWaveformSelectionOrSample => {
                Self::NormalizeWaveformSelectionOrSample
            }
            compat::UiAction::CropWaveformSelection => Self::CropWaveformSelection,
            compat::UiAction::CropWaveformSelectionToNewSample => {
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
                Self::ToggleWaveformSliceSelection { index: index }
            }
            compat::UiAction::AuditionWaveformDuplicateSlice { index } => {
                Self::AuditionWaveformDuplicateSlice { index: index }
            }
            compat::UiAction::ToggleWaveformDuplicateSliceExemption { index } => {
                Self::ToggleWaveformDuplicateSliceExemption { index: index }
            }
            compat::UiAction::MoveWaveformSliceFocus { delta } => {
                Self::MoveWaveformSliceFocus { delta: delta }
            }
            compat::UiAction::ToggleFocusedWaveformSliceExportMark => {
                Self::ToggleFocusedWaveformSliceExportMark
            }
            compat::UiAction::AlignWaveformStartToMarker => Self::AlignWaveformStartToMarker,
            compat::UiAction::DeleteLoadedWaveformSample => Self::DeleteLoadedWaveformSample,
            compat::UiAction::SlideWaveformSelection { delta, fine } => {
                Self::SlideWaveformSelection {
                    delta: delta,
                    fine: fine,
                }
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
                Self::SetInputMonitoringEnabled { enabled: enabled }
            }
            compat::UiAction::SetAdvanceAfterRatingEnabled { enabled } => {
                Self::SetAdvanceAfterRatingEnabled { enabled: enabled }
            }
            compat::UiAction::SetDestructiveYoloMode { enabled } => {
                Self::SetDestructiveYoloMode { enabled: enabled }
            }
            compat::UiAction::SetInvertWaveformScroll { enabled } => {
                Self::SetInvertWaveformScroll { enabled: enabled }
            }
            compat::UiAction::ToggleLoopPlayback => Self::ToggleLoopPlayback,
            compat::UiAction::ToggleLoopLock => Self::ToggleLoopLock,
            compat::UiAction::SetWaveformChannelView { stereo } => {
                Self::SetWaveformChannelView { stereo: stereo }
            }
            compat::UiAction::SetNormalizedAuditionEnabled { enabled } => {
                Self::SetNormalizedAuditionEnabled { enabled: enabled }
            }
            compat::UiAction::SetBpmSnapEnabled { enabled } => {
                Self::SetBpmSnapEnabled { enabled: enabled }
            }
            compat::UiAction::SetRelativeBpmGridEnabled { enabled } => {
                Self::SetRelativeBpmGridEnabled { enabled: enabled }
            }
            compat::UiAction::AdjustWaveformBpm { delta } => {
                Self::AdjustWaveformBpm { delta: delta }
            }
            compat::UiAction::SetWaveformBpmValue { value_tenths } => Self::SetWaveformBpmValue {
                value_tenths: value_tenths,
            },
            compat::UiAction::SetTransientSnapEnabled { enabled } => {
                Self::SetTransientSnapEnabled { enabled: enabled }
            }
            compat::UiAction::SetTransientMarkersEnabled { enabled } => {
                Self::SetTransientMarkersEnabled { enabled: enabled }
            }
            compat::UiAction::ToggleTransientMarkers => Self::ToggleTransientMarkers,
            compat::UiAction::ToggleBpmSnap => Self::ToggleBpmSnap,
            compat::UiAction::SetSliceModeEnabled { enabled } => {
                Self::SetSliceModeEnabled { enabled: enabled }
            }
            compat::UiAction::SetVolume { value_milli } => Self::SetVolume {
                value_milli: value_milli,
            },
            compat::UiAction::CommitVolumeSetting => Self::CommitVolumeSetting,
            compat::UiAction::SeekWaveformPrecise { position_nanos } => Self::SeekWaveformPrecise {
                position_nanos: position_nanos,
            },
            compat::UiAction::SetWaveformCursorPrecise { position_nanos } => {
                Self::SetWaveformCursorPrecise {
                    position_nanos: position_nanos,
                }
            }
            compat::UiAction::SeekWaveform { position_milli } => Self::SeekWaveform {
                position_milli: position_milli,
            },
            compat::UiAction::SetWaveformCursor { position_milli } => Self::SetWaveformCursor {
                position_milli: position_milli,
            },
            compat::UiAction::BeginWaveformSelectionAt { anchor_micros } => {
                Self::BeginWaveformSelectionAt {
                    anchor_micros: anchor_micros,
                }
            }
            compat::UiAction::BeginWaveformSelectionAtPrecise { anchor_nanos } => {
                Self::BeginWaveformSelectionAtPrecise {
                    anchor_nanos: anchor_nanos,
                }
            }
            compat::UiAction::BeginWaveformCircularSlide { anchor_micros } => {
                Self::BeginWaveformCircularSlide {
                    anchor_micros: anchor_micros,
                }
            }
            compat::UiAction::UpdateWaveformCircularSlide { position_micros } => {
                Self::UpdateWaveformCircularSlide {
                    position_micros: position_micros,
                }
            }
            compat::UiAction::FinishWaveformCircularSlide => Self::FinishWaveformCircularSlide,
            compat::UiAction::SetWaveformSelectionRange {
                start_micros,
                end_micros,
                snap_override,
                preserve_view_edge,
            } => Self::SetWaveformSelectionRange {
                start_micros: start_micros,
                end_micros: end_micros,
                snap_override: snap_override,
                preserve_view_edge: preserve_view_edge,
            },
            compat::UiAction::SetWaveformSelectionRangePrecise {
                start_nanos,
                end_nanos,
                snap_override,
                preserve_view_edge,
            } => Self::SetWaveformSelectionRangePrecise {
                start_nanos: start_nanos,
                end_nanos: end_nanos,
                snap_override: snap_override,
                preserve_view_edge: preserve_view_edge,
            },
            compat::UiAction::SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            } => Self::SetWaveformSelectionRangeSmartScale {
                start_micros: start_micros,
                end_micros: end_micros,
            },
            compat::UiAction::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos,
                end_nanos,
            } => Self::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos: start_nanos,
                end_nanos: end_nanos,
            },
            compat::UiAction::SetWaveformEditSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge,
            } => Self::SetWaveformEditSelectionRange {
                start_micros: start_micros,
                end_micros: end_micros,
                preserve_view_edge: preserve_view_edge,
            },
            compat::UiAction::SetWaveformEditSelectionRangePrecise {
                start_nanos,
                end_nanos,
                preserve_view_edge,
            } => Self::SetWaveformEditSelectionRangePrecise {
                start_nanos: start_nanos,
                end_nanos: end_nanos,
                preserve_view_edge: preserve_view_edge,
            },
            compat::UiAction::SetWaveformEditFadeInEnd { position_micros } => {
                Self::SetWaveformEditFadeInEnd {
                    position_micros: position_micros,
                }
            }
            compat::UiAction::SetWaveformEditFadeInMuteStart { position_micros } => {
                Self::SetWaveformEditFadeInMuteStart {
                    position_micros: position_micros,
                }
            }
            compat::UiAction::SetWaveformEditFadeInCurve { curve_milli } => {
                Self::SetWaveformEditFadeInCurve {
                    curve_milli: curve_milli,
                }
            }
            compat::UiAction::SetWaveformEditFadeOutStart { position_micros } => {
                Self::SetWaveformEditFadeOutStart {
                    position_micros: position_micros,
                }
            }
            compat::UiAction::SetWaveformEditFadeOutMuteEnd { position_micros } => {
                Self::SetWaveformEditFadeOutMuteEnd {
                    position_micros: position_micros,
                }
            }
            compat::UiAction::SetWaveformEditFadeOutCurve { curve_milli } => {
                Self::SetWaveformEditFadeOutCurve {
                    curve_milli: curve_milli,
                }
            }
            compat::UiAction::FinishWaveformEditFadeDrag => Self::FinishWaveformEditFadeDrag,
            compat::UiAction::StartWaveformSelectionDrag {
                pointer_x,
                pointer_y,
            } => Self::StartWaveformSelectionDrag {
                pointer_x: pointer_x,
                pointer_y: pointer_y,
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
                pointer_x: pointer_x,
                pointer_y: pointer_y,
                hovered_folder_pane: hovered_folder_pane.map(Into::into),
                hovered_folder_row: hovered_folder_row,
                over_folder_panel: over_folder_panel.map(Into::into),
                over_browser_list: over_browser_list,
                shift_down: shift_down,
                alt_down: alt_down,
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
                pointer_micros: pointer_micros,
                start_micros: start_micros,
                end_micros: end_micros,
            },
            compat::UiAction::BeginWaveformSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            } => Self::BeginWaveformSelectionShiftPrecise {
                pointer_nanos: pointer_nanos,
                start_nanos: start_nanos,
                end_nanos: end_nanos,
            },
            compat::UiAction::BeginWaveformEditSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            } => Self::BeginWaveformEditSelectionShift {
                pointer_micros: pointer_micros,
                start_micros: start_micros,
                end_micros: end_micros,
            },
            compat::UiAction::BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            } => Self::BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos: pointer_nanos,
                start_nanos: start_nanos,
                end_nanos: end_nanos,
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
                center_micros: center_micros,
                center_nanos: center_nanos,
            },
            compat::UiAction::ZoomWaveform {
                zoom_in,
                steps,
                anchor_ratio_micros,
            } => Self::ZoomWaveform {
                zoom_in: zoom_in,
                steps: steps,
                anchor_ratio_micros: anchor_ratio_micros,
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
            UiAction::SelectColumn { index } => Self::SelectColumn { index: index },
            UiAction::MoveColumn { delta } => Self::MoveColumn { delta: delta },
            UiAction::ToggleTransport => Self::ToggleTransport,
            UiAction::PlayCompareAnchor => Self::PlayCompareAnchor,
            UiAction::PlayFromStart => Self::PlayFromStart,
            UiAction::PlayFromCurrentPlayhead => Self::PlayFromCurrentPlayhead,
            UiAction::PlayFromWaveformCursor => Self::PlayFromWaveformCursor,
            UiAction::PlayWaveformAtPrecise { position_nanos } => Self::PlayWaveformAtPrecise {
                position_nanos: position_nanos,
            },
            UiAction::HandleEscape => Self::HandleEscape,
            UiAction::FocusBrowserPanel => Self::FocusBrowserPanel,
            UiAction::FocusSourcesPanel => Self::FocusSourcesPanel,
            UiAction::FocusWaveformPanel => Self::FocusWaveformPanel,
            UiAction::FocusFolderPanel { pane } => Self::FocusFolderPanel {
                pane: pane.map(Into::into),
            },
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
            UiAction::OpenAudioOutputHostPicker => Self::OpenAudioOutputHostPicker,
            UiAction::OpenAudioOutputDevicePicker => Self::OpenAudioOutputDevicePicker,
            UiAction::OpenAudioOutputSampleRatePicker => Self::OpenAudioOutputSampleRatePicker,
            UiAction::OpenAudioInputHostPicker => Self::OpenAudioInputHostPicker,
            UiAction::OpenAudioInputDevicePicker => Self::OpenAudioInputDevicePicker,
            UiAction::OpenAudioInputSampleRatePicker => Self::OpenAudioInputSampleRatePicker,
            UiAction::SetAudioOutputHost { host_id } => {
                Self::SetAudioOutputHost { host_id: host_id }
            }
            UiAction::SetAudioOutputDevice { device_name } => Self::SetAudioOutputDevice {
                device_name: device_name,
            },
            UiAction::SetAudioOutputSampleRate { sample_rate } => Self::SetAudioOutputSampleRate {
                sample_rate: sample_rate,
            },
            UiAction::SetAudioInputHost { host_id } => Self::SetAudioInputHost { host_id: host_id },
            UiAction::SetAudioInputDevice { device_name } => Self::SetAudioInputDevice {
                device_name: device_name,
            },
            UiAction::SetAudioInputSampleRate { sample_rate } => Self::SetAudioInputSampleRate {
                sample_rate: sample_rate,
            },
            UiAction::FocusFolderSearch { pane } => Self::FocusFolderSearch {
                pane: pane.map(Into::into),
            },
            UiAction::SetFolderSearch { pane, query } => Self::SetFolderSearch {
                pane: pane.map(Into::into),
                query: query,
            },
            UiAction::ToggleShowAllFolders { pane } => Self::ToggleShowAllFolders {
                pane: pane.map(Into::into),
            },
            UiAction::ToggleFolderFlattenedView { pane } => Self::ToggleFolderFlattenedView {
                pane: pane.map(Into::into),
            },
            UiAction::FocusSourceRow { pane, index } => Self::FocusSourceRow {
                pane: pane.map(Into::into),
                index: index,
            },
            UiAction::SelectSourceRow { pane, index } => Self::SelectSourceRow {
                pane: pane.map(Into::into),
                index: index,
            },
            UiAction::MoveSourceFocus { delta } => Self::MoveSourceFocus { delta: delta },
            UiAction::ReloadFocusedSourceRow => Self::ReloadFocusedSourceRow,
            UiAction::HardSyncFocusedSourceRow => Self::HardSyncFocusedSourceRow,
            UiAction::OpenFocusedSourceFolder => Self::OpenFocusedSourceFolder,
            UiAction::RemoveFocusedSourceRow => Self::RemoveFocusedSourceRow,
            UiAction::ReloadSourceRow { pane, index } => Self::ReloadSourceRow {
                pane: pane.map(Into::into),
                index: index,
            },
            UiAction::HardSyncSourceRow { pane, index } => Self::HardSyncSourceRow {
                pane: pane.map(Into::into),
                index: index,
            },
            UiAction::OpenSourceFolderRow { pane, index } => Self::OpenSourceFolderRow {
                pane: pane.map(Into::into),
                index: index,
            },
            UiAction::RemoveSourceRow { pane, index } => Self::RemoveSourceRow {
                pane: pane.map(Into::into),
                index: index,
            },
            UiAction::FocusFolderRow { pane, index } => Self::FocusFolderRow {
                pane: pane.map(Into::into),
                index: index,
            },
            UiAction::ActivateFolderRow { pane, index } => Self::ActivateFolderRow {
                pane: pane.map(Into::into),
                index: index,
            },
            UiAction::ToggleFolderRowExpanded { pane, index } => Self::ToggleFolderRowExpanded {
                pane: pane.map(Into::into),
                index: index,
            },
            UiAction::ExpandFocusedFolder => Self::ExpandFocusedFolder,
            UiAction::CollapseFocusedFolder => Self::CollapseFocusedFolder,
            UiAction::ToggleFocusedFolderSelection => Self::ToggleFocusedFolderSelection,
            UiAction::MoveFolderFocus { delta } => Self::MoveFolderFocus { delta: delta },
            UiAction::StartNewFolder => Self::StartNewFolder,
            UiAction::StartNewFolderAtFolderRow { pane, index } => {
                Self::StartNewFolderAtFolderRow {
                    pane: pane.map(Into::into),
                    index: index,
                }
            }
            UiAction::StartNewFolderAtRoot => Self::StartNewFolderAtRoot,
            UiAction::FocusFolderCreateInput => Self::FocusFolderCreateInput,
            UiAction::SetFolderCreateInput { value } => Self::SetFolderCreateInput { value: value },
            UiAction::ConfirmFolderCreate => Self::ConfirmFolderCreate,
            UiAction::CancelFolderCreate => Self::CancelFolderCreate,
            UiAction::StartFolderRename => Self::StartFolderRename,
            UiAction::DeleteFocusedFolder => Self::DeleteFocusedFolder,
            UiAction::RestoreRetainedFolderDeletes => Self::RestoreRetainedFolderDeletes,
            UiAction::PurgeRetainedFolderDeletes => Self::PurgeRetainedFolderDeletes,
            UiAction::ClearFolderDeleteRecoveryLog => Self::ClearFolderDeleteRecoveryLog,
            UiAction::MoveBrowserFocus { delta } => Self::MoveBrowserFocus { delta: delta },
            UiAction::SetBrowserViewStart { visible_row } => Self::SetBrowserViewStart {
                visible_row: visible_row,
            },
            UiAction::FocusBrowserRow { visible_row } => Self::FocusBrowserRow {
                visible_row: visible_row,
            },
            UiAction::SetCompareAnchorFromFocusedBrowserSample => {
                Self::SetCompareAnchorFromFocusedBrowserSample
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
                Self::ToggleBrowserRowSelection {
                    visible_row: visible_row,
                }
            }
            UiAction::StartBrowserSampleDrag {
                visible_row,
                pointer_x,
                pointer_y,
            } => Self::StartBrowserSampleDrag {
                visible_row: visible_row,
                pointer_x: pointer_x,
                pointer_y: pointer_y,
            },
            UiAction::UpdateBrowserSampleDrag {
                pointer_x,
                pointer_y,
                hovered_folder_pane,
                hovered_folder_row,
                over_folder_panel,
                shift_down,
                alt_down,
            } => Self::UpdateBrowserSampleDrag {
                pointer_x: pointer_x,
                pointer_y: pointer_y,
                hovered_folder_pane: hovered_folder_pane.map(Into::into),
                hovered_folder_row: hovered_folder_row,
                over_folder_panel: over_folder_panel.map(Into::into),
                shift_down: shift_down,
                alt_down: alt_down,
            },
            UiAction::FinishBrowserSampleDrag => Self::FinishBrowserSampleDrag,
            UiAction::ExtendBrowserSelectionToRow { visible_row } => {
                Self::ExtendBrowserSelectionToRow {
                    visible_row: visible_row,
                }
            }
            UiAction::AddRangeBrowserSelection { visible_row } => Self::AddRangeBrowserSelection {
                visible_row: visible_row,
            },
            UiAction::ExtendBrowserSelectionFromFocus { delta } => {
                Self::ExtendBrowserSelectionFromFocus { delta: delta }
            }
            UiAction::AddRangeBrowserSelectionFromFocus { delta } => {
                Self::AddRangeBrowserSelectionFromFocus { delta: delta }
            }
            UiAction::ToggleFocusedBrowserRowSelection => Self::ToggleFocusedBrowserRowSelection,
            UiAction::SelectAllBrowserRows => Self::SelectAllBrowserRows,
            UiAction::SetBrowserSearch { query } => Self::SetBrowserSearch { query: query },
            UiAction::ToggleBrowserRatingFilter { level, invert } => {
                Self::ToggleBrowserRatingFilter {
                    level: level,
                    invert: invert,
                }
            }
            UiAction::ToggleBrowserPlaybackAgeFilter { bucket, invert } => {
                Self::ToggleBrowserPlaybackAgeFilter {
                    bucket: bucket.into(),
                    invert: invert,
                }
            }
            UiAction::ToggleBrowserSampleMark => Self::ToggleBrowserSampleMark,
            UiAction::ToggleBrowserMarkedFilter => Self::ToggleBrowserMarkedFilter,
            UiAction::ToggleBrowserTagNamedFilter { invert } => {
                Self::ToggleBrowserTagNamedFilter { invert: invert }
            }
            UiAction::ToggleRandomNavigationMode => Self::ToggleRandomNavigationMode,
            UiAction::ToggleBrowserTagSidebar => Self::ToggleBrowserTagSidebar,
            UiAction::ToggleBrowserTagSidebarAutoRename => Self::ToggleBrowserTagSidebarAutoRename,
            UiAction::ToggleBrowserDuplicateCleanupMode => Self::ToggleBrowserDuplicateCleanupMode,
            UiAction::FocusPreviousBrowserHistory => Self::FocusPreviousBrowserHistory,
            UiAction::FocusNextBrowserHistory => Self::FocusNextBrowserHistory,
            UiAction::ToggleFindSimilarFocusedSample => Self::ToggleFindSimilarFocusedSample,
            UiAction::ToggleBrowserDuplicateCleanupKeep { visible_row } => {
                Self::ToggleBrowserDuplicateCleanupKeep {
                    visible_row: visible_row,
                }
            }
            UiAction::ConfirmBrowserDuplicateCleanup => Self::ConfirmBrowserDuplicateCleanup,
            UiAction::PlayRandomSample => Self::PlayRandomSample,
            UiAction::PlayPreviousRandomSample => Self::PlayPreviousRandomSample,
            UiAction::AdjustSelectedBrowserRating { delta } => {
                Self::AdjustSelectedBrowserRating { delta: delta }
            }
            UiAction::SetBrowserTab { map } => Self::SetBrowserTab { map: map },
            UiAction::FocusBrowserTagSidebarInput => Self::FocusBrowserTagSidebarInput,
            UiAction::SetBrowserTagSidebarInput { value } => {
                Self::SetBrowserTagSidebarInput { value: value }
            }
            UiAction::CommitBrowserTagSidebarInput => Self::CommitBrowserTagSidebarInput,
            UiAction::SetBrowserSidebarLooped { looped } => {
                Self::SetBrowserSidebarLooped { looped: looped }
            }
            UiAction::ToggleBrowserSidebarNormalTag { label } => {
                Self::ToggleBrowserSidebarNormalTag { label: label }
            }
            UiAction::FocusMapSample { sample_id } => Self::FocusMapSample {
                sample_id: sample_id,
            },
            UiAction::SetPromptInput { value } => Self::SetPromptInput { value: value },
            UiAction::StartBrowserRename => Self::StartBrowserRename,
            UiAction::ConfirmBrowserRename => Self::ConfirmBrowserRename,
            UiAction::CancelBrowserRename => Self::CancelBrowserRename,
            UiAction::AutoRenameBrowserSelection { visible_row } => {
                Self::AutoRenameBrowserSelection {
                    visible_row: visible_row,
                }
            }
            UiAction::TagBrowserSelection { target } => Self::TagBrowserSelection {
                target: target.into(),
            },
            UiAction::DeleteBrowserSelection => Self::DeleteBrowserSelection,
            UiAction::NormalizeFocusedBrowserSample => Self::NormalizeFocusedBrowserSample,
            UiAction::NormalizeWaveformSelectionOrSample => {
                Self::NormalizeWaveformSelectionOrSample
            }
            UiAction::CropWaveformSelection => Self::CropWaveformSelection,
            UiAction::CropWaveformSelectionToNewSample => Self::CropWaveformSelectionToNewSample,
            UiAction::TrimWaveformSelection => Self::TrimWaveformSelection,
            UiAction::ReverseWaveformSelection => Self::ReverseWaveformSelection,
            UiAction::FadeWaveformSelectionLeftToRight => Self::FadeWaveformSelectionLeftToRight,
            UiAction::FadeWaveformSelectionRightToLeft => Self::FadeWaveformSelectionRightToLeft,
            UiAction::MuteWaveformSelection => Self::MuteWaveformSelection,
            UiAction::DeleteSelectedSliceMarkers => Self::DeleteSelectedSliceMarkers,
            UiAction::ToggleWaveformSliceSelection { index } => {
                Self::ToggleWaveformSliceSelection { index: index }
            }
            UiAction::AuditionWaveformDuplicateSlice { index } => {
                Self::AuditionWaveformDuplicateSlice { index: index }
            }
            UiAction::ToggleWaveformDuplicateSliceExemption { index } => {
                Self::ToggleWaveformDuplicateSliceExemption { index: index }
            }
            UiAction::MoveWaveformSliceFocus { delta } => {
                Self::MoveWaveformSliceFocus { delta: delta }
            }
            UiAction::ToggleFocusedWaveformSliceExportMark => {
                Self::ToggleFocusedWaveformSliceExportMark
            }
            UiAction::AlignWaveformStartToMarker => Self::AlignWaveformStartToMarker,
            UiAction::DeleteLoadedWaveformSample => Self::DeleteLoadedWaveformSample,
            UiAction::SlideWaveformSelection { delta, fine } => Self::SlideWaveformSelection {
                delta: delta,
                fine: fine,
            },
            UiAction::ConfirmPrompt => Self::ConfirmPrompt,
            UiAction::CancelPrompt => Self::CancelPrompt,
            UiAction::CancelProgress => Self::CancelProgress,
            UiAction::CopySelectionToClipboard => Self::CopySelectionToClipboard,
            UiAction::ToggleHotkeyOverlay => Self::ToggleHotkeyOverlay,
            UiAction::CopyStatusLog => Self::CopyStatusLog,
            UiAction::OpenFeedbackIssuePrompt => Self::OpenFeedbackIssuePrompt,
            UiAction::MoveTrashedSamplesToFolder => Self::MoveDiscardedItemsToFolder,
            UiAction::SetInputMonitoringEnabled { enabled } => {
                Self::SetInputMonitoringEnabled { enabled: enabled }
            }
            UiAction::SetAdvanceAfterRatingEnabled { enabled } => {
                Self::SetAdvanceAfterRatingEnabled { enabled: enabled }
            }
            UiAction::SetDestructiveYoloMode { enabled } => {
                Self::SetDestructiveYoloMode { enabled: enabled }
            }
            UiAction::SetInvertWaveformScroll { enabled } => {
                Self::SetInvertWaveformScroll { enabled: enabled }
            }
            UiAction::ToggleLoopPlayback => Self::ToggleLoopPlayback,
            UiAction::ToggleLoopLock => Self::ToggleLoopLock,
            UiAction::SetWaveformChannelView { stereo } => {
                Self::SetWaveformChannelView { stereo: stereo }
            }
            UiAction::SetNormalizedAuditionEnabled { enabled } => {
                Self::SetNormalizedAuditionEnabled { enabled: enabled }
            }
            UiAction::SetBpmSnapEnabled { enabled } => Self::SetBpmSnapEnabled { enabled: enabled },
            UiAction::SetRelativeBpmGridEnabled { enabled } => {
                Self::SetRelativeBpmGridEnabled { enabled: enabled }
            }
            UiAction::AdjustWaveformBpm { delta } => Self::AdjustWaveformBpm { delta: delta },
            UiAction::SetWaveformBpmValue { value_tenths } => Self::SetWaveformBpmValue {
                value_tenths: value_tenths,
            },
            UiAction::SetTransientSnapEnabled { enabled } => {
                Self::SetTransientSnapEnabled { enabled: enabled }
            }
            UiAction::SetTransientMarkersEnabled { enabled } => {
                Self::SetTransientMarkersEnabled { enabled: enabled }
            }
            UiAction::ToggleTransientMarkers => Self::ToggleTransientMarkers,
            UiAction::ToggleBpmSnap => Self::ToggleBpmSnap,
            UiAction::SetSliceModeEnabled { enabled } => {
                Self::SetSliceModeEnabled { enabled: enabled }
            }
            UiAction::SetVolume { value_milli } => Self::SetVolume {
                value_milli: value_milli,
            },
            UiAction::CommitVolumeSetting => Self::CommitVolumeSetting,
            UiAction::SeekWaveformPrecise { position_nanos } => Self::SeekWaveformPrecise {
                position_nanos: position_nanos,
            },
            UiAction::SetWaveformCursorPrecise { position_nanos } => {
                Self::SetWaveformCursorPrecise {
                    position_nanos: position_nanos,
                }
            }
            UiAction::SeekWaveform { position_milli } => Self::SeekWaveform {
                position_milli: position_milli,
            },
            UiAction::SetWaveformCursor { position_milli } => Self::SetWaveformCursor {
                position_milli: position_milli,
            },
            UiAction::BeginWaveformSelectionAt { anchor_micros } => {
                Self::BeginWaveformSelectionAt {
                    anchor_micros: anchor_micros,
                }
            }
            UiAction::BeginWaveformSelectionAtPrecise { anchor_nanos } => {
                Self::BeginWaveformSelectionAtPrecise {
                    anchor_nanos: anchor_nanos,
                }
            }
            UiAction::BeginWaveformCircularSlide { anchor_micros } => {
                Self::BeginWaveformCircularSlide {
                    anchor_micros: anchor_micros,
                }
            }
            UiAction::UpdateWaveformCircularSlide { position_micros } => {
                Self::UpdateWaveformCircularSlide {
                    position_micros: position_micros,
                }
            }
            UiAction::FinishWaveformCircularSlide => Self::FinishWaveformCircularSlide,
            UiAction::SetWaveformSelectionRange {
                start_micros,
                end_micros,
                snap_override,
                preserve_view_edge,
            } => Self::SetWaveformSelectionRange {
                start_micros: start_micros,
                end_micros: end_micros,
                snap_override: snap_override,
                preserve_view_edge: preserve_view_edge,
            },
            UiAction::SetWaveformSelectionRangePrecise {
                start_nanos,
                end_nanos,
                snap_override,
                preserve_view_edge,
            } => Self::SetWaveformSelectionRangePrecise {
                start_nanos: start_nanos,
                end_nanos: end_nanos,
                snap_override: snap_override,
                preserve_view_edge: preserve_view_edge,
            },
            UiAction::SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            } => Self::SetWaveformSelectionRangeSmartScale {
                start_micros: start_micros,
                end_micros: end_micros,
            },
            UiAction::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos,
                end_nanos,
            } => Self::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos: start_nanos,
                end_nanos: end_nanos,
            },
            UiAction::SetWaveformEditSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge,
            } => Self::SetWaveformEditSelectionRange {
                start_micros: start_micros,
                end_micros: end_micros,
                preserve_view_edge: preserve_view_edge,
            },
            UiAction::SetWaveformEditSelectionRangePrecise {
                start_nanos,
                end_nanos,
                preserve_view_edge,
            } => Self::SetWaveformEditSelectionRangePrecise {
                start_nanos: start_nanos,
                end_nanos: end_nanos,
                preserve_view_edge: preserve_view_edge,
            },
            UiAction::SetWaveformEditFadeInEnd { position_micros } => {
                Self::SetWaveformEditFadeInEnd {
                    position_micros: position_micros,
                }
            }
            UiAction::SetWaveformEditFadeInMuteStart { position_micros } => {
                Self::SetWaveformEditFadeInMuteStart {
                    position_micros: position_micros,
                }
            }
            UiAction::SetWaveformEditFadeInCurve { curve_milli } => {
                Self::SetWaveformEditFadeInCurve {
                    curve_milli: curve_milli,
                }
            }
            UiAction::SetWaveformEditFadeOutStart { position_micros } => {
                Self::SetWaveformEditFadeOutStart {
                    position_micros: position_micros,
                }
            }
            UiAction::SetWaveformEditFadeOutMuteEnd { position_micros } => {
                Self::SetWaveformEditFadeOutMuteEnd {
                    position_micros: position_micros,
                }
            }
            UiAction::SetWaveformEditFadeOutCurve { curve_milli } => {
                Self::SetWaveformEditFadeOutCurve {
                    curve_milli: curve_milli,
                }
            }
            UiAction::FinishWaveformEditFadeDrag => Self::FinishWaveformEditFadeDrag,
            UiAction::StartWaveformSelectionDrag {
                pointer_x,
                pointer_y,
            } => Self::StartWaveformSelectionDrag {
                pointer_x: pointer_x,
                pointer_y: pointer_y,
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
                pointer_x: pointer_x,
                pointer_y: pointer_y,
                hovered_folder_pane: hovered_folder_pane.map(Into::into),
                hovered_folder_row: hovered_folder_row,
                over_folder_panel: over_folder_panel.map(Into::into),
                over_browser_list: over_browser_list,
                shift_down: shift_down,
                alt_down: alt_down,
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
                pointer_micros: pointer_micros,
                start_micros: start_micros,
                end_micros: end_micros,
            },
            UiAction::BeginWaveformSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            } => Self::BeginWaveformSelectionShiftPrecise {
                pointer_nanos: pointer_nanos,
                start_nanos: start_nanos,
                end_nanos: end_nanos,
            },
            UiAction::BeginWaveformEditSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            } => Self::BeginWaveformEditSelectionShift {
                pointer_micros: pointer_micros,
                start_micros: start_micros,
                end_micros: end_micros,
            },
            UiAction::BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            } => Self::BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos: pointer_nanos,
                start_nanos: start_nanos,
                end_nanos: end_nanos,
            },
            UiAction::FinishWaveformEditSelectionDrag => Self::FinishWaveformEditSelectionDrag,
            UiAction::ClearWaveformSelection => Self::ClearWaveformSelection,
            UiAction::ClearWaveformEditSelection => Self::ClearWaveformEditSelection,
            UiAction::ClearWaveformSelections => Self::ClearWaveformSelections,
            UiAction::SetWaveformViewCenter {
                center_micros,
                center_nanos,
            } => Self::SetWaveformViewCenter {
                center_micros: center_micros,
                center_nanos: center_nanos,
            },
            UiAction::ZoomWaveform {
                zoom_in,
                steps,
                anchor_ratio_micros,
            } => Self::ZoomWaveform {
                zoom_in: zoom_in,
                steps: steps,
                anchor_ratio_micros: anchor_ratio_micros,
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
