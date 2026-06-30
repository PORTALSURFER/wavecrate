use super::*;
use crate::app::controller::library::source_folders::delete_recovery::path_policy;

pub(super) fn journaled_staged_roots(journal: &DeleteJournal) -> Vec<PathBuf> {
    journal
        .entries
        .iter()
        .filter_map(|entry| {
            path_policy::validate_journal_relative(&entry.staged_relative, "staged_relative").ok()
        })
        .collect()
}

pub(super) fn find_unjournaled_staged_roots(
    staging_root: &Path,
    source_root: &Path,
    journaled_roots: &[PathBuf],
) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let mut stack = vec![PathBuf::new()];
    while let Some(relative) = stack.pop() {
        let current = staging_root.join(&relative);
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy() == DELETE_JOURNAL_FILE {
                continue;
            }
            let path = entry.path();
            let Ok(metadata) = fs::symlink_metadata(&path) else {
                continue;
            };
            if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
                continue;
            }
            let child_relative = relative.join(entry.file_name());
            if path_is_under_roots(&child_relative, journaled_roots) {
                continue;
            }
            if source_root.join(&child_relative).exists() {
                stack.push(child_relative);
            } else {
                roots.push(child_relative);
            }
        }
    }
    roots
}

fn path_is_under_roots(candidate: &Path, roots: &[PathBuf]) -> bool {
    roots.iter().any(|root| candidate.starts_with(root))
}
