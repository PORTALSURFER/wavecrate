use super::super::db;
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};

pub(super) fn conn_with_schema() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE samples (
            sample_id TEXT PRIMARY KEY,
            content_hash TEXT NOT NULL,
            size INTEGER NOT NULL,
            mtime_ns INTEGER NOT NULL,
            duration_seconds REAL,
            sr_used INTEGER,
            analysis_version TEXT,
            bpm REAL
        );
        CREATE TABLE embeddings (
            sample_id TEXT PRIMARY KEY,
            model_id TEXT NOT NULL,
            dim INTEGER NOT NULL,
            dtype TEXT NOT NULL,
            l2_normed INTEGER NOT NULL,
            vec BLOB NOT NULL,
            created_at INTEGER NOT NULL
        ) WITHOUT ROWID;
        CREATE TABLE similarity_aspect_descriptors (
            sample_id TEXT PRIMARY KEY,
            model_id TEXT NOT NULL,
            dim INTEGER NOT NULL,
            dtype TEXT NOT NULL,
            l2_normed INTEGER NOT NULL,
            valid_mask INTEGER NOT NULL,
            vec BLOB NOT NULL,
            created_at INTEGER NOT NULL
        ) WITHOUT ROWID;
        CREATE TABLE features (
            sample_id TEXT PRIMARY KEY,
            feat_version INTEGER NOT NULL,
            vec_blob BLOB NOT NULL,
            light_dsp_blob BLOB,
            rms REAL,
            computed_at INTEGER NOT NULL
        ) WITHOUT ROWID;
        CREATE TABLE analysis_cache_features (
            content_hash TEXT NOT NULL,
            analysis_version TEXT NOT NULL,
            feat_version INTEGER NOT NULL,
            vec_blob BLOB NOT NULL,
            light_dsp_blob BLOB,
            rms REAL,
            computed_at INTEGER NOT NULL,
            duration_seconds REAL NOT NULL,
            sr_used INTEGER NOT NULL,
            PRIMARY KEY (content_hash)
        );
        CREATE TABLE analysis_cache_embeddings (
            content_hash TEXT NOT NULL,
            analysis_version TEXT NOT NULL,
            model_id TEXT NOT NULL,
            dim INTEGER NOT NULL,
            dtype TEXT NOT NULL,
            l2_normed INTEGER NOT NULL,
            vec BLOB NOT NULL,
            created_at INTEGER NOT NULL,
            PRIMARY KEY (content_hash, model_id)
        );
        CREATE TABLE analysis_cache_aspect_descriptors (
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
        );
        CREATE TABLE metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE analysis_jobs (
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
        );",
    )
    .unwrap();
    conn
}

pub(super) fn insert_sample(conn: &Connection, sample_id: &str, content_hash: &str) {
    conn.execute(
        "INSERT INTO samples (sample_id, content_hash, size, mtime_ns)
         VALUES (?1, ?2, 1, 1)",
        params![sample_id, content_hash],
    )
    .unwrap();
}

pub(super) fn make_job(sample_ids: &[&str], root: &Path) -> db::ClaimedJob {
    let payload = serde_json::to_string(sample_ids).unwrap();
    db::ClaimedJob {
        id: 1,
        sample_id: sample_ids.first().unwrap_or(&"").to_string(),
        content_hash: Some(payload),
        job_type: "embedding_backfill".to_string(),
        source_root: root.to_path_buf(),
    }
}

pub(super) fn make_work(id: &str) -> super::super::model::EmbeddingWork {
    super::super::model::EmbeddingWork {
        content_hash: format!("hash-{id}"),
        absolute_path: PathBuf::from(format!("dummy/{id}.wav")),
        sample_ids: vec![id.to_string()],
    }
}

pub(super) fn count_rows(conn: &Connection, table: &str) -> i64 {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    conn.query_row(&sql, [], |row| row.get(0)).unwrap()
}
