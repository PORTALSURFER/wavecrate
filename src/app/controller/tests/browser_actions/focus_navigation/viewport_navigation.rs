use super::*;

#[test]
fn native_set_browser_view_start_scrolls_without_changing_selection() {
    let mut entries = Vec::new();
    for index in 0..(MAX_RENDERED_BROWSER_ROWS + 8) {
        entries.push(sample_entry(
            &format!("row_{index:03}.wav"),
            crate::sample_sources::Rating::NEUTRAL,
        ));
    }
    let (mut controller, source) = prepare_with_source_and_wav_entries(entries);
    write_test_wav(&source.root.join("row_000.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("row_001.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(1);
    controller.runtime.jobs.pending_audio = None;
    controller.runtime.jobs.pending_playback = None;

    controller.set_browser_view_start_action(2);

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("row_001.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(controller.ui.browser.viewport.view_window_start, 2);
    assert_eq!(controller.ui.browser.viewport.render_window_start, 2);
    assert!(!controller.ui.browser.selection.autoscroll);
    assert!(controller.runtime.jobs.pending_audio.is_none());
    assert!(controller.runtime.jobs.pending_playback.is_none());
}

#[test]
fn native_set_browser_view_start_preserves_requested_top_row_within_visible_bounds() {
    let mut entries = Vec::new();
    for index in 0..(MAX_RENDERED_BROWSER_ROWS + 8) {
        entries.push(sample_entry(
            &format!("row_{index:03}.wav"),
            crate::sample_sources::Rating::NEUTRAL,
        ));
    }
    let (mut controller, _source) = prepare_with_source_and_wav_entries(entries);
    let visible_count = controller.ui.browser.viewport.visible.len();
    let expected_view_start = visible_count.saturating_sub(1);
    let expected_render_start = visible_count.saturating_sub(MAX_RENDERED_BROWSER_ROWS);

    controller.set_browser_view_start_action(visible_count.saturating_sub(1));

    assert_eq!(
        controller.ui.browser.viewport.view_window_start,
        expected_view_start
    );
    assert_eq!(
        controller.ui.browser.viewport.render_window_start,
        expected_render_start
    );
    assert!(!controller.ui.browser.selection.autoscroll);
}

#[test]
fn focus_after_manual_scroll_preserves_requested_top_row_for_small_visible_lists() {
    let mut entries = Vec::new();
    for index in 0..40 {
        entries.push(sample_entry(
            &format!("row_{index:03}.wav"),
            crate::sample_sources::Rating::NEUTRAL,
        ));
    }
    let (mut controller, _source) = prepare_with_source_and_wav_entries(entries);

    controller.set_browser_view_start_action(7);
    controller.focus_browser_row_only(18);

    assert_eq!(controller.ui.browser.selection.selected_visible, Some(18));
    assert_eq!(controller.ui.browser.viewport.view_window_start, 7);
    assert_eq!(controller.ui.browser.viewport.render_window_start, 0);
    assert!(controller.ui.browser.selection.autoscroll);
}
