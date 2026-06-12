use radiant::prelude as ui;
use std::{cell::Ref, collections::HashMap, path::PathBuf};
use wavecrate::sample_sources::SampleCollection;

use super::{
    FileEntry, FolderBrowserState, FolderEntry, SimilarityBrowserState,
    file_columns::{sort_file_indices_by_column_kind, sort_kind_for_details_sort},
    visible_samples::VisibleSampleProjectionRequest,
};

impl FolderBrowserState {
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
