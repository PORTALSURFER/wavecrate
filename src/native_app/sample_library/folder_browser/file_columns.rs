use radiant::{prelude as ui, widgets::DragHandleMessage};

use super::{
    FileColumn, FileColumnDragFeedback, FileColumnKind, FileEntry, FolderBrowserState, FolderEntry,
};

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
        let Some(kind) = FileColumnKind::from_id(&column_id) else {
            return;
        };
        let column_id = kind.id();
        if self.sample_list.file_sort.column_id == column_id {
            self.sample_list.file_sort.direction = self.sample_list.file_sort.direction.toggled();
        } else {
            self.sample_list.file_sort =
                ui::DetailsSort::new(column_id.to_owned(), ui::SortDirection::Ascending);
        }
    }

    pub(super) fn resize_file_column(&mut self, column_id: String, message: DragHandleMessage) {
        let Some(kind) = FileColumnKind::from_id(&column_id) else {
            return;
        };
        let current_width = self
            .sample_list
            .file_columns
            .iter()
            .find(|column| column.kind == kind)
            .map(|column| column.width);
        let Some(update) = ui::update_details_column_resize_drag(
            &mut self.sample_list.file_column_resize,
            kind.id().to_owned(),
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
        let Some(kind) = FileColumnKind::from_id(&column_id) else {
            return;
        };
        let placements = self.details_column_placements();
        ui::update_details_column_reorder_drag(
            &mut self.sample_list.file_column_reorder,
            &mut self.sample_list.file_columns,
            kind.id().to_owned(),
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
        let kind = sort_kind_for_details_sort(&self.sample_list.file_sort);
        if kind == FileColumnKind::Similarity {
            self.sort_files_by_similarity(files);
        } else {
            sort_file_refs_by_column_kind(kind, files);
        }
        if self.sample_list.file_sort.direction == ui::SortDirection::Descending {
            files.reverse();
        }
        if kind != FileColumnKind::Similarity {
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

pub(super) fn sort_file_indices_by_column_kind(
    kind: FileColumnKind,
    folder: &FolderEntry,
    indices: &mut [usize],
) {
    match kind {
        FileColumnKind::Extension => indices.sort_by_cached_key(|index| {
            let file = &folder.files[*index];
            (file.extension.to_ascii_lowercase(), file.name_sort_key())
        }),
        FileColumnKind::Size => indices.sort_by_cached_key(|index| {
            let file = &folder.files[*index];
            (file.size_bytes, file.name_sort_key())
        }),
        FileColumnKind::Modified => indices.sort_by_cached_key(|index| {
            let file = &folder.files[*index];
            (file.modified_rank, file.name_sort_key())
        }),
        FileColumnKind::Kind => indices.sort_by_cached_key(|index| {
            let file = &folder.files[*index];
            (file.kind.clone(), file.name_sort_key())
        }),
        FileColumnKind::Rating => indices.sort_by_cached_key(|index| {
            let file = &folder.files[*index];
            (file.rating.val(), file.name_sort_key())
        }),
        FileColumnKind::Collection => indices.sort_by_cached_key(|index| {
            let file = &folder.files[*index];
            (
                file.first_collection().map(|collection| collection.index()),
                file.name_sort_key(),
            )
        }),
        FileColumnKind::Path => {
            indices.sort_by(|a, b| folder.files[*a].id.cmp(&folder.files[*b].id))
        }
        FileColumnKind::Name | FileColumnKind::Similarity => {
            indices.sort_by_cached_key(|index| folder.files[*index].name_sort_key());
        }
    }
}

pub(super) fn sort_kind_for_details_sort(sort: &ui::DetailsSort) -> FileColumnKind {
    FileColumnKind::from_id(sort.column_id.as_str()).unwrap_or(FileColumnKind::Name)
}

fn sort_file_refs_by_column_kind(kind: FileColumnKind, files: &mut [&FileEntry]) {
    match kind {
        FileColumnKind::Extension => {
            files.sort_by_cached_key(|file| {
                (file.extension.to_ascii_lowercase(), file.name_sort_key())
            });
        }
        FileColumnKind::Size => {
            files.sort_by_cached_key(|file| (file.size_bytes, file.name_sort_key()))
        }
        FileColumnKind::Modified => {
            files.sort_by_cached_key(|file| (file.modified_rank, file.name_sort_key()));
        }
        FileColumnKind::Kind => {
            files.sort_by_cached_key(|file| (file.kind.clone(), file.name_sort_key()))
        }
        FileColumnKind::Rating => {
            files.sort_by_cached_key(|file| (file.rating.val(), file.name_sort_key()))
        }
        FileColumnKind::Collection => files.sort_by_cached_key(|file| {
            (
                file.first_collection().map(|collection| collection.index()),
                file.name_sort_key(),
            )
        }),
        FileColumnKind::Path => files.sort_by(|a, b| a.id.cmp(&b.id)),
        FileColumnKind::Name | FileColumnKind::Similarity => {
            files.sort_by_cached_key(|file| file.name_sort_key());
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use wavecrate::sample_sources::{Rating, SampleCollection};

    fn file_entry(
        stem: &str,
        id_prefix: &str,
        extension: &str,
        size_bytes: u64,
        modified_rank: u64,
        kind: &str,
        rating: Rating,
        collection: Option<SampleCollection>,
    ) -> FileEntry {
        FileEntry {
            id: format!("{id_prefix}/{stem}.{extension}"),
            name: format!("{stem}.{extension}"),
            stem: stem.to_owned(),
            extension: extension.to_owned(),
            kind: kind.to_owned(),
            size: format!("{size_bytes} B"),
            size_bytes,
            modified: modified_rank.to_string(),
            modified_rank,
            rating,
            rating_locked: false,
            collection,
            collections: collection.into_iter().collect(),
        }
    }

    fn sort_names_by(kind: FileColumnKind) -> Vec<String> {
        let low_collection = SampleCollection::new(0).expect("collection 0");
        let high_collection = SampleCollection::new(1).expect("collection 1");
        let files = vec![
            file_entry(
                "alpha",
                "C:/z",
                "wav",
                20,
                3,
                "Audio",
                Rating::NEUTRAL,
                None,
            ),
            file_entry(
                "bravo",
                "C:/a",
                "aif",
                10,
                2,
                "Loop",
                Rating::KEEP_1,
                Some(high_collection),
            ),
            file_entry(
                "charlie",
                "C:/m",
                "mp3",
                30,
                1,
                "Drum",
                Rating::TRASH_1,
                Some(low_collection),
            ),
        ];
        let mut file_refs = files.iter().collect::<Vec<_>>();
        sort_file_refs_by_column_kind(kind, &mut file_refs);
        file_refs
            .into_iter()
            .map(|file| file.stem.clone())
            .collect::<Vec<_>>()
    }

    #[test]
    fn typed_file_column_kinds_map_to_sort_behavior() {
        let cases = [
            (FileColumnKind::Name, vec!["alpha", "bravo", "charlie"]),
            (FileColumnKind::Extension, vec!["bravo", "charlie", "alpha"]),
            (FileColumnKind::Size, vec!["bravo", "alpha", "charlie"]),
            (FileColumnKind::Modified, vec!["charlie", "bravo", "alpha"]),
            (FileColumnKind::Kind, vec!["alpha", "charlie", "bravo"]),
            (FileColumnKind::Rating, vec!["charlie", "alpha", "bravo"]),
            (
                FileColumnKind::Collection,
                vec!["alpha", "charlie", "bravo"],
            ),
            (FileColumnKind::Path, vec!["bravo", "charlie", "alpha"]),
            (
                FileColumnKind::Similarity,
                vec!["alpha", "bravo", "charlie"],
            ),
        ];

        for (kind, expected) in cases {
            assert_eq!(sort_names_by(kind), expected, "{kind:?}");
        }
    }

    #[test]
    fn unknown_sort_id_falls_back_to_name_kind() {
        assert_eq!(
            sort_kind_for_details_sort(&ui::DetailsSort::new(
                "missing-column",
                ui::SortDirection::Ascending,
            )),
            FileColumnKind::Name
        );
    }
}
