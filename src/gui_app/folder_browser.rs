#![allow(missing_docs)]

use radiant::{gui::types::Point, prelude as ui};
use std::{collections::HashSet, path::PathBuf};
use wavecrate::sample_sources::SampleCollection;

use super::GuiMessage;

pub(super) const TREE_ROW_HEIGHT: f32 = 23.0;
const TREE_DEPTH_INDENT: f32 = 4.0;

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
    drop_target_collection: Option<SampleCollection>,
    drag_revision: ui::RevisionCounter,
    collections: Vec<SampleCollectionConfig>,
    selected_collection: Option<SampleCollection>,
    collection_rename_edit: Option<CollectionRenameEdit>,
    collections_panel_height: f32,
    collection_panel_resize: Option<ui::PanelResizeDrag>,
    file_columns: Vec<FileColumn>,
    file_sort: ui::DetailsSort,
    file_column_resize: Option<ui::DetailsColumnResizeDrag>,
    file_column_reorder: Option<ui::DetailsColumnReorderDrag>,
    tree_view_controller: ui::VirtualListController,
    file_view_controller: ui::VirtualListController,
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
            drop_target_collection: None,
            drag_revision: ui::RevisionCounter::default(),
            collections: Self::default_collections(),
            selected_collection: None,
            collection_rename_edit: None,
            collections_panel_height: DEFAULT_COLLECTIONS_PANEL_HEIGHT,
            collection_panel_resize: None,
            file_columns: default_file_columns(),
            file_sort: ui::DetailsSort::new("name", ui::SortDirection::Ascending),
            file_column_resize: None,
            file_column_reorder: None,
            tree_view_controller: ui::VirtualListController::default(),
            file_view_controller: ui::VirtualListController::default(),
        }
    }

    fn from_sources_deferred(sources: Vec<SourceEntry>, selected_source: String) -> Self {
        let source_index = sources
            .iter()
            .position(|source| source.id == selected_source)
            .or(if sources.is_empty() { None } else { Some(0) })
            .expect("folder browser needs at least one source");
        let root_folder = placeholder_folder(&sources[source_index].root);
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
            drop_target_collection: None,
            drag_revision: ui::RevisionCounter::default(),
            collections: Self::default_collections(),
            selected_collection: None,
            collection_rename_edit: None,
            collections_panel_height: DEFAULT_COLLECTIONS_PANEL_HEIGHT,
            collection_panel_resize: None,
            file_columns: default_file_columns(),
            file_sort: ui::DetailsSort::new("name", ui::SortDirection::Ascending),
            file_column_resize: None,
            file_column_reorder: None,
            tree_view_controller: ui::VirtualListController::default(),
            file_view_controller: ui::VirtualListController::default(),
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
        let mut files = if let Some(collection) = self.selected_collection {
            let mut files = Vec::new();
            if let Some(folder) = self.selected_source_root_folder() {
                collect_collection_audio_files(folder, collection, &mut files);
            }
            files
        } else {
            self.selected_files()
                .iter()
                .filter(|file| file.is_audio())
                .collect::<Vec<_>>()
        };
        self.sort_files(&mut files);
        files
    }

    pub(super) fn selected_source_audio_files(&self) -> Vec<&FileEntry> {
        let mut files = Vec::new();
        if let Some(folder) = self.selected_source_root_folder() {
            collect_audio_files(folder, &mut files);
        }
        self.sort_files(&mut files);
        files
    }

    fn selected_source_root_folder(&self) -> Option<&FolderEntry> {
        self.folders.first().or_else(|| {
            self.sources
                .iter()
                .find(|source| source.id == self.selected_source)
                .and_then(|source| source.root_folder.as_ref())
        })
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

    pub(super) fn drag_revision(&self) -> u64 {
        self.drag_revision.get()
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
            | FolderBrowserMessage::DropOnFolder(_)
            | FolderBrowserMessage::DropOnCollection(_) => {}
            FolderBrowserMessage::ClearDropTarget(position) => {
                self.update_drag_pointer(position);
                self.drop_target_folder = None;
            }
            FolderBrowserMessage::HoverDropTarget(id, position) => {
                self.update_drag_pointer(position);
                self.hover_drop_target_folder(&id);
            }
            FolderBrowserMessage::ActivateFolder(id) => {
                self.cancel_rename();
                self.activate_folder(id);
            }
            FolderBrowserMessage::OpenFolderContextMenu(_, _) => {}
            FolderBrowserMessage::CancelRename => {
                self.cancel_rename();
            }
            FolderBrowserMessage::DragFolder(id, message) => {
                self.apply_folder_drag(id, message);
            }
            FolderBrowserMessage::SortFileColumn(column_id) => {
                self.sort_file_column(column_id);
            }
            FolderBrowserMessage::ResizeFileColumn(column_id, message) => {
                self.resize_file_column(column_id, message);
            }
            FolderBrowserMessage::DragFileColumn(column_id, message) => {
                self.drag_file_column(column_id, message);
            }
            FolderBrowserMessage::ResizeCollectionsPanel(message) => {
                self.resize_collections_panel(message);
            }
            FolderBrowserMessage::ActivateCollection(collection) => {
                self.activate_collection(collection);
            }
            FolderBrowserMessage::RenameCollection(collection) => {
                self.begin_rename_collection(collection);
            }
            FolderBrowserMessage::HoverCollectionDropTarget(collection, position) => {
                self.hover_drop_target_collection(collection, position);
            }
        }
    }
}

fn collect_audio_files<'a>(folder: &'a FolderEntry, files: &mut Vec<&'a FileEntry>) {
    files.extend(folder.files.iter().filter(|file| file.is_audio()));
    for child in &folder.children {
        collect_audio_files(child, files);
    }
}

fn collect_collection_audio_files<'a>(
    folder: &'a FolderEntry,
    collection: SampleCollection,
    files: &mut Vec<&'a FileEntry>,
) {
    files.extend(
        folder
            .files
            .iter()
            .filter(|file| file.is_audio() && file.collection == Some(collection)),
    );
    for child in &folder.children {
        collect_collection_audio_files(child, collection, files);
    }
}

mod collections;
use collections::{CollectionRenameEdit, DEFAULT_COLLECTIONS_PANEL_HEIGHT, SampleCollectionConfig};
pub(in crate::gui_app) use collections::{SampleCollectionView, collection_hotkey};

mod path_helpers;
use path_helpers::{folder_label, path_id, rewrite_path_id};

mod folder_entry;
use folder_entry::FolderEntry;

mod drag_drop;
mod drag_drop_move;
mod drag_drop_relocation;

mod delete_workflow;

mod file_selection;

mod file_view_window;

mod file_rename_workflow;

mod file_columns;
#[cfg(test)]
pub(super) use file_columns::MIN_FILE_COLUMN_WIDTH;

mod file_model;
pub(in crate::gui_app) use file_model::FileEntry;
use file_model::plural;

mod scanning;
pub(super) use scanning::scan_source_with_progress;
use scanning::{default_root_path, file_entry, load_root_folder, placeholder_folder};

mod source_management;

mod rename_tree;
mod rename_workflow;

mod state_types;
pub(super) use state_types::FileColumn;
use state_types::{
    FileRenameEdit, FolderBrowserDrag, FolderRenameEdit, FolderRenameKind, SourceEntry,
    VisibleFolder, default_file_columns,
};

mod tree_state;

mod tree_hit_target;

mod tree_view_window;

mod types;
pub(super) use types::{
    FileDeleteTargetView, FileRenameView, FolderBrowserMessage, FolderDeleteTargetView,
    FolderDragPreview, FolderDropResult, FolderScanDiscovery, FolderScanDiscoveryBatch,
    FolderScanProgress, FolderScanRequest, FolderScanResult, RemovedSource, RenameTargetView,
};

mod tag_completion;
mod tag_editor;
mod tag_entry_layout;
mod view;
#[cfg(test)]
pub(super) use view::folder_browser_view;
pub(super) use view::folder_browser_view_mut;

#[cfg(test)]
mod tests;
