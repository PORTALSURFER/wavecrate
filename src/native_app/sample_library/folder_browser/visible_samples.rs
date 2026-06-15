use radiant::prelude as ui;
use std::{
    cell::{Ref, RefCell},
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
};

use super::{
    FileColumn, FileEntry, FileRenameView, FolderBrowserState, SimilarityBrowserState,
    default_file_columns,
};

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
    pub(in crate::native_app) window: ui::VirtualListWindow,
    pub(in crate::native_app) rows: Vec<Option<VisibleSampleRow<'a>>>,
    pub(in crate::native_app) columns: Vec<&'a FileColumn>,
    pub(in crate::native_app) sort: &'a ui::DetailsSort,
    pub(in crate::native_app) similarity_mode_active: bool,
}

pub(in crate::native_app) struct VisibleSampleRow<'a> {
    pub(in crate::native_app) file: &'a FileEntry,
    pub(in crate::native_app) selected: bool,
    pub(in crate::native_app) drag_revision: u64,
    pub(in crate::native_app) drag_active: bool,
    pub(in crate::native_app) drag_source: bool,
    pub(in crate::native_app) cached: bool,
    pub(in crate::native_app) rename: Option<FileRenameView>,
    pub(in crate::native_app) similarity_anchor: bool,
    pub(in crate::native_app) similarity_strength: Option<f32>,
    pub(in crate::native_app) collection_colors: Vec<ui::Rgba8>,
}

#[derive(Clone, Debug)]
pub(super) struct SampleListState {
    pub(super) file_columns: Vec<FileColumn>,
    pub(super) file_sort: ui::DetailsSort,
    pub(super) file_column_resize: Option<ui::DetailsColumnResizeDrag>,
    pub(super) file_column_reorder: Option<ui::DetailsColumnReorderDrag>,
    pub(super) similarity: Option<SimilarityBrowserState>,
    pub(super) random_navigation: RandomNavigationState,
    pub(super) view_controller: ui::VirtualListController,
    pub(super) follow_selection: ui::VirtualListFollowState<String>,
    pub(super) prepared_window: ui::VirtualListWindow,
    pub(super) runtime_viewport_rows: Option<usize>,
    pub(super) content_revision: u64,
    pub(super) projection_cache: VisibleSampleProjectionCache,
}

impl SampleListState {
    pub(super) fn new() -> Self {
        Self {
            file_columns: default_file_columns(),
            file_sort: ui::DetailsSort::new("name", ui::SortDirection::Ascending),
            file_column_resize: None,
            file_column_reorder: None,
            similarity: None,
            random_navigation: RandomNavigationState::default(),
            view_controller: ui::VirtualListController::default(),
            follow_selection: ui::VirtualListFollowState::default(),
            prepared_window: ui::VirtualListWindow::default(),
            runtime_viewport_rows: None,
            content_revision: 0,
            projection_cache: VisibleSampleProjectionCache::default(),
        }
    }

    pub(super) fn reset_view(&mut self) {
        self.view_controller = ui::VirtualListController::default();
        self.follow_selection.clear();
        self.prepared_window = ui::VirtualListWindow::default();
        self.runtime_viewport_rows = None;
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

    pub(super) fn invalidate_for_content_revision(&mut self, content_revision: u64) {
        self.entries
            .get_mut()
            .retain(|key, _| key.content_revision == content_revision);
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.entries.borrow().len()
    }
}

pub(super) struct VisibleSampleProjectionRequest<'a> {
    folder_id: &'a str,
    name_filter: &'a str,
    sort: &'a ui::DetailsSort,
    similarity_anchor_id: Option<&'a str>,
    content_revision: u64,
}

impl<'a> VisibleSampleProjectionRequest<'a> {
    pub(super) fn new(
        folder_id: &'a str,
        name_filter: &'a str,
        sort: &'a ui::DetailsSort,
        similarity_anchor_id: Option<&'a str>,
        content_revision: u64,
    ) -> Self {
        Self {
            folder_id,
            name_filter,
            sort,
            similarity_anchor_id,
            content_revision,
        }
    }

    fn key(&self) -> VisibleSampleProjectionKey {
        VisibleSampleProjectionKey::new(
            self.folder_id.to_owned(),
            self.name_filter.to_owned(),
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
    sort_column_id: String,
    sort_descending: bool,
    similarity_anchor_id: Option<String>,
    content_revision: u64,
}

impl VisibleSampleProjectionKey {
    fn new(
        folder_id: String,
        name_filter: String,
        sort_column_id: String,
        sort_descending: bool,
        similarity_anchor_id: Option<String>,
        content_revision: u64,
    ) -> Self {
        Self {
            folder_id,
            name_filter,
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
        self.sort_column_id.hash(state);
        self.sort_descending.hash(state);
        self.similarity_anchor_id.hash(state);
        self.content_revision.hash(state);
    }
}

impl FolderBrowserState {
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
        let window = self.sample_list.prepared_window;
        let total_count = window.total_items;
        let rows = (window.window_start..window.window_end)
            .map(|index| self.visible_sample_row(index, query))
            .collect();

        VisibleSampleList {
            total_count,
            window,
            rows,
            columns: self.visible_file_columns(),
            sort: self.file_sort(),
            similarity_mode_active: self.similarity_mode_active(),
        }
    }

    fn visible_sample_row<'a>(
        &'a self,
        index: usize,
        query: VisibleSampleQuery<'a>,
    ) -> Option<VisibleSampleRow<'a>> {
        let file = self.selected_audio_file_at_matching_tags(index, query.tags_by_file)?;
        Some(VisibleSampleRow {
            file,
            selected: self.is_file_selected(&file.id),
            drag_revision: self.drag_revision(),
            drag_active: self.file_drag_active(),
            drag_source: self.file_drag_source(&file.id),
            cached: query.cached_sample_paths.contains(&file.id),
            rename: self.file_rename_view(&file.id),
            similarity_anchor: self.file_is_similarity_anchor(&file.id),
            similarity_strength: self.similarity_display_strength_for_file(&file.id),
            collection_colors: file
                .collection_memberships()
                .into_iter()
                .filter_map(|collection| self.collection_color(collection))
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_request_key_tracks_query_and_revision_inputs() {
        let ascending = ui::DetailsSort::new("name", ui::SortDirection::Ascending);
        let descending = ui::DetailsSort::new("size", ui::SortDirection::Descending);

        assert_ne!(
            VisibleSampleProjectionRequest::new("folder", "kick", &ascending, None, 4).key(),
            VisibleSampleProjectionRequest::new("folder", "kick", &descending, None, 4).key()
        );
        assert_ne!(
            VisibleSampleProjectionRequest::new("folder", "kick", &ascending, None, 4).key(),
            VisibleSampleProjectionRequest::new("folder", "snare", &ascending, None, 4).key()
        );
        assert_ne!(
            VisibleSampleProjectionRequest::new("folder", "kick", &ascending, Some("a.wav"), 4)
                .key(),
            VisibleSampleProjectionRequest::new("folder", "kick", &ascending, Some("b.wav"), 4)
                .key()
        );
        assert_ne!(
            VisibleSampleProjectionRequest::new("folder", "kick", &ascending, None, 4).key(),
            VisibleSampleProjectionRequest::new("folder", "kick", &ascending, None, 5).key()
        );
    }
}
