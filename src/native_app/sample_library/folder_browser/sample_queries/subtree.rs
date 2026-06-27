use std::{
    cell::Ref,
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use radiant::prelude as ui;

use super::rating_filter_allows_file;
use super::{
    curation, curation_filter_allows_file, filters, playback_type_filter, rating_filter, traversal,
};
use crate::native_app::sample_library::folder_browser::{
    FileEntry, FolderBrowserState, FolderEntry,
    visible_samples::{VisibleSampleProjectionRequest, VisibleSampleWindowFiles},
};

impl FolderBrowserState {
    pub(super) fn selected_folder_recursive_audio_files_with_sort_tags<'a>(
        &self,
        folder: &'a FolderEntry,
        sort_tags: Option<&HashMap<String, Vec<String>>>,
    ) -> Vec<&'a FileEntry> {
        let ids =
            self.selected_folder_recursive_audio_file_ids_ref_with_sort_tags(folder, sort_tags);
        recursive_audio_files_for_ordered_ids(folder, ids.as_slice())
    }

    pub(super) fn selected_folder_recursive_audio_file_window_matching_tags<'a>(
        &'a self,
        folder: &'a FolderEntry,
        window: ui::VirtualListWindow,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> VisibleSampleWindowFiles<'a> {
        let required_tags = self.active_required_tags();
        let playback_type_filters = self.active_playback_type_filters();
        let ids = self.selected_folder_recursive_audio_file_ids_ref_with_sort_tags(
            folder,
            Some(tags_by_file),
        );
        if required_tags.is_empty()
            && playback_type_filters.is_empty()
            && !self.filters.curation.enabled
        {
            let total_count = ids.len();
            let window_start = window.window_start.min(total_count);
            let window_end = window.window_end.min(total_count);
            return VisibleSampleWindowFiles {
                total_count,
                rows: recursive_audio_files_for_window_ids(
                    folder,
                    &ids.as_slice()[window_start..window_end],
                ),
            };
        }

        let files_by_id = recursive_audio_file_lookup_for_ids(folder, ids.as_slice());
        let mut total_count = 0;
        let rows = ids
            .iter()
            .filter_map(|id| files_by_id.get(id.as_str()).copied())
            .filter(|file| {
                filters::audio_file_matches_parsed_tags(file, tags_by_file, &required_tags)
                    && playback_type_filter::playback_type_filter_matches(
                        file,
                        tags_by_file,
                        &playback_type_filters,
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

    pub(super) fn selected_folder_recursive_audio_file_index_matching_tags(
        &self,
        folder: &FolderEntry,
        selected: &str,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Option<usize> {
        let required_tags = self.active_required_tags();
        let playback_type_filters = self.active_playback_type_filters();
        let ids = self.selected_folder_recursive_audio_file_ids_ref_with_sort_tags(
            folder,
            Some(tags_by_file),
        );
        if required_tags.is_empty()
            && playback_type_filters.is_empty()
            && !self.filters.curation.enabled
        {
            return ids.iter().position(|id| id == selected);
        }

        let files_by_id = recursive_audio_file_lookup_for_ids(folder, ids.as_slice());
        ids.iter()
            .filter_map(|id| files_by_id.get(id.as_str()).copied())
            .filter(|file| {
                filters::audio_file_matches_parsed_tags(file, tags_by_file, &required_tags)
                    && playback_type_filter::playback_type_filter_matches(
                        file,
                        tags_by_file,
                        &playback_type_filters,
                    )
            })
            .position(|file| file.id == selected)
    }

    pub(super) fn selected_folder_recursive_audio_file_ids_ref(
        &self,
        folder: &FolderEntry,
    ) -> Ref<'_, Vec<String>> {
        self.selected_folder_recursive_audio_file_ids_ref_with_sort_tags(folder, None)
    }

    pub(super) fn selected_folder_recursive_audio_file_ids_ref_with_sort_tags(
        &self,
        folder: &FolderEntry,
        sort_tags: Option<&HashMap<String, Vec<String>>>,
    ) -> Ref<'_, Vec<String>> {
        let name_filter = filters::normalized_name_filter(self.active_name_filter());
        let active_rating_filter = self.active_rating_filter();
        let rating_filter_key = rating_filter::rating_filter_key(&active_rating_filter);
        let listing_reveal_id = self.active_listing_reveal_id(sort_tags);
        let curation_key = if sort_tags.is_some() {
            self.filters.curation.cache_key()
        } else {
            String::new()
        };
        let request = VisibleSampleProjectionRequest::new(
            folder.id.as_str(),
            name_filter.as_str(),
            rating_filter_key.as_str(),
            curation_key.as_str(),
            &self.sample_list.file_sort,
            self.similarity_anchor_id(),
            self.sample_list.content_revision,
        )
        .with_listing_reveal(listing_reveal_id)
        .with_playback_type_tag_sort(self.playback_type_tag_sort_enabled(sort_tags));
        self.sample_list.projection_cache.audio_ids(request, || {
            let curation_now = curation::now_epoch_seconds();
            let mut files = Vec::new();
            traversal::collect_audio_files(folder, &mut files);
            filters::filter_audio_files_by_name(&mut files, self.active_name_filter());
            files.retain(|file| {
                rating_filter_allows_file(file, &active_rating_filter, listing_reveal_id)
                    && curation_filter_allows_file(
                        file,
                        sort_tags,
                        &self.filters.curation,
                        curation_now,
                        listing_reveal_id,
                    )
            });
            if let Some(tags_by_file) = sort_tags {
                self.sort_files_matching_tags(&mut files, tags_by_file);
            } else {
                self.sort_files(&mut files);
            }
            files.into_iter().map(|file| file.id.clone()).collect()
        })
    }
}

fn recursive_audio_files_for_ordered_ids<'a>(
    folder: &'a FolderEntry,
    ids: &[String],
) -> Vec<&'a FileEntry> {
    let files_by_id = recursive_audio_file_lookup_for_ids(folder, ids);
    materialize_ordered_ids(ids, &files_by_id)
}

fn recursive_audio_files_for_window_ids<'a>(
    folder: &'a FolderEntry,
    ids: &[String],
) -> Vec<&'a FileEntry> {
    if ids.is_empty() {
        return Vec::new();
    }

    let files_by_id = recursive_audio_file_lookup_for_ids(folder, ids);
    materialize_ordered_ids(ids, &files_by_id)
}

fn materialize_ordered_ids<'a>(
    ids: &[String],
    files_by_id: &HashMap<&str, &'a FileEntry>,
) -> Vec<&'a FileEntry> {
    ids.iter()
        .filter_map(|id| files_by_id.get(id.as_str()).copied())
        .collect()
}

fn recursive_audio_file_lookup_for_ids<'a>(
    folder: &'a FolderEntry,
    ids: &[String],
) -> HashMap<&'a str, &'a FileEntry> {
    if ids.is_empty() {
        return HashMap::new();
    }

    let wanted = ids.iter().map(String::as_str).collect::<HashSet<_>>();
    let mut files_by_id = HashMap::with_capacity(wanted.len());
    traversal::collect_audio_files_matching_ids(folder, &wanted, &mut files_by_id);
    files_by_id
}

pub(super) fn collect_recursive_cache_candidate_paths(
    folder: &FolderEntry,
    name_query: &str,
    max_files: usize,
) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_recursive_cache_candidate_paths_into(folder, name_query, max_files, &mut paths);
    paths
}

fn collect_recursive_cache_candidate_paths_into(
    folder: &FolderEntry,
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
                file.is_audio() && filters::audio_file_matches_name_query(file, name_query)
            })
            .take(max_files.saturating_sub(paths.len()))
            .map(|file| PathBuf::from(&file.id)),
    );

    for child in &folder.children {
        if paths.len() >= max_files {
            break;
        }
        collect_recursive_cache_candidate_paths_into(child, name_query, max_files, paths);
    }
}
