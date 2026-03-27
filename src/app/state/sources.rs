use crate::sample_sources::{SourceId, WavEntry};
use std::path::PathBuf;

/// Sidebar list of sample sources.
#[derive(Clone, Debug, Default)]
pub struct SourcePanelState {
    /// Render rows for configured sources.
    pub rows: Vec<SourceRowView>,
    /// Currently selected row index.
    pub selected: Option<usize>,
    /// Row index with an open context menu.
    pub menu_row: Option<usize>,
    /// Row index to scroll into view.
    pub scroll_to: Option<usize>,
    /// User-defined height for the sources list section, excluding its header.
    pub sources_height_override: Option<f32>,
    /// Cached list height at the start of a sources resize drag for stable deltas.
    pub sources_resize_origin_height: Option<f32>,
    /// Folder browser sub-state.
    pub folders: FolderBrowserUiState,
    /// Drop target sub-state.
    pub drop_targets: DropTargetsUiState,
}

/// Display data for a single source row.
#[derive(Clone, Debug)]
pub struct SourceRowView {
    /// Source identifier.
    pub id: SourceId,
    /// Display name.
    pub name: String,
    /// Display path.
    pub path: String,
    /// Whether the source is missing on disk.
    pub missing: bool,
}

/// UI state for browsing folders within the active source.
#[derive(Clone, Debug)]
pub struct FolderBrowserUiState {
    /// Render rows for the folder tree.
    pub rows: Vec<FolderRowView>,
    /// Currently focused row index.
    pub focused: Option<usize>,
    /// Row index to scroll into view.
    pub scroll_to: Option<usize>,
    /// Previously focused path for restore.
    pub last_focused_path: Option<PathBuf>,
    /// Active search query.
    pub search_query: String,
    /// Whether the tree should include folders without any WAV-backed samples.
    pub show_all_folders: bool,
    /// Whether search focus is requested.
    pub search_focus_requested: bool,
    /// Pending folder action prompt.
    pub pending_action: Option<FolderActionPrompt>,
    /// Inline folder edit state for create or rename flows.
    pub inline_edit: Option<InlineFolderEdit>,
    /// Cached header height for the folder browser section.
    pub header_height: f32,
    /// Delete recovery queue state for staged folder deletes.
    pub delete_recovery: FolderDeleteRecoveryUiState,
}

impl Default for FolderBrowserUiState {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            focused: None,
            scroll_to: None,
            last_focused_path: None,
            search_query: String::new(),
            show_all_folders: true,
            search_focus_requested: false,
            pending_action: None,
            inline_edit: None,
            header_height: 0.0,
            delete_recovery: FolderDeleteRecoveryUiState::default(),
        }
    }
}

/// UI state for staged delete recovery.
#[derive(Clone, Debug, Default)]
pub struct FolderDeleteRecoveryUiState {
    /// Whether recovery is currently running in the background.
    pub in_progress: bool,
    /// Entries reported by the last recovery run.
    pub entries: Vec<FolderDeleteRecoveryEntry>,
    /// Retained folder deletes that can still be restored or purged explicitly.
    pub retained_entries: Vec<RetainedFolderDeleteEntry>,
}

/// Display entry for a recovered staged delete.
#[derive(Clone, Debug)]
pub struct FolderDeleteRecoveryEntry {
    /// Display label for the source.
    pub source_label: String,
    /// Original folder path relative to the source root.
    pub relative_path: PathBuf,
    /// Action taken during recovery.
    pub action: FolderDeleteRecoveryAction,
    /// Outcome of the recovery attempt.
    pub status: FolderDeleteRecoveryStatus,
    /// Optional extra detail for the UI.
    pub detail: Option<String>,
}

/// Recoverable retained folder delete stored in the app-owned staging area.
#[derive(Clone, Debug)]
pub struct RetainedFolderDeleteEntry {
    /// Stable journal identifier for the retained delete.
    pub id: String,
    /// Source identifier that owns the retained delete.
    pub source_id: SourceId,
    /// Source root path that owns the retained delete.
    pub source_root: PathBuf,
    /// Display label for the source in the UI.
    pub source_label: String,
    /// Original folder path relative to the source root.
    pub relative_path: PathBuf,
    /// Relative path of the staged folder inside `.sempal_delete_staging`.
    pub staged_relative: PathBuf,
    /// Snapshot of deleted wav metadata used to restore DB state after restart.
    pub deleted_entries: Vec<WavEntry>,
}

/// Recovery action taken for a staged delete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderDeleteRecoveryAction {
    /// Restore the staged folder into the source.
    Restore,
    /// Finalize the staged delete by removing the folder.
    Finalize,
}

/// Recovery outcome for a staged delete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FolderDeleteRecoveryStatus {
    /// Recovery action succeeded.
    Completed,
    /// Recovery action failed.
    Failed,
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

impl RootFolderFilterMode {
    /// Toggle between showing all descendants and root-only results.
    pub(crate) fn toggle(self) -> Self {
        match self {
            Self::AllDescendants => Self::RootOnly,
            Self::RootOnly => Self::AllDescendants,
        }
    }
}

/// Render-friendly folder row.
#[derive(Clone, Debug)]
pub struct FolderRowView {
    /// Full path for the folder.
    pub path: PathBuf,
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

/// Pending inline action for the folder browser.
#[derive(Clone, Debug)]
pub enum FolderActionPrompt {
    /// Confirm restoring all retained folder deletes currently tracked in Recovery.
    RestoreRetainedDeletes {
        /// Number of retained folder deletes that will be restored.
        entry_count: usize,
    },
    /// Confirm purging all retained folder deletes currently tracked in Recovery.
    PurgeRetainedDeletes {
        /// Number of retained folder deletes that will be purged permanently.
        entry_count: usize,
    },
}

/// Kind of inline folder edit currently shown in the folder tree.
#[derive(Clone, Debug)]
pub enum InlineFolderEditKind {
    /// Create one new folder under the provided parent path.
    Create {
        /// Parent folder path.
        parent: PathBuf,
    },
    /// Rename one existing folder in place.
    Rename {
        /// Folder path to rename.
        target: PathBuf,
    },
}

/// Inline editor state for a pending folder create or rename action.
#[derive(Clone, Debug)]
pub struct InlineFolderEdit {
    /// Stable path context describing the active inline folder action.
    pub kind: InlineFolderEditKind,
    /// Current folder-name input value.
    pub name: String,
    /// Whether the input should be focused.
    pub focus_requested: bool,
    /// Whether the next input activation should select all text once.
    pub select_all_on_focus_requested: bool,
}

/// Sidebar list of configured drop targets.
#[derive(Clone, Debug, Default)]
pub struct DropTargetsUiState {
    /// Render rows for drop targets.
    pub rows: Vec<DropTargetRowView>,
    /// Currently selected row index.
    pub selected: Option<usize>,
    /// Row index with an open context menu.
    pub menu_row: Option<usize>,
    /// Row index to scroll into view.
    pub scroll_to: Option<usize>,
    /// User-defined height for the drop targets section, in points.
    pub height_override: Option<f32>,
    /// Cached height at the start of a resize drag for stable deltas.
    pub resize_origin_height: Option<f32>,
    /// Cached header height for the drop targets section.
    pub header_height: f32,
}

/// Display data for a single drop target row.
#[derive(Clone, Debug)]
pub struct DropTargetRowView {
    /// Drop target path.
    pub path: PathBuf,
    /// Display name.
    pub name: String,
    /// Cached label used for drag payloads.
    pub drag_label: String,
    /// Cached display path used in tooltips.
    pub tooltip_path: String,
    /// Whether the drop target path is missing.
    pub missing: bool,
    /// Optional drop target color.
    pub color: Option<crate::sample_sources::config::DropTargetColor>,
}
