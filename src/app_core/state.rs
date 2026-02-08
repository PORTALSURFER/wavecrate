//! Backend-neutral state types for migration consumers.
//!
//! These aliases keep host-facing migration code (`app_core` projections and
//! native bridge action routing) independent from direct `app::state`
//! module paths while the legacy controller internals are incrementally
//! extracted.

use crate::app_core::actions::NativeBrowserTagTarget;
use crate::app::state as legacy_state;

/// Transitional UI state alias used by migration-facing projection code.
pub type UiState = legacy_state::UiState;

/// Bounds used to query visible points in the map projection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapQueryBounds {
    /// Minimum X coordinate for query.
    pub min_x: f32,
    /// Maximum X coordinate for query.
    pub max_x: f32,
    /// Minimum Y coordinate for query.
    pub min_y: f32,
    /// Maximum Y coordinate for query.
    pub max_y: f32,
}

impl From<legacy_state::MapQueryBounds> for MapQueryBounds {
    fn from(value: legacy_state::MapQueryBounds) -> Self {
        Self {
            min_x: value.min_x,
            max_x: value.max_x,
            min_y: value.min_y,
            max_y: value.max_y,
        }
    }
}

impl From<MapQueryBounds> for legacy_state::MapQueryBounds {
    fn from(value: MapQueryBounds) -> Self {
        Self {
            min_x: value.min_x,
            max_x: value.max_x,
            min_y: value.min_y,
            max_y: value.max_y,
        }
    }
}
/// Browser tab selection state used by migration-facing consumers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleBrowserTab {
    /// List/table browser tab.
    List,
    /// Similarity map browser tab.
    Map,
}

impl From<legacy_state::SampleBrowserTab> for SampleBrowserTab {
    fn from(value: legacy_state::SampleBrowserTab) -> Self {
        match value {
            legacy_state::SampleBrowserTab::List => Self::List,
            legacy_state::SampleBrowserTab::Map => Self::Map,
        }
    }
}

impl From<SampleBrowserTab> for legacy_state::SampleBrowserTab {
    fn from(value: SampleBrowserTab) -> Self {
        match value {
            SampleBrowserTab::List => Self::List,
            SampleBrowserTab::Map => Self::Map,
        }
    }
}

/// Browser tag targets used by migration-facing action routing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserTagTarget {
    /// Mark selection as trash.
    Trash,
    /// Mark selection as neutral.
    Neutral,
    /// Mark selection as keep.
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

impl From<legacy_state::TriageFlagColumn> for TriageFlagColumn {
    fn from(value: legacy_state::TriageFlagColumn) -> Self {
        match value {
            legacy_state::TriageFlagColumn::Trash => Self::Trash,
            legacy_state::TriageFlagColumn::Neutral => Self::Neutral,
            legacy_state::TriageFlagColumn::Keep => Self::Keep,
        }
    }
}

impl From<TriageFlagColumn> for legacy_state::TriageFlagColumn {
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

impl From<legacy_state::UpdateStatus> for UpdateStatus {
    fn from(value: legacy_state::UpdateStatus) -> Self {
        match value {
            legacy_state::UpdateStatus::Idle => Self::Idle,
            legacy_state::UpdateStatus::Checking => Self::Checking,
            legacy_state::UpdateStatus::UpdateAvailable => Self::UpdateAvailable,
            legacy_state::UpdateStatus::Error => Self::Error,
        }
    }
}

impl From<UpdateStatus> for legacy_state::UpdateStatus {
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

impl From<legacy_state::MapRenderMode> for MapRenderMode {
    fn from(value: legacy_state::MapRenderMode) -> Self {
        match value {
            legacy_state::MapRenderMode::Heatmap => Self::Heatmap,
            legacy_state::MapRenderMode::Points => Self::Points,
        }
    }
}

impl From<MapRenderMode> for legacy_state::MapRenderMode {
    fn from(value: MapRenderMode) -> Self {
        match value {
            MapRenderMode::Heatmap => Self::Heatmap,
            MapRenderMode::Points => Self::Points,
        }
    }
}
/// Pending inline action for the sample browser.
#[derive(Clone, Debug)]
pub enum SampleBrowserActionPrompt {
    /// Rename the selected entry.
    Rename {
        /// Path to rename.
        target: std::path::PathBuf,
        /// New name.
        name: String,
    },
}

impl From<legacy_state::SampleBrowserActionPrompt> for SampleBrowserActionPrompt {
    fn from(value: legacy_state::SampleBrowserActionPrompt) -> Self {
        match value {
            legacy_state::SampleBrowserActionPrompt::Rename { target, name } => {
                Self::Rename { target, name }
            }
        }
    }
}

impl From<SampleBrowserActionPrompt> for legacy_state::SampleBrowserActionPrompt {
    fn from(value: SampleBrowserActionPrompt) -> Self {
        match value {
            SampleBrowserActionPrompt::Rename { target, name } => Self::Rename { target, name },
        }
    }
}

/// Pending inline action for the folder browser.
#[derive(Clone, Debug)]
pub enum FolderActionPrompt {
    /// Rename the target folder.
    Rename {
        /// Folder path to rename.
        target: std::path::PathBuf,
        /// New folder name.
        name: String,
    },
}

impl From<legacy_state::FolderActionPrompt> for FolderActionPrompt {
    fn from(value: legacy_state::FolderActionPrompt) -> Self {
        match value {
            legacy_state::FolderActionPrompt::Rename { target, name } => {
                Self::Rename { target, name }
            }
        }
    }
}

impl From<FolderActionPrompt> for legacy_state::FolderActionPrompt {
    fn from(value: FolderActionPrompt) -> Self {
        match value {
            FolderActionPrompt::Rename { target, name } => Self::Rename { target, name },
        }
    }
}
/// Unified drag target variants projected for migration-facing views.
#[derive(Clone, Debug, PartialEq)]
pub enum DragTarget {
    /// No active target.
    None,
    /// Browser triage column target.
    BrowserTriage(TriageFlagColumn),
    /// Sources row target.
    SourcesRow(crate::sample_sources::SourceId),
    /// Folder panel target (optional path).
    FolderPanel {
        /// Optional folder path hovered.
        folder: Option<std::path::PathBuf>,
    },
    /// Drop target row.
    DropTarget {
        /// Path for the drop target.
        path: std::path::PathBuf,
    },
    /// Drop targets panel background.
    DropTargetsPanel,
    /// External target outside the app.
    External,
}

impl From<legacy_state::DragTarget> for DragTarget {
    fn from(value: legacy_state::DragTarget) -> Self {
        match value {
            legacy_state::DragTarget::None => Self::None,
            legacy_state::DragTarget::BrowserTriage(column) => {
                Self::BrowserTriage(column.into())
            }
            legacy_state::DragTarget::SourcesRow(id) => Self::SourcesRow(id),
            legacy_state::DragTarget::FolderPanel { folder } => Self::FolderPanel { folder },
            legacy_state::DragTarget::DropTarget { path } => Self::DropTarget { path },
            legacy_state::DragTarget::DropTargetsPanel => Self::DropTargetsPanel,
            legacy_state::DragTarget::External => Self::External,
        }
    }
}

impl From<DragTarget> for legacy_state::DragTarget {
    fn from(value: DragTarget) -> Self {
        match value {
            DragTarget::None => Self::None,
            DragTarget::BrowserTriage(column) => Self::BrowserTriage(column.into()),
            DragTarget::SourcesRow(id) => Self::SourcesRow(id),
            DragTarget::FolderPanel { folder } => Self::FolderPanel { folder },
            DragTarget::DropTarget { path } => Self::DropTarget { path },
            DragTarget::DropTargetsPanel => Self::DropTargetsPanel,
            DragTarget::External => Self::External,
        }
    }
}
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

impl From<legacy_state::SampleBrowserSort> for SampleBrowserSort {
    fn from(value: legacy_state::SampleBrowserSort) -> Self {
        match value {
            legacy_state::SampleBrowserSort::ListOrder => Self::ListOrder,
            legacy_state::SampleBrowserSort::Similarity => Self::Similarity,
            legacy_state::SampleBrowserSort::PlaybackAgeAsc => Self::PlaybackAgeAsc,
            legacy_state::SampleBrowserSort::PlaybackAgeDesc => Self::PlaybackAgeDesc,
        }
    }
}

impl From<SampleBrowserSort> for legacy_state::SampleBrowserSort {
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

impl From<legacy_state::VisibleRows> for VisibleRows {
    fn from(value: legacy_state::VisibleRows) -> Self {
        match value {
            legacy_state::VisibleRows::All { total } => Self::All { total },
            legacy_state::VisibleRows::List(rows) => Self::List(rows),
        }
    }
}

impl From<VisibleRows> for legacy_state::VisibleRows {
    fn from(value: VisibleRows) -> Self {
        match value {
            VisibleRows::All { total } => Self::All { total },
            VisibleRows::List(rows) => Self::List(rows),
        }
    }
}
/// Destructive selection edits that overwrite audio on disk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DestructiveSelectionEdit {
    /// Crop the selection and discard the rest.
    CropSelection,
    /// Trim everything outside the selection.
    TrimSelection,
    /// Reverse the selected audio.
    ReverseSelection,
    /// Apply a left-to-right fade.
    FadeLeftToRight,
    /// Apply a right-to-left fade.
    FadeRightToLeft,
    /// Apply short fade-in/out ramps at the selection edges to reduce clicks.
    ShortEdgeFades,
    /// Mute the selection.
    MuteSelection,
    /// Normalize the selection.
    NormalizeSelection,
    /// Attempt to remove clicks in the selection.
    ClickRemoval,
}

impl From<legacy_state::DestructiveSelectionEdit> for DestructiveSelectionEdit {
    fn from(value: legacy_state::DestructiveSelectionEdit) -> Self {
        match value {
            legacy_state::DestructiveSelectionEdit::CropSelection => Self::CropSelection,
            legacy_state::DestructiveSelectionEdit::TrimSelection => Self::TrimSelection,
            legacy_state::DestructiveSelectionEdit::ReverseSelection => Self::ReverseSelection,
            legacy_state::DestructiveSelectionEdit::FadeLeftToRight => Self::FadeLeftToRight,
            legacy_state::DestructiveSelectionEdit::FadeRightToLeft => Self::FadeRightToLeft,
            legacy_state::DestructiveSelectionEdit::ShortEdgeFades => Self::ShortEdgeFades,
            legacy_state::DestructiveSelectionEdit::MuteSelection => Self::MuteSelection,
            legacy_state::DestructiveSelectionEdit::NormalizeSelection => Self::NormalizeSelection,
            legacy_state::DestructiveSelectionEdit::ClickRemoval => Self::ClickRemoval,
        }
    }
}

impl From<DestructiveSelectionEdit> for legacy_state::DestructiveSelectionEdit {
    fn from(value: DestructiveSelectionEdit) -> Self {
        match value {
            DestructiveSelectionEdit::CropSelection => Self::CropSelection,
            DestructiveSelectionEdit::TrimSelection => Self::TrimSelection,
            DestructiveSelectionEdit::ReverseSelection => Self::ReverseSelection,
            DestructiveSelectionEdit::FadeLeftToRight => Self::FadeLeftToRight,
            DestructiveSelectionEdit::FadeRightToLeft => Self::FadeRightToLeft,
            DestructiveSelectionEdit::ShortEdgeFades => Self::ShortEdgeFades,
            DestructiveSelectionEdit::MuteSelection => Self::MuteSelection,
            DestructiveSelectionEdit::NormalizeSelection => Self::NormalizeSelection,
            DestructiveSelectionEdit::ClickRemoval => Self::ClickRemoval,
        }
    }
}

/// Confirmation prompt content for destructive edits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DestructiveEditPrompt {
    /// Edit type that will be applied.
    pub edit: DestructiveSelectionEdit,
    /// Prompt title text.
    pub title: String,
    /// Prompt body text.
    pub message: String,
}

impl From<legacy_state::DestructiveEditPrompt> for DestructiveEditPrompt {
    fn from(value: legacy_state::DestructiveEditPrompt) -> Self {
        Self {
            edit: value.edit.into(),
            title: value.title,
            message: value.message,
        }
    }
}

impl From<DestructiveEditPrompt> for legacy_state::DestructiveEditPrompt {
    fn from(value: DestructiveEditPrompt) -> Self {
        Self {
            edit: value.edit.into(),
            title: value.title,
            message: value.message,
        }
    }
}
/// Root selection behavior for the folder browser.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RootFolderFilterMode {
    /// Root selection includes all descendants.
    #[default]
    AllDescendants,
    /// Root selection includes only files at the source root.
    RootOnly,
}

impl From<legacy_state::RootFolderFilterMode> for RootFolderFilterMode {
    fn from(value: legacy_state::RootFolderFilterMode) -> Self {
        match value {
            legacy_state::RootFolderFilterMode::AllDescendants => Self::AllDescendants,
            legacy_state::RootFolderFilterMode::RootOnly => Self::RootOnly,
        }
    }
}

impl From<RootFolderFilterMode> for legacy_state::RootFolderFilterMode {
    fn from(value: RootFolderFilterMode) -> Self {
        match value {
            RootFolderFilterMode::AllDescendants => Self::AllDescendants,
            RootFolderFilterMode::RootOnly => Self::RootOnly,
        }
    }
}

/// Render-friendly folder row.
#[derive(Clone, Debug)]
pub struct FolderRowView {
    /// Full path for the folder.
    pub path: std::path::PathBuf,
    /// Display name.
    pub name: String,
    /// Depth in the tree.
    pub depth: usize,
    /// Whether the folder has children.
    pub has_children: bool,
    /// Whether the folder is expanded.
    pub expanded: bool,
    /// Whether the folder is selected.
    pub selected: bool,
    /// Whether the folder is negated in filters.
    pub negated: bool,
    /// Optional hotkey number.
    pub hotkey: Option<u8>,
    /// Whether this row represents the root.
    pub is_root: bool,
    /// Root filter mode when this row represents the root.
    pub root_filter_mode: Option<RootFolderFilterMode>,
}

impl From<legacy_state::FolderRowView> for FolderRowView {
    fn from(value: legacy_state::FolderRowView) -> Self {
        Self {
            path: value.path,
            name: value.name,
            depth: value.depth,
            has_children: value.has_children,
            expanded: value.expanded,
            selected: value.selected,
            negated: value.negated,
            hotkey: value.hotkey,
            is_root: value.is_root,
            root_filter_mode: value.root_filter_mode.map(Into::into),
        }
    }
}

impl From<FolderRowView> for legacy_state::FolderRowView {
    fn from(value: FolderRowView) -> Self {
        Self {
            path: value.path,
            name: value.name,
            depth: value.depth,
            has_children: value.has_children,
            expanded: value.expanded,
            selected: value.selected,
            negated: value.negated,
            hotkey: value.hotkey,
            is_root: value.is_root,
            root_filter_mode: value.root_filter_mode.map(Into::into),
        }
    }
}

/// Inline editor state for a pending folder creation.
#[derive(Clone, Debug)]
pub struct InlineFolderCreation {
    /// Parent folder path.
    pub parent: std::path::PathBuf,
    /// New folder name.
    pub name: String,
    /// Whether the input should be focused.
    pub focus_requested: bool,
}

impl From<legacy_state::InlineFolderCreation> for InlineFolderCreation {
    fn from(value: legacy_state::InlineFolderCreation) -> Self {
        Self {
            parent: value.parent,
            name: value.name,
            focus_requested: value.focus_requested,
        }
    }
}

impl From<InlineFolderCreation> for legacy_state::InlineFolderCreation {
    fn from(value: InlineFolderCreation) -> Self {
        Self {
            parent: value.parent,
            name: value.name,
            focus_requested: value.focus_requested,
        }
    }
}

/// Recovery action taken for a staged delete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderDeleteRecoveryAction {
    /// Restore the staged folder into the source.
    Restore,
    /// Finalize the staged delete by removing the folder.
    Finalize,
}

impl From<legacy_state::FolderDeleteRecoveryAction> for FolderDeleteRecoveryAction {
    fn from(value: legacy_state::FolderDeleteRecoveryAction) -> Self {
        match value {
            legacy_state::FolderDeleteRecoveryAction::Restore => Self::Restore,
            legacy_state::FolderDeleteRecoveryAction::Finalize => Self::Finalize,
        }
    }
}

impl From<FolderDeleteRecoveryAction> for legacy_state::FolderDeleteRecoveryAction {
    fn from(value: FolderDeleteRecoveryAction) -> Self {
        match value {
            FolderDeleteRecoveryAction::Restore => Self::Restore,
            FolderDeleteRecoveryAction::Finalize => Self::Finalize,
        }
    }
}

/// Recovery outcome for a staged delete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderDeleteRecoveryStatus {
    /// Recovery action succeeded.
    Completed,
    /// Recovery action failed.
    Failed,
}

impl From<legacy_state::FolderDeleteRecoveryStatus> for FolderDeleteRecoveryStatus {
    fn from(value: legacy_state::FolderDeleteRecoveryStatus) -> Self {
        match value {
            legacy_state::FolderDeleteRecoveryStatus::Completed => Self::Completed,
            legacy_state::FolderDeleteRecoveryStatus::Failed => Self::Failed,
        }
    }
}

impl From<FolderDeleteRecoveryStatus> for legacy_state::FolderDeleteRecoveryStatus {
    fn from(value: FolderDeleteRecoveryStatus) -> Self {
        match value {
            FolderDeleteRecoveryStatus::Completed => Self::Completed,
            FolderDeleteRecoveryStatus::Failed => Self::Failed,
        }
    }
}

/// Display entry for a recovered staged delete.
#[derive(Clone, Debug)]
pub struct FolderDeleteRecoveryEntry {
    /// Display label for the source.
    pub source_label: String,
    /// Original folder path relative to the source root.
    pub relative_path: std::path::PathBuf,
    /// Action taken during recovery.
    pub action: FolderDeleteRecoveryAction,
    /// Outcome of the recovery attempt.
    pub status: FolderDeleteRecoveryStatus,
    /// Optional extra detail for the UI.
    pub detail: Option<String>,
}

impl From<legacy_state::FolderDeleteRecoveryEntry> for FolderDeleteRecoveryEntry {
    fn from(value: legacy_state::FolderDeleteRecoveryEntry) -> Self {
        Self {
            source_label: value.source_label,
            relative_path: value.relative_path,
            action: value.action.into(),
            status: value.status.into(),
            detail: value.detail,
        }
    }
}

impl From<FolderDeleteRecoveryEntry> for legacy_state::FolderDeleteRecoveryEntry {
    fn from(value: FolderDeleteRecoveryEntry) -> Self {
        Self {
            source_label: value.source_label,
            relative_path: value.relative_path,
            action: value.action.into(),
            status: value.status.into(),
            detail: value.detail,
        }
    }
}
