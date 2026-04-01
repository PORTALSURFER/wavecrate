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
