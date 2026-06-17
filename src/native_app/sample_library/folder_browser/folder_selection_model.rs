use std::collections::HashSet;

use radiant::{prelude as ui, widgets::PointerModifiers};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FolderSelectionModel {
    focused_id: String,
    anchor_id: Option<String>,
    selected_ids: HashSet<String>,
    explicit: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FolderSelectionToggleAdvanceOutcome {
    pub(super) toggled_id: String,
    pub(super) toggled_selected: bool,
    pub(super) focused_id: String,
}

impl FolderSelectionModel {
    pub(super) fn new(
        focused_id: String,
        anchor_id: Option<String>,
        selected_ids: HashSet<String>,
        explicit: bool,
    ) -> Self {
        let selected_ids = if !explicit && selected_ids.is_empty() {
            [focused_id.clone()].into_iter().collect()
        } else {
            selected_ids
        };
        Self {
            focused_id,
            anchor_id,
            selected_ids,
            explicit,
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

    pub(super) fn explicit(&self) -> bool {
        self.explicit
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
        self.explicit = modifiers.shift || modifiers.command;
        true
    }

    pub(super) fn navigate(
        &mut self,
        delta: i32,
        extend: bool,
        preserve_selection: bool,
        visible_ids: &[String],
    ) -> Option<String> {
        let was_explicit = self.explicit;
        let mut selection = self.keyed_selection();
        let target = if preserve_selection {
            let target = navigate_focus_only(selection.focused_key()?, delta, visible_ids)?;
            selection.focus(target.clone(), visible_ids);
            target
        } else if self.explicit && !extend {
            let target = navigate_focus_only(selection.focused_key()?, delta, visible_ids)?;
            selection.focus(target.clone(), visible_ids);
            target
        } else if extend {
            selection.navigate(delta as isize, visible_ids, true)?
        } else {
            selection.navigate(delta as isize, visible_ids, false)?
        };
        self.apply_keyed_selection(selection);
        self.explicit = if preserve_selection {
            was_explicit
        } else {
            extend || (was_explicit && !extend)
        };
        Some(target)
    }

    pub(super) fn toggle_focused_and_advance(
        &mut self,
        visible_ids: &[String],
    ) -> Option<FolderSelectionToggleAdvanceOutcome> {
        let current_index = visible_ids.iter().position(|id| id == &self.focused_id)?;
        let toggled_id = self.focused_id.clone();
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
        self.focused_id = focused_id.clone();
        Some(FolderSelectionToggleAdvanceOutcome {
            toggled_id,
            toggled_selected,
            focused_id,
        })
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
        if !self.explicit && self.selected_ids.is_empty() {
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
        if !self.explicit && self.selected_ids.is_empty() {
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
        self.explicit = false;
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
        let mut selection =
            FolderSelectionModel::new(String::from("root"), None, set(&["root"]), false);

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
        let mut selection =
            FolderSelectionModel::new(String::from("root"), None, set(&["root"]), false);
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
            true,
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
            true,
        );

        let outcome = selection.toggle_focused_and_advance(&visible_ids);

        assert_eq!(
            outcome,
            Some(FolderSelectionToggleAdvanceOutcome {
                toggled_id: String::from("loops"),
                toggled_selected: false,
                focused_id: String::from("loops"),
            })
        );
        assert_eq!(selection.selected_ids(), &set(&["drums"]));
    }

    #[test]
    fn x_toggle_marks_focused_folder_and_advances_from_implicit_selection() {
        let visible_ids = ids(&["root", "drums", "loops"]);
        let mut selection =
            FolderSelectionModel::new(String::from("root"), None, set(&["root"]), false);

        let outcome = selection.toggle_focused_and_advance(&visible_ids);

        assert_eq!(
            outcome,
            Some(FolderSelectionToggleAdvanceOutcome {
                toggled_id: String::from("root"),
                toggled_selected: true,
                focused_id: String::from("drums"),
            })
        );
        assert!(selection.explicit());
        assert_eq!(selection.focused_id(), "drums");
        assert_eq!(selection.selected_ids(), &set(&["root"]));
    }
}
