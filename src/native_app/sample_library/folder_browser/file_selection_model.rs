use std::collections::HashSet;

use radiant::{prelude as ui, widgets::PointerModifiers};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FileSelectionModel {
    focused_id: Option<String>,
    selected_ids: HashSet<String>,
    explicit: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SelectionToggleAdvanceOutcome {
    pub(super) toggled_id: String,
    pub(super) toggled_selected: bool,
    pub(super) focused_id: String,
}

impl FileSelectionModel {
    pub(super) fn new(
        focused_id: Option<String>,
        selected_ids: HashSet<String>,
        explicit: bool,
    ) -> Self {
        Self {
            focused_id,
            selected_ids,
            explicit,
        }
    }

    pub(super) fn focused_id(&self) -> Option<&str> {
        self.focused_id.as_deref()
    }

    pub(super) fn selected_ids(&self) -> &HashSet<String> {
        &self.selected_ids
    }

    pub(super) fn explicit(&self) -> bool {
        self.explicit
    }

    pub(super) fn active_ids(&self) -> HashSet<String> {
        if self.explicit || !self.selected_ids.is_empty() {
            return self.selected_ids.clone();
        }
        self.focused_id
            .as_deref()
            .map(|id| [id.to_string()].into_iter().collect())
            .unwrap_or_default()
    }

    pub(super) fn contains_selected_id(&self, file_id: &str) -> bool {
        self.selected_ids.contains(file_id)
    }

    pub(super) fn select_single(&mut self, id: String, visible_ids: &[String]) -> bool {
        if !visible_ids.contains(&id) {
            return false;
        }
        let mut selection = ui::KeyedListSelection::new();
        selection.select_with_intent(id, visible_ids, ui::ListSelectionIntent::Replace);
        self.apply_keyed_selection(selection);
        true
    }

    pub(super) fn select_with_modifiers(
        &mut self,
        id: String,
        visible_ids: &[String],
        modifiers: PointerModifiers,
    ) -> bool {
        if !visible_ids.contains(&id) {
            return false;
        }
        let mut selection = self.keyed_selection();
        selection.select_with_intent(
            id,
            visible_ids,
            ui::ListSelectionIntent::from_extend_toggle(modifiers.shift, modifiers.command),
        );
        self.apply_keyed_selection(selection);
        self.explicit = modifiers.shift || modifiers.command;
        true
    }

    pub(super) fn focus_preserving_selection(
        &mut self,
        id: String,
        visible_ids: &[String],
    ) -> bool {
        if self.selected_ids.contains(&id) && visible_ids.contains(&id) {
            self.focused_id = Some(id);
            return true;
        }
        self.select_single(id, visible_ids)
    }

    pub(super) fn select_all(&mut self, ids: Vec<String>) -> usize {
        let mut selection = self.keyed_selection();
        selection.select_all(&ids);
        self.apply_keyed_selection(selection);
        self.explicit = true;
        self.selected_ids.len()
    }

    pub(super) fn navigate(
        &mut self,
        delta: i32,
        extend: bool,
        visible_ids: &[String],
    ) -> Option<String> {
        if self.explicit && !extend {
            return self.navigate_focus_preserving_selection(delta, visible_ids);
        }

        let mut selection = self.keyed_selection();
        let target = if extend {
            selection.navigate_preserving_existing(delta as isize, visible_ids)?
        } else {
            selection.navigate(delta as isize, visible_ids, false)?
        };
        self.apply_keyed_selection(selection);
        self.explicit = extend;
        Some(target)
    }

    pub(super) fn toggle_focused_and_advance(
        &mut self,
        visible_ids: &[String],
    ) -> Option<SelectionToggleAdvanceOutcome> {
        let focused = self.focused_id.as_ref()?;
        let current_index = visible_ids.iter().position(|id| id == focused)?;
        let toggled_id = focused.clone();
        let already_marked = self.explicit && self.selected_ids.contains(&toggled_id);
        let toggled_selected = if already_marked {
            self.selected_ids.remove(&toggled_id);
            false
        } else {
            self.selected_ids.insert(toggled_id.clone());
            true
        };
        self.explicit = true;
        let focused_id =
            visible_ids[current_index.saturating_add(1).min(visible_ids.len() - 1)].clone();
        self.focused_id = Some(focused_id.clone());
        Some(SelectionToggleAdvanceOutcome {
            toggled_id,
            toggled_selected,
            focused_id,
        })
    }

    pub(super) fn retain_visible(&mut self, visible_ids: &HashSet<String>) {
        self.selected_ids.retain(|id| visible_ids.contains(id));
        if self
            .focused_id
            .as_ref()
            .is_some_and(|id| !visible_ids.contains(id))
        {
            self.focused_id = None;
        }
        if self.focused_id.is_none() && self.selected_ids.is_empty() {
            self.explicit = false;
        }
    }

    fn navigate_focus_preserving_selection(
        &mut self,
        delta: i32,
        visible_ids: &[String],
    ) -> Option<String> {
        let current = self.focused_id.as_ref()?;
        let current_index = visible_ids.iter().position(|id| id == current)?;
        let target_index =
            ui::list_index_after_delta(current_index, delta as isize, visible_ids.len())?;
        if target_index == current_index {
            return None;
        }
        let target = visible_ids[target_index].clone();
        self.focused_id = Some(target.clone());
        Some(target)
    }

    fn keyed_selection(&self) -> ui::KeyedListSelection<String> {
        ui::KeyedListSelection::from_parts(
            self.focused_id.clone(),
            self.focused_id.clone(),
            self.selected_ids.clone(),
        )
    }

    fn apply_keyed_selection(&mut self, selection: ui::KeyedListSelection<String>) {
        self.focused_id = selection.focused_key().cloned();
        self.selected_ids = selection.selected_keys().iter().cloned().collect();
        self.explicit = false;
    }
}
