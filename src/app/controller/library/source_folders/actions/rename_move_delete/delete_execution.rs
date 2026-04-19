//! Folder delete execution and test-fault helpers.

use super::*;

impl AppController {
    pub(super) fn remove_folder(&mut self, target: &Path) -> Result<bool, String> {
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
            if self.try_injected_folder_delete_failure(
                &source,
                &absolute,
                &target_path,
                &entries,
            )? {
                return Ok(true);
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
        let pending_source_id = source.id.clone();
        let pending_path = target.to_path_buf();
        if let Err(err) = self.runtime.jobs.begin_one_shot_file_op(move |cancel| {
            FileOpResult::FolderDelete(run_folder_delete_job(
                source,
                target_path,
                entries,
                next_focus,
                cancel,
            ))
        }) {
            self.finish_pending_file_mutation(&pending_source_id, [pending_path]);
            return Err(err);
        }
        Ok(true)
    }

    fn try_injected_folder_delete_failure(
        &mut self,
        source: &SampleSource,
        absolute: &Path,
        target_path: &Path,
        entries: &[WavEntry],
    ) -> Result<bool, String> {
        #[cfg(not(test))]
        {
            let _ = (source, absolute, target_path, entries);
            return Ok(false);
        }
        #[cfg(test)]
        {
            if self.runtime.fail_next_folder_delete_db {
                let staging_root = source.root.join(delete_recovery::DELETE_STAGING_DIR);
                let staged = delete_recovery::stage_folder_for_delete(
                    absolute,
                    &staging_root,
                    target_path,
                    entries,
                )?;
                delete_recovery::rollback_staged_folder(
                    &staged,
                    absolute,
                    &staging_root,
                    "Injected folder delete DB failure",
                )?;
                self.runtime.fail_next_folder_delete_db = false;
                return Ok(true);
            }
            if self.runtime.fail_after_folder_delete_stage {
                let staging_root = source.root.join(delete_recovery::DELETE_STAGING_DIR);
                delete_recovery::stage_folder_for_delete(
                    absolute,
                    &staging_root,
                    target_path,
                    entries,
                )?;
                self.runtime.fail_after_folder_delete_stage = false;
                return Ok(true);
            }
            if self.runtime.fail_after_folder_delete_db_commit {
                let staging_root = source.root.join(delete_recovery::DELETE_STAGING_DIR);
                let staged = delete_recovery::stage_folder_for_delete(
                    absolute,
                    &staging_root,
                    target_path,
                    entries,
                )?;
                let db = crate::sample_sources::SourceDatabase::open(&source.root)
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
                    .map_err(|err| format!("Failed to save folder delete: {err}"))?;
                delete_recovery::mark_delete_retained(&staging_root, &staged.id)?;
                self.runtime.fail_after_folder_delete_db_commit = false;
                return Ok(true);
            }
            Ok(false)
        }
    }
}

/// Execute the background folder delete job, retaining staged content for explicit recovery.
pub(super) fn run_folder_delete_job(
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
            delete_recovery::rollback_staged_folder(&staged, &absolute, &staging_root, &message)?;
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
