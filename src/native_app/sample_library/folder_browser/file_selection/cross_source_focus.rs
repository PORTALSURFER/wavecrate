use std::path::{Path, PathBuf};

use super::super::{FolderBrowserState, path_helpers::path_id, scanning::load_root_folder};

impl FolderBrowserState {
    pub(in crate::native_app) fn focus_file_across_sources(&mut self, path: &Path) -> bool {
        self.ensure_loaded_source_containing_path(path);
        let file_id = path_id(path);
        let Some(parent) = path.parent() else {
            return false;
        };
        let parent_id = path_id(parent);
        let Some((source_id, source_root, root_folder)) =
            self.find_loaded_source_containing_file(path, parent, &file_id)
        else {
            return false;
        };

        let source_changed = self.source.selected_source != source_id;
        self.cancel_rename();
        self.selection.selected_collection = None;
        self.selection.folder_before_collection = None;
        self.collection_panel.rename_edit = None;
        self.source.selected_source = source_id;
        self.selection.select_folder_after_tree_changed(parent_id);
        self.selection.set_focus_file_set(file_id);
        self.reset_file_view();
        self.tree.folders = vec![root_folder];
        self.prewarm_selected_source_audio_projection_cache();
        if source_changed {
            self.tree.expanded_folders.clear();
        }
        self.tree
            .expanded_folders
            .extend(folder_ancestor_ids(&source_root, parent));
        true
    }

    fn find_loaded_source_containing_file(
        &self,
        path: &Path,
        parent: &Path,
        file_id: &str,
    ) -> Option<(String, PathBuf, super::super::FolderEntry)> {
        self.source.sources.iter().find_map(|source| {
            if !path.starts_with(&source.root) {
                return None;
            }
            let root_folder = source.root_folder.as_ref()?;
            let parent_folder = root_folder.find(&path_id(parent))?;
            parent_folder
                .files
                .iter()
                .any(|file| file.id == file_id && file.is_audio())
                .then(|| (source.id.clone(), source.root.clone(), root_folder.clone()))
        })
    }

    fn ensure_loaded_source_containing_path(&mut self, path: &Path) {
        let Some(index) = self
            .source
            .sources
            .iter()
            .enumerate()
            .filter(|(_, source)| path.starts_with(&source.root))
            .max_by_key(|(_, source)| source.root.components().count())
            .map(|(index, _)| index)
        else {
            return;
        };
        if self.source.sources[index].root_folder.is_none() {
            let root = self.source.sources[index].root.clone();
            self.source.sources[index].root_folder = Some(load_root_folder(root));
            self.source.sources[index].loading_task = None;
            self.bump_file_content_revision();
        }
    }
}

fn folder_ancestor_ids(root: &Path, folder: &Path) -> Vec<String> {
    let mut ids = vec![path_id(root)];
    let Ok(relative) = folder.strip_prefix(root) else {
        return ids;
    };
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        ids.push(path_id(&current));
    }
    ids
}
