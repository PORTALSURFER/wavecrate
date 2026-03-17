use self::folder_sample_move_task::prepare_folder_sample_move_transaction;
use crate::app::controller::jobs::{
    FileOpMessage, FolderMoveRequest, FolderMoveResult, FolderSampleMoveRequest,
    FolderSampleMoveResult,
};
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
/// Staged folder-sample move transaction helpers split from the main worker loop.
mod folder_sample_move_task;

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
        let transaction = match prepare_folder_sample_move_transaction(&db, &source_root, request) {
            Ok(transaction) => transaction,
            Err(err) => {
                errors.push(err);
                completed += 1;
                report_progress(sender, completed, detail);
                continue;
            }
        };
        #[cfg(test)]
        run_before_folder_sample_batch_hook();
        if !transaction.commit_db_stage(&mut errors) {
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        if !transaction.finalize_filesystem_stage(&mut errors) {
            completed += 1;
            report_progress(sender, completed, detail);
            continue;
        }
        moved.push(transaction.into_success(&mut errors));
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
