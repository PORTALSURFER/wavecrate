use std::path::PathBuf;

use radiant::prelude as ui;

use super::super::FolderBrowserState;
use super::SelectedFileRatingCandidate;

impl FolderBrowserState {
    pub(in crate::native_app) fn selected_file_paths(&self) -> Vec<PathBuf> {
        let selected = self.selection.active_file_ids();
        self.selected_audio_files()
            .into_iter()
            .filter(|file| selected.contains(&file.id))
            .filter(|file| !file.is_missing())
            .map(|file| PathBuf::from(&file.id))
            .collect()
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

    pub(in crate::native_app) fn selected_file_rating_candidates(
        &self,
    ) -> Vec<SelectedFileRatingCandidate> {
        let selected = self.selection.active_file_ids();
        self.selected_audio_files()
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
        let selected = self.selection.active_file_ids();
        self.selected_audio_files()
            .into_iter()
            .filter(|file| selected.contains(&file.id))
            .count()
    }

    pub(in crate::native_app::sample_library::folder_browser) fn selected_audio_file_ids(
        &self,
    ) -> Vec<String> {
        self.selected_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect()
    }

    pub(in crate::native_app::sample_library::folder_browser) fn selected_audio_file_ids_matching_tags(
        &self,
        tags_by_file: &std::collections::HashMap<String, Vec<String>>,
    ) -> Vec<String> {
        self.selected_audio_files_matching_tags(tags_by_file)
            .into_iter()
            .map(|file| file.id.clone())
            .collect()
    }
}

fn random_candidate_from_ids(file_ids: &[String], unit: f32) -> Option<String> {
    let index = ui::unit_interval_index(unit, file_ids.len())?;
    Some(file_ids[index].clone())
}

fn first_audio_file_in_folder(folder: &super::super::FolderEntry) -> Option<&str> {
    if let Some(file) = folder.files.iter().find(|file| file.is_audio()) {
        return Some(file.id.as_str());
    }
    folder.children.iter().find_map(first_audio_file_in_folder)
}

#[cfg(test)]
mod tests {
    use super::random_candidate_from_ids;

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
}
