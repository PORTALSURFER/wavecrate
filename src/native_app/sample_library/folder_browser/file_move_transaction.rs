use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use super::{FileMoveConflict, FileMoveItem};

#[derive(Debug, Default)]
pub(super) struct FileMovePlan {
    pub(super) ready: Vec<FileTransfer>,
    pub(super) conflicts: Vec<FileMoveConflict>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct FileTransfer {
    pub(super) source_path: PathBuf,
    pub(super) destination_path: PathBuf,
    pub(super) copy_only: bool,
}

impl FileTransfer {
    fn move_file(source_path: PathBuf, destination_path: PathBuf) -> Self {
        Self {
            source_path,
            destination_path,
            copy_only: false,
        }
    }

    fn copy_file(source_path: PathBuf, destination_path: PathBuf) -> Self {
        Self {
            source_path,
            destination_path,
            copy_only: true,
        }
    }
}

#[derive(Debug)]
pub(super) struct OverwriteBackup {
    destination_path: PathBuf,
    backup_path: PathBuf,
}

pub(super) fn file_move_plan_to_folder(
    source_root: &Path,
    source_database_root: &Path,
    file_ids: &[String],
    target_path: &Path,
    target_protected: bool,
) -> Result<FileMovePlan, String> {
    let file_moves = file_ids
        .iter()
        .map(|file_id| FileMoveItem {
            source_root: source_root.to_path_buf(),
            source_database_root: source_database_root.to_path_buf(),
            file_id: file_id.clone(),
            copy_only: false,
        })
        .collect::<Vec<_>>();
    file_move_items_plan_to_folder(&file_moves, target_path, target_protected)
}

pub(super) fn file_move_items_plan_to_folder(
    file_moves: &[FileMoveItem],
    target_path: &Path,
    target_protected: bool,
) -> Result<FileMovePlan, String> {
    let mut plan = FileMovePlan::default();
    let mut seen = HashSet::new();
    let mut planned_destinations = HashSet::new();
    for item in file_moves {
        if !seen.insert(item.file_id.clone()) {
            continue;
        }
        let old_path = PathBuf::from(&item.file_id);
        if !old_path.is_file() {
            return Err(format!(
                "File move failed: {} is missing",
                old_path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| old_path.display().to_string())
            ));
        }
        if old_path.parent() == Some(target_path) {
            continue;
        }
        let Some(file_name) = old_path.file_name() else {
            return Err(String::from("File move failed: source file has no name"));
        };
        let mut new_path = target_path.join(file_name);
        if item.copy_only {
            new_path = unique_planned_destination(new_path, &mut planned_destinations);
            plan.ready.push(FileTransfer::copy_file(old_path, new_path));
        } else if new_path.exists() || !planned_destinations.insert(new_path.clone()) {
            plan.conflicts.push(FileMoveConflict {
                source_root: item.source_root.clone(),
                source_database_root: item.source_database_root.clone(),
                source_path: old_path,
                destination_path: new_path,
                destination_protected: target_protected,
            });
        } else {
            plan.ready.push(FileTransfer::move_file(old_path, new_path));
        }
    }
    Ok(plan)
}

#[cfg(test)]
pub(super) fn rename_files_with_rollback(
    moves: &[(PathBuf, PathBuf)],
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    rename_files_with_rollback_and_progress(moves, |_, _| {})
}

pub(super) fn rename_files_with_rollback_and_progress(
    moves: &[(PathBuf, PathBuf)],
    progress: impl FnMut(usize, &Path),
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let transfers = moves
        .iter()
        .map(|(source_path, destination_path)| {
            FileTransfer::move_file(source_path.clone(), destination_path.clone())
        })
        .collect::<Vec<_>>();
    transfer_files_with_rollback_and_progress(&transfers, progress)
}

pub(super) fn transfer_files_with_rollback_and_progress(
    transfers: &[FileTransfer],
    mut progress: impl FnMut(usize, &Path),
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let mut completed = Vec::new();
    for transfer in transfers {
        let result = if transfer.copy_only {
            copy_file(&transfer.source_path, &transfer.destination_path)
        } else {
            move_file(&transfer.source_path, &transfer.destination_path)
        };
        if let Err(error) = result {
            rollback_completed_file_transfers(&completed);
            return Err(error);
        }
        completed.push(transfer.clone());
        progress(completed.len(), &transfer.destination_path);
    }
    Ok(completed
        .into_iter()
        .map(|transfer| (transfer.source_path, transfer.destination_path))
        .collect())
}

pub(super) fn move_file_to_unique_destination(
    source_path: &Path,
    target_folder: &Path,
    error_prefix: &'static str,
) -> Result<(PathBuf, PathBuf), String> {
    let Some(file_name) = source_path.file_name() else {
        return Err(format!("{error_prefix}: file has no name"));
    };
    let destination = unique_destination(&target_folder.join(file_name));
    move_file_with_prefix(source_path, &destination, error_prefix)?;
    Ok((source_path.to_path_buf(), destination))
}

fn rollback_completed_file_transfers(completed: &[FileTransfer]) {
    for transfer in completed.iter().rev() {
        if transfer.copy_only {
            let _ = fs::remove_file(&transfer.destination_path);
        } else {
            let _ = move_file(&transfer.destination_path, &transfer.source_path);
        }
    }
}

pub(super) fn move_file(source: &Path, destination: &Path) -> Result<(), String> {
    move_file_with_prefix(source, destination, "File move failed")
}

fn copy_file(source: &Path, destination: &Path) -> Result<(), String> {
    fs::copy(source, destination)
        .map(|_| ())
        .map_err(|err| format!("File copy failed: {err}"))
}

fn move_file_with_prefix(
    source: &Path,
    destination: &Path,
    error_prefix: &'static str,
) -> Result<(), String> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(rename_error) => {
            if let Err(copy_error) = fs::copy(source, destination) {
                return Err(format!(
                    "{error_prefix}: {rename_error}; copy failed: {copy_error}"
                ));
            }
            if let Err(remove_error) = fs::remove_file(source) {
                let _ = fs::remove_file(destination);
                return Err(format!(
                    "{error_prefix}: copied but failed to remove original: {remove_error}"
                ));
            }
            Ok(())
        }
    }
}

pub(super) fn unique_destination(first_candidate: &Path) -> PathBuf {
    if !first_candidate.exists() {
        return first_candidate.to_path_buf();
    }
    let parent = first_candidate.parent().unwrap_or_else(|| Path::new(""));
    let stem = first_candidate
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("sample"));
    let extension = first_candidate
        .extension()
        .map(|extension| extension.to_string_lossy().to_string());
    for count in 1.. {
        let file_name = match &extension {
            Some(extension) => format!("{stem}_copy{count:03}.{extension}"),
            None => format!("{stem}_copy{count:03}"),
        };
        let candidate = parent.join(file_name);
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("unbounded copy suffix search should find a destination")
}

fn unique_planned_destination(
    first_candidate: PathBuf,
    planned_destinations: &mut HashSet<PathBuf>,
) -> PathBuf {
    if !first_candidate.exists() && planned_destinations.insert(first_candidate.clone()) {
        return first_candidate;
    }
    let parent = first_candidate.parent().unwrap_or_else(|| Path::new(""));
    let stem = first_candidate
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("sample"));
    let extension = first_candidate
        .extension()
        .map(|extension| extension.to_string_lossy().to_string());
    for count in 1.. {
        let file_name = match &extension {
            Some(extension) => format!("{stem}_copy{count:03}.{extension}"),
            None => format!("{stem}_copy{count:03}"),
        };
        let candidate = parent.join(file_name);
        if !candidate.exists() && planned_destinations.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("unbounded copy suffix search should find a destination")
}

pub(super) fn move_existing_destination_to_backup(
    destination_path: &Path,
) -> Result<OverwriteBackup, String> {
    let backup_path = unique_overwrite_backup_path(destination_path);
    move_file_with_prefix(destination_path, &backup_path, "File overwrite failed")?;
    Ok(OverwriteBackup {
        destination_path: destination_path.to_path_buf(),
        backup_path,
    })
}

pub(super) fn move_file_over_backup(
    source_path: &Path,
    destination_path: &Path,
) -> Result<(), String> {
    move_file_with_prefix(source_path, destination_path, "File overwrite failed")
}

pub(super) fn restore_overwrite_backup(backup: &OverwriteBackup) {
    let _ = move_file(&backup.backup_path, &backup.destination_path);
}

pub(super) fn remove_overwrite_backup(backup: &OverwriteBackup) {
    let _ = fs::remove_file(&backup.backup_path);
}

fn unique_overwrite_backup_path(destination_path: &Path) -> PathBuf {
    let parent = destination_path.parent().unwrap_or_else(|| Path::new(""));
    let file_name = destination_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("file"));
    for count in 1.. {
        let candidate = parent.join(format!(
            ".wavecrate-overwrite-backup-{count:03}-{file_name}"
        ));
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("unbounded backup suffix search should find a destination")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("{name}-{suffix}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn file_move_plan_deduplicates_ready_paths_and_reports_conflicts() {
        let root = temp_dir("wavecrate-file-move-plan");
        let source = root.join("source");
        let target = root.join("target");
        fs::create_dir_all(&source).expect("create source");
        fs::create_dir_all(&target).expect("create target");
        let ready = source.join("ready.wav");
        let conflict = source.join("conflict.wav");
        let existing = target.join("conflict.wav");
        fs::write(&ready, b"ready").expect("write ready");
        fs::write(&conflict, b"source").expect("write conflict source");
        fs::write(&existing, b"existing").expect("write conflict destination");

        let plan = file_move_plan_to_folder(
            &root,
            &root,
            &[
                ready.display().to_string(),
                ready.display().to_string(),
                conflict.display().to_string(),
            ],
            &target,
            true,
        )
        .expect("plan file moves");

        assert_eq!(
            plan.ready,
            vec![FileTransfer::move_file(
                ready.clone(),
                target.join("ready.wav")
            )]
        );
        assert_eq!(
            plan.conflicts,
            vec![FileMoveConflict {
                source_root: root.clone(),
                source_database_root: root.clone(),
                source_path: conflict,
                destination_path: existing,
                destination_protected: true,
            }]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rename_files_with_rollback_restores_completed_moves_after_later_failure() {
        let root = temp_dir("wavecrate-file-move-rollback");
        let source = root.join("source");
        let target = root.join("target");
        fs::create_dir_all(&source).expect("create source");
        fs::create_dir_all(&target).expect("create target");
        let first_source = source.join("first.wav");
        let second_source = source.join("second.wav");
        let first_destination = target.join("first.wav");
        let missing_destination = root.join("missing-parent").join("second.wav");
        fs::write(&first_source, b"first").expect("write first");
        fs::write(&second_source, b"second").expect("write second");

        let result = rename_files_with_rollback(&[
            (first_source.clone(), first_destination.clone()),
            (second_source.clone(), missing_destination),
        ]);

        assert!(result.is_err());
        assert_eq!(
            fs::read(&first_source).expect("read first source"),
            b"first"
        );
        assert!(!first_destination.exists());
        assert_eq!(
            fs::read(&second_source).expect("read second source"),
            b"second"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn move_file_to_unique_destination_renames_conflicting_extracted_file() {
        let root = temp_dir("wavecrate-file-move-unique-extracted");
        let source = root.join("source");
        let target = root.join("target");
        fs::create_dir_all(&source).expect("create source");
        fs::create_dir_all(&target).expect("create target");
        let extracted = source.join("loop.wav");
        let existing = target.join("loop.wav");
        fs::write(&extracted, b"extracted").expect("write extracted");
        fs::write(&existing, b"existing").expect("write existing");

        let moved = move_file_to_unique_destination(&extracted, &target, "Extraction move failed")
            .expect("move extracted file");

        let renamed = target.join("loop_copy001.wav");
        assert_eq!(moved, (extracted.clone(), renamed.clone()));
        assert!(!extracted.exists());
        assert_eq!(fs::read(existing).expect("read existing"), b"existing");
        assert_eq!(fs::read(renamed).expect("read renamed"), b"extracted");
        let _ = fs::remove_dir_all(root);
    }
}
