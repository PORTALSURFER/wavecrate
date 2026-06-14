use super::*;

#[test]
fn sample_browser_indices_track_tags() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("trash.wav", crate::sample_sources::Rating::TRASH_3),
        sample_entry("neutral.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("keep.wav", crate::sample_sources::Rating::KEEP_1),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.sample_view.wav.selected_wav = Some(PathBuf::from("neutral.wav"));
    controller.sample_view.wav.loaded_wav = Some(PathBuf::from("keep.wav"));
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    assert_eq!(controller.browser_indices(TriageFlagColumn::Trash).len(), 1);
    assert_eq!(
        controller.browser_indices(TriageFlagColumn::Neutral).len(),
        1
    );
    assert_eq!(controller.browser_indices(TriageFlagColumn::Keep).len(), 1);
    assert_eq!(visible_indices(&controller), vec![0, 1, 2]);

    let selected = controller.ui.browser.selection.selected.unwrap();
    assert_eq!(selected.column, TriageFlagColumn::Neutral);
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    let loaded = controller.ui.browser.selection.loaded.unwrap();
    assert_eq!(loaded.column, TriageFlagColumn::Keep);
    assert_eq!(controller.ui.browser.selection.loaded_visible, Some(2));
}

#[test]
fn browser_filter_limits_visible_rows() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("trash.wav", crate::sample_sources::Rating::TRASH_3),
        sample_entry("neutral.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("keep.wav", crate::sample_sources::Rating::KEEP_1),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.set_browser_filter(TriageFlagFilter::Keep);
    assert_eq!(visible_indices(&controller), vec![2]);
    controller.set_browser_filter(TriageFlagFilter::Trash);
    assert_eq!(visible_indices(&controller), vec![0]);
    controller.set_browser_filter(TriageFlagFilter::Untagged);
    assert_eq!(visible_indices(&controller), vec![1]);
    controller.set_browser_filter(TriageFlagFilter::All);
    assert_eq!(visible_indices(&controller), vec![0, 1, 2]);
}
