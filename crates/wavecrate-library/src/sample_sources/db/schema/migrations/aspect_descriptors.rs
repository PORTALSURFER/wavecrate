use rusqlite::Connection;

use super::super::super::SourceDbError;
use super::super::super::util::map_sql_error;

pub(super) fn ensure_aspect_descriptor_tables(
    connection: &Connection,
) -> Result<(), SourceDbError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS similarity_aspect_descriptors (
                sample_id TEXT PRIMARY KEY,
                model_id TEXT NOT NULL,
                dim INTEGER NOT NULL,
                dtype TEXT NOT NULL,
                l2_normed INTEGER NOT NULL,
                valid_mask INTEGER NOT NULL,
                vec BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY(sample_id) REFERENCES samples(sample_id) ON DELETE CASCADE
            ) WITHOUT ROWID;
            CREATE INDEX IF NOT EXISTS idx_similarity_aspect_descriptors_model_id
                ON similarity_aspect_descriptors (model_id);
            CREATE TABLE IF NOT EXISTS analysis_cache_aspect_descriptors (
                content_hash TEXT NOT NULL,
                analysis_version TEXT NOT NULL,
                model_id TEXT NOT NULL,
                dim INTEGER NOT NULL,
                dtype TEXT NOT NULL,
                l2_normed INTEGER NOT NULL,
                valid_mask INTEGER NOT NULL,
                vec BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (content_hash, model_id)
            ) WITHOUT ROWID;
            CREATE INDEX IF NOT EXISTS idx_cache_aspect_descriptors_model_id
                ON analysis_cache_aspect_descriptors (model_id);",
        )
        .map_err(map_sql_error)?;
    Ok(())
}
