use wavecrate::sample_sources::SampleCollection;

use super::super::FolderBrowserState;

impl FolderBrowserState {
    pub(in crate::native_app::sample_library::folder_browser) fn activate_collection(
        &mut self,
        collection: SampleCollection,
    ) {
        if self.selection.selected_collection != Some(collection) {
            self.collection_panel.rename_edit = None;
            self.selection.enter_collection(collection);
            self.reset_folder_focus_to_selected_source_root();
            self.reset_file_view();
        }
    }

    pub(in crate::native_app) fn collection_focus_active(&self) -> bool {
        self.selection.selected_collection.is_some()
    }

    pub(in crate::native_app) fn exit_collection_focus(&mut self) -> bool {
        let restored_folder = self
            .selection
            .folder_before_collection
            .as_deref()
            .filter(|folder_id| self.find_folder(folder_id).is_some())
            .map(str::to_owned);
        let restored = self.selection.exit_collection(restored_folder);
        if restored {
            self.collection_panel.rename_edit = None;
            self.reset_file_view();
        }
        restored
    }

    fn reset_folder_focus_to_selected_source_root(&mut self) {
        if let Some(root) = self.tree.folders.first() {
            self.selection.set_folder_focus(root.id.clone());
        }
    }
}
