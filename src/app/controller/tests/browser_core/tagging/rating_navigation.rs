use super::*;
#[test]
fn tagging_keeps_selection_on_same_sample() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.sample_view.wav.selected_wav = Some(PathBuf::from("one.wav"));
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.tag_selected(crate::sample_sources::Rating::KEEP_1);

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("one.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(0));
    assert_eq!(
        controller.wav_entry(0).unwrap().tag,
        crate::sample_sources::Rating::KEEP_1
    );
}

#[test]
fn left_tagging_from_keep_untags_then_trashes() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::KEEP_1),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.sample_view.wav.selected_wav = Some(PathBuf::from("one.wav"));
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.tag_selected_left();
    assert_eq!(
        controller.wav_entry(0).unwrap().tag,
        crate::sample_sources::Rating::NEUTRAL
    );

    controller.tag_selected_left();
    assert_eq!(
        controller.wav_entry(0).unwrap().tag,
        crate::sample_sources::Rating::TRASH_3
    );
}

#[test]
fn tagging_under_filter_advances_focus_to_next_visible() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("four.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.set_browser_filter(TriageFlagFilter::Untagged);

    controller.focus_browser_row_only(1);
    controller.tag_selected(crate::sample_sources::Rating::KEEP_1);

    assert_eq!(visible_indices(&controller), vec![0, 2, 3]);
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("three.wav"))
    );
}

#[test]
fn tagging_under_search_filter_updates_hidden_selected_paths() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    controller.focus_browser_row_only(0);

    controller.set_browser_search(String::from("one"));
    controller.tag_selected(crate::sample_sources::Rating::KEEP_1);

    let one_index = controller.wav_index_for_path(Path::new("one.wav")).unwrap();
    let two_index = controller.wav_index_for_path(Path::new("two.wav")).unwrap();
    let three_index = controller
        .wav_index_for_path(Path::new("three.wav"))
        .unwrap();

    assert_eq!(
        controller.wav_entry(one_index).unwrap().tag,
        crate::sample_sources::Rating::KEEP_1
    );
    assert_eq!(
        controller.wav_entry(two_index).unwrap().tag,
        crate::sample_sources::Rating::KEEP_1
    );
    assert_eq!(
        controller.wav_entry(three_index).unwrap().tag,
        crate::sample_sources::Rating::NEUTRAL
    );
    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")]
    );
}
