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

impl From<crate::app::state::SampleBrowserActionPrompt> for SampleBrowserActionPrompt {
    fn from(value: crate::app::state::SampleBrowserActionPrompt) -> Self {
        match value {
            crate::app::state::SampleBrowserActionPrompt::Rename { target, name } => {
                Self::Rename { target, name }
            }
        }
    }
}

impl From<SampleBrowserActionPrompt> for crate::app::state::SampleBrowserActionPrompt {
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

impl From<crate::app::state::FolderActionPrompt> for FolderActionPrompt {
    fn from(value: crate::app::state::FolderActionPrompt) -> Self {
        match value {
            crate::app::state::FolderActionPrompt::Rename { target, name } => {
                Self::Rename { target, name }
            }
        }
    }
}

impl From<FolderActionPrompt> for crate::app::state::FolderActionPrompt {
    fn from(value: FolderActionPrompt) -> Self {
        match value {
            FolderActionPrompt::Rename { target, name } => Self::Rename { target, name },
        }
    }
}
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

impl From<crate::app::state::DestructiveSelectionEdit> for DestructiveSelectionEdit {
    fn from(value: crate::app::state::DestructiveSelectionEdit) -> Self {
        match value {
            crate::app::state::DestructiveSelectionEdit::CropSelection => Self::CropSelection,
            crate::app::state::DestructiveSelectionEdit::TrimSelection => Self::TrimSelection,
            crate::app::state::DestructiveSelectionEdit::ReverseSelection => Self::ReverseSelection,
            crate::app::state::DestructiveSelectionEdit::FadeLeftToRight => Self::FadeLeftToRight,
            crate::app::state::DestructiveSelectionEdit::FadeRightToLeft => Self::FadeRightToLeft,
            crate::app::state::DestructiveSelectionEdit::ShortEdgeFades => Self::ShortEdgeFades,
            crate::app::state::DestructiveSelectionEdit::MuteSelection => Self::MuteSelection,
            crate::app::state::DestructiveSelectionEdit::NormalizeSelection => Self::NormalizeSelection,
            crate::app::state::DestructiveSelectionEdit::ClickRemoval => Self::ClickRemoval,
        }
    }
}

impl From<DestructiveSelectionEdit> for crate::app::state::DestructiveSelectionEdit {
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

impl From<crate::app::state::DestructiveEditPrompt> for DestructiveEditPrompt {
    fn from(value: crate::app::state::DestructiveEditPrompt) -> Self {
        Self {
            edit: value.edit.into(),
            title: value.title,
            message: value.message,
        }
    }
}

impl From<DestructiveEditPrompt> for crate::app::state::DestructiveEditPrompt {
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

impl From<crate::app::state::RootFolderFilterMode> for RootFolderFilterMode {
    fn from(value: crate::app::state::RootFolderFilterMode) -> Self {
        match value {
            crate::app::state::RootFolderFilterMode::AllDescendants => Self::AllDescendants,
            crate::app::state::RootFolderFilterMode::RootOnly => Self::RootOnly,
        }
    }
}

impl From<RootFolderFilterMode> for crate::app::state::RootFolderFilterMode {
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

impl From<crate::app::state::FolderRowView> for FolderRowView {
    fn from(value: crate::app::state::FolderRowView) -> Self {
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

impl From<FolderRowView> for crate::app::state::FolderRowView {
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

impl From<crate::app::state::InlineFolderCreation> for InlineFolderCreation {
    fn from(value: crate::app::state::InlineFolderCreation) -> Self {
        Self {
            parent: value.parent,
            name: value.name,
            focus_requested: value.focus_requested,
        }
    }
}

impl From<InlineFolderCreation> for crate::app::state::InlineFolderCreation {
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

impl From<crate::app::state::FolderDeleteRecoveryAction> for FolderDeleteRecoveryAction {
    fn from(value: crate::app::state::FolderDeleteRecoveryAction) -> Self {
        match value {
            crate::app::state::FolderDeleteRecoveryAction::Restore => Self::Restore,
            crate::app::state::FolderDeleteRecoveryAction::Finalize => Self::Finalize,
        }
    }
}

impl From<FolderDeleteRecoveryAction> for crate::app::state::FolderDeleteRecoveryAction {
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

impl From<crate::app::state::FolderDeleteRecoveryStatus> for FolderDeleteRecoveryStatus {
    fn from(value: crate::app::state::FolderDeleteRecoveryStatus) -> Self {
        match value {
            crate::app::state::FolderDeleteRecoveryStatus::Completed => Self::Completed,
            crate::app::state::FolderDeleteRecoveryStatus::Failed => Self::Failed,
        }
    }
}

impl From<FolderDeleteRecoveryStatus> for crate::app::state::FolderDeleteRecoveryStatus {
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

impl From<crate::app::state::FolderDeleteRecoveryEntry> for FolderDeleteRecoveryEntry {
    fn from(value: crate::app::state::FolderDeleteRecoveryEntry) -> Self {
        Self {
            source_label: value.source_label,
            relative_path: value.relative_path,
            action: value.action.into(),
            status: value.status.into(),
            detail: value.detail,
        }
    }
}

impl From<FolderDeleteRecoveryEntry> for crate::app::state::FolderDeleteRecoveryEntry {
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
