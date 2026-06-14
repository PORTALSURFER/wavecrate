use super::support::*;

#[test]
fn folder_filter_visible_rows_match_sync_pipeline() {
    let entries = vec![
        search_entry("root.wav", Rating::NEUTRAL),
        search_entry("drums/kick.wav", Rating::NEUTRAL),
        search_entry("hits/snare.wav", Rating::NEUTRAL),
    ];
    let (mut controller, source) = prepare_with_source_and_wav_entries(entries.clone());
    controller.ui_cache.folders.models.insert(
        FolderBrowserCacheKey {
            pane: FolderPaneId::Upper,
            source_id: source.id.clone(),
        },
        FolderBrowserModel {
            selected: BTreeSet::from([PathBuf::from(""), PathBuf::from("drums")]),
            negated: BTreeSet::from([PathBuf::from("hits")]),
            file_scope_mode: crate::app::state::FolderFileScopeMode::DirectOnly,
            ..FolderBrowserModel::default()
        },
    );
    controller.rebuild_browser_lists();

    let controller_visible = visible_indices(&controller);
    assert_eq!(controller_visible, vec![0, 1]);

    let mut cache = SearchWorkerCache {
        entries: Some(compact_entries(&entries)),
        ..SearchWorkerCache::default()
    };
    let job = SearchJob {
        folder_selection: Some(BTreeSet::from([PathBuf::from(""), PathBuf::from("drums")])),
        folder_negated: Some(BTreeSet::from([PathBuf::from("hits")])),
        file_scope_mode: crate::app::state::FolderFileScopeMode::DirectOnly,
        ..make_search_job(&source, "")
    };
    let queue = SearchJobQueue::new();
    queue.send(SearchJob {
        folder_selection: Some(BTreeSet::from([PathBuf::from(""), PathBuf::from("drums")])),
        folder_negated: Some(BTreeSet::from([PathBuf::from("hits")])),
        file_scope_mode: crate::app::state::FolderFileScopeMode::DirectOnly,
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
            has_folder_filters: true,
        },
    )
    .expect("expected worker visible rows");

    assert_eq!(worker_visible, controller_visible);
}
