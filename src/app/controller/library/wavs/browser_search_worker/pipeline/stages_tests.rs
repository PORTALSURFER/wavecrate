use super::stages::{
    BuildVisibleRowsParams, build_visible_rows_for_job, ensure_search_cache_ready_for_job,
    ensure_search_entries_loaded_for_job, reusable_prefix_query_scores, sort_visible_indices,
    try_reuse_cached_query_scores,
};
use super::*;
use crate::app::state::SimilarQuery;
use crate::sample_sources::{SourceDatabase, SourceId};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[test]
fn query_score_cache_reuse_moves_hit_to_front() {
    let mut cache = SearchWorkerCache {
        path_fingerprint: 12,
        query_score_cache: vec![
            WorkerQueryScoreCacheEntry {
                scope: WorkerQueryScoreCacheScope {
                    source_id: "source-a".to_string(),
                    path_fingerprint: 12,
                },
                query: "snare".to_string(),
                scores: Arc::from([Some(10), Some(8)]),
                matched_indices: Arc::from([0, 1]),
            },
            WorkerQueryScoreCacheEntry {
                scope: WorkerQueryScoreCacheScope {
                    source_id: "source-a".to_string(),
                    path_fingerprint: 12,
                },
                query: "kick".to_string(),
                scores: Arc::from([Some(4), Some(3)]),
                matched_indices: Arc::from([0, 1]),
            },
        ],
        ..SearchWorkerCache::default()
    };

    let reused = try_reuse_cached_query_scores(&mut cache, "source-a", 12, "kick", 2);
    assert!(reused.is_some());
    let reused_scores = reused.unwrap_or_else(|| Arc::from([]));
    assert_eq!(reused_scores.as_ref(), [Some(4), Some(3)]);
    assert_eq!(cache.query_score_cache[0].query, "kick");
}

#[test]
fn prefix_query_score_cache_prefers_longest_matching_prefix() {
    let cache = SearchWorkerCache {
        path_fingerprint: 12,
        query_score_cache: vec![
            WorkerQueryScoreCacheEntry {
                scope: WorkerQueryScoreCacheScope {
                    source_id: "source-a".to_string(),
                    path_fingerprint: 12,
                },
                query: "k".to_string(),
                scores: Arc::from([Some(4), None]),
                matched_indices: Arc::from([0]),
            },
            WorkerQueryScoreCacheEntry {
                scope: WorkerQueryScoreCacheScope {
                    source_id: "source-a".to_string(),
                    path_fingerprint: 12,
                },
                query: "ki".to_string(),
                scores: Arc::from([Some(3), None]),
                matched_indices: Arc::from([0]),
            },
        ],
        ..SearchWorkerCache::default()
    };

    let reused = reusable_prefix_query_scores(&cache, "source-a", 12, "kick", 2)
        .expect("expected reusable prefix cache");

    assert_eq!(reused.query, "ki");
    assert_eq!(reused.matched_indices.as_ref(), &[0]);
}

#[test]
fn metadata_only_refresh_reuses_cached_paths_and_scores() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).expect("create source root");

    let db = SourceDatabase::open(&root).expect("open source db");
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

    let entries = cache.entries.as_ref().expect("entries loaded");
    let initial_path_ptrs: Vec<*const u8> = entries
        .iter()
        .map(|entry| entry.relative_path.as_ptr())
        .collect();
    let initial_label_ptrs: Vec<*const u8> = entries
        .iter()
        .map(|entry| entry.display_label.as_ptr())
        .collect();
    let path_fingerprint = cache.path_fingerprint;
    cache.query_score_cache.push(WorkerQueryScoreCacheEntry {
        scope: WorkerQueryScoreCacheScope {
            source_id: source_id.clone(),
            path_fingerprint,
        },
        query: "kick".to_string(),
        scores: Arc::from([Some(10), None]),
        matched_indices: Arc::from([0]),
    });

    db.set_tag(Path::new("drums/kick.wav"), Rating::KEEP_1)
        .expect("update tag");
    db.set_last_played_at(Path::new("drums/snare.wav"), 42)
        .expect("update last played");

    assert!(ensure_search_cache_ready_for_job(
        &mut cache, &job, &source_id
    ));
    assert!(ensure_search_entries_loaded_for_job(
        &mut cache, &job, &queue, generation
    ));

    let refreshed = cache.entries.as_ref().expect("entries refreshed");
    let refreshed_path_ptrs: Vec<*const u8> = refreshed
        .iter()
        .map(|entry| entry.relative_path.as_ptr())
        .collect();
    let refreshed_label_ptrs: Vec<*const u8> = refreshed
        .iter()
        .map(|entry| entry.display_label.as_ptr())
        .collect();

    assert_eq!(refreshed_path_ptrs, initial_path_ptrs);
    assert_eq!(refreshed_label_ptrs, initial_label_ptrs);
    assert_eq!(cache.path_fingerprint, path_fingerprint);
    assert_eq!(cache.query_score_cache.len(), 1);
    assert_eq!(refreshed[0].tag, Rating::KEEP_1);
    assert_eq!(refreshed[1].last_played_at, Some(42));
}

#[test]
fn metadata_delta_refresh_updates_only_targeted_rows() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).expect("create source root");

    let db = SourceDatabase::open(&root).expect("open source db");
    db.upsert_file(Path::new("drums/kick.wav"), 1, 1)
        .expect("insert kick");
    db.upsert_file(Path::new("drums/snare.wav"), 1, 2)
        .expect("insert snare");

    let base_job = SearchJob {
        source_id: SourceId::new(),
        source_root: root.clone(),
        ..make_search_job("kick")
    };
    let source_id = base_job.source_id.as_str().to_string();
    let queue = SearchJobQueue::new();
    queue.send(SearchJob {
        source_id: base_job.source_id.clone(),
        source_root: base_job.source_root.clone(),
        ..make_search_job("kick")
    });
    let generation = queue
        .take_blocking()
        .expect("expected queued search job generation")
        .generation;
    let mut cache = SearchWorkerCache::default();

    assert!(ensure_search_cache_ready_for_job(
        &mut cache, &base_job, &source_id
    ));
    assert!(ensure_search_entries_loaded_for_job(
        &mut cache, &base_job, &queue, generation
    ));

    db.set_last_played_at(Path::new("drums/snare.wav"), 77)
        .expect("update snare playback age");
    let delta_job = SearchJob {
        metadata_delta_paths: vec![PathBuf::from("drums/snare.wav")],
        source_id: base_job.source_id.clone(),
        source_root: base_job.source_root.clone(),
        ..make_search_job("kick")
    };

    assert!(ensure_search_cache_ready_for_job(
        &mut cache, &delta_job, &source_id
    ));
    assert!(ensure_search_entries_loaded_for_job(
        &mut cache, &delta_job, &queue, generation
    ));

    let refreshed = cache.entries.as_ref().expect("entries refreshed");
    assert_eq!(refreshed[0].last_played_at, None);
    assert_eq!(refreshed[1].last_played_at, Some(77));
}

#[test]
fn path_set_refresh_rebuilds_entries_and_clears_query_scores() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).expect("create source root");

    let db = SourceDatabase::open(&root).expect("open source db");
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
        },
        CompactSearchEntry {
            display_label: "b".into(),
            relative_path: "b.wav".into(),
            tag: Rating::NEUTRAL,
            locked: false,
            last_played_at: Some(10),
        },
        CompactSearchEntry {
            display_label: "c".into(),
            relative_path: "c.wav".into(),
            tag: Rating::NEUTRAL,
            locked: false,
            last_played_at: None,
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
        },
        CompactSearchEntry {
            display_label: "snare".into(),
            relative_path: "snare.wav".into(),
            tag: Rating::NEUTRAL,
            locked: false,
            last_played_at: None,
        },
        CompactSearchEntry {
            display_label: "hat".into(),
            relative_path: "hat.wav".into(),
            tag: Rating::NEUTRAL,
            locked: false,
            last_played_at: None,
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

#[test]
fn similarity_visible_rows_keep_sparse_lookup_compact() {
    let entries = vec![
        CompactSearchEntry {
            display_label: "anchor".into(),
            relative_path: "anchor.wav".into(),
            tag: Rating::NEUTRAL,
            locked: false,
            last_played_at: None,
        },
        CompactSearchEntry {
            display_label: "close".into(),
            relative_path: "close.wav".into(),
            tag: Rating::NEUTRAL,
            locked: false,
            last_played_at: None,
        },
        CompactSearchEntry {
            display_label: "missing".into(),
            relative_path: "missing.wav".into(),
            tag: Rating::NEUTRAL,
            locked: false,
            last_played_at: None,
        },
        CompactSearchEntry {
            display_label: "far".into(),
            relative_path: "far.wav".into(),
            tag: Rating::NEUTRAL,
            locked: false,
            last_played_at: None,
        },
    ];
    let mut cache = SearchWorkerCache {
        entries: Some(entries),
        ..SearchWorkerCache::default()
    };
    let job = SearchJob {
        sort: SampleBrowserSort::Similarity,
        similar_query: Some(SimilarQuery {
            sample_id: "source::anchor.wav".to_string(),
            label: "anchor".to_string(),
            indices: vec![3, 1],
            scores: vec![0.9, 0.4],
            anchor_index: None,
        }),
        ..make_search_job("")
    };
    let queue = SearchJobQueue::new();
    queue.send(make_search_job(""));
    let generation = queue
        .take_blocking()
        .expect("expected queued search job generation")
        .generation;

    let visible = build_visible_rows_for_job(
        &mut cache,
        BuildVisibleRowsParams {
            job: &job,
            has_query: false,
            scores: &Arc::from([]),
            entries_len: 4,
            queue: &queue,
            generation,
            source_id: "source-a",
            has_folder_filters: false,
        },
    )
    .expect("expected visible rows");

    assert_eq!(visible, vec![3, 1]);
    assert_eq!(cache.similar_lookup_scratch, vec![(1, 0.4), (3, 0.9)]);
}

fn make_search_job(query: &str) -> SearchJob {
    SearchJob {
        request_id: 1,
        source_id: SourceId::new(),
        source_root: PathBuf::from("root"),
        query: query.to_string(),
        filter: TriageFlagFilter::All,
        rating_filter: BTreeSet::new(),
        playback_age_filter: BTreeSet::new(),
        marked_only: false,
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
