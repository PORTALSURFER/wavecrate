#![allow(missing_docs)]

use radiant::{gui::types::Point, prelude as ui, widgets::DragHandleMessage};
use std::{collections::HashSet, path::PathBuf};

use super::GuiMessage;

const TREE_ROW_HEIGHT: f32 = 23.0;
const TREE_DEPTH_INDENT: f32 = 4.0;
const MIN_FILE_COLUMN_WIDTH: f32 = 48.0;
const MAX_FILE_COLUMN_WIDTH: f32 = 420.0;

#[derive(Clone, Debug)]
pub(super) struct FolderBrowserState {
    selected_source: String,
    sources: Vec<SourceEntry>,
    selected_folder: String,
    selected_file: Option<String>,
    selected_file_ids: HashSet<String>,
    expanded_folders: HashSet<String>,
    folders: Vec<FolderEntry>,
    rename_edit: Option<FolderRenameEdit>,
    file_rename_edit: Option<FileRenameEdit>,
    drag: Option<FolderBrowserDrag>,
    drag_pointer: Option<Point>,
    drop_target_folder: Option<String>,
    file_columns: Vec<FileColumn>,
    file_sort: ui::DetailsSort,
    file_column_resize: Option<FileColumnResize>,
    file_view_start: usize,
}

impl FolderBrowserState {
    pub(super) fn load_default() -> Self {
        Self::from_root(default_root_path())
    }

    fn from_root(root: PathBuf) -> Self {
        let sources = vec![SourceEntry::new("assets", "Assets", root)];
        Self::from_sources(sources, String::from("assets"))
    }

    fn from_sources(sources: Vec<SourceEntry>, selected_source: String) -> Self {
        let mut sources = sources;
        let source_index = sources
            .iter()
            .position(|source| source.id == selected_source)
            .or(if sources.is_empty() { None } else { Some(0) })
            .expect("folder browser needs at least one source");
        let root_folder = load_root_folder(sources[source_index].root.clone());
        sources[source_index].root_folder = Some(root_folder.clone());
        let root_id = root_folder.id.clone();
        Self {
            selected_source: sources[source_index].id.clone(),
            sources,
            selected_folder: root_id.clone(),
            selected_file: None,
            selected_file_ids: HashSet::new(),
            expanded_folders: [root_id].into_iter().collect(),
            folders: vec![root_folder],
            rename_edit: None,
            file_rename_edit: None,
            drag: None,
            drag_pointer: None,
            drop_target_folder: None,
            file_columns: default_file_columns(),
            file_sort: ui::DetailsSort::new("name", ui::SortDirection::Ascending),
            file_column_resize: None,
            file_view_start: 0,
        }
    }

    #[cfg(test)]
    pub(super) fn root_path(&self) -> &std::path::Path {
        self.folders
            .first()
            .map(|folder| std::path::Path::new(&folder.id))
            .unwrap_or_else(|| std::path::Path::new(""))
    }

    #[cfg(test)]
    pub(super) fn source_labels(&self) -> Vec<String> {
        self.source_labels_for_tests()
    }

    pub(super) fn selected_files(&self) -> &[FileEntry] {
        self.selected_folder()
            .map(|folder| folder.files.as_slice())
            .unwrap_or(&[])
    }

    pub(super) fn selected_audio_files(&self) -> Vec<&FileEntry> {
        let mut files = self
            .selected_files()
            .iter()
            .filter(|file| file.is_audio())
            .collect::<Vec<_>>();
        self.sort_files(&mut files);
        files
    }

    pub(super) fn visible_file_columns(&self) -> Vec<&FileColumn> {
        self.file_columns.iter().collect()
    }

    pub(super) fn file_sort(&self) -> &ui::DetailsSort {
        &self.file_sort
    }

    pub(super) fn selected_file_id(&self) -> Option<&str> {
        self.selected_file.as_deref()
    }

    pub(super) fn folder_path(&self, folder_id: &str) -> Option<PathBuf> {
        self.find_folder(folder_id)
            .map(|folder| PathBuf::from(&folder.id))
    }

    pub(super) fn context_sample_path(&self, file_id: &str) -> Option<PathBuf> {
        if self.selected_file_ids.contains(file_id)
            && let Some(focused) = self.selected_file.as_deref()
            && self.selected_file_ids.contains(focused)
        {
            return Some(PathBuf::from(focused));
        }

        self.selected_audio_files()
            .into_iter()
            .find(|file| file.id == file_id)
            .map(|file| PathBuf::from(&file.id))
    }

    pub(super) fn is_file_selected(&self, file_id: &str) -> bool {
        if self.selected_file_ids.is_empty() {
            return self.selected_file.as_deref() == Some(file_id);
        }
        self.selected_file_ids.contains(file_id)
    }

    pub(super) fn scan_is_active(&self, source_id: &str, task_id: u64) -> bool {
        self.sources
            .iter()
            .any(|source| source.id == source_id && source.loading_task == Some(task_id))
    }

    pub(super) fn apply_message(&mut self, message: FolderBrowserMessage) {
        match message {
            FolderBrowserMessage::AddSource
            | FolderBrowserMessage::SelectSource(_)
            | FolderBrowserMessage::OpenSourceContextMenu(_, _)
            | FolderBrowserMessage::BeginRenameSelected
            | FolderBrowserMessage::BeginCreateSubfolder
            | FolderBrowserMessage::RenameInput(_)
            | FolderBrowserMessage::DropOnFolder(_) => {}
            FolderBrowserMessage::ClearDropTarget => {
                self.drop_target_folder = None;
            }
            FolderBrowserMessage::HoverDropTarget(id) => {
                self.hover_drop_target_folder(&id);
            }
            FolderBrowserMessage::ActivateFolder(id) => {
                self.cancel_rename();
                self.activate_folder(id);
            }
            FolderBrowserMessage::OpenFolderContextMenu(_, _) => {}
            FolderBrowserMessage::DragFolder(id, message) => {
                self.apply_folder_drag(id, message);
            }
            FolderBrowserMessage::SortFileColumn(column_id) => {
                self.sort_file_column(column_id);
            }
            FolderBrowserMessage::ResizeFileColumn(column_id, message) => {
                self.resize_file_column(column_id, message);
            }
        }
    }

    fn selected_folder(&self) -> Option<&FolderEntry> {
        self.find_folder(&self.selected_folder)
            .or_else(|| self.folders.first())
    }

    fn find_folder(&self, id: &str) -> Option<&FolderEntry> {
        self.folders.iter().find_map(|folder| folder.find(id))
    }

    fn folder_has_children(&self, id: &str) -> bool {
        self.find_folder(id).is_some_and(FolderEntry::has_children)
    }

    fn is_expanded(&self, id: &str) -> bool {
        self.expanded_folders.contains(id)
    }

    fn activate_folder(&mut self, id: String) {
        if !self.folder_has_children(&id) {
            self.select_folder(id);
            return;
        }
        if !self.is_expanded(&id) {
            self.expanded_folders.insert(id.clone());
            self.select_folder(id);
        } else if self.selected_folder == id {
            self.expanded_folders.remove(&id);
        } else {
            self.select_folder(id);
        }
    }

    fn select_folder(&mut self, id: String) {
        self.cancel_rename();
        self.selected_folder = id;
        self.selected_file = None;
        self.selected_file_ids.clear();
        self.file_view_start = 0;
    }

    fn sort_file_column(&mut self, column_id: String) {
        if self.file_sort.column_id == column_id {
            self.file_sort.direction = self.file_sort.direction.toggled();
        } else {
            self.file_sort = ui::DetailsSort::new(column_id, ui::SortDirection::Ascending);
        }
    }

    fn resize_file_column(&mut self, column_id: String, message: DragHandleMessage) {
        match message {
            DragHandleMessage::Started { position } => {
                if let Some(column) = self
                    .file_columns
                    .iter()
                    .find(|column| column.id == column_id)
                {
                    self.file_column_resize = Some(FileColumnResize {
                        column_id,
                        start_x: position.x,
                        start_width: column.width,
                    });
                }
            }
            DragHandleMessage::Moved { position } | DragHandleMessage::Ended { position } => {
                let Some(resize) = self.file_column_resize.clone() else {
                    return;
                };
                if let Some(column) = self
                    .file_columns
                    .iter_mut()
                    .find(|column| column.id == resize.column_id)
                {
                    column.width = (resize.start_width + position.x - resize.start_x)
                        .clamp(MIN_FILE_COLUMN_WIDTH, MAX_FILE_COLUMN_WIDTH);
                }
                if matches!(message, DragHandleMessage::Ended { .. }) {
                    self.file_column_resize = None;
                }
            }
        }
    }

    fn sort_files<'a>(&self, files: &mut Vec<&'a FileEntry>) {
        files.sort_by(|a, b| {
            let ordering = match self.file_sort.column_id.as_str() {
                "extension" => a
                    .extension
                    .to_ascii_lowercase()
                    .cmp(&b.extension.to_ascii_lowercase())
                    .then_with(|| {
                        a.name
                            .to_ascii_lowercase()
                            .cmp(&b.name.to_ascii_lowercase())
                    }),
                "size" => a.size_bytes.cmp(&b.size_bytes).then_with(|| {
                    a.name
                        .to_ascii_lowercase()
                        .cmp(&b.name.to_ascii_lowercase())
                }),
                "modified" => a.modified_rank.cmp(&b.modified_rank).then_with(|| {
                    a.name
                        .to_ascii_lowercase()
                        .cmp(&b.name.to_ascii_lowercase())
                }),
                "kind" => a.kind.cmp(&b.kind).then_with(|| {
                    a.name
                        .to_ascii_lowercase()
                        .cmp(&b.name.to_ascii_lowercase())
                }),
                "path" => a.id.cmp(&b.id),
                _ => a
                    .name
                    .to_ascii_lowercase()
                    .cmp(&b.name.to_ascii_lowercase()),
            };
            match self.file_sort.direction {
                ui::SortDirection::Ascending => ordering,
                ui::SortDirection::Descending => ordering.reverse(),
            }
        });
    }

    fn selected_folder_is_source_root(&self) -> bool {
        self.sources.iter().any(|source| {
            source.id == self.selected_source && path_id(&source.root) == self.selected_folder
        })
    }

    fn selected_folder_is_source_root_id(&self, folder_id: &str) -> bool {
        self.sources
            .iter()
            .any(|source| source.id == self.selected_source && path_id(&source.root) == folder_id)
    }

    fn visible_folders(&self) -> Vec<VisibleFolder> {
        let mut folders = Vec::new();
        for folder in &self.folders {
            self.push_visible_folder(folder, 0, &mut folders);
        }
        folders
    }

    fn push_visible_folder(
        &self,
        folder: &FolderEntry,
        depth: usize,
        folders: &mut Vec<VisibleFolder>,
    ) {
        let drag_active = self.drag.is_some();
        let drop_candidate = drag_active && self.can_drop_drag_on_folder(&folder.id);
        folders.push(VisibleFolder {
            id: folder.id.clone(),
            name: folder.name.clone(),
            depth,
            has_children: folder.has_children(),
            expanded: self.is_expanded(&folder.id),
            selected: self.selected_folder == folder.id,
            drag_active,
            drop_candidate,
            drop_target: drop_candidate
                && self.drop_target_folder.as_deref() == Some(folder.id.as_str()),
            rename_draft: self
                .rename_edit
                .as_ref()
                .filter(|edit| edit.folder_id == folder.id)
                .map(|edit| edit.draft.clone()),
            rename_input_id: self
                .rename_edit
                .as_ref()
                .filter(|edit| edit.folder_id == folder.id)
                .map(|edit| edit.input_id),
        });
        if self.is_expanded(&folder.id) {
            for child in &folder.children {
                self.push_visible_folder(child, depth + 1, folders);
            }
        }
    }
}

mod path_helpers;
use path_helpers::{folder_label, path_id, rewrite_path_id};

mod folder_entry;
use folder_entry::FolderEntry;

mod drag_drop;
mod drag_drop_relocation;

mod delete_workflow;

mod file_selection;

mod file_model;
pub(in crate::gui_app) use file_model::FileEntry;
use file_model::plural;

mod scanning;
pub(super) use scanning::scan_source_with_progress;
use scanning::{default_root_path, file_entry, load_root_folder};

mod source_management;

mod rename_workflow;

mod state_types;
pub(super) use state_types::FileColumn;
use state_types::{
    FileColumnResize, FileRenameEdit, FolderBrowserDrag, FolderRenameEdit, FolderRenameKind,
    SourceEntry, VisibleFolder, default_file_columns,
};

mod tree_widgets;

mod types;
pub(super) use types::{
    FileDeleteTargetView, FileRenameView, FolderBrowserMessage, FolderDeleteTargetView,
    FolderDragPreview, FolderDropResult, FolderScanDiscoveryBatch, FolderScanProgress,
    FolderScanRequest, FolderScanResult, RenameTargetView,
};

mod view;
pub(super) use view::folder_browser_view;

#[cfg(test)]
mod tests;
