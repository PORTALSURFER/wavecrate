use super::super::*;
use crate::app::state::RootFolderFilterMode;
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

impl AppController {
    pub(crate) fn folder_selection_for_filter(&self) -> Option<&BTreeSet<PathBuf>> {
        let id = self.selection_state.ctx.selected_source.as_ref()?;
        self.ui_cache
            .folders
            .models
            .get(id)
            .map(|model| &model.selected)
    }

    /// Read the active root filter mode for the current source.
    pub(crate) fn root_folder_filter_mode_for_filter(&self) -> Option<RootFolderFilterMode> {
        let id = self.selection_state.ctx.selected_source.as_ref()?;
        self.ui_cache
            .folders
            .models
            .get(id)
            .map(|model| model.root_filter_mode)
    }

    pub(crate) fn folder_negation_for_filter(&self) -> Option<&BTreeSet<PathBuf>> {
        let id = self.selection_state.ctx.selected_source.as_ref()?;
        self.ui_cache
            .folders
            .models
            .get(id)
            .map(|model| &model.negated)
    }

    pub(crate) fn folder_filter_accepts(&self, relative_path: &Path) -> bool {
        let selection = self.folder_selection_for_filter();
        let negated = self.folder_negation_for_filter();
        let root_mode = self
            .root_folder_filter_mode_for_filter()
            .unwrap_or_default();
        folder_filter_accepts(relative_path, selection, negated, root_mode)
    }
}

pub(crate) fn folder_filter_accepts(
    relative_path: &Path,
    selection: Option<&BTreeSet<PathBuf>>,
    negated: Option<&BTreeSet<PathBuf>>,
    root_mode: RootFolderFilterMode,
) -> bool {
    let has_selection = selection.is_some_and(|set| !set.is_empty());
    if has_selection {
        let selection = selection.expect("checked above");
        let root_selected = selection.contains(Path::new(""));
        let selected = if root_selected {
            match root_mode {
                RootFolderFilterMode::AllDescendants => true,
                RootFolderFilterMode::RootOnly => {
                    let in_root = relative_path
                        .parent()
                        .unwrap_or(Path::new(""))
                        .as_os_str()
                        .is_empty();
                    let in_selected_folder = selection
                        .iter()
                        .filter(|folder| !folder.as_os_str().is_empty())
                        .any(|folder| relative_path.starts_with(folder));
                    in_root || in_selected_folder
                }
            }
        } else {
            selection
                .iter()
                .filter(|folder| !folder.as_os_str().is_empty())
                .any(|folder| relative_path.starts_with(folder))
        };
        if !selected {
            return false;
        }
    }
    let excluded = negated
        .map(|set| is_negated_relative_path(relative_path, set))
        .unwrap_or(false);
    !excluded
}

/// Hash the active folder-filter payload into a stable cache fingerprint.
pub(crate) fn folder_filter_fingerprint(
    selection: Option<&BTreeSet<PathBuf>>,
    negated: Option<&BTreeSet<PathBuf>>,
    root_mode: RootFolderFilterMode,
) -> u64 {
    hash_value(&(selection, negated, root_mode_key(root_mode)))
}

/// Build folder-filter acceptance values for a path stream in source order.
pub(crate) fn build_folder_filter_acceptance_map<'a>(
    relative_paths: impl IntoIterator<Item = Option<&'a Path>>,
    selection: Option<&BTreeSet<PathBuf>>,
    negated: Option<&BTreeSet<PathBuf>>,
    root_mode: RootFolderFilterMode,
) -> Vec<bool> {
    relative_paths
        .into_iter()
        .map(|relative_path| {
            relative_path
                .is_some_and(|path| folder_filter_accepts(path, selection, negated, root_mode))
        })
        .collect()
}

/// Report whether folder-based filtering is active.
pub(crate) fn folder_filters_active(
    selection: Option<&BTreeSet<PathBuf>>,
    negated: Option<&BTreeSet<PathBuf>>,
    root_mode: RootFolderFilterMode,
) -> bool {
    let has_negation = negated.is_some_and(|set| !set.is_empty());
    if has_negation {
        return true;
    }
    let Some(selection) = selection else {
        return false;
    };
    if selection.is_empty() {
        return false;
    }
    let root_selected = selection.contains(Path::new(""));
    if root_selected && root_mode == RootFolderFilterMode::AllDescendants {
        return false;
    }
    true
}

fn is_negated_relative_path(relative_path: &Path, negated: &BTreeSet<PathBuf>) -> bool {
    if negated.is_empty() {
        return false;
    }
    let root_negated = negated.contains(Path::new(""));
    if root_negated {
        let parent = relative_path.parent().unwrap_or(Path::new(""));
        if parent.as_os_str().is_empty() {
            return true;
        }
    }
    negated
        .iter()
        .filter(|folder| !folder.as_os_str().is_empty())
        .any(|folder| relative_path.starts_with(folder))
}

fn root_mode_key(mode: RootFolderFilterMode) -> u8 {
    match mode {
        RootFolderFilterMode::AllDescendants => 0,
        RootFolderFilterMode::RootOnly => 1,
    }
}

fn hash_value<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_filter_fingerprint_changes_with_root_mode() {
        let selection = BTreeSet::from([PathBuf::from("")]);

        let all =
            folder_filter_fingerprint(Some(&selection), None, RootFolderFilterMode::AllDescendants);
        let root_only =
            folder_filter_fingerprint(Some(&selection), None, RootFolderFilterMode::RootOnly);

        assert_ne!(all, root_only);
    }

    #[test]
    fn build_folder_filter_acceptance_map_matches_direct_contract() {
        let selection = BTreeSet::from([PathBuf::from(""), PathBuf::from("drums")]);
        let negated = BTreeSet::from([PathBuf::from("hits")]);
        let paths = [
            Some(Path::new("root.wav")),
            Some(Path::new("drums/kick.wav")),
            Some(Path::new("hits/snare.wav")),
            None,
        ];

        let accepts = build_folder_filter_acceptance_map(
            paths,
            Some(&selection),
            Some(&negated),
            RootFolderFilterMode::RootOnly,
        );

        assert_eq!(accepts, vec![true, true, false, false]);
    }
}
