use radiant::prelude as ui;

use crate::native_app::sample_library::folder_browser::{FolderBrowserState, FolderEntry};

impl FolderBrowserState {
    pub(in crate::native_app::sample_library::folder_browser) fn selected_audio_file_index_without_tag_filter(
        &self,
        selected: &str,
    ) -> Option<usize> {
        if let Some(collection) = self.selection.selected_collection {
            let ids = self.selected_collection_audio_file_ids_ref(collection);
            return ids.iter().position(|id| id == selected);
        }

        let folder = self.selected_folder()?;
        if self.folder_subtree_listing_enabled() {
            let ids = self.selected_folder_recursive_audio_file_ids_ref(folder);
            return ids.iter().position(|id| id == selected);
        }

        let indices = self.selected_folder_audio_file_indices_ref(folder);
        index_in_folder_indices(folder, indices.as_slice(), selected)
    }

    pub(in crate::native_app::sample_library::folder_browser) fn neighboring_selected_audio_file_id_without_tag_filter(
        &self,
        selected: &str,
        delta: i32,
    ) -> Option<String> {
        if let Some(collection) = self.selection.selected_collection {
            let ids = self.selected_collection_audio_file_ids_ref(collection);
            return neighboring_id_in_ids(ids.as_slice(), selected, delta);
        }

        let folder = self.selected_folder()?;
        if self.folder_subtree_listing_enabled() {
            let ids = self.selected_folder_recursive_audio_file_ids_ref(folder);
            return neighboring_id_in_ids(ids.as_slice(), selected, delta);
        }

        let indices = self.selected_folder_audio_file_indices_ref(folder);
        neighboring_id_in_folder_indices(folder, indices.as_slice(), selected, delta)
    }
}

fn neighboring_id_in_ids(ids: &[String], selected: &str, delta: i32) -> Option<String> {
    let current_index = ids.iter().position(|id| id == selected)?;
    let target_index = ui::list_index_after_delta(current_index, delta as isize, ids.len())?;
    if target_index == current_index {
        return None;
    }
    ids.get(target_index).cloned()
}

fn neighboring_id_in_folder_indices(
    folder: &FolderEntry,
    indices: &[usize],
    selected: &str,
    delta: i32,
) -> Option<String> {
    let current_index = index_in_folder_indices(folder, indices, selected)?;
    let target_index = ui::list_index_after_delta(current_index, delta as isize, indices.len())?;
    if target_index == current_index {
        return None;
    }
    indices
        .get(target_index)
        .and_then(|file_index| folder.files.get(*file_index))
        .map(|file| file.id.clone())
}

fn index_in_folder_indices(
    folder: &FolderEntry,
    indices: &[usize],
    selected: &str,
) -> Option<usize> {
    indices.iter().position(|file_index| {
        folder
            .files
            .get(*file_index)
            .is_some_and(|file| file.id == selected)
    })
}
