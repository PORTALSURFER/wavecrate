//! Background worker for drop-target copy and move operations.

use super::super::move_transaction::{
    load_sample_move_metadata, move_sample_file, prepare_staged_copy, prepare_staged_move,
};
use super::transactions::{
    clear_file_op_journal_entry, register_drop_target_target_entry, rollback_staged_copy,
    rollback_staged_move, rollback_staged_move_after_target_db_stage, sample_move_metadata,
    warn_on_journal_stage_update,
};
use super::{DroppedSampleMetadata, copy_destination_relative, move_destination_relative};
use crate::app::controller::jobs::{
    DropTargetTransferKind, DropTargetTransferMetadata, DropTargetTransferRequest,
    DropTargetTransferResult, DropTargetTransferSuccess, FileOpMessage,
};
use crate::sample_sources::db::file_ops_journal;
use crate::sample_sources::{SourceDatabase, SourceId};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::Sender,
};

/// Execute a batch of drop-target copy or move requests on the file-op worker.
pub(super) fn run_drop_target_transfer_task(
    kind: DropTargetTransferKind,
    target_source_id: SourceId,
    target_root: PathBuf,
    target_relative_folder: PathBuf,
    requests: Vec<DropTargetTransferRequest>,
    mut errors: Vec<String>,
    cancel: Arc<AtomicBool>,
    sender: Option<&Sender<FileOpMessage>>,
) -> DropTargetTransferResult {
    let target_dir = if target_relative_folder.as_os_str().is_empty() {
        target_root.clone()
    } else {
        target_root.join(&target_relative_folder)
    };
    let target_label = target_dir.display().to_string();
    let mut transferred = Vec::new();
    let mut cancelled = false;
    if !target_dir.is_dir() {
        errors.push(format!("Drop target missing: {}", target_dir.display()));
        return DropTargetTransferResult {
            kind,
            target_source_id,
            target_label,
            transferred,
            errors,
            cancelled,
        };
    }
    let target_db = match SourceDatabase::open(&target_root) {
        Ok(db) => db,
        Err(err) => {
            errors.push(format!("Failed to open target DB: {err}"));
            return DropTargetTransferResult {
                kind,
                target_source_id,
                target_label,
                transferred,
                errors,
                cancelled,
            };
        }
    };
    let mut source_dbs = HashMap::new();
    let mut progress = DropTargetTransferProgress::new(kind, sender);
    for request in requests {
        if cancel.load(Ordering::Relaxed) {
            cancelled = true;
            break;
        }
        let detail = Some(format!(
            "{} {}",
            kind.action_present_participle(),
            request.relative_path.display()
        ));
        if let Some(success) = run_drop_target_transfer_request(
            kind,
            &target_root,
            &target_relative_folder,
            &target_db,
            &mut source_dbs,
            request,
            &mut errors,
        ) {
            transferred.push(success);
        }
        progress.complete(detail);
    }
    DropTargetTransferResult {
        kind,
        target_source_id,
        target_label,
        transferred,
        errors,
        cancelled,
    }
}

/// Progress reporter for drop-target worker steps.
struct DropTargetTransferProgress<'a> {
    sender: Option<&'a Sender<FileOpMessage>>,
    completed: usize,
}

impl<'a> DropTargetTransferProgress<'a> {
    fn new(_kind: DropTargetTransferKind, sender: Option<&'a Sender<FileOpMessage>>) -> Self {
        Self {
            sender,
            completed: 0,
        }
    }

    fn complete(&mut self, detail: Option<String>) {
        self.completed = self.completed.saturating_add(1);
        if let Some(tx) = self.sender {
            let _ = tx.send(FileOpMessage::Progress {
                completed: self.completed,
                detail,
            });
        }
    }
}

/// Run one drop-target request through its staged transaction pipeline.
fn run_drop_target_transfer_request(
    kind: DropTargetTransferKind,
    target_root: &Path,
    target_relative_folder: &Path,
    target_db: &SourceDatabase,
    source_dbs: &mut HashMap<PathBuf, SourceDatabase>,
    request: DropTargetTransferRequest,
    errors: &mut Vec<String>,
) -> Option<DropTargetTransferSuccess> {
    let source_db = match source_db_for(target_root, target_db, source_dbs, &request.source_root) {
        Ok(db) => db,
        Err(err) => {
            errors.push(err);
            return None;
        }
    };
    let source_absolute = request.source_root.join(&request.relative_path);
    if !source_absolute.exists() {
        errors.push(format!("File missing: {}", request.relative_path.display()));
        return None;
    }
    let file_name = match file_name_for_request(&request.relative_path) {
        Ok(file_name) => file_name,
        Err(err) => {
            errors.push(err);
            return None;
        }
    };
    let metadata = match request
        .metadata
        .map(drop_target_metadata_from_request)
        .map(Ok)
        .unwrap_or_else(|| load_dropped_sample_metadata(source_db, &request.relative_path))
    {
        Ok(metadata) => metadata,
        Err(err) => {
            errors.push(err);
            return None;
        }
    };
    let target_relative = match kind {
        DropTargetTransferKind::Copy => {
            copy_destination_relative(target_root, target_relative_folder, &file_name)
        }
        DropTargetTransferKind::Move => {
            move_destination_relative(target_root, target_relative_folder, &file_name)
        }
    };
    let target_relative = match target_relative {
        Ok(path) => path,
        Err(err) => {
            errors.push(err);
            return None;
        }
    };
    match kind {
        DropTargetTransferKind::Copy => run_drop_target_copy(
            target_root,
            target_db,
            request,
            source_absolute,
            target_relative,
            metadata,
            errors,
        ),
        DropTargetTransferKind::Move => run_drop_target_move(
            target_root,
            target_db,
            source_db,
            request,
            target_relative,
            metadata,
            errors,
        ),
    }
}

/// Convert controller-captured metadata into the worker-local representation.
fn drop_target_metadata_from_request(
    metadata: DropTargetTransferMetadata,
) -> DroppedSampleMetadata {
    DroppedSampleMetadata {
        tag: metadata.tag,
        looped: metadata.looped,
        locked: metadata.locked,
        last_played_at: metadata.last_played_at,
    }
}

/// Open or reuse the source database for one drop-target request.
fn source_db_for<'a>(
    target_root: &Path,
    target_db: &'a SourceDatabase,
    source_dbs: &'a mut HashMap<PathBuf, SourceDatabase>,
    source_root: &Path,
) -> Result<&'a SourceDatabase, String> {
    if source_root == target_root {
        return Ok(target_db);
    }
    if !source_dbs.contains_key(source_root) {
        let db = SourceDatabase::open(source_root)
            .map_err(|err| format!("Failed to open source DB: {err}"))?;
        source_dbs.insert(source_root.to_path_buf(), db);
    }
    source_dbs
        .get(source_root)
        .ok_or_else(|| "Source database unavailable".to_string())
}

/// Read the source-row metadata needed to recreate the target DB row.
fn load_dropped_sample_metadata(
    db: &SourceDatabase,
    relative_path: &Path,
) -> Result<DroppedSampleMetadata, String> {
    let metadata = load_sample_move_metadata(db, relative_path)?;
    let locked = match db.locked_for_path(relative_path) {
        Ok(Some(locked)) => locked,
        Ok(None) => return Err("Sample not found in database".to_string()),
        Err(err) => return Err(format!("Failed to read database: {err}")),
    };
    Ok(DroppedSampleMetadata {
        tag: metadata.tag,
        looped: metadata.looped,
        locked,
        last_played_at: metadata.last_played_at,
    })
}

/// Resolve the file name for a requested source-relative sample path.
fn file_name_for_request(relative_path: &Path) -> Result<OsString, String> {
    relative_path
        .file_name()
        .map(|name| name.to_owned())
        .ok_or_else(|| "Sample name unavailable for drop".to_string())
}

/// Run the staged copy flow for a drop-target request.
fn run_drop_target_copy(
    target_root: &Path,
    target_db: &SourceDatabase,
    request: DropTargetTransferRequest,
    source_absolute: PathBuf,
    target_relative: PathBuf,
    metadata: DroppedSampleMetadata,
    errors: &mut Vec<String>,
) -> Option<DropTargetTransferSuccess> {
    let prepared = match prepare_staged_copy(
        target_db,
        &source_absolute,
        target_root,
        &target_relative,
        sample_move_metadata(metadata),
    ) {
        Ok(prepared) => prepared,
        Err(err) => {
            errors.push(err);
            return None;
        }
    };
    if let Err(err) = register_drop_target_target_entry(
        target_db,
        &target_relative,
        prepared.file_size,
        prepared.modified_ns,
        metadata,
    ) {
        rollback_staged_copy(target_db, &prepared);
        errors.push(err);
        return None;
    }
    warn_on_journal_stage_update(
        target_db,
        &prepared.op_id,
        file_ops_journal::FileOpStage::TargetDb,
    );
    if let Err(err) = move_sample_file(&prepared.staged_absolute, &prepared.target_absolute) {
        errors.push(format!("Failed to finalize copy: {err}"));
        return None;
    }
    clear_file_op_journal_entry(target_db, &prepared.op_id);
    Some(DropTargetTransferSuccess {
        source_id: request.source_id,
        source_relative: request.relative_path,
        target_relative,
        file_size: prepared.file_size,
        modified_ns: prepared.modified_ns,
        tag: metadata.tag,
        looped: metadata.looped,
        locked: metadata.locked,
        last_played_at: metadata.last_played_at,
    })
}

/// Run the staged move flow for a drop-target request.
fn run_drop_target_move(
    target_root: &Path,
    target_db: &SourceDatabase,
    source_db: &SourceDatabase,
    request: DropTargetTransferRequest,
    target_relative: PathBuf,
    metadata: DroppedSampleMetadata,
    errors: &mut Vec<String>,
) -> Option<DropTargetTransferSuccess> {
    let prepared = match prepare_staged_move(
        target_db,
        &request.source_root,
        &request.relative_path,
        target_root,
        &target_relative,
        sample_move_metadata(metadata),
    ) {
        Ok(prepared) => prepared,
        Err(err) => {
            errors.push(err);
            return None;
        }
    };
    if let Err(err) = register_drop_target_target_entry(
        target_db,
        &target_relative,
        prepared.file_size,
        prepared.modified_ns,
        metadata,
    ) {
        rollback_staged_move(target_db, &prepared);
        errors.push(err);
        return None;
    }
    warn_on_journal_stage_update(
        target_db,
        &prepared.op_id,
        file_ops_journal::FileOpStage::TargetDb,
    );
    if let Err(err) = source_db.remove_file(&request.relative_path) {
        rollback_staged_move_after_target_db_stage(target_db, &prepared, &target_relative);
        errors.push(format!("Failed to drop database row: {err}"));
        return None;
    }
    warn_on_journal_stage_update(
        target_db,
        &prepared.op_id,
        file_ops_journal::FileOpStage::SourceDb,
    );
    if let Err(err) = move_sample_file(&prepared.staged_absolute, &prepared.target_absolute) {
        errors.push(format!("Failed to finalize move: {err}"));
        return None;
    }
    clear_file_op_journal_entry(target_db, &prepared.op_id);
    Some(DropTargetTransferSuccess {
        source_id: request.source_id,
        source_relative: request.relative_path,
        target_relative,
        file_size: prepared.file_size,
        modified_ns: prepared.modified_ns,
        tag: metadata.tag,
        looped: metadata.looped,
        locked: metadata.locked,
        last_played_at: metadata.last_played_at,
    })
}
