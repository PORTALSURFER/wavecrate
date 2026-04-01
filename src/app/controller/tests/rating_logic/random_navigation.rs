use super::*;

#[test]
fn advance_after_rating_respects_random_navigation() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("a.wav", Rating::NEUTRAL),
        sample_entry("b.wav", Rating::NEUTRAL),
        sample_entry("c.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("a.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("b.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("c.wav"), &[0.0, 0.1]);

    controller.ui.browser.search.random_navigation_mode = true;
    controller.settings.controls.advance_after_rating = true;

    let id = source.id.clone();
    controller
        .history
        .random_history
        .mark_played(&id, &PathBuf::from("a.wav"));
    controller
        .history
        .random_history
        .mark_played(&id, &PathBuf::from("b.wav"));

    controller.focus_browser_row(0);
    assert_eq!(controller.selected_row_index(), Some(0));

    controller.adjust_selected_rating(1);

    let selected_path = controller
        .sample_view
        .wav
        .selected_wav
        .as_ref()
        .expect("selection");
    assert_eq!(selected_path, &PathBuf::from("c.wav"));
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_audio
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("c.wav"))
    );
    assert_eq!(
        controller
            .runtime
            .jobs
            .pending_playback
            .as_ref()
            .map(|pending| pending.relative_path.as_path()),
        Some(Path::new("c.wav"))
    );
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(Path::new("c.wav"))
    );
    assert!(controller.ui.waveform.image.is_none());

    wait_for_loaded_waveform(&mut controller, Path::new("c.wav"));

    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(Path::new("c.wav"))
    );
    assert!(controller.runtime.jobs.pending_audio.is_none());
    assert!(controller.ui.waveform.loading.is_none());
    assert!(controller.ui.waveform.image.is_some());
}

#[test]
fn rating_previous_random_history_entry_restores_waveform_for_replacement() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![
        sample_entry("a.wav", Rating::NEUTRAL),
        sample_entry("b.wav", Rating::NEUTRAL),
        sample_entry("c.wav", Rating::NEUTRAL),
    ]);
    write_test_wav(&source.root.join("a.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("b.wav"), &[0.0, 0.1]);
    write_test_wav(&source.root.join("c.wav"), &[0.0, 0.1]);

    controller.settings.controls.advance_after_rating = true;
    controller.settings.feature_flags.autoplay_selection = false;
    controller.set_browser_rating_filter(0, false);
    controller.toggle_random_navigation_mode();

    let source_id = source.id.clone();
    controller
        .history
        .random_history
        .mark_played(&source_id, Path::new("b.wav"));
    controller
        .history
        .random_history
        .mark_played(&source_id, Path::new("c.wav"));
    controller
        .history
        .random_history
        .entries
        .push_back(RandomHistoryEntry {
            source_id: source_id.clone(),
            relative_path: PathBuf::from("b.wav"),
        });
    controller
        .history
        .random_history
        .entries
        .push_back(RandomHistoryEntry {
            source_id,
            relative_path: PathBuf::from("c.wav"),
        });
    controller.history.random_history.cursor = Some(1);

    controller.play_previous_random_sample();
    wait_for_loaded_waveform(&mut controller, Path::new("b.wav"));

    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(Path::new("b.wav"))
    );
    assert!(controller.ui.waveform.image.is_some());

    controller.adjust_selected_rating(1);

    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(Path::new("a.wav"))
    );
    assert_eq!(
        controller.ui.waveform.loading.as_deref(),
        Some(Path::new("a.wav"))
    );
    assert!(controller.ui.waveform.image.is_none());

    wait_for_loaded_waveform(&mut controller, Path::new("a.wav"));

    assert_eq!(
        controller.sample_view.wav.loaded_wav.as_deref(),
        Some(Path::new("a.wav"))
    );
    assert!(controller.runtime.jobs.pending_audio.is_none());
    assert!(controller.ui.waveform.loading.is_none());
    assert!(controller.ui.waveform.image.is_some());
}
