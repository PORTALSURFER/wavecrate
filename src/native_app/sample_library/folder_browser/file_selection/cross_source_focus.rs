use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use super::super::{
    FolderBrowserState, listing::BrowserListingRevealReason, path_helpers::path_id,
    scanning::load_source_snapshot,
};

impl FolderBrowserState {
    pub(in crate::native_app) fn focus_file_across_sources(&mut self, path: &Path) -> bool {
        self.focus_file_across_sources_with_visible_ids(path, None)
    }

    pub(in crate::native_app) fn focus_file_across_sources_matching_tags(
        &mut self,
        path: &Path,
        tags_by_file: &HashMap<String, Vec<String>>,
    ) -> bool {
        self.focus_file_across_sources_matching_tags_for_reason(
            path,
            tags_by_file,
            BrowserListingRevealReason::LoadedFileFocus,
        )
    }

    pub(in crate::native_app) fn focus_file_across_sources_matching_tags_for_reason(
        &mut self,
        path: &Path,
        tags_by_file: &HashMap<String, Vec<String>>,
        reason: BrowserListingRevealReason,
    ) -> bool {
        self.focus_file_across_sources_with_visible_ids(path, Some((tags_by_file, reason)))
    }

    fn focus_file_across_sources_with_visible_ids(
        &mut self,
        path: &Path,
        tags_by_file: Option<(&HashMap<String, Vec<String>>, BrowserListingRevealReason)>,
    ) -> bool {
        self.ensure_loaded_source_containing_path(path);
        let file_id = path_id(path);
        if self.focus_file_in_current_visible_list(&file_id, tags_by_file.map(|(tags, _)| tags)) {
            return true;
        }
        if let Some((tags_by_file, reason)) = tags_by_file
            && self.focus_file_in_current_reveal_list(&file_id, tags_by_file, reason)
        {
            return true;
        }
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
        if source_changed {
            self.tree.expanded_folders.clear();
        }
        self.tree
            .expanded_folders
            .extend(folder_ancestor_ids(&source_root, parent));
        if let Some((tags_by_file, reason)) = tags_by_file {
            self.reveal_selected_file_if_hidden(tags_by_file, reason);
        }
        true
    }

    fn focus_file_in_current_visible_list(
        &mut self,
        file_id: &str,
        tags_by_file: Option<&HashMap<String, Vec<String>>>,
    ) -> bool {
        let visible_ids = tags_by_file.map_or_else(
            || self.selected_audio_file_ids(),
            |tags_by_file| self.selected_audio_file_ids_matching_tags(tags_by_file),
        );
        if !visible_ids.iter().any(|id| id == file_id) {
            return false;
        }
        self.cancel_rename();
        self.selection.set_focus_file_set(file_id.to_owned());
        true
    }

    fn focus_file_in_current_reveal_list(
        &mut self,
        file_id: &str,
        tags_by_file: &HashMap<String, Vec<String>>,
        reason: BrowserListingRevealReason,
    ) -> bool {
        let snapshot = self.selection.snapshot();
        let previous_reveals = self.sample_list.listing_reveals.clone();
        self.selection.set_focus_file_set(file_id.to_owned());
        self.sample_list
            .listing_reveals
            .set(file_id.to_owned(), reason);
        self.sample_list.projection_cache.clear();

        let visible = self
            .selected_audio_file_ids_matching_tags(tags_by_file)
            .iter()
            .any(|id| id == file_id);
        if visible {
            self.cancel_rename();
            return true;
        }

        self.selection.restore_snapshot(snapshot);
        self.sample_list.listing_reveals = previous_reveals;
        self.sample_list.projection_cache.clear();
        false
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
            let database_root = self.source.sources[index].database_root.clone();
            let snapshot = load_source_snapshot(root, database_root);
            self.source.sources[index].root_folder = Some(snapshot.folder);
            self.source.sources[index].missing_collection_snapshot =
                snapshot.missing_collection_snapshot;
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
