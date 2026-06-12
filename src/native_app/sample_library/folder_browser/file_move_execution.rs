use std::path::{Path, PathBuf};

use super::{
    FileMoveConflict, FileMoveConflictResolution,
    file_move_transaction::{
        move_existing_destination_to_backup, move_file_over_backup,
        move_file_to_unique_destination, remove_overwrite_backup, rename_files_with_rollback,
        restore_overwrite_backup, rollback_completed_file_moves, rollback_overwrite_move,
        unique_destination,
    },
};

pub(super) fn execute_folder_move(
    old_path: &Path,
    new_path: &Path,
    apply_move: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    std::fs::rename(old_path, new_path).map_err(|error| format!("Folder move failed: {error}"))?;
    if let Err(error) = apply_move() {
        let _ = std::fs::rename(new_path, old_path);
        return Err(error);
    }
    Ok(())
}

pub(super) fn execute_ready_file_moves(
    moves: &[(PathBuf, PathBuf)],
    apply_moves: impl FnOnce(&[(PathBuf, PathBuf)]) -> Result<(), String>,
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let completed = rename_files_with_rollback(moves)?;
    if let Err(error) = apply_moves(&completed) {
        rollback_completed_file_moves(&completed);
        return Err(error);
    }
    Ok(completed)
}

pub(super) fn execute_extracted_file_move(
    path: &Path,
    target_folder: &Path,
    apply_moves: impl FnOnce(&[(PathBuf, PathBuf)]) -> Result<(), String>,
) -> Result<(PathBuf, PathBuf), String> {
    let completed_move =
        move_file_to_unique_destination(path, target_folder, "Extraction move failed")?;
    if let Err(error) = apply_moves(std::slice::from_ref(&completed_move)) {
        rollback_completed_file_moves(std::slice::from_ref(&completed_move));
        return Err(error);
    }
    Ok(completed_move)
}

pub(super) fn execute_file_move_conflict(
    conflict: &FileMoveConflict,
    resolution: FileMoveConflictResolution,
    apply_moves: impl FnOnce(&[(PathBuf, PathBuf)]) -> Result<(), String>,
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    match resolution {
        FileMoveConflictResolution::Overwrite => execute_overwrite_conflict(conflict, apply_moves),
        FileMoveConflictResolution::Rename => execute_rename_conflict(conflict, apply_moves),
        FileMoveConflictResolution::Skip => Ok(Vec::new()),
    }
}

fn execute_overwrite_conflict(
    conflict: &FileMoveConflict,
    apply_moves: impl FnOnce(&[(PathBuf, PathBuf)]) -> Result<(), String>,
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
    if let Err(error) = apply_moves(&completed) {
        rollback_overwrite_move(&completed[0], &backup);
        return Err(error);
    }
    remove_overwrite_backup(&backup);
    Ok(completed)
}

fn execute_rename_conflict(
    conflict: &FileMoveConflict,
    apply_moves: impl FnOnce(&[(PathBuf, PathBuf)]) -> Result<(), String>,
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let destination = unique_destination(&conflict.destination_path);
    let move_pair = (conflict.source_path.clone(), destination);
    let completed = rename_files_with_rollback(std::slice::from_ref(&move_pair))?;
    if let Err(error) = apply_moves(&completed) {
        rollback_completed_file_moves(&completed);
        return Err(error);
    }
    Ok(completed)
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
    fn ready_file_moves_roll_back_when_state_apply_fails() {
        let root = temp_dir("wavecrate-file-move-execution-ready-rollback");
        let source_dir = root.join("source");
        let target_dir = root.join("target");
        fs::create_dir_all(&source_dir).expect("create source");
        fs::create_dir_all(&target_dir).expect("create target");
        let source = source_dir.join("kick.wav");
        let destination = target_dir.join("kick.wav");
        fs::write(&source, b"source").expect("write source");

        let result = execute_ready_file_moves(&[(source.clone(), destination.clone())], |_| {
            Err(String::from("state apply failed"))
        });

        assert_eq!(result, Err(String::from("state apply failed")));
        assert_eq!(fs::read(&source).expect("read source"), b"source");
        assert!(!destination.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn overwrite_conflict_restores_source_and_destination_when_state_apply_fails() {
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

        let result =
            execute_file_move_conflict(&conflict, FileMoveConflictResolution::Overwrite, |_| {
                Err(String::from("state apply failed"))
            });

        assert_eq!(result, Err(String::from("state apply failed")));
        assert_eq!(fs::read(&source).expect("read source"), b"source");
        assert_eq!(
            fs::read(&destination).expect("read destination"),
            b"destination"
        );
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
