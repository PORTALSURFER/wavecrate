use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use super::{FolderBrowserState, FolderDropResult, plural};

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
        let moves = file_moves_to_folder(file_ids, &target_path)?;
        if moves.is_empty() {
            return Ok(FolderDropResult {
                moved_paths: Vec::new(),
                status: Some(String::from("File move unchanged")),
            });
        }
        let completed = rename_files_with_rollback(&moves)?;
        if let Err(error) = self.relocate_moved_files(&completed, &target_path) {
            rollback_completed_file_moves(&completed);
            return Err(error);
        }
        Ok(FolderDropResult {
            moved_paths: completed.clone(),
            status: Some(format!(
                "Moved {} file{}",
                completed.len(),
                plural(completed.len())
            )),
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
}

fn file_moves_to_folder(
    file_ids: &[String],
    target_path: &Path,
) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let mut moves = Vec::new();
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
            return Err(format!(
                "File move failed: {} already exists",
                file_name.to_string_lossy()
            ));
        }
        moves.push((old_path, new_path));
    }
    Ok(moves)
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
