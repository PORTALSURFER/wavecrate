use std::{cell::Ref, collections::HashMap, path::PathBuf};

use super::{
    FileColumnKind, FileEntry, FolderBrowserState, FolderEntry,
    file_columns::sort_kind_for_details_sort,
    playback_type_filter, rating_filter,
    visible_samples::{VisibleSampleProjectionRequest, VisibleSampleWindowFiles},
};

mod adjacent;
mod cache_warming;
mod collection;
mod filters;
mod ordering;
mod subtree;
mod traversal;

impl FolderBrowserState {
    pub(in crate::native_app) fn selected_files(&self) -> &[FileEntry] {
        self.selected_folder()
            .map(|folder| folder.files.as_slice())
            .unwrap_or(&[])
    }

    pub(in crate::native_app) fn selected_audio_files(&self) -> Vec<&FileEntry> {
        self.selected_audio_files_with_sort_tags(None)
    }

    fn selected_audio_files_with_sort_tags(
        &self,
        sort_tags: Option<&HashMap<String, Vec<String>>>,
    ) -> Vec<&FileEntry> {
        if let Some(collection) = self.selection.selected_collection {
            return self.selected_collection_audio_files_with_sort_tags(collection, sort_tags);
        }

        let Some(folder) = self.selected_folder() else {
            return Vec::new();
        };
        if self.folder_subtree_listing_enabled() {
            return self.selected_folder_recursive_audio_files_with_sort_tags(folder, sort_tags);
        }
        self.selected_folder_audio_files_with_sort_tags(folder, sort_tags)
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
            for folder in self.loaded_source_root_folders() {
                collect_collection_cache_candidate_paths(
                    folder,
                    collection,
                    &name_query,
                    max_files,
                    &mut paths,
                );
                if paths.len() >= max_files {
                    break;
                }
            }
            return paths;
        }

        let Some(folder) = self.selected_folder() else {
            return Vec::new();
        };
        if self.folder_subtree_listing_enabled() {
            return subtree::collect_recursive_cache_candidate_paths(
                folder,
                &name_query,
                max_files,
            );
        }
        collect_local_cache_candidate_paths(folder, &name_query, max_files)
    }

    pub(in crate::native_app) fn selected_source_cache_warm_request(
        &self,
    ) -> Option<(String, Vec<PathBuf>)> {
        self.source_cache_warm_request(self.source.selected_source.as_str())
    }

    pub(in crate::native_app) fn source_cache_warm_request(
        &self,
        source_id: &str,
    ) -> Option<(String, Vec<PathBuf>)> {
        let folder = self.source_root_folder(source_id)?;
        let mut paths = Vec::new();
        collect_source_cache_candidate_paths(folder, &mut paths);
        Some((folder.id.clone(), paths))
    }

    pub(in crate::native_app) fn selected_audio_files_matching_tags(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Vec<&FileEntry> {
        let mut files = self.selected_audio_files_with_sort_tags(Some(tags_by_file));
        filters::filter_audio_files_by_tags(&mut files, tags_by_file, &self.filters.tag_filter);
        files.retain(|file| {
            playback_type_filter::playback_type_filter_matches(
                file,
                tags_by_file,
                &self.filters.playback_type_filter,
            )
        });
        files
    }

    pub(in crate::native_app) fn selected_audio_file_count_matching_tags(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> usize {
        let name_query = filters::normalized_name_filter(&self.filters.name_filter);
        let required_tags = filters::parsed_tag_filter(&self.filters.tag_filter);
        let playback_type_filter = &self.filters.playback_type_filter;
        if required_tags.is_empty()
            && playback_type_filter.is_empty()
            && self.selection.selected_collection.is_none()
        {
            return self.selected_folder_audio_file_count();
        }
        if let Some(collection) = self.selection.selected_collection {
            if required_tags.is_empty() && playback_type_filter.is_empty() {
                return self
                    .selected_collection_audio_file_ids_ref(collection)
                    .len();
            }
            return self.selected_audio_files_matching_tags(tags_by_file).len();
        }
        if self.folder_subtree_listing_enabled() {
            return self
                .selected_folder()
                .map(|folder| {
                    traversal::count_matching_audio_files_in_folder(
                        folder,
                        &name_query,
                        &required_tags,
                        tags_by_file,
                        &self.filters.rating_filter,
                        playback_type_filter,
                        None,
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
                    && playback_type_filter::playback_type_filter_matches(
                        file,
                        tags_by_file,
                        playback_type_filter,
                    )
                    && rating_filter::rating_filter_matches(file, &self.filters.rating_filter)
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
        if self.folder_subtree_listing_enabled() {
            return self
                .selected_folder_recursive_audio_file_ids_ref(folder)
                .len();
        }
        self.selected_folder_audio_file_indices_ref(folder).len()
    }

    pub(super) fn selected_audio_file_window_matching_tags(
        &self,
        window: radiant::prelude::VirtualListWindow,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> VisibleSampleWindowFiles<'_> {
        if let Some(collection) = self.selection.selected_collection {
            return self.selected_collection_audio_file_window_matching_tags(
                collection,
                window,
                tags_by_file,
            );
        }

        let Some(folder) = self.selected_folder() else {
            return VisibleSampleWindowFiles {
                total_count: 0,
                rows: Vec::new(),
            };
        };
        if self.folder_subtree_listing_enabled() {
            return self.selected_folder_recursive_audio_file_window_matching_tags(
                folder,
                window,
                tags_by_file,
            );
        }

        let required_tags = filters::parsed_tag_filter(&self.filters.tag_filter);
        let playback_type_filter = &self.filters.playback_type_filter;
        let indices =
            self.selected_folder_audio_file_indices_ref_with_sort_tags(folder, Some(tags_by_file));
        if required_tags.is_empty() && playback_type_filter.is_empty() {
            let total_count = indices.len();
            return VisibleSampleWindowFiles {
                total_count,
                rows: (window.window_start.min(total_count)..window.window_end.min(total_count))
                    .filter_map(|index| {
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
                    && playback_type_filter::playback_type_filter_matches(
                        file,
                        tags_by_file,
                        playback_type_filter,
                    )
            })
            .filter_map(|file| {
                let row = (total_count >= window.window_start && total_count < window.window_end)
                    .then_some(file);
                total_count += 1;
                row
            })
            .collect();

        VisibleSampleWindowFiles { total_count, rows }
    }

    pub(super) fn uncached_selected_audio_file_window_matching_tags(
        &self,
        window: radiant::prelude::VirtualListWindow,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> VisibleSampleWindowFiles<'_> {
        let files = self.uncached_selected_audio_files_matching_tags(tags_by_file);
        let total_count = files.len();
        VisibleSampleWindowFiles {
            total_count,
            rows: (window.window_start.min(total_count)..window.window_end.min(total_count))
                .filter_map(|index| files.get(index).copied())
                .collect(),
        }
    }

    fn uncached_selected_audio_files_matching_tags(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Vec<&FileEntry> {
        if self.selection.selected_collection.is_some() || self.folder_subtree_listing_enabled() {
            return self.selected_audio_files_matching_tags(tags_by_file);
        }

        let Some(folder) = self.selected_folder() else {
            return Vec::new();
        };
        let name_query = filters::normalized_name_filter(&self.filters.name_filter);
        let required_tags = filters::parsed_tag_filter(&self.filters.tag_filter);
        let playback_type_filter = &self.filters.playback_type_filter;
        let mut files = folder
            .files
            .iter()
            .filter(|file| {
                file.is_audio()
                    && filters::audio_file_matches_name_query(file, &name_query)
                    && filters::audio_file_matches_parsed_tags(file, tags_by_file, &required_tags)
                    && playback_type_filter::playback_type_filter_matches(
                        file,
                        tags_by_file,
                        playback_type_filter,
                    )
                    && rating_filter::rating_filter_matches(file, &self.filters.rating_filter)
            })
            .collect::<Vec<_>>();
        self.sort_files_matching_tags(&mut files, tags_by_file);
        files
    }

    pub(in crate::native_app) fn selected_audio_file_index_matching_tags(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Option<usize> {
        let selected = self.selection.selected_file.as_deref()?;
        let required_tags = filters::parsed_tag_filter(&self.filters.tag_filter);
        let playback_type_filter = &self.filters.playback_type_filter;
        if let Some(collection) = self.selection.selected_collection {
            if required_tags.is_empty() && playback_type_filter.is_empty() {
                return self
                    .selected_collection_audio_file_ids_ref_with_sort_tags(
                        collection,
                        Some(tags_by_file),
                    )
                    .iter()
                    .position(|id| id == selected);
            }
            return self
                .selected_audio_files_matching_tags(tags_by_file)
                .iter()
                .position(|file| file.id == selected);
        }
        if self.folder_subtree_listing_enabled() {
            return self.selected_folder().and_then(|folder| {
                self.selected_folder_recursive_audio_file_index_matching_tags(
                    folder,
                    selected,
                    tags_by_file,
                )
            });
        }
        let folder = self.selected_folder()?;
        self.selected_folder_audio_file_indices_ref_with_sort_tags(folder, Some(tags_by_file))
            .iter()
            .filter_map(|file_index| folder.files.get(*file_index))
            .filter(|file| {
                filters::audio_file_matches_parsed_tags(file, tags_by_file, &required_tags)
                    && playback_type_filter::playback_type_filter_matches(
                        file,
                        tags_by_file,
                        playback_type_filter,
                    )
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

    pub(in crate::native_app) fn loaded_source_audio_files(&self) -> Vec<&FileEntry> {
        let mut files = Vec::new();
        for folder in self.loaded_source_root_folders() {
            traversal::collect_audio_files(folder, &mut files);
        }
        self.sort_files(&mut files);
        files
    }

    fn loaded_source_root_folders(&self) -> Vec<&FolderEntry> {
        let mut folders = self
            .source
            .sources
            .iter()
            .filter_map(|source| source.root_folder.as_ref())
            .collect::<Vec<_>>();
        for folder in &self.tree.folders {
            if !folders
                .iter()
                .any(|source_root| source_root.id == folder.id)
            {
                folders.push(folder);
            }
        }
        folders
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

    fn source_root_folder(&self, source_id: &str) -> Option<&FolderEntry> {
        self.source
            .sources
            .iter()
            .find(|source| source.id == source_id)
            .and_then(|source| source.root_folder.as_ref())
    }

    fn selected_folder_audio_files_with_sort_tags<'a>(
        &self,
        folder: &'a FolderEntry,
        sort_tags: Option<&HashMap<String, Vec<String>>>,
    ) -> Vec<&'a FileEntry> {
        self.selected_folder_audio_file_indices_ref_with_sort_tags(folder, sort_tags)
            .iter()
            .filter_map(|index| folder.files.get(*index))
            .collect()
    }

    fn selected_folder_audio_file_indices_ref(&self, folder: &FolderEntry) -> Ref<'_, Vec<usize>> {
        self.selected_folder_audio_file_indices_ref_with_sort_tags(folder, None)
    }

    pub(super) fn selected_folder_audio_file_indices_ref_with_sort_tags(
        &self,
        folder: &FolderEntry,
        sort_tags: Option<&HashMap<String, Vec<String>>>,
    ) -> Ref<'_, Vec<usize>> {
        let name_filter = filters::normalized_name_filter(&self.filters.name_filter);
        let rating_filter_key = rating_filter::rating_filter_key(&self.filters.rating_filter);
        let request = VisibleSampleProjectionRequest::new(
            folder.id.as_str(),
            name_filter.as_str(),
            rating_filter_key.as_str(),
            &self.sample_list.file_sort,
            self.similarity_anchor_id(),
            self.sample_list.content_revision,
        )
        .with_playback_type_tag_sort(self.playback_type_tag_sort_enabled(sort_tags));
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
                            && rating_filter::rating_filter_matches(
                                file,
                                &self.filters.rating_filter,
                            )
                    })
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                if let Some(tags_by_file) = sort_tags {
                    ordering::sort_file_indices_matching_tags(
                        self,
                        folder,
                        &mut indices,
                        tags_by_file,
                    );
                } else {
                    ordering::sort_file_indices(self, folder, &mut indices);
                }
                ordering::sort_file_indices_by_similarity(self, folder, &mut indices);
                indices
            })
    }

    pub(super) fn playback_type_tag_sort_enabled(
        &self,
        sort_tags: Option<&HashMap<String, Vec<String>>>,
    ) -> bool {
        sort_tags.is_some()
            && sort_kind_for_details_sort(&self.sample_list.file_sort)
                == FileColumnKind::PlaybackType
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

fn filter_audio_files_by_rating(
    files: &mut Vec<&FileEntry>,
    rating_filter: &std::collections::BTreeSet<i8>,
) {
    files.retain(|file| rating_filter::rating_filter_matches(file, rating_filter));
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

fn collect_source_cache_candidate_paths(folder: &FolderEntry, paths: &mut Vec<PathBuf>) {
    paths.extend(
        folder
            .files
            .iter()
            .filter(|file| file.is_audio())
            .map(|file| PathBuf::from(&file.id)),
    );

    for child in &folder.children {
        collect_source_cache_candidate_paths(child, paths);
    }
}
