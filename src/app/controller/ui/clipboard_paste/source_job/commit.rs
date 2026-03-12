use super::*;
use crate::sample_sources::db::file_ops_journal;

pub(super) struct DbCommittedSourcePaste {
    pub(super) staged: StagedSourcePaste,
}

/// Persist staged copy metadata into the source DB before filesystem finalize.
pub(super) fn commit_source_copy(
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
