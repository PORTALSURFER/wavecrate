use radiant::{gui::types::Point, prelude as ui};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    path::PathBuf,
};
use wavecrate::sample_sources::SampleCollection;

use super::{
    CollectionRenameEdit, DEFAULT_COLLECTIONS_PANEL_HEIGHT, DEFAULT_FILTER_PANEL_HEIGHT,
    FileColumn, FileEntry, FileMoveConflictBatch, FileRenameEdit, FolderBrowserDrag,
    FolderBrowserMessage, FolderEntry, FolderRenameEdit, SampleCollectionConfig, SourceEntry,
    default_file_columns, default_root_path, load_root_folder, placeholder_folder,
};

const DEFAULT_METADATA_PANEL_HEIGHT: f32 = 148.0;

#[derive(Clone, Debug)]
pub(in crate::gui_app) struct FolderBrowserState {
    pub(super) selected_source: String,
    pub(super) sources: Vec<SourceEntry>,
    pub(super) selected_folder: String,
    pub(super) selected_file: Option<String>,
    pub(super) selected_file_ids: HashSet<String>,
    pub(super) name_filter: String,
    pub(super) tag_filter: String,
    pub(super) expanded_folders: HashSet<String>,
    pub(super) folders: Vec<FolderEntry>,
    pub(super) rename_edit: Option<FolderRenameEdit>,
    pub(super) file_rename_edit: Option<FileRenameEdit>,
    pub(super) drag: Option<FolderBrowserDrag>,
    pub(super) drag_pointer: Option<Point>,
    pub(super) drop_target: ui::ExclusiveOpen<FolderBrowserDropTarget>,
    pub(super) pending_file_move_conflicts: Option<FileMoveConflictBatch>,
    pub(super) drag_revision: ui::RevisionCounter,
    pub(super) collections: Vec<SampleCollectionConfig>,
    pub(super) selected_collection: Option<SampleCollection>,
    pub(super) collection_rename_edit: Option<CollectionRenameEdit>,
    pub(super) collections_panel: ui::PanelResizeState,
    pub(super) filter_panel: ui::PanelResizeState,
    pub(super) metadata_panel: ui::PanelResizeState,
    pub(super) file_columns: Vec<FileColumn>,
    pub(super) file_sort: ui::DetailsSort,
    pub(super) file_column_resize: Option<ui::DetailsColumnResizeDrag>,
    pub(super) file_column_reorder: Option<ui::DetailsColumnReorderDrag>,
    pub(super) tree_view_controller: ui::VirtualListController,
    pub(super) file_view_controller: ui::VirtualListController,
    pub(super) tree_view_follow_selection: ui::VirtualListFollowState<String>,
    pub(super) file_view_follow_selection: ui::VirtualListFollowState<String>,
    file_content_revision: u64,
    selected_audio_projection_cache: RefCell<HashMap<SelectedAudioProjectionKey, Vec<usize>>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::gui_app) enum FolderBrowserDropTarget {
    Folder(String),
    Collection(SampleCollection),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SelectedAudioProjectionKey {
    folder_id: String,
    name_filter: String,
    sort_column_id: String,
    sort_descending: bool,
    content_revision: u64,
}

impl Hash for SelectedAudioProjectionKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.folder_id.hash(state);
        self.name_filter.hash(state);
        self.sort_column_id.hash(state);
        self.sort_descending.hash(state);
        self.content_revision.hash(state);
    }
}

impl FolderBrowserState {
    pub(in crate::gui_app) fn load_default() -> Self {
        Self::from_root(default_root_path())
    }

    pub(in crate::gui_app) fn from_root(root: PathBuf) -> Self {
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

    pub(super) fn from_sources_deferred(
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
        let state = Self {
            selected_source: sources[source_index].id.clone(),
            sources,
            selected_folder: root_id.clone(),
            selected_file: None,
            selected_file_ids: HashSet::new(),
            name_filter: String::new(),
            tag_filter: String::new(),
            expanded_folders: [root_id].into_iter().collect(),
            folders: vec![root_folder],
            rename_edit: None,
            file_rename_edit: None,
            drag: None,
            drag_pointer: None,
            drop_target: ui::ExclusiveOpen::new(),
            pending_file_move_conflicts: None,
            drag_revision: ui::RevisionCounter::default(),
            collections: Self::default_collections(),
            selected_collection: None,
            collection_rename_edit: None,
            collections_panel: ui::PanelResizeState::new(DEFAULT_COLLECTIONS_PANEL_HEIGHT),
            filter_panel: ui::PanelResizeState::new(DEFAULT_FILTER_PANEL_HEIGHT),
            metadata_panel: ui::PanelResizeState::new(DEFAULT_METADATA_PANEL_HEIGHT),
            file_columns: default_file_columns(),
            file_sort: ui::DetailsSort::new("name", ui::SortDirection::Ascending),
            file_column_resize: None,
            file_column_reorder: None,
            tree_view_controller: ui::VirtualListController::default(),
            file_view_controller: ui::VirtualListController::default(),
            tree_view_follow_selection: ui::VirtualListFollowState::default(),
            file_view_follow_selection: ui::VirtualListFollowState::default(),
            file_content_revision: 0,
            selected_audio_projection_cache: RefCell::new(HashMap::new()),
        };
        state.prewarm_selected_source_audio_projection_cache();
        state
    }

    #[cfg(test)]
    pub(in crate::gui_app) fn root_path(&self) -> &std::path::Path {
        self.folders
            .first()
            .map(|folder| std::path::Path::new(&folder.id))
            .unwrap_or_else(|| std::path::Path::new(""))
    }

    #[cfg(test)]
    pub(in crate::gui_app) fn source_labels(&self) -> Vec<String> {
        self.source_labels_for_tests()
    }

    pub(in crate::gui_app) fn selected_files(&self) -> &[FileEntry] {
        self.selected_folder()
            .map(|folder| folder.files.as_slice())
            .unwrap_or(&[])
    }

    pub(in crate::gui_app) fn selected_audio_files(&self) -> Vec<&FileEntry> {
        if let Some(collection) = self.selected_collection {
            let mut files = Vec::new();
            if let Some(folder) = self.selected_source_root_folder() {
                collect_collection_audio_files(folder, collection, &mut files);
            }
            filter_audio_files_by_name(&mut files, &self.name_filter);
            self.sort_files(&mut files);
            return files;
        }

        let Some(folder) = self.selected_folder() else {
            return Vec::new();
        };
        self.selected_folder_audio_file_indices(folder)
            .into_iter()
            .filter_map(|index| folder.files.get(index))
            .collect()
    }

    pub(in crate::gui_app) fn selected_audio_files_matching_tags(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Vec<&FileEntry> {
        let mut files = self.selected_audio_files();
        filter_audio_files_by_tags(&mut files, tags_by_file, &self.tag_filter);
        files
    }

    pub(in crate::gui_app) fn selected_audio_file_count_matching_tags(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> usize {
        let name_query = normalized_name_filter(&self.name_filter);
        let required_tags = parsed_tag_filter(&self.tag_filter);
        if required_tags.is_empty() && self.selected_collection.is_none() {
            return self.selected_folder_audio_file_count();
        }
        if let Some(collection) = self.selected_collection {
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

    pub(in crate::gui_app) fn selected_folder_audio_file_count(&self) -> usize {
        if self.selected_collection.is_some() {
            return self.selected_audio_files().len();
        }
        let Some(folder) = self.selected_folder() else {
            return 0;
        };
        self.selected_folder_audio_file_indices(folder).len()
    }

    pub(in crate::gui_app) fn selected_audio_file_at_matching_tags(
        &self,
        index: usize,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Option<&FileEntry> {
        let required_tags = parsed_tag_filter(&self.tag_filter);
        if self.selected_collection.is_some() {
            return self
                .selected_audio_files_matching_tags(tags_by_file)
                .get(index)
                .copied();
        }
        let folder = self.selected_folder()?;
        if required_tags.is_empty() {
            return self
                .selected_folder_audio_file_indices(folder)
                .get(index)
                .and_then(|file_index| folder.files.get(*file_index));
        }
        self.selected_folder_audio_file_indices(folder)
            .into_iter()
            .filter_map(|file_index| folder.files.get(file_index))
            .filter(|file| audio_file_matches_parsed_tags(file, tags_by_file, &required_tags))
            .nth(index)
    }

    pub(in crate::gui_app) fn selected_audio_file_index_matching_tags(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Option<usize> {
        let selected = self.selected_file.as_deref()?;
        let required_tags = parsed_tag_filter(&self.tag_filter);
        if self.selected_collection.is_some() {
            return self
                .selected_audio_files_matching_tags(tags_by_file)
                .iter()
                .position(|file| file.id == selected);
        }
        let folder = self.selected_folder()?;
        self.selected_folder_audio_file_indices(folder)
            .into_iter()
            .filter_map(|file_index| folder.files.get(file_index))
            .filter(|file| audio_file_matches_parsed_tags(file, tags_by_file, &required_tags))
            .position(|file| file.id == selected)
    }

    pub(in crate::gui_app) fn selected_source_audio_files(&self) -> Vec<&FileEntry> {
        let mut files = Vec::new();
        if let Some(folder) = self.selected_source_root_folder() {
            collect_audio_files(folder, &mut files);
        }
        self.sort_files(&mut files);
        files
    }

    pub(super) fn selected_source_root_folder(&self) -> Option<&FolderEntry> {
        self.folders.first().or_else(|| {
            self.sources
                .iter()
                .find(|source| source.id == self.selected_source)
                .and_then(|source| source.root_folder.as_ref())
        })
    }

    pub(in crate::gui_app) fn selected_file_id(&self) -> Option<&str> {
        self.selected_file.as_deref()
    }

    pub(in crate::gui_app) fn folder_path(&self, folder_id: &str) -> Option<PathBuf> {
        self.find_folder(folder_id)
            .map(|folder| PathBuf::from(&folder.id))
    }

    pub(in crate::gui_app) fn context_sample_path(&self, file_id: &str) -> Option<PathBuf> {
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

    pub(in crate::gui_app) fn is_file_selected(&self, file_id: &str) -> bool {
        if self.selected_file_ids.is_empty() {
            return self.selected_file.as_deref() == Some(file_id);
        }
        self.selected_file_ids.contains(file_id)
    }

    pub(in crate::gui_app) fn drag_revision(&self) -> u64 {
        self.drag_revision.get()
    }

    pub(in crate::gui_app) fn scan_is_active(&self, source_id: &str, task_id: u64) -> bool {
        self.sources
            .iter()
            .any(|source| source.id == source_id && source.loading_task == Some(task_id))
    }

    pub(in crate::gui_app) fn apply_message(&mut self, message: FolderBrowserMessage) {
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
            }
            FolderBrowserMessage::ToggleFolderExpansion(id) => {
                self.cancel_rename();
                self.toggle_folder_expansion(id);
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
        self.file_content_revision = self.file_content_revision.saturating_add(1);
        self.selected_audio_projection_cache.get_mut().clear();
    }

    fn selected_folder_audio_file_indices(&self, folder: &FolderEntry) -> Vec<usize> {
        let key = SelectedAudioProjectionKey {
            folder_id: folder.id.clone(),
            name_filter: normalized_name_filter(&self.name_filter),
            sort_column_id: self.file_sort.column_id.clone(),
            sort_descending: self.file_sort.direction == ui::SortDirection::Descending,
            content_revision: self.file_content_revision,
        };
        if let Some(indices) = self.selected_audio_projection_cache.borrow().get(&key) {
            return indices.clone();
        }

        let mut indices = folder
            .files
            .iter()
            .enumerate()
            .filter(|(_, file)| {
                file.is_audio() && audio_file_matches_name_query(file, &key.name_filter)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        self.sort_file_indices(folder, &mut indices);
        self.selected_audio_projection_cache
            .borrow_mut()
            .insert(key, indices.clone());
        indices
    }

    fn sort_file_indices(&self, folder: &FolderEntry, indices: &mut [usize]) {
        match self.file_sort.column_id.as_str() {
            "extension" => indices.sort_by_cached_key(|index| {
                let file = &folder.files[*index];
                (file.extension.to_ascii_lowercase(), file.name_sort_key())
            }),
            "size" => indices.sort_by_cached_key(|index| {
                let file = &folder.files[*index];
                (file.size_bytes, file.name_sort_key())
            }),
            "modified" => indices.sort_by_cached_key(|index| {
                let file = &folder.files[*index];
                (file.modified_rank, file.name_sort_key())
            }),
            "kind" => indices.sort_by_cached_key(|index| {
                let file = &folder.files[*index];
                (file.kind.clone(), file.name_sort_key())
            }),
            "rating" => indices.sort_by_cached_key(|index| {
                let file = &folder.files[*index];
                (file.rating.val(), file.name_sort_key())
            }),
            "collection" => indices.sort_by_cached_key(|index| {
                let file = &folder.files[*index];
                (
                    file.first_collection().map(|collection| collection.index()),
                    file.name_sort_key(),
                )
            }),
            "path" => indices.sort_by(|a, b| folder.files[*a].id.cmp(&folder.files[*b].id)),
            _ => indices.sort_by_cached_key(|index| folder.files[*index].name_sort_key()),
        }
        if self.file_sort.direction == ui::SortDirection::Descending {
            indices.reverse();
        }
    }

    pub(super) fn prewarm_selected_source_audio_projection_cache(&self) {
        if let Some(root) = self.folders.first() {
            self.prewarm_folder_audio_projection_cache(root);
        }
    }

    fn prewarm_folder_audio_projection_cache(&self, folder: &FolderEntry) {
        let _ = self.selected_folder_audio_file_indices(folder);
        for child in &folder.children {
            self.prewarm_folder_audio_projection_cache(child);
        }
    }

    #[cfg(test)]
    pub(in crate::gui_app) fn selected_audio_projection_cache_len_for_tests(&self) -> usize {
        self.selected_audio_projection_cache.borrow().len()
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
