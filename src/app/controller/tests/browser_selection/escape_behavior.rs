use super::*;

#[test]
fn escape_clears_selection() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    assert_eq!(controller.ui.browser.selection.selected_paths.len(), 2);

    controller.clear_browser_selection();

    assert!(controller.ui.browser.selection.selected_paths.is_empty());
    assert!(
        controller
            .ui
            .browser
            .selection
            .selection_anchor_visible
            .is_none()
    );
}

#[test]
fn escape_handler_clears_waveform_and_browser_state() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller
        .selection_state
        .range
        .set_range(Some(SelectionRange::new(0.2, 0.8)));
    controller.apply_selection(controller.selection_state.range.range());
    controller
        .ui
        .browser
        .selection
        .selected_paths
        .push(PathBuf::from("one.wav"));
    controller.ui.browser.selection.selection_anchor_visible = Some(0);

    controller.handle_escape();

    assert!(controller.selection_state.range.range().is_none());
    assert!(controller.ui.waveform.selection.is_none());
    assert!(controller.ui.browser.selection.selected_paths.is_empty());
    assert!(
        controller
            .ui
            .browser
            .selection
            .selection_anchor_visible
            .is_none()
    );
}

#[test]
fn escape_clears_waveform_cursor_and_resets_start_marker() {
    let (mut controller, _source) = dummy_controller();
    controller.ui.waveform.cursor = Some(0.55);
    controller.ui.waveform.last_start_marker = Some(0.55);
    controller.ui.waveform.cursor_last_hover_at = Some(Instant::now());
    controller.ui.waveform.cursor_last_navigation_at = Some(Instant::now());

    controller.handle_escape();

    assert!(controller.ui.waveform.cursor.is_none());
    assert_eq!(controller.ui.waveform.last_start_marker, Some(0.0));
    assert!(controller.ui.waveform.cursor_last_hover_at.is_none());
    assert!(controller.ui.waveform.cursor_last_navigation_at.is_none());
}

#[test]
fn escape_stops_playback_before_clearing_selection() {
    let Some(player) = crate::audio::AudioPlayer::playing_for_tests() else {
        return;
    };
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();
    controller.set_wav_entries_for_tests(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller
        .selection_state
        .range
        .set_range(Some(SelectionRange::new(0.25, 0.75)));
    controller.apply_selection(controller.selection_state.range.range());
    controller.audio.player = Some(std::rc::Rc::new(std::cell::RefCell::new(player)));

    controller.handle_escape();

    assert!(controller.selection_state.range.range().is_some());
    assert!(controller.ui.waveform.selection.is_some());
    assert!(!controller.is_playing());
}
