//! Migration-facing state contract and small projection helpers.
//!
//! App-core projections and tests should import state DTOs from this module
//! instead of reaching into the legacy `app` tree directly. Most entries are
//! compatibility aliases while the legacy controller still owns the backing UI
//! state, but this file is the documented boundary for state ownership during
//! migration. The active ownership inventory and exit criteria live next to the
//! single legacy crossing in `app_core::app_api`.

use crate::app_core::app_api::state as app_state;
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

// Map projection state.

/// Normalized map query bounds shared with map projection helpers.
pub type MapQueryBounds = app_state::MapQueryBounds;

/// Cached map bounds used by migration-facing map projections.
pub type MapBounds = app_state::MapBounds;

/// Cached projected map point payload used by migration-facing map projections.
pub type MapPoint = app_state::MapPoint;

/// Map similarity-preparation status surfaced by map projections.
pub type MapSimilarityPrepStatus = app_state::MapSimilarityPrepStatus;

/// Map rendering mode exposed by migration-facing projections.
pub type MapRenderMode = app_state::MapRenderMode;

// Options/audio status state.

/// Active audio-picker target shown in options flows.
pub type AudioPickerTarget = app_state::AudioPickerTarget;

/// Active audio output description.
#[cfg(test)]
pub(crate) type ActiveAudioOutput = app_state::ActiveAudioOutput;

/// Audio host option description.
#[cfg(test)]
pub(crate) type AudioHostView = app_state::AudioHostView;

/// Audio device option description.
#[cfg(test)]
pub(crate) type AudioDeviceView = app_state::AudioDeviceView;

/// Update status exposed by migration-facing projections.
pub type UpdateStatus = app_state::UpdateStatus;

/// UI status tone used for app-level status messages.
pub type StatusTone = app_state::StatusTone;

/// Progress task kind used by progress overlays.
#[cfg(test)]
pub(crate) type ProgressTaskKind = app_state::ProgressTaskKind;

/// Progress overlay state used by controller action tests.
#[cfg(test)]
pub(crate) type ProgressOverlayState = app_state::ProgressOverlayState;

/// Issue-token status used by controller action tests.
#[cfg(test)]
pub(crate) type IssueTokenStatus = app_state::IssueTokenStatus;

// Prompt, drag/drop, and waveform state.

/// Pending modal folder action prompt.
pub type FolderActionPrompt = app_state::FolderActionPrompt;

/// Pending options-panel confirmation prompt.
pub type OptionsPanelPrompt = app_state::OptionsPanelPrompt;

/// Destructive edit action exposed by prompt surfaces.
pub type DestructiveSelectionEdit = app_state::DestructiveSelectionEdit;

/// Prompt model for destructive edits.
pub type DestructiveEditPrompt = app_state::DestructiveEditPrompt;

/// Unified drag target used by migration-facing drag/drop projections.
pub type DragTarget = app_state::DragTarget;

/// Active drag payload.
pub type DragPayload = app_state::DragPayload;

/// Drag source used by waveform and browser actions.
pub type DragSource = app_state::DragSource;

/// Focus context shared by controller action routing.
pub type FocusContext = app_state::FocusContext;

/// UI-space point for drag/drop and waveform interactions.
pub type UiPoint = app_state::UiPoint;

/// Waveform comparison-anchor state.
#[cfg(test)]
pub(crate) type CompareAnchorState = app_state::CompareAnchorState;

/// Waveform view state projected through migration-facing APIs.
#[cfg(test)]
pub(crate) type WaveformView = app_state::WaveformView;

/// Waveform slice batch profile used by waveform projection and options tests.
pub type WaveformSliceBatchProfile = app_state::WaveformSliceBatchProfile;

/// Waveform slice-review state used by options tests.
#[cfg(test)]
pub(crate) type WaveformSliceReviewState = app_state::WaveformSliceReviewState;

/// Waveform duplicate-cleanup state used by options tests.
#[cfg(test)]
pub(crate) type WaveformDuplicateCleanupState = app_state::WaveformDuplicateCleanupState;

/// Waveform duplicate-cleanup preview used by options tests.
#[cfg(test)]
pub(crate) type WaveformDuplicateCleanupPreview = app_state::WaveformDuplicateCleanupPreview;
