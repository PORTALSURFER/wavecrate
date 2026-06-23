use radiant::prelude as ui;
use std::{
    cell::{Ref, RefCell},
    collections::{BTreeMap, HashMap, HashSet},
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};

use super::{
    FileColumn, FileEntry, FileRenameView, FolderBrowserState, SimilarityAspectStrengths,
    SimilarityBrowserState, default_file_columns,
};
use wavecrate::sample_sources::config::SimilarityAspectSettings;

const COPY_FLASH_FRAMES: u8 = 12;

#[derive(Clone, Copy)]
pub(in crate::native_app) struct VisibleSampleQuery<'a> {
    pub(in crate::native_app) tags_by_file: &'a HashMap<String, Vec<String>>,
    pub(in crate::native_app) cached_sample_paths: &'a HashSet<String>,
}

#[derive(Clone, Copy)]
pub(in crate::native_app) struct VisibleSampleWindowPolicy<'a> {
    pub(in crate::native_app) tags_by_file: &'a HashMap<String, Vec<String>>,
    pub(in crate::native_app) viewport_rows: usize,
    pub(in crate::native_app) overscan_rows: usize,
    pub(in crate::native_app) guard_rows: usize,
}

pub(in crate::native_app) struct VisibleSampleList<'a> {
    pub(in crate::native_app) total_count: usize,
    pub(in crate::native_app) includes_subfolders: bool,
    pub(in crate::native_app) window: ui::VirtualListWindow,
    pub(in crate::native_app) rows: Vec<VisibleSampleRow<'a>>,
    pub(in crate::native_app) columns: Vec<&'a FileColumn>,
    pub(in crate::native_app) sort: &'a ui::DetailsSort,
    pub(in crate::native_app) similarity_mode_active: bool,
    pub(in crate::native_app) similarity_controls: &'a SimilarityAspectSettings,
}

pub(super) struct VisibleSampleWindowFiles<'a> {
    pub(super) total_count: usize,
    pub(super) rows: Vec<&'a FileEntry>,
}

pub(in crate::native_app) struct VisibleSampleRow<'a> {
    pub(in crate::native_app) file: &'a FileEntry,
    pub(in crate::native_app) selected: bool,
    pub(in crate::native_app) copy_flash: bool,
    pub(in crate::native_app) drag_active: bool,
    pub(in crate::native_app) drag_source: bool,
    pub(in crate::native_app) cached: bool,
    pub(in crate::native_app) missing: bool,
    pub(in crate::native_app) rename: Option<FileRenameView>,
    pub(in crate::native_app) similarity_anchor: bool,
    pub(in crate::native_app) similarity_strength: Option<f32>,
    pub(in crate::native_app) similarity_aspect_strengths: SimilarityAspectStrengths,
    pub(in crate::native_app) collection_colors: Vec<ui::Rgba8>,
    pub(in crate::native_app) source_folder_path: String,
}

#[derive(Clone, Debug)]
pub(super) struct SampleListState {
    pub(super) file_columns: Vec<FileColumn>,
    pub(super) file_sort: ui::DetailsSort,
    pub(super) file_column_resize: Option<ui::DetailsColumnResizeDrag>,
    pub(super) file_column_reorder: Option<ui::DetailsColumnReorderDrag>,
    pub(super) similarity_controls: SimilarityAspectSettings,
    pub(super) similarity: Option<SimilarityBrowserState>,
    pub(super) random_navigation: RandomNavigationState,
    pub(super) include_subfolders: bool,
    pub(super) view_controller: ui::VirtualListController,
    pub(super) follow_selection: ui::VirtualListFollowState<String>,
    pub(super) prepared_window: ui::VirtualListWindow,
    pub(super) content_revision: u64,
    pub(super) missing_collection_files: Vec<FileEntry>,
    pub(super) missing_collection_counts: BTreeMap<u8, usize>,
    pub(super) projection_cache: VisibleSampleProjectionCache,
    copy_flash_file_ids: HashSet<String>,
    copy_flash_frames: u8,
}

impl SampleListState {
    pub(super) fn new() -> Self {
        Self {
            file_columns: default_file_columns(),
            file_sort: ui::DetailsSort::new("name", ui::SortDirection::Ascending),
            file_column_resize: None,
            file_column_reorder: None,
            similarity_controls: SimilarityAspectSettings::default(),
            similarity: None,
            random_navigation: RandomNavigationState::default(),
            include_subfolders: false,
            view_controller: ui::VirtualListController::default(),
            follow_selection: ui::VirtualListFollowState::default(),
            prepared_window: ui::VirtualListWindow::default(),
            content_revision: 0,
            missing_collection_files: Vec::new(),
            missing_collection_counts: BTreeMap::new(),
            projection_cache: VisibleSampleProjectionCache::default(),
            copy_flash_file_ids: HashSet::new(),
            copy_flash_frames: 0,
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

#[derive(Clone, Debug, Default)]
pub(super) struct RandomNavigationState {
    pub(super) enabled: bool,
    result_ids: Vec<String>,
    visited: HashSet<String>,
    history: Vec<String>,
}

impl RandomNavigationState {
    pub(super) fn set_enabled(
        &mut self,
        enabled: bool,
        selected_file: Option<&str>,
        ids: &[String],
    ) {
        if self.enabled == enabled {
            return;
        }
        self.enabled = enabled;
        self.reset_for_selection(selected_file, ids);
    }

    pub(super) fn reconcile(&mut self, selected_file: Option<&str>, ids: &[String]) {
        if self.result_ids == ids {
            return;
        }
        self.reset_for_selection(selected_file, ids);
    }

    pub(super) fn previous(&mut self) -> Option<String> {
        if self.history.len() <= 1 {
            return None;
        }
        self.history.pop();
        self.history.last().cloned()
    }

    pub(super) fn next(&mut self, selected_file: Option<&str>, ids: &[String]) -> Option<String> {
        self.reconcile(selected_file, ids);
        if ids.len() <= 1 {
            return None;
        }
        if let Some(selected) = selected_file.filter(|selected| ids.iter().any(|id| id == selected))
        {
            self.record_selected(selected);
        }

        let target = self.random_unvisited(ids).or_else(|| {
            self.reset_cycle(selected_file, ids);
            self.random_unvisited(ids)
        })?;
        self.record_selected(&target);
        Some(target)
    }

    fn reset_for_selection(&mut self, selected_file: Option<&str>, ids: &[String]) {
        self.result_ids = ids.to_vec();
        self.visited.clear();
        self.history.clear();
        if let Some(selected) = selected_file.filter(|selected| ids.iter().any(|id| id == selected))
        {
            self.record_selected(selected);
        }
    }

    fn reset_cycle(&mut self, selected_file: Option<&str>, ids: &[String]) {
        self.visited.clear();
        self.history.clear();
        if let Some(selected) = selected_file.filter(|selected| ids.iter().any(|id| id == selected))
        {
            self.record_selected(selected);
        }
    }

    fn random_unvisited(&self, ids: &[String]) -> Option<String> {
        use rand::Rng;

        let candidates = ids
            .iter()
            .filter(|id| !self.visited.contains(*id))
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return None;
        }
        let index = rand::rng().random_range(0..candidates.len());
        Some(candidates[index].to_string())
    }

    fn record_selected(&mut self, selected: &str) {
        self.visited.insert(selected.to_owned());
        if self.history.last().is_none_or(|last| last != selected) {
            self.history.push(selected.to_owned());
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct VisibleSampleProjectionCache {
    entries: RefCell<HashMap<VisibleSampleProjectionKey, Vec<usize>>>,
    id_entries: RefCell<HashMap<VisibleSampleProjectionKey, Vec<String>>>,
}

impl VisibleSampleProjectionCache {
    pub(super) fn audio_indices(
        &self,
        request: VisibleSampleProjectionRequest<'_>,
        build: impl FnOnce() -> Vec<usize>,
    ) -> Ref<'_, Vec<usize>> {
        let key = request.key();
        if !self.entries.borrow().contains_key(&key) {
            self.entries.borrow_mut().insert(key.clone(), build());
        }
        Ref::map(self.entries.borrow(), |entries| {
            entries
                .get(&key)
                .expect("visible sample projection cache should contain computed key")
        })
    }

    pub(super) fn audio_ids(
        &self,
        request: VisibleSampleProjectionRequest<'_>,
        build: impl FnOnce() -> Vec<String>,
    ) -> Ref<'_, Vec<String>> {
        let key = request.key();
        if !self.id_entries.borrow().contains_key(&key) {
            self.id_entries.borrow_mut().insert(key.clone(), build());
        }
        Ref::map(self.id_entries.borrow(), |entries| {
            entries
                .get(&key)
                .expect("visible sample id projection cache should contain computed key")
        })
    }

    pub(super) fn invalidate_for_content_revision(&mut self, content_revision: u64) {
        self.entries
            .get_mut()
            .retain(|key, _| key.content_revision == content_revision);
        self.id_entries
            .get_mut()
            .retain(|key, _| key.content_revision == content_revision);
    }

    pub(super) fn clear(&self) {
        self.entries.borrow_mut().clear();
        self.id_entries.borrow_mut().clear();
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.entries.borrow().len() + self.id_entries.borrow().len()
    }
}

pub(super) struct VisibleSampleProjectionRequest<'a> {
    folder_id: &'a str,
    name_filter: &'a str,
    rating_filter: &'a str,
    sort: &'a ui::DetailsSort,
    similarity_anchor_id: Option<&'a str>,
    content_revision: u64,
}

impl<'a> VisibleSampleProjectionRequest<'a> {
    pub(super) fn new(
        folder_id: &'a str,
        name_filter: &'a str,
        rating_filter: &'a str,
        sort: &'a ui::DetailsSort,
        similarity_anchor_id: Option<&'a str>,
        content_revision: u64,
    ) -> Self {
        Self {
            folder_id,
            name_filter,
            rating_filter,
            sort,
            similarity_anchor_id,
            content_revision,
        }
    }

    fn key(&self) -> VisibleSampleProjectionKey {
        VisibleSampleProjectionKey::new(
            self.folder_id.to_owned(),
            self.name_filter.to_owned(),
            self.rating_filter.to_owned(),
            self.sort.column_id.clone(),
            self.sort.direction == ui::SortDirection::Descending,
            self.similarity_anchor_id.map(str::to_owned),
            self.content_revision,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VisibleSampleProjectionKey {
    folder_id: String,
    name_filter: String,
    rating_filter: String,
    sort_column_id: String,
    sort_descending: bool,
    similarity_anchor_id: Option<String>,
    content_revision: u64,
}

impl VisibleSampleProjectionKey {
    fn new(
        folder_id: String,
        name_filter: String,
        rating_filter: String,
        sort_column_id: String,
        sort_descending: bool,
        similarity_anchor_id: Option<String>,
        content_revision: u64,
    ) -> Self {
        Self {
            folder_id,
            name_filter,
            rating_filter,
            sort_column_id,
            sort_descending,
            similarity_anchor_id,
            content_revision,
        }
    }
}

impl Hash for VisibleSampleProjectionKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.folder_id.hash(state);
        self.name_filter.hash(state);
        self.rating_filter.hash(state);
        self.sort_column_id.hash(state);
        self.sort_descending.hash(state);
        self.similarity_anchor_id.hash(state);
        self.content_revision.hash(state);
    }
}

impl FolderBrowserState {
    pub(in crate::native_app) fn flash_copied_file_paths<I, P>(&mut self, paths: I)
    where
        I: IntoIterator<Item = P>,
        P: AsRef<std::path::Path>,
    {
        self.sample_list.copy_flash_file_ids.clear();
        self.sample_list
            .copy_flash_file_ids
            .extend(paths.into_iter().filter_map(copy_flash_file_id));
        self.sample_list.copy_flash_frames = if self.sample_list.copy_flash_file_ids.is_empty() {
            0
        } else {
            COPY_FLASH_FRAMES
        };
    }

    pub(in crate::native_app) fn copy_flash_active(&self) -> bool {
        self.sample_list.copy_flash_frames > 0
    }

    pub(in crate::native_app) fn advance_copy_flash_frame(&mut self) {
        if self.sample_list.copy_flash_frames == 0 {
            return;
        }
        self.sample_list.copy_flash_frames = self.sample_list.copy_flash_frames.saturating_sub(1);
        if self.sample_list.copy_flash_frames == 0 {
            self.sample_list.copy_flash_file_ids.clear();
        }
    }

    pub(in crate::native_app) fn prepare_visible_sample_window(
        &mut self,
        policy: VisibleSampleWindowPolicy<'_>,
    ) -> ui::VirtualListWindow {
        self.follow_selected_file_view_matching_tags(
            policy.viewport_rows,
            policy.overscan_rows,
            policy.guard_rows,
            policy.tags_by_file,
        )
    }

    pub(in crate::native_app) fn visible_samples<'a>(
        &'a self,
        query: VisibleSampleQuery<'a>,
    ) -> VisibleSampleList<'a> {
        let prepared_window = self.sample_list.prepared_window;
        let mut window_files =
            self.selected_audio_file_window_matching_tags(prepared_window, query.tags_by_file);
        let mut window = reconcile_visible_sample_window(prepared_window, window_files.total_count);
        if window != prepared_window {
            window_files =
                self.selected_audio_file_window_matching_tags(window, query.tags_by_file);
        }
        if !window_files_complete(window, &window_files) {
            self.sample_list.projection_cache.clear();
            window_files =
                self.selected_audio_file_window_matching_tags(window, query.tags_by_file);
        }
        if !window_files_complete(window, &window_files) {
            window_files =
                self.uncached_selected_audio_file_window_matching_tags(window, query.tags_by_file);
            let repaired_window = reconcile_visible_sample_window(window, window_files.total_count);
            if repaired_window != window {
                window = repaired_window;
                window_files = self
                    .uncached_selected_audio_file_window_matching_tags(window, query.tags_by_file);
            }
        }
        let rows = window_files
            .rows
            .into_iter()
            .map(|file| self.visible_sample_row_for_file(file, query))
            .collect();

        VisibleSampleList {
            total_count: window_files.total_count,
            includes_subfolders: self.folder_subtree_listing_enabled(),
            window,
            rows,
            columns: self.visible_file_columns(),
            sort: self.file_sort(),
            similarity_mode_active: self.similarity_mode_active(),
            similarity_controls: self.similarity_controls(),
        }
    }

    fn visible_sample_row_for_file<'a>(
        &'a self,
        file: &'a FileEntry,
        query: VisibleSampleQuery<'a>,
    ) -> VisibleSampleRow<'a> {
        VisibleSampleRow {
            file,
            selected: self.is_file_selected(&file.id),
            copy_flash: self.copied_file_flash_active(&file.id),
            drag_active: self.file_drag_active(),
            drag_source: self.file_drag_source(&file.id),
            cached: query.cached_sample_paths.contains(&file.id),
            missing: file.is_missing(),
            rename: self.file_rename_view(&file.id),
            similarity_anchor: self.file_is_similarity_anchor(&file.id),
            similarity_strength: self.similarity_display_strength_for_file(&file.id),
            similarity_aspect_strengths: self
                .similarity_aspect_display_strengths_for_file(&file.id),
            collection_colors: file
                .collection_memberships()
                .into_iter()
                .filter_map(|collection| self.collection_color(collection))
                .collect(),
            source_folder_path: self.visible_source_folder_path_for_file(file),
        }
    }

    fn visible_source_folder_path_for_file(&self, file: &FileEntry) -> String {
        if self.collection_focus_active() {
            self.source_folder_path_for_file(file)
        } else {
            String::new()
        }
    }

    fn source_folder_path_for_file(&self, file: &FileEntry) -> String {
        let file_path = Path::new(&file.id);
        let folder_path = self
            .source_relative_file_path(file_path)
            .and_then(|(_, relative)| relative.parent().map(Path::to_path_buf))
            .or_else(|| file_path.parent().map(Path::to_path_buf))
            .unwrap_or_default();
        folder_path_display(folder_path)
    }

    fn copied_file_flash_active(&self, file_id: &str) -> bool {
        self.copy_flash_active() && self.sample_list.copy_flash_file_ids.contains(file_id)
    }
}

fn folder_path_display(path: PathBuf) -> String {
    if path.as_os_str().is_empty() {
        String::from(".")
    } else {
        path.to_string_lossy().into_owned()
    }
}

fn copy_flash_file_id(path: impl AsRef<std::path::Path>) -> Option<String> {
    path.as_ref()
        .to_str()
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
}

fn window_files_complete(
    window: ui::VirtualListWindow,
    window_files: &VisibleSampleWindowFiles<'_>,
) -> bool {
    window_files.rows.len() == window.window_len()
}

fn reconcile_visible_sample_window(
    window: ui::VirtualListWindow,
    total_count: usize,
) -> ui::VirtualListWindow {
    if window_is_valid_for_total(window, total_count) {
        return window;
    }

    let viewport_len = window.viewport_len().max(1);
    let overscan = window
        .viewport_start
        .saturating_sub(window.window_start)
        .max(window.window_end.saturating_sub(window.viewport_end));

    ui::resolve_virtual_list_window(ui::VirtualListWindowRequest {
        total_items: total_count,
        viewport_len,
        requested_start: window.viewport_start,
        overscan,
        focused_index: None,
        previous_start: None,
        guard_band: 0,
    })
}

fn window_is_valid_for_total(window: ui::VirtualListWindow, total_count: usize) -> bool {
    window.total_items == total_count
        && window.viewport_start <= window.viewport_end
        && window.window_start <= window.window_end
        && window.window_start <= window.viewport_start
        && window.viewport_end <= window.window_end
        && window.window_end <= total_count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_request_key_tracks_query_and_revision_inputs() {
        let ascending = ui::DetailsSort::new("name", ui::SortDirection::Ascending);
        let descending = ui::DetailsSort::new("size", ui::SortDirection::Descending);

        assert_ne!(
            VisibleSampleProjectionRequest::new("folder", "kick", "", &ascending, None, 4).key(),
            VisibleSampleProjectionRequest::new("folder", "kick", "", &descending, None, 4).key()
        );
        assert_ne!(
            VisibleSampleProjectionRequest::new("folder", "kick", "", &ascending, None, 4).key(),
            VisibleSampleProjectionRequest::new("folder", "snare", "", &ascending, None, 4).key()
        );
        assert_ne!(
            VisibleSampleProjectionRequest::new("folder", "kick", "", &ascending, Some("a.wav"), 4)
                .key(),
            VisibleSampleProjectionRequest::new("folder", "kick", "", &ascending, Some("b.wav"), 4)
                .key()
        );
        assert_ne!(
            VisibleSampleProjectionRequest::new("folder", "kick", "", &ascending, None, 4).key(),
            VisibleSampleProjectionRequest::new("folder", "kick", "", &ascending, None, 5).key()
        );
        assert_ne!(
            VisibleSampleProjectionRequest::new("folder", "kick", "-1,2", &ascending, None, 4)
                .key(),
            VisibleSampleProjectionRequest::new("folder", "kick", "2", &ascending, None, 4).key()
        );
    }

    #[test]
    fn stale_prepared_window_is_clamped_to_current_total() {
        let stale = ui::VirtualListWindow {
            total_items: 10_000,
            viewport_start: 9_990,
            viewport_end: 10_000,
            window_start: 9_986,
            window_end: 10_000,
        };

        let reconciled = reconcile_visible_sample_window(stale, 24);

        assert_eq!(reconciled.total_items, 24);
        assert_eq!(reconciled.viewport_start, 14);
        assert_eq!(reconciled.viewport_end, 24);
        assert_eq!(reconciled.window_start, 10);
        assert_eq!(reconciled.window_end, 24);
    }
}
