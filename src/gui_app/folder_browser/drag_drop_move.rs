use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use super::{
    FileMoveConflict, FileMoveConflictBatch, FileMoveConflictResolution, FileMoveConflictView,
    FolderBrowserState, FolderDropResult, plural,
};

#[derive(Debug, Default)]
struct FileMovePlan {
    ready: Vec<(PathBuf, PathBuf)>,
    conflicts: Vec<FileMoveConflict>,
}

#[derive(Debug)]
struct OverwriteBackup {
    destination_path: PathBuf,
    backup_path: PathBuf,
}

impl FolderBrowserState {
    pub(super) fn move_folder_to_folder(
        &mut self,
        folder_id: &str,
        target_folder_id: &str,
    ) -> Result<FolderDropResult, String> {
        if self.rename_active() {
            return Err(String::from("Finish rename before moving a folder"));
        }
        if self.selected_folder_is_source_root_id(folder_id) {
            return Err(String::from("Root folder cannot be moved"));
        }
        let source_folder = self
            .find_folder(folder_id)
            .cloned()
            .ok_or_else(|| String::from("Folder move failed: source folder is missing"))?;
        let target_folder = self
            .find_folder(target_folder_id)
            .cloned()
            .ok_or_else(|| String::from("Folder move failed: target folder is missing"))?;
        let old_path = PathBuf::from(&source_folder.id);
        let target_path = PathBuf::from(&target_folder.id);
        if target_path.starts_with(&old_path) {
            return Err(String::from(
                "Folder move failed: cannot move a folder into itself",
            ));
        }
        let Some(folder_name) = old_path.file_name() else {
            return Err(String::from(
                "Folder move failed: source folder has no name",
            ));
        };
        let new_path = target_path.join(folder_name);
        if old_path == new_path {
            return Ok(FolderDropResult {
                moved_paths: Vec::new(),
                status: Some(String::from("Folder move unchanged")),
            });
        }
        if new_path.exists() {
            return Err(format!(
                "Folder move failed: {} already exists",
                folder_name.to_string_lossy()
            ));
        }
        fs::rename(&old_path, &new_path).map_err(|error| format!("Folder move failed: {error}"))?;
        if let Err(error) = self.relocate_moved_folder(&old_path, &new_path, &target_path) {
            let _ = fs::rename(&new_path, &old_path);
            return Err(error);
        }
        Ok(FolderDropResult {
            moved_paths: vec![(old_path, new_path.clone())],
            status: Some(format!(
                "Moved folder {}",
                new_path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| new_path.display().to_string())
            )),
        })
    }

    pub(super) fn move_files_to_folder(
        &mut self,
        file_ids: &[String],
        target_folder_id: &str,
    ) -> Result<FolderDropResult, String> {
        if self.rename_active() {
            return Err(String::from("Finish rename before moving files"));
        }
        let target_folder = self
            .find_folder(target_folder_id)
            .cloned()
            .ok_or_else(|| String::from("File move failed: target folder is missing"))?;
        let target_path = PathBuf::from(&target_folder.id);
        if !target_path.is_dir() {
            return Err(String::from("File move failed: target folder is missing"));
        }
        let plan = file_move_plan_to_folder(file_ids, &target_path)?;
        if plan.ready.is_empty() && plan.conflicts.is_empty() {
            return Ok(FolderDropResult {
                moved_paths: Vec::new(),
                status: Some(String::from("File move unchanged")),
            });
        }
        let completed = if plan.ready.is_empty() {
            Vec::new()
        } else {
            let completed = rename_files_with_rollback(&plan.ready)?;
            if let Err(error) = self.relocate_moved_files(&completed, &target_path) {
                rollback_completed_file_moves(&completed);
                return Err(error);
            }
            completed
        };
        if !plan.conflicts.is_empty() {
            self.pending_file_move_conflicts = Some(FileMoveConflictBatch {
                target_folder: target_path,
                conflicts: plan.conflicts,
                current_index: 0,
                resolved_count: 0,
                skipped_count: 0,
            });
        }
        let status = file_move_status(completed.len(), self.pending_file_move_conflict_count());
        Ok(FolderDropResult {
            moved_paths: completed.clone(),
            status: Some(status),
        })
    }

    pub(super) fn move_extracted_file_to_folder(
        &mut self,
        path: &Path,
        target_folder_id: &str,
    ) -> Result<FolderDropResult, String> {
        if self.rename_active() {
            return Err(String::from("Finish rename before moving files"));
        }
        if !path.is_file() {
            return Err(format!(
                "Extraction move failed: {} is missing",
                path.file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string())
            ));
        }
        let target_folder = self
            .find_folder(target_folder_id)
            .cloned()
            .ok_or_else(|| String::from("Extraction move failed: target folder is missing"))?;
        let target_path = PathBuf::from(&target_folder.id);
        if !target_path.is_dir() {
            return Err(String::from(
                "Extraction move failed: target folder is missing",
            ));
        }
        if path.parent() == Some(target_path.as_path()) {
            return Ok(FolderDropResult {
                moved_paths: Vec::new(),
                status: Some(String::from("Extraction kept in current folder")),
            });
        }
        let Some(file_name) = path.file_name() else {
            return Err(String::from("Extraction move failed: file has no name"));
        };
        let new_path = unique_destination(&target_path.join(file_name));
        fs::rename(path, &new_path).map_err(|error| format!("Extraction move failed: {error}"))?;
        let completed = vec![(path.to_path_buf(), new_path.clone())];
        let previous_selected_folder = self.selected_folder.clone();
        let previous_selected_file = self.selected_file.clone();
        let previous_selected_file_ids = self.selected_file_ids.clone();
        let previous_file_view_controller = self.file_view_controller.clone();
        if let Err(error) = self.relocate_moved_files(&completed, &target_path) {
            rollback_completed_file_moves(&completed);
            return Err(error);
        }
        self.selected_folder = previous_selected_folder;
        self.selected_file = previous_selected_file;
        self.selected_file_ids = previous_selected_file_ids;
        self.file_view_controller = previous_file_view_controller;
        Ok(FolderDropResult {
            moved_paths: completed,
            status: Some(format!(
                "Extracted {}",
                new_path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| new_path.display().to_string())
            )),
        })
    }

    pub(in crate::gui_app) fn pending_file_move_conflict_view(
        &self,
    ) -> Option<FileMoveConflictView> {
        let batch = self.pending_file_move_conflicts.as_ref()?;
        let conflict = batch.conflicts.get(batch.current_index)?;
        Some(FileMoveConflictView {
            source_path: conflict.source_path.clone(),
            destination_path: conflict.destination_path.clone(),
            file_name: conflict
                .destination_path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| conflict.destination_path.display().to_string()),
            destination_folder: batch.target_folder.display().to_string(),
            current_number: batch.current_index + 1,
            total_count: batch.conflicts.len(),
        })
    }

    pub(in crate::gui_app) fn pending_file_move_conflict_count(&self) -> usize {
        self.pending_file_move_conflicts
            .as_ref()
            .map(|batch| batch.conflicts.len().saturating_sub(batch.current_index))
            .unwrap_or(0)
    }

    pub(in crate::gui_app) fn cancel_file_move_conflicts(&mut self) -> Option<String> {
        let batch = self.pending_file_move_conflicts.take()?;
        let remaining = batch.conflicts.len().saturating_sub(batch.current_index);
        Some(format!(
            "Skipped {} file conflict{}",
            remaining,
            plural(remaining)
        ))
    }

    pub(in crate::gui_app) fn resolve_next_file_move_conflict(
        &mut self,
        resolution: FileMoveConflictResolution,
    ) -> Result<FolderDropResult, String> {
        let Some(mut batch) = self.pending_file_move_conflicts.take() else {
            return Ok(FolderDropResult::default());
        };
        let Some(conflict) = batch.conflicts.get(batch.current_index).cloned() else {
            return Ok(FolderDropResult {
                moved_paths: Vec::new(),
                status: Some(String::from("No file move conflicts pending")),
            });
        };

        let mut moved_paths = Vec::new();
        match resolution {
            FileMoveConflictResolution::Overwrite => {
                let backup = match move_existing_destination_to_backup(&conflict.destination_path) {
                    Ok(backup) => backup,
                    Err(error) => {
                        self.pending_file_move_conflicts = Some(batch);
                        return Err(error);
                    }
                };
                if let Err(error) =
                    move_file_over_backup(&conflict.source_path, &conflict.destination_path)
                {
                    restore_overwrite_backup(&backup);
                    self.pending_file_move_conflicts = Some(batch);
                    return Err(error);
                }
                let completed = vec![(conflict.source_path.clone(), conflict.destination_path)];
                if let Err(error) = self.relocate_moved_files(&completed, &batch.target_folder) {
                    rollback_overwrite_move(&completed[0], &backup);
                    self.pending_file_move_conflicts = Some(batch);
                    return Err(error);
                }
                remove_overwrite_backup(&backup);
                moved_paths = completed;
                batch.resolved_count += 1;
            }
            FileMoveConflictResolution::Rename => {
                let destination = unique_destination(&conflict.destination_path);
                let move_pair = (conflict.source_path, destination);
                let completed = match rename_files_with_rollback(std::slice::from_ref(&move_pair)) {
                    Ok(completed) => completed,
                    Err(error) => {
                        self.pending_file_move_conflicts = Some(batch);
                        return Err(error);
                    }
                };
                if let Err(error) = self.relocate_moved_files(&completed, &batch.target_folder) {
                    rollback_completed_file_moves(&completed);
                    self.pending_file_move_conflicts = Some(batch);
                    return Err(error);
                }
                moved_paths = completed;
                batch.resolved_count += 1;
            }
            FileMoveConflictResolution::Skip => {
                batch.skipped_count += 1;
            }
        }

        batch.current_index += 1;
        let status = conflict_resolution_status(&batch, resolution, moved_paths.len());
        if batch.current_index < batch.conflicts.len() {
            self.pending_file_move_conflicts = Some(batch);
        }
        Ok(FolderDropResult {
            moved_paths,
            status: Some(status),
        })
    }
}

fn file_move_plan_to_folder(
    file_ids: &[String],
    target_path: &Path,
) -> Result<FileMovePlan, String> {
    let mut plan = FileMovePlan::default();
    let mut seen = HashSet::new();
    for id in file_ids {
        if !seen.insert(id.clone()) {
            continue;
        }
        let old_path = PathBuf::from(id);
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
        let new_path = target_path.join(file_name);
        if new_path.exists() {
            plan.conflicts.push(FileMoveConflict {
                source_path: old_path,
                destination_path: new_path,
            });
        } else {
            plan.ready.push((old_path, new_path));
        }
    }
    Ok(plan)
}

fn rename_files_with_rollback(
    moves: &[(PathBuf, PathBuf)],
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let mut completed = Vec::new();
    for (old_path, new_path) in moves {
        if let Err(error) = fs::rename(old_path, new_path) {
            rollback_completed_file_moves(&completed);
            return Err(format!("File move failed: {error}"));
        }
        completed.push((old_path.clone(), new_path.clone()));
    }
    Ok(completed)
}

fn rollback_completed_file_moves(completed: &[(PathBuf, PathBuf)]) {
    for (moved_old, moved_new) in completed.iter().rev() {
        let _ = fs::rename(moved_new, moved_old);
    }
}

fn unique_destination(first_candidate: &Path) -> PathBuf {
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

fn file_move_status(moved_count: usize, conflict_count: usize) -> String {
    match (moved_count, conflict_count) {
        (0, 0) => String::from("File move unchanged"),
        (0, conflicts) => format!("Resolve {} file conflict{}", conflicts, plural(conflicts)),
        (moved, 0) => format!("Moved {} file{}", moved, plural(moved)),
        (moved, conflicts) => format!(
            "Moved {} file{}; resolve {} conflict{}",
            moved,
            plural(moved),
            conflicts,
            plural(conflicts)
        ),
    }
}

fn conflict_resolution_status(
    batch: &FileMoveConflictBatch,
    resolution: FileMoveConflictResolution,
    moved_count: usize,
) -> String {
    let remaining = batch
        .conflicts
        .len()
        .saturating_sub(batch.current_index + 1);
    if remaining > 0 {
        return format!(
            "{}; {} conflict{} remaining",
            conflict_resolution_action_status(resolution, moved_count),
            remaining,
            plural(remaining)
        );
    }
    format!(
        "Resolved {} file conflict{}; skipped {}",
        batch.resolved_count,
        plural(batch.resolved_count),
        batch.skipped_count
    )
}

fn conflict_resolution_action_status(
    resolution: FileMoveConflictResolution,
    moved_count: usize,
) -> &'static str {
    match (resolution, moved_count) {
        (FileMoveConflictResolution::Overwrite, 1) => "Overwrote conflicting file",
        (FileMoveConflictResolution::Rename, 1) => "Moved file with a new name",
        (FileMoveConflictResolution::Skip, _) => "Skipped conflicting file",
        _ => "Resolved file conflict",
    }
}

fn move_existing_destination_to_backup(destination_path: &Path) -> Result<OverwriteBackup, String> {
    let backup_path = unique_overwrite_backup_path(destination_path);
    fs::rename(destination_path, &backup_path)
        .map_err(|error| format!("File overwrite failed: {error}"))?;
    Ok(OverwriteBackup {
        destination_path: destination_path.to_path_buf(),
        backup_path,
    })
}

fn move_file_over_backup(source_path: &Path, destination_path: &Path) -> Result<(), String> {
    fs::rename(source_path, destination_path)
        .map_err(|error| format!("File overwrite failed: {error}"))
}

fn rollback_overwrite_move(completed: &(PathBuf, PathBuf), backup: &OverwriteBackup) {
    let (source_path, destination_path) = completed;
    let _ = fs::rename(destination_path, source_path);
    restore_overwrite_backup(backup);
}

fn restore_overwrite_backup(backup: &OverwriteBackup) {
    let _ = fs::rename(&backup.backup_path, &backup.destination_path);
}

fn remove_overwrite_backup(backup: &OverwriteBackup) {
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
