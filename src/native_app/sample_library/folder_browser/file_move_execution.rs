use std::{
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

use super::{
    FileMoveConflict, FileMoveConflictBatch, FileMoveConflictCompletion,
    FileMoveConflictExecutionFailure, FileMoveConflictExecutionSuccess, FileMoveConflictResolution,
    FileMoveConflictResolutionRequest, FolderMoveCompletion, FolderMoveRequest, FolderMoveSuccess,
    drag_drop_relocation::{persist_moved_file_metadata, persist_moved_folder_metadata},
    file_move_progress::{
        FileMoveProgressReporter, file_move_conflict_progress_label,
        file_move_conflict_progress_total, folder_move_progress_label,
    },
    file_move_transaction::{FileMovePlan, file_move_plan_to_folder},
    file_move_transaction::{
        move_existing_destination_to_backup, move_file_over_backup,
        move_file_to_unique_destination, remove_overwrite_backup,
        rename_files_with_rollback_and_progress, restore_overwrite_backup, unique_destination,
    },
};
use crate::native_app::{
    app::GuiMessage, sample_library::file_actions::sample_path_label,
    waveform::remap_persisted_waveform_cache_after_move,
};

#[cfg(test)]
pub(in crate::native_app) fn execute_folder_move_request(
    request: FolderMoveRequest,
) -> FolderMoveCompletion {
    execute_folder_move_request_with_progress(request, 0, None)
}

pub(in crate::native_app) fn execute_folder_move_request_with_progress(
    request: FolderMoveRequest,
    task_id: u64,
    sender: Option<Sender<GuiMessage>>,
) -> FolderMoveCompletion {
    let reporter =
        FileMoveProgressReporter::new(task_id, folder_move_progress_label(&request), sender);
    let result = execute_folder_move_request_result(&request, &reporter);
    FolderMoveCompletion {
        task_id,
        request,
        result,
    }
}

fn execute_folder_move_request_result(
    request: &FolderMoveRequest,
    reporter: &FileMoveProgressReporter,
) -> Result<FolderMoveSuccess, String> {
    match request {
        FolderMoveRequest::Folder {
            source_root,
            old_path,
            new_path,
            ..
        } => execute_folder_move(source_root, old_path, new_path, reporter),
        FolderMoveRequest::Files {
            source_root,
            file_ids,
            target_folder,
            remove_from_collection,
        } => execute_file_drop(
            source_root,
            file_ids,
            target_folder,
            *remove_from_collection,
            reporter,
        ),
        FolderMoveRequest::ExtractedFile {
            source_root,
            path,
            target_folder,
        } => execute_extracted_file_drop(source_root, path, target_folder, reporter),
    }
}

fn execute_folder_move(
    source_root: &Path,
    old_path: &Path,
    new_path: &Path,
    reporter: &FileMoveProgressReporter,
) -> Result<FolderMoveSuccess, String> {
    if new_path.exists() {
        let folder_name = new_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| new_path.display().to_string());
        return Err(format!("Folder move failed: {folder_name} already exists"));
    }
    reporter.emit(0, 2, format!("Moving {}", sample_path_label(old_path)));
    std::fs::rename(old_path, new_path).map_err(|error| format!("Folder move failed: {error}"))?;
    reporter.emit(1, 2, String::from("Preserving waveform cache"));
    let moved_paths = vec![(old_path.to_path_buf(), new_path.to_path_buf())];
    remap_persisted_waveform_cache_for_moves(&moved_paths);
    reporter.emit(1, 2, String::from("Updating metadata"));
    let metadata_error = persist_moved_folder_metadata(source_root, old_path, new_path).err();
    reporter.emit(2, 2, String::from("Done"));
    Ok(FolderMoveSuccess {
        moved_paths,
        conflicts: Vec::new(),
        metadata_error,
    })
}

fn execute_file_drop(
    source_root: &Path,
    file_ids: &[String],
    target_folder: &Path,
    remove_from_collection: Option<wavecrate::sample_sources::SampleCollection>,
    reporter: &FileMoveProgressReporter,
) -> Result<FolderMoveSuccess, String> {
    if !target_folder.is_dir() {
        return Err(String::from("File move failed: target folder is missing"));
    }
    let plan = file_move_plan_to_folder(file_ids, target_folder)?;
    let total = file_move_work_total(&plan);
    reporter.emit(0, total, String::from("Moving files"));
    let moved_paths = rename_files_with_rollback_and_progress(&plan.ready, |completed, path| {
        reporter.emit(
            completed,
            total,
            format!("Moved {}", sample_path_label(path)),
        );
    })?;
    let checked = moved_paths.len().saturating_add(plan.conflicts.len());
    reporter.emit(checked, total, String::from("Preserving waveform cache"));
    remap_persisted_waveform_cache_for_moves(&moved_paths);
    reporter.emit(checked, total, String::from("Updating metadata"));
    let metadata_error =
        persist_moved_file_metadata(source_root, &moved_paths, remove_from_collection).err();
    reporter.emit(total, total, String::from("Done"));
    Ok(FolderMoveSuccess {
        moved_paths,
        conflicts: plan.conflicts,
        metadata_error,
    })
}

fn execute_extracted_file_drop(
    source_root: &Path,
    path: &Path,
    target_folder: &Path,
    reporter: &FileMoveProgressReporter,
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
    reporter.emit(0, 2, format!("Moving {}", sample_path_label(path)));
    let completed = move_file_to_unique_destination(path, target_folder, "Extraction move failed")?;
    reporter.emit(1, 2, String::from("Preserving waveform cache"));
    let moved_paths = vec![completed];
    remap_persisted_waveform_cache_for_moves(&moved_paths);
    reporter.emit(1, 2, String::from("Updating metadata"));
    let metadata_error = persist_moved_file_metadata(source_root, &moved_paths, None).err();
    reporter.emit(2, 2, String::from("Done"));
    Ok(FolderMoveSuccess {
        moved_paths,
        conflicts: Vec::new(),
        metadata_error,
    })
}

#[cfg(test)]
pub(in crate::native_app) fn execute_file_move_conflict_request(
    batch: FileMoveConflictBatch,
    request: FileMoveConflictResolutionRequest,
) -> FileMoveConflictCompletion {
    execute_file_move_conflict_request_with_progress(batch, request, 0, None)
}

pub(in crate::native_app) fn execute_file_move_conflict_request_with_progress(
    mut batch: FileMoveConflictBatch,
    request: FileMoveConflictResolutionRequest,
    task_id: u64,
    sender: Option<Sender<GuiMessage>>,
) -> FileMoveConflictCompletion {
    let reporter = FileMoveProgressReporter::new(
        task_id,
        file_move_conflict_progress_label(&batch, request),
        sender,
    );
    if request.apply_to_remaining {
        batch.batch_policy = Some(request.resolution);
    }

    let total = file_move_conflict_progress_total(&batch, request);
    let mut moved_paths = Vec::new();
    let mut last_resolution = request.resolution;
    loop {
        let Some(conflict) = batch.conflicts.get(batch.current_index).cloned() else {
            break;
        };
        let completed_conflicts = batch.resolved_count.saturating_add(batch.skipped_count);
        reporter.emit(
            completed_conflicts,
            total,
            format!("Resolving {}", sample_path_label(&conflict.source_path)),
        );
        let resolution = batch.batch_policy.unwrap_or(request.resolution);
        let completed = match execute_file_move_conflict(&conflict, resolution) {
            Ok(completed) => completed,
            Err(error) => {
                batch.batch_policy = None;
                let metadata_error = persist_moved_file_metadata(
                    &batch.source_root,
                    &moved_paths,
                    batch.remove_from_collection,
                )
                .err();
                return FileMoveConflictCompletion {
                    task_id,
                    result: Err(FileMoveConflictExecutionFailure {
                        batch,
                        moved_paths,
                        error,
                        metadata_error,
                    }),
                };
            }
        };
        remap_persisted_waveform_cache_for_moves(&completed);
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
        let completed_conflicts = batch.resolved_count.saturating_add(batch.skipped_count);
        reporter.emit(
            completed_conflicts,
            total,
            String::from("Resolving conflicts"),
        );
        if batch.batch_policy.is_none() {
            break;
        }
    }
    let completed_conflicts = batch.resolved_count.saturating_add(batch.skipped_count);
    reporter.emit(
        completed_conflicts,
        total,
        String::from("Updating metadata"),
    );
    let metadata_error = persist_moved_file_metadata(
        &batch.source_root,
        &moved_paths,
        batch.remove_from_collection,
    )
    .err();
    reporter.emit(total, total, String::from("Done"));

    FileMoveConflictCompletion {
        task_id,
        result: Ok(FileMoveConflictExecutionSuccess {
            batch,
            moved_paths,
            last_resolution,
            metadata_error,
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
    rename_files_with_rollback_and_progress(std::slice::from_ref(&move_pair), |_, _| {})
}

fn file_move_work_total(plan: &FileMovePlan) -> usize {
    plan.ready
        .len()
        .saturating_add(plan.conflicts.len())
        .saturating_add(1)
        .max(1)
}

fn remap_persisted_waveform_cache_for_moves(moved_paths: &[(PathBuf, PathBuf)]) {
    for (old_path, new_path) in moved_paths {
        remap_persisted_waveform_cache_after_move(old_path, new_path);
    }
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
            source_root: root.clone(),
            file_ids: vec![source.display().to_string()],
            target_folder: target_dir,
            remove_from_collection: None,
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
    fn ready_file_moves_emit_progress_messages() {
        let root = temp_dir("wavecrate-file-move-execution-progress");
        let source_dir = root.join("source");
        let target_dir = root.join("target");
        fs::create_dir_all(&source_dir).expect("create source");
        fs::create_dir_all(&target_dir).expect("create target");
        let first = source_dir.join("first.wav");
        let second = source_dir.join("second.wav");
        fs::write(&first, b"first").expect("write first");
        fs::write(&second, b"second").expect("write second");
        let request = FolderMoveRequest::Files {
            source_root: root.clone(),
            file_ids: vec![first.display().to_string(), second.display().to_string()],
            target_folder: target_dir,
            remove_from_collection: None,
        };
        let (sender, receiver) = std::sync::mpsc::channel();

        let completion = execute_folder_move_request_with_progress(request, 42, Some(sender));

        assert!(
            completion
                .result
                .expect("move succeeds")
                .metadata_error
                .is_none()
        );
        let progress = receiver
            .try_iter()
            .filter_map(|message| match message {
                GuiMessage::FileMoveProgress(progress) => Some(progress),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            progress.iter().any(|progress| progress.completed > 0),
            "file move worker should stream progress: {progress:?}"
        );
        assert!(
            progress
                .last()
                .is_some_and(|progress| progress.completed == progress.total),
            "file move worker should finish progress at total: {progress:?}"
        );
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
            source_root: root.clone(),
            target_folder: target_dir.clone(),
            remove_from_collection: None,
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
