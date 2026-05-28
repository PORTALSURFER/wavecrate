//! Folder rename/delete controller entrypoints and state updates.

use super::super::delete_recovery;
use super::ops;
use super::*;
use crate::app::controller::jobs::{FileOpResult, FolderDeleteResult, FolderRenameResult};
use crate::app::controller::undo::{UndoEntry, UndoExecution};
use crate::app::state::FolderActionPrompt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::AtomicBool};

mod delete_execution;
mod delete_recovery_state;
mod rename_execution;

use self::rename_execution::run_folder_rename_job;

impl AppController {
    /// Open a confirmation prompt for deleting the currently focused folder.
    pub(crate) fn request_delete_focused_folder(&mut self) -> bool {
        let Some(target) = self.focused_folder_path() else {
            self.set_status("Focus a folder to delete it", StatusTone::Info);
            return false;
        };
        self.ui.sources.folders.pending_action = Some(FolderActionPrompt::Delete { target });
        true
    }

    /// Confirm the active folder delete prompt.
    pub(crate) fn apply_pending_folder_delete(&mut self) -> bool {
        let Some(FolderActionPrompt::Delete { target }) =
            self.ui.sources.folders.pending_action.clone()
        else {
            return false;
        };
        self.ui.sources.folders.pending_action = None;
        self.delete_folder_at_path(&target);
        true
    }

    /// Cancel the active folder delete prompt.
    pub(crate) fn cancel_pending_folder_delete(&mut self) -> bool {
        if matches!(
            self.ui.sources.folders.pending_action,
            Some(FolderActionPrompt::Delete { .. })
        ) {
            self.ui.sources.folders.pending_action = None;
            return true;
        }
        false
    }

    /// Delete the currently focused folder after validating recovery and root-folder constraints.
    pub(crate) fn delete_focused_folder(&mut self) {
        let Some(target) = self.focused_folder_path() else {
            self.set_status("Focus a folder to delete it", StatusTone::Info);
            return;
        };
        self.delete_folder_at_path(&target);
    }

    fn delete_folder_at_path(&mut self, target: &Path) {
        let Some(source) = self.current_source() else {
            self.set_status("Select a source first", StatusTone::Info);
            return;
        };
        if self.warn_if_retained_delete_path_busy(&source.id, target, "deleting") {
            return;
        }
        if target.as_os_str().is_empty() {
            self.set_status("Root folder cannot be deleted", StatusTone::Info);
            return;
        }
        match self.remove_folder(target) {
            Ok(true) => {}
            Ok(false) => {}
            Err(err) => self.set_status(err, StatusTone::Error),
        }
    }

    /// Rename `target` to `new_name` while preserving undo and recovery metadata.
    pub(crate) fn rename_folder(&mut self, target: &Path, new_name: &str) -> Result<(), String> {
        let new_relative = ops::rename_target(target, new_name)?;
        let source = self
            .current_source()
            .ok_or_else(|| "Select a source first".to_string())?;
        if self.warn_if_retained_delete_path_busy(&source.id, target, "renaming") {
            return Err("Folder is busy with retained delete recovery".to_string());
        }
        if target == new_relative {
            return Ok(());
        }
        let absolute_old = source.root.join(target);
        let absolute_new = source.root.join(&new_relative);
        if !absolute_old.exists() {
            return Err(format!("Folder not found: {}", target.display()));
        }
        if absolute_new.exists() {
            return Err(format!("Folder already exists: {}", new_relative.display()));
        }
        if self.runtime.jobs.file_ops_in_progress() {
            return Err("File operation already in progress".to_string());
        }
        let target_path = target.to_path_buf();
        let affected = self.folder_entries(target);
        if cfg!(test) {
            self.begin_pending_file_mutation(&source.id, [target_path.clone()]);
            let result = run_folder_rename_job(
                source,
                target_path,
                new_relative,
                affected,
                Arc::new(AtomicBool::new(false)),
            );
            self.apply_file_op_result(FileOpResult::FolderRename(result));
            return Ok(());
        }
        self.begin_pending_file_mutation(&source.id, [target_path.clone()]);
        self.set_status(
            format!("Renaming folder {}...", target.display()),
            StatusTone::Busy,
        );
        let pending_source_id = source.id.clone();
        let pending_path = target.to_path_buf();
        if let Err(err) = self.runtime.jobs.begin_one_shot_file_op(move |cancel| {
            FileOpResult::FolderRename(run_folder_rename_job(
                source,
                target_path,
                new_relative,
                affected,
                cancel,
            ))
        }) {
            self.finish_pending_file_mutation(&pending_source_id, [pending_path]);
            return Err(err);
        }
        Ok(())
    }
}
