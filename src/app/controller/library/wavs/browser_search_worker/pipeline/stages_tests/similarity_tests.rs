use super::super::stages::{BuildVisibleRowsParams, build_visible_rows_for_job};
use super::super::*;
use super::support::make_search_job;
use crate::app::state::{SimilarQuery, empty_similarity_aspect_score_rows};
use std::sync::Arc;

#[test]
fn similarity_visible_rows_keep_sparse_lookup_compact() {
    let entries = vec![
        CompactSearchEntry {
            display_label: "anchor".into(),
            relative_path: "anchor.wav".into(),
            tag: Rating::NEUTRAL,
            locked: false,
            last_played_at: None,
            tag_named: false,
        },
        CompactSearchEntry {
            display_label: "close".into(),
            relative_path: "close.wav".into(),
            tag: Rating::NEUTRAL,
            locked: false,
            last_played_at: None,
            tag_named: false,
        },
        CompactSearchEntry {
            display_label: "missing".into(),
            relative_path: "missing.wav".into(),
            tag: Rating::NEUTRAL,
            locked: false,
            last_played_at: None,
            tag_named: false,
        },
        CompactSearchEntry {
            display_label: "far".into(),
            relative_path: "far.wav".into(),
            tag: Rating::NEUTRAL,
            locked: false,
            last_played_at: None,
            tag_named: false,
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
            aspect_scores: empty_similarity_aspect_score_rows(2),
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
