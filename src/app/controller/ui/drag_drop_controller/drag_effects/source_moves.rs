#![allow(clippy::too_many_arguments)]

use super::super::DragDropController;
use super::move_transaction::{
    PreparedStagedMove, SampleMoveMetadata, load_sample_move_metadata, prepare_staged_move,
    remove_move_journal_entry, report_staged_move_failure,
};
use crate::app::controller::StatusTone;
use crate::app::controller::jobs::{
    FileOpMessage, FileOpResult, SourceMoveRequest, SourceMoveResult, SourceMoveSuccess,
};
use crate::app::state::DragSample;
use crate::sample_sources::db::file_ops_journal;
use crate::sample_sources::{Rating, SourceDatabase, SourceId, WavEntry};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
};
use tracing::info;

impl DragDropController<'_> {
    pub(crate) fn handle_sample_drop_to_source(
        &mut self,
        source_id: SourceId,
        relative_path: PathBuf,
        target_source_id: SourceId,
    ) -> bool {
        let sample = DragSample {
            source_id,
            relative_path,
        };
        self.handle_samples_drop_to_source(std::slice::from_ref(&sample), target_source_id);
        true
    }

    pub(crate) fn handle_samples_drop_to_source(
        &mut self,
        samples: &[DragSample],
        target_source_id: SourceId,
    ) {
        if samples.is_empty() {
            return;
        }
        if self.runtime.jobs.file_ops_in_progress() {
            self.set_status(
                "Another file operation is already running",
                StatusTone::Warning,
            );
            return;
        }
        if samples
            .iter()
            .all(|sample| sample.source_id == target_source_id)
        {
            self.set_status("Samples are already in that source", StatusTone::Info);
            return;
        }
        let Some(target_source) = self
            .library
            .sources
            .iter()
            .find(|source| source.id == target_source_id)
            .cloned()
        else {
            self.set_status("Target source not available for move", StatusTone::Error);
            return;
        };
        let mut requests = Vec::new();
        let mut errors = Vec::new();
        for sample in samples {
            let Some(source) = self
                .library
                .sources
                .iter()
                .find(|source| source.id == sample.source_id)
                .cloned()
            else {
                errors.push(format!(
                    "Source not available for move: {}",
                    sample.relative_path.display()
                ));
                continue;
            };
            requests.push(SourceMoveRequest {
                source_id: source.id,
                source_root: source.root,
                relative_path: sample.relative_path.clone(),
            });
        }
        if requests.is_empty() {
            if let Some(err) = errors.pop() {
                self.set_status(err, StatusTone::Error);
            }
            return;
        }
        self.set_status("Moving samples...", StatusTone::Busy);
        self.show_status_progress(
            crate::app::state::ProgressTaskKind::FileOps,
            "Moving samples",
            requests.len(),
            true,
        );
        let target_root = target_source.root.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        #[cfg(test)]
        {
            let result = run_source_move_task(
                target_source_id,
                target_root,
                requests,
                errors,
                cancel,
                None,
            );
            let message = FileOpMessage::Finished(FileOpResult::SourceMove(result));
            if let FileOpMessage::Finished(FileOpResult::SourceMove(result)) = message {
                self.apply_source_move_result(result);
            }
            if self.ui.progress.task == Some(crate::app::state::ProgressTaskKind::FileOps) {
                self.clear_progress();
            }
        }
        #[cfg(not(test))]
        {
            let (tx, rx) = std::sync::mpsc::channel();
            self.runtime.jobs.start_file_ops(rx, cancel.clone());
            std::thread::spawn(move || {
                let result = run_source_move_task(
                    target_source_id,
                    target_root,
                    requests,
                    errors,
                    cancel,
                    Some(&tx),
                );
                let _ = tx.send(FileOpMessage::Finished(FileOpResult::SourceMove(result)));
            });
        }
    }

    /// Apply a completed background source move job.
    pub(crate) fn apply_source_move_result(&mut self, result: SourceMoveResult) {
        let Some(target_source) = self
            .library
            .sources
            .iter()
            .find(|source| source.id == result.target_source_id)
            .cloned()
        else {
            self.set_status("Target source not available for move", StatusTone::Error);
            return;
        };
        let mut moved_sources = HashSet::new();
        for entry in &result.moved {
            let Some(source) = self
                .library
                .sources
                .iter()
                .find(|source| source.id == entry.source_id)
                .cloned()
            else {
                continue;
            };
            self.prune_cached_sample(&source, &entry.relative_path);
            self.insert_cached_entry(
                &target_source,
                WavEntry {
                    relative_path: entry.target_relative.clone(),
                    file_size: entry.file_size,
                    modified_ns: entry.modified_ns,
                    content_hash: None,
                    tag: entry.tag,
                    looped: entry.looped,
                    missing: false,
                    last_played_at: entry.last_played_at,
                },
            );
            moved_sources.insert(source.id.clone());
            moved_sources.insert(target_source.id.clone());
        }
        for source_id in moved_sources {
            let Some(source) = self
                .library
                .sources
                .iter()
                .find(|source| source.id == source_id)
                .cloned()
            else {
                continue;
            };
            self.invalidate_wav_entries_for_source_preserve_folders(&source);
        }
        let moved = result.moved.len();
        if moved == 0 && result.errors.is_empty() {
            if result.cancelled {
                self.set_status("Move cancelled", StatusTone::Warning);
            } else {
                self.set_status("No samples moved", StatusTone::Warning);
            }
            return;
        }
        let tone = if result.errors.is_empty() && !result.cancelled {
            StatusTone::Info
        } else {
            StatusTone::Warning
        };
        let mut message = format!("Moved {moved} sample(s)");
        if !result.errors.is_empty() {
            message.push_str(&format!(" with {} error(s)", result.errors.len()));
        }
        if result.cancelled {
            message.push_str(" (cancelled)");
        }
        self.set_status(message, tone);
        for err in &result.errors {
            eprintln!("Source move error: {err}");
        }
        info!(
            "Source move completed: {} moved, {} errors",
            moved,
            result.errors.len()
        );
    }

    pub(super) fn register_moved_sample_for_source(
        &mut self,
        source: &crate::sample_sources::SampleSource,
        relative_path: &Path,
        file_size: u64,
        modified_ns: i64,
        tag: Rating,
        looped: bool,
        last_played_at: Option<i64>,
    ) -> Result<(), String> {
        let db = self
            .database_for(source)
            .map_err(|err| format!("Database unavailable: {err}"))?;
        db.upsert_file(relative_path, file_size, modified_ns)
            .map_err(|err| format!("Failed to register file: {err}"))?;
        db.set_tag(relative_path, tag)
            .map_err(|err| format!("Failed to set tag: {err}"))?;
        db.set_looped(relative_path, looped)
            .map_err(|err| format!("Failed to set loop marker: {err}"))?;
        if let Some(last_played_at) = last_played_at {
            db.set_last_played_at(relative_path, last_played_at)
                .map_err(|err| format!("Failed to copy playback age: {err}"))?;
        }
        Ok(())
    }

    pub(super) fn remove_source_db_entry(
        &mut self,
        source: &crate::sample_sources::SampleSource,
        relative_path: &Path,
    ) -> Result<(), String> {
        let db = self
            .database_for(source)
            .map_err(|err| format!("Database unavailable: {err}"))?;
        db.remove_file(relative_path)
            .map_err(|err| format!("Failed to drop database row: {err}"))
    }
}

fn unique_destination_path(root: &Path, relative: &Path) -> Result<PathBuf, String> {
    if !root.join(relative).exists() {
        return Ok(relative.to_path_buf());
    }
    let parent = relative.parent().unwrap_or_else(|| Path::new(""));
    let file_name = relative
        .file_name()
        .ok_or_else(|| "Sample has no file name".to_string())?;
    let stem = Path::new(file_name)
        .file_stem()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "sample".to_string());
    let extension = Path::new(file_name)
        .extension()
        .map(|ext| ext.to_string_lossy().to_string());
    for index in 1..=999 {
        let suffix = format!("{stem}_move{index:03}");
        let file_name = if let Some(ext) = &extension {
            format!("{suffix}.{ext}")
        } else {
            suffix
        };
        let candidate = parent.join(file_name);
        if !root.join(&candidate).exists() {
            return Ok(candidate);
        }
    }
    Err("Failed to find destination file name".into())
}

fn run_source_move_task(
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
    let mut source_dbs: HashMap<PathBuf, SourceDatabase> = HashMap::new();

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

/// Per-task progress counter and sender wrapper for source-move worker updates.
struct SourceMoveProgress<'a> {
    sender: Option<&'a Sender<FileOpMessage>>,
    completed: usize,
}

impl<'a> SourceMoveProgress<'a> {
    /// Create a new source-move progress tracker.
    fn new(sender: Option<&'a Sender<FileOpMessage>>) -> Self {
        Self {
            sender,
            completed: 0,
        }
    }

    /// Record one completed request and publish a progress update.
    fn complete(&mut self, detail: Option<String>) {
        self.completed = self.completed.saturating_add(1);
        report_progress(self.sender, self.completed, detail);
    }
}

/// Execute one source move request end-to-end and return a success payload on completion.
fn run_source_move_request(
    target_root: &Path,
    target_db: &SourceDatabase,
    source_dbs: &mut HashMap<PathBuf, SourceDatabase>,
    request: SourceMoveRequest,
    errors: &mut Vec<String>,
) -> Option<SourceMoveSuccess> {
    let absolute = request.source_root.join(&request.relative_path);
    if !absolute.is_file() {
        errors.push(format!("File missing: {}", request.relative_path.display()));
        return None;
    }

    let target_relative = match unique_destination_path(target_root, &request.relative_path) {
        Ok(path) => path,
        Err(err) => {
            errors.push(err);
            return None;
        }
    };
    if let Some(parent) = target_relative.parent() {
        let target_dir = target_root.join(parent);
        if let Err(err) = std::fs::create_dir_all(&target_dir) {
            errors.push(format!(
                "Failed to create target folder {}: {err}",
                target_dir.display()
            ));
            return None;
        }
    }

    let source_db = match source_db_for_request(source_dbs, &request.source_root) {
        Ok(source_db) => source_db,
        Err(err) => {
            errors.push(err);
            return None;
        }
    };
    let metadata = match load_sample_move_metadata(source_db, &request.relative_path) {
        Ok(metadata) => metadata,
        Err(err) => {
            errors.push(err);
            return None;
        }
    };
    let prepared = match prepare_staged_move(
        target_db,
        &request.source_root,
        &request.relative_path,
        target_root,
        &target_relative,
        metadata,
    ) {
        Ok(prepared) => prepared,
        Err(err) => {
            errors.push(err);
            return None;
        }
    };

    if !commit_target_db_registration(target_db, &target_relative, metadata, &prepared, errors) {
        return None;
    }

    if let Err(err) = source_db.remove_file(&request.relative_path) {
        errors.push(format!("Failed to drop database row: {err}"));
        return None;
    }
    if let Err(err) = file_ops_journal::update_stage(
        target_db,
        &prepared.op_id,
        file_ops_journal::FileOpStage::SourceDb,
        None,
        None,
    ) {
        errors.push(format!("Failed to update move journal: {err}"));
    }
    if let Err(err) = std::fs::rename(&prepared.staged_absolute, &prepared.target_absolute) {
        errors.push(format!("Failed to finalize move: {err}"));
        return None;
    }

    remove_move_journal_entry(errors, target_db, &prepared.op_id);
    Some(SourceMoveSuccess {
        source_id: request.source_id,
        relative_path: request.relative_path,
        target_relative,
        file_size: prepared.file_size,
        modified_ns: prepared.modified_ns,
        tag: metadata.tag,
        looped: metadata.looped,
        last_played_at: metadata.last_played_at,
    })
}

/// Resolve or open the source database handle for one request source root.
fn source_db_for_request<'a>(
    source_dbs: &'a mut HashMap<PathBuf, SourceDatabase>,
    source_root: &Path,
) -> Result<&'a mut SourceDatabase, String> {
    match source_dbs.entry(source_root.to_path_buf()) {
        std::collections::hash_map::Entry::Occupied(entry) => Ok(entry.into_mut()),
        std::collections::hash_map::Entry::Vacant(entry) => match SourceDatabase::open(source_root)
        {
            Ok(db) => Ok(entry.insert(db)),
            Err(err) => Err(format!("Failed to open source DB: {err}")),
        },
    }
}

/// Register the staged target file in target DB and commit associated metadata updates.
fn commit_target_db_registration(
    target_db: &SourceDatabase,
    target_relative: &Path,
    metadata: SampleMoveMetadata,
    prepared: &PreparedStagedMove,
    errors: &mut Vec<String>,
) -> bool {
    let mut batch = match target_db.write_batch() {
        Ok(batch) => batch,
        Err(err) => {
            report_staged_move_failure(
                errors,
                target_db,
                prepared,
                format!("Failed to open target DB batch: {err}"),
            );
            return false;
        }
    };
    if let Err(err) = batch.upsert_file(target_relative, prepared.file_size, prepared.modified_ns) {
        report_staged_move_failure(
            errors,
            target_db,
            prepared,
            format!("Failed to register file: {err}"),
        );
        return false;
    }
    if let Err(err) = batch.set_tag(target_relative, metadata.tag) {
        report_staged_move_failure(
            errors,
            target_db,
            prepared,
            format!("Failed to set tag: {err}"),
        );
        return false;
    }
    if let Err(err) = batch.set_looped(target_relative, metadata.looped) {
        report_staged_move_failure(
            errors,
            target_db,
            prepared,
            format!("Failed to set loop marker: {err}"),
        );
        return false;
    }
    if let Some(last_played_at) = metadata.last_played_at
        && let Err(err) = batch.set_last_played_at(target_relative, last_played_at)
    {
        report_staged_move_failure(
            errors,
            target_db,
            prepared,
            format!("Failed to copy playback age: {err}"),
        );
        return false;
    }
    if let Err(err) = batch.commit() {
        report_staged_move_failure(
            errors,
            target_db,
            prepared,
            format!("Failed to commit target DB update: {err}"),
        );
        return false;
    }
    if let Err(err) = file_ops_journal::update_stage(
        target_db,
        &prepared.op_id,
        file_ops_journal::FileOpStage::TargetDb,
        None,
        None,
    ) {
        errors.push(format!("Failed to update move journal: {err}"));
    }
    true
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
    use crate::app::controller::AppController;
    use crate::app::controller::test_support::{sample_entry, write_test_wav};
    use crate::sample_sources::{Rating, SampleSource};
    use crate::waveform::WaveformRenderer;
    use tempfile::tempdir;

    #[test]
    fn moving_multiple_samples_to_source_clears_browser_rows() {
        let temp = tempdir().unwrap();
        let source_root = temp.path().join("source_a");
        let target_root = temp.path().join("source_b");
        std::fs::create_dir_all(&source_root).unwrap();
        std::fs::create_dir_all(&target_root).unwrap();
        let source = SampleSource::new(source_root);
        let target = SampleSource::new(target_root);
        let renderer = WaveformRenderer::new(10, 10);
        let mut controller = AppController::new(renderer, None);
        controller.library.sources.push(source.clone());
        controller.library.sources.push(target.clone());
        controller.selection_state.ctx.selected_source = Some(source.id.clone());
        controller.cache_db(&source).unwrap();
        controller.cache_db(&target).unwrap();
        write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1, -0.1]);
        write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1, -0.1]);
        controller
            .ensure_sample_db_entry(&source, Path::new("one.wav"))
            .unwrap();
        controller
            .ensure_sample_db_entry(&source, Path::new("two.wav"))
            .unwrap();
        controller.set_wav_entries_for_tests(vec![
            sample_entry("one.wav", Rating::NEUTRAL),
            sample_entry("two.wav", Rating::NEUTRAL),
        ]);
        controller.rebuild_wav_lookup();
        controller.rebuild_browser_lists();

        let samples = vec![
            DragSample {
                source_id: source.id.clone(),
                relative_path: PathBuf::from("one.wav"),
            },
            DragSample {
                source_id: source.id.clone(),
                relative_path: PathBuf::from("two.wav"),
            },
        ];
        controller
            .drag_drop()
            .handle_samples_drop_to_source(&samples, target.id.clone());

        assert!(
            controller
                .wav_index_for_path(Path::new("one.wav"))
                .is_none()
        );
        assert!(
            controller
                .wav_index_for_path(Path::new("two.wav"))
                .is_none()
        );
    }

    #[test]
    /// Each processed request should emit exactly one progress update, including failures.
    fn source_move_task_reports_progress_once_for_missing_file_request() {
        let temp = match tempdir() {
            Ok(temp) => temp,
            Err(err) => panic!("failed to create tempdir: {err}"),
        };
        let source_root = temp.path().join("source_a");
        let target_root = temp.path().join("source_b");
        if let Err(err) = std::fs::create_dir_all(&source_root) {
            panic!("failed to create source root: {err}");
        }
        if let Err(err) = std::fs::create_dir_all(&target_root) {
            panic!("failed to create target root: {err}");
        }
        let target_source_id = SourceId::from_string("target");
        let request = SourceMoveRequest {
            source_id: SourceId::from_string("source"),
            source_root: source_root.clone(),
            relative_path: PathBuf::from("missing.wav"),
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel();

        let result = run_source_move_task(
            target_source_id,
            target_root,
            vec![request],
            Vec::new(),
            cancel,
            Some(&tx),
        );

        assert!(result.moved.is_empty());
        assert_eq!(result.errors.len(), 1);
        assert!(
            result.errors[0].contains("File missing"),
            "expected missing-file error, got: {}",
            result.errors[0]
        );

        let progress_messages = rx
            .try_iter()
            .filter(|message| matches!(message, FileOpMessage::Progress { .. }))
            .count();
        assert_eq!(progress_messages, 1);
    }
}
