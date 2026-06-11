use radiant::prelude as ui;
use std::collections::{HashMap, HashSet};

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
