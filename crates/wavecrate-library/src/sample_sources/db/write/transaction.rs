use super::super::util::map_sql_error;
use super::super::{SourceDatabase, SourceDbError, SourceWriteBatch};

impl SourceWriteBatch<'_> {
    /// Commit all batched operations atomically.
    pub fn commit(self) -> Result<(), SourceDbError> {
        SourceDatabase::bump_revision(&self.tx)?;
        if self.paths_revision_dirty {
            SourceDatabase::bump_wav_paths_revision(&self.tx)?;
        }
        self.tx.commit().map_err(map_sql_error)?;
        crate::sqlite_wal::maybe_checkpoint_database_file(
            &self.db_path,
            "source_db",
            self.telemetry_label,
        );
        Ok(())
    }
}
