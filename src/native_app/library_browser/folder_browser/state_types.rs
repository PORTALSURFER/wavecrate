use std::path::PathBuf;

use super::FolderEntry;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct SourceEntry {
    pub(in crate::native_app) id: String,
    pub(in crate::native_app) label: String,
    pub(super) root: PathBuf,
    pub(super) root_folder: Option<FolderEntry>,
    pub(in crate::native_app) loading_task: Option<u64>,
}

impl SourceEntry {
    pub(in crate::native_app) fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        root: PathBuf,
    ) -> Self {
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
pub(in crate::native_app) struct FileColumn {
    pub(in crate::native_app) id: String,
    pub(in crate::native_app) label: String,
    pub(in crate::native_app) width: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum FolderBrowserDrag {
    Folder { folder_id: String },
    Files { file_ids: Vec<String> },
    ExtractedFile { path: PathBuf },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct VisibleFolder {
    pub(in crate::native_app) id: String,
    pub(in crate::native_app) name: String,
    pub(in crate::native_app) depth: usize,
    pub(in crate::native_app) is_source_root: bool,
    pub(in crate::native_app) has_children: bool,
    pub(in crate::native_app) expanded: bool,
    pub(in crate::native_app) selected: bool,
    pub(in crate::native_app) drag_active: bool,
    pub(in crate::native_app) drag_source: bool,
    pub(in crate::native_app) drop_candidate: bool,
    pub(in crate::native_app) drop_target: bool,
    pub(in crate::native_app) drop_target_active: bool,
    pub(in crate::native_app) rename_draft: Option<String>,
    pub(in crate::native_app) rename_input_id: Option<u64>,
}

pub(super) fn default_file_columns() -> Vec<FileColumn> {
    vec![
        file_column("name", "Name", 240.0),
        file_column("rating", "Rating", 68.0),
        file_column("collection", "Col", 58.0),
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
