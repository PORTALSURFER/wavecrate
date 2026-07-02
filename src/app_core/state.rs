//! Migration-facing state contract and small projection helpers.
//!
//! App-core projections and tests should import state DTOs from this module
//! instead of reaching into the legacy `app` tree directly. Most entries are
//! compatibility aliases while the legacy controller still owns the backing UI
//! state, but this file is the documented app-core re-export boundary for state
//! ownership during migration. The active ownership inventory and exit criteria
//! live in `app_core::app_api`.

pub use crate::app_core::browser_source_state::{
    BrowserBpmFacet, BrowserDuplicateCleanupState, BrowserSidebarFilterFacet,
    BrowserSidebarFilterOption, BrowserSidebarFilterState, BrowserTagTarget, FolderBrowserUiState,
    FolderDeleteRecoveryAction, FolderDeleteRecoveryEntry, FolderDeleteRecoveryStatus,
    FolderFileScopeMode, FolderPaneId, FolderRowView, InlineFolderEdit, InlineFolderEditKind,
    PlaybackAgeBucket, PlaybackAgeFilterChip, RetainedFolderDeleteEntry, SampleBrowserActionPrompt,
    SampleBrowserSort, SampleBrowserTab, SourceRowView, TagNamedFilter, TriageFlagColumn,
    TriageFlagFilter, UiState, VisibleRows, browser_playback_age_filter_chips,
};
#[cfg(test)]
pub(crate) use crate::app_core::browser_source_state::{SampleBrowserIndex, SimilarQuery};
#[cfg(test)]
pub(crate) use crate::app_core::projection_state::{
    ActiveAudioOutput, AudioDeviceView, AudioHostView, CompareAnchorState, IssueTokenStatus,
    ProgressOverlayState, ProgressTaskKind, WaveformDuplicateCleanupPreview,
    WaveformDuplicateCleanupState, WaveformSliceReviewState, WaveformView,
};
pub use crate::app_core::projection_state::{
    AudioPickerTarget, DestructiveEditPrompt, DestructiveSelectionEdit, DragPayload, DragSource,
    DragTarget, FocusContext, FolderActionPrompt, MapBounds, MapPoint, MapQueryBounds,
    MapRenderMode, MapSimilarityPrepStatus, OptionsPanelPrompt, StatusTone, UiPoint, UpdateStatus,
    WaveformSliceBatchProfile,
};
