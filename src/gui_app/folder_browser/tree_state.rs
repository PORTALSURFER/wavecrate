use super::{
    FolderBrowserDropTarget, FolderBrowserState, FolderEntry, FolderVerifyRequest, VisibleFolder,
    path_helpers::path_id,
};

impl FolderBrowserState {
    pub(super) fn selected_folder(&self) -> Option<&FolderEntry> {
        if self.selected_collection.is_some() {
            return None;
        }
        self.find_folder(&self.selected_folder)
            .or_else(|| self.folders.first())
    }

    pub(in crate::gui_app) fn selected_folder_path(&self) -> Option<std::path::PathBuf> {
        self.selected_folder()
            .map(|folder| std::path::PathBuf::from(&folder.id))
    }

    pub(in crate::gui_app) fn selected_folder_verify_request(&self) -> Option<FolderVerifyRequest> {
        let folder = self.selected_folder()?;
        Some(FolderVerifyRequest {
            source_id: self.selected_source.clone(),
            folder_path: std::path::PathBuf::from(&folder.id),
            cached_child_ids: folder
                .children
                .iter()
                .map(|child| child.id.clone())
                .collect(),
            cached_file_signatures: folder
                .files
                .iter()
                .map(|file| (file.id.clone(), file.size_bytes))
                .collect(),
        })
    }

    pub(super) fn find_folder(&self, id: &str) -> Option<&FolderEntry> {
        self.folders.iter().find_map(|folder| folder.find(id))
    }

    pub(super) fn folder_has_children(&self, id: &str) -> bool {
        self.find_folder(id).is_some_and(FolderEntry::has_children)
    }

    pub(super) fn is_expanded(&self, id: &str) -> bool {
        self.expanded_folders.contains(id)
    }

    pub(super) fn activate_folder(&mut self, id: String) {
        if self.selected_folder_is_source_root_id(&id) {
            self.select_folder(id);
            return;
        }
        if !self.folder_has_children(&id) {
            self.select_folder(id);
            return;
        }
        if !self.is_expanded(&id) {
            self.expanded_folders.insert(id.clone());
            self.select_folder(id);
        } else if self.selected_folder == id {
            self.expanded_folders.remove(&id);
        } else {
            self.select_folder(id);
        }
    }

    pub(super) fn toggle_folder_expansion(&mut self, id: String) {
        if self.selected_folder_is_source_root_id(&id) || !self.folder_has_children(&id) {
            return;
        }
        if !self.expanded_folders.remove(&id) {
            self.expanded_folders.insert(id);
        }
    }

    pub(super) fn select_folder(&mut self, id: String) {
        self.cancel_rename();
        self.selected_collection = None;
        self.collection_rename_edit = None;
        self.selected_folder = id;
        self.selected_file = None;
        self.selected_file_ids.clear();
        self.reset_file_view();
    }

    pub(super) fn selected_folder_is_source_root(&self) -> bool {
        self.sources.iter().any(|source| {
            source.id == self.selected_source && path_id(&source.root) == self.selected_folder
        })
    }

    pub(super) fn selected_folder_is_source_root_id(&self, folder_id: &str) -> bool {
        self.sources
            .iter()
            .any(|source| source.id == self.selected_source && path_id(&source.root) == folder_id)
    }

    pub(super) fn visible_folders(&self) -> Vec<VisibleFolder> {
        let mut folders = Vec::new();
        for folder in &self.folders {
            self.push_visible_folder(folder, 0, &mut folders);
        }
        folders
    }

    #[cfg(test)]
    pub(in crate::gui_app) fn first_visible_child_folder_expansion_for_tests(
        &self,
    ) -> Option<(String, bool)> {
        self.visible_folders()
            .into_iter()
            .find(|folder| folder.has_children && !folder.selected)
            .map(|folder| (folder.id, folder.expanded))
    }

    #[cfg(test)]
    pub(in crate::gui_app) fn folder_expansion_for_tests(&self, folder_id: &str) -> Option<bool> {
        self.visible_folders()
            .into_iter()
            .find(|folder| folder.id == folder_id)
            .map(|folder| folder.expanded)
    }

    fn push_visible_folder(
        &self,
        folder: &FolderEntry,
        depth: usize,
        folders: &mut Vec<VisibleFolder>,
    ) {
        let is_source_root = self.selected_folder_is_source_root_id(&folder.id);
        let drag_active = self.drag.is_some();
        let drop_target_active = matches!(
            self.drop_target.current(),
            Some(FolderBrowserDropTarget::Folder(_))
        );
        let drag_source = matches!(
            self.drag.as_ref(),
            Some(super::FolderBrowserDrag::Folder { folder_id }) if folder_id == &folder.id
        );
        let drop_candidate = drag_active && self.can_drop_drag_on_folder(&folder.id);
        folders.push(VisibleFolder {
            id: folder.id.clone(),
            name: if is_source_root {
                String::from(".")
            } else {
                folder.name.clone()
            },
            depth,
            is_source_root,
            has_children: folder.has_children(),
            expanded: is_source_root || self.is_expanded(&folder.id),
            selected: self.selected_collection.is_none() && self.selected_folder == folder.id,
            drag_active,
            drag_source,
            drop_candidate,
            drop_target: drop_candidate
                && self
                    .drop_target
                    .is_open(&FolderBrowserDropTarget::Folder(folder.id.clone())),
            drop_target_active,
            rename_draft: self
                .rename_edit
                .as_ref()
                .filter(|edit| edit.folder_id == folder.id)
                .map(|edit| edit.draft.clone()),
            rename_input_id: self
                .rename_edit
                .as_ref()
                .filter(|edit| edit.folder_id == folder.id)
                .map(|edit| edit.input_id),
        });
        if is_source_root || self.is_expanded(&folder.id) {
            for child in &folder.children {
                self.push_visible_folder(child, depth + 1, folders);
            }
        }
    }
}
