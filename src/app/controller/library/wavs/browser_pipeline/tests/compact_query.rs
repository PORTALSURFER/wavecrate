use super::*;
#[test]
fn folder_filter_build_does_not_fault_wav_pages_when_compact_cache_is_available() {
    let entries = vec![
        search_entry("root.wav", Rating::NEUTRAL, None),
        search_entry("drums/kick.wav", Rating::NEUTRAL, None),
        search_entry("hits/snare.wav", Rating::NEUTRAL, None),
    ];
    let (mut controller, source) = prepare_with_source_and_wav_entries(entries);
    controller.ui_cache.folders.models.insert(
        crate::app::controller::state::cache::FolderBrowserCacheKey {
            pane: crate::app::state::FolderPaneId::Upper,
            source_id: source.id,
        },
        crate::app::controller::library::source_folders::FolderBrowserModel {
            selected: BTreeSet::from([PathBuf::from(""), PathBuf::from("drums")]),
            negated: BTreeSet::from([PathBuf::from("hits")]),
            file_scope_mode: FolderFileScopeMode::DirectOnly,
            ..Default::default()
        },
    );
    clear_loaded_wav_pages(&mut controller);

    let (visible, _, _) = build_visible_rows(&mut controller, None, None);

    match visible {
        VisibleRows::List(rows) => {
            let mut visible_paths = rows
                .iter()
                .map(|index| {
                    controller.ui_cache.browser.pipeline.compact_entries[*index]
                        .relative_path
                        .to_string_lossy()
                        .replace('\\', "/")
                })
                .collect::<Vec<_>>();
            visible_paths.sort();
            assert_eq!(
                visible_paths,
                vec![String::from("drums/kick.wav"), String::from("root.wav")]
            );
        }
        VisibleRows::All { total } => panic!("expected folder-filtered rows, got all {total}"),
    }
    assert!(controller.wav_entries.pages.is_empty());
    assert_eq!(
        controller.ui_cache.browser.pipeline.folder_filtered_rows,
        vec![0, 2]
    );
    assert_eq!(
        controller.ui_cache.browser.pipeline.filtered_rows,
        vec![0, 2]
    );
}

#[test]
fn playback_age_filter_build_does_not_fault_wav_pages_when_compact_cache_is_available() {
    const WEEK_SECS: i64 = 7 * 24 * 60 * 60;

    let entries = vec![
        search_entry("aging.wav", Rating::NEUTRAL, Some(100)),
        search_entry("fresh.wav", Rating::NEUTRAL, Some(100 + WEEK_SECS)),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);
    controller
        .ui
        .browser
        .search
        .playback_age_filter
        .insert(PlaybackAgeFilterChip::OlderThanWeek);
    clear_loaded_wav_pages(&mut controller);

    let (visible, _, _) = super::visible_rows::build_visible_rows_with_now(
        &mut controller,
        None,
        None,
        100 + WEEK_SECS + 1,
    );

    match visible {
        VisibleRows::List(rows) => assert_eq!(&*rows, &[0usize]),
        VisibleRows::All { total } => {
            panic!("expected playback-age-filtered rows, got all {total}")
        }
    }
    assert!(controller.wav_entries.pages.is_empty());
}

#[test]
fn text_query_branch_uses_search_scores_to_filter_visible_rows() {
    let entries = vec![
        search_entry("kick.wav", Rating::NEUTRAL, None),
        search_entry("snare.wav", Rating::NEUTRAL, None),
        search_entry("hat.wav", Rating::NEUTRAL, None),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);
    controller.ui.browser.search.search_query = String::from("snare");

    let (visible, focused, loaded) = build_visible_rows(&mut controller, Some(1), Some(0));

    match visible {
        VisibleRows::List(rows) => assert_eq!(&*rows, &[1usize]),
        VisibleRows::All { total } => panic!("expected query-filtered rows, got all {total}"),
    }
    assert_eq!(focused, Some(0));
    assert_eq!(loaded, None);
}

#[test]
fn similar_query_branch_applies_similarity_order_after_filters() {
    let entries = vec![
        search_entry("keep_a.wav", Rating::KEEP_1, None),
        search_entry("neutral.wav", Rating::NEUTRAL, None),
        search_entry("keep_b.wav", Rating::KEEP_3, None),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);
    controller.ui.browser.search.filter = TriageFlagFilter::Keep;
    controller.ui.browser.search.sort = SampleBrowserSort::Similarity;
    controller.ui.browser.search.similar_query = Some(SimilarQuery {
        sample_id: String::from("sample-keep"),
        label: String::from("keep"),
        indices: vec![0, 1, 2],
        scores: vec![0.4, 1.0, 0.9],
        aspect_scores: empty_similarity_aspect_score_rows(3),
        anchor_index: None,
    });

    let (visible, focused, loaded) = build_visible_rows(&mut controller, Some(2), Some(0));

    match visible {
        VisibleRows::List(rows) => assert_eq!(&*rows, &[2usize, 0usize]),
        VisibleRows::All { total } => panic!("expected similarity-sorted rows, got all {total}"),
    }
    assert_eq!(focused, Some(0));
    assert_eq!(loaded, Some(1));
}
