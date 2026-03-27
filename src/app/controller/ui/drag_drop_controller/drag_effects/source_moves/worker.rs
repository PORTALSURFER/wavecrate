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

#[cfg(test)]
mod tests {
    use super::super::source_move_test_guard;
    use super::*;
    use crate::app::controller::jobs::SourceMoveRequest;
    use crate::app::controller::test_support::write_test_wav;
    use crate::sample_sources::{SampleSource, SourceDatabase};
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn source_move_task_uses_unique_target_name_on_collision() {
        let _guard = source_move_test_guard();
        set_before_source_move_target_db_stage_hook(None);
        set_after_source_move_progress_hook(None);
        let temp = tempdir().unwrap();
        let source_root = temp.path().join("source_a");
        let target_root = temp.path().join("source_b");
        std::fs::create_dir_all(&source_root).unwrap();
        std::fs::create_dir_all(&target_root).unwrap();
        let source = SampleSource::new(source_root.clone());
        write_test_wav(&source_root.join("one.wav"), &[0.0, 0.1, -0.1]);
        write_test_wav(&target_root.join("one.wav"), &[0.0, 0.2, -0.2]);
        let source_db = SourceDatabase::open(&source_root).unwrap();
        source_db.upsert_file(Path::new("one.wav"), 3, 1).unwrap();
        source_db
            .set_tag(Path::new("one.wav"), crate::sample_sources::Rating::KEEP_1)
            .unwrap();
        source_db.set_looped(Path::new("one.wav"), true).unwrap();
        source_db.set_locked(Path::new("one.wav"), true).unwrap();
        source_db
            .set_last_played_at(Path::new("one.wav"), 42)
            .unwrap();
        let cancel = Arc::new(AtomicBool::new(false));

        let result = run_source_move_task(
            SourceId::from_string("target"),
            target_root.clone(),
            vec![SourceMoveRequest {
                source_id: source.id,
                source_root: source_root.clone(),
                relative_path: PathBuf::from("one.wav"),
            }],
            Vec::new(),
            cancel,
            None,
        );

        assert_eq!(result.moved.len(), 1);
        assert_eq!(
            result.moved[0].target_relative,
            PathBuf::from("one_move001.wav")
        );
        assert!(result.moved[0].looped);
        assert!(result.moved[0].locked);
        assert_eq!(result.moved[0].last_played_at, Some(42));
        assert!(target_root.join("one_move001.wav").is_file());
        assert!(!source_root.join("one.wav").exists());
        let target_db = SourceDatabase::open(&target_root).unwrap();
        assert_eq!(
            target_db
                .tag_for_path(Path::new("one_move001.wav"))
                .unwrap(),
            Some(crate::sample_sources::Rating::KEEP_1)
        );
        assert_eq!(
            target_db
                .looped_for_path(Path::new("one_move001.wav"))
                .unwrap(),
            Some(true)
        );
        assert_eq!(
            target_db
                .locked_for_path(Path::new("one_move001.wav"))
                .unwrap(),
            Some(true)
        );
    }

    #[test]
    fn source_move_task_rolls_back_when_target_db_stage_fails() {
        let _guard = source_move_test_guard();
        set_before_source_move_target_db_stage_hook(None);
        set_after_source_move_progress_hook(None);
        let temp = tempdir().unwrap();
        let source_root = temp.path().join("source_a");
        let target_root = temp.path().join("source_b");
        std::fs::create_dir_all(&source_root).unwrap();
        std::fs::create_dir_all(&target_root).unwrap();
        let source = SampleSource::new(source_root.clone());
        write_test_wav(&source_root.join("one.wav"), &[0.0, 0.1, -0.1]);
        let source_db = SourceDatabase::open(&source_root).unwrap();
        source_db.upsert_file(Path::new("one.wav"), 3, 1).unwrap();
        set_before_source_move_target_db_stage_hook(Some(Box::new(|| {
            Err("Simulated target DB failure".into())
        })));

        let result = run_source_move_task(
            SourceId::from_string("target"),
            target_root.clone(),
            vec![SourceMoveRequest {
                source_id: source.id,
                source_root: source_root.clone(),
                relative_path: PathBuf::from("one.wav"),
            }],
            Vec::new(),
            Arc::new(AtomicBool::new(false)),
            None,
        );

        assert!(source_root.join("one.wav").is_file());
        assert!(result.moved.is_empty());
        assert!(
            result
                .errors
                .iter()
                .any(|error| error.contains("Simulated target DB failure"))
        );
    }

    #[test]
    fn source_move_task_reports_progress_once_for_missing_file_request() {
        let _guard = source_move_test_guard();
        set_before_source_move_target_db_stage_hook(None);
        set_after_source_move_progress_hook(None);
        let temp = tempdir().unwrap();
        let source_root = temp.path().join("source_a");
        let target_root = temp.path().join("source_b");
        std::fs::create_dir_all(&source_root).unwrap();
        std::fs::create_dir_all(&target_root).unwrap();
        let request = SourceMoveRequest {
            source_id: SourceId::from_string("source"),
            source_root,
            relative_path: PathBuf::from("missing.wav"),
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel();

        let result = run_source_move_task(
            SourceId::from_string("target"),
            target_root,
            vec![request],
            Vec::new(),
            cancel,
            Some(&tx),
        );

        assert!(result.moved.is_empty());
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("File missing"));
        let progress_messages = rx
            .try_iter()
            .filter(|message| matches!(message, FileOpMessage::Progress { .. }))
            .count();
        assert_eq!(progress_messages, 1);
    }

    #[test]
    fn source_move_task_cancels_after_first_completed_request() {
        let _guard = source_move_test_guard();
        set_before_source_move_target_db_stage_hook(None);
        set_after_source_move_progress_hook(None);
        let temp = tempdir().unwrap();
        let source_root = temp.path().join("source_a");
        let target_root = temp.path().join("source_b");
        std::fs::create_dir_all(&source_root).unwrap();
        std::fs::create_dir_all(&target_root).unwrap();
        let source = SampleSource::new(source_root.clone());
        for name in ["one.wav", "two.wav"] {
            write_test_wav(&source_root.join(name), &[0.0, 0.1, -0.1]);
        }
        let source_db = SourceDatabase::open(&source_root).unwrap();
        source_db.upsert_file(Path::new("one.wav"), 3, 1).unwrap();
        source_db.upsert_file(Path::new("two.wav"), 3, 1).unwrap();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_for_hook = cancel.clone();
        set_after_source_move_progress_hook(Some(Box::new(move |completed| {
            if completed == 1 {
                cancel_for_hook.store(true, Ordering::Relaxed);
            }
        })));

        let result = run_source_move_task(
            SourceId::from_string("target"),
            target_root.clone(),
            vec![
                SourceMoveRequest {
                    source_id: source.id.clone(),
                    source_root: source_root.clone(),
                    relative_path: PathBuf::from("one.wav"),
                },
                SourceMoveRequest {
                    source_id: source.id,
                    source_root: source_root.clone(),
                    relative_path: PathBuf::from("two.wav"),
                },
            ],
            Vec::new(),
            cancel,
            None,
        );

        assert!(result.cancelled);
        assert_eq!(result.moved.len(), 1);
        assert!(target_root.join(&result.moved[0].target_relative).is_file());
        assert!(source_root.join("two.wav").is_file());
        set_after_source_move_progress_hook(None);
    }
}
