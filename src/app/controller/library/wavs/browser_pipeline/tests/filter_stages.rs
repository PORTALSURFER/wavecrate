use super::*;
#[test]
fn folder_stage_acceptance_matches_root_and_negated_filters() {
    let entries = vec![
        search_entry("root.wav", Rating::NEUTRAL, None),
        search_entry("drums/kick.wav", Rating::NEUTRAL, None),
        search_entry("hits/snare.wav", Rating::NEUTRAL, None),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);
    let selection = BTreeSet::from([PathBuf::from(""), PathBuf::from("drums")]);
    let negated = BTreeSet::from([PathBuf::from("hits")]);
    let file_scope_mode = FolderFileScopeMode::DirectOnly;
    let folder_hash = crate::app::controller::library::source_folders::folder_filter_fingerprint(
        Some(&selection),
        Some(&negated),
        file_scope_mode,
    );

    ensure_base_stage(&mut controller);
    ensure_folder_acceptance_stage(
        &mut controller,
        Some(&selection),
        Some(&negated),
        file_scope_mode,
        folder_hash,
        true,
    );

    assert_eq!(
        controller.ui_cache.browser.pipeline.folder_accepts_by_index,
        vec![true, true, false]
    );
}

#[test]
fn build_visible_rows_uses_all_fast_path_when_filters_are_idle() {
    let entries = vec![
        search_entry("a.wav", Rating::NEUTRAL, None),
        search_entry("b.wav", Rating::NEUTRAL, None),
        search_entry("c.wav", Rating::NEUTRAL, None),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);

    let (visible, focused, loaded) = build_visible_rows(&mut controller, None, None);

    match visible {
        VisibleRows::All { total } => assert_eq!(total, 3),
        VisibleRows::List(rows) => panic!("expected fast-path all rows, got {:?}", rows),
    }
    assert_eq!(focused, None);
    assert_eq!(loaded, None);
    assert!(
        controller
            .ui_cache
            .browser
            .pipeline
            .folder_accepts_by_index
            .is_empty()
    );
    assert!(
        controller
            .ui_cache
            .browser
            .pipeline
            .folder_filtered_rows
            .is_empty()
    );
}

#[test]
fn keep_filter_reuses_retained_triage_rows() {
    let entries = vec![
        search_entry("neutral.wav", Rating::NEUTRAL, None),
        search_entry("keep.wav", Rating::KEEP_1, None),
        search_entry("trash.wav", Rating::TRASH_1, None),
        search_entry("keep-two.wav", Rating::KEEP_3, None),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);
    controller.ui.browser.search.filter = TriageFlagFilter::Keep;

    let (visible, _, _) = build_visible_rows(&mut controller, None, None);

    match visible {
        VisibleRows::List(rows) => assert_eq!(&*rows, &[1usize, 3usize]),
        VisibleRows::All { total } => panic!("expected keep-filtered rows, got all {total}"),
    }
    assert_eq!(
        controller.ui_cache.browser.pipeline.filtered_rows,
        controller.ui_cache.browser.pipeline.keep_rows
    );
}

#[test]
fn build_visible_rows_sort_stage_maps_focus_and_loaded_positions() {
    let entries = vec![
        search_entry("older.wav", Rating::NEUTRAL, Some(20)),
        search_entry("newer.wav", Rating::NEUTRAL, Some(10)),
        search_entry("never.wav", Rating::NEUTRAL, None),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);
    controller.ui.browser.search.sort = SampleBrowserSort::PlaybackAgeAsc;

    let (visible, focused, loaded) = build_visible_rows(&mut controller, Some(1), Some(0));

    match visible {
        VisibleRows::All { total } => panic!("expected sorted list rows, got all {total}"),
        VisibleRows::List(rows) => assert_eq!(&*rows, &[2usize, 1usize, 0usize]),
    }
    assert_eq!(focused, Some(1));
    assert_eq!(loaded, Some(2));
    assert_eq!(
        controller
            .ui_cache
            .browser
            .pipeline
            .sorted_visible_position(1),
        Some(1)
    );
    assert_eq!(
        controller
            .ui_cache
            .browser
            .pipeline
            .sorted_visible_position(0),
        Some(2)
    );
}
