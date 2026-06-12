use super::super::stages::{
    ensure_search_cache_ready_for_job, ensure_search_entries_loaded_for_job,
};
use super::super::*;
use super::support::{
    cached_display_labels, loaded_search_cache_for_tests, make_search_job, raw_source_conn,
    set_raw_source_metadata,
};
use std::path::{Path, PathBuf};

#[test]
/// Verifies revision read failure preserves existing search cache.
fn revision_read_failure_preserves_existing_search_cache() {
    let (temp, root, _db, job, queue, generation, mut cache, source_id) =
        loaded_search_cache_for_tests("");
    let _keep_temp = temp;
    let original_revision = cache.revision;
    let original_paths_revision = cache.paths_revision;
    let original_labels = cached_display_labels(&cache);
    set_raw_source_metadata(&root, "revision", "not-a-number");

    assert!(ensure_search_cache_ready_for_job(
        &mut cache, &job, &source_id
    ));
    assert!(!ensure_search_entries_loaded_for_job(
        &mut cache, &job, &queue, generation
    ));

    assert_eq!(cache.revision, original_revision);
    assert_eq!(cache.paths_revision, original_paths_revision);
    assert_eq!(cached_display_labels(&cache), original_labels);
    assert!(cache.entries.is_some());
}

#[test]
/// Verifies metadata read failure preserves existing search cache.
fn metadata_read_failure_preserves_existing_search_cache() {
    let (temp, root, _db, job, queue, generation, mut cache, source_id) =
        loaded_search_cache_for_tests("");
    let _keep_temp = temp;
    let original_revision = cache.revision;
    let original_labels = cached_display_labels(&cache);
    set_raw_source_metadata(&root, "revision", &(original_revision + 1).to_string());
    raw_source_conn(&root)
        .execute("DROP TABLE source_tags", [])
        .unwrap();

    assert!(ensure_search_cache_ready_for_job(
        &mut cache, &job, &source_id
    ));
    assert!(!ensure_search_entries_loaded_for_job(
        &mut cache, &job, &queue, generation
    ));

    assert_eq!(cache.revision, original_revision);
    assert_eq!(cached_display_labels(&cache), original_labels);
    assert!(cache.entries.is_some());
}

#[test]
/// Handles targeted metadata delta read failure falls back to successful full reload.
fn targeted_metadata_delta_read_failure_falls_back_to_successful_full_reload() {
    let (temp, _root, db, base_job, queue, generation, mut cache, source_id) =
        loaded_search_cache_for_tests("");
    let _keep_temp = temp;
    db.set_tag(Path::new("one.wav"), Rating::KEEP_1)
        .expect("update one");
    let delta_job = SearchJob {
        metadata_delta_paths: vec![PathBuf::from("../bad-delta.wav")],
        source_id: base_job.source_id.clone(),
        source_root: base_job.source_root.clone(),
        ..make_search_job("")
    };

    assert!(ensure_search_cache_ready_for_job(
        &mut cache, &delta_job, &source_id
    ));
    assert!(ensure_search_entries_loaded_for_job(
        &mut cache, &delta_job, &queue, generation
    ));

    let refreshed = cache.entries.as_ref().expect("entries refreshed");
    assert_eq!(refreshed[0].tag, Rating::KEEP_1);
}

#[test]
/// Verifies targeted metadata delta failure preserves cache when full reload fails.
fn targeted_metadata_delta_failure_preserves_cache_when_full_reload_fails() {
    let (temp, root, _db, base_job, queue, generation, mut cache, source_id) =
        loaded_search_cache_for_tests("");
    let _keep_temp = temp;
    let original_revision = cache.revision;
    let original_labels = cached_display_labels(&cache);
    set_raw_source_metadata(&root, "revision", &(original_revision + 1).to_string());
    raw_source_conn(&root)
        .execute("DROP TABLE source_tags", [])
        .unwrap();
    let delta_job = SearchJob {
        metadata_delta_paths: vec![PathBuf::from("../bad-delta.wav")],
        source_id: base_job.source_id.clone(),
        source_root: base_job.source_root.clone(),
        ..make_search_job("")
    };

    assert!(ensure_search_cache_ready_for_job(
        &mut cache, &delta_job, &source_id
    ));
    assert!(!ensure_search_entries_loaded_for_job(
        &mut cache, &delta_job, &queue, generation
    ));

    assert_eq!(cache.revision, original_revision);
    assert_eq!(cached_display_labels(&cache), original_labels);
    assert!(cache.entries.is_some());
}
