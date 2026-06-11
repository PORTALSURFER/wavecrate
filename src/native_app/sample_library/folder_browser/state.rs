use radiant::{gui::types::Point, prelude as ui};
use std::{
    cell::Ref,
    collections::{HashMap, HashSet},
    path::PathBuf,
};
use wavecrate::sample_sources::SampleCollection;

use super::{
    CollectionRenameEdit, DEFAULT_COLLECTIONS_PANEL_HEIGHT, DEFAULT_FILTER_PANEL_HEIGHT,
    FileColumn, FileEntry, FileMoveConflictBatch, FileRenameEdit, FolderBrowserDrag,
    FolderBrowserMessage, FolderEntry, FolderRenameEdit, SampleCollectionConfig,
    SimilarityBrowserState, SourceEntry,
    collections::default_collections,
    default_file_columns, default_root_path,
    file_columns::{sort_file_indices_by_column_kind, sort_kind_for_details_sort},
    load_root_folder, placeholder_folder,
    visible_samples::{VisibleSampleProjectionCache, VisibleSampleProjectionRequest},
};

const DEFAULT_METADATA_PANEL_HEIGHT: f32 = 148.0;

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

#[derive(Clone, Debug)]
pub(super) struct BrowserSourceState {
    pub(super) selected_source: String,
    pub(super) sources: Vec<SourceEntry>,
}

impl BrowserSourceState {
    fn new(sources: Vec<SourceEntry>, selected_source: String) -> Self {
        Self {
            selected_source,
            sources,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct BrowserSelectionState {
    pub(super) selected_folder: String,
    pub(super) selected_file: Option<String>,
    pub(super) selected_file_ids: HashSet<String>,
    pub(super) selected_file_ids_explicit: bool,
    pub(super) selected_collection: Option<SampleCollection>,
    pub(super) folder_before_collection: Option<String>,
}

impl BrowserSelectionState {
    fn new(selected_folder: String) -> Self {
        Self {
            selected_folder,
            selected_file: None,
            selected_file_ids: HashSet::new(),
            selected_file_ids_explicit: false,
            selected_collection: None,
            folder_before_collection: None,
        }
    }

    pub(super) fn clear_file_selection(&mut self) {
        self.selected_file = None;
        self.selected_file_ids.clear();
        self.selected_file_ids_explicit = false;
    }

    pub(super) fn select_folder(&mut self, folder_id: String) {
        self.selected_collection = None;
        self.folder_before_collection = None;
        self.selected_folder = folder_id;
        self.clear_file_selection();
    }

    pub(super) fn enter_collection(&mut self, collection: SampleCollection) {
        if self.selected_collection.is_none() {
            self.folder_before_collection = Some(self.selected_folder.clone());
        }
        self.selected_collection = Some(collection);
        self.clear_file_selection();
    }

    pub(super) fn exit_collection(&mut self, restored_folder: Option<String>) -> bool {
        if self.selected_collection.take().is_none() {
            self.folder_before_collection = None;
            return false;
        }
        if let Some(folder) = restored_folder {
            self.selected_folder = folder;
        }
        self.folder_before_collection = None;
        self.clear_file_selection();
        true
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct BrowserFilterState {
    pub(super) name_filter: String,
    pub(super) tag_filter: String,
}

#[derive(Clone, Debug)]
pub(super) struct FolderTreeState {
    pub(super) folders: Vec<FolderEntry>,
    pub(super) expanded_folders: HashSet<String>,
    pub(super) view_controller: ui::VirtualListController,
    pub(super) follow_selection: ui::VirtualListFollowState<String>,
}

impl FolderTreeState {
    fn new(root_folder: FolderEntry, root_id: String) -> Self {
        Self {
            folders: vec![root_folder],
            expanded_folders: [root_id].into_iter().collect(),
            view_controller: ui::VirtualListController::default(),
            follow_selection: ui::VirtualListFollowState::default(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct BrowserRenameState {
    pub(super) folder: Option<FolderRenameEdit>,
    pub(super) file: Option<FileRenameEdit>,
}

#[derive(Clone, Debug)]
pub(super) struct BrowserDragDropState {
    pub(super) drag: Option<FolderBrowserDrag>,
    pub(super) drag_pointer: Option<Point>,
    pub(super) drop_target: ui::ExclusiveOpen<FolderBrowserDropTarget>,
    pub(super) pending_file_move_conflicts: Option<FileMoveConflictBatch>,
    pub(super) revision: ui::RevisionCounter,
}

impl BrowserDragDropState {
    fn new() -> Self {
        Self {
            drag: None,
            drag_pointer: None,
            drop_target: ui::ExclusiveOpen::new(),
            pending_file_move_conflicts: None,
            revision: ui::RevisionCounter::default(),
        }
    }

    #[cfg(test)]
    pub(super) fn revision(&self) -> u64 {
        self.revision.get()
    }
}

#[derive(Clone, Debug)]
pub(super) struct CollectionPanelState {
    pub(super) collections: Vec<SampleCollectionConfig>,
    pub(super) rename_edit: Option<CollectionRenameEdit>,
}

impl CollectionPanelState {
    fn new() -> Self {
        Self {
            collections: default_collections(),
            rename_edit: None,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct BrowserPanelLayoutState {
    pub(super) collections: ui::PanelResizeState,
    pub(super) filter: ui::PanelResizeState,
    pub(super) metadata: ui::PanelResizeState,
}

impl BrowserPanelLayoutState {
    fn new() -> Self {
        Self {
            collections: ui::PanelResizeState::new(DEFAULT_COLLECTIONS_PANEL_HEIGHT),
            filter: ui::PanelResizeState::new(DEFAULT_FILTER_PANEL_HEIGHT),
            metadata: ui::PanelResizeState::new(DEFAULT_METADATA_PANEL_HEIGHT),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct SampleListState {
    pub(super) file_columns: Vec<FileColumn>,
    pub(super) file_sort: ui::DetailsSort,
    pub(super) file_column_resize: Option<ui::DetailsColumnResizeDrag>,
    pub(super) file_column_reorder: Option<ui::DetailsColumnReorderDrag>,
    pub(super) similarity: Option<SimilarityBrowserState>,
    pub(super) view_controller: ui::VirtualListController,
    pub(super) follow_selection: ui::VirtualListFollowState<String>,
    pub(super) prepared_window: ui::VirtualListWindow,
    pub(super) content_revision: u64,
    projection_cache: VisibleSampleProjectionCache,
}

impl SampleListState {
    fn new() -> Self {
        Self {
            file_columns: default_file_columns(),
            file_sort: ui::DetailsSort::new("name", ui::SortDirection::Ascending),
            file_column_resize: None,
            file_column_reorder: None,
            similarity: None,
            view_controller: ui::VirtualListController::default(),
            follow_selection: ui::VirtualListFollowState::default(),
            prepared_window: ui::VirtualListWindow::default(),
            content_revision: 0,
            projection_cache: VisibleSampleProjectionCache::default(),
        }
    }

    pub(super) fn reset_view(&mut self) {
        self.view_controller = ui::VirtualListController::default();
        self.follow_selection.clear();
        self.prepared_window = ui::VirtualListWindow::default();
    }

    pub(super) fn bump_content_revision(&mut self) {
        self.content_revision = self.content_revision.saturating_add(1);
        self.projection_cache
            .invalidate_for_content_revision(self.content_revision);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum FolderBrowserDropTarget {
    Folder(String),
    Collection(SampleCollection),
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

    pub(in crate::native_app) fn selected_files(&self) -> &[FileEntry] {
        self.selected_folder()
            .map(|folder| folder.files.as_slice())
            .unwrap_or(&[])
    }

    pub(in crate::native_app) fn selected_audio_files(&self) -> Vec<&FileEntry> {
        if let Some(collection) = self.selection.selected_collection {
            let mut files = Vec::new();
            if let Some(folder) = self.selected_source_root_folder() {
                collect_collection_audio_files(folder, collection, &mut files);
            }
            filter_audio_files_by_name(&mut files, &self.filters.name_filter);
            self.sort_files(&mut files);
            return files;
        }

        let Some(folder) = self.selected_folder() else {
            return Vec::new();
        };
        self.selected_folder_audio_files(folder)
    }

    pub(in crate::native_app) fn selected_folder_cache_warm_request(
        &self,
    ) -> Option<(String, Vec<PathBuf>)> {
        let folder = self.selected_folder()?;
        let paths = folder
            .files
            .iter()
            .filter(|file| file.is_audio())
            .map(|file| PathBuf::from(&file.id))
            .collect::<Vec<_>>();
        Some((folder.id.clone(), paths))
    }

    pub(in crate::native_app) fn selected_audio_files_matching_tags(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Vec<&FileEntry> {
        let mut files = self.selected_audio_files();
        filter_audio_files_by_tags(&mut files, tags_by_file, &self.filters.tag_filter);
        files
    }

    pub(in crate::native_app) fn selected_audio_file_count_matching_tags(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> usize {
        let name_query = normalized_name_filter(&self.filters.name_filter);
        let required_tags = parsed_tag_filter(&self.filters.tag_filter);
        if required_tags.is_empty() && self.selection.selected_collection.is_none() {
            return self.selected_folder_audio_file_count();
        }
        if let Some(collection) = self.selection.selected_collection {
            return self
                .selected_source_root_folder()
                .map(|folder| {
                    count_matching_audio_files_in_folder(
                        folder,
                        &name_query,
                        &required_tags,
                        tags_by_file,
                        Some(collection),
                    )
                })
                .unwrap_or_default();
        }

        self.selected_files()
            .iter()
            .filter(|file| {
                file.is_audio()
                    && audio_file_matches_name_query(file, &name_query)
                    && audio_file_matches_parsed_tags(file, tags_by_file, &required_tags)
            })
            .count()
    }

    pub(in crate::native_app) fn selected_folder_audio_file_count(&self) -> usize {
        if self.selection.selected_collection.is_some() {
            return self.selected_audio_files().len();
        }
        let Some(folder) = self.selected_folder() else {
            return 0;
        };
        self.selected_folder_audio_file_indices_ref(folder).len()
    }

    pub(in crate::native_app) fn selected_audio_file_at_matching_tags(
        &self,
        index: usize,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Option<&FileEntry> {
        let required_tags = parsed_tag_filter(&self.filters.tag_filter);
        if self.selection.selected_collection.is_some() {
            return self
                .selected_audio_files_matching_tags(tags_by_file)
                .get(index)
                .copied();
        }
        let folder = self.selected_folder()?;
        if required_tags.is_empty() {
            return self
                .selected_folder_audio_file_indices_ref(folder)
                .get(index)
                .and_then(|file_index| folder.files.get(*file_index));
        }
        self.selected_folder_audio_file_indices_ref(folder)
            .iter()
            .filter_map(|file_index| folder.files.get(*file_index))
            .filter(|file| audio_file_matches_parsed_tags(file, tags_by_file, &required_tags))
            .nth(index)
    }

    pub(in crate::native_app) fn selected_audio_file_index_matching_tags(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Option<usize> {
        let selected = self.selection.selected_file.as_deref()?;
        let required_tags = parsed_tag_filter(&self.filters.tag_filter);
        if self.selection.selected_collection.is_some() {
            return self
                .selected_audio_files_matching_tags(tags_by_file)
                .iter()
                .position(|file| file.id == selected);
        }
        let folder = self.selected_folder()?;
        self.selected_folder_audio_file_indices_ref(folder)
            .iter()
            .filter_map(|file_index| folder.files.get(*file_index))
            .filter(|file| audio_file_matches_parsed_tags(file, tags_by_file, &required_tags))
            .position(|file| file.id == selected)
    }

    pub(in crate::native_app) fn selected_source_audio_files(&self) -> Vec<&FileEntry> {
        let mut files = Vec::new();
        if let Some(folder) = self.selected_source_root_folder() {
            collect_audio_files(folder, &mut files);
        }
        self.sort_files(&mut files);
        files
    }

    pub(super) fn selected_source_root_folder(&self) -> Option<&FolderEntry> {
        self.tree.folders.first().or_else(|| {
            self.source
                .sources
                .iter()
                .find(|source| source.id == self.source.selected_source)
                .and_then(|source| source.root_folder.as_ref())
        })
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

    #[cfg(test)]
    pub(in crate::native_app) fn set_similarity_scores_for_tests(
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

    fn selected_folder_audio_files<'a>(&self, folder: &'a FolderEntry) -> Vec<&'a FileEntry> {
        self.selected_folder_audio_file_indices_ref(folder)
            .iter()
            .filter_map(|index| folder.files.get(*index))
            .collect()
    }

    fn selected_folder_audio_file_indices_ref(&self, folder: &FolderEntry) -> Ref<'_, Vec<usize>> {
        let name_filter = normalized_name_filter(&self.filters.name_filter);
        let request = VisibleSampleProjectionRequest::new(
            folder.id.as_str(),
            name_filter.as_str(),
            &self.sample_list.file_sort,
            self.similarity_anchor_id(),
            self.sample_list.content_revision,
        );
        self.sample_list
            .projection_cache
            .audio_indices(request, || {
                let mut indices = folder
                    .files
                    .iter()
                    .enumerate()
                    .filter(|(_, file)| {
                        file.is_audio() && audio_file_matches_name_query(file, &name_filter)
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                self.sort_file_indices(folder, &mut indices);
                self.sort_file_indices_by_similarity(folder, &mut indices);
                indices
            })
    }

    fn sort_file_indices(&self, folder: &FolderEntry, indices: &mut [usize]) {
        sort_file_indices_by_column_kind(
            sort_kind_for_details_sort(&self.sample_list.file_sort),
            folder,
            indices,
        );
        if self.sample_list.file_sort.direction == ui::SortDirection::Descending {
            indices.reverse();
        }
    }

    fn sort_file_indices_by_similarity(&self, folder: &FolderEntry, indices: &mut [usize]) {
        let Some(similarity) = self.sample_list.similarity.as_ref() else {
            return;
        };
        let base_order = indices
            .iter()
            .enumerate()
            .map(|(order, index)| (*index, order))
            .collect::<HashMap<_, _>>();
        indices.sort_by(|left, right| {
            similarity_file_order(folder, similarity, &base_order, *left, *right)
        });
    }

    pub(super) fn prewarm_selected_source_audio_projection_cache(&self) {
        if let Some(root) = self.tree.folders.first() {
            self.prewarm_folder_audio_projection_cache(root);
        }
    }

    fn prewarm_folder_audio_projection_cache(&self, folder: &FolderEntry) {
        let _ = self.selected_folder_audio_file_indices_ref(folder);
        for child in &folder.children {
            self.prewarm_folder_audio_projection_cache(child);
        }
    }

    #[cfg(test)]
    pub(in crate::native_app) fn selected_audio_projection_cache_len_for_tests(&self) -> usize {
        self.sample_list.projection_cache.len()
    }
}

fn selected_source_index(sources: &[SourceEntry], selected_source: &str) -> usize {
    sources
        .iter()
        .position(|source| source.id == selected_source)
        .or(if sources.is_empty() { None } else { Some(0) })
        .expect("folder browser needs at least one source")
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
            .filter(|file| file.is_audio() && file.belongs_to_collection(collection)),
    );
    for child in &folder.children {
        collect_collection_audio_files(child, collection, files);
    }
}

fn similarity_file_order(
    folder: &FolderEntry,
    similarity: &SimilarityBrowserState,
    base_order: &HashMap<usize, usize>,
    left: usize,
    right: usize,
) -> std::cmp::Ordering {
    let left_file = &folder.files[left];
    let right_file = &folder.files[right];
    match (
        left_file.id == similarity.anchor_id(),
        right_file.id == similarity.anchor_id(),
    ) {
        (true, false) => return std::cmp::Ordering::Less,
        (false, true) => return std::cmp::Ordering::Greater,
        _ => {}
    }

    match (
        similarity.raw_score_for(&left_file.id),
        similarity.raw_score_for(&right_file.id),
    ) {
        (Some(left_score), Some(right_score)) => right_score
            .total_cmp(&left_score)
            .then_with(|| base_order_for(left, base_order).cmp(&base_order_for(right, base_order))),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => base_order_for(left, base_order).cmp(&base_order_for(right, base_order)),
    }
}

fn base_order_for(index: usize, base_order: &HashMap<usize, usize>) -> usize {
    base_order.get(&index).copied().unwrap_or(usize::MAX)
}

fn count_matching_audio_files_in_folder(
    folder: &FolderEntry,
    name_query: &str,
    required_tags: &[String],
    tags_by_file: &HashMap<String, Vec<String>>,
    collection: Option<SampleCollection>,
) -> usize {
    let local_count = folder
        .files
        .iter()
        .filter(|file| {
            file.is_audio()
                && collection.is_none_or(|collection| file.belongs_to_collection(collection))
                && audio_file_matches_name_query(file, name_query)
                && audio_file_matches_parsed_tags(file, tags_by_file, required_tags)
        })
        .count();
    local_count
        + folder
            .children
            .iter()
            .map(|child| {
                count_matching_audio_files_in_folder(
                    child,
                    name_query,
                    required_tags,
                    tags_by_file,
                    collection,
                )
            })
            .sum::<usize>()
}

fn filter_audio_files_by_name(files: &mut Vec<&FileEntry>, name_filter: &str) {
    let query = normalized_name_filter(name_filter);
    files.retain(|file| audio_file_matches_name_query(file, &query));
}

fn filter_audio_files_by_tags(
    files: &mut Vec<&FileEntry>,
    tags_by_file: &HashMap<String, Vec<String>>,
    tag_filter: &str,
) {
    let required_tags = parsed_tag_filter(tag_filter);
    files.retain(|file| audio_file_matches_parsed_tags(file, tags_by_file, &required_tags));
}

fn audio_file_matches_name_query(file: &FileEntry, query: &str) -> bool {
    query.is_empty()
        || file.name.to_ascii_lowercase().contains(query)
        || file.stem.to_ascii_lowercase().contains(query)
}

fn audio_file_matches_parsed_tags(
    file: &FileEntry,
    tags_by_file: &HashMap<String, Vec<String>>,
    required_tags: &[String],
) -> bool {
    if required_tags.is_empty() {
        return true;
    }
    let Some(file_tags) = tags_by_file.get(&file.id) else {
        return false;
    };
    required_tags.iter().all(|required| {
        file_tags
            .iter()
            .any(|tag| tag.trim().eq_ignore_ascii_case(required))
    })
}

fn parsed_tag_filter(tag_filter: &str) -> Vec<String> {
    tag_filter
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(|tag| tag.to_ascii_lowercase())
        .collect()
}

fn normalized_name_filter(name_filter: &str) -> String {
    name_filter.trim().to_ascii_lowercase()
}
