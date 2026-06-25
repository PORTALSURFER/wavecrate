use std::path::{Path, PathBuf};

use super::super::super::move_transaction::{
    move_sample_file, prepare_staged_copy, prepare_staged_move,
};
use super::super::DroppedSampleMetadata;
use super::super::transactions::{
    clear_file_op_journal_entry, register_drop_target_target_entry, rollback_staged_copy,
    rollback_staged_move, rollback_staged_move_after_target_db_stage, sample_move_metadata,
    warn_on_journal_stage_update,
};
use crate::app::controller::jobs::{DropTargetTransferRequest, DropTargetTransferSuccess};
use crate::sample_sources::SourceDatabase;
use crate::sample_sources::db::file_ops_journal;

pub(super) fn run_drop_target_copy(
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
        sample_move_metadata(&metadata),
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
        &metadata,
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
    Some(drop_target_transfer_success(
        request,
        target_relative,
        prepared.file_size,
        prepared.modified_ns,
        metadata,
    ))
}

pub(super) fn run_drop_target_move(
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
        sample_move_metadata(&metadata),
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
        &metadata,
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
    Some(drop_target_transfer_success(
        request,
        target_relative,
        prepared.file_size,
        prepared.modified_ns,
        metadata,
    ))
}

fn drop_target_transfer_success(
    request: DropTargetTransferRequest,
    target_relative: PathBuf,
    file_size: u64,
    modified_ns: i64,
    metadata: DroppedSampleMetadata,
) -> DropTargetTransferSuccess {
    DropTargetTransferSuccess {
        source_id: request.source_id,
        source_relative: request.relative_path,
        target_relative,
        file_size,
        modified_ns,
        tag: metadata.tag,
        looped: metadata.looped,
        locked: metadata.locked,
        last_played_at: metadata.last_played_at,
        last_curated_at: metadata.last_curated_at,
        sound_type: metadata.sound_type,
        user_tag: metadata.user_tag.clone(),
        normal_tags: metadata.normal_tags.clone(),
        collection: metadata.collection,
    }
}
