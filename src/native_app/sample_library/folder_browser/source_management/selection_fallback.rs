use super::super::{FolderBrowserState, FolderEntry, RemovedSource, scanning::placeholder_folder};

impl FolderBrowserState {
    pub(in crate::native_app) fn remove_source(
        &mut self,
        source_id: &str,
    ) -> Result<RemovedSource, String> {
        let index = self
            .source
            .sources
            .iter()
            .position(|source| source.id == source_id)
            .ok_or_else(|| String::from("Source is unavailable"))?;
        if self.source.sources[index].is_default_assets_source() {
            return Err(String::from("Default source cannot be removed"));
        }
        let source = self.source.sources.remove(index);
        let removed = RemovedSource {
            label: source.label.clone(),
            root: source.root.clone(),
        };
        self.cancel_rename();
        self.clear_drag();
        if self.source.selected_source == source.id {
            self.select_first_available_source();
        }
        Ok(removed)
    }

    pub(in crate::native_app::sample_library::folder_browser) fn select_pending_source(
        &mut self,
        id: String,
        folder: FolderEntry,
    ) {
        self.cancel_rename();
        self.selection.selected_collection = None;
        self.collection_panel.rename_edit = None;
        let root_id = folder.id.clone();
        self.source.selected_source = id;
        self.selection.select_folder(root_id.clone());
        self.reset_tree_view();
        self.reset_file_view();
        self.tree.expanded_folders.clear();
        self.tree.expanded_folders.insert(root_id);
        self.tree.folders = vec![folder];
        self.bump_file_content_revision();
    }

    pub(in crate::native_app::sample_library::folder_browser) fn select_loaded_source(
        &mut self,
        id: String,
        root_folder: FolderEntry,
    ) {
        self.cancel_rename();
        self.selection.selected_collection = None;
        self.collection_panel.rename_edit = None;
        let root_id = root_folder.id.clone();
        self.source.selected_source = id;
        self.selection.select_folder(root_id.clone());
        self.reset_tree_view();
        self.reset_file_view();
        self.tree.expanded_folders.clear();
        self.tree.expanded_folders.insert(root_id);
        self.tree.folders = vec![root_folder];
        self.bump_file_content_revision();
        self.prewarm_selected_source_audio_projection_cache();
    }

    fn select_first_available_source(&mut self) {
        let Some(source) = self.source.sources.first().cloned() else {
            *self = Self::empty();
            return;
        };
        if let Some(root_folder) = source.root_folder {
            self.select_loaded_source(source.id, root_folder);
        } else {
            self.select_pending_source(source.id, placeholder_folder(&source.root));
        }
    }
}
