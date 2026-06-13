use super::*;
#[test]
fn duplicate_cleanup_projection_filters_missing_indices_and_remaps_focus() {
    let entries = vec![
        search_entry("keep.wav", Rating::KEEP_1, None),
        search_entry("neutral.wav", Rating::NEUTRAL, None),
        search_entry("trash.wav", Rating::TRASH_1, None),
    ];
    let (mut controller, source) = prepare_with_source_and_wav_entries(entries);
    controller.ui.browser.duplicate_cleanup = Some(BrowserDuplicateCleanupState::new(
        source.id,
        String::from("sample-1"),
        PathBuf::from("trash.wav"),
        String::from("trash.wav"),
        vec![2, 99, 0],
        vec![1.0, 0.8, 0.6],
        2,
    ));

    let (visible, focused, loaded) = build_visible_rows(&mut controller, Some(0), Some(2));

    match visible {
        VisibleRows::List(rows) => assert_eq!(&*rows, &[2usize, 0usize]),
        VisibleRows::All { total } => panic!("expected duplicate-cleanup rows, got all {total}"),
    }
    assert_eq!(focused, Some(1));
    assert_eq!(loaded, Some(0));
}

#[test]
fn marked_only_filter_keeps_only_session_marked_rows() {
    let entries = vec![
        search_entry("one.wav", Rating::NEUTRAL, None),
        search_entry("two.wav", Rating::NEUTRAL, None),
        search_entry("three.wav", Rating::NEUTRAL, None),
    ];
    let (mut controller, source) = prepare_with_source_and_wav_entries(entries);
    controller.focus_browser_row_only(1);
    controller.toggle_browser_sample_mark();
    assert!(controller.browser_sample_marked(&source.id, Path::new("two.wav")));
    controller.ui.browser.search.marked_only = true;

    let (visible, focused, loaded) = build_visible_rows(&mut controller, Some(1), None);

    match visible {
        VisibleRows::List(rows) => assert_eq!(&*rows, &[1usize]),
        VisibleRows::All { total } => panic!("expected marked-only rows, got all {total}"),
    }
    assert_eq!(focused, Some(0));
    assert_eq!(loaded, None);
}

#[test]
fn mark_toggle_keeps_filtered_fingerprint_stable_when_marked_filter_is_off() {
    let entries = vec![
        search_entry("one.wav", Rating::NEUTRAL, None),
        search_entry("two.wav", Rating::NEUTRAL, None),
    ];
    let (mut controller, _) = prepare_with_source_and_wav_entries(entries);

    let _ = build_visible_rows(&mut controller, Some(0), None);
    let before = controller.ui_cache.browser.pipeline.filtered_fingerprint;

    controller.focus_browser_row_only(1);
    controller.toggle_browser_sample_mark();

    let _ = build_visible_rows(&mut controller, Some(0), None);
    let after = controller.ui_cache.browser.pipeline.filtered_fingerprint;

    assert_eq!(before, after);
}
