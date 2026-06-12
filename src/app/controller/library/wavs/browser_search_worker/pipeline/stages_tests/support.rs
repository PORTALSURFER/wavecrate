use super::super::stages::{
    ensure_search_cache_ready_for_job, ensure_search_entries_loaded_for_job,
};
use super::super::*;
use crate::sample_sources::{SourceDatabase, SourceId};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

/// Build a loaded search-cache fixture with a temp source database.
pub(super) fn loaded_search_cache_for_tests(
    query: &str,
) -> (
    tempfile::TempDir,
    PathBuf,
    SourceDatabase,
    SearchJob,
    SearchJobQueue,
    u64,
    SearchWorkerCache,
    String,
) {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).expect("create source root");
    let db = SourceDatabase::open(&root).expect("open source db");
    db.upsert_file(Path::new("one.wav"), 1, 1)
        .expect("insert one");
    db.upsert_file(Path::new("two.wav"), 1, 2)
        .expect("insert two");
    let job = SearchJob {
        source_id: SourceId::new(),
        source_root: root.clone(),
        ..make_search_job(query)
    };
    let source_id = job.source_id.as_str().to_string();
    let queue = SearchJobQueue::new();
    queue.send(SearchJob {
        source_id: job.source_id.clone(),
        source_root: job.source_root.clone(),
        ..make_search_job(query)
    });
    let generation = queue
        .take_blocking()
        .expect("expected queued search job generation")
        .generation;
    let mut cache = SearchWorkerCache::default();
    assert!(ensure_search_cache_ready_for_job(
        &mut cache, &job, &source_id
    ));
    assert!(ensure_search_entries_loaded_for_job(
        &mut cache, &job, &queue, generation
    ));
    (temp, root, db, job, queue, generation, cache, source_id)
}

/// Return cached display labels from a loaded worker cache.
pub(super) fn cached_display_labels(cache: &SearchWorkerCache) -> Vec<String> {
    cache
        .entries
        .as_ref()
        .expect("entries loaded")
        .iter()
        .map(|entry| entry.display_label.to_string())
        .collect()
}

/// Open a raw source database connection for metadata corruption tests.
pub(super) fn raw_source_conn(root: &Path) -> rusqlite::Connection {
    rusqlite::Connection::open(crate::sample_sources::database_path_for(root)).unwrap()
}

/// Write raw metadata directly for revision fallback tests.
pub(super) fn set_raw_source_metadata(root: &Path, key: &str, value: &str) {
    raw_source_conn(root)
        .execute(
            "INSERT INTO metadata (key, value)
             VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        )
        .unwrap();
}

pub(super) fn make_search_job(query: &str) -> SearchJob {
    SearchJob {
        request_id: 1,
        source_id: SourceId::new(),
        source_root: PathBuf::from("root"),
        query: query.to_string(),
        filter: TriageFlagFilter::All,
        rating_filter: BTreeSet::new(),
        playback_age_filter: BTreeSet::new(),
        marked_only: false,
        tag_named_filter: crate::app::state::TagNamedFilter::All,
        sidebar_filters: Default::default(),
        sidebar_bpm_values: Default::default(),
        marked_paths: BTreeSet::new(),
        sort: SampleBrowserSort::ListOrder,
        similar_query: None,
        duplicate_cleanup: None,
        folder_selection: None,
        folder_negated: None,
        file_scope_mode: crate::app::state::FolderFileScopeMode::AllDescendants,
        metadata_delta_paths: Vec::new(),
        playback_age_now_unix_secs: 0,
    }
}
