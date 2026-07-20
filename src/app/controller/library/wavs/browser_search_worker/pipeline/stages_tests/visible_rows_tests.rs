use super::super::stages::{
    BuildVisibleRowsParams, build_visible_rows_for_job, ensure_search_cache_ready_for_job,
    ensure_search_entries_loaded_for_job, sort_visible_indices,
};
use super::super::*;
use super::support::make_search_job;
use crate::sample_sources::{SourceDatabase, SourceId};
use std::path::Path;
use std::sync::Arc;
use tempfile::tempdir;

#[test]
fn path_set_refresh_rebuilds_entries_and_clears_query_scores() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).expect("create source root");

    let db = SourceDatabase::open_for_test_fixture_source_write(&root).expect("open source db");
    db.upsert_file(Path::new("drums/kick.wav"), 1, 1)
        .expect("insert kick");
    db.upsert_file(Path::new("drums/snare.wav"), 1, 2)
        .expect("insert snare");

    let job = SearchJob {
        source_id: SourceId::new(),
        source_root: root.clone(),
        ..make_search_job("kick")
    };
    let source_id = job.source_id.as_str().to_string();
    let queue = SearchJobQueue::new();
    queue.send(SearchJob {
        source_id: job.source_id.clone(),
        source_root: job.source_root.clone(),
        ..make_search_job("kick")
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

    cache.query_score_cache.push(WorkerQueryScoreCacheEntry {
        scope: WorkerQueryScoreCacheScope {
            source_id: source_id.clone(),
            path_fingerprint: cache.path_fingerprint,
        },
        query: "kick".to_string(),
        scores: Arc::from([Some(10), None]),
        matched_indices: Arc::from([0]),
    });
    let initial_path_fingerprint = cache.path_fingerprint;
    let initial_paths_revision = cache.paths_revision;

    db.upsert_file(Path::new("drums/hat.wav"), 1, 3)
        .expect("insert hat");

    assert!(ensure_search_cache_ready_for_job(
        &mut cache, &job, &source_id
    ));
    assert!(ensure_search_entries_loaded_for_job(
        &mut cache, &job, &queue, generation
    ));

    let refreshed = cache.entries.as_ref().expect("entries refreshed");
    assert_eq!(refreshed.len(), 3);
    assert!(cache.path_fingerprint != initial_path_fingerprint);
    assert!(cache.paths_revision > initial_paths_revision);
    assert!(cache.query_score_cache.is_empty());
}

#[test]
fn sort_visible_indices_respects_playback_age_and_list_order() {
    let entries = vec![
        CompactSearchEntry {
            display_label: "a".into(),
            relative_path: "a.wav".into(),
            tag: Rating::NEUTRAL,
            locked: false,
            last_played_at: Some(20),
            tag_named: false,
        },
        CompactSearchEntry {
            display_label: "b".into(),
            relative_path: "b.wav".into(),
            tag: Rating::NEUTRAL,
            locked: false,
            last_played_at: Some(10),
            tag_named: false,
        },
        CompactSearchEntry {
            display_label: "c".into(),
            relative_path: "c.wav".into(),
            tag: Rating::NEUTRAL,
            locked: false,
            last_played_at: None,
            tag_named: false,
        },
    ];

    let mut asc = vec![0, 1, 2];
    sort_visible_indices(&entries, &mut asc, SampleBrowserSort::PlaybackAgeAsc);
    assert_eq!(asc, vec![2, 1, 0]);

    let mut desc = vec![0, 1, 2];
    sort_visible_indices(&entries, &mut desc, SampleBrowserSort::PlaybackAgeDesc);
    assert_eq!(desc, vec![0, 1, 2]);

    let mut list_order = vec![2, 0, 1];
    sort_visible_indices(&entries, &mut list_order, SampleBrowserSort::ListOrder);
    assert_eq!(list_order, vec![0, 1, 2]);
}

#[test]
fn list_order_query_orders_results_by_score() {
    let entries = vec![
        CompactSearchEntry {
            display_label: "kick".into(),
            relative_path: "kick.wav".into(),
            tag: Rating::NEUTRAL,
            locked: false,
            last_played_at: None,
            tag_named: false,
        },
        CompactSearchEntry {
            display_label: "snare".into(),
            relative_path: "snare.wav".into(),
            tag: Rating::NEUTRAL,
            locked: false,
            last_played_at: None,
            tag_named: false,
        },
        CompactSearchEntry {
            display_label: "hat".into(),
            relative_path: "hat.wav".into(),
            tag: Rating::NEUTRAL,
            locked: false,
            last_played_at: None,
            tag_named: false,
        },
    ];
    let mut cache = SearchWorkerCache {
        entries: Some(entries),
        scored_index_scratch: vec![(99, 99)],
        ..SearchWorkerCache::default()
    };
    let scores: Arc<[Option<i64>]> = Arc::from([Some(1), Some(100), Some(2)]);
    let queue = SearchJobQueue::new();
    queue.send(make_search_job("q"));
    let generation = queue
        .take_blocking()
        .expect("expected queued search job generation")
        .generation;

    let visible = build_visible_rows_for_job(
        &mut cache,
        BuildVisibleRowsParams {
            job: &make_search_job("q"),
            has_query: true,
            scores: &scores,
            entries_len: 3,
            queue: &queue,
            generation,
            source_id: "source-a",
            has_folder_filters: false,
        },
    )
    .expect("expected visible rows");

    assert_eq!(visible, vec![1, 2, 0]);
    assert_eq!(cache.scored_index_scratch, vec![(1, 100), (2, 2), (0, 1)]);
}
