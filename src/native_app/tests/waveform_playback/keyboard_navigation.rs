use super::*;

#[test]
fn rapid_navigation_harness_keeps_ui_responsive_while_business_work_is_slow() {
    let source_root = tempfile::tempdir().expect("source root");
    for name in ["a.wav", "b.wav", "c.wav"] {
        write_test_wav_i16(
            &source_root.path().join(name),
            &[0, 256, -256, 512, -512, 128],
        );
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
    let state = std::rc::Rc::new(std::cell::RefCell::new(state));
    let state_for_view = std::rc::Rc::clone(&state);
    let state_for_update = std::rc::Rc::clone(&state);
    let bridge = ui::app(())
        .view(move |()| {
            let mut state = lock_navigation_harness_state(&state_for_view);
            crate::native_app::test_support::state::view(&mut state)
        })
        .handle_message(move |(), message, context| {
            lock_navigation_harness_state(&state_for_update).apply_message(message, context);
        })
        .into_bridge();
    let mut runtime = radiant::runtime::SurfaceRuntime::new(bridge, ui::Vector2::new(900.0, 620.0));
    apply_strict_update_diagnostics(&mut runtime);

    runtime.dispatch_message(
        crate::native_app::test_support::state::GuiMessage::NavigateBrowser {
            delta: 1,
            extend: false,
            preserve_selection: false,
        },
    );
    assert_eq!(
        lock_navigation_harness_state(&state)
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned),
        Some(second.clone()),
        "navigation feedback must update before deferred business work completes"
    );
    assert!(
        lock_navigation_harness_state(&state)
            .background
            .sample_load_task
            .active()
            .is_none(),
        "first key repeat should not synchronously start decode work"
    );
    let stale_deferred_ticket = lock_navigation_harness_state(&state)
        .background
        .deferred_sample_load_task
        .active()
        .expect("first navigation queues deferred load");

    runtime.dispatch_message(
        crate::native_app::test_support::state::GuiMessage::NavigateBrowser {
            delta: 1,
            extend: false,
            preserve_selection: false,
        },
    );
    assert_eq!(
        lock_navigation_harness_state(&state)
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned),
        Some(third.clone()),
        "rapid navigation should advance selection without waiting for the older load"
    );

    runtime.dispatch_message(
        crate::native_app::test_support::state::GuiMessage::DeferredSampleLoad {
            ticket: stale_deferred_ticket,
            path: second.clone(),
            autoplay: true,
            check_cache: true,
            scheduled_at: std::time::Instant::now(),
        },
    );
    assert!(
        lock_navigation_harness_state(&state)
            .background
            .sample_load_task
            .active()
            .is_none(),
        "stale deferred navigation work must not start a sample-load worker"
    );

    let current_deferred_ticket = lock_navigation_harness_state(&state)
        .background
        .deferred_sample_load_task
        .active()
        .expect("current navigation keeps a deferred load");
    runtime.dispatch_message(
        crate::native_app::test_support::state::GuiMessage::DeferredSampleLoad {
            ticket: current_deferred_ticket,
            path: third.clone(),
            autoplay: true,
            check_cache: true,
            scheduled_at: std::time::Instant::now(),
        },
    );
    let stale_sample_load_ticket = lock_navigation_harness_state(&state)
        .background
        .sample_load_task
        .active()
        .expect("settled navigation queues sample-load business work");
    let diagnostics_after_queue = runtime.runtime_diagnostics();
    assert_eq!(
        diagnostics_after_queue.ui.slow_update_handlers, 0,
        "sample navigation updates should stay below Radiant's slow-handler threshold"
    );
    assert!(
        diagnostics_after_queue
            .business
            .recent
            .iter()
            .any(|event| event.name == "gui-sample-load"),
        "settled navigation should use Radiant BusinessRuntime for sample load work"
    );

    runtime.dispatch_message(
        crate::native_app::test_support::state::GuiMessage::NavigateBrowser {
            delta: -1,
            extend: false,
            preserve_selection: false,
        },
    );
    assert_eq!(
        lock_navigation_harness_state(&state)
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned),
        Some(second.clone()),
        "new navigation should update immediately while the previous worker is pending"
    );

    runtime.dispatch_message(
        crate::native_app::test_support::state::GuiMessage::SampleLoadFinished(
            ui::TaskCompletion {
                ticket: stale_sample_load_ticket,
                output: crate::native_app::test_support::state::SampleLoadResult {
                    path: third,
                    result: Err(String::from("synthetic slow decode finished late")),
                    autoplay: true,
                },
            },
        ),
    );

    assert_eq!(
        lock_navigation_harness_state(&state)
            .library
            .folder_browser
            .selected_file_id()
            .map(str::to_owned),
        Some(second.clone()),
        "stale worker completion must not overwrite current navigation state"
    );
    assert!(
        !lock_navigation_harness_state(&state)
            .ui
            .status
            .sample
            .contains("synthetic slow decode"),
        "stale worker errors must not surface as current sample-load failures"
    );
    assert_eq!(
        runtime.runtime_diagnostics().ui.slow_update_handlers,
        0,
        "stale completion handling should also stay off the slow UI path"
    );
}

fn lock_navigation_harness_state(
    state: &std::rc::Rc<std::cell::RefCell<NativeAppState>>,
) -> std::cell::RefMut<'_, NativeAppState> {
    state.borrow_mut()
}

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

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::NavigateBrowser {
            delta: 1,
            extend: false,
            preserve_selection: false,
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
            preserve_selection: false,
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

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::NavigateBrowser {
            delta: 1,
            extend: false,
            preserve_selection: false,
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

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::NavigateBrowser {
            delta: 1,
            extend: false,
            preserve_selection: false,
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

    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::NavigateBrowser {
            delta: 1,
            extend: false,
            preserve_selection: false,
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
