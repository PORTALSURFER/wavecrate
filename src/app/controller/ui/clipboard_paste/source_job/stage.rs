use super::*;
use crate::app::controller::library::wav_io::file_metadata;
use crate::sample_sources::db::file_ops_journal;

pub(super) struct StagedSourcePaste {
    pub(super) prepared: PreparedSourcePaste,
    pub(super) file_size: u64,
    pub(super) modified_ns: i64,
}

/// Copy one source file into the staged journal path and persist stage metadata.
pub(super) fn stage_source_copy(
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
