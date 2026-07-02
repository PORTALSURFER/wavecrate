//! Browser, source, folder, and library-hygiene state contracts owned by app-core.
//!
//! The legacy controller still stores the backing UI state during migration, so
//! these contracts remain representation aliases for now. Keeping the aliases
//! grouped here gives browser/source/folder projections one explicit app-core
//! ownership point instead of depending on the broad `app_api::state` bridge.

use crate::app_core::actions::NativeBrowserTagTarget;

/// Full UI state projected through migration-facing APIs.
pub type UiState = crate::app::state::UiState;

/// Source row projection shown in the source list.
pub type SourceRowView = crate::app::state::SourceRowView;

/// Browser tab selection state.
pub type SampleBrowserTab = crate::app::state::SampleBrowserTab;

/// Selected browser index in row/column space.
#[cfg(test)]
pub(crate) type SampleBrowserIndex = crate::app::state::SampleBrowserIndex;

/// Source-pane identifier used by the two-pane source browser.
pub type FolderPaneId = crate::app::state::FolderPaneId;

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
pub type TriageFlagColumn = crate::app::state::TriageFlagColumn;

/// Folder-filter scope for source root traversal.
pub type FolderFileScopeMode = crate::app::state::FolderFileScopeMode;

/// Folder row view projection for migration-facing renderer models.
pub type FolderRowView = crate::app::state::FolderRowView;

/// Folder-browser UI substate for source projection helpers.
pub type FolderBrowserUiState = crate::app::state::FolderBrowserUiState;

/// Inline folder create/rename draft state.
pub type InlineFolderEdit = crate::app::state::InlineFolderEdit;

/// Inline folder draft kind metadata.
pub type InlineFolderEditKind = crate::app::state::InlineFolderEditKind;

/// Recovery action for staged folder delete.
pub type FolderDeleteRecoveryAction = crate::app::state::FolderDeleteRecoveryAction;

/// Recovery status for staged folder delete.
pub type FolderDeleteRecoveryStatus = crate::app::state::FolderDeleteRecoveryStatus;

/// Recovery row entry for staged folder delete.
pub type FolderDeleteRecoveryEntry = crate::app::state::FolderDeleteRecoveryEntry;

/// Recoverable retained folder delete entry projected from startup recovery.
pub type RetainedFolderDeleteEntry = crate::app::state::RetainedFolderDeleteEntry;

/// Browser row filter used by the sample table.
pub type TriageFlagFilter = crate::app::state::TriageFlagFilter;

/// Browser playback-age filter chip state.
pub type PlaybackAgeFilterChip = crate::app::state::PlaybackAgeFilterChip;

/// Browser playback-age row-visual bucket state.
pub type PlaybackAgeBucket = crate::app::state::PlaybackAgeBucket;

/// Browser sidebar filter facet identifier.
pub type BrowserSidebarFilterFacet = crate::app::state::BrowserSidebarFilterFacet;

/// Browser sidebar filter option payload.
pub type BrowserSidebarFilterOption = crate::app::state::BrowserSidebarFilterOption;

/// Browser sidebar filter state.
pub type BrowserSidebarFilterState = crate::app::state::BrowserSidebarFilterState;

/// Browser BPM facet used by sidebar filter actions.
pub type BrowserBpmFacet = crate::app::state::BrowserBpmFacet;

/// Browser tag-named filter state.
pub type TagNamedFilter = crate::app::state::TagNamedFilter;

/// Browser duplicate-cleanup workflow state.
pub type BrowserDuplicateCleanupState = crate::app::state::BrowserDuplicateCleanupState;

/// Similar-browser query state.
#[cfg(test)]
pub(crate) type SimilarQuery = crate::app::state::SimilarQuery;

/// Browser sort mode used by migration-facing projections.
pub type SampleBrowserSort = crate::app::state::SampleBrowserSort;

/// Visible row projection used by migration-facing helpers.
pub type VisibleRows = crate::app::state::VisibleRows;

/// Pending inline sample rename prompt.
pub type SampleBrowserActionPrompt = crate::app::state::SampleBrowserActionPrompt;

/// Return the fixed browser playback-age chip order used across migration-facing UI surfaces.
pub fn browser_playback_age_filter_chips() -> [PlaybackAgeFilterChip; 3] {
    crate::app::state::browser_playback_age_filter_chips()
}

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
