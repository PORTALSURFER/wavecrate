use std::collections::HashSet;
use std::path::Path;

use radiant::widgets::PointerModifiers;

use super::{
    file_selection_model::{FileSelectionModel, SelectionToggleAdvanceOutcome},
    folder_selection_model::FolderSelectionModel,
    path_helpers::rewrite_path_id,
};
use wavecrate::sample_sources::SampleCollection;

#[derive(Clone, Debug)]
pub(super) struct BrowserSelectionState {
    pub(super) selected_folder: String,
    pub(super) selected_folder_ids: HashSet<String>,
    pub(super) folder_selection_anchor: Option<String>,
    pub(super) selected_file: Option<String>,
    pub(super) selected_file_ids: HashSet<String>,
    pub(super) selected_file_ids_explicit: bool,
    pub(super) selected_collection: Option<SampleCollection>,
    pub(super) folder_before_collection: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BrowserSelectionSnapshot {
    pub(super) selected_folder: String,
    pub(super) selected_file: Option<String>,
    pub(super) selected_file_ids: HashSet<String>,
    pub(super) selected_file_ids_explicit: bool,
}

impl BrowserSelectionState {
    pub(super) fn new(selected_folder: String) -> Self {
        Self {
            selected_folder: selected_folder.clone(),
            selected_folder_ids: [selected_folder].into_iter().collect(),
            folder_selection_anchor: None,
            selected_file: None,
            selected_file_ids: HashSet::new(),
            selected_file_ids_explicit: false,
            selected_collection: None,
            folder_before_collection: None,
        }
    }

    pub(super) fn clear_file_selection(&mut self) {
        self.selected_file = None;
        self.selected_file_ids.clear();
        self.selected_file_ids_explicit = false;
    }

    pub(super) fn select_folder(&mut self, folder_id: String) {
        self.selected_collection = None;
        self.folder_before_collection = None;
        self.select_single_folder(folder_id);
        self.clear_file_selection();
    }

    pub(super) fn select_folder_with_modifiers(
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

    pub(super) fn navigate_folder(
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

    pub(super) fn toggle_focused_folder(&mut self, visible_ids: &[String]) -> Option<bool> {
        let mut selection = self.folder_selection_model();
        let selected = selection.toggle_focused(visible_ids)?;
        self.apply_folder_selection_model(selection);
        self.clear_file_selection();
        Some(selected)
    }

    pub(super) fn selected_folder_ids_contains(&self, folder_id: &str) -> bool {
        self.selected_folder_ids.contains(folder_id)
    }

    pub(super) fn selected_folder_count(&self) -> usize {
        self.selected_folder_ids.len()
    }

    pub(super) fn retain_existing_folders(
        &mut self,
        existing_ids: &HashSet<String>,
        fallback_id: String,
    ) {
        let mut selection = self.folder_selection_model();
        selection.retain_existing(existing_ids, fallback_id);
        self.apply_folder_selection_model(selection);
    }

    pub(super) fn discard_folder(&mut self, folder_id: &str, fallback_id: String) {
        let mut selection = self.folder_selection_model();
        selection.remove_id(folder_id, fallback_id);
        self.apply_folder_selection_model(selection);
    }

    pub(super) fn enter_collection(&mut self, collection: SampleCollection) {
        if self.selected_collection.is_none() {
            self.folder_before_collection = Some(self.selected_folder.clone());
        }
        self.selected_collection = Some(collection);
        self.clear_file_selection();
    }

    pub(super) fn exit_collection(&mut self, restored_folder: Option<String>) -> bool {
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

    pub(super) fn active_file_ids(&self) -> HashSet<String> {
        self.file_selection_model().active_ids()
    }

    pub(super) fn selected_file_count(&self) -> usize {
        self.selected_file_ids.len()
    }

    pub(super) fn selected_folder_id(&self) -> &str {
        &self.selected_folder
    }

    pub(super) fn selected_file_id(&self) -> Option<&str> {
        self.selected_file.as_deref()
    }

    pub(super) fn selected_file_ids_contains(&self, file_id: &str) -> bool {
        self.file_selection_model().contains_selected_id(file_id)
    }

    pub(super) fn selected_collection_active_without_file_focus(&self) -> bool {
        self.selected_collection.is_some() && self.selected_file.is_none()
    }

    pub(super) fn selected_file_active(&self) -> bool {
        self.selected_file.is_some()
    }

    pub(super) fn snapshot(&self) -> BrowserSelectionSnapshot {
        BrowserSelectionSnapshot {
            selected_folder: self.selected_folder.clone(),
            selected_file: self.selected_file.clone(),
            selected_file_ids: self.selected_file_ids.clone(),
            selected_file_ids_explicit: self.selected_file_ids_explicit,
        }
    }

    pub(super) fn restore_snapshot(&mut self, snapshot: BrowserSelectionSnapshot) {
        self.selected_folder = snapshot.selected_folder;
        self.selected_file = snapshot.selected_file;
        self.selected_file_ids = snapshot.selected_file_ids;
        self.selected_file_ids_explicit = snapshot.selected_file_ids_explicit;
    }

    pub(super) fn select_single_file(&mut self, id: String, visible_ids: &[String]) -> bool {
        if !visible_ids.contains(&id) {
            return false;
        }
        let mut selection = self.file_selection_model();
        let selected = selection.select_single(id, visible_ids);
        self.apply_file_selection_model(selection);
        selected
    }

    pub(super) fn select_file_with_modifiers(
        &mut self,
        id: String,
        visible_ids: &[String],
        modifiers: PointerModifiers,
    ) -> bool {
        if !visible_ids.contains(&id) {
            return false;
        }
        let mut selection = self.file_selection_model();
        let selected = selection.select_with_modifiers(id, visible_ids, modifiers);
        self.apply_file_selection_model(selection);
        selected
    }

    pub(super) fn focus_file_preserving_selection(
        &mut self,
        id: String,
        visible_ids: &[String],
    ) -> bool {
        let mut selection = self.file_selection_model();
        let focused = selection.focus_preserving_selection(id, visible_ids);
        self.apply_file_selection_model(selection);
        focused
    }

    pub(super) fn select_all_files(&mut self, ids: Vec<String>) -> usize {
        let mut selection = self.file_selection_model();
        let count = selection.select_all(ids);
        self.apply_file_selection_model(selection);
        count
    }

    pub(super) fn navigate_file(
        &mut self,
        delta: i32,
        extend: bool,
        visible_ids: &[String],
    ) -> Option<String> {
        let mut selection = self.file_selection_model();
        let target = selection.navigate(delta, extend, visible_ids)?;
        self.apply_file_selection_model(selection);
        Some(target)
    }

    pub(super) fn toggle_focused_file_and_advance(
        &mut self,
        visible_ids: &[String],
    ) -> Option<SelectionToggleAdvanceOutcome> {
        let mut selection = self.file_selection_model();
        let outcome = selection.toggle_focused_and_advance(visible_ids)?;
        self.apply_file_selection_model(selection);
        Some(outcome)
    }

    pub(super) fn retain_visible_files(&mut self, visible_ids: &HashSet<String>) {
        let mut selection = self.file_selection_model();
        selection.retain_visible(visible_ids);
        self.apply_file_selection_model(selection);
    }

    pub(super) fn select_folder_after_tree_changed(&mut self, folder_id: String) {
        self.select_single_folder(folder_id);
        self.clear_file_selection();
    }

    pub(super) fn set_focus_file_set(&mut self, file_id: String) {
        self.selected_file = Some(file_id.clone());
        self.selected_file_ids.clear();
        self.selected_file_ids.insert(file_id);
        self.selected_file_ids_explicit = false;
    }

    pub(super) fn set_folder_focus(&mut self, folder_id: String) {
        self.selected_folder = folder_id;
        self.selected_folder_ids
            .insert(self.selected_folder.clone());
        self.folder_selection_anchor
            .get_or_insert_with(|| self.selected_folder.clone());
    }

    pub(super) fn rewrite_folder_prefix(&mut self, old_path: &Path, new_path: &Path) {
        self.selected_folder = rewrite_path_id(&self.selected_folder, old_path, new_path);
        self.selected_folder_ids = self
            .selected_folder_ids
            .iter()
            .map(|id| rewrite_path_id(id, old_path, new_path))
            .collect();
        self.folder_selection_anchor = self
            .folder_selection_anchor
            .take()
            .map(|id| rewrite_path_id(&id, old_path, new_path));
        self.selected_file = self
            .selected_file
            .take()
            .map(|id| rewrite_path_id(&id, old_path, new_path));
        self.selected_file_ids = self
            .selected_file_ids
            .iter()
            .map(|id| rewrite_path_id(id, old_path, new_path))
            .collect();
    }

    pub(super) fn set_renamed_file(&mut self, new_id: String) {
        self.set_focus_file_set(new_id);
    }

    pub(super) fn restore_after_moved_files(
        &mut self,
        snapshot: BrowserSelectionSnapshot,
        moved_old_ids: &HashSet<String>,
        visible_ids: &[String],
    ) {
        let visible_id_set = visible_ids.iter().cloned().collect::<HashSet<_>>();
        self.selected_folder = snapshot.selected_folder;
        self.selected_file_ids = snapshot
            .selected_file_ids
            .into_iter()
            .filter(|id| !moved_old_ids.contains(id) && visible_id_set.contains(id))
            .collect();
        self.selected_file_ids_explicit = snapshot.selected_file_ids_explicit;
        self.selected_file = snapshot
            .selected_file
            .filter(|id| !moved_old_ids.contains(id) && visible_id_set.contains(id))
            .or_else(|| {
                visible_ids
                    .iter()
                    .find(|id| self.selected_file_ids.contains(*id))
                    .cloned()
            });

        if self.selected_file.is_none()
            && self.selected_file_ids.is_empty()
            && let Some(first_visible) = visible_ids.first().cloned()
        {
            self.set_focus_file_set(first_visible);
        }
    }

    pub(super) fn select_moved_files(
        &mut self,
        target_folder_id: String,
        moved_file_ids: &[(String, String)],
    ) {
        let selected_file_was_moved = self
            .selected_file
            .as_ref()
            .is_some_and(|id| moved_file_ids.iter().any(|(old_id, _)| old_id == id));
        self.selected_file = if selected_file_was_moved {
            self.selected_file.take().map(|id| {
                moved_file_ids
                    .iter()
                    .find(|(old_id, _)| old_id == &id)
                    .map(|(_, new_id)| new_id.clone())
                    .unwrap_or(id)
            })
        } else {
            moved_file_ids.first().map(|(_, new_id)| new_id.clone())
        };
        self.selected_file_ids = if self
            .selected_file_ids
            .iter()
            .any(|id| moved_file_ids.iter().any(|(old_id, _)| old_id == id))
        {
            self.selected_file_ids
                .iter()
                .map(|id| {
                    moved_file_ids
                        .iter()
                        .find(|(old_id, _)| old_id == id)
                        .map(|(_, new_id)| new_id.clone())
                        .unwrap_or_else(|| id.clone())
                })
                .collect()
        } else {
            moved_file_ids
                .iter()
                .map(|(_, new_id)| new_id.clone())
                .collect()
        };
        self.selected_folder = target_folder_id;
        self.selected_file_ids_explicit = self.selected_file_ids.len() > 1;
    }

    pub(super) fn discard_files(&mut self, removed_ids: &HashSet<String>) {
        if self
            .selected_file
            .as_ref()
            .is_some_and(|id| removed_ids.contains(id))
        {
            self.selected_file = None;
        }
        self.selected_file_ids
            .retain(|id| !removed_ids.contains(id));
        if self.selected_file.is_none() && self.selected_file_ids.is_empty() {
            self.selected_file_ids_explicit = false;
        }
    }

    fn file_selection_model(&self) -> FileSelectionModel {
        FileSelectionModel::new(
            self.selected_file.clone(),
            self.selected_file_ids.clone(),
            self.selected_file_ids_explicit,
        )
    }

    fn apply_file_selection_model(&mut self, selection: FileSelectionModel) {
        self.selected_file = selection.focused_id().map(ToOwned::to_owned);
        self.selected_file_ids = selection.selected_ids().clone();
        self.selected_file_ids_explicit = selection.explicit();
    }

    fn select_single_folder(&mut self, folder_id: String) {
        self.selected_folder = folder_id.clone();
        self.selected_folder_ids.clear();
        self.selected_folder_ids.insert(folder_id.clone());
        self.folder_selection_anchor = Some(folder_id);
    }

    fn folder_selection_model(&self) -> FolderSelectionModel {
        FolderSelectionModel::new(
            self.selected_folder.clone(),
            self.folder_selection_anchor.clone(),
            self.selected_folder_ids.clone(),
        )
    }

    fn apply_folder_selection_model(&mut self, selection: FolderSelectionModel) {
        self.selected_folder = selection.focused_id().to_owned();
        self.folder_selection_anchor = selection.anchor_id().map(ToOwned::to_owned);
        self.selected_folder_ids = selection.selected_ids().clone();
    }
}
