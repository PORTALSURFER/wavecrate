//! Pure folder-selection mutation helpers shared by controller selection flows.

use super::super::*;
use std::collections::BTreeSet;

#[derive(Clone, Copy)]
pub(super) enum FolderSelectMode {
    Replace,
    Toggle,
}

pub(super) fn ancestors(path: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let mut current = path.parent();
    while let Some(parent) = current {
        if parent.as_os_str().is_empty() {
            break;
        }
        result.push(parent.to_path_buf());
        current = parent.parent();
    }
    result
}

pub(super) fn remove_descendants(selected: &mut BTreeSet<PathBuf>, path: &Path) {
    let descendants: Vec<PathBuf> = selected
        .iter()
        .filter(|candidate| candidate != &path && candidate.starts_with(path))
        .cloned()
        .collect();
    for descendant in descendants {
        selected.remove(&descendant);
    }
}

pub(super) fn insert_folder(selected: &mut BTreeSet<PathBuf>, path: &Path, has_children: bool) {
    selected.insert(path.to_path_buf());
    for ancestor in ancestors(path) {
        selected.remove(&ancestor);
    }
    if has_children {
        remove_descendants(selected, path);
    }
}

pub(super) fn apply_root_selection(model: &mut FolderBrowserModel, mode: FolderSelectMode) -> bool {
    let before = model.selected.clone();
    let root_path = PathBuf::new();
    match mode {
        FolderSelectMode::Replace => {
            if !model.selected.contains(&root_path) {
                model.selected.clear();
                model.selected.insert(root_path.clone());
                model.selection_anchor = Some(root_path.clone());
            }
        }
        FolderSelectMode::Toggle => {
            if model.selected.contains(&root_path) {
                model.selected.remove(&root_path);
                if model.selection_anchor.as_ref() == Some(&root_path) {
                    model.selection_anchor = None;
                }
            } else {
                model.selected.insert(root_path.clone());
                if model.selection_anchor.is_none() {
                    model.selection_anchor = Some(root_path.clone());
                }
            }
        }
    }
    if model.selected.is_empty() {
        model.selection_anchor = None;
    }
    let changed = before != model.selected;
    if changed {
        model.focused = Some(root_path);
    }
    changed
}

pub(super) fn apply_path_selection(
    model: &mut FolderBrowserModel,
    path: &Path,
    has_children: bool,
    mode: FolderSelectMode,
) -> bool {
    if !model.available.contains(path) {
        return false;
    }
    let before = model.selected.clone();
    match mode {
        FolderSelectMode::Replace => {
            model.selected.clear();
            insert_folder(&mut model.selected, path, has_children);
            model.selection_anchor = Some(path.to_path_buf());
        }
        FolderSelectMode::Toggle => {
            if model.selected.contains(path) {
                model.selected.remove(path);
                if model.selection_anchor.as_deref() == Some(path) {
                    model.selection_anchor = None;
                }
            } else {
                insert_folder(&mut model.selected, path, has_children);
                if model.selection_anchor.is_none() {
                    model.selection_anchor = Some(path.to_path_buf());
                }
            }
        }
    }
    if model.selected.is_empty() {
        model.selection_anchor = None;
    }
    let changed = before != model.selected;
    if changed {
        model.focused = Some(path.to_path_buf());
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_with_available(paths: &[&str]) -> FolderBrowserModel {
        let mut model = FolderBrowserModel::default();
        model.available = paths.iter().map(PathBuf::from).collect();
        model
    }

    #[test]
    fn insert_folder_prunes_overlapping_ancestors_and_descendants() {
        let mut selected = BTreeSet::from([
            PathBuf::from("drums"),
            PathBuf::from("drums/kicks/short"),
            PathBuf::from("drums/snares"),
        ]);

        insert_folder(&mut selected, Path::new("drums/kicks"), true);

        assert_eq!(
            selected,
            BTreeSet::from([PathBuf::from("drums/kicks"), PathBuf::from("drums/snares")])
        );
    }

    #[test]
    fn apply_root_selection_keeps_file_scope_mode_on_repeat_replace() {
        let mut model = FolderBrowserModel::default();

        assert!(apply_root_selection(&mut model, FolderSelectMode::Replace));
        assert_eq!(
            model.file_scope_mode,
            crate::app::state::FolderFileScopeMode::DirectOnly
        );
        assert_eq!(model.selected, BTreeSet::from([PathBuf::new()]));

        assert!(!apply_root_selection(&mut model, FolderSelectMode::Replace));
        assert_eq!(
            model.file_scope_mode,
            crate::app::state::FolderFileScopeMode::DirectOnly
        );
        assert_eq!(model.selected, BTreeSet::from([PathBuf::new()]));
    }

    #[test]
    fn apply_path_selection_toggle_clears_anchor_when_last_path_is_removed() {
        let mut model = model_with_available(&["kits", "kits/house"]);

        assert!(apply_path_selection(
            &mut model,
            Path::new("kits/house"),
            false,
            FolderSelectMode::Toggle,
        ));
        assert_eq!(model.selection_anchor, Some(PathBuf::from("kits/house")));

        assert!(apply_path_selection(
            &mut model,
            Path::new("kits/house"),
            false,
            FolderSelectMode::Toggle,
        ));
        assert!(model.selected.is_empty());
        assert!(model.selection_anchor.is_none());
    }
}
