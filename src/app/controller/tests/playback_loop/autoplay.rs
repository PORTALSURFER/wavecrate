use super::*;

#[test]
fn selection_autoplay_preserves_active_loop_playback() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.settings.feature_flags.autoplay_selection = true;
    controller.ui.waveform.loop_enabled = true;

    let wav_path = source.root.join("non_loop.wav");
    write_test_wav(&wav_path, &[0.0, 0.5, -0.5]);
    controller.set_wav_entries_for_tests(vec![WavEntry {
        relative_path: PathBuf::from("non_loop.wav"),
        file_size: 0,
        modified_ns: 0,
        content_hash: None,
        tag: crate::sample_sources::Rating::NEUTRAL,
        looped: false,
        locked: false,
        missing: false,
        last_played_at: None,
    }]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.select_wav_by_path(Path::new("non_loop.wav"));

    assert!(controller.ui.waveform.loop_enabled);
    let pending = controller
        .runtime
        .jobs
        .pending_playback
        .as_ref()
        .expect("pending playback to be queued");
    assert!(pending.looped);
}

#[test]
fn loading_non_looped_sample_preserves_loop_when_locked() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.settings.feature_flags.autoplay_selection = true;
    controller.ui.waveform.loop_enabled = true;
    controller.set_loop_lock_enabled(true);

    let wav_path = source.root.join("locked_loop.wav");
    write_test_wav(&wav_path, &[0.0, 0.5, -0.5]);
    controller.set_wav_entries_for_tests(vec![WavEntry {
        relative_path: PathBuf::from("locked_loop.wav"),
        file_size: 0,
        modified_ns: 0,
        content_hash: None,
        tag: crate::sample_sources::Rating::NEUTRAL,
        looped: false,
        locked: false,
        missing: false,
        last_played_at: None,
    }]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.select_wav_by_path(Path::new("locked_loop.wav"));

    assert!(controller.ui.waveform.loop_enabled);
    let pending = controller
        .runtime
        .jobs
        .pending_playback
        .as_ref()
        .expect("pending playback to be queued");
    assert!(pending.looped);
}

#[test]
fn loading_looped_sample_preserves_locked_off_override() {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    controller.settings.feature_flags.autoplay_selection = true;
    controller.ui.waveform.loop_enabled = false;
    controller.set_loop_lock_enabled(true);

    let wav_path = source.root.join("locked_loop_off.wav");
    write_test_wav(&wav_path, &[0.0, 0.5, -0.5]);
    controller.set_wav_entries_for_tests(vec![WavEntry {
        relative_path: PathBuf::from("locked_loop_off.wav"),
        file_size: 0,
        modified_ns: 0,
        content_hash: None,
        tag: crate::sample_sources::Rating::NEUTRAL,
        looped: true,
        locked: false,
        missing: false,
        last_played_at: None,
    }]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    controller.select_wav_by_path(Path::new("locked_loop_off.wav"));

    assert!(!controller.ui.waveform.loop_enabled);
    let pending = controller
        .runtime
        .jobs
        .pending_playback
        .as_ref()
        .expect("pending playback to be queued");
    assert!(!pending.looped);
}
