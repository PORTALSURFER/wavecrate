use super::*;

#[test]
fn browser_selection_is_cleared_when_focus_leaves_browser() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.focus_browser_row(0);
    assert_eq!(controller.ui.browser.selected_visible, Some(0));
    assert!(controller.ui.browser.selected.is_some());

    controller.focus_sources_list();
    controller.blur_browser_focus();

    assert!(controller.ui.browser.selected_visible.is_none());
    assert!(controller.ui.browser.selected.is_none());
    assert!(controller.ui.browser.selected_paths.is_empty());
}

#[test]
fn browser_selection_is_retained_when_waveform_focused() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source);
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.focus_browser_row(0);
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("one.wav"))
    );
    assert_eq!(controller.ui.browser.selected_visible, Some(0));

    controller.focus_waveform_context();
    controller.blur_browser_focus();

    controller.rebuild_browser_lists();
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("one.wav"))
    );
    let visible_row = controller.visible_row_for_path(Path::new("one.wav"));
    let selected_visible = controller.ui.browser.selected_visible;
    assert_eq!(selected_visible, visible_row);
}
