use super::super::*;
use crate::app::state::FolderFileScopeMode;
use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

impl AppController {
    pub(crate) fn folder_selection_for_filter(&self) -> Option<&BTreeSet<PathBuf>> {
        self.current_folder_model().map(|model| &model.selected)
    }

    /// Read the active folder file-scope mode for the current source.
    pub(crate) fn folder_file_scope_mode_for_filter(&self) -> Option<FolderFileScopeMode> {
        self.current_folder_model()
            .map(|model| model.file_scope_mode)
    }

    pub(crate) fn folder_negation_for_filter(&self) -> Option<&BTreeSet<PathBuf>> {
        self.current_folder_model().map(|model| &model.negated)
    }

    pub(crate) fn folder_filter_accepts(&self, relative_path: &Path) -> bool {
        let selection = self.folder_selection_for_filter();
        let negated = self.folder_negation_for_filter();
        let file_scope_mode = self.folder_file_scope_mode_for_filter().unwrap_or_default();
        folder_filter_accepts(relative_path, selection, negated, file_scope_mode)
    }
}

pub(crate) fn folder_filter_accepts(
    relative_path: &Path,
    selection: Option<&BTreeSet<PathBuf>>,
    negated: Option<&BTreeSet<PathBuf>>,
    file_scope_mode: FolderFileScopeMode,
) -> bool {
    let has_selection = selection.is_some_and(|set| !set.is_empty());
    if has_selection {
        let selection = selection.expect("checked above");
        let selected = selection
            .iter()
            .any(|folder| relative_path_matches_scope(relative_path, folder, file_scope_mode));
        if !selected {
            return false;
        }
    }
    let excluded = negated
        .map(|set| {
            set.iter()
                .any(|folder| relative_path_matches_scope(relative_path, folder, file_scope_mode))
        })
        .unwrap_or(false);
    !excluded
}

/// Hash the active folder-filter payload into a stable cache fingerprint.
pub(crate) fn folder_filter_fingerprint(
    selection: Option<&BTreeSet<PathBuf>>,
    negated: Option<&BTreeSet<PathBuf>>,
    file_scope_mode: FolderFileScopeMode,
) -> u64 {
    hash_value(&(selection, negated, file_scope_mode_key(file_scope_mode)))
}

/// Build folder-filter acceptance values for a path stream in source order.
pub(crate) fn build_folder_filter_acceptance_map<'a>(
    relative_paths: impl IntoIterator<Item = Option<&'a Path>>,
    selection: Option<&BTreeSet<PathBuf>>,
    negated: Option<&BTreeSet<PathBuf>>,
    file_scope_mode: FolderFileScopeMode,
) -> Vec<bool> {
    relative_paths
        .into_iter()
        .map(|relative_path| {
            relative_path.is_some_and(|path| {
                folder_filter_accepts(path, selection, negated, file_scope_mode)
            })
        })
        .collect()
}

/// Report whether folder-based filtering is active.
pub(crate) fn folder_filters_active(
    selection: Option<&BTreeSet<PathBuf>>,
    negated: Option<&BTreeSet<PathBuf>>,
    file_scope_mode: FolderFileScopeMode,
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
    if root_selected && file_scope_mode == FolderFileScopeMode::AllDescendants {
        return false;
    }
    true
}

fn relative_path_matches_scope(
    relative_path: &Path,
    folder: &Path,
    file_scope_mode: FolderFileScopeMode,
) -> bool {
    match file_scope_mode {
        FolderFileScopeMode::DirectOnly => {
            relative_path.parent().unwrap_or(Path::new("")) == folder
        }
        FolderFileScopeMode::AllDescendants => {
            folder.as_os_str().is_empty() || relative_path.starts_with(folder)
        }
    }
}

fn file_scope_mode_key(mode: FolderFileScopeMode) -> u8 {
    match mode {
        FolderFileScopeMode::DirectOnly => 0,
        FolderFileScopeMode::AllDescendants => 1,
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
    fn folder_filter_fingerprint_changes_with_file_scope_mode() {
        let selection = BTreeSet::from([PathBuf::from("")]);

        let direct_only =
            folder_filter_fingerprint(Some(&selection), None, FolderFileScopeMode::DirectOnly);
        let all_descendants =
            folder_filter_fingerprint(Some(&selection), None, FolderFileScopeMode::AllDescendants);

        assert_ne!(direct_only, all_descendants);
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
            FolderFileScopeMode::DirectOnly,
        );

        assert_eq!(accepts, vec![true, true, false, false]);
    }

    #[test]
    fn build_folder_filter_acceptance_map_includes_descendants_when_flattened() {
        let selection = BTreeSet::from([PathBuf::from("drums")]);
        let paths = [
            Some(Path::new("drums/root.wav")),
            Some(Path::new("drums/kicks/deep.wav")),
            Some(Path::new("bass/sub.wav")),
        ];

        let accepts = build_folder_filter_acceptance_map(
            paths,
            Some(&selection),
            None,
            FolderFileScopeMode::AllDescendants,
        );

        assert_eq!(accepts, vec![true, true, false]);
    }
}
