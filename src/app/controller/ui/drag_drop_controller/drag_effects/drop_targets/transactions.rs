use super::DroppedSampleMetadata;
use super::super::move_transaction::{
    PreparedStagedCopy, PreparedStagedMove, SampleMoveMetadata, move_sample_file,
};
use crate::sample_sources::db::file_ops_journal;
use std::path::Path;
use tracing::warn;

/// Convert controller-copied sample metadata into journal metadata fields.
pub(super) fn sample_move_metadata(metadata: DroppedSampleMetadata) -> SampleMoveMetadata {
    SampleMoveMetadata {
        tag: metadata.tag,
        looped: metadata.looped,
        last_played_at: metadata.last_played_at,
    }
}

/// Commit the target DB row for a staged drop-target move or copy.
pub(super) fn register_drop_target_target_entry(
    target_db: &crate::sample_sources::SourceDatabase,
    relative_path: &Path,
    file_size: u64,
    modified_ns: i64,
    metadata: DroppedSampleMetadata,
) -> Result<(), String> {
    let mut batch = target_db
        .write_batch()
        .map_err(|err| format!("Failed to open target DB batch: {err}"))?;
    batch
        .upsert_file(relative_path, file_size, modified_ns)
        .map_err(|err| format!("Failed to register file: {err}"))?;
    batch
        .set_tag(relative_path, metadata.tag)
        .map_err(|err| format!("Failed to set tag: {err}"))?;
    batch
        .set_looped(relative_path, metadata.looped)
        .map_err(|err| format!("Failed to set loop marker: {err}"))?;
    batch
        .set_locked(relative_path, metadata.locked)
        .map_err(|err| format!("Failed to set keep lock: {err}"))?;
    if let Some(last_played_at) = metadata.last_played_at {
        batch
            .set_last_played_at(relative_path, last_played_at)
            .map_err(|err| format!("Failed to copy playback age: {err}"))?;
    }
    batch
        .commit()
        .map_err(|err| format!("Failed to commit target DB update: {err}"))
}

/// Advance the journal stage and log non-fatal failures.
pub(super) fn warn_on_journal_stage_update(
    target_db: &crate::sample_sources::SourceDatabase,
    op_id: &str,
    stage: file_ops_journal::FileOpStage,
) {
    if let Err(err) = file_ops_journal::update_stage(target_db, op_id, stage, None, None) {
        warn!("Drop target journal stage update failed for {op_id}: {err}");
    }
}

/// Remove a completed journal row and log non-fatal cleanup failures.
pub(super) fn clear_file_op_journal_entry(
    target_db: &crate::sample_sources::SourceDatabase,
    op_id: &str,
) {
    if let Err(err) = file_ops_journal::remove_entry(target_db, op_id) {
        warn!("Drop target journal cleanup failed for {op_id}: {err}");
    }
}

/// Roll back a staged copy before the target DB stage commits.
pub(super) fn rollback_staged_copy(
    target_db: &crate::sample_sources::SourceDatabase,
    prepared: &PreparedStagedCopy,
) {
    let _ = std::fs::remove_file(&prepared.staged_absolute);
    clear_file_op_journal_entry(target_db, &prepared.op_id);
}

/// Roll back a staged move before the source DB cleanup begins.
pub(super) fn rollback_staged_move(
    target_db: &crate::sample_sources::SourceDatabase,
    prepared: &PreparedStagedMove,
) {
    let _ = move_sample_file(&prepared.staged_absolute, &prepared.source_absolute);
    clear_file_op_journal_entry(target_db, &prepared.op_id);
}

/// Roll back a staged move after the target DB row exists but before finalize succeeds.
pub(super) fn rollback_staged_move_after_target_db_stage(
    target_db: &crate::sample_sources::SourceDatabase,
    prepared: &PreparedStagedMove,
    target_relative: &Path,
) {
    let mut cleanup_complete = true;
    if let Err(err) = target_db.remove_file(target_relative) {
        cleanup_complete = false;
        warn!(
            "Drop target rollback could not remove target DB row {}: {err}",
            target_relative.display()
        );
    }
    if let Err(err) = move_sample_file(&prepared.staged_absolute, &prepared.source_absolute) {
        cleanup_complete = false;
        warn!(
            "Drop target rollback could not restore source file {}: {err}",
            prepared.source_absolute.display()
        );
    }
    if cleanup_complete {
        clear_file_op_journal_entry(target_db, &prepared.op_id);
    }
}
