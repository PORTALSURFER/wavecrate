use rusqlite::{Connection, params};

pub(super) struct TestDb {
    pub(super) conn: Connection,
}

impl TestDb {
    pub(super) fn new() -> Self {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA_SQL).unwrap();
        Self { conn }
    }

    pub(super) fn insert_sample(&self, row: SampleRow<'_>) {
        self.conn
            .execute(
                "INSERT INTO samples (
                    sample_id,
                    content_hash,
                    size,
                    mtime_ns,
                    duration_seconds,
                    analysis_version,
                    long_sample_mark
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    row.sample_id,
                    row.content_hash,
                    row.size,
                    row.mtime_ns,
                    row.duration_seconds,
                    row.analysis_version,
                    row.long_sample_mark,
                ],
            )
            .unwrap();
    }

    pub(super) fn insert_wav_file(&self, path: &str) {
        self.conn
            .execute(
                "INSERT INTO wav_files (path, file_size, modified_ns, tag, missing)
                 VALUES (?1, 1, 1, 0, 0)",
                params![path],
            )
            .unwrap();
    }

    pub(super) fn insert_job(&self, row: JobRow<'_>) {
        self.conn
            .execute(
                "INSERT INTO analysis_jobs (
                    sample_id,
                    source_id,
                    relative_path,
                    job_type,
                    status,
                    attempts,
                    created_at,
                    running_at,
                    last_error
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    row.sample_id,
                    row.source_id,
                    row.relative_path,
                    row.job_type,
                    row.status,
                    row.attempts,
                    row.created_at,
                    row.running_at,
                    row.last_error,
                ],
            )
            .unwrap();
    }

    pub(super) fn insert_orphaned_artifacts(&self, sample_id: &str) {
        self.conn
            .execute(
                "INSERT INTO analysis_features (sample_id, content_hash, features)
                 VALUES (?1, 'h', NULL)",
                params![sample_id],
            )
            .unwrap();
        self.conn
            .execute(
                "INSERT INTO features (sample_id, feat_version, vec_blob, computed_at)
                 VALUES (?1, 1, x'00', 0)",
                params![sample_id],
            )
            .unwrap();
        self.conn
            .execute(
                "INSERT INTO embeddings (sample_id, model_id, dim, dtype, l2_normed, vec, created_at)
                 VALUES (?1, 'm', 1, 'f32', 1, x'00', 0)",
                params![sample_id],
            )
            .unwrap();
    }
}

pub(super) struct SampleRow<'a> {
    sample_id: &'a str,
    content_hash: &'a str,
    size: i64,
    mtime_ns: i64,
    duration_seconds: Option<f64>,
    analysis_version: Option<&'a str>,
    long_sample_mark: Option<i64>,
}

impl<'a> SampleRow<'a> {
    pub(super) fn new(sample_id: &'a str, content_hash: &'a str) -> Self {
        Self {
            sample_id,
            content_hash,
            size: 1,
            mtime_ns: 1,
            duration_seconds: None,
            analysis_version: None,
            long_sample_mark: None,
        }
    }

    pub(super) fn with_file_state(mut self, size: i64, mtime_ns: i64) -> Self {
        self.size = size;
        self.mtime_ns = mtime_ns;
        self
    }

    pub(super) fn with_duration(mut self, duration_seconds: f64) -> Self {
        self.duration_seconds = Some(duration_seconds);
        self
    }

    pub(super) fn with_analysis_version(mut self, analysis_version: &'a str) -> Self {
        self.analysis_version = Some(analysis_version);
        self
    }

    pub(super) fn with_long_mark(mut self, long_sample_mark: i64) -> Self {
        self.long_sample_mark = Some(long_sample_mark);
        self
    }
}

pub(super) struct JobRow<'a> {
    sample_id: &'a str,
    source_id: &'a str,
    relative_path: &'a str,
    job_type: &'a str,
    status: &'a str,
    attempts: i64,
    created_at: i64,
    running_at: Option<i64>,
    last_error: Option<&'a str>,
}

impl<'a> JobRow<'a> {
    pub(super) fn new(sample_id: &'a str, job_type: &'a str, status: &'a str) -> Self {
        Self {
            sample_id,
            source_id: "",
            relative_path: "",
            job_type,
            status,
            attempts: 0,
            created_at: 0,
            running_at: None,
            last_error: None,
        }
    }

    pub(super) fn with_source(mut self, source_id: &'a str, relative_path: &'a str) -> Self {
        self.source_id = source_id;
        self.relative_path = relative_path;
        self
    }

    pub(super) fn with_attempts(mut self, attempts: i64) -> Self {
        self.attempts = attempts;
        self
    }

    pub(super) fn with_created_at(mut self, created_at: i64) -> Self {
        self.created_at = created_at;
        self
    }

    pub(super) fn with_running_at(mut self, running_at: i64) -> Self {
        self.running_at = Some(running_at);
        self
    }

    pub(super) fn with_last_error(mut self, last_error: &'a str) -> Self {
        self.last_error = Some(last_error);
        self
    }
}

const SCHEMA_SQL: &str = "CREATE TABLE analysis_jobs (
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
    CREATE TABLE samples (
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
    CREATE TABLE wav_files (
        path TEXT PRIMARY KEY,
        file_size INTEGER NOT NULL,
        modified_ns INTEGER NOT NULL,
        tag INTEGER NOT NULL DEFAULT 0,
        missing INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE analysis_features (
        sample_id TEXT PRIMARY KEY,
        content_hash TEXT NOT NULL,
        features BLOB
    );
    CREATE TABLE features (
        sample_id TEXT PRIMARY KEY,
        feat_version INTEGER NOT NULL,
        vec_blob BLOB NOT NULL,
        computed_at INTEGER NOT NULL
    ) WITHOUT ROWID;
    CREATE TABLE embeddings (
        sample_id TEXT PRIMARY KEY,
        model_id TEXT NOT NULL,
        dim INTEGER NOT NULL,
        dtype TEXT NOT NULL,
        l2_normed INTEGER NOT NULL,
        vec BLOB NOT NULL,
        created_at INTEGER NOT NULL
    ) WITHOUT ROWID;
    CREATE TABLE metadata (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );";
