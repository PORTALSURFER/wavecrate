use std::{cell::Ref, collections::HashMap, path::PathBuf};

use radiant::prelude as ui;

use super::{filters, rating_filter, traversal};
use crate::native_app::sample_library::folder_browser::{
    FileEntry, FolderBrowserState, FolderEntry,
    visible_samples::{VisibleSampleProjectionRequest, VisibleSampleWindowFiles},
};

impl FolderBrowserState {
    pub(super) fn selected_folder_recursive_audio_files<'a>(
        &self,
        folder: &'a FolderEntry,
    ) -> Vec<&'a FileEntry> {
        self.selected_folder_recursive_audio_file_ids_ref(folder)
            .iter()
            .filter_map(|id| folder.find_file(id))
            .collect()
    }

    pub(super) fn selected_folder_recursive_audio_file_window_matching_tags<'a>(
        &'a self,
        folder: &'a FolderEntry,
        window: ui::VirtualListWindow,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> VisibleSampleWindowFiles<'a> {
        let required_tags = filters::parsed_tag_filter(&self.filters.tag_filter);
        let ids = self.selected_folder_recursive_audio_file_ids_ref(folder);
        if required_tags.is_empty() {
            let total_count = ids.len();
            return VisibleSampleWindowFiles {
                total_count,
                rows: (window.window_start.min(total_count)..window.window_end.min(total_count))
                    .filter_map(|index| ids.get(index))
                    .filter_map(|id| folder.find_file(id))
                    .collect(),
            };
        }

        let mut total_count = 0;
        let rows = ids
            .iter()
            .filter_map(|id| folder.find_file(id))
            .filter(|file| {
                filters::audio_file_matches_parsed_tags(file, tags_by_file, &required_tags)
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

    pub(super) fn selected_folder_recursive_audio_file_ids_ref(
        &self,
        folder: &FolderEntry,
    ) -> Ref<'_, Vec<String>> {
        let name_filter = filters::normalized_name_filter(&self.filters.name_filter);
        let rating_filter_key = rating_filter::rating_filter_key(&self.filters.rating_filter);
        let request = VisibleSampleProjectionRequest::new(
            folder.id.as_str(),
            name_filter.as_str(),
            rating_filter_key.as_str(),
            &self.sample_list.file_sort,
            self.similarity_anchor_id(),
            self.sample_list.content_revision,
        );
        self.sample_list.projection_cache.audio_ids(request, || {
            let mut files = Vec::new();
            traversal::collect_audio_files(folder, &mut files);
            filters::filter_audio_files_by_name(&mut files, &self.filters.name_filter);
            files.retain(|file| {
                rating_filter::rating_filter_matches(file, &self.filters.rating_filter)
            });
            self.sort_files(&mut files);
            files.into_iter().map(|file| file.id.clone()).collect()
        })
    }
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
