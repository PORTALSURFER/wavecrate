use super::{FolderBrowserState, FolderEntry};

impl FolderBrowserState {
    pub(super) fn mutate_selected_source_trees(
        &mut self,
        mut mutate: impl FnMut(&mut FolderEntry) -> bool,
    ) -> bool {
        let active_tree_changed = self
            .tree
            .folders
            .first_mut()
            .is_some_and(|root_folder| mutate(root_folder));

        let mut parked_tree_snapshot = None;
        let parked_tree_changed = self
            .source
            .sources
            .iter_mut()
            .find(|source| source.id == self.source.selected_source)
            .and_then(|source| source.root_folder.as_mut())
            .is_some_and(|root_folder| {
                let changed = mutate(root_folder);
                if changed && !active_tree_changed {
                    parked_tree_snapshot = Some(root_folder.clone());
                }
                changed
            });

        if let Some(root_folder) = parked_tree_snapshot {
            self.tree.folders = vec![root_folder];
        }

        active_tree_changed || parked_tree_changed
    }

    pub(super) fn try_mutate_selected_source_trees(
        &mut self,
        unavailable_error: &str,
        mut mutate: impl FnMut(&mut FolderEntry) -> Result<(), String>,
    ) -> Result<(), String> {
        let mut active_tree_snapshot = None;
        let mut first_error = None;

        if let Some(root_folder) = self.tree.folders.first_mut() {
            match mutate(root_folder) {
                Ok(()) => active_tree_snapshot = Some(root_folder.clone()),
                Err(error) => first_error = Some(error),
            }
        }

        let mut parked_tree_snapshot = None;
        let parked_tree_mutated = self
            .source
            .sources
            .iter_mut()
            .find(|source| source.id == self.source.selected_source)
            .and_then(|source| source.root_folder.as_mut())
            .is_some_and(|root_folder| match mutate(root_folder) {
                Ok(()) => {
                    if active_tree_snapshot.is_none() {
                        parked_tree_snapshot = Some(root_folder.clone());
                    }
                    true
                }
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                    false
                }
            });

        if let Some(root_folder) = parked_tree_snapshot {
            self.tree.folders = vec![root_folder];
        }

        if active_tree_snapshot.is_some() || parked_tree_mutated {
            Ok(())
        } else {
            Err(first_error.unwrap_or_else(|| unavailable_error.to_string()))
        }
    }
}
