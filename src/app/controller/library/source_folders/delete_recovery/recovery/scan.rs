use super::*;

pub(super) fn journaled_staged_roots(journal: &DeleteJournal) -> Vec<PathBuf> {
    journal
        .entries
        .iter()
        .map(|entry| PathBuf::from(&entry.staged_relative))
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
            if !path.is_dir() {
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
