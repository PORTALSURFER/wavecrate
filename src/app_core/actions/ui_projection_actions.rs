//! Wavecrate-owned UI runtime action DTOs.
//!
//! These actions describe Wavecrate user intent inside app-core. Retained
//! legacy input shapes are owned by the sibling compatibility adapter so the
//! primary action contract can stay focused on active behavior.

use super::ui_projection_dtos::{FolderPaneIdModel, PlaybackAgeFilterChip};
use crate::app_core::state::{BrowserSidebarFilterFacet, BrowserSidebarFilterOption};
use serde::{Deserialize, Serialize};

mod browser;
mod compatibility;
mod domain;
mod history_update;
mod options;
#[cfg(test)]
mod precision_eq;
mod transport;

pub use self::browser::BrowserTagTarget;
pub use self::compatibility::{CompatibilityAction, upgrade_compatibility_action};
pub use self::domain::UiActionDomain;
pub use self::history_update::HistoryUpdateAction;
pub use self::options::OptionsAction;
pub use self::transport::TransportAction;

#[cfg_attr(not(test), derive(PartialEq, Eq))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UiAction {
    Transport(TransportAction),
    HistoryAndUpdate(HistoryUpdateAction),

    // Focus and shell-surface actions.
    FocusBrowserPanel,
    FocusSourcesPanel,
    FocusWaveformPanel,
    FocusFolderPanel,
    FocusLoadedSampleInBrowser,
    FocusBrowserSearch,
    BlurBrowserSearch,
    OpenAddSourceDialog,
    FocusFolderSearch,
    SetFolderSearch {
        query: String,
    },
    ToggleShowAllFolders,
    ToggleFolderFlattenedView,

    // Sources and folder tree actions.
    FocusSourceRow {
        index: usize,
    },
    SelectSourceRow {
        index: usize,
    },
    MoveSourceFocus {
        delta: i8,
    },
    ReloadFocusedSourceRow,
    HardSyncFocusedSourceRow,
    OpenFocusedSourceFolder,
    RemoveFocusedSourceRow,
    ReloadSourceRow {
        index: usize,
    },
    HardSyncSourceRow {
        index: usize,
    },
    OpenSourceFolderRow {
        index: usize,
    },
    RemoveSourceRow {
        index: usize,
    },
    FocusFolderRow {
        index: usize,
    },
    ActivateFolderRow {
        index: usize,
    },
    ToggleFolderRowExpanded {
        index: usize,
    },
    ExpandFocusedFolder,
    CollapseFocusedFolder,
    ToggleFocusedFolderSelection,
    MoveFolderFocus {
        delta: i8,
    },
    StartNewFolder,
    StartNewFolderAtFolderRow {
        index: usize,
    },
    StartNewFolderAtRoot,
    FocusFolderCreateInput,
    SetFolderCreateInput {
        value: String,
    },
    ConfirmFolderCreate,
    CancelFolderCreate,
    StartFolderRename,
    DeleteFocusedFolder,
    RestoreRetainedFolderDeletes,
    PurgeRetainedFolderDeletes,
    ClearFolderDeleteRecoveryLog,

    // Browser navigation, selection, search, and map actions.
    MoveBrowserFocus {
        delta: i8,
    },
    SetBrowserViewStart {
        visible_row: usize,
    },
    FocusBrowserRow {
        visible_row: usize,
    },
    SetCompareAnchorFromFocusedBrowserSample,
    CommitFocusedBrowserRow,
    SaveWaveformSelectionToBrowser,
    SaveWaveformSelectionToBrowserWithKeep2,
    CommitWaveformEditFades,
    DetectWaveformSilenceSlices,
    DetectWaveformExactDuplicateSlices,
    CleanWaveformExactDuplicateSlices,
    ToggleBrowserRowSelection {
        visible_row: usize,
    },
    StartBrowserSampleDrag {
        visible_row: usize,
        pointer_x: u16,
        pointer_y: u16,
    },
    UpdateBrowserSampleDrag {
        pointer_x: u16,
        pointer_y: u16,
        hovered_folder_pane: Option<FolderPaneIdModel>,
        hovered_folder_row: Option<usize>,
        over_folder_panel: Option<FolderPaneIdModel>,
        shift_down: bool,
        alt_down: bool,
    },
    FinishBrowserSampleDrag,
    ExtendBrowserSelectionToRow {
        visible_row: usize,
    },
    AddRangeBrowserSelection {
        visible_row: usize,
    },
    ExtendBrowserSelectionFromFocus {
        delta: i8,
    },
    AddRangeBrowserSelectionFromFocus {
        delta: i8,
    },
    ToggleFocusedBrowserRowSelection,
    SelectAllBrowserRows,
    SetBrowserSearch {
        query: String,
    },
    ToggleBrowserRatingFilter {
        level: i8,
        invert: bool,
    },
    ToggleBrowserPlaybackAgeFilter {
        bucket: PlaybackAgeFilterChip,
        invert: bool,
    },
    ToggleBrowserSidebarFilter {
        option: BrowserSidebarFilterOption,
        additive: bool,
    },
    ClearBrowserSidebarFilter {
        facet: BrowserSidebarFilterFacet,
    },
    ToggleBrowserSampleMark,
    ToggleBrowserMarkedFilter,
    ToggleBrowserTagNamedFilter {
        invert: bool,
    },
    ToggleRandomNavigationMode,
    ToggleBrowserTagSidebar,
    ToggleBrowserTagSidebarAutoRename,
    ToggleBrowserDuplicateCleanupMode,
    FocusPreviousBrowserHistory,
    FocusNextBrowserHistory,
    ToggleFindSimilarFocusedSample,
    ToggleBrowserDuplicateCleanupKeep {
        visible_row: usize,
    },
    ConfirmBrowserDuplicateCleanup,
    PlayRandomSample,
    PlayPreviousRandomSample,
    AdjustSelectedBrowserRating {
        delta: i8,
    },
    SetBrowserTab {
        map: bool,
    },
    FocusBrowserTagSidebarInput,
    SetBrowserTagSidebarInput {
        value: String,
    },
    CommitBrowserTagSidebarInput,
    SetBrowserSidebarLooped {
        looped: bool,
    },
    ToggleBrowserSidebarNormalTag {
        label: String,
    },
    FocusMapSample {
        sample_id: String,
    },

    // Prompt, rename, and confirmation actions.
    SetPromptInput {
        value: String,
    },
    StartBrowserRename,
    ConfirmBrowserRename,
    CancelBrowserRename,
    AutoRenameBrowserSelection {
        visible_row: Option<usize>,
    },
    TagBrowserSelection {
        target: BrowserTagTarget,
    },
    DeleteBrowserSelection,
    NormalizeFocusedBrowserSample,
    NormalizeWaveformSelectionOrSample,
    CropWaveformSelection,
    CropWaveformSelectionToNewSample,
    TrimWaveformSelection,
    ReverseWaveformSelection,
    FadeWaveformSelectionLeftToRight,
    FadeWaveformSelectionRightToLeft,
    MuteWaveformSelection,
    DeleteSelectedSliceMarkers,
    ToggleWaveformSliceSelection {
        index: usize,
    },
    AuditionWaveformDuplicateSlice {
        index: usize,
    },
    ToggleWaveformDuplicateSliceExemption {
        index: usize,
    },
    MoveWaveformSliceFocus {
        delta: i8,
    },
    ToggleFocusedWaveformSliceExportMark,
    AlignWaveformStartToMarker,
    DeleteLoadedWaveformSample,
    SlideWaveformSelection {
        delta: i8,
        fine: bool,
    },
    ConfirmPrompt,
    CancelPrompt,
    CancelProgress,
    CopySelectionToClipboard,
    ToggleHotkeyOverlay,
    CopyStatusLog,
    OpenFeedbackIssuePrompt,
    MoveTrashedSamplesToFolder,

    // Options and persistent interaction toggles.
    ToggleLoopPlayback,
    ToggleLoopLock,
    SetWaveformChannelView {
        stereo: bool,
    },
    SetNormalizedAuditionEnabled {
        enabled: bool,
    },
    SetBpmSnapEnabled {
        enabled: bool,
    },
    SetRelativeBpmGridEnabled {
        enabled: bool,
    },
    AdjustWaveformBpm {
        delta: i8,
    },
    SetWaveformBpmValue {
        value_tenths: u16,
    },
    SetTransientSnapEnabled {
        enabled: bool,
    },
    SetTransientMarkersEnabled {
        enabled: bool,
    },
    ToggleTransientMarkers,
    ToggleBpmSnap,
    SetSliceModeEnabled {
        enabled: bool,
    },

    // Waveform transport, edit, and gesture actions.
    SeekWaveformPrecise {
        position_nanos: u32,
    },
    SetWaveformCursorPrecise {
        position_nanos: u32,
    },
    BeginWaveformSelectionAt {
        anchor_micros: u32,
    },
    BeginWaveformSelectionAtPrecise {
        anchor_nanos: u32,
    },
    BeginWaveformCircularSlide {
        anchor_micros: u32,
    },
    UpdateWaveformCircularSlide {
        position_micros: u32,
    },
    FinishWaveformCircularSlide,
    SetWaveformSelectionRange {
        start_micros: u32,
        end_micros: u32,
        snap_override: bool,
        preserve_view_edge: bool,
    },
    SetWaveformSelectionRangePrecise {
        start_nanos: u32,
        end_nanos: u32,
        snap_override: bool,
        preserve_view_edge: bool,
    },
    SetWaveformSelectionRangeSmartScale {
        start_micros: u32,
        end_micros: u32,
    },
    SetWaveformSelectionRangeSmartScalePrecise {
        start_nanos: u32,
        end_nanos: u32,
    },
    SetWaveformEditSelectionRange {
        start_micros: u32,
        end_micros: u32,
        preserve_view_edge: bool,
    },
    SetWaveformEditSelectionRangePrecise {
        start_nanos: u32,
        end_nanos: u32,
        preserve_view_edge: bool,
    },
    SetWaveformEditFadeInEnd {
        position_micros: u32,
    },
    SetWaveformEditFadeInMuteStart {
        position_micros: u32,
    },
    SetWaveformEditFadeInCurve {
        curve_milli: u16,
    },
    SetWaveformEditFadeOutStart {
        position_micros: u32,
    },
    SetWaveformEditFadeOutMuteEnd {
        position_micros: u32,
    },
    SetWaveformEditFadeOutCurve {
        curve_milli: u16,
    },
    FinishWaveformEditFadeDrag,
    StartWaveformSelectionDrag {
        pointer_x: u16,
        pointer_y: u16,
    },
    UpdateWaveformSelectionDrag {
        pointer_x: u16,
        pointer_y: u16,
        hovered_folder_pane: Option<FolderPaneIdModel>,
        hovered_folder_row: Option<usize>,
        over_folder_panel: Option<FolderPaneIdModel>,
        over_browser_list: bool,
        shift_down: bool,
        alt_down: bool,
    },
    FinishWaveformSelectionDrag,
    FinishWaveformSelectionRangeDrag,
    FinishWaveformSelectionSmartScaleDrag,
    BeginWaveformSelectionShift {
        pointer_micros: u32,
        start_micros: u32,
        end_micros: u32,
    },
    BeginWaveformSelectionShiftPrecise {
        pointer_nanos: u32,
        start_nanos: u32,
        end_nanos: u32,
    },
    BeginWaveformEditSelectionShift {
        pointer_micros: u32,
        start_micros: u32,
        end_micros: u32,
    },
    BeginWaveformEditSelectionShiftPrecise {
        pointer_nanos: u32,
        start_nanos: u32,
        end_nanos: u32,
    },
    FinishWaveformEditSelectionDrag,
    ClearWaveformSelection,
    ClearWaveformEditSelection,
    ClearWaveformSelections,
    SetWaveformViewCenter {
        center_micros: u32,
        center_nanos: Option<u32>,
    },
    ZoomWaveform {
        zoom_in: bool,
        steps: u8,
        anchor_ratio_micros: Option<u32>,
    },
    ZoomWaveformToSelection,
    ZoomWaveformFull,

    #[serde(untagged)]
    Compatibility(CompatibilityAction),
    #[serde(untagged)]
    Options(OptionsAction),
}

#[allow(non_upper_case_globals)]
impl UiAction {
    pub const Undo: Self = Self::Compatibility(CompatibilityAction::Undo);
    pub const Redo: Self = Self::Compatibility(CompatibilityAction::Redo);
    pub const CheckForUpdates: Self = Self::Compatibility(CompatibilityAction::CheckForUpdates);
    pub const OpenUpdateLink: Self = Self::Compatibility(CompatibilityAction::OpenUpdateLink);
    pub const InstallUpdate: Self = Self::Compatibility(CompatibilityAction::InstallUpdate);
    pub const DismissUpdate: Self = Self::Compatibility(CompatibilityAction::DismissUpdate);
}
