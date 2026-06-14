use std::collections::HashMap;
use std::path::PathBuf;

use super::{
    BrowserDragDropState, BrowserFilterState, BrowserPanelLayoutState, BrowserRenameState,
    BrowserSelectionState, BrowserSourceState, CollectionPanelState, FolderBrowserMessage,
    FolderEntry, FolderTreeState, SampleListState, SimilarityBrowserState, SourceEntry,
    default_root_path, load_root_folder, placeholder_folder,
};

#[derive(Clone, Debug)]
pub(in crate::native_app) struct FolderBrowserState {
    pub(super) source: BrowserSourceState,
    pub(super) selection: BrowserSelectionState,
    pub(super) filters: BrowserFilterState,
    pub(super) tree: FolderTreeState,
    pub(super) rename: BrowserRenameState,
    pub(super) drag_drop: BrowserDragDropState,
    pub(super) collection_panel: CollectionPanelState,
    pub(super) panel_layout: BrowserPanelLayoutState,
    pub(super) sample_list: SampleListState,
}

impl FolderBrowserState {
    pub(in crate::native_app) fn load_default() -> Self {
        Self::from_root(default_root_path())
    }

    pub(in crate::native_app) fn from_root(root: PathBuf) -> Self {
        let sources = vec![SourceEntry::new("assets", "Assets", root)];
        Self::from_sources(sources, String::from("assets"))
    }

    pub(super) fn from_sources(sources: Vec<SourceEntry>, selected_source: String) -> Self {
        let mut sources = sources;
        let source_index = selected_source_index(&sources, &selected_source);
        let root_folder = load_root_folder(sources[source_index].root.clone());
        sources[source_index].root_folder = Some(root_folder.clone());
        Self::new(sources, source_index, root_folder)
    }

    pub(in crate::native_app) fn from_sources_deferred(
        sources: Vec<SourceEntry>,
        selected_source: String,
    ) -> Self {
        let source_index = selected_source_index(&sources, &selected_source);
        let root_folder = sources[source_index]
            .root_folder
            .clone()
            .unwrap_or_else(|| placeholder_folder(&sources[source_index].root));
        Self::new(sources, source_index, root_folder)
    }

    fn new(sources: Vec<SourceEntry>, source_index: usize, root_folder: FolderEntry) -> Self {
        let root_id = root_folder.id.clone();
        let selected_source = sources[source_index].id.clone();
        let state = Self {
            source: BrowserSourceState::new(sources, selected_source),
            selection: BrowserSelectionState::new(root_id.clone()),
            filters: BrowserFilterState::default(),
            tree: FolderTreeState::new(root_folder, root_id),
            rename: BrowserRenameState::default(),
            drag_drop: BrowserDragDropState::new(),
            collection_panel: CollectionPanelState::new(),
            panel_layout: BrowserPanelLayoutState::new(),
            sample_list: SampleListState::new(),
        };
        state.prewarm_selected_source_audio_projection_cache();
        state
    }

    #[cfg(test)]
    pub(in crate::native_app) fn root_path(&self) -> &std::path::Path {
        self.tree
            .folders
            .first()
            .map(|folder| std::path::Path::new(&folder.id))
            .unwrap_or_else(|| std::path::Path::new(""))
    }

    #[cfg(test)]
    pub(in crate::native_app) fn source_labels(&self) -> Vec<String> {
        self.source_labels_for_tests()
    }

    pub(in crate::native_app) fn selected_file_id(&self) -> Option<&str> {
        self.selection.selected_file_id()
    }

    pub(in crate::native_app) fn similarity_anchor_id(&self) -> Option<&str> {
        self.sample_list
            .similarity
            .as_ref()
            .map(SimilarityBrowserState::anchor_id)
    }

    pub(in crate::native_app) fn similarity_mode_active(&self) -> bool {
        self.sample_list.similarity.is_some()
    }

    pub(in crate::native_app) fn file_is_similarity_anchor(&self, file_id: &str) -> bool {
        self.similarity_anchor_id() == Some(file_id)
    }

    pub(in crate::native_app) fn similarity_display_strength_for_file(
        &self,
        file_id: &str,
    ) -> Option<f32> {
        self.sample_list
            .similarity
            .as_ref()
            .and_then(|similarity| similarity.display_strength_for(file_id))
    }

    pub(in crate::native_app) fn toggle_similarity_anchor(&mut self, file_id: String) {
        if self.file_is_similarity_anchor(&file_id) {
            self.sample_list.similarity = None;
        } else {
            self.sample_list.similarity = Some(SimilarityBrowserState::new(file_id));
        }
        self.bump_file_content_revision();
    }

    pub(in crate::native_app) fn set_similarity_scores(
        &mut self,
        anchor_id: String,
        scores_by_file: HashMap<String, f32>,
    ) {
        self.sample_list.similarity = Some(SimilarityBrowserState::with_scores(
            anchor_id,
            scores_by_file,
        ));
        self.bump_file_content_revision();
    }

    #[cfg(test)]
    pub(in crate::native_app) fn set_similarity_scores_for_tests(
        &mut self,
        anchor_id: String,
        scores_by_file: HashMap<String, f32>,
    ) {
        self.set_similarity_scores(anchor_id, scores_by_file);
    }

    pub(in crate::native_app) fn folder_path(&self, folder_id: &str) -> Option<PathBuf> {
        self.find_folder(folder_id)
            .map(|folder| PathBuf::from(&folder.id))
    }

    pub(in crate::native_app) fn context_sample_path(&self, file_id: &str) -> Option<PathBuf> {
        if self.selection.selected_file_ids.contains(file_id)
            && let Some(focused) = self.selection.selected_file.as_deref()
            && self.selection.selected_file_ids.contains(focused)
        {
            return Some(PathBuf::from(focused));
        }

        self.selected_audio_files()
            .into_iter()
            .find(|file| file.id == file_id)
            .map(|file| PathBuf::from(&file.id))
    }

    pub(in crate::native_app) fn is_file_selected(&self, file_id: &str) -> bool {
        self.selection.selected_file_ids_contains(file_id)
    }

    pub(in crate::native_app) fn drag_revision(&self) -> u64 {
        self.drag_drop.revision.get()
    }

    pub(in crate::native_app) fn scan_is_active(&self, source_id: &str, task_id: u64) -> bool {
        self.source
            .sources
            .iter()
            .any(|source| source.id == source_id && source.loading_task == Some(task_id))
    }

    pub(in crate::native_app) fn apply_message(&mut self, message: FolderBrowserMessage) {
        match message {
            FolderBrowserMessage::AddSource
            | FolderBrowserMessage::SelectSource(_)
            | FolderBrowserMessage::OpenSourceContextMenu(_, _)
            | FolderBrowserMessage::BeginRenameSelected
            | FolderBrowserMessage::BeginCreateSubfolder
            | FolderBrowserMessage::RenameInput(_)
            | FolderBrowserMessage::DropOnFolder(_)
            | FolderBrowserMessage::DropOnCollection(_) => {}
            FolderBrowserMessage::NameFilterInput(message) => {
                self.apply_name_filter_input(message);
            }
            FolderBrowserMessage::TagFilterInput(message) => {
                self.apply_tag_filter_input(message);
            }
            FolderBrowserMessage::ClearDropTarget(position) => {
                self.clear_drop_target_folder(position);
            }
            FolderBrowserMessage::ClearDropTargetUnless(id, position) => {
                self.clear_drop_target_folder_unless(&id, position);
            }
            FolderBrowserMessage::HoverDropTarget(id, position) => {
                self.update_drag_pointer(position);
                self.hover_drop_target_folder(&id);
            }
            FolderBrowserMessage::ActivateFolder(id) => {
                self.cancel_rename();
                self.activate_folder(id);
                self.sync_tree_view_to_selection(
                    super::FOLDER_TREE_PROJECTED_VIEWPORT_ROWS,
                    super::FOLDER_TREE_OVERSCAN_ROWS,
                    super::FOLDER_TREE_EDGE_CONTEXT_ROWS,
                );
            }
            FolderBrowserMessage::ToggleFolderExpansion(id) => {
                self.cancel_rename();
                self.toggle_folder_expansion(id);
                self.sync_tree_view_to_selection(
                    super::FOLDER_TREE_PROJECTED_VIEWPORT_ROWS,
                    super::FOLDER_TREE_OVERSCAN_ROWS,
                    super::FOLDER_TREE_EDGE_CONTEXT_ROWS,
                );
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
            FolderBrowserMessage::CancelFileColumnDrag => {
                self.cancel_file_column_drag();
            }
            FolderBrowserMessage::ExitCollectionFocus => {
                self.exit_collection_focus();
            }
            FolderBrowserMessage::ToggleSimilarityAnchor(file_id) => {
                self.toggle_similarity_anchor(file_id);
            }
            FolderBrowserMessage::ResizeCollectionsPanel(message) => {
                self.resize_collections_panel(message);
            }
            FolderBrowserMessage::ResizeFilterPanel(message) => {
                self.resize_filter_panel(message);
            }
            FolderBrowserMessage::ResizeMetadataPanel(message) => {
                self.resize_metadata_panel(message);
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

    pub(super) fn bump_file_content_revision(&mut self) {
        self.sample_list.bump_content_revision();
    }
}

fn selected_source_index(sources: &[SourceEntry], selected_source: &str) -> usize {
    sources
        .iter()
        .position(|source| source.id == selected_source)
        .or(if sources.is_empty() { None } else { Some(0) })
        .expect("folder browser needs at least one source")
}
