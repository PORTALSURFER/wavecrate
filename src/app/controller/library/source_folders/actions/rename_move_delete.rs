use super::super::delete_recovery;
use super::ops;
use super::*;
use crate::app::controller::undo::{UndoEntry, UndoExecution};
use std::fs;
use std::path::{Path, PathBuf};

impl AppController {
    pub(crate) fn delete_focused_folder(&mut self) {
        let Some(target) = self.focused_folder_path() else {
            self.set_status("Focus a folder to delete it", StatusTone::Info);
            return;
        };
        if target.as_os_str().is_empty() {
            self.set_status("Root folder cannot be deleted", StatusTone::Info);
            return;
        }
        match self.remove_folder(&target) {
            Ok(true) => self.set_status(
                format!("Deleted folder {}", target.display()),
                StatusTone::Info,
            ),
            Ok(false) => {}
            Err(err) => self.set_status(err, StatusTone::Error),
        }
    }

    pub(crate) fn rename_folder(&mut self, target: &Path, new_name: &str) -> Result<(), String> {
        let new_relative = ops::rename_target(target, new_name)?;
        let source = self
            .current_source()
            .ok_or_else(|| "Select a source first".to_string())?;
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
        let affected = self.folder_entries(target);
        fs::rename(&absolute_old, &absolute_new)
            .map_err(|err| format!("Failed to rename folder: {err}"))?;
        self.rewrite_entries_for_folder(&source, target, &new_relative, &affected)?;
        self.remap_folder_state(target, &new_relative);
        self.remap_manual_folders(target, &new_relative);
        self.refresh_folder_browser();
        self.set_status(
            format!("Renamed folder to {}", new_relative.display()),
            StatusTone::Info,
        );
        Ok(())
    }

    fn remove_folder(&mut self, target: &Path) -> Result<bool, String> {
        let source = self
            .current_source()
            .ok_or_else(|| "Select a source first".to_string())?;
        let absolute = source.root.join(target);
        if !absolute.exists() {
            return Err(format!("Folder not found: {}", target.display()));
        }
        if !self.confirm_folder_delete(target) {
            return Ok(false);
        }
        let before = self.capture_meaningful_ui_snapshot();
        let next_focus = self.next_folder_focus_after_delete(target);
        let entries = self.folder_entries(target);
        let staging_root = source.root.join(delete_recovery::DELETE_STAGING_DIR);
        let staged =
            delete_recovery::stage_folder_for_delete(&absolute, &staging_root, target, &entries)?;
        #[cfg(test)]
        if self.runtime.fail_after_folder_delete_stage {
            self.runtime.fail_after_folder_delete_stage = false;
            return Err("Simulated crash after staging".to_string());
        }
        #[cfg(test)]
        if self.runtime.fail_next_folder_delete_db {
            self.runtime.fail_next_folder_delete_db = false;
            return delete_recovery::rollback_staged_folder(
                &staged,
                &absolute,
                &staging_root,
                "Simulated database failure",
            )
            .map(|_| false);
        }
        if let Err(err) = self.remove_folder_entries_from_db(&source, &entries) {
            return delete_recovery::rollback_staged_folder(
                &staged,
                &absolute,
                &staging_root,
                &err,
            )
            .map(|_| false);
        }
        delete_recovery::mark_delete_retained(&staging_root, &staged.id)?;
        self.apply_deleted_folder_state(&source, target, next_focus.as_deref(), &entries);
        #[cfg(test)]
        if self.runtime.fail_after_folder_delete_db_commit {
            self.runtime.fail_after_folder_delete_db_commit = false;
            return Err("Simulated crash after database commit".to_string());
        }
        let after = self.capture_meaningful_ui_snapshot();
        let entry = self.deleted_folder_undo_entry(
            source.clone(),
            staging_root,
            staged,
            entries,
            next_focus,
        );
        self.push_undo_entry(AppController::attach_meaningful_ui_restore(
            entry, before, after,
        ));
        Ok(true)
    }

    fn deleted_folder_undo_entry(
        &self,
        source: SampleSource,
        staging_root: PathBuf,
        staged: delete_recovery::DeleteStagingInfo,
        entries: Vec<WavEntry>,
        next_focus: Option<PathBuf>,
    ) -> UndoEntry<AppController> {
        let label = format!("Delete folder {}", staged.original_relative.display());
        let undo_source = source.clone();
        let undo_staging_root = staging_root.clone();
        let undo_staged = staged.clone();
        let undo_entries = entries.clone();
        let redo_source = source;
        let redo_staging_root = staging_root;
        let redo_staged = staged;
        let redo_entries = entries;
        UndoEntry::new(
            label,
            move |controller: &mut AppController| {
                controller.restore_deleted_folder(
                    &undo_source,
                    &undo_staging_root,
                    &undo_staged,
                    &undo_entries,
                )?;
                Ok(UndoExecution::Applied)
            },
            move |controller: &mut AppController| {
                controller.redelete_restored_folder(
                    &redo_source,
                    &redo_staging_root,
                    &redo_staged,
                    &redo_entries,
                    next_focus.as_deref(),
                )?;
                Ok(UndoExecution::Applied)
            },
        )
    }

    fn restore_deleted_folder(
        &mut self,
        source: &SampleSource,
        staging_root: &Path,
        staged: &delete_recovery::DeleteStagingInfo,
        entries: &[WavEntry],
    ) -> Result<(), String> {
        let absolute = source.root.join(&staged.original_relative);
        if absolute.exists() {
            return Err(format!(
                "Cannot undo delete because folder already exists: {}",
                staged.original_relative.display()
            ));
        }
        delete_recovery::restore_deleted_folder(staged, &absolute, staging_root)?;
        self.restore_folder_entries_in_db(source, entries)?;
        for entry in entries {
            self.insert_cached_entry(source, entry.clone());
        }
        self.refresh_folder_browser();
        Ok(())
    }

    fn redelete_restored_folder(
        &mut self,
        source: &SampleSource,
        staging_root: &Path,
        staged: &delete_recovery::DeleteStagingInfo,
        entries: &[WavEntry],
        next_focus: Option<&Path>,
    ) -> Result<(), String> {
        let absolute = source.root.join(&staged.original_relative);
        if !absolute.exists() {
            return Err(format!(
                "Cannot redo delete because folder is missing: {}",
                staged.original_relative.display()
            ));
        }
        delete_recovery::restage_deleted_folder(&absolute, staging_root, staged, entries)?;
        if let Err(err) = self.remove_folder_entries_from_db(source, entries) {
            return delete_recovery::rollback_staged_folder(staged, &absolute, staging_root, &err);
        }
        delete_recovery::mark_delete_retained(staging_root, &staged.id)?;
        self.apply_deleted_folder_state(source, &staged.original_relative, next_focus, entries);
        Ok(())
    }

    fn remove_folder_entries_from_db(
        &mut self,
        source: &SampleSource,
        entries: &[WavEntry],
    ) -> Result<(), String> {
        if entries.is_empty() {
            return Ok(());
        }
        let db = self
            .database_for(source)
            .map_err(|err| format!("Database unavailable: {err}"))?;
        let mut batch = db
            .write_batch()
            .map_err(|err| format!("Failed to start database update: {err}"))?;
        for entry in entries {
            batch
                .remove_file(&entry.relative_path)
                .map_err(|err| format!("Failed to drop database row: {err}"))?;
        }
        batch
            .commit()
            .map_err(|err| format!("Failed to save folder delete: {err}"))
    }

    pub(crate) fn restore_folder_entries_in_db(
        &mut self,
        source: &SampleSource,
        entries: &[WavEntry],
    ) -> Result<(), String> {
        if entries.is_empty() {
            return Ok(());
        }
        let db = self
            .database_for(source)
            .map_err(|err| format!("Database unavailable: {err}"))?;
        let mut batch = db
            .write_batch()
            .map_err(|err| format!("Failed to start database update: {err}"))?;
        for entry in entries {
            if let Some(content_hash) = entry.content_hash.as_deref() {
                batch
                    .upsert_file_with_hash(
                        &entry.relative_path,
                        entry.file_size,
                        entry.modified_ns,
                        content_hash,
                    )
                    .map_err(|err| format!("Failed to restore database row: {err}"))?;
            } else {
                batch
                    .upsert_file(&entry.relative_path, entry.file_size, entry.modified_ns)
                    .map_err(|err| format!("Failed to restore database row: {err}"))?;
            }
            batch
                .set_tag(&entry.relative_path, entry.tag)
                .map_err(|err| format!("Failed to restore tag: {err}"))?;
            batch
                .set_looped(&entry.relative_path, entry.looped)
                .map_err(|err| format!("Failed to restore loop marker: {err}"))?;
            batch
                .set_locked(&entry.relative_path, entry.locked)
                .map_err(|err| format!("Failed to restore keep lock: {err}"))?;
            if let Some(last_played_at) = entry.last_played_at {
                batch
                    .set_last_played_at(&entry.relative_path, last_played_at)
                    .map_err(|err| format!("Failed to restore playback age: {err}"))?;
            }
        }
        batch
            .commit()
            .map_err(|err| format!("Failed to restore folder delete state: {err}"))
    }

    fn apply_deleted_folder_state(
        &mut self,
        source: &SampleSource,
        target: &Path,
        next_focus: Option<&Path>,
        entries: &[WavEntry],
    ) {
        for entry in entries {
            self.prune_cached_sample(source, &entry.relative_path);
        }
        self.update_manual_folders(|set| {
            set.retain(|path| !path.starts_with(target));
        });
        self.prune_folder_state(target);
        self.refresh_folder_browser();
        if let Some(path) = next_focus {
            self.focus_folder_by_path(path);
        } else {
            self.ui.sources.folders.focused = None;
            self.ui.sources.folders.scroll_to = None;
        }
        self.ui.sources.folders.pending_action = None;
        self.ui.sources.folders.new_folder = None;
    }

    fn next_folder_focus_after_delete(&self, target: &Path) -> Option<PathBuf> {
        let rows = &self.ui.sources.folders.rows;
        let target_index = rows.iter().position(|row| row.path == target)?;
        let mut after = rows
            .iter()
            .skip(target_index + 1)
            .filter(|row| !row.path.starts_with(target));
        if let Some(row) = after.next() {
            return Some(row.path.clone());
        }
        rows.iter()
            .take(target_index)
            .rev()
            .find(|row| !row.path.starts_with(target))
            .map(|row| row.path.clone())
    }

    /// Remap folder selection state after a folder move within the current source.
    pub(crate) fn remap_folder_state(&mut self, old: &Path, new: &Path) {
        let Some(model) = self.current_folder_model_mut() else {
            return;
        };
        ops::remap_path_set(&mut model.selected, old, new);
        ops::remap_path_set(&mut model.negated, old, new);
        ops::remap_path_set(&mut model.expanded, old, new);
        ops::remap_path_set(&mut model.available, old, new);
        ops::remap_path_set(&mut model.disk_folders, old, new);
        ops::remap_path_map(&mut model.hotkeys, old, new);
        model.focused = ops::remap_path_option(model.focused.take(), old, new);
        model.selection_anchor = ops::remap_path_option(model.selection_anchor.take(), old, new);
        self.ui.sources.folders.last_focused_path =
            ops::remap_path_option(self.ui.sources.folders.last_focused_path.take(), old, new);
    }

    fn prune_folder_state(&mut self, target: &Path) {
        let Some(model) = self.current_folder_model_mut() else {
            return;
        };
        model.selected.retain(|path| !path.starts_with(target));
        model.negated.retain(|path| !path.starts_with(target));
        model.expanded.retain(|path| !path.starts_with(target));
        model.available.retain(|path| !path.starts_with(target));
        model.disk_folders.retain(|path| !path.starts_with(target));
        model.hotkeys.retain(|_, path| !path.starts_with(target));
        if model
            .focused
            .as_ref()
            .is_some_and(|path| path.starts_with(target))
        {
            model.focused = None;
        }
        if model
            .selection_anchor
            .as_ref()
            .is_some_and(|path| path.starts_with(target))
        {
            model.selection_anchor = None;
        }
        if self
            .ui
            .sources
            .folders
            .last_focused_path
            .as_ref()
            .is_some_and(|path| path.starts_with(target))
        {
            self.ui.sources.folders.last_focused_path = None;
        }
    }
}
