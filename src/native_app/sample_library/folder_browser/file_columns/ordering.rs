use std::collections::HashMap;
use std::path::Path;

use super::super::{
    FileColumnKind, FileEntry, FolderBrowserState, FolderEntry, SimilarityBrowserState, curation,
    playback_type_filter,
};

impl FolderBrowserState {
    pub(in crate::native_app) fn sort_file_column(&mut self, column_id: String) {
        let Some(kind) = FileColumnKind::from_id(&column_id) else {
            return;
        };
        let column_id = kind.id();
        if self.sample_list.file_sort.column_id == column_id {
            self.sample_list.file_sort.direction = self.sample_list.file_sort.direction.toggled();
        } else {
            self.sample_list.file_sort = radiant::application::DetailsSort::new(
                column_id.to_owned(),
                radiant::application::SortDirection::Ascending,
            );
        }
    }

    pub(in crate::native_app) fn sort_files(&self, files: &mut Vec<&FileEntry>) {
        self.sort_files_with_tag_metadata(files, None);
    }

    pub(in crate::native_app) fn sort_files_matching_tags(
        &self,
        files: &mut Vec<&FileEntry>,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) {
        self.sort_files_with_tag_metadata(files, Some(tags_by_file));
    }

    fn sort_files_with_tag_metadata(
        &self,
        files: &mut Vec<&FileEntry>,
        tags_by_file: Option<&HashMap<String, Vec<String>>>,
    ) {
        let kind = sort_kind_for_details_sort(&self.sample_list.file_sort);
        if kind == FileColumnKind::Similarity {
            self.sort_files_by_similarity(files);
        } else {
            sort_file_refs_by_column_kind(kind, files, tags_by_file);
        }
        if self.sample_list.file_sort.direction == radiant::application::SortDirection::Descending {
            files.reverse();
        }
        if self.filters.curation.enabled
            && let Some(tags_by_file) = tags_by_file
        {
            curation::sort_files_for_curation(
                files,
                tags_by_file,
                &self.filters.curation,
                curation::now_epoch_seconds(),
            );
            return;
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
            .collect::<HashMap<_, _>>();
        files
            .sort_by(|left, right| similarity_file_ref_order(similarity, &base_order, left, right));
    }
}

pub(in crate::native_app) fn sort_file_indices_by_column_kind(
    kind: FileColumnKind,
    folder: &FolderEntry,
    indices: &mut [usize],
    tags_by_file: Option<&HashMap<String, Vec<String>>>,
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
        FileColumnKind::PlaybackType => indices.sort_by_cached_key(|index| {
            let file = &folder.files[*index];
            (
                playback_type_filter::playback_type_sort_rank(file, tags_by_file),
                file.name_sort_key(),
            )
        }),
        FileColumnKind::Collection => indices.sort_by_cached_key(|index| {
            let file = &folder.files[*index];
            (
                file.first_collection().map(|collection| collection.index()),
                file.name_sort_key(),
            )
        }),
        FileColumnKind::SourceFolder => indices.sort_by_cached_key(|index| {
            let file = &folder.files[*index];
            (source_folder_sort_key(&file.id), file.name_sort_key())
        }),
        FileColumnKind::Path => {
            indices.sort_by(|a, b| folder.files[*a].id.cmp(&folder.files[*b].id))
        }
        FileColumnKind::Name
        | FileColumnKind::Curation
        | FileColumnKind::Harvest
        | FileColumnKind::Similarity => {
            indices.sort_by_cached_key(|index| folder.files[*index].name_sort_key())
        }
    }
}

pub(in crate::native_app) fn sort_kind_for_details_sort(
    sort: &radiant::application::DetailsSort,
) -> FileColumnKind {
    FileColumnKind::from_id(sort.column_id.as_str()).unwrap_or(FileColumnKind::Name)
}

fn sort_file_refs_by_column_kind(
    kind: FileColumnKind,
    files: &mut [&FileEntry],
    tags_by_file: Option<&HashMap<String, Vec<String>>>,
) {
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
        FileColumnKind::PlaybackType => files.sort_by_cached_key(|file| {
            (
                playback_type_filter::playback_type_sort_rank(file, tags_by_file),
                file.name_sort_key(),
            )
        }),
        FileColumnKind::Collection => files.sort_by_cached_key(|file| {
            (
                file.first_collection().map(|collection| collection.index()),
                file.name_sort_key(),
            )
        }),
        FileColumnKind::SourceFolder => {
            files.sort_by_cached_key(|file| {
                (source_folder_sort_key(&file.id), file.name_sort_key())
            });
        }
        FileColumnKind::Path => files.sort_by(|a, b| a.id.cmp(&b.id)),
        FileColumnKind::Name
        | FileColumnKind::Curation
        | FileColumnKind::Harvest
        | FileColumnKind::Similarity => files.sort_by_cached_key(|file| file.name_sort_key()),
    }
}

fn source_folder_sort_key(file_id: &str) -> String {
    Path::new(file_id)
        .parent()
        .map(|path| path.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

fn similarity_file_ref_order(
    similarity: &SimilarityBrowserState,
    base_order: &HashMap<&str, usize>,
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
        similarity.effective_score_for(&left.id),
        similarity.effective_score_for(&right.id),
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

fn base_order_for(id: &str, base_order: &HashMap<&str, usize>) -> usize {
    base_order.get(id).copied().unwrap_or(usize::MAX)
}

#[cfg(test)]
pub(super) fn sort_file_refs_by_column_kind_for_tests(
    kind: FileColumnKind,
    files: &mut [&FileEntry],
    tags_by_file: Option<&HashMap<String, Vec<String>>>,
) {
    sort_file_refs_by_column_kind(kind, files, tags_by_file);
}
