use super::*;
use crate::app::controller::jobs::{
    ClipboardPasteOutcome, ClipboardPasteResult, FileOpMessage, SourcePasteAdded,
};
use crate::app::controller::library::wav_io::file_metadata;
use crate::sample_sources::db::file_ops_journal;
use crate::sample_sources::{SourceDatabase, is_supported_audio};

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

struct SourcePasteContext {
    source_id: SourceId,
    source_root: PathBuf,
    target_folder: PathBuf,
    target_root: PathBuf,
    db: SourceDatabase,
}

impl SourcePasteContext {
    fn new(job: SourceClipboardPasteJob) -> Result<Self, String> {
        validate_relative_folder_path(&job.target_folder)?;
        if !job.source_root.is_dir() {
            return Err("Source folder is not available".to_string());
        }
        let target_root = job.source_root.join(&job.target_folder);
        if !target_root.exists() {
            std::fs::create_dir_all(&target_root).map_err(|err| {
                format!("Failed to create folder {}: {err}", target_root.display())
            })?;
        } else if !target_root.is_dir() {
            return Err(format!(
                "Target folder is not a directory: {}",
                target_root.display()
            ));
        }
        let db = SourceDatabase::open(&job.source_root)
            .map_err(|err| format!("Failed to open source DB: {err}"))?;
        Ok(Self {
            source_id: job.source_id,
            source_root: job.source_root,
            target_folder: job.target_folder,
            target_root,
            db,
        })
    }
}

struct PreparedSourcePaste {
    source_path: PathBuf,
    relative: PathBuf,
    staged_relative: PathBuf,
    staged_absolute: PathBuf,
    absolute: PathBuf,
    op_id: String,
}

struct StagedSourcePaste {
    prepared: PreparedSourcePaste,
    file_size: u64,
    modified_ns: i64,
}

struct DbCommittedSourcePaste {
    staged: StagedSourcePaste,
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
        match prepare_source_paste(&context, path) {
            Ok(None) => {
                progress.skipped += 1;
            }
            Ok(Some(prepared)) => {
                match stage_source_copy(&context.db, prepared, job.action_label) {
                    Ok(staged) => match commit_source_copy(&context.db, staged) {
                        Ok(committed) => match finalize_source_copy(&context.db, committed) {
                            Ok(result) => added.push(result),
                            Err(errors) => progress.errors.extend(errors),
                        },
                        Err(errors) => progress.errors.extend(errors),
                    },
                    Err(errors) => progress.errors.extend(errors),
                }
            }
            Err(err) => progress.errors.push(err),
        }
        progress.finish_path(sender, detail);
    }
    ClipboardPasteOutcome::Source {
        source_id: context.source_id,
        added,
    }
}

fn prepare_source_paste(
    context: &SourcePasteContext,
    path: &Path,
) -> Result<Option<PreparedSourcePaste>, String> {
    if !path.is_file() || !is_supported_audio(path) {
        return Ok(None);
    }
    let relative_name = unique_destination_name(&context.target_root, path)?;
    let relative = if context.target_folder.as_os_str().is_empty() {
        relative_name
    } else {
        context.target_folder.join(relative_name)
    };
    let op_id = file_ops_journal::new_op_id();
    let staged_relative = file_ops_journal::staged_relative_for_target(&relative, &op_id)
        .map_err(|err| format!("Failed to build staging path: {err}"))?;
    Ok(Some(PreparedSourcePaste {
        source_path: path.to_path_buf(),
        staged_absolute: context.source_root.join(&staged_relative),
        absolute: context.source_root.join(&relative),
        relative,
        staged_relative,
        op_id,
    }))
}

fn stage_source_copy(
    db: &SourceDatabase,
    prepared: PreparedSourcePaste,
    action_label: &str,
) -> Result<StagedSourcePaste, Vec<String>> {
    let journal_entry = match file_ops_journal::FileOpJournalEntry::new_copy(
        prepared.op_id.clone(),
        prepared.relative.clone(),
        prepared.staged_relative.clone(),
    ) {
        Ok(entry) => entry,
        Err(err) => return Err(vec![format!("Failed to stage copy journal: {err}")]),
    };
    if let Err(err) = file_ops_journal::insert_entry(db, &journal_entry) {
        return Err(vec![format!("Failed to record copy journal: {err}")]);
    }
    if let Err(err) = std::fs::copy(&prepared.source_path, &prepared.staged_absolute) {
        return Err(report_staged_copy_failure(
            db,
            &prepared.staged_absolute,
            &prepared.op_id,
            format!(
                "Failed to {} {}: {err}",
                action_label,
                prepared.source_path.display()
            ),
        ));
    }
    let (file_size, modified_ns) = match file_metadata(&prepared.staged_absolute) {
        Ok(meta) => meta,
        Err(err) => {
            return Err(report_staged_copy_failure(
                db,
                &prepared.staged_absolute,
                &prepared.op_id,
                err,
            ));
        }
    };
    if let Err(err) = file_ops_journal::update_stage(
        db,
        &prepared.op_id,
        file_ops_journal::FileOpStage::Staged,
        Some(file_size),
        Some(modified_ns),
    ) {
        return Err(report_staged_copy_failure(
            db,
            &prepared.staged_absolute,
            &prepared.op_id,
            format!("Failed to update copy journal: {err}"),
        ));
    }
    Ok(StagedSourcePaste {
        prepared,
        file_size,
        modified_ns,
    })
}

fn commit_source_copy(
    db: &SourceDatabase,
    staged: StagedSourcePaste,
) -> Result<DbCommittedSourcePaste, Vec<String>> {
    let mut batch = match db.write_batch() {
        Ok(batch) => batch,
        Err(err) => {
            return Err(report_staged_copy_failure(
                db,
                &staged.prepared.staged_absolute,
                &staged.prepared.op_id,
                format!("Failed to open source DB batch: {err}"),
            ));
        }
    };
    if let Err(err) = batch.upsert_file(
        &staged.prepared.relative,
        staged.file_size,
        staged.modified_ns,
    ) {
        return Err(report_staged_copy_failure(
            db,
            &staged.prepared.staged_absolute,
            &staged.prepared.op_id,
            format!("Failed to register file: {err}"),
        ));
    }
    if let Err(err) = batch.commit() {
        return Err(report_staged_copy_failure(
            db,
            &staged.prepared.staged_absolute,
            &staged.prepared.op_id,
            format!("Failed to commit source DB update: {err}"),
        ));
    }
    if let Err(err) = file_ops_journal::update_stage(
        db,
        &staged.prepared.op_id,
        file_ops_journal::FileOpStage::TargetDb,
        None,
        None,
    ) {
        return Err(vec![format!("Failed to update copy journal: {err}")]);
    }
    Ok(DbCommittedSourcePaste { staged })
}

fn finalize_source_copy(
    db: &SourceDatabase,
    committed: DbCommittedSourcePaste,
) -> Result<SourcePasteAdded, Vec<String>> {
    if let Err(err) = std::fs::rename(
        &committed.staged.prepared.staged_absolute,
        &committed.staged.prepared.absolute,
    ) {
        return Err(vec![format!("Failed to finalize copy: {err}")]);
    }
    let mut errors = Vec::new();
    remove_copy_journal_entry(&mut errors, db, &committed.staged.prepared.op_id);
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(SourcePasteAdded {
        relative_path: committed.staged.prepared.relative,
        file_size: committed.staged.file_size,
        modified_ns: committed.staged.modified_ns,
    })
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

fn report_staged_copy_failure(
    db: &SourceDatabase,
    staged_absolute: &Path,
    op_id: &str,
    primary_error: String,
) -> Vec<String> {
    let mut errors = Vec::new();
    remove_staged_file(&mut errors, staged_absolute);
    remove_copy_journal_entry(&mut errors, db, op_id);
    errors.push(primary_error);
    errors
}

fn remove_copy_journal_entry(errors: &mut Vec<String>, db: &SourceDatabase, op_id: &str) {
    if let Err(err) = file_ops_journal::remove_entry(db, op_id) {
        errors.push(format!("Failed to clear copy journal: {err}"));
    }
}

fn remove_staged_file(errors: &mut Vec<String>, path: &Path) {
    if let Err(err) = std::fs::remove_file(path) {
        errors.push(format!(
            "Failed to remove staged file {}: {err}",
            path.display()
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::controller::test_support::{dummy_controller, write_test_wav};
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
