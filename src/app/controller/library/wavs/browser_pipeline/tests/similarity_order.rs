use super::*;
#[test]
fn similarity_sort_reuses_pipeline_lookup_scratch() {
    let entries = vec![
        search_entry("anchor.wav", Rating::NEUTRAL, None),
        search_entry("close.wav", Rating::NEUTRAL, None),
        search_entry("far.wav", Rating::NEUTRAL, None),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);
    controller.ui.browser.search.sort = SampleBrowserSort::Similarity;
    controller.ui.browser.search.similar_query = Some(SimilarQuery {
        sample_id: "source::anchor.wav".to_string(),
        label: "anchor".to_string(),
        indices: vec![0, 1, 2],
        scores: vec![1.0, 0.8, 0.3],
        aspect_scores: empty_similarity_aspect_score_rows(3),
        anchor_index: Some(0),
    });

    let _ = build_visible_rows(&mut controller, Some(0), None);
    let first_capacity = controller
        .ui_cache
        .browser
        .pipeline
        .similar_lookup_scratch
        .capacity();
    assert_eq!(
        controller
            .ui_cache
            .browser
            .pipeline
            .similar_lookup_scratch
            .len(),
        3
    );

    controller.ui.browser.search.similar_query = Some(SimilarQuery {
        sample_id: "source::anchor.wav".to_string(),
        label: "anchor".to_string(),
        indices: vec![0, 2, 1],
        scores: vec![1.0, 0.9, 0.2],
        aspect_scores: empty_similarity_aspect_score_rows(3),
        anchor_index: Some(0),
    });
    let (visible, _, _) = build_visible_rows(&mut controller, Some(0), None);

    match visible {
        VisibleRows::List(rows) => assert_eq!(&*rows, &[0usize, 2usize, 1usize]),
        VisibleRows::All { total } => panic!("expected similarity-sorted rows, got all {total}"),
    }
    assert_eq!(
        controller
            .ui_cache
            .browser
            .pipeline
            .similar_lookup_scratch
            .capacity(),
        first_capacity
    );
}

#[test]
fn similarity_sort_keeps_sparse_lookup_compact() {
    let entries = vec![
        search_entry("anchor.wav", Rating::NEUTRAL, None),
        search_entry("close.wav", Rating::NEUTRAL, None),
        search_entry("missing.wav", Rating::NEUTRAL, None),
        search_entry("far.wav", Rating::NEUTRAL, None),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);
    controller.ui.browser.search.sort = SampleBrowserSort::Similarity;
    controller.ui.browser.search.similar_query = Some(SimilarQuery {
        sample_id: "source::anchor.wav".to_string(),
        label: "anchor".to_string(),
        indices: vec![3, 1],
        scores: vec![0.9, 0.4],
        aspect_scores: empty_similarity_aspect_score_rows(2),
        anchor_index: None,
    });

    let (visible, _, _) = build_visible_rows(&mut controller, None, None);

    match visible {
        VisibleRows::List(rows) => assert_eq!(&*rows, &[3usize, 1usize]),
        VisibleRows::All { total } => panic!("expected similarity-sorted rows, got all {total}"),
    }
    assert_eq!(
        controller.ui_cache.browser.pipeline.similar_lookup_scratch,
        vec![(1, 0.4), (3, 0.9)]
    );
}
