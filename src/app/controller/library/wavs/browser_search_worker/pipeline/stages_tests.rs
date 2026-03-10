use super::stages::{
    BuildVisibleRowsParams, build_visible_rows_for_job, reusable_prefix_query_scores,
    sort_visible_indices, try_reuse_cached_query_scores,
};
use super::*;
use crate::sample_sources::SourceId;
use std::collections::BTreeSet;
use std::path::PathBuf;

#[test]
fn query_score_cache_reuse_moves_hit_to_front() {
    let mut cache = SearchWorkerCache {
        revision: 12,
        query_score_cache: vec![
            WorkerQueryScoreCacheEntry {
                scope: WorkerQueryScoreCacheScope {
                    source_id: "source-a".to_string(),
                    revision: 12,
                },
                query: "snare".to_string(),
                scores: Arc::from([Some(10), Some(8)]),
                matched_indices: Arc::from([0, 1]),
            },
            WorkerQueryScoreCacheEntry {
                scope: WorkerQueryScoreCacheScope {
                    source_id: "source-a".to_string(),
                    revision: 12,
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
        revision: 12,
        query_score_cache: vec![
            WorkerQueryScoreCacheEntry {
                scope: WorkerQueryScoreCacheScope {
                    source_id: "source-a".to_string(),
                    revision: 12,
                },
                query: "k".to_string(),
                scores: Arc::from([Some(4), None]),
                matched_indices: Arc::from([0]),
            },
            WorkerQueryScoreCacheEntry {
                scope: WorkerQueryScoreCacheScope {
                    source_id: "source-a".to_string(),
                    revision: 12,
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
fn list_order_query_keeps_source_order_without_score_sort_scratch() {
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

    assert_eq!(visible, vec![0, 1, 2]);
    assert!(cache.scored_index_scratch.is_empty());
}

fn make_search_job(query: &str) -> SearchJob {
    SearchJob {
        request_id: 1,
        source_id: SourceId::new(),
        source_root: PathBuf::from("root"),
        query: query.to_string(),
        filter: TriageFlagFilter::All,
        rating_filter: BTreeSet::new(),
        sort: SampleBrowserSort::ListOrder,
        similar_query: None,
        folder_selection: None,
        folder_negated: None,
        root_mode: crate::app::state::RootFolderFilterMode::AllDescendants,
    }
}
