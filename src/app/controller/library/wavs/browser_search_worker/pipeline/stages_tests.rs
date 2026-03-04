use super::stages::{sort_visible_indices, try_reuse_cached_query_scores};
use super::*;

#[test]
fn query_score_cache_reuse_moves_hit_to_front() {
    let mut cache = SearchWorkerCache {
        revision: 12,
        query_score_cache: vec![
            WorkerQueryScoreCacheEntry {
                source_id: "source-a".to_string(),
                revision: 12,
                query: "snare".to_string(),
                scores: Arc::from([Some(10), Some(8)]),
            },
            WorkerQueryScoreCacheEntry {
                source_id: "source-a".to_string(),
                revision: 12,
                query: "kick".to_string(),
                scores: Arc::from([Some(4), Some(3)]),
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
fn sort_visible_indices_respects_playback_age_and_list_order() {
    let entries = vec![
        CompactSearchEntry {
            display_label: "a".into(),
            relative_path: "a.wav".into(),
            tag: Rating::NEUTRAL,
            last_played_at: Some(20),
        },
        CompactSearchEntry {
            display_label: "b".into(),
            relative_path: "b.wav".into(),
            tag: Rating::NEUTRAL,
            last_played_at: Some(10),
        },
        CompactSearchEntry {
            display_label: "c".into(),
            relative_path: "c.wav".into(),
            tag: Rating::NEUTRAL,
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
