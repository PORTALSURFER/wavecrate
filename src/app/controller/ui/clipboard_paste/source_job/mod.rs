use super::*;
use crate::app::controller::jobs::{
    ClipboardPasteOutcome, ClipboardPasteResult, FileOpMessage, SourcePasteAdded,
};
use std::sync::mpsc::Sender;

mod cleanup;
mod commit;
mod finalize;
mod prepare;
mod stage;

use cleanup::report_staged_copy_failure;
use commit::{DbCommittedSourcePaste, commit_source_copy};
use finalize::finalize_source_copy;
use prepare::{PreparedSourcePaste, SourcePasteContext, prepare_source_paste};
use stage::{StagedSourcePaste, stage_source_copy};

/// Run one clipboard paste/import job through the staged source-copy workflow.
pub(super) fn run_clipboard_paste_job(
    job: ClipboardPasteJob,
    cancel: Arc<AtomicBool>,
    sender: Option<&Sender<FileOpMessage>>,
) -> ClipboardPasteResult {
    let mut progress = ClipboardPasteProgress::default();
    let outcome = match job.kind {
        ClipboardPasteJobKind::Source(ref source_job) => {
            run_source_paste_job(&job, source_job.clone(), cancel, sender, &mut progress)
        }
    };
    ClipboardPasteResult {
        outcome,
        skipped: progress.skipped,
        errors: progress.errors,
        cancelled: progress.cancelled,
        target_label: job.target_label,
        action_past_tense: job.action_past_tense,
    }
}

#[derive(Default)]
struct ClipboardPasteProgress {
    skipped: usize,
    errors: Vec<String>,
    completed: usize,
    cancelled: bool,
}

impl ClipboardPasteProgress {
    fn finish_path(&mut self, sender: Option<&Sender<FileOpMessage>>, detail: Option<String>) {
        self.completed += 1;
        report_progress(sender, self.completed, detail);
    }
}

fn run_source_paste_job(
    job: &ClipboardPasteJob,
    source_job: SourceClipboardPasteJob,
    cancel: Arc<AtomicBool>,
    sender: Option<&Sender<FileOpMessage>>,
    progress: &mut ClipboardPasteProgress,
) -> ClipboardPasteOutcome {
    let mut added = Vec::new();
    let context = match SourcePasteContext::new(source_job.clone()) {
        Ok(context) => context,
        Err(err) => {
            progress.errors.push(err);
            return ClipboardPasteOutcome::Source {
                source_id: source_job.source_id,
                added,
            };
        }
    };

    for path in &job.paths {
        if cancel.load(Ordering::Relaxed) {
            progress.cancelled = true;
            break;
        }

        let detail = Some(format!("{} {}", job.action_progress, path.display()));
        match process_source_paste_path(&context, path, job.action_label) {
            Ok(Some(result)) => added.push(result),
            Ok(None) => progress.skipped += 1,
            Err(errors) => progress.errors.extend(errors),
        }
        progress.finish_path(sender, detail);
    }

    ClipboardPasteOutcome::Source {
        source_id: context.source_id,
        added,
    }
}

fn process_source_paste_path(
    context: &SourcePasteContext,
    path: &Path,
    action_label: &str,
) -> Result<Option<SourcePasteAdded>, Vec<String>> {
    let prepared = match prepare_source_paste(context, path) {
        Ok(Some(prepared)) => prepared,
        Ok(None) => return Ok(None),
        Err(err) => return Err(vec![err]),
    };
    let staged = stage_source_copy(&context.db, prepared, action_label)?;
    let committed = commit_source_copy(&context.db, staged)?;
    finalize_source_copy(&context.db, committed).map(Some)
}

fn report_progress(
    sender: Option<&Sender<FileOpMessage>>,
    completed: usize,
    detail: Option<String>,
) {
    if let Some(tx) = sender {
        let _ = tx.send(FileOpMessage::Progress { completed, detail });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::library::wav_io::file_metadata;
    use crate::app::controller::test_support::{dummy_controller, write_test_wav};
    use crate::sample_sources::Rating;
    use crate::sample_sources::db::file_ops_journal;
    use tempfile::tempdir;

    #[test]
    fn precommit_failure_removes_staged_file_and_journal() {
        let (mut controller, source) = dummy_controller();
        controller.library.sources.push(source.clone());
        controller.cache_db(&source).unwrap();
        let db = SourceDatabase::open(&source.root).unwrap();
        let op_id = file_ops_journal::new_op_id();
        let relative = PathBuf::from("Drums").join("kick.wav");
        let staged_relative =
            file_ops_journal::staged_relative_for_target(&relative, &op_id).unwrap();
        let staged_absolute = source.root.join(&staged_relative);
        std::fs::create_dir_all(staged_absolute.parent().unwrap()).unwrap();
        write_test_wav(&staged_absolute, &[0.0, 0.2, -0.2]);
        let entry = file_ops_journal::FileOpJournalEntry::new_copy(
            op_id.clone(),
            relative,
            staged_relative,
            Rating::NEUTRAL,
            false,
            None,
        )
        .unwrap();
        file_ops_journal::insert_entry(&db, &entry).unwrap();

        let errors = report_staged_copy_failure(&db, &staged_absolute, &op_id, "boom".into());

        assert!(errors.iter().any(|err| err == "boom"));
        assert!(!staged_absolute.exists());
        assert!(
            file_ops_journal::list_entries(&db)
                .unwrap()
                .entries
                .is_empty()
        );
    }

    #[test]
    fn finalize_failure_keeps_staged_file_and_journal_for_recovery() {
        let (mut controller, source) = dummy_controller();
        controller.library.sources.push(source.clone());
        controller.cache_db(&source).unwrap();
        let source_job = SourceClipboardPasteJob {
            source_id: source.id.clone(),
            source_root: source.root.clone(),
            target_folder: PathBuf::from("Drums"),
        };
        let context = SourcePasteContext::new(source_job).unwrap();
        let op_id = file_ops_journal::new_op_id();
        let relative = PathBuf::from("Drums").join("snare.wav");
        let staged_relative =
            file_ops_journal::staged_relative_for_target(&relative, &op_id).unwrap();
        let staged_absolute = source.root.join(&staged_relative);
        std::fs::create_dir_all(staged_absolute.parent().unwrap()).unwrap();
        write_test_wav(&staged_absolute, &[0.0, 0.1, -0.1]);
        let entry = file_ops_journal::FileOpJournalEntry::new_copy(
            op_id.clone(),
            relative.clone(),
            staged_relative.clone(),
            Rating::NEUTRAL,
            false,
            None,
        )
        .unwrap();
        file_ops_journal::insert_entry(&context.db, &entry).unwrap();
        let (file_size, modified_ns) = file_metadata(&staged_absolute).unwrap();
        file_ops_journal::update_stage(
            &context.db,
            &op_id,
            file_ops_journal::FileOpStage::TargetDb,
            Some(file_size),
            Some(modified_ns),
        )
        .unwrap();
        let absolute = source.root.join(&relative);
        std::fs::create_dir_all(&absolute).unwrap();

        let result = finalize_source_copy(
            &context.db,
            DbCommittedSourcePaste {
                staged: StagedSourcePaste {
                    prepared: PreparedSourcePaste {
                        source_path: tempdir().unwrap().path().join("ignored.wav"),
                        relative,
                        staged_relative,
                        staged_absolute: staged_absolute.clone(),
                        absolute,
                        op_id: op_id.clone(),
                    },
                    file_size,
                    modified_ns,
                },
            },
        );

        assert!(result.is_err());
        assert!(staged_absolute.exists());
        assert!(
            file_ops_journal::list_entries(&context.db)
                .unwrap()
                .entries
                .iter()
                .any(|entry| entry.id == op_id)
        );
    }
}
