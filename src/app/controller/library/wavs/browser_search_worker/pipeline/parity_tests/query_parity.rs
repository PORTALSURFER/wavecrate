use super::support::*;

#[test]
fn list_order_query_matches_sync_score_ranked_visible_rows() {
    let entries = vec![
        search_entry("zzabc.wav", Rating::NEUTRAL),
        search_entry("abc_extra.wav", Rating::NEUTRAL),
        search_entry("abc.wav", Rating::NEUTRAL),
    ];
    let (mut controller, source) = prepare_with_source_and_wav_entries(entries.clone());

    controller.set_browser_search("abc");

    let controller_visible = visible_indices(&controller);
    assert_ne!(controller_visible, vec![0, 1, 2]);

    let worker_scores = controller.ui_cache.browser.search.scores.clone();
    let mut cache = SearchWorkerCache {
        entries: Some(compact_entries(&entries)),
        ..SearchWorkerCache::default()
    };
    let job = make_search_job(&source, "abc");
    let queue = SearchJobQueue::new();
    queue.send(make_search_job(&source, "abc"));
    let generation = queue
        .take_blocking()
        .expect("expected queued search job generation")
        .generation;

    let worker_visible = build_visible_rows_for_job(
        &mut cache,
        BuildVisibleRowsParams {
            job: &job,
            has_query: true,
            scores: &worker_scores,
            entries_len: entries.len(),
            queue: &queue,
            generation,
            source_id: source.id.as_str(),
            has_folder_filters: false,
        },
    )
    .expect("expected worker visible rows");

    assert_eq!(worker_visible, controller_visible);
}
