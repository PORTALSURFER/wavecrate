use super::*;
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

#[test]
fn moving_browser_focus_queues_async_preview_playback() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(0);
    controller.sample_view.wav.loaded_wav = Some(PathBuf::from("one.wav"));
    controller.ui.loaded_wav = Some(PathBuf::from("one.wav"));
    controller.runtime.jobs.pending_audio = None;
    controller.runtime.jobs.pending_playback = None;

    controller.focus_browser_delta_action(1);

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(Path::new("one.wav"))
    );
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert!(controller.ui.waveform.image.is_none());
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .map(|pending| pending.relative_path.clone()),
        Some(PathBuf::from("two.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .map(|pending| pending.relative_path.clone()),
        Some(PathBuf::from("two.wav"))
    );
}

#[test]
fn moving_browser_focus_preserves_multi_selection_and_anchor() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("three.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(0);
    controller.toggle_focused_selection();
    controller.runtime.jobs.pending_audio = None;
    controller.runtime.jobs.pending_playback = None;

    controller.focus_browser_delta_action(1);

    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(0)
    );
    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![PathBuf::from("one.wav")]
    );
    assert_eq!(
        controller.ui.browser.selection.last_focused_path.as_deref(),
        Some(Path::new("two.wav"))
    );
}

#[test]
fn shift_extension_after_keyboard_toggle_uses_preserved_anchor() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("three.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(0);
    controller.toggle_focused_selection();
    controller.focus_browser_delta_action(1);
    controller.focus_browser_delta_action(1);

    controller.extend_browser_selection_from_focus_action(-1);

    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(0)
    );
    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![PathBuf::from("one.wav"), PathBuf::from("two.wav")]
    );
}

#[test]
fn native_focus_browser_row_clears_selection_and_queues_async_preview() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.settings.feature_flags.autoplay_selection = false;
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("three.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(0);
    controller.toggle_focused_selection();
    controller.sample_view.wav.loaded_wav = Some(PathBuf::from("one.wav"));
    controller.ui.loaded_wav = Some(PathBuf::from("one.wav"));
    controller.audio.pending_age_update = Some(PendingAgeUpdate {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("one.wav"),
        played_at: 123,
    });
    controller.runtime.jobs.pending_audio = None;
    controller.runtime.jobs.pending_playback = None;

    controller.apply_ui_action(NativeUiAction::FocusBrowserRow { visible_row: 1 });

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(Path::new("one.wav"))
    );
    assert_eq!(
        controller.ui.loaded_wav.as_deref(),
        Some(Path::new("one.wav"))
    );
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert!(controller.ui.waveform.image.is_none());
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .map(|pending| pending.relative_path.clone()),
        Some(PathBuf::from("two.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .map(|pending| pending.relative_path.clone()),
        Some(PathBuf::from("two.wav"))
    );
    assert!(controller.ui.browser.selection.selected_paths.is_empty());
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(1)
    );
    assert!(controller.audio.pending_age_update.is_some());
    assert!(controller.runtime.pending_similarity_refresh.is_none());
}

#[test]
fn native_move_browser_focus_queues_async_preview_playback() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.settings.feature_flags.autoplay_selection = false;
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(0);
    controller.sample_view.wav.loaded_wav = Some(PathBuf::from("one.wav"));
    controller.ui.loaded_wav = Some(PathBuf::from("one.wav"));
    controller.runtime.jobs.pending_audio = None;
    controller.runtime.jobs.pending_playback = None;

    controller.apply_ui_action(NativeUiAction::MoveBrowserFocus { delta: 1 });

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(Path::new("one.wav"))
    );
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert!(controller.ui.waveform.image.is_none());
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .map(|pending| pending.relative_path.clone()),
        Some(PathBuf::from("two.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .map(|pending| pending.relative_path.clone()),
        Some(PathBuf::from("two.wav"))
    );
}

#[test]
fn native_move_browser_focus_preserves_multi_selection_membership() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.settings.feature_flags.autoplay_selection = false;
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("three.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(0);
    controller.toggle_focused_selection();
    controller.runtime.jobs.pending_audio = None;
    controller.runtime.jobs.pending_playback = None;

    controller.apply_ui_action(NativeUiAction::MoveBrowserFocus { delta: 1 });

    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(
        controller.ui.browser.selection.selection_anchor_visible,
        Some(0)
    );
    assert_eq!(
        controller.ui.browser.selection.selected_paths,
        vec![PathBuf::from("one.wav")]
    );
}

#[test]
fn native_move_browser_focus_uses_random_mode_pool_without_repeating_current_row() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("three.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.focus_browser_row_only(0);
    controller.toggle_random_navigation_mode();
    controller
        .history
        .random_history
        .mark_played(&source.id, Path::new("two.wav"));
    controller.runtime.jobs.pending_audio = None;
    controller.runtime.jobs.pending_playback = None;

    controller.apply_ui_action(NativeUiAction::MoveBrowserFocus { delta: 1 });

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("three.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(2));
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .map(|pending| pending.relative_path.clone()),
        Some(PathBuf::from("three.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .map(|pending| pending.relative_path.clone()),
        Some(PathBuf::from("three.wav"))
    );
}
