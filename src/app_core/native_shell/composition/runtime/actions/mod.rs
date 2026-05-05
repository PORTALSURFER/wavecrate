//! Sempal user-intent actions emitted through the legacy Radiant compatibility path.
//!
//! [`UiAction`] intentionally remains the single compatibility surface between
//! the native runtime and the host bridge. The enum stays centralized so hosts
//! can inspect the full action catalog in one place.
//!
//! This module is intentionally broad rather than split by action family. The
//! runtime, host bridge, and automation catalog all rely on one inspectable
//! action surface, so the preferred maintenance approach is to keep the enum
//! centralized while improving internal organization around it.

use super::{FolderPaneIdModel, PlaybackAgeFilterChip};
use serde::{Deserialize, Serialize};

/// Triage targets used by native browser action surfaces.
pub type BrowserTriageTarget = crate::gui::selection::TriageTarget;

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
    /// Replay the stored compare-anchor item without changing browser focus.
    PlayCompareAnchor,
    /// Start playback from the beginning of the active content item.
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
    /// Focus the loaded content item in the primary list.
    FocusLoadedContentInList,
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
    /// Return from one picker to the main options overview.
    ShowOptionsOverview,
    /// Expand the primary group picker inside the options panel.
    OpenPrimaryGroupPicker,
    /// Expand the primary item picker inside the options panel.
    OpenPrimaryItemPicker,
    /// Expand the primary numeric-setting picker inside the options panel.
    OpenPrimaryNumberPicker,
    /// Expand the secondary group picker inside the options panel.
    OpenSecondaryGroupPicker,
    /// Expand the secondary item picker inside the options panel.
    OpenSecondaryItemPicker,
    /// Expand the secondary numeric-setting picker inside the options panel.
    OpenSecondaryNumberPicker,
    /// Apply one primary group selection.
    SetPrimaryGroup {
        /// Selected group identifier, or `None` for the system default.
        group_id: Option<String>,
    },
    /// Apply one primary item selection.
    SetPrimaryItem {
        /// Selected item name, or `None` for the group default.
        item_name: Option<String>,
    },
    /// Apply one primary numeric-setting selection.
    SetPrimaryNumber {
        /// Selected numeric value, or `None` for the item default.
        value: Option<u32>,
    },
    /// Apply one secondary group selection.
    SetSecondaryGroup {
        /// Selected group identifier, or `None` for the system default.
        group_id: Option<String>,
    },
    /// Apply one secondary item selection.
    SetSecondaryItem {
        /// Selected item name, or `None` for the group default.
        item_name: Option<String>,
    },
    /// Apply one secondary numeric-setting selection.
    SetSecondaryNumber {
        /// Selected numeric value, or `None` for the item default.
        value: Option<u32>,
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
    /// Toggle whether the folder tree shows disk folders without indexed content.
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
    /// Store the focused content item as the compare-anchor reference.
    SetCompareAnchorFromFocusedContent,
    /// Commit the currently focused browser row as the active loaded content.
    CommitFocusedBrowserRow,
    /// Save the current waveform selection or slices into the browser as new content.
    SaveWaveformSelectionToBrowser,
    /// Save the current waveform selection or slices and mark exported clips keep-2.
    SaveWaveformSelectionToBrowserWithKeep2,
    /// Commit preview fades for the active waveform edit selection.
    CommitWaveformEditFades,
    /// Detect silence-split waveform slices for the loaded content.
    DetectWaveformSilenceSlices,
    /// Detect near-duplicate windows for the loaded content using the current selection size.
    DetectWaveformExactDuplicateSlices,
    /// Clean near-duplicate windows while keeping the first occurrence.
    CleanWaveformExactDuplicateSlices,
    /// Toggle browser-row selection by visible index.
    ToggleBrowserRowSelection {
        /// Target visible row index in the browser list.
        visible_row: usize,
    },
    /// Start dragging one browser content item or the active browser multi-selection.
    ///
    /// The runtime emits this only after a browser-row press exceeds drag slop,
    /// so plain clicks can still resolve into the existing focus/selection
    /// actions on release without changing preview behavior.
    StartContentItemDrag {
        /// Target visible row index that armed the drag session.
        visible_row: usize,
        /// Pointer x-position in logical UI coordinates.
        pointer_x: u16,
        /// Pointer y-position in logical UI coordinates.
        pointer_y: u16,
    },
    /// Update the active browser content-item drag with the latest pointer position.
    UpdateContentItemDrag {
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
    /// Finish the active browser content-item drag gesture.
    FinishContentItemDrag,
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
    /// Toggle the session mark for the focused content row or current multi-selection.
    ToggleContentMark,
    /// Toggle whether the browser shows only session-marked items.
    ToggleBrowserMarkedFilter,
    /// Toggle whether the browser matches the host-defined derived-label state.
    ToggleBrowserDerivedLabelFilter {
        /// Whether the click should invert the derived-label filter.
        invert: bool,
    },
    /// Toggle sticky random navigation mode for browser next/previous stepping.
    ToggleRandomNavigationMode,
    /// Toggle the browser-local metadata pill editor.
    ToggleBrowserPillEditor,
    /// Toggle the host-defined primary side effect for browser metadata edits.
    ToggleBrowserPillEditorPrimaryAction,
    /// Toggle browser duplicate-cleanup mode for the focused browser item.
    ToggleBrowserDuplicateCleanupMode,
    /// Focus the previous browser item from focus history.
    FocusPreviousBrowserHistory,
    /// Focus the next browser item from focus history.
    FocusNextBrowserHistory,
    /// Toggle find-similar mode for the focused content item.
    ToggleFindSimilarFocusedContent,
    /// Toggle whether one duplicate-cleanup browser row should be kept.
    ToggleBrowserDuplicateCleanupKeep {
        /// Target visible row index in the browser list.
        visible_row: usize,
    },
    /// Confirm duplicate cleanup and trash every unkept duplicate.
    ConfirmBrowserDuplicateCleanup,
    /// Play a random visible content item.
    PlayRandomContentItem,
    /// Replay the previous random-visible content item.
    PlayPreviousRandomContentItem,
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
    /// Focus the browser metadata pill-editor input field.
    FocusBrowserPillEditorInput,
    /// Set the browser metadata pill-editor input value.
    SetBrowserPillEditorInput {
        /// Full pill-editor input text.
        value: String,
    },
    /// Commit the browser metadata pill-editor input value.
    CommitBrowserPillEditorInput,
    /// Apply one playback-type value to the browser selection.
    SetBrowserSidebarLooped {
        /// Playback type to apply.
        looped: bool,
    },
    /// Toggle one pill option for the browser selection.
    ToggleBrowserPillOption {
        /// Pill option label to assign or remove.
        label: String,
    },
    /// Focus a specific spatial content item by stable content id.
    FocusSpatialContentItem {
        /// Stable content identifier used by spatial hit-testing.
        content_id: String,
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
    /// Apply a triage mark to focused/selected browser rows.
    SetBrowserTriageMark {
        /// Triage bucket applied to focused/selected browser rows.
        target: BrowserTriageTarget,
    },
    /// Delete focused/selected browser rows.
    DeleteBrowserSelection,
    /// Normalize the focused list content item in-place.
    NormalizeFocusedContentItem,
    /// Normalize the waveform selection, or the loaded content when no selection is active.
    NormalizeWaveformSelectionOrLoadedContent,
    /// Crop the waveform file down to the active selection.
    CropWaveformSelection,
    /// Write the active waveform selection to a new sibling content item.
    CropWaveformSelectionToNewContentItem,
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
    /// Delete the currently loaded waveform content and navigate to the next candidate.
    DeleteLoadedWaveformContent,
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
    /// Copy the current content file(s) or timeline selection clip to the clipboard.
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
    /// Move all discarded items into the configured destination folder.
    MoveDiscardedItemsToFolder,

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
    /// Enter or cycle the locked loop override across content changes.
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
    /// Hosts use this for wrap-drag content rotation: while the gesture is
    /// active, pointer motion rotates the waveform preview in a wrapping
    /// manner, and release commits the rotated content to disk.
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
        /// Whether the pointer currently hovers the content list.
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
