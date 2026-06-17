use super::BrowserSelectionState;
use crate::native_app::sample_library::folder_browser::folder_selection_model::FolderSelectionModel;
use radiant::widgets::PointerModifiers;
use std::collections::HashSet;

impl BrowserSelectionState {
    pub(in crate::native_app::sample_library::folder_browser) fn select_folder(
        &mut self,
        folder_id: String,
    ) {
        self.selected_collection = None;
        self.folder_before_collection = None;
        self.select_single_folder(folder_id);
        self.clear_file_selection();
    }

    pub(in crate::native_app::sample_library::folder_browser) fn select_folder_with_modifiers(
        &mut self,
        folder_id: String,
        visible_ids: &[String],
        modifiers: PointerModifiers,
    ) -> bool {
        self.selected_collection = None;
        self.folder_before_collection = None;
        let mut selection = self.folder_selection_model();
        let selected = selection.select_with_modifiers(folder_id, visible_ids, modifiers);
        self.apply_folder_selection_model(selection);
        self.clear_file_selection();
        selected
    }

    pub(in crate::native_app::sample_library::folder_browser) fn navigate_folder(
        &mut self,
        delta: i32,
        extend: bool,
        preserve_selection: bool,
        visible_ids: &[String],
    ) -> Option<String> {
        let mut selection = self.folder_selection_model();
        let target = selection.navigate(delta, extend, preserve_selection, visible_ids)?;
        self.apply_folder_selection_model(selection);
        self.clear_file_selection();
        Some(target)
    }

    pub(in crate::native_app::sample_library::folder_browser) fn toggle_focused_folder(
        &mut self,
        visible_ids: &[String],
    ) -> Option<bool> {
        let mut selection = self.folder_selection_model();
        let outcome = selection.toggle_focused_and_advance(visible_ids)?;
        let selected = outcome.toggled_selected;
        self.apply_folder_selection_model(selection);
        self.clear_file_selection();
        Some(selected)
    }

    pub(in crate::native_app::sample_library::folder_browser) fn selected_folder_ids_contains(
        &self,
        folder_id: &str,
    ) -> bool {
        self.selected_folder_ids.contains(folder_id)
    }

    pub(in crate::native_app::sample_library::folder_browser) fn selected_folder_count(
        &self,
    ) -> usize {
        self.selected_folder_ids.len()
    }

    pub(in crate::native_app::sample_library::folder_browser) fn selected_folder_id(&self) -> &str {
        &self.selected_folder
    }

    pub(in crate::native_app::sample_library::folder_browser) fn retain_existing_folders(
        &mut self,
        existing_ids: &HashSet<String>,
        fallback_id: String,
    ) {
        let mut selection = self.folder_selection_model();
        selection.retain_existing(existing_ids, fallback_id);
        self.apply_folder_selection_model(selection);
    }

    pub(in crate::native_app::sample_library::folder_browser) fn discard_folder(
        &mut self,
        folder_id: &str,
        fallback_id: String,
    ) {
        let mut selection = self.folder_selection_model();
        selection.remove_id(folder_id, fallback_id);
        self.apply_folder_selection_model(selection);
    }

    pub(in crate::native_app::sample_library::folder_browser) fn select_folder_after_tree_changed(
        &mut self,
        folder_id: String,
    ) {
        self.select_single_folder(folder_id);
        self.clear_file_selection();
    }

    pub(in crate::native_app::sample_library::folder_browser) fn set_folder_focus(
        &mut self,
        folder_id: String,
    ) {
        self.selected_folder = folder_id;
        self.selected_folder_ids
            .insert(self.selected_folder.clone());
        self.selected_folder_ids_explicit = false;
        self.folder_selection_anchor
            .get_or_insert_with(|| self.selected_folder.clone());
    }

    fn select_single_folder(&mut self, folder_id: String) {
        self.selected_folder = folder_id.clone();
        self.selected_folder_ids.clear();
        self.selected_folder_ids.insert(folder_id.clone());
        self.selected_folder_ids_explicit = false;
        self.folder_selection_anchor = Some(folder_id);
    }

    fn folder_selection_model(&self) -> FolderSelectionModel {
        FolderSelectionModel::new(
            self.selected_folder.clone(),
            self.folder_selection_anchor.clone(),
            self.selected_folder_ids.clone(),
            self.selected_folder_ids_explicit,
        )
    }

    fn apply_folder_selection_model(&mut self, selection: FolderSelectionModel) {
        self.selected_folder = selection.focused_id().to_owned();
        self.folder_selection_anchor = selection.anchor_id().map(ToOwned::to_owned);
        self.selected_folder_ids = selection.selected_ids().clone();
        self.selected_folder_ids_explicit = selection.explicit();
    }
}
