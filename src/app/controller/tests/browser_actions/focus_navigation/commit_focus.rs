use super::*;

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

    controller.prepare_native_frame(false);

    assert!(controller.runtime.pending_similarity_refresh.is_some());
    controller.flush_pending_focused_similarity_highlight_refresh();
    assert!(controller.runtime.pending_similarity_refresh.is_some());
}

#[test]
fn commit_focused_browser_row_ignores_hidden_stale_focus() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.settings.feature_flags.autoplay_selection = false;

    controller.focus_browser_row_only(1);
    controller.runtime.jobs.pending_audio = None;
    controller.runtime.jobs.pending_playback = None;

    controller.set_browser_search("one");

    assert_eq!(controller.ui.browser.selection.selected_visible, None);
    assert_eq!(
        controller.ui.browser.selection.last_focused_path.as_deref(),
        Some(Path::new("two.wav"))
    );

    assert!(!controller.commit_focused_browser_row());
    assert!(controller.runtime.jobs.pending_audio.is_none());
    assert!(controller.runtime.jobs.pending_playback.is_none());
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
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
fn commit_focus_after_preview_same_row_applies_commit_side_effects() {
    let (mut controller, _source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);

    controller.focus_browser_row_only(1);

    assert!(controller.history.focus_history.entries.is_empty());
    assert!(controller.runtime.pending_similarity_refresh.is_none());
    assert!(controller.ui.browser.selection.commit_focus_pending);

    assert!(controller.commit_focused_browser_row());
    controller.prepare_native_frame(false);

    let focused = controller
        .history
        .focus_history
        .entries
        .back()
        .expect("focused history entry");
    assert_eq!(focused.relative_path, Path::new("two.wav"));
    assert!(controller.runtime.pending_similarity_refresh.is_some());
    assert!(!controller.ui.browser.selection.commit_focus_pending);
}

#[test]
fn commit_focus_defers_audio_dispatch_until_frame_prepare() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.settings.feature_flags.autoplay_selection = false;
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(0);
    controller.runtime.jobs.pending_audio = None;
    controller.runtime.jobs.pending_playback = None;

    controller.focus_browser_row(1);

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("two.wav"))
    );
    assert_eq!(controller.ui.waveform.loading.as_deref(), Some(Path::new("two.wav")));
    assert!(controller.runtime.jobs.pending_audio.is_none());
    assert!(controller.runtime.pending_browser_focus_commit.is_some());
    assert!(controller.history.focus_history.entries.is_empty());
    assert!(controller.runtime.pending_similarity_refresh.is_none());

    controller.prepare_native_frame(false);

    assert!(controller.runtime.pending_browser_focus_commit.is_none());
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("two.wav"))
    );
    assert!(controller
        .history
        .focus_history
        .entries
        .back()
        .is_some_and(|entry| entry.relative_path == Path::new("two.wav")));
    assert!(controller.runtime.pending_similarity_refresh.is_some());
}

#[test]
fn stale_commit_focus_loading_is_dropped_when_focus_changes_before_prepare() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", crate::sample_sources::Rating::NEUTRAL),
        sample_entry("two.wav", crate::sample_sources::Rating::NEUTRAL),
    ]);
    controller.settings.feature_flags.autoplay_selection = false;
    write_test_wav(&source.root.join("one.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("two.wav"), &[0.0, 0.1]);

    controller.focus_browser_row_only(0);
    controller.runtime.jobs.pending_audio = None;
    controller.runtime.jobs.pending_playback = None;

    controller.focus_browser_row(1);
    controller.focus_browser_row_only(0);

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("one.wav"))
    );
    assert_eq!(controller.ui.waveform.loading.as_deref(), Some(Path::new("two.wav")));

    controller.prepare_native_frame(false);

    assert!(controller.runtime.jobs.pending_audio.is_none());
    assert!(controller.runtime.jobs.pending_playback.is_none());
    assert!(controller.ui.waveform.loading.is_none());
    assert!(controller.history.focus_history.entries.is_empty());
    assert!(controller.runtime.pending_similarity_refresh.is_none());
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
