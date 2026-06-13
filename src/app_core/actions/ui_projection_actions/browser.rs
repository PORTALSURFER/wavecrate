use serde::{Deserialize, Serialize};

use super::super::ui_projection_dtos::{FolderPaneIdModel, PlaybackAgeFilterChip};
use crate::app_core::state::{BrowserSidebarFilterFacet, BrowserSidebarFilterOption};

/// Triage targets used by UI browser action surfaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrowserTagTarget {
    /// Move selected/focused rows to trash.
    Trash,
    /// Set selected/focused rows to neutral.
    Neutral,
    /// Mark selected/focused rows as keep.
    Keep,
}

/// Browser navigation, selection, search, and map actions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrowserAction {
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
}
