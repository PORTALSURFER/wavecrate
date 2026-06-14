use super::support::*;

#[test]
/// Sidebar format facets should filter the same rows in sync and worker paths.
fn sidebar_format_filter_visible_rows_match_sync_pipeline() {
    let entries = vec![
        search_entry("kick.wav", Rating::NEUTRAL),
        search_entry("render.aiff", Rating::NEUTRAL),
        search_entry("loops/snare.WAV", Rating::NEUTRAL),
    ];
    let (mut controller, source) = prepare_with_source_and_wav_entries(entries.clone());

    controller.toggle_browser_sidebar_filter(
        crate::app::state::BrowserSidebarFilterOption::Format(
            crate::app::state::BrowserFormatFacet::Wav,
        ),
        true,
    );

    let controller_visible = visible_indices(&controller);
    assert_eq!(controller_visible, vec![0, 2]);

    let mut cache = SearchWorkerCache {
        entries: Some(compact_entries(&entries)),
        ..SearchWorkerCache::default()
    };
    let sidebar_filters = crate::app::state::BrowserSidebarFilterState {
        formats: BTreeSet::from([crate::app::state::BrowserFormatFacet::Wav]),
        ..Default::default()
    };
    let job = SearchJob {
        sidebar_filters: sidebar_filters.clone(),
        ..make_search_job(&source, "")
    };
    let queue = SearchJobQueue::new();
    queue.send(SearchJob {
        sidebar_filters,
        ..make_search_job(&source, "")
    });
    let generation = queue
        .take_blocking()
        .expect("expected queued search job generation")
        .generation;
    let empty_scores: Arc<[Option<i64>]> = Arc::from([]);

    let worker_visible = build_visible_rows_for_job(
        &mut cache,
        BuildVisibleRowsParams {
            job: &job,
            has_query: false,
            scores: &empty_scores,
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
