use super::super::move_transaction::{
    load_sample_move_metadata, prepare_staged_move, remove_move_journal_entry,
    rollback_staged_move_to_source,
};
use crate::app::controller::jobs::{
    FileOpMessage, FolderEntryMove, FolderMoveRequest, FolderMoveResult, FolderSampleMoveRequest,
    FolderSampleMoveResult,
};
use crate::sample_sources::db::file_ops_journal;
use crate::sample_sources::{SourceDatabase, SourceId};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
};
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

/// Folder-level move worker implementation split from the sample-move worker.
mod folder_move_task;

/// Delegate folder-level move execution to the dedicated folder worker module.
pub(super) fn run_folder_move_task(
    request: FolderMoveRequest,
    cancel: Arc<AtomicBool>,
    sender: Option<&Sender<FileOpMessage>>,
) -> FolderMoveResult {
    folder_move_task::run_folder_move_task(request, cancel, sender)
}

#[cfg(test)]
/// Optional one-shot hook used by tests to force deterministic timing around DB writes.
type BeforeFolderSampleBatchHook = Box<dyn FnMut() + Send>;

#[cfg(test)]
/// Global storage for the optional pre-batch hook used by folder-sample move tests.
static BEFORE_FOLDER_SAMPLE_BATCH_HOOK: OnceLock<Mutex<Option<BeforeFolderSampleBatchHook>>> =
    OnceLock::new();

#[cfg(test)]
/// Invoke and clear the one-shot pre-batch hook when tests configure one.
fn run_before_folder_sample_batch_hook() {
    if let Some(hook_slot) = BEFORE_FOLDER_SAMPLE_BATCH_HOOK.get()
        && let Ok(mut guard) = hook_slot.lock()
        && let Some(mut hook) = guard.take()
    {
        hook();
    }
}

/// Configure an optional test hook invoked immediately before folder-sample DB writes.
#[cfg(test)]
pub(super) fn set_before_folder_sample_batch_hook(hook: Option<BeforeFolderSampleBatchHook>) {
    let hook_slot = BEFORE_FOLDER_SAMPLE_BATCH_HOOK.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = hook_slot.lock() {
        *guard = hook;
    }
}

/// Execute a background batch of sample moves inside one source folder.
pub(super) fn run_folder_sample_move_task(
    source_id: SourceId,
    source_root: PathBuf,
    requests: Vec<FolderSampleMoveRequest>,
    mut errors: Vec<String>,
    cancel: Arc<AtomicBool>,
    sender: Option<&Sender<FileOpMessage>>,
) -> FolderSampleMoveResult {
    let mut moved = Vec::new();
    let mut completed = 0usize;
    let mut cancelled = false;
    if !source_root.is_dir() {
        errors.push(format!("Source folder missing: {}", source_root.display()));
        return FolderSampleMoveResult {
            source_id,
            moved,
            errors,
            cancelled,
        };
    }
    let db = match SourceDatabase::open(&source_root) {
        Ok(db) => db,
        Err(err) => {
            errors.push(format!("Failed to open source DB: {err}"));
            return FolderSampleMoveResult {
                source_id,
                moved,
                errors,
                cancelled,
            };
        }
    };
    for request in requests {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }
        let detail = Some(format!("Moving {}", request.relative_path.display()));
        let absolute = source_root.join(&request.relative_path);
        if !absolute.is_file() {
            errors.push(format!("File missing: {}", request.relative_path.display()));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        if let Some(parent) = request.target_relative.parent() {
            let target_dir = source_root.join(parent);
            if !target_dir.is_dir() {
                errors.push(format!("Folder not found: {}", parent.display()));
                completed += 1;
                report_progress(sender, completed, detail);
                continue;
            }
        }
        let target_absolute = source_root.join(&request.target_relative);
        if target_absolute.exists() {
            errors.push(format!(
                "A file already exists at {}",
                request.target_relative.display()
            ));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        let metadata = match load_sample_move_metadata(&db, &request.relative_path) {
            Ok(metadata) => metadata,
            Err(err) => {
                errors.push(err);
                completed += 1;
                report_progress(sender, completed, detail);
                continue;
            }
        };
        let prepared = match prepare_staged_move(
            &db,
            &source_root,
            &request.relative_path,
            &source_root,
            &request.target_relative,
            metadata,
        ) {
            Ok(prepared) => prepared,
            Err(err) => {
                errors.push(err);
                completed += 1;
                report_progress(sender, completed, detail);
                continue;
            }
        };
        #[cfg(test)]
        run_before_folder_sample_batch_hook();
        let mut batch = match db.write_batch() {
            Ok(batch) => batch,
            Err(err) => {
                rollback_staged_move_to_source(
                    &mut errors,
                    &prepared.staged_absolute,
                    &prepared.source_absolute,
                );
                remove_move_journal_entry(&mut errors, &db, &prepared.op_id);
                errors.push(format!("Failed to start database update: {err}"));
                completed += 1;
                report_progress(sender, completed, detail);
                continue;
            }
        };
        if let Err(err) = batch.remove_file(&request.relative_path) {
            rollback_staged_move_to_source(
                &mut errors,
                &prepared.staged_absolute,
                &prepared.source_absolute,
            );
            remove_move_journal_entry(&mut errors, &db, &prepared.op_id);
            errors.push(format!("Failed to drop old entry: {err}"));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        if let Err(err) = batch.upsert_file(
            &request.target_relative,
            prepared.file_size,
            prepared.modified_ns,
        ) {
            rollback_staged_move_to_source(
                &mut errors,
                &prepared.staged_absolute,
                &prepared.source_absolute,
            );
            remove_move_journal_entry(&mut errors, &db, &prepared.op_id);
            errors.push(format!("Failed to register moved file: {err}"));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        if let Err(err) = batch.set_tag(&request.target_relative, metadata.tag) {
            rollback_staged_move_to_source(
                &mut errors,
                &prepared.staged_absolute,
                &prepared.source_absolute,
            );
            remove_move_journal_entry(&mut errors, &db, &prepared.op_id);
            errors.push(format!("Failed to copy tag: {err}"));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        if let Err(err) = batch.set_looped(&request.target_relative, metadata.looped) {
            rollback_staged_move_to_source(
                &mut errors,
                &prepared.staged_absolute,
                &prepared.source_absolute,
            );
            remove_move_journal_entry(&mut errors, &db, &prepared.op_id);
            errors.push(format!("Failed to copy loop marker: {err}"));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        if let Some(last_played_at) = metadata.last_played_at
            && let Err(err) = batch.set_last_played_at(&request.target_relative, last_played_at)
        {
            rollback_staged_move_to_source(
                &mut errors,
                &prepared.staged_absolute,
                &prepared.source_absolute,
            );
            remove_move_journal_entry(&mut errors, &db, &prepared.op_id);
            errors.push(format!("Failed to copy playback age: {err}"));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        if let Err(err) = batch.commit() {
            rollback_staged_move_to_source(
                &mut errors,
                &prepared.staged_absolute,
                &prepared.source_absolute,
            );
            remove_move_journal_entry(&mut errors, &db, &prepared.op_id);
            errors.push(format!("Failed to save move: {err}"));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        if let Err(err) = file_ops_journal::update_stage(
            &db,
            &prepared.op_id,
            file_ops_journal::FileOpStage::TargetDb,
            None,
            None,
        ) {
            errors.push(format!("Failed to update move journal: {err}"));
        }
        if let Err(err) = file_ops_journal::update_stage(
            &db,
            &prepared.op_id,
            file_ops_journal::FileOpStage::SourceDb,
            None,
            None,
        ) {
            errors.push(format!("Failed to update move journal: {err}"));
        }
        if let Err(err) = std::fs::rename(&prepared.staged_absolute, &prepared.target_absolute) {
            errors.push(format!("Failed to finalize move: {err}"));
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        remove_move_journal_entry(&mut errors, &db, &prepared.op_id);
        moved.push(FolderEntryMove {
            old_relative: request.relative_path,
            new_relative: request.target_relative,
            file_size: prepared.file_size,
            modified_ns: prepared.modified_ns,
            tag: metadata.tag,
            looped: metadata.looped,
            last_played_at: metadata.last_played_at,
        });
        completed += 1;
        report_progress(sender, completed, detail);
    }
    FolderSampleMoveResult {
        source_id,
        moved,
        errors,
        cancelled,
    }
}

/// Forward one progress update to the optional file-op channel sender.
pub(super) fn report_progress(
    sender: Option<&Sender<FileOpMessage>>,
    completed: usize,
    detail: Option<String>,
) {
    if let Some(tx) = sender {
        let _ = tx.send(FileOpMessage::Progress { completed, detail });
    }
}
