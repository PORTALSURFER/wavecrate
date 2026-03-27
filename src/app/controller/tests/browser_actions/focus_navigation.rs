use super::super::super::test_support::{
    prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
};
use crate::app::controller::state::audio::PendingAgeUpdate;
use crate::app::controller::ui::hotkeys;
use crate::app::state::FocusContext;
use crate::app_core::actions::NativeUiAction;
use crate::app_core::controller::AppControllerNativeRuntimeExt;
use crate::app_core::ui::MAX_RENDERED_BROWSER_ROWS;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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
    assert_eq!(controller.sample_view.wav.loaded_wav, None);
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
fn native_focus_browser_row_queues_async_preview_without_blocking_selection() {
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
    controller.audio.pending_age_update = Some(PendingAgeUpdate {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("one.wav"),
        played_at: 123,
    });
    controller.runtime.jobs.pending_audio = None;
    controller.runtime.jobs.pending_playback = None;

    controller.apply_native_ui_action(NativeUiAction::FocusBrowserRow { visible_row: 1 });

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(controller.sample_view.wav.loaded_wav, None);
    assert_eq!(controller.ui.loaded_wav, None);
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

    controller.apply_native_ui_action(NativeUiAction::MoveBrowserFocus { delta: 1 });

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
    assert_eq!(controller.sample_view.wav.loaded_wav, None);
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

    controller.apply_native_ui_action(NativeUiAction::MoveBrowserFocus { delta: 1 });

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

#[test]
fn preview_focus_defers_pending_age_update_until_commit() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);

    controller.focus_browser_row_only(0);
    controller.audio.pending_age_update = Some(PendingAgeUpdate {
        source_id: source.id.clone(),
        root: source.root.clone(),
        relative_path: PathBuf::from("one.wav"),
        played_at: 123,
    });

    controller.focus_browser_row_only(1);
    assert!(controller.audio.pending_age_update.is_some());

    assert!(controller.commit_focused_browser_row());
    assert!(controller.audio.pending_age_update.is_none());
}

#[test]
fn commit_focus_debounces_similarity_refresh_flush() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);

    controller.focus_browser_row_only(0);
    controller.focus_browser_row(1);

    assert!(controller.runtime.pending_similarity_refresh.is_some());
    controller.flush_pending_focused_similarity_highlight_refresh();
    assert!(controller.runtime.pending_similarity_refresh.is_some());
}

#[test]
fn commit_focus_flush_queues_async_similarity_query_without_immediate_highlight() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.sample_view.wav.selected_wav = Some(PathBuf::from("one.wav"));
    controller.defer_focused_similarity_highlight_refresh(
        controller.selected_sample_id().expect("selected sample id"),
        PathBuf::from("one.wav"),
        Some(0),
    );
    controller.runtime.pending_similarity_refresh_not_before =
        Some(Instant::now() - Duration::from_millis(1));

    controller.flush_pending_focused_similarity_highlight_refresh();

    assert!(controller.runtime.pending_similarity_refresh.is_none());
    assert!(
        controller
            .runtime
            .pending_focused_similarity_query
            .is_some()
    );
    assert!(controller.ui.browser.search.focused_similarity.is_none());
}

#[test]
fn f_hotkey_focuses_loaded_sample_in_browser() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.sample_view.wav.loaded_wav = Some(PathBuf::from("two.wav"));
    controller.ui.focus.set_context(FocusContext::Waveform);

    let action = hotkeys::iter_actions()
        .find(|action| {
            matches!(
                action.action,
                radiant::app::UiAction::FocusLoadedSampleInBrowser
            )
        })
        .expect("missing focus loaded sample hotkey");

    controller.handle_hotkey(action, FocusContext::SampleBrowser);

    assert_eq!(controller.ui.focus.context, FocusContext::SampleBrowser);
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert_eq!(controller.ui.browser.selection.selected_visible, Some(1));
}
