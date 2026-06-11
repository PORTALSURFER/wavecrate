use std::collections::HashSet;
use std::path::Path;

use radiant::{prelude as ui, widgets::PointerModifiers};

use super::{path_helpers::rewrite_path_id, state::BrowserSelectionState};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BrowserSelectionSnapshot {
    pub(super) selected_folder: String,
    pub(super) selected_file: Option<String>,
    pub(super) selected_file_ids: HashSet<String>,
    pub(super) selected_file_ids_explicit: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SelectionToggleAdvanceOutcome {
    pub(super) toggled_id: String,
    pub(super) toggled_selected: bool,
    pub(super) focused_id: String,
}

impl BrowserSelectionState {
    pub(super) fn active_file_ids(&self) -> HashSet<String> {
        if self.selected_file_ids_explicit || !self.selected_file_ids.is_empty() {
            return self.selected_file_ids.clone();
        }
        self.selected_file
            .as_deref()
            .map(|id| [id.to_string()].into_iter().collect())
            .unwrap_or_default()
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
        self.selected_file_ids.contains(file_id)
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
        let mut selection = ui::KeyedListSelection::new();
        selection.select_with_intent(id, visible_ids, ui::ListSelectionIntent::Replace);
        self.apply_file_selection_model(selection);
        true
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
        selection.select_with_intent(
            id,
            visible_ids,
            ui::ListSelectionIntent::from_extend_toggle(modifiers.shift, modifiers.command),
        );
        self.apply_file_selection_model(selection);
        self.selected_file_ids_explicit = modifiers.shift || modifiers.command;
        true
    }

    pub(super) fn focus_file_preserving_selection(
        &mut self,
        id: String,
        visible_ids: &[String],
    ) -> bool {
        if self.selected_file_ids.contains(&id) && visible_ids.contains(&id) {
            self.selected_file = Some(id);
            return true;
        }
        self.select_single_file(id, visible_ids)
    }

    pub(super) fn select_all_files(&mut self, ids: Vec<String>) -> usize {
        let mut selection = self.file_selection_model();
        selection.select_all(&ids);
        self.apply_file_selection_model(selection);
        self.selected_file_ids_explicit = true;
        self.selected_file_ids.len()
    }

    pub(super) fn navigate_file(
        &mut self,
        delta: i32,
        extend: bool,
        visible_ids: &[String],
    ) -> Option<String> {
        if self.selected_file_ids_explicit && !extend {
            return self.navigate_focused_file_preserving_selection(delta, visible_ids);
        }

        let mut selection = self.file_selection_model();
        let target = if extend {
            selection.navigate_preserving_existing(delta as isize, visible_ids)?
        } else {
            selection.navigate(delta as isize, visible_ids, false)?
        };
        self.apply_file_selection_model(selection);
        self.selected_file_ids_explicit = extend;
        Some(target)
    }

    pub(super) fn toggle_focused_file_and_advance(
        &mut self,
        visible_ids: &[String],
    ) -> Option<SelectionToggleAdvanceOutcome> {
        let focused = self.selected_file.as_ref()?;
        let current_index = visible_ids.iter().position(|id| id == focused)?;
        let toggled_id = focused.clone();
        let already_marked =
            self.selected_file_ids_explicit && self.selected_file_ids.contains(&toggled_id);
        let toggled_selected = if already_marked {
            self.selected_file_ids.remove(&toggled_id);
            false
        } else {
            self.selected_file_ids.insert(toggled_id.clone());
            true
        };
        self.selected_file_ids_explicit = true;
        let focused_id =
            visible_ids[current_index.saturating_add(1).min(visible_ids.len() - 1)].clone();
        self.selected_file = Some(focused_id.clone());
        Some(SelectionToggleAdvanceOutcome {
            toggled_id,
            toggled_selected,
            focused_id,
        })
    }

    pub(super) fn retain_visible_files(&mut self, visible_ids: &HashSet<String>) {
        self.selected_file_ids.retain(|id| visible_ids.contains(id));
        if self
            .selected_file
            .as_ref()
            .is_some_and(|id| !visible_ids.contains(id))
        {
            self.selected_file = None;
        }
        if self.selected_file.is_none() && self.selected_file_ids.is_empty() {
            self.selected_file_ids_explicit = false;
        }
    }

    pub(super) fn select_folder_after_tree_changed(&mut self, folder_id: String) {
        self.selected_folder = folder_id;
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
    }

    pub(super) fn select_folder_and_clear_files(&mut self, folder_id: String) {
        self.selected_folder = folder_id;
        self.clear_file_selection();
    }

    pub(super) fn rewrite_folder_prefix(&mut self, old_path: &Path, new_path: &Path) {
        self.selected_folder = rewrite_path_id(&self.selected_folder, old_path, new_path);
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

    fn navigate_focused_file_preserving_selection(
        &mut self,
        delta: i32,
        visible_ids: &[String],
    ) -> Option<String> {
        let current = self.selected_file.as_ref()?;
        let current_index = visible_ids.iter().position(|id| id == current)?;
        let target_index =
            ui::list_index_after_delta(current_index, delta as isize, visible_ids.len())?;
        if target_index == current_index {
            return None;
        }
        let target = visible_ids[target_index].clone();
        self.selected_file = Some(target.clone());
        Some(target)
    }

    fn file_selection_model(&self) -> ui::KeyedListSelection<String> {
        ui::KeyedListSelection::from_parts(
            self.selected_file.clone(),
            self.selected_file.clone(),
            self.selected_file_ids.clone(),
        )
    }

    fn apply_file_selection_model(&mut self, selection: ui::KeyedListSelection<String>) {
        self.selected_file = selection.focused_key().cloned();
        self.selected_file_ids = selection.selected_keys().iter().cloned().collect();
        self.selected_file_ids_explicit = false;
    }
}
