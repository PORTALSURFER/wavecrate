use super::{LibraryDatabase, LibraryError, map_sql_error};

impl LibraryDatabase {
    pub(super) fn apply_pragmas(&self) -> Result<(), LibraryError> {
        let pragmas = "PRAGMA journal_mode=WAL;
                 PRAGMA synchronous = NORMAL;
                 PRAGMA foreign_keys=ON;
                 PRAGMA busy_timeout=5000;
                 PRAGMA temp_store=MEMORY;
                 PRAGMA cache_size=-64000;
                 PRAGMA mmap_size=268435456;";
        self.connection
            .execute_batch(pragmas)
            .map_err(map_sql_error)?;
        crate::sqlite_wal::apply_workload_wal_pragmas(&self.connection).map_err(map_sql_error)?;
        if let Err(err) = crate::sqlite_ext::try_load_optional_extension(&self.connection) {
            tracing::debug!("SQLite extension not loaded: {err}");
        }
        Ok(())
    }

    pub(super) fn apply_schema(&self) -> Result<(), LibraryError> {
        self.connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS metadata (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                 CREATE TABLE IF NOT EXISTS sources (
                    id TEXT PRIMARY KEY,
                    root TEXT NOT NULL,
                    sort_order INTEGER NOT NULL,
                    role TEXT NOT NULL DEFAULT 'normal',
                    metadata_storage TEXT NOT NULL DEFAULT 'source_folder',
                    primary_import_folder TEXT NOT NULL DEFAULT '_Wavecrate Inbox'
                );
                 CREATE TABLE IF NOT EXISTS analysis_jobs (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    sample_id TEXT NOT NULL,
                    job_type TEXT NOT NULL,
                    content_hash TEXT,
                    status TEXT NOT NULL,
                    attempts INTEGER NOT NULL DEFAULT 0,
                    created_at INTEGER NOT NULL,
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
                    light_dsp_blob BLOB,
                    rms REAL,
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
                    light_dsp_blob BLOB,
                    rms REAL,
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
                 CREATE TABLE IF NOT EXISTS harvest_files (
                    source_id TEXT NOT NULL,
                    relative_path TEXT NOT NULL,
                    file_size INTEGER,
                    modified_ns INTEGER,
                    content_hash TEXT,
                    harvest_state TEXT NOT NULL DEFAULT 'new',
                    discovered_at INTEGER NOT NULL,
                    seen_at INTEGER,
                    touched_at INTEGER,
                    done_at INTEGER,
                    ignored_at INTEGER,
                    note TEXT,
                    PRIMARY KEY (source_id, relative_path)
                 ) WITHOUT ROWID;
                 CREATE INDEX IF NOT EXISTS idx_harvest_files_source_state
                    ON harvest_files (source_id, harvest_state, relative_path);
                 CREATE INDEX IF NOT EXISTS idx_harvest_files_content_hash
                    ON harvest_files (content_hash);
                 CREATE TABLE IF NOT EXISTS harvest_derivations (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    parent_source_id TEXT NOT NULL,
                    parent_relative_path TEXT NOT NULL,
                    parent_file_size INTEGER,
                    parent_modified_ns INTEGER,
                    parent_content_hash TEXT,
                    child_source_id TEXT NOT NULL,
                    child_relative_path TEXT NOT NULL,
                    child_file_size INTEGER,
                    child_modified_ns INTEGER,
                    child_content_hash TEXT,
                    operation TEXT NOT NULL,
                    source_range_start REAL,
                    source_range_end REAL,
                    output_duration_seconds REAL,
                    destination_folder TEXT,
                    inherited_rating INTEGER,
                    inherited_tags_json TEXT NOT NULL DEFAULT '[]',
                    inherited_playback_type TEXT,
                    tool_version TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS idx_harvest_derivations_parent
                    ON harvest_derivations (parent_source_id, parent_relative_path, created_at, id);
                 CREATE INDEX IF NOT EXISTS idx_harvest_derivations_child
                    ON harvest_derivations (child_source_id, child_relative_path, created_at, id);
                 CREATE INDEX IF NOT EXISTS idx_harvest_derivations_parent_hash
                    ON harvest_derivations (parent_content_hash);
                 CREATE INDEX IF NOT EXISTS idx_harvest_derivations_child_hash
                    ON harvest_derivations (child_content_hash);",
            )
            .map_err(map_sql_error)?;
        Ok(())
    }
}
