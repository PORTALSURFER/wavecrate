//! Backend-neutral state aliases for migration consumers.
//!
//! These aliases keep host-facing migration code (`app_core` projections and
//! native bridge action routing) independent from direct `app::state`
//! module paths while the legacy controller internals are incrementally
//! extracted.

/// Alias for the legacy UI state root during migration.
pub type UiState = crate::app::state::UiState;
/// Alias for map query bounds in similarity map projection.
pub type MapQueryBounds = crate::app::state::MapQueryBounds;
/// Browser tab selection state used by migration-facing consumers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleBrowserTab {
    /// List/table browser tab.
    List,
    /// Similarity map browser tab.
    Map,
}

impl From<crate::app::state::SampleBrowserTab> for SampleBrowserTab {
    fn from(value: crate::app::state::SampleBrowserTab) -> Self {
        match value {
            crate::app::state::SampleBrowserTab::List => Self::List,
            crate::app::state::SampleBrowserTab::Map => Self::Map,
        }
    }
}

impl From<SampleBrowserTab> for crate::app::state::SampleBrowserTab {
    fn from(value: SampleBrowserTab) -> Self {
        match value {
            SampleBrowserTab::List => Self::List,
            SampleBrowserTab::Map => Self::Map,
        }
    }
}

/// Browser triage columns used in migration-facing drag/drop projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriageFlagColumn {
    /// Trash column.
    Trash,
    /// Neutral column.
    Neutral,
    /// Keep column.
    Keep,
}

impl From<crate::app::state::TriageFlagColumn> for TriageFlagColumn {
    fn from(value: crate::app::state::TriageFlagColumn) -> Self {
        match value {
            crate::app::state::TriageFlagColumn::Trash => Self::Trash,
            crate::app::state::TriageFlagColumn::Neutral => Self::Neutral,
            crate::app::state::TriageFlagColumn::Keep => Self::Keep,
        }
    }
}

impl From<TriageFlagColumn> for crate::app::state::TriageFlagColumn {
    fn from(value: TriageFlagColumn) -> Self {
        match value {
            TriageFlagColumn::Trash => Self::Trash,
            TriageFlagColumn::Neutral => Self::Neutral,
            TriageFlagColumn::Keep => Self::Keep,
        }
    }
}

/// Update status surfaced by migration-facing render projections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateStatus {
    /// No update activity in progress.
    Idle,
    /// Update check in progress.
    Checking,
    /// A newer update is available.
    UpdateAvailable,
    /// Update check failed.
    Error,
}

impl From<crate::app::state::UpdateStatus> for UpdateStatus {
    fn from(value: crate::app::state::UpdateStatus) -> Self {
        match value {
            crate::app::state::UpdateStatus::Idle => Self::Idle,
            crate::app::state::UpdateStatus::Checking => Self::Checking,
            crate::app::state::UpdateStatus::UpdateAvailable => Self::UpdateAvailable,
            crate::app::state::UpdateStatus::Error => Self::Error,
        }
    }
}

impl From<UpdateStatus> for crate::app::state::UpdateStatus {
    fn from(value: UpdateStatus) -> Self {
        match value {
            UpdateStatus::Idle => Self::Idle,
            UpdateStatus::Checking => Self::Checking,
            UpdateStatus::UpdateAvailable => Self::UpdateAvailable,
            UpdateStatus::Error => Self::Error,
        }
    }
}

/// Map render mode used by migration-facing render projections.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapRenderMode {
    /// Render a density heatmap.
    Heatmap,
    /// Render individual points.
    Points,
}

impl From<crate::app::state::MapRenderMode> for MapRenderMode {
    fn from(value: crate::app::state::MapRenderMode) -> Self {
        match value {
            crate::app::state::MapRenderMode::Heatmap => Self::Heatmap,
            crate::app::state::MapRenderMode::Points => Self::Points,
        }
    }
}

impl From<MapRenderMode> for crate::app::state::MapRenderMode {
    fn from(value: MapRenderMode) -> Self {
        match value {
            MapRenderMode::Heatmap => Self::Heatmap,
            MapRenderMode::Points => Self::Points,
        }
    }
}
/// Alias for browser inline action prompts.
pub type SampleBrowserActionPrompt = crate::app::state::SampleBrowserActionPrompt;
/// Alias for folder action prompts.
pub type FolderActionPrompt = crate::app::state::FolderActionPrompt;
/// Alias for drag-and-drop target state.
pub type DragTarget = crate::app::state::DragTarget;
/// Browser sort mode used by migration-facing projections and bridge actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleBrowserSort {
    /// Preserve the original list order.
    ListOrder,
    /// Sort by similarity score.
    Similarity,
    /// Sort by playback age ascending.
    PlaybackAgeAsc,
    /// Sort by playback age descending.
    PlaybackAgeDesc,
}

impl From<crate::app::state::SampleBrowserSort> for SampleBrowserSort {
    fn from(value: crate::app::state::SampleBrowserSort) -> Self {
        match value {
            crate::app::state::SampleBrowserSort::ListOrder => Self::ListOrder,
            crate::app::state::SampleBrowserSort::Similarity => Self::Similarity,
            crate::app::state::SampleBrowserSort::PlaybackAgeAsc => Self::PlaybackAgeAsc,
            crate::app::state::SampleBrowserSort::PlaybackAgeDesc => Self::PlaybackAgeDesc,
        }
    }
}

impl From<SampleBrowserSort> for crate::app::state::SampleBrowserSort {
    fn from(value: SampleBrowserSort) -> Self {
        match value {
            SampleBrowserSort::ListOrder => Self::ListOrder,
            SampleBrowserSort::Similarity => Self::Similarity,
            SampleBrowserSort::PlaybackAgeAsc => Self::PlaybackAgeAsc,
            SampleBrowserSort::PlaybackAgeDesc => Self::PlaybackAgeDesc,
        }
    }
}

/// Visible row projection used by migration-facing browser helpers.
#[derive(Clone, Debug)]
pub enum VisibleRows {
    /// All rows are visible; total stores the count.
    All {
        /// Total number of rows.
        total: usize,
    },
    /// Only the provided indices are visible.
    List(Vec<usize>),
}

impl VisibleRows {
    /// Return the number of visible rows represented by this projection.
    pub fn len(&self) -> usize {
        match self {
            VisibleRows::All { total } => *total,
            VisibleRows::List(rows) => rows.len(),
        }
    }

    /// Resolve a visible row index to an absolute row index.
    pub fn get(&self, row: usize) -> Option<usize> {
        match self {
            VisibleRows::All { total } => (row < *total).then_some(row),
            VisibleRows::List(rows) => rows.get(row).copied(),
        }
    }
}

impl From<crate::app::state::VisibleRows> for VisibleRows {
    fn from(value: crate::app::state::VisibleRows) -> Self {
        match value {
            crate::app::state::VisibleRows::All { total } => Self::All { total },
            crate::app::state::VisibleRows::List(rows) => Self::List(rows),
        }
    }
}

impl From<VisibleRows> for crate::app::state::VisibleRows {
    fn from(value: VisibleRows) -> Self {
        match value {
            VisibleRows::All { total } => Self::All { total },
            VisibleRows::List(rows) => Self::List(rows),
        }
    }
}
/// Alias for destructive edit prompt state.
pub type DestructiveEditPrompt = crate::app::state::DestructiveEditPrompt;
/// Alias for destructive waveform edit enum.
pub type DestructiveSelectionEdit = crate::app::state::DestructiveSelectionEdit;
/// Alias for inline folder creation state.
pub type InlineFolderCreation = crate::app::state::InlineFolderCreation;
/// Alias for folder row projection state.
pub type FolderRowView = crate::app::state::FolderRowView;
/// Alias for folder delete recovery entry state.
pub type FolderDeleteRecoveryEntry = crate::app::state::FolderDeleteRecoveryEntry;
/// Alias for folder delete recovery action state.
pub type FolderDeleteRecoveryAction = crate::app::state::FolderDeleteRecoveryAction;
/// Alias for folder delete recovery status state.
pub type FolderDeleteRecoveryStatus = crate::app::state::FolderDeleteRecoveryStatus;
