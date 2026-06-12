use super::*;

#[test]
fn keyboard_navigation_defers_sample_loading_until_navigation_settles() {
    let source_root = tempfile::tempdir().expect("source root");
    for name in ["a.wav", "b.wav", "c.wav"] {
        fs::write(source_root.path().join(name), []).expect("sample file");
    }

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let files = state.library.folder_browser.selected_audio_files();
    assert!(files.len() >= 3, "expected three visible samples");
    let first = files[0].id.clone();
    let second = files[1].id.clone();
    let third = files[2].id.clone();
    state.library.folder_browser.select_file(first);

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::NavigateBrowser {
            delta: 1,
            extend: false,
        },
        &mut context,
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(second.as_str())
    );
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_some(),
        "keyboard navigation should queue only a deferred latest load"
    );
    assert!(
        state.background.sample_load_task.active().is_none(),
        "keyboard navigation must not synchronously start decode work"
    );
    assert_eq!(
        state.waveform.load.label, None,
        "keyboard navigation should not enter the loading UI until the deferred load fires"
    );
    let stale_ticket = state
        .background
        .deferred_sample_load_task
        .active()
        .expect("deferred navigation load ticket");

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::NavigateBrowser {
            delta: 1,
            extend: false,
        },
        &mut context,
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(third.as_str())
    );
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_some()
    );
    assert!(state.background.sample_load_task.active().is_none());

    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::DeferredSampleLoad {
            ticket: stale_ticket,
            path: second,
            autoplay: true,
            check_cache: false,
            scheduled_at: std::time::Instant::now(),
        },
        &mut context,
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(third.as_str())
    );
    assert!(
        state.background.sample_load_task.active().is_none(),
        "stale deferred navigation loads must not start decode work"
    );
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_some()
    );
}

#[test]
fn keyboard_navigation_uses_memory_waveform_cache_without_worker() {
    let source_root = tempfile::tempdir().expect("source root");
    let first_path = source_root.path().join("a.wav");
    let second_path = source_root.path().join("b.wav");
    write_test_wav_i16(&first_path, &[0, 256, -256, 512]);
    write_test_wav_i16(&second_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let first = first_path.display().to_string();
    let second = second_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.library.folder_browser.select_file(first);
    let loaded =
        crate::native_app::test_support::state::WaveformState::load_path(second_path.clone())
            .expect("sample loads");
    state.remember_waveform(&loaded);
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::synthetic_for_tests();

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::NavigateBrowser {
            delta: 1,
            extend: false,
        },
        &mut context,
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(second.as_str())
    );
    assert_eq!(state.waveform.current.path(), second_path);
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none(),
        "memory-cached keyboard navigation should not debounce a reload"
    );
    assert!(
        state.background.sample_load_task.active().is_none(),
        "memory-cached keyboard navigation should not queue decode work"
    );
    assert!(
        state.ui.status.sample.contains("b.wav"),
        "cached keyboard navigation should update the visible status, got {}",
        state.ui.status.sample
    );
}

#[test]
fn keyboard_navigation_defers_persisted_cache_probe_until_navigation_settles() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let first_path = source_root.path().join("a.wav");
    let second_path = source_root.path().join("b.wav");
    write_test_wav_i16(&first_path, &[0, 256, -256, 512]);
    write_test_wav_i16(&second_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let first = first_path.display().to_string();
    let second = second_path.display().to_string();

    let waveform =
        crate::native_app::test_support::state::WaveformState::load_path(second_path.clone())
            .expect("cache sample");
    let file = waveform.file();
    crate::native_app::waveform::store_cached_waveform_file_for_tests(&file);
    wait_for_playback_ready_cache(&second);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.library.folder_browser.select_file(first);

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::NavigateBrowser {
            delta: 1,
            extend: false,
        },
        &mut context,
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(second.as_str())
    );
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_some(),
        "keyboard navigation should debounce persisted cache promotion"
    );
    assert!(
        state.background.sample_load_task.active().is_none(),
        "keyboard navigation must not probe persisted playback cache on the UI thread"
    );
    assert_eq!(
        state.waveform.load.label, None,
        "keyboard navigation should keep focus movement separate from loading UI"
    );

    let deferred_ticket = state
        .background
        .deferred_sample_load_task
        .active()
        .expect("deferred persisted cache load");
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::DeferredSampleLoad {
            ticket: deferred_ticket,
            path: second,
            autoplay: true,
            check_cache: true,
            scheduled_at: std::time::Instant::now(),
        },
        &mut context,
    );

    assert!(
        state.background.sample_load_task.active().is_some(),
        "deferred keyboard load should start cache promotion only after navigation settles"
    );
}

#[test]
fn keyboard_navigation_plays_loaded_sample_without_deferred_reload() {
    let Ok(player) = wavecrate::audio::AudioPlayer::new() else {
        return;
    };
    let source_root = tempfile::tempdir().expect("source root");
    for (name, samples) in [
        ("a.wav", &[0, 256, -256, 512][..]),
        ("b.wav", &[0, 1024, -2048, 4096, -1024, 512][..]),
    ] {
        write_test_wav_i16(&source_root.path().join(name), samples);
    }

    let mut state = gui_state_for_span_tests();
    state.audio.player = Some(player);
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let files = state.library.folder_browser.selected_audio_files();
    assert!(files.len() >= 2, "expected two visible samples");
    let first = files[0].id.clone();
    let second = files[1].id.clone();
    state.library.folder_browser.select_file(first);
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(PathBuf::from(&second))
            .expect("sample loads");

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::NavigateBrowser {
            delta: 1,
            extend: false,
        },
        &mut context,
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(second.as_str())
    );
    assert!(
        state.waveform.current.is_playing(),
        "resident waveform should audition immediately during keyboard navigation"
    );
    assert_eq!(state.audio.current_playback_span, Some((0.0, 1.0)));
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none(),
        "already loaded navigation target should not queue a deferred reload"
    );
    assert!(
        state.background.sample_load_task.active().is_none(),
        "already loaded navigation target must not start decode work"
    );
}
