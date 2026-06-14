use std::collections::HashSet;

use radiant::{prelude as ui, widgets::PointerModifiers};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FolderSelectionModel {
    focused_id: String,
    anchor_id: Option<String>,
    selected_ids: HashSet<String>,
}

impl FolderSelectionModel {
    pub(super) fn new(
        focused_id: String,
        anchor_id: Option<String>,
        selected_ids: HashSet<String>,
    ) -> Self {
        let selected_ids = if selected_ids.is_empty() {
            [focused_id.clone()].into_iter().collect()
        } else {
            selected_ids
        };
        Self {
            focused_id,
            anchor_id,
            selected_ids,
        }
    }

    pub(super) fn focused_id(&self) -> &str {
        &self.focused_id
    }

    pub(super) fn anchor_id(&self) -> Option<&str> {
        self.anchor_id.as_deref()
    }

    pub(super) fn selected_ids(&self) -> &HashSet<String> {
        &self.selected_ids
    }

    #[cfg(test)]
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
        true
    }

    pub(super) fn navigate(
        &mut self,
        delta: i32,
        extend: bool,
        preserve_selection: bool,
        visible_ids: &[String],
    ) -> Option<String> {
        let mut selection = self.keyed_selection();
        let target = if preserve_selection {
            let target = navigate_focus_only(selection.focused_key()?, delta, visible_ids)?;
            selection.focus(target.clone(), visible_ids);
            target
        } else if extend {
            selection.navigate(delta as isize, visible_ids, true)?
        } else {
            selection.navigate(delta as isize, visible_ids, false)?
        };
        self.apply_keyed_selection(selection);
        Some(target)
    }

    pub(super) fn toggle_focused(&mut self, visible_ids: &[String]) -> Option<bool> {
        if !visible_ids.contains(&self.focused_id) {
            return None;
        }
        let mut selection = self.keyed_selection();
        selection.select_with_intent(
            self.focused_id.clone(),
            visible_ids,
            ui::ListSelectionIntent::Toggle,
        );
        let selected = selection.is_selected(&self.focused_id);
        self.apply_keyed_selection(selection);
        Some(selected)
    }

    pub(super) fn retain_existing(&mut self, existing_ids: &HashSet<String>, fallback_id: String) {
        self.selected_ids.retain(|id| existing_ids.contains(id));
        if !existing_ids.contains(&self.focused_id) {
            self.focused_id = fallback_id;
        }
        if self
            .anchor_id
            .as_ref()
            .is_some_and(|id| !existing_ids.contains(id))
        {
            self.anchor_id = Some(self.focused_id.clone());
        }
        if self.selected_ids.is_empty() {
            self.selected_ids.insert(self.focused_id.clone());
        }
    }

    pub(super) fn remove_id(&mut self, id: &str, fallback_id: String) {
        self.selected_ids.remove(id);
        if self.focused_id == id {
            self.focused_id = fallback_id;
        }
        if self.anchor_id.as_deref() == Some(id) {
            self.anchor_id = Some(self.focused_id.clone());
        }
        if self.selected_ids.is_empty() {
            self.selected_ids.insert(self.focused_id.clone());
        }
    }

    fn keyed_selection(&self) -> ui::KeyedListSelection<String> {
        ui::KeyedListSelection::from_parts(
            Some(self.focused_id.clone()),
            self.anchor_id
                .clone()
                .or_else(|| Some(self.focused_id.clone())),
            self.selected_ids.clone(),
        )
    }

    fn apply_keyed_selection(&mut self, selection: ui::KeyedListSelection<String>) {
        if let Some(focused) = selection.focused_key() {
            self.focused_id = focused.clone();
        }
        self.anchor_id = selection.anchor_key().cloned();
        self.selected_ids = selection.selected_keys().iter().cloned().collect();
        if self.selected_ids.is_empty() {
            self.selected_ids.insert(self.focused_id.clone());
        }
    }
}

fn navigate_focus_only(current: &String, delta: i32, visible_ids: &[String]) -> Option<String> {
    let current_index = visible_ids.iter().position(|id| id == current)?;
    let target_index =
        ui::list_index_after_delta(current_index, delta as isize, visible_ids.len())?;
    (target_index != current_index).then(|| visible_ids[target_index].clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn set(values: &[&str]) -> HashSet<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn command_click_toggles_membership_without_clearing_other_folders() {
        let visible_ids = ids(&["root", "drums", "loops"]);
        let mut selection = FolderSelectionModel::new(String::from("root"), None, set(&["root"]));

        assert!(selection.select_with_modifiers(
            String::from("loops"),
            &visible_ids,
            PointerModifiers {
                command: true,
                ..PointerModifiers::default()
            },
        ));

        assert_eq!(selection.focused_id(), "loops");
        assert_eq!(selection.selected_ids(), &set(&["root", "loops"]));
    }

    #[test]
    fn shift_click_extends_from_anchor() {
        let visible_ids = ids(&["root", "drums", "kicks", "snares"]);
        let mut selection = FolderSelectionModel::new(String::from("root"), None, set(&["root"]));
        assert!(selection.select_single(String::from("drums"), &visible_ids));

        assert!(selection.select_with_modifiers(
            String::from("snares"),
            &visible_ids,
            PointerModifiers {
                shift: true,
                ..PointerModifiers::default()
            },
        ));

        assert_eq!(selection.anchor_id(), Some("drums"));
        assert_eq!(
            selection.selected_ids(),
            &set(&["drums", "kicks", "snares"])
        );
    }

    #[test]
    fn preserve_navigation_moves_focus_without_changing_selection() {
        let visible_ids = ids(&["root", "drums", "loops"]);
        let mut selection = FolderSelectionModel::new(
            String::from("drums"),
            Some(String::from("drums")),
            set(&["drums", "loops"]),
        );

        let target = selection.navigate(-1, false, true, &visible_ids);

        assert_eq!(target.as_deref(), Some("root"));
        assert_eq!(selection.focused_id(), "root");
        assert_eq!(selection.selected_ids(), &set(&["drums", "loops"]));
    }

    #[test]
    fn x_toggle_removes_focused_folder_when_multiple_folders_are_selected() {
        let visible_ids = ids(&["root", "drums", "loops"]);
        let mut selection = FolderSelectionModel::new(
            String::from("loops"),
            Some(String::from("drums")),
            set(&["drums", "loops"]),
        );

        let selected = selection.toggle_focused(&visible_ids);

        assert_eq!(selected, Some(false));
        assert_eq!(selection.selected_ids(), &set(&["drums"]));
    }
}
