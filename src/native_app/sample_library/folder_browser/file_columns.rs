use radiant::{prelude as ui, widgets::DragHandleMessage};

use super::{FileColumn, FileColumnDragFeedback, FileEntry, FolderBrowserState};

pub(in crate::native_app) const MIN_FILE_COLUMN_WIDTH: f32 = 48.0;
const MAX_FILE_COLUMN_WIDTH: f32 = 420.0;
pub(in crate::native_app) const FILE_COLUMN_GAP: f32 = 10.0;
const FILE_COLUMN_RESIZE_HANDLE_WIDTH: f32 = 4.0;
const FILE_COLUMN_DROP_MARKER_HANDLE_OFFSET: f32 =
    FILE_COLUMN_GAP + FILE_COLUMN_RESIZE_HANDLE_WIDTH * 0.5;

impl FolderBrowserState {
    pub(in crate::native_app) fn visible_file_columns(&self) -> Vec<&FileColumn> {
        self.sample_list.file_columns.iter().collect()
    }

    pub(in crate::native_app) fn file_sort(&self) -> &ui::DetailsSort {
        &self.sample_list.file_sort
    }

    pub(in crate::native_app) fn file_column_drag_feedback(
        &self,
    ) -> Option<FileColumnDragFeedback> {
        let drag = self.sample_list.file_column_reorder.as_ref()?;
        let feedback = ui::details_column_drag_feedback(
            drag,
            &self.details_column_placements(),
            FILE_COLUMN_GAP,
            FILE_COLUMN_DROP_MARKER_HANDLE_OFFSET,
        )?;
        let column = self
            .sample_list
            .file_columns
            .iter()
            .find(|column| column.id == feedback.column_id)?;
        Some(FileColumnDragFeedback {
            label: column.label.clone(),
            pointer: feedback.pointer,
            width: feedback.width,
            marker_x: feedback.marker_x,
        })
    }

    pub(super) fn sort_file_column(&mut self, column_id: String) {
        if self.sample_list.file_sort.column_id == column_id {
            self.sample_list.file_sort.direction = self.sample_list.file_sort.direction.toggled();
        } else {
            self.sample_list.file_sort =
                ui::DetailsSort::new(column_id, ui::SortDirection::Ascending);
        }
    }

    pub(super) fn resize_file_column(&mut self, column_id: String, message: DragHandleMessage) {
        let current_width = self
            .sample_list
            .file_columns
            .iter()
            .find(|column| column.id == column_id)
            .map(|column| column.width);
        let Some(update) = ui::update_details_column_resize_drag(
            &mut self.sample_list.file_column_resize,
            column_id,
            message,
            current_width,
            MIN_FILE_COLUMN_WIDTH,
            MAX_FILE_COLUMN_WIDTH,
        ) else {
            return;
        };
        let Some(column) = self
            .sample_list
            .file_columns
            .iter_mut()
            .find(|column| column.id == update.column_id)
        else {
            return;
        };
        column.width = update.width;
    }

    pub(super) fn drag_file_column(&mut self, column_id: String, message: DragHandleMessage) {
        let placements = self.details_column_placements();
        ui::update_details_column_reorder_drag(
            &mut self.sample_list.file_column_reorder,
            &mut self.sample_list.file_columns,
            column_id,
            message,
            &placements,
            FILE_COLUMN_GAP,
            |column: &FileColumn| column.id.as_str(),
        );
    }

    pub(super) fn cancel_file_column_drag(&mut self) {
        self.sample_list.file_column_reorder = None;
        self.sample_list.file_column_resize = None;
    }

    pub(in crate::native_app) fn file_column_drag_active(&self) -> bool {
        self.sample_list.file_column_reorder.is_some()
            || self.sample_list.file_column_resize.is_some()
    }

    fn details_column_placements(&self) -> Vec<ui::DetailsColumnPlacement> {
        self.sample_list
            .file_columns
            .iter()
            .map(|column| ui::DetailsColumnPlacement::new(column.id.as_str(), column.width))
            .collect()
    }

    pub(super) fn sort_files(&self, files: &mut Vec<&FileEntry>) {
        match self.sample_list.file_sort.column_id.as_str() {
            "extension" => files.sort_by_cached_key(|file| {
                (file.extension.to_ascii_lowercase(), file.name_sort_key())
            }),
            "size" => files.sort_by_cached_key(|file| (file.size_bytes, file.name_sort_key())),
            "modified" => {
                files.sort_by_cached_key(|file| (file.modified_rank, file.name_sort_key()))
            }
            "kind" => files.sort_by_cached_key(|file| (file.kind.clone(), file.name_sort_key())),
            "rating" => files.sort_by_cached_key(|file| (file.rating.val(), file.name_sort_key())),
            "collection" => files.sort_by_cached_key(|file| {
                (
                    file.first_collection().map(|collection| collection.index()),
                    file.name_sort_key(),
                )
            }),
            "similarity" => self.sort_files_by_similarity(files),
            "path" => files.sort_by(|a, b| a.id.cmp(&b.id)),
            _ => files.sort_by_cached_key(|file| file.name_sort_key()),
        }
        if self.sample_list.file_sort.direction == ui::SortDirection::Descending {
            files.reverse();
        }
        if self.sample_list.file_sort.column_id.as_str() != "similarity" {
            self.sort_files_by_similarity(files);
        }
    }

    fn sort_files_by_similarity(&self, files: &mut [&FileEntry]) {
        let Some(similarity) = self.sample_list.similarity.as_ref() else {
            return;
        };
        let base_order = files
            .iter()
            .enumerate()
            .map(|(order, file)| (file.id.as_str(), order))
            .collect::<std::collections::HashMap<_, _>>();
        files
            .sort_by(|left, right| similarity_file_ref_order(similarity, &base_order, left, right));
    }
}

fn similarity_file_ref_order(
    similarity: &super::SimilarityBrowserState,
    base_order: &std::collections::HashMap<&str, usize>,
    left: &FileEntry,
    right: &FileEntry,
) -> std::cmp::Ordering {
    match (
        left.id == similarity.anchor_id(),
        right.id == similarity.anchor_id(),
    ) {
        (true, false) => return std::cmp::Ordering::Less,
        (false, true) => return std::cmp::Ordering::Greater,
        _ => {}
    }
    match (
        similarity.raw_score_for(&left.id),
        similarity.raw_score_for(&right.id),
    ) {
        (Some(left_score), Some(right_score)) => {
            right_score.total_cmp(&left_score).then_with(|| {
                base_order_for(left.id.as_str(), base_order)
                    .cmp(&base_order_for(right.id.as_str(), base_order))
            })
        }
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => base_order_for(left.id.as_str(), base_order)
            .cmp(&base_order_for(right.id.as_str(), base_order)),
    }
}

fn base_order_for(id: &str, base_order: &std::collections::HashMap<&str, usize>) -> usize {
    base_order.get(id).copied().unwrap_or(usize::MAX)
}
