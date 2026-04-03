use super::super::delete_recovery;
use super::ops;
use super::*;
use crate::app::controller::jobs::{FileOpMessage, FileOpResult, FolderDeleteResult, FolderRenameResult};
use crate::app::controller::undo::{UndoEntry, UndoExecution};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::AtomicBool};

impl AppController {
    pub(crate) fn delete_focused_folder(&mut self) {
        let Some(target) = self.focused_folder_path() else {
            self.set_status("Focus a folder to delete it", StatusTone::Info);
            return;
        };
        let Some(source) = self.current_source() else {
            self.set_status("Select a source first", StatusTone::Info);
            return;
        };
        if self.warn_if_retained_delete_path_busy(&source.id, &target, "deleting") {
            return;
        }
        if target.as_os_str().is_empty() {
            self.set_status("Root folder cannot be deleted", StatusTone::Info);
            return;
        }
        match self.remove_folder(&target) {
            Ok(true) => {}
            Ok(false) => {}
            Err(err) => self.set_status(err, StatusTone::Error),
        }
    }

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
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.runtime.jobs.start_file_ops(rx, cancel.clone());
        std::thread::spawn(move || {
            let result = run_folder_rename_job(source, target_path, new_relative, affected, cancel);
            let _ = tx.send(FileOpMessage::Finished(FileOpResult::FolderRename(result)));
        });
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
        if self.runtime.jobs.file_ops_in_progress() {
            return Err("File operation already in progress".to_string());
        }
        let target_path = target.to_path_buf();
        let next_focus = self.next_folder_focus_after_delete(target);
        let entries = self.folder_entries(target);
        if cfg!(test) {
            #[cfg(test)]
            {
                if self.runtime.fail_next_folder_delete_db {
                    let staging_root = source.root.join(delete_recovery::DELETE_STAGING_DIR);
                    let staged = delete_recovery::stage_folder_for_delete(
                        &absolute,
                        &staging_root,
                        &target_path,
                        &entries,
                    )?;
                    delete_recovery::rollback_staged_folder(
                        &staged,
                        &absolute,
                        &staging_root,
                        "Injected folder delete DB failure",
                    )?;
                    self.runtime.fail_next_folder_delete_db = false;
                    return Ok(true);
                }
                if self.runtime.fail_after_folder_delete_stage {
                    let staging_root = source.root.join(delete_recovery::DELETE_STAGING_DIR);
                    delete_recovery::stage_folder_for_delete(
                        &absolute,
                        &staging_root,
                        &target_path,
                        &entries,
                    )?;
                    self.runtime.fail_after_folder_delete_stage = false;
                    return Ok(true);
                }
                if self.runtime.fail_after_folder_delete_db_commit {
                    let staging_root = source.root.join(delete_recovery::DELETE_STAGING_DIR);
                    let staged = delete_recovery::stage_folder_for_delete(
                        &absolute,
                        &staging_root,
                        &target_path,
                        &entries,
                    )?;
                    let db = crate::sample_sources::SourceDatabase::open(&source.root)
                        .map_err(|err| format!("Database unavailable: {err}"))?;
                    let mut batch = db
                        .write_batch()
                        .map_err(|err| format!("Failed to start database update: {err}"))?;
                    for entry in &entries {
                        batch
                            .remove_file(&entry.relative_path)
                            .map_err(|err| format!("Failed to drop database row: {err}"))?;
                    }
                    batch
                        .commit()
                        .map_err(|err| format!("Failed to save folder delete: {err}"))?;
                    delete_recovery::mark_delete_retained(&staging_root, &staged.id)?;
                    self.runtime.fail_after_folder_delete_db_commit = false;
                    return Ok(true);
                }
            }
            self.begin_pending_file_mutation(&source.id, [target_path.clone()]);
            let result = run_folder_delete_job(
                source,
                target_path,
                entries,
                next_focus,
                Arc::new(AtomicBool::new(false)),
            );
            self.apply_file_op_result(FileOpResult::FolderDelete(result));
            return Ok(true);
        }
        self.begin_pending_file_mutation(&source.id, [target_path.clone()]);
        self.set_status(
            format!("Deleting folder {}...", target.display()),
            StatusTone::Busy,
        );
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.runtime.jobs.start_file_ops(rx, cancel.clone());
        std::thread::spawn(move || {
            let result = run_folder_delete_job(source, target_path, entries, next_focus, cancel);
            let _ = tx.send(FileOpMessage::Finished(FileOpResult::FolderDelete(result)));
        });
        Ok(true)
    }

    pub(crate) fn deleted_folder_undo_entry(
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

    pub(crate) fn apply_deleted_folder_state(
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
        self.ui.sources.folders.inline_edit = None;
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

fn run_folder_rename_job(
    source: SampleSource,
    old_folder: PathBuf,
    new_folder: PathBuf,
    affected: Vec<WavEntry>,
    cancel: Arc<AtomicBool>,
) -> FolderRenameResult {
    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        return FolderRenameResult {
            source_id: source.id,
            old_folder,
            new_folder,
            entries: Vec::new(),
            result: Err(String::from("Folder rename cancelled")),
        };
    }
    let absolute_old = source.root.join(&old_folder);
    let absolute_new = source.root.join(&new_folder);
    let result = fs::rename(&absolute_old, &absolute_new)
        .map_err(|err| format!("Failed to rename folder: {err}"))
        .and_then(|_| {
            let db = crate::sample_sources::SourceDatabase::open(&source.root)
                .map_err(|err| format!("Database unavailable: {err}"))?;
            let mut batch = db
                .write_batch()
                .map_err(|err| format!("Failed to start database update: {err}"))?;
            let mut entries = Vec::with_capacity(affected.len());
            for entry in &affected {
                let new_relative = new_folder.join(
                    entry.relative_path.strip_prefix(&old_folder).map_err(|_| {
                        format!("Folder entry missing expected prefix: {}", entry.relative_path.display())
                    })?,
                );
                batch
                    .remove_file(&entry.relative_path)
                    .map_err(|err| format!("Failed to drop old entry: {err}"))?;
                batch
                    .upsert_file(&new_relative, entry.file_size, entry.modified_ns)
                    .map_err(|err| format!("Failed to register renamed entry: {err}"))?;
                batch
                    .set_tag(&new_relative, entry.tag)
                    .map_err(|err| format!("Failed to copy tag: {err}"))?;
                batch
                    .set_looped(&new_relative, entry.looped)
                    .map_err(|err| format!("Failed to copy loop marker: {err}"))?;
                batch
                    .set_locked(&new_relative, entry.locked)
                    .map_err(|err| format!("Failed to copy keep lock: {err}"))?;
                if let Some(last_played_at) = entry.last_played_at {
                    batch
                        .set_last_played_at(&new_relative, last_played_at)
                        .map_err(|err| format!("Failed to copy playback age: {err}"))?;
                }
                entries.push(WavEntry {
                    relative_path: new_relative,
                    ..entry.clone()
                });
            }
            batch
                .commit()
                .map_err(|err| format!("Failed to save folder rename: {err}"))?;
            Ok(entries)
        });
    FolderRenameResult {
        source_id: source.id,
        old_folder,
        new_folder,
        entries: result.clone().unwrap_or_default(),
        result: result.map(|_| ()),
    }
}

fn run_folder_delete_job(
    source: SampleSource,
    relative_path: PathBuf,
    entries: Vec<WavEntry>,
    next_focus: Option<PathBuf>,
    cancel: Arc<AtomicBool>,
) -> FolderDeleteResult {
    let staging_root = source.root.join(delete_recovery::DELETE_STAGING_DIR);
    if cancel.load(std::sync::atomic::Ordering::Relaxed) {
        return FolderDeleteResult {
            source_id: source.id,
            source_root: source.root,
            relative_path,
            entries,
            staging_root,
            staged: None,
            next_focus,
            result: Err(String::from("Folder delete cancelled")),
        };
    }
    let absolute = source.root.join(&relative_path);
    let result = delete_recovery::stage_folder_for_delete(
        &absolute,
        &staging_root,
        &relative_path,
        &entries,
    )
    .and_then(|staged| {
        let db = crate::sample_sources::SourceDatabase::open(&source.root)
            .map_err(|err| format!("Database unavailable: {err}"))?;
        let mut batch = db
            .write_batch()
            .map_err(|err| format!("Failed to start database update: {err}"))?;
        for entry in &entries {
            batch
                .remove_file(&entry.relative_path)
                .map_err(|err| format!("Failed to drop database row: {err}"))?;
        }
        if let Err(err) = batch.commit() {
            let message = format!("Failed to save folder delete: {err}");
            delete_recovery::rollback_staged_folder(
                &staged,
                &absolute,
                &staging_root,
                &message,
            )?;
            return Err(message);
        }
        delete_recovery::mark_delete_retained(&staging_root, &staged.id)?;
        Ok(staged)
    });
    FolderDeleteResult {
        source_id: source.id,
        source_root: source.root,
        relative_path,
        entries,
        staging_root,
        staged: result.as_ref().ok().cloned(),
        next_focus,
        result: result.map(|_| ()),
    }
}
