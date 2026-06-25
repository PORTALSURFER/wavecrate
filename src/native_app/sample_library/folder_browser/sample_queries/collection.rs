use std::{
    cell::Ref,
    collections::{HashMap, HashSet},
};

use radiant::prelude as ui;
use wavecrate::sample_sources::SampleCollection;

use super::{
    curation, curation_filter_allows_file, filter_audio_files_by_rating, filters,
    playback_type_filter, rating_filter, traversal,
};
use crate::native_app::sample_library::folder_browser::{
    FileEntry, FolderBrowserState,
    visible_samples::{VisibleSampleProjectionRequest, VisibleSampleWindowFiles},
};

impl FolderBrowserState {
    pub(super) fn selected_collection_audio_files_with_sort_tags(
        &self,
        collection: SampleCollection,
        sort_tags: Option<&HashMap<String, Vec<String>>>,
    ) -> Vec<&FileEntry> {
        let ids = self.selected_collection_audio_file_ids_ref_with_sort_tags(collection, sort_tags);
        self.collection_audio_files_for_ordered_ids(ids.as_slice())
    }

    pub(super) fn selected_collection_audio_file_window_matching_tags(
        &self,
        collection: SampleCollection,
        window: ui::VirtualListWindow,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> VisibleSampleWindowFiles<'_> {
        let required_tags = filters::parsed_tag_filter(&self.filters.tag_filter);
        let playback_type_filters = &self.filters.playback_type_filter;
        let ids = self
            .selected_collection_audio_file_ids_ref_with_sort_tags(collection, Some(tags_by_file));
        if required_tags.is_empty()
            && playback_type_filters.is_empty()
            && !self.filters.curation.enabled
        {
            let total_count = ids.len();
            let window_start = window.window_start.min(total_count);
            let window_end = window.window_end.min(total_count);
            return VisibleSampleWindowFiles {
                total_count,
                rows: self.collection_audio_files_for_window_ids(
                    &ids.as_slice()[window_start..window_end],
                ),
            };
        }

        let files_by_id = self.collection_audio_file_lookup_for_ids(ids.as_slice());
        let mut total_count = 0;
        let rows = ids
            .iter()
            .filter_map(|id| files_by_id.get(id.as_str()).copied())
            .filter(|file| {
                filters::audio_file_matches_parsed_tags(file, tags_by_file, &required_tags)
                    && playback_type_filter::playback_type_filter_matches(
                        file,
                        tags_by_file,
                        playback_type_filters,
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

    pub(super) fn selected_collection_audio_file_ids_ref(
        &self,
        collection: SampleCollection,
    ) -> Ref<'_, Vec<String>> {
        self.selected_collection_audio_file_ids_ref_with_sort_tags(collection, None)
    }

    pub(super) fn selected_collection_audio_file_ids_ref_with_sort_tags(
        &self,
        collection: SampleCollection,
        sort_tags: Option<&HashMap<String, Vec<String>>>,
    ) -> Ref<'_, Vec<String>> {
        let name_filter = filters::normalized_name_filter(&self.filters.name_filter);
        let rating_filter_key = rating_filter::rating_filter_key(&self.filters.rating_filter);
        let collection_key = format!("collection:{}", collection.index());
        let curation_focus_override = self.active_curation_focus_override_id(sort_tags);
        let curation_key = if sort_tags.is_some() {
            self.filters.curation.cache_key()
        } else {
            String::new()
        };
        let request = VisibleSampleProjectionRequest::new(
            collection_key.as_str(),
            name_filter.as_str(),
            rating_filter_key.as_str(),
            curation_key.as_str(),
            &self.sample_list.file_sort,
            self.similarity_anchor_id(),
            self.sample_list.content_revision,
        )
        .with_curation_focus_override(curation_focus_override)
        .with_playback_type_tag_sort(self.playback_type_tag_sort_enabled(sort_tags));
        self.sample_list.projection_cache.audio_ids(request, || {
            let curation_now = curation::now_epoch_seconds();
            let mut files = Vec::new();
            for folder in self.loaded_source_root_folders() {
                traversal::collect_collection_audio_files(folder, collection, &mut files);
            }
            files.extend(
                self.sample_list
                    .missing_collection_files
                    .iter()
                    .filter(|file| file.belongs_to_collection(collection)),
            );
            filters::filter_audio_files_by_name(&mut files, &self.filters.name_filter);
            filter_audio_files_by_rating(&mut files, &self.filters.rating_filter);
            if self.filters.curation.enabled
                && let Some(tags_by_file) = sort_tags
            {
                files.retain(|file| {
                    curation_filter_allows_file(
                        file,
                        Some(tags_by_file),
                        &self.filters.curation,
                        curation_now,
                        curation_focus_override,
                    )
                });
            }
            if let Some(tags_by_file) = sort_tags {
                self.sort_files_matching_tags(&mut files, tags_by_file);
            } else {
                self.sort_files(&mut files);
            }
            files.into_iter().map(|file| file.id.clone()).collect()
        })
    }

    fn collection_audio_files_for_ordered_ids(&self, ids: &[String]) -> Vec<&FileEntry> {
        let files_by_id = self.collection_audio_file_lookup_for_ids(ids);
        materialize_ordered_ids(ids, &files_by_id)
    }

    fn collection_audio_files_for_window_ids(&self, ids: &[String]) -> Vec<&FileEntry> {
        if ids.is_empty() {
            return Vec::new();
        }

        let files_by_id = self.collection_audio_file_lookup_for_ids(ids);
        materialize_ordered_ids(ids, &files_by_id)
    }

    fn collection_audio_file_lookup_for_ids<'a>(
        &'a self,
        ids: &[String],
    ) -> HashMap<&'a str, &'a FileEntry> {
        if ids.is_empty() {
            return HashMap::new();
        }

        let wanted = ids.iter().map(String::as_str).collect::<HashSet<_>>();
        let mut files_by_id = HashMap::with_capacity(wanted.len());
        for folder in self.loaded_source_root_folders() {
            traversal::collect_audio_files_matching_ids(folder, &wanted, &mut files_by_id);
            if files_by_id.len() == wanted.len() {
                return files_by_id;
            }
        }
        for file in &self.sample_list.missing_collection_files {
            if wanted.contains(file.id.as_str()) {
                files_by_id.insert(file.id.as_str(), file);
                if files_by_id.len() == wanted.len() {
                    return files_by_id;
                }
            }
        }
        files_by_id
    }
}

fn materialize_ordered_ids<'a>(
    ids: &[String],
    files_by_id: &HashMap<&str, &'a FileEntry>,
) -> Vec<&'a FileEntry> {
    ids.iter()
        .filter_map(|id| files_by_id.get(id.as_str()).copied())
        .collect()
}
