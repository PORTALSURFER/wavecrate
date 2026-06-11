use std::{collections::HashSet, path::PathBuf};

use wavecrate::sample_sources::SampleCollection;

use super::{super::FolderBrowserState, model::SelectedFileCollectionCandidate};

impl FolderBrowserState {
    pub(in crate::native_app) fn set_file_collection_state(
        &mut self,
        path: &std::path::Path,
        collection: SampleCollection,
    ) -> bool {
        let path_id = path.to_string_lossy();
        let mut updated = false;
        for folder in &mut self.tree.folders {
            updated |= folder.set_file_collection(path_id.as_ref(), collection);
        }
        for source in &mut self.source.sources {
            if let Some(root_folder) = &mut source.root_folder {
                updated |= root_folder.set_file_collection(path_id.as_ref(), collection);
            }
        }
        if updated {
            self.bump_file_content_revision();
        }
        updated
    }

    pub(in crate::native_app) fn remove_file_collection_state(
        &mut self,
        path: &std::path::Path,
        collection: SampleCollection,
    ) -> bool {
        let path_id = path.to_string_lossy();
        let mut updated = false;
        for folder in &mut self.tree.folders {
            updated |= folder.remove_file_collection(path_id.as_ref(), collection);
        }
        for source in &mut self.source.sources {
            if let Some(root_folder) = &mut source.root_folder {
                updated |= root_folder.remove_file_collection(path_id.as_ref(), collection);
            }
        }
        self.reconcile_active_collection_selection(collection);
        if updated {
            self.bump_file_content_revision();
        }
        updated
    }

    pub(in crate::native_app) fn selected_file_collection_candidates(
        &self,
        collection: SampleCollection,
    ) -> Vec<SelectedFileCollectionCandidate> {
        self.selected_audio_files()
            .into_iter()
            .filter(|file| self.is_file_selected(&file.id))
            .map(|file| SelectedFileCollectionCandidate {
                path: PathBuf::from(&file.id),
                assigned: file.belongs_to_collection(collection),
            })
            .collect()
    }

    pub(in crate::native_app) fn context_file_collection_candidate(
        &self,
        path: &std::path::Path,
        collection: SampleCollection,
    ) -> Option<SelectedFileCollectionCandidate> {
        let path_id = path.to_string_lossy();
        self.selected_audio_files()
            .into_iter()
            .find(|file| file.id == path_id)
            .map(|file| SelectedFileCollectionCandidate {
                path: PathBuf::from(&file.id),
                assigned: file.belongs_to_collection(collection),
            })
    }

    pub(in crate::native_app) fn active_collection_for_context_file(
        &self,
        path: &std::path::Path,
    ) -> Option<SampleCollection> {
        let collection = self.selection.selected_collection?;
        self.context_file_collection_candidate(path, collection)
            .filter(|candidate| candidate.assigned)
            .map(|_| collection)
    }

    fn reconcile_active_collection_selection(&mut self, collection: SampleCollection) {
        if self.selection.selected_collection != Some(collection) {
            return;
        }
        let visible_ids = self
            .selected_audio_files()
            .into_iter()
            .map(|file| file.id.clone())
            .collect::<Vec<_>>();
        let visible_id_set = visible_ids.iter().cloned().collect::<HashSet<_>>();
        self.selection
            .selected_file_ids
            .retain(|file_id| visible_id_set.contains(file_id));
        if self
            .selection
            .selected_file
            .as_ref()
            .is_some_and(|file_id| !visible_id_set.contains(file_id))
        {
            self.selection.selected_file = visible_ids.first().cloned();
        }
    }
}
