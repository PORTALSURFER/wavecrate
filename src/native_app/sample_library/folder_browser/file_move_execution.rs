use std::path::{Path, PathBuf};

use super::{
    FileMoveConflict, FileMoveConflictBatch, FileMoveConflictCompletion,
    FileMoveConflictExecutionFailure, FileMoveConflictExecutionSuccess, FileMoveConflictResolution,
    FileMoveConflictResolutionRequest, FolderMoveCompletion, FolderMoveRequest, FolderMoveSuccess,
    file_move_transaction::file_move_plan_to_folder,
    file_move_transaction::{
        move_existing_destination_to_backup, move_file_over_backup,
        move_file_to_unique_destination, remove_overwrite_backup, rename_files_with_rollback,
        restore_overwrite_backup, unique_destination,
    },
};

pub(in crate::native_app) fn execute_folder_move_request(
    request: FolderMoveRequest,
) -> FolderMoveCompletion {
    let result = execute_folder_move_request_result(&request);
    FolderMoveCompletion { request, result }
}

fn execute_folder_move_request_result(
    request: &FolderMoveRequest,
) -> Result<FolderMoveSuccess, String> {
    match request {
        FolderMoveRequest::Folder {
            old_path, new_path, ..
        } => execute_folder_move(old_path, new_path),
        FolderMoveRequest::Files {
            file_ids,
            target_folder,
        } => execute_file_drop(file_ids, target_folder),
        FolderMoveRequest::ExtractedFile {
            path,
            target_folder,
        } => execute_extracted_file_drop(path, target_folder),
    }
}

fn execute_folder_move(old_path: &Path, new_path: &Path) -> Result<FolderMoveSuccess, String> {
    if new_path.exists() {
        let folder_name = new_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| new_path.display().to_string());
        return Err(format!("Folder move failed: {folder_name} already exists"));
    }
    std::fs::rename(old_path, new_path).map_err(|error| format!("Folder move failed: {error}"))?;
    Ok(FolderMoveSuccess {
        moved_paths: vec![(old_path.to_path_buf(), new_path.to_path_buf())],
        conflicts: Vec::new(),
    })
}

fn execute_file_drop(
    file_ids: &[String],
    target_folder: &Path,
) -> Result<FolderMoveSuccess, String> {
    if !target_folder.is_dir() {
        return Err(String::from("File move failed: target folder is missing"));
    }
    let plan = file_move_plan_to_folder(file_ids, target_folder)?;
    let moved_paths = rename_files_with_rollback(&plan.ready)?;
    Ok(FolderMoveSuccess {
        moved_paths,
        conflicts: plan.conflicts,
    })
}

fn execute_extracted_file_drop(
    path: &Path,
    target_folder: &Path,
) -> Result<FolderMoveSuccess, String> {
    if !path.is_file() {
        return Err(format!(
            "Extraction move failed: {} is missing",
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| path.display().to_string())
        ));
    }
    if !target_folder.is_dir() {
        return Err(String::from(
            "Extraction move failed: target folder is missing",
        ));
    }
    let completed = move_file_to_unique_destination(path, target_folder, "Extraction move failed")?;
    Ok(FolderMoveSuccess {
        moved_paths: vec![completed],
        conflicts: Vec::new(),
    })
}

pub(in crate::native_app) fn execute_file_move_conflict_request(
    mut batch: FileMoveConflictBatch,
    request: FileMoveConflictResolutionRequest,
) -> FileMoveConflictCompletion {
    if request.apply_to_remaining {
        batch.batch_policy = Some(request.resolution);
    }

    let mut moved_paths = Vec::new();
    let mut last_resolution = request.resolution;
    loop {
        let Some(conflict) = batch.conflicts.get(batch.current_index).cloned() else {
            break;
        };
        let resolution = batch.batch_policy.unwrap_or(request.resolution);
        let completed = match execute_file_move_conflict(&conflict, resolution) {
            Ok(completed) => completed,
            Err(error) => {
                batch.batch_policy = None;
                return FileMoveConflictCompletion {
                    result: Err(FileMoveConflictExecutionFailure {
                        batch,
                        moved_paths,
                        error,
                    }),
                };
            }
        };
        match resolution {
            FileMoveConflictResolution::Overwrite | FileMoveConflictResolution::Rename => {
                batch.resolved_count += 1;
            }
            FileMoveConflictResolution::Skip => {
                batch.skipped_count += 1;
            }
        }
        batch.current_index += 1;
        last_resolution = resolution;
        moved_paths.extend(completed);
        if batch.batch_policy.is_none() {
            break;
        }
    }

    FileMoveConflictCompletion {
        result: Ok(FileMoveConflictExecutionSuccess {
            batch,
            moved_paths,
            last_resolution,
        }),
    }
}

fn execute_file_move_conflict(
    conflict: &FileMoveConflict,
    resolution: FileMoveConflictResolution,
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    match resolution {
        FileMoveConflictResolution::Overwrite => execute_overwrite_conflict(conflict),
        FileMoveConflictResolution::Rename => execute_rename_conflict(conflict),
        FileMoveConflictResolution::Skip => Ok(Vec::new()),
    }
}

fn execute_overwrite_conflict(
    conflict: &FileMoveConflict,
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let backup = move_existing_destination_to_backup(&conflict.destination_path)?;
    if let Err(error) = move_file_over_backup(&conflict.source_path, &conflict.destination_path) {
        restore_overwrite_backup(&backup);
        return Err(error);
    }

    let completed = vec![(
        conflict.source_path.clone(),
        conflict.destination_path.clone(),
    )];
    remove_overwrite_backup(&backup);
    Ok(completed)
}

fn execute_rename_conflict(conflict: &FileMoveConflict) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let destination = unique_destination(&conflict.destination_path);
    let move_pair = (conflict.source_path.clone(), destination);
    rename_files_with_rollback(std::slice::from_ref(&move_pair))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

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
    fn ready_file_moves_execute_as_worker_request() {
        let root = temp_dir("wavecrate-file-move-execution-ready-rollback");
        let source_dir = root.join("source");
        let target_dir = root.join("target");
        fs::create_dir_all(&source_dir).expect("create source");
        fs::create_dir_all(&target_dir).expect("create target");
        let source = source_dir.join("kick.wav");
        let destination = target_dir.join("kick.wav");
        fs::write(&source, b"source").expect("write source");

        let request = FolderMoveRequest::Files {
            file_ids: vec![source.display().to_string()],
            target_folder: target_dir,
        };
        let result = execute_folder_move_request(request).result;

        assert_eq!(
            result.expect("move succeeds").moved_paths,
            vec![(source.clone(), destination.clone())]
        );
        assert!(!source.exists());
        assert_eq!(fs::read(&destination).expect("read destination"), b"source");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn overwrite_conflict_executes_as_worker_request() {
        let root = temp_dir("wavecrate-file-move-execution-overwrite-rollback");
        let source_dir = root.join("source");
        let target_dir = root.join("target");
        fs::create_dir_all(&source_dir).expect("create source");
        fs::create_dir_all(&target_dir).expect("create target");
        let source = source_dir.join("kick.wav");
        let destination = target_dir.join("kick.wav");
        fs::write(&source, b"source").expect("write source");
        fs::write(&destination, b"destination").expect("write destination");
        let conflict = FileMoveConflict {
            source_path: source.clone(),
            destination_path: destination.clone(),
        };

        let batch = FileMoveConflictBatch {
            target_folder: target_dir.clone(),
            conflicts: vec![conflict],
            current_index: 0,
            resolved_count: 0,
            skipped_count: 0,
            batch_policy: None,
        };
        let result = execute_file_move_conflict_request(
            batch,
            FileMoveConflictResolutionRequest::new(FileMoveConflictResolution::Overwrite, false),
        );

        assert_eq!(
            result.result.expect("overwrite succeeds").moved_paths,
            vec![(source.clone(), destination.clone())]
        );
        assert!(!source.exists());
        assert_eq!(fs::read(&destination).expect("read destination"), b"source");
        assert!(
            fs::read_dir(&target_dir)
                .expect("read target")
                .all(|entry| !entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".wavecrate-overwrite-backup-")),
            "overwrite backup should be restored and removed from view"
        );
        let _ = fs::remove_dir_all(root);
    }
}
