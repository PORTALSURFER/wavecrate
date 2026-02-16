//! Migration-facing state aliases and small projection helpers.
//!
//! This module keeps the public `app_core` state API stable while the legacy app
//! module remains the source of truth during migration.

use crate::app_core::actions::NativeBrowserTagTarget;
use crate::app_core::app_api::state as app_state;

/// Full UI state projected through migration-facing APIs.
pub type UiState = app_state::UiState;

/// Normalized map query bounds shared with map projection helpers.
pub type MapQueryBounds = app_state::MapQueryBounds;

/// Browser tab selection state.
pub type SampleBrowserTab = app_state::SampleBrowserTab;

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

/// Browser row filter used by the sample table.
pub type TriageFlagFilter = app_state::TriageFlagFilter;

/// Update status exposed by migration-facing projections.
pub type UpdateStatus = app_state::UpdateStatus;

/// Map rendering mode exposed by migration-facing projections.
pub type MapRenderMode = app_state::MapRenderMode;

/// UI status tone used for app-level status messages.
pub type StatusTone = app_state::StatusTone;

/// Pending inline sample rename prompt.
pub type SampleBrowserActionPrompt = app_state::SampleBrowserActionPrompt;

/// Pending inline folder rename/create prompt.
pub type FolderActionPrompt = app_state::FolderActionPrompt;

/// Unified drag target used by migration-facing drag/drop projections.
pub type DragTarget = app_state::DragTarget;

/// Browser sort mode used by migration-facing projections.
pub type SampleBrowserSort = app_state::SampleBrowserSort;

/// Visible row projection used by migration-facing helpers.
pub type VisibleRows = app_state::VisibleRows;

/// Destructive edit action exposed by prompt surfaces.
pub type DestructiveSelectionEdit = app_state::DestructiveSelectionEdit;

/// Prompt model for destructive edits.
pub type DestructiveEditPrompt = app_state::DestructiveEditPrompt;

/// Folder-filter scope for source root traversal.
pub type RootFolderFilterMode = app_state::RootFolderFilterMode;

/// Folder row view projection for migration-facing renderer models.
pub type FolderRowView = app_state::FolderRowView;

/// Inline folder creation prompt state.
pub type InlineFolderCreation = app_state::InlineFolderCreation;

/// Recovery action for staged folder delete.
pub type FolderDeleteRecoveryAction = app_state::FolderDeleteRecoveryAction;

/// Recovery status for staged folder delete.
pub type FolderDeleteRecoveryStatus = app_state::FolderDeleteRecoveryStatus;

/// Recovery row entry for staged folder delete.
pub type FolderDeleteRecoveryEntry = app_state::FolderDeleteRecoveryEntry;

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
