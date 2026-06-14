use std::path::PathBuf;

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
