use std::{collections::HashMap, path::PathBuf};

use radiant::prelude as ui;

use super::super::{FileEntry, FolderBrowserState, curation};
use super::SelectedFileRatingCandidate;

impl FolderBrowserState {
    pub(in crate::native_app) fn selected_file_paths(&self) -> Vec<PathBuf> {
        self.selected_existing_audio_files()
            .into_iter()
            .map(|file| PathBuf::from(&file.id))
            .collect()
    }

    pub(in crate::native_app) fn explicit_multi_file_selection_active(&self) -> bool {
        self.selection.selected_file_ids_explicit && self.selection.selected_file_ids.len() > 1
    }

    pub(in crate::native_app) fn explicit_selected_file_paths(&self) -> Vec<PathBuf> {
        if !self.explicit_multi_file_selection_active() {
            return Vec::new();
        }
        let selected = &self.selection.selected_file_ids;
        let mut paths = self
            .loaded_source_audio_files()
            .into_iter()
            .filter(|file| selected.contains(&file.id))
            .filter(|file| !file.is_missing())
            .map(|file| PathBuf::from(&file.id))
            .collect::<Vec<_>>();
        paths.sort();
        paths
    }

    pub(in crate::native_app) fn selected_normalization_paths(&self) -> Vec<PathBuf> {
        if self.selection.selected_collection.is_some() {
            return self.selected_file_paths();
        }
        let mut paths = self
            .selection
            .active_file_ids()
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        paths.sort();
        paths
    }

    pub(in crate::native_app) fn active_file_ids_for_diagnostics(&self) -> Vec<String> {
        let mut ids = self
            .selection
            .active_file_ids()
            .into_iter()
            .collect::<Vec<_>>();
        ids.sort();
        ids
    }

    pub(in crate::native_app) fn selected_file_ids_for_diagnostics(&self) -> Vec<String> {
        let mut ids = self
            .selection
            .selected_file_ids()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        ids.sort();
        ids
    }

    pub(in crate::native_app) fn selected_file_ids_explicit_for_diagnostics(&self) -> bool {
        self.selection.selected_file_ids_explicit()
    }

    pub(in crate::native_app) fn first_audio_file_path(&self) -> Option<PathBuf> {
        self.source
            .sources
            .iter()
            .find(|source| source.id == self.source.selected_source)
            .and_then(|source| source.root_folder.as_ref())
            .and_then(first_audio_file_in_folder)
            .map(PathBuf::from)
    }

    pub(in crate::native_app) fn random_playback_available(&self) -> bool {
        !self.selected_audio_file_ids().is_empty()
            || self
                .selected_source_audio_files()
                .into_iter()
                .next()
                .is_some()
    }

    pub(in crate::native_app) fn random_playback_candidate(&self, unit: f32) -> Option<String> {
        let visible_files = self.selected_audio_file_ids();
        if let Some(candidate) = random_candidate_from_ids(&visible_files, unit) {
            return Some(candidate);
        }

        let source_files = self
            .selected_source_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>();
        random_candidate_from_ids(&source_files, unit)
    }

    pub(in crate::native_app) fn random_listed_playback_candidate(
        &self,
        unit: f32,
        tags_by_file: &HashMap<String, Vec<String>>,
        avoid_file_id: Option<&str>,
    ) -> Option<String> {
        let listed_files = self.selected_audio_file_ids_matching_tags(tags_by_file);
        random_candidate_from_ids_excluding(&listed_files, unit, avoid_file_id)
            .or_else(|| random_candidate_from_ids(&listed_files, unit))
    }

    pub(in crate::native_app) fn selected_file_rating_candidates_matching_tags(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Vec<SelectedFileRatingCandidate> {
        let selected = self.selection.active_file_ids();
        self.selected_audio_files_matching_tags(tags_by_file)
            .into_iter()
            .filter(|file| selected.contains(&file.id))
            .filter(|file| !file.is_missing())
            .map(|file| SelectedFileRatingCandidate {
                path: PathBuf::from(&file.id),
                rating: file.rating,
                locked: file.rating_locked,
            })
            .collect()
    }

    pub(in crate::native_app) fn selected_audio_file_count(&self) -> usize {
        self.selected_existing_audio_files().len()
    }

    pub(in crate::native_app::sample_library::folder_browser) fn selected_audio_file_ids(
        &self,
    ) -> Vec<String> {
        self.selected_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect()
    }

    pub(in crate::native_app) fn selected_audio_file_ids_matching_tags(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Vec<String> {
        self.browser_listing_snapshot(tags_by_file).ids().to_vec()
    }

    pub(in crate::native_app::sample_library::folder_browser) fn selected_curation_bucket_file_ids_matching_tags(
        &self,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> Vec<String> {
        if !self.curation_mode_enabled() {
            return self.selected_audio_file_ids_matching_tags(tags_by_file);
        }
        let now = curation::now_epoch_seconds();
        let files = self.selected_audio_files_matching_tags(tags_by_file);
        let Some(best_priority) = files
            .iter()
            .filter_map(|file| {
                curation::curation_priority_for_file(file, tags_by_file, self.curation_mode(), now)
            })
            .min()
        else {
            return Vec::new();
        };
        files
            .into_iter()
            .filter(|file| {
                curation::curation_priority_for_file(file, tags_by_file, self.curation_mode(), now)
                    == Some(best_priority)
            })
            .map(|file| file.id.clone())
            .collect()
    }

    fn selected_existing_audio_files(&self) -> Vec<&FileEntry> {
        let selected = self.selection.active_file_ids();
        if selected.is_empty() {
            return Vec::new();
        }
        self.loaded_source_audio_files()
            .into_iter()
            .filter(|file| selected.contains(&file.id))
            .filter(|file| !file.is_missing())
            .collect()
    }
}

fn random_candidate_from_ids(file_ids: &[String], unit: f32) -> Option<String> {
    let index = ui::unit_interval_index(unit, file_ids.len())?;
    Some(file_ids[index].clone())
}

fn random_candidate_from_ids_excluding(
    file_ids: &[String],
    unit: f32,
    avoid_file_id: Option<&str>,
) -> Option<String> {
    let avoid_file_id = avoid_file_id?;
    let candidates = file_ids
        .iter()
        .filter(|id| id.as_str() != avoid_file_id)
        .cloned()
        .collect::<Vec<_>>();
    random_candidate_from_ids(&candidates, unit)
}

fn first_audio_file_in_folder(folder: &super::super::FolderEntry) -> Option<&str> {
    if let Some(file) = folder.files.iter().find(|file| file.is_audio()) {
        return Some(file.id.as_str());
    }
    folder.children.iter().find_map(first_audio_file_in_folder)
}

#[cfg(test)]
mod tests {
    use super::{random_candidate_from_ids, random_candidate_from_ids_excluding};

    #[test]
    fn random_candidate_clamps_unit_to_available_id_range() {
        let ids = vec![String::from("a"), String::from("b"), String::from("c")];

        assert_eq!(
            random_candidate_from_ids(&ids, 0.0),
            Some(String::from("a"))
        );
        assert_eq!(
            random_candidate_from_ids(&ids, 0.5),
            Some(String::from("b"))
        );
        assert_eq!(
            random_candidate_from_ids(&ids, 1.0),
            Some(String::from("c"))
        );
    }

    #[test]
    fn random_candidate_returns_none_for_empty_id_list() {
        assert_eq!(random_candidate_from_ids(&[], 0.5), None);
    }

    #[test]
    fn random_candidate_can_skip_current_file_when_alternatives_exist() {
        let ids = vec![String::from("a"), String::from("b"), String::from("c")];

        assert_eq!(
            random_candidate_from_ids_excluding(&ids, 0.0, Some("a")),
            Some(String::from("b"))
        );
        assert_eq!(
            random_candidate_from_ids_excluding(&ids, 1.0, Some("c")),
            Some(String::from("b"))
        );
        assert_eq!(
            random_candidate_from_ids_excluding(&ids, 0.5, Some("missing")),
            Some(String::from("b"))
        );
    }
}
