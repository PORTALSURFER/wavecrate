//! Background worker for cross-source sample moves.

use crate::app::controller::jobs::{
    FileOpMessage, SourceMoveRequest, SourceMoveResult, SourceMoveSuccess,
};
use crate::sample_sources::{SourceDatabase, SourceId};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
};
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

#[cfg(test)]
mod tests;
mod transaction;

use transaction::prepare_source_move_transaction;

#[cfg(test)]
type BeforeSourceMoveTargetDbStageHook = Box<dyn FnMut() -> Result<(), String> + Send>;
#[cfg(test)]
type AfterSourceMoveProgressHook = Box<dyn FnMut(usize) + Send>;

#[cfg(test)]
static BEFORE_SOURCE_MOVE_TARGET_DB_STAGE_HOOK: OnceLock<
    Mutex<Option<BeforeSourceMoveTargetDbStageHook>>,
> = OnceLock::new();
#[cfg(test)]
static AFTER_SOURCE_MOVE_PROGRESS_HOOK: OnceLock<Mutex<Option<AfterSourceMoveProgressHook>>> =
    OnceLock::new();

/// Execute a batch of cross-source sample moves in the background worker.
pub(super) fn run_source_move_task(
    target_source_id: SourceId,
    target_root: PathBuf,
    requests: Vec<SourceMoveRequest>,
    mut errors: Vec<String>,
    cancel: Arc<AtomicBool>,
    sender: Option<&Sender<FileOpMessage>>,
) -> SourceMoveResult {
    let mut progress = SourceMoveProgress::new(sender);
    let mut moved = Vec::new();
    let mut cancelled = false;
    if !target_root.is_dir() {
        errors.push(format!(
            "Target source folder missing: {}",
            target_root.display()
        ));
        return SourceMoveResult {
            target_source_id,
            moved,
            errors,
            cancelled,
        };
    }
    let target_db = match SourceDatabase::open(&target_root) {
        Ok(db) => db,
        Err(err) => {
            errors.push(format!("Failed to open target DB: {err}"));
            return SourceMoveResult {
                target_source_id,
                moved,
                errors,
                cancelled,
            };
        }
    };
    let mut source_dbs = HashMap::new();
    for request in requests {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }
        let detail = Some(format!("Moving {}", request.relative_path.display()));
        if let Some(success) = run_source_move_request(
            &target_root,
            &target_db,
            &mut source_dbs,
            request,
            &mut errors,
        ) {
            moved.push(success);
        }
        progress.complete(detail);
    }
    SourceMoveResult {
        target_source_id,
        moved,
        errors,
        cancelled,
    }
}

/// Progress reporter for the source-move worker.
struct SourceMoveProgress<'a> {
    sender: Option<&'a Sender<FileOpMessage>>,
    completed: usize,
}

impl<'a> SourceMoveProgress<'a> {
    fn new(sender: Option<&'a Sender<FileOpMessage>>) -> Self {
        Self {
            sender,
            completed: 0,
        }
    }

    fn complete(&mut self, detail: Option<String>) {
        self.completed = self.completed.saturating_add(1);
        report_progress(self.sender, self.completed, detail);
        #[cfg(test)]
        run_after_source_move_progress_hook(self.completed);
    }
}

/// Run one source-move request through its staged transaction pipeline.
fn run_source_move_request(
    target_root: &std::path::Path,
    target_db: &SourceDatabase,
    source_dbs: &mut HashMap<PathBuf, SourceDatabase>,
    request: SourceMoveRequest,
    errors: &mut Vec<String>,
) -> Option<SourceMoveSuccess> {
    let mut transaction =
        match prepare_source_move_transaction(target_root, target_db, source_dbs, request) {
            Ok(transaction) => transaction,
            Err(err) => {
                errors.push(err);
                return None;
            }
        };
    if !transaction.commit_target_db_stage(errors) {
        return None;
    }
    if !transaction.commit_source_db_stage(errors) {
        return None;
    }
    if !transaction.finalize_filesystem_stage(errors) {
        return None;
    }
    Some(transaction.into_success(errors))
}

/// Forward one source-move progress update to the optional file-op channel sender.
pub(super) fn report_progress(
    sender: Option<&Sender<FileOpMessage>>,
    completed: usize,
    detail: Option<String>,
) {
    if let Some(tx) = sender {
        let _ = tx.send(FileOpMessage::Progress { completed, detail });
    }
}

#[cfg(test)]
pub(super) fn set_before_source_move_target_db_stage_hook(
    hook: Option<BeforeSourceMoveTargetDbStageHook>,
) {
    let hook_slot = BEFORE_SOURCE_MOVE_TARGET_DB_STAGE_HOOK.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = hook_slot.lock() {
        *guard = hook;
    }
}

/// Test-only hook runner invoked immediately before the target-db stage begins.
#[cfg(test)]
pub(super) fn run_before_source_move_target_db_stage_hook() -> Result<(), String> {
    if let Some(hook_slot) = BEFORE_SOURCE_MOVE_TARGET_DB_STAGE_HOOK.get()
        && let Ok(mut guard) = hook_slot.lock()
        && let Some(mut hook) = guard.take()
    {
        return hook();
    }
    Ok(())
}

/// Configure a test-only hook invoked after each completed source-move request.
#[cfg(test)]
pub(super) fn set_after_source_move_progress_hook(hook: Option<AfterSourceMoveProgressHook>) {
    let hook_slot = AFTER_SOURCE_MOVE_PROGRESS_HOOK.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = hook_slot.lock() {
        *guard = hook;
    }
}

#[cfg(test)]
fn run_after_source_move_progress_hook(completed: usize) {
    if let Some(hook_slot) = AFTER_SOURCE_MOVE_PROGRESS_HOOK.get()
        && let Ok(mut guard) = hook_slot.lock()
        && let Some(hook) = guard.as_mut()
    {
        hook(completed);
    }
}
