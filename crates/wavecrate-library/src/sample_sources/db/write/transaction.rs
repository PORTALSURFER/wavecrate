use std::path::PathBuf;

use rusqlite::OptionalExtension;

use super::super::util::{map_sql_error, normalize_relative_path};
use super::super::{SourceDatabase, SourceDbError, SourceManifestEntry, SourceWriteBatch};

impl SourceWriteBatch<'_> {
    /// Commit all batched operations atomically.
    pub fn commit(self) -> Result<(), SourceDbError> {
        self.prepare_commit()?;
        self.tx.commit().map_err(map_sql_error)?;
        crate::sqlite_wal::maybe_checkpoint_database_file(
            &self.db_path,
            "source_db",
            self.telemetry_label,
        );
        Ok(())
    }

    /// Commit the batch and return the manifest snapshot owned by that exact revision.
    ///
    /// The snapshot is read from the active write transaction after its revision bump and before
    /// `COMMIT`. A later writer therefore cannot advance the returned revision or alter the
    /// returned manifest between the authoritative mutation and delta publication.
    pub fn commit_with_manifest_snapshot(
        self,
    ) -> Result<(u64, Vec<SourceManifestEntry>), SourceDbError> {
        self.prepare_commit()?;
        let snapshot = manifest_snapshot(&self.tx)?;
        self.tx.commit().map_err(map_sql_error)?;
        crate::sqlite_wal::maybe_checkpoint_database_file(
            &self.db_path,
            "source_db",
            self.telemetry_label,
        );
        Ok(snapshot)
    }

    fn prepare_commit(&self) -> Result<(), SourceDbError> {
        SourceDatabase::bump_revision(&self.tx)?;
        if self.paths_revision_dirty {
            SourceDatabase::bump_wav_paths_revision(&self.tx)?;
        }
        Ok(())
    }
}

fn manifest_snapshot(
    connection: &rusqlite::Connection,
) -> Result<(u64, Vec<SourceManifestEntry>), SourceDbError> {
    let revision = connection
        .query_row(
            "SELECT value FROM metadata WHERE key = 'revision'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(map_sql_error)?
        .map(|raw| raw.parse::<u64>().map_err(|_| SourceDbError::Unexpected))
        .transpose()?
        .unwrap_or_default();
    let filter = crate::sample_sources::supported_audio_where_clause();
    let sql = format!(
        "SELECT path, file_identity, content_hash, file_size, modified_ns
         FROM wav_files
         WHERE {filter} AND missing = 0
         ORDER BY path ASC"
    );
    let raw_entries = {
        let mut statement = connection.prepare(&sql).map_err(map_sql_error)?;
        statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get::<_, i64>(3)?,
                    row.get(4)?,
                ))
            })
            .map_err(map_sql_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(map_sql_error)?
    };
    let entries = raw_entries
        .into_iter()
        .filter_map(
            |(raw_path, file_identity, content_hash, file_size, modified_ns)| {
                let normalized = match normalize_relative_path(std::path::Path::new(&raw_path)) {
                    Ok(normalized) => normalized,
                    Err(error) => {
                        tracing::warn!(
                            path = raw_path,
                            %error,
                            "Skipping source manifest row with invalid relative path"
                        );
                        return None;
                    }
                };
                Some(SourceManifestEntry {
                    relative_path: PathBuf::from(normalized),
                    file_identity,
                    content_hash,
                    file_size: file_size.max(0) as u64,
                    modified_ns,
                })
            },
        )
        .collect();
    Ok((revision, entries))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn commit_snapshot_stays_bound_to_its_own_revision() {
        let directory = tempfile::tempdir().expect("source root");
        let database = SourceDatabase::open(directory.path()).expect("source database");
        let mut first = database.write_batch().expect("first batch");
        first
            .upsert_file_with_hash(Path::new("first.wav"), 5, 10, "first-hash")
            .expect("insert first file");
        let (committed_revision, committed_manifest) = first
            .commit_with_manifest_snapshot()
            .expect("commit first manifest");

        let mut second = database.write_batch().expect("second batch");
        second
            .upsert_file_with_hash(Path::new("second.wav"), 6, 20, "second-hash")
            .expect("insert second file");
        second.commit().expect("commit second manifest");

        assert_eq!(committed_manifest.len(), 1);
        assert_eq!(committed_manifest[0].relative_path, Path::new("first.wav"));
        assert!(database.get_revision().expect("current revision") > committed_revision);
    }
}
