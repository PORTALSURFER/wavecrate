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

    pub(in crate::native_app) fn remove_moved_file_collection_states(
        &mut self,
        moved_paths: &[(PathBuf, PathBuf)],
        collection: SampleCollection,
    ) -> bool {
        let mut updated = false;
        for (_, new_path) in moved_paths {
            let path_id = new_path.to_string_lossy();
            for folder in &mut self.tree.folders {
                updated |= folder.remove_file_collection(path_id.as_ref(), collection);
            }
            for source in &mut self.source.sources {
                if let Some(root_folder) = &mut source.root_folder {
                    updated |= root_folder.remove_file_collection(path_id.as_ref(), collection);
                }
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
            .filter(|file| !file.is_missing())
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

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, fs, path::PathBuf};

    use wavecrate::sample_sources::SampleCollection;

    use super::super::super::{FolderBrowserState, path_id};

    #[test]
    fn selected_file_collection_candidates_report_membership_for_explicit_selection() {
        let Fixture {
            root,
            mut browser,
            kick,
            snare,
            hat,
            first,
            second: _,
        } = fixture();
        browser.set_file_collection_state(&kick, first);

        browser.selection.selected_file = Some(path_id(&kick));
        browser.selection.selected_file_ids =
            HashSet::from([path_id(&kick), path_id(&snare), path_id(&hat)]);
        browser.selection.selected_file_ids_explicit = true;

        assert_eq!(
            browser.selected_file_collection_candidates(first),
            vec![
                candidate(&hat, false),
                candidate(&kick, true),
                candidate(&snare, false),
            ]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn context_file_collection_candidate_uses_visible_file_membership() {
        let Fixture {
            root,
            mut browser,
            kick,
            snare,
            hat: _,
            first,
            second,
        } = fixture();
        browser.set_file_collection_state(&kick, first);
        browser.set_file_collection_state(&kick, second);

        assert_eq!(
            browser.context_file_collection_candidate(&kick, second),
            Some(candidate(&kick, true))
        );
        assert_eq!(
            browser.context_file_collection_candidate(&snare, second),
            Some(candidate(&snare, false))
        );
        assert_eq!(
            browser.active_collection_for_context_file(&kick),
            None,
            "context collection lookup should require collection focus"
        );

        browser.selection.selected_collection = Some(second);
        assert_eq!(
            browser.active_collection_for_context_file(&kick),
            Some(second)
        );
        assert_eq!(browser.active_collection_for_context_file(&snare), None);
        let _ = fs::remove_dir_all(root);
    }

    struct Fixture {
        root: PathBuf,
        browser: FolderBrowserState,
        kick: PathBuf,
        snare: PathBuf,
        hat: PathBuf,
        first: SampleCollection,
        second: SampleCollection,
    }

    fn fixture() -> Fixture {
        let root = temp_source_root("wavecrate-collection-assignment");
        let kick = root.join("kick.wav");
        let snare = root.join("snare.wav");
        let hat = root.join("hat.wav");
        fs::write(&kick, []).expect("write kick sample");
        fs::write(&snare, []).expect("write snare sample");
        fs::write(&hat, []).expect("write hat sample");
        Fixture {
            browser: FolderBrowserState::from_root(root.clone()),
            root,
            kick,
            snare,
            hat,
            first: SampleCollection::new(0).expect("collection 0"),
            second: SampleCollection::new(1).expect("collection 1"),
        }
    }

    fn candidate(path: &std::path::Path, assigned: bool) -> super::SelectedFileCollectionCandidate {
        super::SelectedFileCollectionCandidate {
            path: PathBuf::from(path_id(path)),
            assigned,
        }
    }

    fn temp_source_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "{name}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("create temp root");
        root
    }
}
