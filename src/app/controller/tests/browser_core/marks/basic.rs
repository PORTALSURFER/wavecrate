use super::*;

#[test]
fn browser_sample_mark_toggle_marks_and_unmarks_focused_row() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
    ]);

    controller.focus_browser_row_only(1);
    controller.toggle_browser_sample_mark();

    assert!(controller.browser_sample_marked(&source.id, Path::new("two.wav")));
    assert!(!controller.browser_sample_marked(&source.id, Path::new("one.wav")));

    controller.toggle_browser_sample_mark();

    assert!(!controller.browser_sample_marked(&source.id, Path::new("two.wav")));
}

#[test]
fn browser_sample_mark_toggle_applies_to_selection_and_focused_row() {
    let (mut controller, source) = browser_mark_fixture();

    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    controller.toggle_browser_sample_mark();

    assert!(controller.browser_sample_marked(&source.id, Path::new("one.wav")));
    assert!(controller.browser_sample_marked(&source.id, Path::new("two.wav")));
    assert!(!controller.browser_sample_marked(&source.id, Path::new("three.wav")));
}

#[test]
fn multi_selection_mark_does_not_auto_advance() {
    let (mut controller, source) = browser_mark_fixture();

    controller.focus_browser_row_only(0);
    controller.toggle_browser_row_selection(1);
    controller.toggle_browser_sample_mark();

    assert!(controller.browser_sample_marked(&source.id, Path::new("one.wav")));
    assert!(controller.browser_sample_marked(&source.id, Path::new("two.wav")));
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert!(controller.runtime.jobs.pending_audio.is_none());
    assert!(controller.runtime.jobs.pending_playback.is_none());
    assert!(controller.ui.waveform.loading.is_none());
}
