use super::{DeleteRecoveryAction, DeleteRecoveryEntry, DeleteRecoveryStatus, SampleSource};
use std::{
    fs,
    path::{Path, PathBuf},
};

const RESTORE_SUFFIX: &str = ".restored";

pub(super) fn recovery_entry(
    source: &SampleSource,
    original_relative: PathBuf,
    action: DeleteRecoveryAction,
    outcome: Result<Option<String>, String>,
) -> DeleteRecoveryEntry {
    let (status, detail) = match outcome {
        Ok(detail) => (DeleteRecoveryStatus::Completed, detail),
        Err(err) => (DeleteRecoveryStatus::Failed, Some(err)),
    };
    DeleteRecoveryEntry {
        source_id: source.id.clone(),
        source_root: source.root.clone(),
        original_relative,
        action,
        status,
        detail,
    }
}

pub(super) fn restore_staged_folder(
    staged: &Path,
    original: &Path,
) -> Result<Option<String>, String> {
    if !staged.exists() {
        return Err("Staged folder missing".into());
    }
    let (target, detail) = unique_restore_path(original);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("Failed to restore folder: {err}"))?;
    }
    fs::rename(staged, &target).map_err(|err| format!("Failed to restore folder: {err}"))?;
    Ok(detail)
}

pub(super) fn unique_restore_path(original: &Path) -> (PathBuf, Option<String>) {
    if !original.exists() {
        return (original.to_path_buf(), None);
    }
    let parent = original.parent().unwrap_or_else(|| Path::new(""));
    let name = original
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("folder");
    for idx in 1..=1000 {
        let candidate = parent.join(format!("{name}{RESTORE_SUFFIX}-{idx}"));
        if !candidate.exists() {
            return (
                candidate.clone(),
                Some(format!("Restored as {}", candidate.display())),
            );
        }
    }
    let fallback = parent.join(format!("{name}{RESTORE_SUFFIX}-{}", uuid::Uuid::new_v4()));
    (
        fallback.clone(),
        Some(format!("Restored as {}", fallback.display())),
    )
}
