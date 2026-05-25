use std::path::PathBuf;

use super::FolderEntry;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SourceEntry {
    pub(super) id: String,
    pub(super) label: String,
    pub(super) root: PathBuf,
    pub(super) root_folder: Option<FolderEntry>,
    pub(super) loading_task: Option<u64>,
}

impl SourceEntry {
    pub(super) fn new(id: impl Into<String>, label: impl Into<String>, root: PathBuf) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            root,
            root_folder: None,
            loading_task: None,
        }
    }

    pub(super) fn is_default_assets_source(&self) -> bool {
        self.id == "assets" && self.root.ends_with("assets")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FolderRenameEdit {
    pub(super) folder_id: String,
    pub(super) draft: String,
    pub(super) input_id: u64,
    pub(super) kind: FolderRenameKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum FolderRenameKind {
    Rename,
    Create { parent_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FileRenameEdit {
    pub(super) file_id: String,
    pub(super) draft: String,
    pub(super) input_id: u64,
    pub(super) selection_start: usize,
    pub(super) selection_end: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::gui_app) struct FileColumn {
    pub(in crate::gui_app) id: String,
    pub(in crate::gui_app) label: String,
    pub(in crate::gui_app) width: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct FileColumnResize {
    pub(super) column_id: String,
    pub(super) start_x: f32,
    pub(super) start_width: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum FolderBrowserDrag {
    Folder { folder_id: String },
    Files { file_ids: Vec<String> },
    ExtractedFile { path: PathBuf },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct VisibleFolder {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) depth: usize,
    pub(super) has_children: bool,
    pub(super) expanded: bool,
    pub(super) selected: bool,
    pub(super) drag_active: bool,
    pub(super) drag_source: bool,
    pub(super) drop_candidate: bool,
    pub(super) drop_target: bool,
    pub(super) drop_target_active: bool,
    pub(super) rename_draft: Option<String>,
    pub(super) rename_input_id: Option<u64>,
}

pub(super) fn default_file_columns() -> Vec<FileColumn> {
    vec![
        file_column("name", "Name", 240.0),
        file_column("extension", "Ext", 54.0),
        file_column("size", "Size", 78.0),
        file_column("modified", "Modified", 112.0),
    ]
}

fn file_column(id: &str, label: &str, width: f32) -> FileColumn {
    FileColumn {
        id: id.to_owned(),
        label: label.to_owned(),
        width,
    }
}
