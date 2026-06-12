use super::super::stages::{reusable_prefix_query_scores, try_reuse_cached_query_scores};
use super::super::*;

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
