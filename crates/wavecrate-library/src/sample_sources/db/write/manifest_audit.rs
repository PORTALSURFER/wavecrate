use std::path::PathBuf;

use rusqlite::{Transaction, TransactionBehavior, params};

use super::super::util::{map_sql_error, normalize_relative_path, parse_relative_path_from_db};
use super::super::{META_LAST_MANIFEST_AUDIT_AT, SourceDatabase, SourceDbError, SourceWriteBatch};

impl SourceDatabase {
    /// Start a durable manifest-audit cycle or recover the paths already checked by an
    /// interrupted cycle.
    ///
    /// Audit bookkeeping deliberately does not advance the source revision: it is private scan
    /// progress, not a committed manifest mutation.
    pub fn begin_or_resume_manifest_audit(
        &self,
        started_at: i64,
    ) -> Result<Vec<PathBuf>, SourceDbError> {
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(map_sql_error)?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO source_manifest_audit_state (
                    singleton, started_at, checked_files
                 ) VALUES (1, ?1, 0)",
                [started_at],
            )
            .map_err(map_sql_error)?;
        let raw_paths = {
            let mut statement = transaction
                .prepare("SELECT path FROM source_manifest_audit_seen ORDER BY path ASC")
                .map_err(map_sql_error)?;
            statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(map_sql_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(map_sql_error)?
        };
        transaction.commit().map_err(map_sql_error)?;
        Ok(raw_paths
            .into_iter()
            .filter_map(|path| match parse_relative_path_from_db(&path) {
                Ok(path) => Some(path),
                Err(error) => {
                    tracing::warn!(
                        %error,
                        path,
                        "Skipping invalid durable manifest-audit checkpoint path"
                    );
                    None
                }
            })
            .collect())
    }

    /// Durably checkpoint paths completed by the current manifest-audit cycle.
    pub fn checkpoint_manifest_audit_paths(&self, paths: &[PathBuf]) -> Result<(), SourceDbError> {
        if paths.is_empty() {
            return Ok(());
        }
        let transaction =
            Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
                .map_err(map_sql_error)?;
        {
            let mut insert = transaction
                .prepare("INSERT OR IGNORE INTO source_manifest_audit_seen (path) VALUES (?1)")
                .map_err(map_sql_error)?;
            for path in paths {
                let normalized = normalize_relative_path(path)?;
                insert.execute([normalized]).map_err(map_sql_error)?;
            }
        }
        transaction
            .execute(
                "UPDATE source_manifest_audit_state
                 SET checked_files = (
                     SELECT COUNT(*) FROM source_manifest_audit_seen
                 )
                 WHERE singleton = 1",
                [],
            )
            .map_err(map_sql_error)?;
        transaction.commit().map_err(map_sql_error)?;
        Ok(())
    }
}

impl SourceWriteBatch<'_> {
    /// Finish a complete manifest-audit cycle in the same transaction that publishes its
    /// completion timestamp.
    pub fn complete_manifest_audit(&mut self, completed_at: i64) -> Result<(), SourceDbError> {
        self.set_metadata(META_LAST_MANIFEST_AUDIT_AT, &completed_at.to_string())?;
        self.tx
            .execute("DELETE FROM source_manifest_audit_seen", [])
            .map_err(map_sql_error)?;
        self.tx
            .execute(
                "DELETE FROM source_manifest_audit_state WHERE singleton = ?1",
                params![1],
            )
            .map_err(map_sql_error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn interrupted_manifest_audit_recovers_only_durable_checked_paths() {
        let directory = tempfile::tempdir().expect("source");
        let database = SourceDatabase::open_for_source_write(directory.path()).expect("database");

        assert!(
            database
                .begin_or_resume_manifest_audit(10)
                .expect("begin audit")
                .is_empty()
        );
        database
            .checkpoint_manifest_audit_paths(&[
                Path::new("a.wav").to_path_buf(),
                Path::new("nested/b.wav").to_path_buf(),
            ])
            .expect("checkpoint audit");

        assert_eq!(
            database
                .begin_or_resume_manifest_audit(20)
                .expect("resume audit"),
            vec![PathBuf::from("a.wav"), PathBuf::from("nested/b.wav")]
        );

        let mut batch = database.write_batch().expect("completion batch");
        batch.complete_manifest_audit(30).expect("complete audit");
        batch.commit().expect("commit completion");

        assert!(
            database
                .begin_or_resume_manifest_audit(40)
                .expect("new audit")
                .is_empty()
        );
        assert_eq!(
            database
                .get_metadata(META_LAST_MANIFEST_AUDIT_AT)
                .expect("audit metadata")
                .as_deref(),
            Some("30")
        );
    }
}
