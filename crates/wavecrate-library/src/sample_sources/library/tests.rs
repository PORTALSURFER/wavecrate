use super::*;
use rusqlite::{Connection, OptionalExtension};
use std::collections::HashSet;
use tempfile::tempdir;

struct TableContract {
    name: &'static str,
    columns: &'static [&'static str],
}

const LIBRARY_DB_SCHEMA_CONTRACT: &[TableContract] = &[
    TableContract {
        name: "metadata",
        columns: &["key", "value"],
    },
    TableContract {
        name: "sources",
        columns: &["id", "root", "sort_order"],
    },
    TableContract {
        name: "analysis_jobs",
        columns: &[
            "id",
            "sample_id",
            "job_type",
            "content_hash",
            "status",
            "attempts",
            "created_at",
            "last_error",
        ],
    },
    TableContract {
        name: "samples",
        columns: &[
            "sample_id",
            "content_hash",
            "size",
            "mtime_ns",
            "duration_seconds",
            "sr_used",
            "analysis_version",
            "bpm",
            "long_sample_mark",
        ],
    },
    TableContract {
        name: "analysis_features",
        columns: &["sample_id", "content_hash", "features"],
    },
    TableContract {
        name: "features",
        columns: &[
            "sample_id",
            "feat_version",
            "vec_blob",
            "light_dsp_blob",
            "rms",
            "computed_at",
        ],
    },
    TableContract {
        name: "layout_umap",
        columns: &[
            "sample_id",
            "model_id",
            "umap_version",
            "x",
            "y",
            "created_at",
        ],
    },
    TableContract {
        name: "hdbscan_clusters",
        columns: &[
            "sample_id",
            "model_id",
            "method",
            "umap_version",
            "cluster_id",
            "created_at",
        ],
    },
    TableContract {
        name: "embeddings",
        columns: &[
            "sample_id",
            "model_id",
            "dim",
            "dtype",
            "l2_normed",
            "vec",
            "created_at",
        ],
    },
    TableContract {
        name: "analysis_cache_features",
        columns: &[
            "content_hash",
            "analysis_version",
            "feat_version",
            "vec_blob",
            "light_dsp_blob",
            "rms",
            "computed_at",
            "duration_seconds",
            "sr_used",
        ],
    },
    TableContract {
        name: "analysis_cache_embeddings",
        columns: &[
            "content_hash",
            "analysis_version",
            "model_id",
            "dim",
            "dtype",
            "l2_normed",
            "vec",
            "created_at",
        ],
    },
    TableContract {
        name: "ann_index_meta",
        columns: &[
            "model_id",
            "index_path",
            "count",
            "params_json",
            "updated_at",
        ],
    },
];

fn with_config_home<T>(dir: &Path, f: impl FnOnce() -> T) -> T {
    let _guard = crate::app_dirs::ConfigBaseGuard::set(dir.to_path_buf());
    f()
}

#[test]
fn library_database_satisfies_schema_contract() {
    let temp = tempdir().unwrap();
    with_config_home(temp.path(), || {
        let conn = open_connection().unwrap();
        assert_library_db_schema_contract(&conn);
    });
}

#[test]
fn recovers_from_library_lock_poisoning() {
    let temp = tempdir().unwrap();
    with_config_home(temp.path(), || {
        let result = std::panic::catch_unwind(|| {
            let _guard = LIBRARY_LOCK.lock().unwrap();
            panic!("poison library lock");
        });
        assert!(result.is_err());

        let state = LibraryState {
            sources: Vec::new(),
        };
        save(&state).unwrap();
        let loaded = load().unwrap();
        assert!(loaded.sources.is_empty());
    });
}

fn assert_library_db_schema_contract(connection: &Connection) {
    for table in LIBRARY_DB_SCHEMA_CONTRACT {
        let actual = column_names(connection, table.name)
            .into_iter()
            .collect::<HashSet<_>>();
        assert!(
            !actual.is_empty(),
            "expected library DB table `{}` to exist",
            table.name
        );
        for column in table.columns {
            assert!(
                actual.contains(*column),
                "expected library DB table `{}` to contain column `{}`; actual columns: {:?}",
                table.name,
                column,
                actual
            );
        }
    }
}

fn column_names(connection: &Connection, table_name: &str) -> Vec<String> {
    let pragma = format!("PRAGMA table_info({table_name})");
    let mut stmt = connection.prepare(&pragma).unwrap();
    stmt.query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
}

#[test]
fn creates_embedding_tables() {
    let temp = tempdir().unwrap();
    with_config_home(temp.path(), || {
        let conn = open_connection().unwrap();
        for table in [
            "embeddings",
            "ann_index_meta",
            "layout_umap",
            "hdbscan_clusters",
        ] {
            let exists: Option<String> = conn
                .query_row(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .optional()
                .unwrap();
            assert_eq!(exists.as_deref(), Some(table));
        }
    });
}

#[test]
fn applies_workload_pragmas_and_indices() {
    let temp = tempdir().unwrap();
    with_config_home(temp.path(), || {
        let conn = open_connection().unwrap();

        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .unwrap();
        assert_eq!(journal_mode.to_ascii_lowercase(), "wal");

        let synchronous: i64 = conn
            .query_row("PRAGMA synchronous", [], |row| row.get(0))
            .unwrap();
        assert_eq!(synchronous, 1, "expected PRAGMA synchronous=NORMAL (1)");

        let wal_autocheckpoint: i64 = conn
            .query_row("PRAGMA wal_autocheckpoint", [], |row| row.get(0))
            .unwrap();
        assert_eq!(wal_autocheckpoint, 4096);

        let journal_size_limit: i64 = conn
            .query_row("PRAGMA journal_size_limit", [], |row| row.get(0))
            .unwrap();
        assert_eq!(journal_size_limit, 67_108_864);

        let busy_timeout: i64 = conn
            .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
            .unwrap();
        assert_eq!(busy_timeout, 5000);

        let idx: Option<String> = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type='index' AND name='idx_analysis_jobs_status_created_id'",
                [],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        assert_eq!(idx.as_deref(), Some("idx_analysis_jobs_status_created_id"));
    });
}

#[test]
fn reuses_known_source_id_for_same_root() {
    let temp = tempdir().unwrap();
    with_config_home(temp.path(), || {
        let root = normalize_path(Path::new("some/root"));
        let id = SourceId::new();
        save(&LibraryState {
            sources: vec![SampleSource::new_with_id(id.clone(), root.clone())],
        })
        .unwrap();

        // Simulate removal by saving with no sources; mapping should still be remembered.
        save(&LibraryState { sources: vec![] }).unwrap();

        let reused = lookup_source_id_for_root(&root)
            .unwrap()
            .expect("expected mapping");
        assert_eq!(reused.as_str(), id.as_str());
    });
}

#[test]
fn corrupt_known_source_metadata_is_reported() {
    let temp = tempdir().unwrap();
    with_config_home(temp.path(), || {
        let db = LibraryDatabase::open().unwrap();
        db.set_metadata(KNOWN_SOURCES_KEY, "{not valid json")
            .unwrap();

        let err = db
            .lookup_known_source_id(Path::new("some/root"))
            .unwrap_err();

        assert!(matches!(
            err,
            LibraryError::MetadataJson {
                key: KNOWN_SOURCES_KEY,
                ..
            }
        ));
        assert!(err.to_string().contains(KNOWN_SOURCES_KEY));
    });
}

#[test]
fn replace_state_rolls_back_sources_when_known_source_metadata_write_fails() {
    let temp = tempdir().unwrap();
    with_config_home(temp.path(), || {
        let original_root = normalize_path(Path::new("original/root"));
        let original_id = SourceId::new();
        save(&LibraryState {
            sources: vec![SampleSource::new_with_id(
                original_id.clone(),
                original_root.clone(),
            )],
        })
        .unwrap();

        let mut db = LibraryDatabase::open().unwrap();
        db.connection
            .execute_batch(
                "CREATE TRIGGER fail_known_sources_update
                 BEFORE UPDATE ON metadata
                 WHEN NEW.key = 'known_sources_v1'
                 BEGIN
                     SELECT RAISE(FAIL, 'known source metadata write failed');
                 END;",
            )
            .unwrap();

        let new_root = normalize_path(Path::new("new/root"));
        let err = db
            .replace_state(&LibraryState {
                sources: vec![SampleSource::new(new_root)],
            })
            .unwrap_err();

        assert!(matches!(err, LibraryError::Sql(_)));
        let sources = db.load_sources().unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].id.as_str(), original_id.as_str());
        assert_eq!(sources[0].root, original_root);
    });
}
