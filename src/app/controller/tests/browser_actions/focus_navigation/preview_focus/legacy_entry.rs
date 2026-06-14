use super::super::*;
use crate::app::state::TriageFlagFilter;

#[test]
fn focus_hotkey_does_not_autoplay_browser_sample() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);

    assert!(controller.settings.feature_flags.autoplay_selection);

    controller.focus_browser_list();

    assert_eq!(controller.ui.focus.context, FocusContext::SampleBrowser);
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("one.wav"))
    );
    assert!(controller.runtime.jobs.pending_playback.is_none());
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(0));
}

#[test]
fn focus_browser_list_uses_first_visible_row_when_filters_hide_absolute_row_zero() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("three.wav"), &[0.0, 0.1]);
    controller.set_browser_search("two");

    controller.focus_browser_list();

    assert_eq!(controller.ui.focus.context, FocusContext::SampleBrowser);
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(0));
    assert!(controller.runtime.jobs.pending_playback.is_none());
}

#[test]
fn focus_browser_list_prefers_current_focus_over_stale_anchor_when_reentering() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("three.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(0);
    controller.extend_browser_selection_to_row(2);
    controller.focus_waveform_context();

    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(0)
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(2));

    controller.focus_browser_list();

    assert_eq!(controller.ui.focus.context, FocusContext::SampleBrowser);
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("three.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(2));
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(2)
    );
}

#[test]
fn focus_browser_list_ignores_hidden_stale_anchor_when_filters_replace_visible_rows() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("keep-a.wav", crate::sample_sources::Rating::KEEP_1),
        sample_entry("neutral-a.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("neutral-b.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("keep-b.wav", crate::sample_sources::Rating::KEEP_1),
    ]);
    write_test_wav(&source.root.join("keep-a.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("neutral-a.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("neutral-b.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("keep-b.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(1);
    controller.extend_browser_selection_to_row(2);
    controller.focus_waveform_context();
    controller.set_browser_filter(TriageFlagFilter::Keep);

    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(1)
    );
    assert!(controller.ui.browser.selection.selected_visible.is_none());
    assert_eq!(controller.ui.browser.viewport.visible.len(), 2);

    controller.focus_browser_list();

    assert_eq!(controller.ui.focus.context, FocusContext::SampleBrowser);
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("keep-a.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(0));
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(0)
    );
}
