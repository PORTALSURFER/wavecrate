//! Migration-facing state contract and small projection helpers.
//!
//! App-core projections and tests should import state DTOs from this module
//! instead of reaching into the legacy `app` tree directly. Most entries are
//! compatibility aliases while the legacy controller still owns the backing UI
//! state, but this file is the documented boundary for state ownership during
//! migration.

use crate::app::state as app_state;
use crate::app_core::actions::NativeBrowserTagTarget;

// Root UI and source/browser shell state.

/// Full UI state projected through migration-facing APIs.
pub type UiState = app_state::UiState;

/// Source row projection shown in the source list.
pub type SourceRowView = app_state::SourceRowView;

/// Browser tab selection state.
pub type SampleBrowserTab = app_state::SampleBrowserTab;

/// Selected browser index in row/column space.
#[cfg(test)]
pub(crate) type SampleBrowserIndex = app_state::SampleBrowserIndex;

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

// Source/folder browser state.

/// Source-pane identifier used by the two-pane source browser.
pub type FolderPaneId = app_state::FolderPaneId;

/// Browser tag target for triage actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserTagTarget {
    /// Mark the sample as trash.
    Trash,
    /// Mark the sample as neutral.
    Neutral,
    /// Mark the sample as keep.
    Keep,
}

impl From<NativeBrowserTagTarget> for BrowserTagTarget {
    fn from(value: NativeBrowserTagTarget) -> Self {
        match value {
            NativeBrowserTagTarget::Trash => Self::Trash,
            NativeBrowserTagTarget::Neutral => Self::Neutral,
            NativeBrowserTagTarget::Keep => Self::Keep,
        }
    }
}

/// Triaged browser column identifier.
pub type TriageFlagColumn = app_state::TriageFlagColumn;

/// Folder-filter scope for source root traversal.
pub type FolderFileScopeMode = app_state::FolderFileScopeMode;

/// Folder row view projection for migration-facing renderer models.
pub type FolderRowView = app_state::FolderRowView;

/// Folder-browser UI substate for source projection helpers.
pub type FolderBrowserUiState = app_state::FolderBrowserUiState;

/// Inline folder create/rename draft state.
pub type InlineFolderEdit = app_state::InlineFolderEdit;

/// Inline folder draft kind metadata.
pub type InlineFolderEditKind = app_state::InlineFolderEditKind;

/// Recovery action for staged folder delete.
pub type FolderDeleteRecoveryAction = app_state::FolderDeleteRecoveryAction;

/// Recovery status for staged folder delete.
pub type FolderDeleteRecoveryStatus = app_state::FolderDeleteRecoveryStatus;

/// Recovery row entry for staged folder delete.
pub type FolderDeleteRecoveryEntry = app_state::FolderDeleteRecoveryEntry;

/// Recoverable retained folder delete entry projected from startup recovery.
pub type RetainedFolderDeleteEntry = app_state::RetainedFolderDeleteEntry;

// Browser filter, row, and prompt state.

/// Browser row filter used by the sample table.
pub type TriageFlagFilter = app_state::TriageFlagFilter;

/// Browser playback-age filter chip state.
pub type PlaybackAgeFilterChip = app_state::PlaybackAgeFilterChip;

/// Browser playback-age row-visual bucket state.
pub type PlaybackAgeBucket = app_state::PlaybackAgeBucket;

/// Browser sidebar filter facet identifier.
pub type BrowserSidebarFilterFacet = app_state::BrowserSidebarFilterFacet;

/// Browser sidebar filter option payload.
pub type BrowserSidebarFilterOption = app_state::BrowserSidebarFilterOption;

/// Browser sidebar filter state.
pub type BrowserSidebarFilterState = app_state::BrowserSidebarFilterState;

/// Browser BPM facet used by sidebar filter actions.
pub type BrowserBpmFacet = app_state::BrowserBpmFacet;

/// Browser tag-named filter state.
pub type TagNamedFilter = app_state::TagNamedFilter;

/// Browser duplicate-cleanup workflow state.
pub type BrowserDuplicateCleanupState = app_state::BrowserDuplicateCleanupState;

/// Similar-browser query state.
#[cfg(test)]
pub(crate) type SimilarQuery = app_state::SimilarQuery;

/// Browser sort mode used by migration-facing projections.
pub type SampleBrowserSort = app_state::SampleBrowserSort;

/// Visible row projection used by migration-facing helpers.
pub type VisibleRows = app_state::VisibleRows;

/// Pending inline sample rename prompt.
pub type SampleBrowserActionPrompt = app_state::SampleBrowserActionPrompt;

/// Return the fixed browser playback-age chip order used across migration-facing UI surfaces.
pub fn browser_playback_age_filter_chips() -> [PlaybackAgeFilterChip; 3] {
    app_state::browser_playback_age_filter_chips()
}

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

#[cfg(test)]
mod tests {
    use super::{BrowserTagTarget, NativeBrowserTagTarget};

    #[test]
    fn browser_tag_target_maps_radiant_values() {
        assert_eq!(
            BrowserTagTarget::from(NativeBrowserTagTarget::Trash),
            BrowserTagTarget::Trash
        );
        assert_eq!(
            BrowserTagTarget::from(NativeBrowserTagTarget::Neutral),
            BrowserTagTarget::Neutral
        );
        assert_eq!(
            BrowserTagTarget::from(NativeBrowserTagTarget::Keep),
            BrowserTagTarget::Keep
        );
    }
}
