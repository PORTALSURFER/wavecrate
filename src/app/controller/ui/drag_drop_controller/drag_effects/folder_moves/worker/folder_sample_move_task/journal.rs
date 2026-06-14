use super::*;
use crate::sample_sources::db::file_ops_journal;

impl FolderSampleMoveTransaction<'_> {
    pub(super) fn record_committed_db_stages(&self, errors: &mut Vec<String>) {
        if let Err(err) = file_ops_journal::update_stage(
            self.db,
            &self.prepared.op_id,
            file_ops_journal::FileOpStage::TargetDb,
            None,
            None,
        ) {
            errors.push(format!("Failed to update move journal: {err}"));
        }
        if let Err(err) = file_ops_journal::update_stage(
            self.db,
            &self.prepared.op_id,
            file_ops_journal::FileOpStage::SourceDb,
            None,
            None,
        ) {
            errors.push(format!("Failed to update move journal: {err}"));
        }
    }

    pub(super) fn clear_move_journal_entry(&self, errors: &mut Vec<String>) {
        super::super::super::super::move_transaction::remove_move_journal_entry(
            errors,
            self.db,
            &self.prepared.op_id,
        );
    }
}
