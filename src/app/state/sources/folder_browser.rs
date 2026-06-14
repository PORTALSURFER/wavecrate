use super::{FolderDeleteRecoveryUiState, InlineFolderEdit};
use std::path::PathBuf;

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
    /// Whether folder filtering includes descendant files in a flattened list.
    pub flattened_view: bool,
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
            show_all_folders: false,
            flattened_view: false,
            search_focus_requested: false,
            pending_action: None,
            inline_edit: None,
            header_height: 0.0,
            delete_recovery: FolderDeleteRecoveryUiState::default(),
        }
    }
}

/// Folder file-scope behavior for the browser filter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FolderFileScopeMode {
    /// Show only files whose immediate parent is the selected folder.
    #[default]
    DirectOnly,
    /// Show files from the selected folder and all descendant folders.
    AllDescendants,
}

impl FolderFileScopeMode {
    /// Toggle between direct-only and descendant-flattened folder matching.
    pub(crate) fn toggle(self) -> Self {
        match self {
            Self::DirectOnly => Self::AllDescendants,
            Self::AllDescendants => Self::DirectOnly,
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
    /// Folder file-scope mode when this row represents the root.
    pub file_scope_mode: Option<FolderFileScopeMode>,
}

/// Pending inline action for the folder browser.
#[derive(Clone, Debug)]
pub enum FolderActionPrompt {
    /// Confirm deleting one folder.
    Delete {
        /// Folder path selected for deletion when the prompt was opened.
        target: PathBuf,
    },
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
