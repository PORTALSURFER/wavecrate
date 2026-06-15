use std::{cell::Ref, collections::HashMap, path::PathBuf};

use super::{
    FileEntry, FolderBrowserState, FolderEntry,
    visible_samples::{VisibleSampleProjectionRequest, VisibleSampleWindowFiles},
};

mod cache_warming;
mod filters;
mod ordering;
mod traversal;

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
                traversal::collect_collection_audio_files(folder, collection, &mut files);
            }
            filters::filter_audio_files_by_name(&mut files, &self.filters.name_filter);
            self.sort_files(&mut files);
            return files;
        }

        let Some(folder) = self.selected_folder() else {
            return Vec::new();
        };
        self.selected_folder_audio_files(folder)
    }

    pub(in crate::native_app) fn selected_cache_candidate_paths(
        &self,
        max_files: usize,
    ) -> Vec<PathBuf> {
        if max_files == 0 {
            return Vec::new();
        }

        let name_query = filters::normalized_name_filter(&self.filters.name_filter);
        if let Some(collection) = self.selection.selected_collection {
            let mut paths = Vec::new();
            if let Some(folder) = self.selected_source_root_folder() {
                collect_collection_cache_candidate_paths(
                    folder,
                    collection,
                    &name_query,
                    max_files,
                    &mut paths,
                );
            }
            return paths;
        }

        let Some(folder) = self.selected_folder() else {
            return Vec::new();
        };
        collect_local_cache_candidate_paths(folder, &name_query, max_files)
    }

    pub(in crate::native_app) fn selected_folder_cache_warm_request(
        &self,
        max_files: usize,
    ) -> Option<(String, Vec<PathBuf>)> {
        let folder = self.selected_folder()?;
        let name_query = filters::normalized_name_filter(&self.filters.name_filter);
        let paths = collect_local_cache_candidate_paths(folder, &name_query, max_files);
        Some((folder.id.clone(), paths))
    }

    pub(in crate::native_app) fn selected_audio_files_matching_tags(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Vec<&FileEntry> {
        let mut files = self.selected_audio_files();
        filters::filter_audio_files_by_tags(&mut files, tags_by_file, &self.filters.tag_filter);
        files
    }

    pub(in crate::native_app) fn selected_audio_file_count_matching_tags(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> usize {
        let name_query = filters::normalized_name_filter(&self.filters.name_filter);
        let required_tags = filters::parsed_tag_filter(&self.filters.tag_filter);
        if required_tags.is_empty() && self.selection.selected_collection.is_none() {
            return self.selected_folder_audio_file_count();
        }
        if let Some(collection) = self.selection.selected_collection {
            return self
                .selected_source_root_folder()
                .map(|folder| {
                    traversal::count_matching_audio_files_in_folder(
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
                    && filters::audio_file_matches_name_query(file, &name_query)
                    && filters::audio_file_matches_parsed_tags(file, tags_by_file, &required_tags)
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

    pub(super) fn selected_audio_file_window_matching_tags(
        &self,
        window: radiant::prelude::VirtualListWindow,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> VisibleSampleWindowFiles<'_> {
        if self.selection.selected_collection.is_some() {
            let files = self.selected_audio_files_matching_tags(tags_by_file);
            let total_count = files.len();
            return VisibleSampleWindowFiles {
                total_count,
                rows: (window.window_start.min(total_count)..window.window_end.min(total_count))
                    .map(|index| files.get(index).copied())
                    .collect(),
            };
        }

        let Some(folder) = self.selected_folder() else {
            return VisibleSampleWindowFiles {
                total_count: 0,
                rows: Vec::new(),
            };
        };

        let required_tags = filters::parsed_tag_filter(&self.filters.tag_filter);
        let indices = self.selected_folder_audio_file_indices_ref(folder);
        if required_tags.is_empty() {
            let total_count = indices.len();
            return VisibleSampleWindowFiles {
                total_count,
                rows: (window.window_start.min(total_count)..window.window_end.min(total_count))
                    .map(|index| {
                        indices
                            .get(index)
                            .and_then(|file_index| folder.files.get(*file_index))
                    })
                    .collect(),
            };
        }

        let mut total_count = 0;
        let rows = indices
            .iter()
            .filter_map(|file_index| folder.files.get(*file_index))
            .filter(|file| {
                filters::audio_file_matches_parsed_tags(file, tags_by_file, &required_tags)
            })
            .filter_map(|file| {
                let row = (total_count >= window.window_start && total_count < window.window_end)
                    .then_some(Some(file));
                total_count += 1;
                row
            })
            .collect();

        VisibleSampleWindowFiles { total_count, rows }
    }

    pub(in crate::native_app) fn selected_audio_file_index_matching_tags(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Option<usize> {
        let selected = self.selection.selected_file.as_deref()?;
        let required_tags = filters::parsed_tag_filter(&self.filters.tag_filter);
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
            .filter(|file| {
                filters::audio_file_matches_parsed_tags(file, tags_by_file, &required_tags)
            })
            .position(|file| file.id == selected)
    }

    pub(in crate::native_app) fn selected_source_audio_files(&self) -> Vec<&FileEntry> {
        let mut files = Vec::new();
        if let Some(folder) = self.selected_source_root_folder() {
            traversal::collect_audio_files(folder, &mut files);
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
        let name_filter = filters::normalized_name_filter(&self.filters.name_filter);
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
                        file.is_audio()
                            && filters::audio_file_matches_name_query(file, &name_filter)
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                ordering::sort_file_indices(self, folder, &mut indices);
                ordering::sort_file_indices_by_similarity(self, folder, &mut indices);
                indices
            })
    }

    pub(super) fn prewarm_selected_source_audio_projection_cache(&self) {
        cache_warming::prewarm_selected_source_audio_projection_cache(self);
    }

    #[cfg(test)]
    pub(in crate::native_app) fn selected_audio_projection_cache_len_for_tests(&self) -> usize {
        self.sample_list.projection_cache.len()
    }
}

fn collect_local_cache_candidate_paths(
    folder: &FolderEntry,
    name_query: &str,
    max_files: usize,
) -> Vec<PathBuf> {
    folder
        .files
        .iter()
        .filter(|file| file.is_audio() && filters::audio_file_matches_name_query(file, name_query))
        .take(max_files)
        .map(|file| PathBuf::from(&file.id))
        .collect()
}

fn collect_collection_cache_candidate_paths(
    folder: &FolderEntry,
    collection: wavecrate::sample_sources::SampleCollection,
    name_query: &str,
    max_files: usize,
    paths: &mut Vec<PathBuf>,
) {
    if paths.len() >= max_files {
        return;
    }

    paths.extend(
        folder
            .files
            .iter()
            .filter(|file| {
                file.is_audio()
                    && file.belongs_to_collection(collection)
                    && filters::audio_file_matches_name_query(file, name_query)
            })
            .take(max_files.saturating_sub(paths.len()))
            .map(|file| PathBuf::from(&file.id)),
    );

    for child in &folder.children {
        if paths.len() >= max_files {
            break;
        }
        collect_collection_cache_candidate_paths(child, collection, name_query, max_files, paths);
    }
}
