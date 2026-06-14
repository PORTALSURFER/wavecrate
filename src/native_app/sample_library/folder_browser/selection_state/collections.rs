use super::BrowserSelectionState;
use wavecrate::sample_sources::SampleCollection;

impl BrowserSelectionState {
    pub(in crate::native_app::sample_library::folder_browser) fn enter_collection(
        &mut self,
        collection: SampleCollection,
    ) {
        if self.selected_collection.is_none() {
            self.folder_before_collection = Some(self.selected_folder.clone());
        }
        self.selected_collection = Some(collection);
        self.clear_file_selection();
    }

    pub(in crate::native_app::sample_library::folder_browser) fn exit_collection(
        &mut self,
        restored_folder: Option<String>,
    ) -> bool {
        if self.selected_collection.take().is_none() {
            self.folder_before_collection = None;
            return false;
        }
        if let Some(folder) = restored_folder {
            self.selected_folder = folder;
        }
        self.folder_before_collection = None;
        self.clear_file_selection();
        true
    }

    pub(in crate::native_app::sample_library::folder_browser) fn selected_collection_active_without_file_focus(
        &self,
    ) -> bool {
        self.selected_collection.is_some() && self.selected_file.is_none()
    }
}
