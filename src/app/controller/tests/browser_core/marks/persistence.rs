use super::*;

#[test]
fn marked_filter_composes_with_rating_search_and_folder_filters() -> Result<(), String> {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    std::fs::create_dir_all(source.root.join("drums")).expect("create drums folder");
    std::fs::create_dir_all(source.root.join("fx")).expect("create fx folder");
    controller.set_wav_entries_for_tests(vec![
        sample_entry("drums/kick_marked.wav", Rating::KEEP_1),
        sample_entry("drums/snare_marked.wav", Rating::NEUTRAL),
        sample_entry("fx/kick_marked.wav", Rating::KEEP_1),
        sample_entry("drums/kick_unmarked.wav", Rating::KEEP_1),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();
    controller.focus_browser_row_only(2);
    controller.toggle_browser_sample_mark();

    controller.refresh_folder_browser_for_tests();
    let drums_index = controller
        .ui
        .sources
        .folders
        .rows
        .iter()
        .position(|row| row.path == PathBuf::from("drums"))
        .expect("expected drums folder");
    controller.replace_folder_selection(drums_index);
    controller.set_browser_search("kick");
    controller.set_browser_rating_filter(1, false);
    controller.toggle_browser_marked_filter();

    assert!(controller.ui.browser.search.marked_only);
    assert_eq!(
        visible_paths(&mut controller),
        vec![PathBuf::from("drums/kick_marked.wav")]
    );
    Ok(())
}

#[test]
fn browser_sample_marks_survive_source_switches_within_session() {
    let (mut controller, source_a) = dummy_controller();
    let source_b = SampleSource::new(source_a.root.parent().unwrap().join("source_b"));
    std::fs::create_dir_all(&source_b.root).expect("create second source root");
    controller.library.sources.push(source_a.clone());
    controller.library.sources.push(source_b.clone());
    controller.selection_state.ctx.selected_source = Some(source_a.id.clone());

    controller.set_wav_entries_for_tests(vec![sample_entry("a.wav", Rating::NEUTRAL)]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();

    controller.select_source(Some(source_b.id.clone()));
    controller.set_wav_entries_for_tests(vec![sample_entry("b.wav", Rating::NEUTRAL)]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();

    assert!(controller.browser_sample_marked(&source_a.id, Path::new("a.wav")));
    assert!(controller.browser_sample_marked(&source_b.id, Path::new("b.wav")));
    assert_eq!(controller.ui.browser.marks.marked_paths.len(), 2);
}

#[test]
fn browser_sample_marks_follow_renames_and_prune_deleted_entries() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("old.wav", Rating::NEUTRAL),
        sample_entry("keep.wav", Rating::NEUTRAL),
    ]);

    controller.focus_browser_row_only(0);
    controller.toggle_browser_sample_mark();
    controller.update_selection_paths(&source, Path::new("old.wav"), Path::new("renamed.wav"));
    controller.set_wav_entries_for_tests(vec![
        sample_entry("renamed.wav", Rating::NEUTRAL),
        sample_entry("keep.wav", Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    assert!(!controller.browser_sample_marked(&source.id, Path::new("old.wav")));
    assert!(controller.browser_sample_marked(&source.id, Path::new("renamed.wav")));

    controller.set_wav_entries_for_tests(vec![sample_entry("keep.wav", Rating::NEUTRAL)]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    assert!(!controller.browser_sample_marked(&source.id, Path::new("renamed.wav")));
}
