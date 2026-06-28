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
