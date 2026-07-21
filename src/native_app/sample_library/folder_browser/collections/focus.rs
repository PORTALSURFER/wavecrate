use wavecrate::sample_sources::SampleCollection;

use super::super::FolderBrowserState;

impl FolderBrowserState {
    pub(in crate::native_app::sample_library::folder_browser) fn activate_collection(
        &mut self,
        collection: SampleCollection,
    ) {
        self.clear_source_keyboard_focus();
        if self.selection.selected_collection != Some(collection) {
            self.collection_panel.rename_edit = None;
            self.selection.enter_collection(collection);
            self.reset_folder_focus_to_selected_source_root();
            self.refresh_missing_collection_state();
            self.reset_file_view();
        }
    }

    pub(in crate::native_app) fn collection_focus_active(&self) -> bool {
        self.selection.selected_collection.is_some()
    }

    pub(in crate::native_app) fn selected_collection(&self) -> Option<SampleCollection> {
        self.selection.selected_collection
    }

    pub(in crate::native_app) fn collection_keyboard_focus_active(&self) -> bool {
        self.selection
            .selected_collection_active_without_file_focus()
    }

    pub(in crate::native_app) fn navigate_selected_collection(
        &mut self,
        delta: i32,
    ) -> Option<usize> {
        if delta == 0 || self.rename_active() || !self.collection_keyboard_focus_active() {
            return None;
        }
        let selected = self.selection.selected_collection?;
        let current_index = self
            .collection_panel
            .collections
            .iter()
            .position(|entry| entry.collection == selected)?;
        let target_index = collection_index_after_delta(
            current_index,
            delta,
            self.collection_panel.collections.len(),
        )?;
        if target_index == current_index {
            return None;
        }
        let target = self.collection_panel.collections[target_index].collection;
        self.activate_collection(target);
        Some(target_index)
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
            self.refresh_missing_collection_state();
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

fn collection_index_after_delta(current_index: usize, delta: i32, len: usize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let target = current_index as i32 + delta;
    Some(target.clamp(0, (len - 1) as i32) as usize)
}
