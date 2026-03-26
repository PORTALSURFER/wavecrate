//! Base DDL for source database tables and indices.

use rusqlite::Connection;

use super::super::SourceDbError;
use super::super::util::map_sql_error;

const BASE_SCHEMA_SQL: &str = "CREATE TABLE IF NOT EXISTS metadata (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS wav_files (
        path TEXT PRIMARY KEY,
        file_size INTEGER NOT NULL,
        modified_ns INTEGER NOT NULL,
        tag INTEGER NOT NULL DEFAULT 0,
        looped INTEGER NOT NULL DEFAULT 0,
        locked INTEGER NOT NULL DEFAULT 0,
        missing INTEGER NOT NULL DEFAULT 0,
        extension TEXT NOT NULL DEFAULT '',
        last_played_at INTEGER
    );
    CREATE TABLE IF NOT EXISTS analysis_jobs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        sample_id TEXT NOT NULL,
        source_id TEXT NOT NULL DEFAULT '',
        relative_path TEXT NOT NULL DEFAULT '',
        job_type TEXT NOT NULL,
        content_hash TEXT,
        status TEXT NOT NULL,
        attempts INTEGER NOT NULL DEFAULT 0,
        created_at INTEGER NOT NULL,
        running_at INTEGER,
        last_error TEXT,
        UNIQUE(sample_id, job_type)
    );
    CREATE INDEX IF NOT EXISTS idx_analysis_jobs_status_created_id
        ON analysis_jobs (status, created_at, id);
    CREATE INDEX IF NOT EXISTS idx_analysis_jobs_status_sample_id
        ON analysis_jobs (status, sample_id);
    CREATE TABLE IF NOT EXISTS samples (
        sample_id TEXT PRIMARY KEY,
        content_hash TEXT NOT NULL,
        size INTEGER NOT NULL,
        mtime_ns INTEGER NOT NULL,
        duration_seconds REAL,
        sr_used INTEGER,
        analysis_version TEXT,
        bpm REAL,
        long_sample_mark INTEGER
    );
    CREATE TABLE IF NOT EXISTS analysis_features (
        sample_id TEXT PRIMARY KEY,
        content_hash TEXT NOT NULL,
        features BLOB
    );
    CREATE TABLE IF NOT EXISTS features (
        sample_id TEXT PRIMARY KEY,
        feat_version INTEGER NOT NULL,
        vec_blob BLOB NOT NULL,
        computed_at INTEGER NOT NULL
    ) WITHOUT ROWID;
    CREATE TABLE IF NOT EXISTS layout_umap (
        sample_id TEXT PRIMARY KEY,
        model_id TEXT NOT NULL,
        umap_version TEXT NOT NULL,
        x REAL NOT NULL,
        y REAL NOT NULL,
        created_at INTEGER NOT NULL,
        FOREIGN KEY(sample_id) REFERENCES samples(sample_id) ON DELETE CASCADE
    ) WITHOUT ROWID;
    CREATE INDEX IF NOT EXISTS idx_layout_umap_model_version
        ON layout_umap (model_id, umap_version);
    CREATE INDEX IF NOT EXISTS idx_layout_umap_xy
        ON layout_umap (x, y);
    CREATE TABLE IF NOT EXISTS hdbscan_clusters (
        sample_id TEXT NOT NULL,
        model_id TEXT NOT NULL,
        method TEXT NOT NULL,
        umap_version TEXT NOT NULL,
        cluster_id INTEGER NOT NULL,
        created_at INTEGER NOT NULL,
        PRIMARY KEY (sample_id, model_id, method, umap_version),
        FOREIGN KEY(sample_id) REFERENCES samples(sample_id) ON DELETE CASCADE
    ) WITHOUT ROWID;
    CREATE INDEX IF NOT EXISTS idx_hdbscan_clusters_set
        ON hdbscan_clusters (model_id, method, umap_version);
    CREATE INDEX IF NOT EXISTS idx_hdbscan_clusters_cluster_id
        ON hdbscan_clusters (cluster_id);
    CREATE TABLE IF NOT EXISTS embeddings (
        sample_id TEXT PRIMARY KEY,
        model_id TEXT NOT NULL,
        dim INTEGER NOT NULL,
        dtype TEXT NOT NULL,
        l2_normed INTEGER NOT NULL,
        vec BLOB NOT NULL,
        created_at INTEGER NOT NULL
    ) WITHOUT ROWID;
    CREATE INDEX IF NOT EXISTS idx_embeddings_model_id ON embeddings (model_id);
    CREATE TABLE IF NOT EXISTS analysis_cache_features (
        content_hash TEXT PRIMARY KEY,
        analysis_version TEXT NOT NULL,
        feat_version INTEGER NOT NULL,
        vec_blob BLOB NOT NULL,
        computed_at INTEGER NOT NULL,
        duration_seconds REAL NOT NULL,
        sr_used INTEGER NOT NULL
    ) WITHOUT ROWID;
    CREATE TABLE IF NOT EXISTS analysis_cache_embeddings (
        content_hash TEXT NOT NULL,
        analysis_version TEXT NOT NULL,
        model_id TEXT NOT NULL,
        dim INTEGER NOT NULL,
        dtype TEXT NOT NULL,
        l2_normed INTEGER NOT NULL,
        vec BLOB NOT NULL,
        created_at INTEGER NOT NULL,
        PRIMARY KEY (content_hash, model_id)
    ) WITHOUT ROWID;
    CREATE INDEX IF NOT EXISTS idx_cache_embeddings_model_id
        ON analysis_cache_embeddings (model_id);
    CREATE TABLE IF NOT EXISTS ann_index_meta (
        model_id TEXT PRIMARY KEY,
        index_path TEXT NOT NULL,
        count INTEGER NOT NULL,
        params_json TEXT NOT NULL,
        updated_at INTEGER NOT NULL
    ) WITHOUT ROWID;
    CREATE TABLE IF NOT EXISTS file_ops_journal (
        id TEXT PRIMARY KEY,
        op_type TEXT NOT NULL,
        stage TEXT NOT NULL,
        source_root TEXT,
        source_relative TEXT,
        target_relative TEXT NOT NULL,
        staged_relative TEXT,
        file_size INTEGER,
        modified_ns INTEGER,
        tag INTEGER,
        looped INTEGER,
        locked INTEGER,
        last_played_at INTEGER,
        created_at INTEGER NOT NULL
    );";

const INDEX_SQL: &str = "CREATE INDEX IF NOT EXISTS idx_wav_files_missing
         ON wav_files(path) WHERE missing != 0;
     CREATE INDEX IF NOT EXISTS idx_wav_files_extension
         ON wav_files(extension);
     CREATE INDEX IF NOT EXISTS idx_analysis_jobs_source_job_status_created
         ON analysis_jobs (source_id, job_type, status, created_at);
     CREATE INDEX IF NOT EXISTS idx_analysis_jobs_job_status
         ON analysis_jobs (job_type, status);
     CREATE INDEX IF NOT EXISTS idx_file_ops_journal_stage
         ON file_ops_journal (stage);";

/// Create all base tables used by the source database.
pub(super) fn apply_base_schema(connection: &Connection) -> Result<(), SourceDbError> {
    connection
        .execute_batch(BASE_SCHEMA_SQL)
        .map_err(map_sql_error)?;
    Ok(())
}

/// Create all supporting indices used by the source database.
pub(super) fn apply_indices(connection: &Connection) -> Result<(), SourceDbError> {
    connection.execute_batch(INDEX_SQL).map_err(map_sql_error)?;
    Ok(())
}
