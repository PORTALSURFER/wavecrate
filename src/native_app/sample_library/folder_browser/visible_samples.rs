use radiant::prelude as ui;
use std::{
    cell::{Ref, RefCell},
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
};

use super::{FileColumn, FileEntry, FileRenameView, FolderBrowserState};

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
    pub(in crate::native_app) rows: Vec<VisibleSampleRow<'a>>,
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

#[derive(Clone, Debug, Default)]
pub(super) struct VisibleSampleProjectionCache {
    entries: RefCell<HashMap<VisibleSampleProjectionKey, Vec<usize>>>,
}

impl VisibleSampleProjectionCache {
    pub(super) fn get_or_build(
        &self,
        key: VisibleSampleProjectionKey,
        build: impl FnOnce() -> Vec<usize>,
    ) -> Ref<'_, Vec<usize>> {
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct VisibleSampleProjectionKey {
    pub(super) folder_id: String,
    pub(super) name_filter: String,
    pub(super) sort_column_id: String,
    pub(super) sort_descending: bool,
    pub(super) similarity_anchor_id: Option<String>,
    content_revision: u64,
}

impl VisibleSampleProjectionKey {
    pub(super) fn new(
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
            .filter_map(|index| self.visible_sample_row(index, query))
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
