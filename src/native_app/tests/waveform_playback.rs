use super::*;

static WAVEFORM_CONFIG_BASE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn set_waveform_test_config_base(
    path: PathBuf,
) -> (
    std::sync::MutexGuard<'static, ()>,
    wavecrate::app_dirs::ConfigBaseGuard,
) {
    let lock = WAVEFORM_CONFIG_BASE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let guard = wavecrate::app_dirs::ConfigBaseGuard::set(path);
    (lock, guard)
}

#[test]
fn looped_waveform_click_resolves_to_full_sample_without_playmark() {
    let mut state = gui_state_for_span_tests();
    state.audio.loop_playback = true;

    let span = state.resolve_playback_span(0.45, 1.0, None);

    assert_eq!(span.start_ratio, 0.0);
    assert_eq!(span.end_ratio, 1.0);
    assert_eq!(span.offset_ratio, 0.45);
}

#[test]
fn looped_waveform_click_resolves_to_playmark_span_when_selected() {
    let mut state = gui_state_for_span_tests();
    state.audio.loop_playback = true;
    state
        .waveform
        .current
        .apply_interaction(WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Play,
            visible_ratio: 0.25,
        });
    state
        .waveform
        .current
        .apply_interaction(WaveformInteraction::UpdateSelection {
            visible_ratio: 0.60,
        });
    state
        .waveform
        .current
        .apply_interaction(WaveformInteraction::FinishSelection {
            visible_ratio: 0.60,
        });

    let inside_span = state.resolve_playback_span(0.45, 1.0, None);
    assert_eq!(inside_span.start_ratio, 0.25);
    assert_eq!(inside_span.end_ratio, 0.60);
    assert_eq!(inside_span.offset_ratio, 0.45);

    let outside_span = state.resolve_playback_span(0.85, 1.0, None);
    assert_eq!(outside_span.start_ratio, 0.25);
    assert_eq!(outside_span.end_ratio, 0.60);
    assert_eq!(outside_span.offset_ratio, 0.25);
}

#[test]
fn random_audition_span_uses_fixed_window_inside_long_sample() {
    let (start, end) = crate::native_app::audio::playback::random_audition_span_for_unit(20.0, 0.5);

    assert!((start - 0.4).abs() < 0.001, "start was {start}");
    assert!((end - 0.6).abs() < 0.001, "end was {end}");
}

#[test]
fn random_audition_span_plays_whole_short_sample() {
    assert_eq!(
        crate::native_app::audio::playback::random_audition_span_for_unit(2.0, 0.75),
        (0.0, 1.0)
    );
}

#[test]
fn random_audition_prefers_marked_play_ranges_and_selects_the_chosen_range() {
    let mut state = gui_state_for_span_tests();

    for (start, end) in [(0.10, 0.20), (0.55, 0.70)] {
        state
            .waveform
            .current
            .apply_interaction(WaveformInteraction::BeginSelection {
                kind: WaveformSelectionKind::Play,
                visible_ratio: start,
            });
        state
            .waveform
            .current
            .apply_interaction(WaveformInteraction::UpdateSelection { visible_ratio: end });
        state
            .waveform
            .current
            .apply_interaction(WaveformInteraction::FinishSelection { visible_ratio: end });
    }

    let span = state.random_audition_span_for_loaded_waveform(0.75);

    assert_eq!(
        span.source,
        crate::native_app::audio::playback::RandomAuditionSource::MarkedRange
    );
    assert!(
        (span.start - 0.55).abs() < 0.001,
        "start was {}",
        span.start
    );
    assert!((span.end - 0.70).abs() < 0.001, "end was {}", span.end);
    assert_eq!(
        state.waveform.current.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.55, 0.70))
    );
}

#[test]
fn random_audition_is_one_shot_even_when_loop_is_enabled() {
    let Ok(player) = wavecrate::audio::AudioPlayer::new() else {
        return;
    };
    let mut state = gui_state_for_span_tests();
    state.audio.player = Some(player);
    state.audio.loop_playback = true;

    let mut context = ui::UpdateContext::default();
    state.play_random_sample_range_with_unit(0.5, &mut context);

    assert!(!state.audio.loop_playback);
    assert!(state.waveform.current.is_playing());
    assert_eq!(state.audio.current_playback_span, Some((0.0, 1.0)));
    assert!(
        state
            .audio
            .player
            .as_ref()
            .is_some_and(|player| !player.is_looping())
    );
}

#[test]
fn random_audition_for_unloaded_selection_resumes_after_sample_load() {
    let (mut state, _source_root, selected_file) =
        native_app_state_with_temp_sample("random-load.wav");
    let path = PathBuf::from(&selected_file);
    write_test_wav_i16(&path, &[0, 1024, -2048, 4096, -1024, 512]);
    state.waveform.current = crate::native_app::test_support::WaveformState::empty();
    state.audio.loop_playback = true;
    assert!(!state.waveform.current.has_loaded_sample());

    let mut context = ui::UpdateContext::default();
    state.play_random_sample_range_with_unit(0.5, &mut context);

    assert!(matches!(
        state.audio.pending_sample_playback,
        Some(crate::native_app::test_support::PendingSamplePlayback::RandomAudition { unit })
            if (unit - 0.5).abs() < f32::EPSILON
    ));

    start_deferred_sample_load_for_tests(&mut state, selected_file.clone(), false, &mut context);
    let ticket = state
        .background
        .sample_load_task
        .active()
        .expect("sample load queued");
    state.apply_message(
        crate::native_app::test_support::GuiMessage::SampleLoadFinished(ui::TaskCompletion {
            ticket,
            output: crate::native_app::test_support::SampleLoadResult {
                path: selected_file.clone(),
                result: crate::native_app::test_support::WaveformState::load_path(path),
                autoplay: false,
            },
        }),
        &mut context,
    );

    assert_eq!(state.audio.pending_sample_playback, None);
    assert!(
        state.audio.pending_playback_start.is_some(),
        "random audition should request playback even when the audio device is still opening"
    );
    assert!(
        !state.audio.loop_playback,
        "random audition should remain one-shot after the selected sample loads"
    );
    assert!(
        state.ui.status.sample.contains("Playback unavailable")
            || state.ui.status.sample.contains("Random audition"),
        "random load completion should route through random playback, got {}",
        state.ui.status.sample
    );
}

#[test]
fn normalize_wav_file_in_place_scales_loaded_sample_peak() {
    let root = std::env::temp_dir().join(format!(
        "wavecrate-default-gui-normalize-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(&root).expect("create temp root");
    let path = root.join("quiet.wav");
    write_test_wav_i16(&path, &[0, 1024, -2048, 4096]);

    crate::native_app::test_support::normalize_wav_file_in_place(&path).expect("normalize wav");

    let samples = read_test_wav_f32(&path);
    let peak = samples
        .iter()
        .copied()
        .map(f32::abs)
        .fold(0.0_f32, f32::max);
    assert!((peak - 1.0).abs() < 0.000_001, "peak was {peak}");
    assert!(samples.iter().all(|sample| sample.is_finite()));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn normalize_selected_samples_queues_worker_without_rewriting_on_ui_thread() {
    let (mut state, _source_root, selected_file) = native_app_state_with_temp_sample("quiet.wav");
    let path = PathBuf::from(&selected_file);
    write_test_wav_i16(&path, &[0, 1024, -2048, 4096]);
    let before = fs::read(&path).expect("read wav before normalization");

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::NormalizeSelectedSamples,
        &mut context,
    );

    assert_eq!(
        fs::read(&path).expect("read wav after queue"),
        before,
        "normalization must not rewrite the selected sample on the UI thread"
    );
    let progress = state
        .background
        .normalization_progress
        .as_ref()
        .expect("normalization progress should be visible after queueing");
    assert_eq!(progress.completed, 0);
    assert_eq!(progress.total, 1);
    assert_eq!(progress.detail, "Queued");
    assert!(state.ui.status.sample.contains("Normalizing 1 sample"));
}

#[test]
fn sample_selection_loads_selected_file_into_waveform() {
    let mut state = crate::native_app::test_support::NativeAppStateFixture::default()
        .with_synthetic_waveform()
        .with_sample_status("")
        .build();
    let sample_path = first_visible_asset_file_path(&state.library.folder_browser);
    let sample_name = PathBuf::from(&sample_path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .expect("sample file name");

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::SelectSampleWithModifiers {
            path: sample_path.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );
    assert_eq!(
        state.waveform.load.label.as_deref(),
        Some(sample_name.as_str())
    );
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_some(),
        "selection should debounce uncached sample loading before queueing decode work"
    );
    start_deferred_sample_load_for_tests(&mut state, sample_path.clone(), true, &mut context);
    let ticket = state
        .background
        .sample_load_task
        .active()
        .expect("sample load queued");
    state.apply_message(
        crate::native_app::test_support::GuiMessage::SampleLoadFinished(ui::TaskCompletion {
            ticket,
            output: crate::native_app::test_support::SampleLoadResult {
                path: sample_path.clone(),
                result: crate::native_app::test_support::WaveformState::load_path(
                    sample_path.clone().into(),
                ),
                autoplay: true,
            },
        }),
        &mut context,
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(sample_path.as_str())
    );
    assert_eq!(state.waveform.current.file_name(), sample_name);
    assert_eq!(state.waveform.load.label, None);
    assert!(state.waveform.current.frames() > 0);
    assert!(state.ui.status.sample.contains(&sample_name));
    assert!(
        state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&sample_path)
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::SelectSampleWithModifiers {
            path: sample_path.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );

    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none(),
        "repeat selection should use the memory waveform cache without a deferred worker"
    );
    assert!(
        state.background.sample_load_task.active().is_none(),
        "repeat selection must not start decode work"
    );
    assert_eq!(state.waveform.current.file_name(), sample_name);
}

#[test]
fn repeat_sample_selection_uses_memory_waveform_cache_without_worker() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("resident.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let loaded = crate::native_app::test_support::WaveformState::load_path(sample_path.clone())
        .expect("sample loads");
    state.remember_waveform(&loaded);
    state.waveform.current = crate::native_app::test_support::WaveformState::synthetic_for_tests();
    state.waveform.load.label = Some(String::from("previous.wav"));
    state.waveform.load.progress = 0.42;
    state.waveform.load.target_progress = 0.84;

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::SelectSampleWithModifiers {
            path: sample_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );

    assert_eq!(state.waveform.current.path(), sample_path);
    assert_eq!(state.waveform.load.label, None);
    assert_eq!(state.waveform.load.progress, 0.0);
    assert_eq!(state.waveform.load.target_progress, 0.0);
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none(),
        "memory-cached repeat selection should not debounce a reload"
    );
    assert!(
        state.background.sample_load_task.active().is_none(),
        "memory-cached repeat selection should not queue decode work"
    );
    assert!(
        state.ui.status.sample.contains("resident.wav"),
        "cached selection should update the visible status, got {}",
        state.ui.status.sample
    );
    assert!(
        state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&sample_path_string)
    );
}

#[test]
fn memory_cached_load_without_autoplay_stops_current_playback_state() {
    let source_root = tempfile::tempdir().expect("source root");
    let current_path = source_root.path().join("current.wav");
    let cached_path = source_root.path().join("cached.wav");
    write_test_wav_i16(&current_path, &[0, 256, -256, 512]);
    write_test_wav_i16(&cached_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let cached_path_string = cached_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);

    let cached = crate::native_app::test_support::WaveformState::load_path(cached_path.clone())
        .expect("sample loads");
    state.remember_waveform(&cached);

    state.waveform.current =
        crate::native_app::test_support::WaveformState::load_path(current_path)
            .expect("current sample loads");
    state.waveform.current.start_playback(0.25);
    state.audio.current_playback_span = Some((0.25, 1.0));

    let mut context = ui::UpdateContext::default();
    state.load_sample_without_autoplay(cached_path_string, &mut context);

    assert_eq!(state.waveform.current.path(), cached_path);
    assert!(!state.waveform.current.is_playing());
    assert_eq!(state.audio.current_playback_span, None);
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none(),
        "memory-cached non-autoplay load should not debounce a reload"
    );
    assert!(
        state.background.sample_load_task.active().is_none(),
        "memory-cached non-autoplay load should not queue decode work"
    );
    assert!(
        state.ui.status.sample.contains("Loaded cached.wav"),
        "cached non-autoplay load should update status, got {}",
        state.ui.status.sample
    );
}

#[test]
fn keyboard_and_mouse_uncached_selection_use_same_fast_debounce() {
    assert_eq!(
        crate::native_app::test_support::KEYBOARD_SAMPLE_LOAD_DEBOUNCE,
        crate::native_app::test_support::UNCACHED_SAMPLE_LOAD_DEBOUNCE,
        "keyboard navigation should not wait longer than mouse selection before audition loading"
    );
}

#[test]
fn uncached_selected_sample_load_uses_foreground_priority() {
    assert_eq!(
        crate::native_app::audio::sample_load_actions::foreground_sample_load_priority(),
        ui::TaskPriority::Interactive,
        "selected uncached audition loads must outrank background cache warming"
    );
}

#[test]
fn active_folder_cache_warm_uses_lower_priority_than_selected_sample_load() {
    assert_eq!(
        crate::native_app::audio::sample_load_actions::active_folder_cache_warm_priority(),
        ui::TaskPriority::Idle
    );
    assert_ne!(
        crate::native_app::audio::sample_load_actions::foreground_sample_load_priority(),
        crate::native_app::audio::sample_load_actions::active_folder_cache_warm_priority(),
        "background folder warming must not share the foreground audition lane"
    );
}

#[test]
fn frame_queues_audio_output_warm_up_before_explicit_playback() {
    let mut state = gui_state_for_span_tests();
    assert!(state.audio.player.is_none());
    assert!(state.background.audio_open_task.active().is_none());

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::Frame,
        &mut context,
    );

    assert!(
        state.background.audio_open_task.active().is_some(),
        "frame processing should begin audio output warm-up before the first explicit playback"
    );
}

#[test]
fn wav_load_reports_playback_ready_before_waveform_summary_completion() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("early-ready.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let playback_ready = std::cell::Cell::new(false);

    let waveform = crate::native_app::test_support::WaveformState::load_path_with_progress_cancel_and_playback_ready(
        sample_path.clone(),
        |progress| {
            if progress >= 0.62 {
                assert!(
                    playback_ready.get(),
                    "WAV playback samples should be available before waveform summary work"
                );
            }
        },
        || false,
        |ready| {
            assert_eq!(ready.path, sample_path);
            assert_eq!(ready.sample_rate, 48_000);
            assert_eq!(ready.channels, 1);
            assert!(!ready.playback_samples.is_empty());
            playback_ready.set(true);
        },
    )
    .expect("wav should load");

    assert!(playback_ready.get());
    assert!(waveform.playback_samples().is_some());
}

#[test]
fn playback_ready_message_starts_audio_before_full_waveform_finish() {
    let Ok(player) = wavecrate::audio::AudioPlayer::new() else {
        return;
    };
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("early-message.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.audio.player = Some(player);
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state
        .library
        .folder_browser
        .select_file(sample_path_string.clone());
    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::SelectSampleWithModifiers {
            path: sample_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );
    start_deferred_sample_load_for_tests(
        &mut state,
        sample_path_string.clone(),
        true,
        &mut context,
    );
    let ticket = state
        .background
        .sample_load_task
        .active()
        .expect("sample load queued");
    let samples = std::sync::Arc::from(vec![0.0_f32, 0.25, -0.25, 0.5]);

    state.apply_message(
        crate::native_app::test_support::GuiMessage::SamplePlaybackReady(ui::TaskCompletion {
            ticket,
            output: crate::native_app::test_support::SamplePlaybackReady {
                path: sample_path_string.clone(),
                audio: super::super::waveform::WaveformPlaybackReady {
                    path: sample_path.clone(),
                    audio_bytes: std::sync::Arc::from(fs::read(&sample_path).expect("read wav")),
                    playback_samples: samples,
                    sample_rate: 48_000,
                    channels: 1,
                    frames: 4,
                },
                autoplay: true,
            },
        }),
        &mut context,
    );

    assert_eq!(
        state.audio.early_sample_playback_path.as_deref(),
        Some(sample_path_string.as_str())
    );
    assert_eq!(state.audio.current_playback_span, Some((0.0, 1.0)));
    assert!(
        !state.waveform.current.is_playing(),
        "waveform visuals should wait for full waveform completion"
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::SampleLoadFinished(ui::TaskCompletion {
            ticket,
            output: crate::native_app::test_support::SampleLoadResult {
                path: sample_path_string.clone(),
                result: crate::native_app::test_support::WaveformState::load_path(
                    sample_path.clone(),
                ),
                autoplay: true,
            },
        }),
        &mut context,
    );

    assert_eq!(state.audio.early_sample_playback_path, None);
    assert!(state.waveform.current.is_playing());
    assert_eq!(state.audio.current_playback_span, Some((0.0, 1.0)));
}

#[test]
fn stale_playback_ready_message_is_ignored_after_selection_changes() {
    let source_root = tempfile::tempdir().expect("source root");
    let first_path = source_root.path().join("first.wav");
    let second_path = source_root.path().join("second.wav");
    write_test_wav_i16(&first_path, &[0, 1024, -2048, 4096]);
    write_test_wav_i16(&second_path, &[0, 512, -512, 1024]);
    let first_path_string = first_path.display().to_string();
    let second_path_string = second_path.display().to_string();

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state
        .library
        .folder_browser
        .select_file(first_path_string.clone());
    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::SelectSampleWithModifiers {
            path: first_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );
    start_deferred_sample_load_for_tests(&mut state, first_path_string.clone(), true, &mut context);
    let ticket = state
        .background
        .sample_load_task
        .active()
        .expect("sample load queued");
    state.library.folder_browser.select_file(second_path_string);

    state.apply_message(
        crate::native_app::test_support::GuiMessage::SamplePlaybackReady(ui::TaskCompletion {
            ticket,
            output: crate::native_app::test_support::SamplePlaybackReady {
                path: first_path_string.clone(),
                audio: super::super::waveform::WaveformPlaybackReady {
                    path: first_path,
                    audio_bytes: std::sync::Arc::from(Vec::<u8>::new()),
                    playback_samples: std::sync::Arc::from(vec![0.0_f32, 0.25, -0.25, 0.5]),
                    sample_rate: 48_000,
                    channels: 1,
                    frames: 4,
                },
                autoplay: true,
            },
        }),
        &mut context,
    );

    assert_eq!(state.audio.early_sample_playback_path, None);
    assert_eq!(state.audio.current_playback_span, None);
    assert!(
        !state.ui.status.sample.contains("Playing"),
        "stale playback-ready messages must not start old selection audio"
    );
}

#[test]
fn keyboard_navigation_defers_sample_loading_until_navigation_settles() {
    let source_root = tempfile::tempdir().expect("source root");
    for name in ["a.wav", "b.wav", "c.wav"] {
        fs::write(source_root.path().join(name), []).expect("sample file");
    }

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
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
        crate::native_app::test_support::GuiMessage::NavigateBrowser {
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
        crate::native_app::test_support::GuiMessage::NavigateBrowser {
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
        crate::native_app::test_support::GuiMessage::DeferredSampleLoad {
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
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.library.folder_browser.select_file(first);
    let loaded = crate::native_app::test_support::WaveformState::load_path(second_path.clone())
        .expect("sample loads");
    state.remember_waveform(&loaded);
    state.waveform.current = crate::native_app::test_support::WaveformState::synthetic_for_tests();

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::NavigateBrowser {
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

    let waveform = crate::native_app::test_support::WaveformState::load_path(second_path.clone())
        .expect("cache sample");
    let file = waveform.file();
    super::super::waveform::store_cached_waveform_file_for_tests(&file);
    wait_for_playback_ready_cache(&second);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.library.folder_browser.select_file(first);

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::NavigateBrowser {
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
        crate::native_app::test_support::GuiMessage::DeferredSampleLoad {
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
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let files = state.library.folder_browser.selected_audio_files();
    assert!(files.len() >= 2, "expected two visible samples");
    let first = files[0].id.clone();
    let second = files[1].id.clone();
    state.library.folder_browser.select_file(first);
    state.waveform.current =
        crate::native_app::test_support::WaveformState::load_path(PathBuf::from(&second))
            .expect("sample loads");

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::NavigateBrowser {
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

#[test]
fn file_rename_remaps_loaded_waveform_and_cache_without_reload() {
    let source_root = tempfile::tempdir().expect("source root");
    let old_path = source_root.path().join("loaded.wav");
    write_test_wav_i16(&old_path, &[0, 1024, -1024, 512]);
    let new_path = source_root.path().join("renamed.wav");

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state
        .library
        .folder_browser
        .select_file(old_path.display().to_string());
    state.waveform.current =
        crate::native_app::test_support::WaveformState::load_path(old_path.clone())
            .expect("sample loads");
    let loaded = state.waveform.current.clone();
    state.remember_waveform(&loaded);
    assert!(state.waveform.cache.entries.contains_key(&old_path));
    assert!(
        state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&old_path.display().to_string())
    );

    state
        .library
        .folder_browser
        .begin_rename_selected()
        .expect("rename can start")
        .expect("rename input");
    state.apply_folder_browser_rename_input(radiant::widgets::TextInputMessage::Submitted {
        value: String::from("renamed"),
    });

    assert_eq!(state.waveform.current.path(), new_path);
    assert!(state.waveform.current.has_loaded_sample());
    assert!(state.waveform.cache.entries.contains_key(&new_path));
    assert!(!state.waveform.cache.entries.contains_key(&old_path));
    assert!(
        state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&new_path.display().to_string())
    );
    assert!(
        !state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&old_path.display().to_string())
    );
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none()
    );
    assert!(state.background.sample_load_task.active().is_none());
    let new_id = new_path.display().to_string();
    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(new_id.as_str())
    );
}

#[test]
fn folder_rename_remaps_loaded_waveform_and_cache_without_reload() {
    let source_root = tempfile::tempdir().expect("source root");
    let old_folder = source_root.path().join("drums");
    fs::create_dir_all(&old_folder).expect("create source folder");
    let old_path = old_folder.join("loaded.wav");
    write_test_wav_i16(&old_path, &[0, 1024, -1024, 512]);
    let new_folder = source_root.path().join("breaks");
    let new_path = new_folder.join("loaded.wav");

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::FolderBrowser(
            crate::native_app::test_support::FolderBrowserMessage::ActivateFolder(
                old_folder.display().to_string(),
            ),
        ),
        &mut context,
    );
    state
        .library
        .folder_browser
        .select_file(old_path.display().to_string());
    state.waveform.current =
        crate::native_app::test_support::WaveformState::load_path(old_path.clone())
            .expect("sample loads");
    let loaded = state.waveform.current.clone();
    state.remember_waveform(&loaded);

    state.apply_message(
        crate::native_app::test_support::GuiMessage::FolderBrowser(
            crate::native_app::test_support::FolderBrowserMessage::ActivateFolder(
                old_folder.display().to_string(),
            ),
        ),
        &mut context,
    );
    state
        .library
        .folder_browser
        .begin_rename_selected()
        .expect("rename can start")
        .expect("rename input");
    state.apply_folder_browser_rename_input(radiant::widgets::TextInputMessage::Submitted {
        value: String::from("breaks"),
    });

    assert_eq!(state.waveform.current.path(), new_path);
    assert!(state.waveform.current.has_loaded_sample());
    assert!(state.waveform.cache.entries.contains_key(&new_path));
    assert!(!state.waveform.cache.entries.contains_key(&old_path));
    assert!(
        state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&new_path.display().to_string())
    );
    assert!(
        !state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&old_path.display().to_string())
    );
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none()
    );
    assert!(state.background.sample_load_task.active().is_none());
}

#[test]
fn sample_selection_starts_playback_ready_persisted_cache_load_after_restart() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("cached.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path = sample_path.display().to_string();
    let sample_name = PathBuf::from(&sample_path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .expect("sample file name");

    let waveform =
        crate::native_app::test_support::WaveformState::load_path(sample_path.clone().into())
            .expect("cache sample");
    let file = waveform.file();
    super::super::waveform::store_cached_waveform_file_for_tests(&file);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.refresh_persisted_waveform_cache_indicators();

    assert!(
        state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&sample_path),
        "persisted cache should mark the sample as ready before it enters memory cache"
    );

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::SelectSampleWithModifiers {
            path: sample_path.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );

    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none(),
        "playback-ready persisted cache should not wait for a debounce after restart"
    );
    assert!(
        state.background.sample_load_task.active().is_some(),
        "playback-ready persisted cache should start worker loading immediately"
    );
    assert!(
        state.waveform.load.label.as_deref() == Some(sample_name.as_str()),
        "selection should show loading state while the persisted cache is promoted"
    );
    assert!(
        !state
            .waveform
            .cache
            .entries
            .contains_key(&PathBuf::from(&sample_path)),
        "persisted cache promotion must stay off the UI thread until background loading completes"
    );
}

#[test]
fn playback_ready_persisted_cache_marks_row_without_memory_warm_after_restart() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("warm-before-click.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();
    let sample_path = PathBuf::from(&sample_path_string);

    let waveform = crate::native_app::test_support::WaveformState::load_path(sample_path.clone())
        .expect("cache sample");
    let file = waveform.file();
    super::super::waveform::store_cached_waveform_file_for_tests(&file);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.refresh_persisted_waveform_cache_indicators();

    assert!(
        state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&sample_path_string)
    );
    assert!(
        !state.waveform.cache.entries.contains_key(&sample_path),
        "restart indicator refresh should not synchronously deserialize cached waveforms"
    );
    assert!(
        state.waveform.cache.warm_pending.is_empty(),
        "playback-ready persisted caches should not be loaded into memory from UI refresh"
    );

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::SelectSampleWithModifiers {
            path: sample_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );

    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none(),
        "selection of a playback-ready cached file should not wait for debounce"
    );
    assert!(state.background.sample_load_task.active().is_some());
    assert_ne!(state.waveform.current.path(), sample_path);

    let ticket = state
        .background
        .sample_load_task
        .active()
        .expect("persisted cache load queued");
    state.apply_message(
        crate::native_app::test_support::GuiMessage::SampleLoadFinished(ui::TaskCompletion {
            ticket,
            output: crate::native_app::test_support::SampleLoadResult {
                path: sample_path_string,
                result:
                    crate::native_app::test_support::WaveformState::load_persisted_playback_cache(
                        sample_path.clone(),
                    ),
                autoplay: false,
            },
        }),
        &mut context,
    );

    assert_eq!(state.waveform.current.path(), sample_path);
    assert!(
        state.waveform.current.audio_bytes().is_empty(),
        "playback-ready persisted cache should not reread source WAV bytes"
    );
}

#[test]
fn folder_activation_schedules_cache_indicator_refresh_without_ui_thread_probe() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let folder = source_root.path().join("large-folder");
    fs::create_dir_all(&folder).expect("create folder");
    let sample_path = folder.join("cached.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();

    let waveform = crate::native_app::test_support::WaveformState::load_path(sample_path.clone())
        .expect("cache sample");
    let file = waveform.file();
    super::super::waveform::store_cached_waveform_file_for_tests(&file);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    assert!(
        !state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&sample_path_string)
    );

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::FolderBrowser(
            crate::native_app::test_support::FolderBrowserMessage::ActivateFolder(
                folder.display().to_string(),
            ),
        ),
        &mut context,
    );

    assert!(
        state
            .waveform
            .cache
            .indicator_refresh_task
            .active()
            .is_some(),
        "folder activation should queue cache indicator probing off the UI thread"
    );
    assert!(
        !state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&sample_path_string),
        "folder activation must not synchronously read persisted cache metadata"
    );
    assert!(
        state.waveform.cache.warm_pending.is_empty(),
        "summary cache warming should wait for the background indicator probe"
    );
}

#[test]
fn folder_activation_delays_active_folder_cache_warm() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let folder = source_root.path().join("large-folder");
    fs::create_dir_all(&folder).expect("create folder");
    let first = folder.join("first.wav");
    let second = folder.join("second.wav");
    write_test_wav_i16(&first, &[0, 1024, -2048, 4096, -1024, 512]);
    write_test_wav_i16(&second, &[0, 512, -512, 1024, -1024, 0]);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let mut context = ui::UpdateContext::default();

    state.apply_message(
        crate::native_app::test_support::GuiMessage::FolderBrowser(
            crate::native_app::test_support::FolderBrowserMessage::ActivateFolder(
                folder.display().to_string(),
            ),
        ),
        &mut context,
    );

    assert!(
        state
            .waveform
            .cache
            .active_folder_warm_delay_task
            .active()
            .is_some(),
        "folder activation should wait briefly before assuming browse intent"
    );
    assert!(
        state
            .waveform
            .cache
            .active_folder_warm_task
            .active()
            .is_none(),
        "active folder cache warm must not start during folder activation"
    );
    assert_eq!(state.waveform.cache.active_folder_warm_pending.len(), 2);
}

#[test]
fn changing_folder_cancels_previous_active_folder_cache_warm() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let first_folder = source_root.path().join("first-folder");
    let second_folder = source_root.path().join("second-folder");
    fs::create_dir_all(&first_folder).expect("create first folder");
    fs::create_dir_all(&second_folder).expect("create second folder");
    write_test_wav_i16(&first_folder.join("first.wav"), &[0, 1024, -2048, 4096]);
    write_test_wav_i16(&second_folder.join("second.wav"), &[0, 512, -512, 1024]);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::FolderBrowser(
            crate::native_app::test_support::FolderBrowserMessage::ActivateFolder(
                first_folder.display().to_string(),
            ),
        ),
        &mut context,
    );
    let first_ticket = state
        .waveform
        .cache
        .active_folder_warm_delay_task
        .active()
        .expect("first folder warm delay");
    state.apply_message(
        crate::native_app::test_support::GuiMessage::ActiveFolderCacheWarmReady(first_ticket),
        &mut context,
    );
    assert!(
        state
            .waveform
            .cache
            .active_folder_warm_task
            .active()
            .is_some()
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::FolderBrowser(
            crate::native_app::test_support::FolderBrowserMessage::ActivateFolder(
                second_folder.display().to_string(),
            ),
        ),
        &mut context,
    );

    assert!(
        state
            .waveform
            .cache
            .active_folder_warm_task
            .active()
            .is_none(),
        "changing folders should cancel the active warm worker"
    );
    let second_folder_id = second_folder.display().to_string();
    assert_eq!(
        state.waveform.cache.active_folder_warm_folder_id.as_deref(),
        Some(second_folder_id.as_str())
    );
    assert_eq!(state.waveform.cache.active_folder_warm_pending.len(), 1);
}

#[test]
fn active_folder_cache_warm_generates_playback_ready_cache_for_uncached_file() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("uncached-warm.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path = PathBuf::from(sample_path.display().to_string());

    assert!(!super::super::waveform::cached_waveform_file_playback_ready_exists(&sample_path));

    let token = ui::CancellationToken::new();
    let loaded = crate::native_app::audio::sample_load_actions::warm_active_folder_waveform_cache(
        vec![sample_path.clone()],
        &token,
    );
    super::super::waveform::flush_background_waveform_cache_stores_for_shutdown();

    assert_eq!(loaded.len(), 1);
    assert!(
        super::super::waveform::cached_waveform_file_playback_ready_exists(&sample_path),
        "active folder warm should persist playback readiness for future selection"
    );
}

#[test]
fn summary_only_persisted_cache_is_not_marked_playback_ready_after_restart() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("summary-only.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();
    let sample_path = PathBuf::from(&sample_path_string);

    let file = super::super::waveform::test_waveform_file_from_mono_samples(
        sample_path.clone(),
        fs::read(&sample_path).expect("read wav").into(),
        vec![0.0, 0.25, -0.25, 0.5, -0.5, 0.125],
    );
    super::super::waveform::store_summary_only_cached_waveform_file_for_tests(&file);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.refresh_persisted_waveform_cache_indicators();

    assert!(
        !state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&sample_path_string),
        "summary-only persisted cache must not paint the row as playback-ready"
    );
    assert_eq!(
        state.waveform.cache.warm_pending.iter().collect::<Vec<_>>(),
        vec![&sample_path],
        "summary-only persisted cache should still be warmed in the background"
    );
}

#[test]
fn summary_only_persisted_cache_selection_uses_loading_pipeline_after_restart() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("summary-only-click.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();
    let sample_path = PathBuf::from(&sample_path_string);

    let file = super::super::waveform::test_waveform_file_from_mono_samples(
        sample_path.clone(),
        fs::read(&sample_path).expect("read wav").into(),
        vec![0.0, 0.25, -0.25, 0.5, -0.5, 0.125],
    );
    super::super::waveform::store_summary_only_cached_waveform_file_for_tests(&file);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state.refresh_persisted_waveform_cache_indicators();

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::SelectSampleWithModifiers {
            path: sample_path_string.clone(),
            modifiers: Default::default(),
        },
        &mut context,
    );

    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_some(),
        "summary-only cache selection should not synchronously decode long playback samples"
    );
    assert_eq!(
        state.waveform.current.path(),
        PathBuf::from("synthetic-waveform"),
        "selection should wait for the normal loading pipeline instead of hydrating a partial cache"
    );
}

#[test]
fn background_warm_upgrades_summary_only_cache_to_playback_ready() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("summary-only-warm.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path_string = sample_path.display().to_string();
    let sample_path = PathBuf::from(&sample_path_string);

    let file = super::super::waveform::test_waveform_file_from_mono_samples(
        sample_path.clone(),
        fs::read(&sample_path).expect("read wav").into(),
        vec![0.0, 0.25, -0.25, 0.5, -0.5, 0.125],
    );
    super::super::waveform::store_summary_only_cached_waveform_file_for_tests(&file);

    let result =
        crate::native_app::audio::sample_load_actions::warm_persisted_waveform_cache(vec![
            sample_path.clone(),
        ]);
    assert_eq!(result.loaded.len(), 1);

    let mut restarted_state = gui_state_for_span_tests();
    restarted_state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    restarted_state.refresh_persisted_waveform_cache_indicators();

    assert!(
        restarted_state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&sample_path_string),
        "background warm should persist playback readiness for future restarts"
    );
}

#[test]
fn normal_sample_load_persists_bright_cache_indicator_before_restart() {
    let config_base = tempfile::tempdir().expect("config base");
    let (_config_lock, _base_guard) =
        set_waveform_test_config_base(config_base.path().to_path_buf());
    let source_root = tempfile::tempdir().expect("source root");
    let sample_path = source_root.path().join("fresh-cache.wav");
    write_test_wav_i16(&sample_path, &[0, 1024, -2048, 4096, -1024, 512]);
    let sample_path = sample_path.display().to_string();

    let _waveform =
        crate::native_app::test_support::WaveformState::load_path(sample_path.clone().into())
            .expect("load sample");

    wait_for_playback_ready_cache(&sample_path);

    let mut restarted_state = gui_state_for_span_tests();
    restarted_state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    restarted_state.refresh_persisted_waveform_cache_indicators();

    assert!(
        restarted_state
            .waveform
            .cache
            .cached_sample_paths
            .contains(&sample_path),
        "freshly loaded cache indicator should survive immediate restart"
    );
}

fn wait_for_playback_ready_cache(sample_path: &str) {
    let path = PathBuf::from(sample_path);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        if super::super::waveform::cached_waveform_file_playback_ready_exists(&path) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
fn selecting_another_sample_cancels_metadata_tag_entry() {
    let source_root = tempfile::tempdir().expect("source root");
    let first_path = source_root.path().join("first.wav");
    let second_path = source_root.path().join("second.wav");
    fs::write(&first_path, []).expect("first sample");
    fs::write(&second_path, []).expect("second sample");

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let first_file = first_path.display().to_string();
    let second_file = second_path.display().to_string();
    state.library.folder_browser.select_file(first_file.clone());
    state.metadata.tag_draft = String::from("ki");
    state.metadata.tag_tokens = vec![String::from("warm")];
    state.metadata.tag_input_mode =
        crate::native_app::test_support::MetadataTagInputMode::Category {
            pending_tag: String::from("new-tag"),
        };
    state.metadata.tag_completion_cycle.select("ki", 2, 4);

    state.select_sample_with_modifiers(
        second_file.clone(),
        PointerModifiers::default(),
        &mut ui::UpdateContext::default(),
    );

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(second_file.as_str())
    );
    assert!(state.metadata.tag_draft.is_empty());
    assert!(state.metadata.tag_tokens.is_empty());
    assert_eq!(
        state.metadata.tag_input_mode,
        crate::native_app::test_support::MetadataTagInputMode::Tag
    );
    assert_eq!(state.metadata.tag_completion_cycle.query_key(), None);
    assert_eq!(state.metadata.tag_completion_cycle.stored_index(), 0);
    assert_eq!(state.pending_metadata_tag_category_tag(), None);
    assert!(!state.metadata_tag_completion_active());
}

#[test]
fn play_selected_sample_uses_active_playmark_selection_span() {
    let Ok(player) = wavecrate::audio::AudioPlayer::new() else {
        return;
    };
    let mut state = NativeAppState::load_default().expect("default state loads");
    state.audio.player = Some(player);
    let sample_path = first_visible_asset_file_path(&state.library.folder_browser);
    state.waveform.current =
        crate::native_app::test_support::WaveformState::load_path(sample_path.into())
            .expect("test sample loads");
    state
        .waveform
        .current
        .apply_interaction(WaveformInteraction::BeginSelection {
            kind: WaveformSelectionKind::Play,
            visible_ratio: 0.25,
        });
    state
        .waveform
        .current
        .apply_interaction(WaveformInteraction::UpdateSelection {
            visible_ratio: 0.60,
        });
    state
        .waveform
        .current
        .apply_interaction(WaveformInteraction::FinishSelection {
            visible_ratio: 0.60,
        });
    state.audio.loop_playback = true;

    let mut context = ui::UpdateContext::default();
    state.play_selected_sample(&mut context);

    assert!(state.waveform.current.is_playing());
    assert_eq!(state.waveform.current.play_mark_ratio(), Some(0.25));
    assert_eq!(state.audio.current_playback_span, Some((0.25, 0.6)));
    assert!(
        state
            .audio
            .player
            .as_ref()
            .is_some_and(|player| player.is_looping())
    );
    let progress = state
        .audio
        .player
        .as_ref()
        .and_then(|player| player.progress())
        .expect("playback progress");
    assert!(
        (0.24..=0.35).contains(&progress),
        "spacebar playback should start inside the playmark selection, got {progress}"
    );
}

#[test]
fn looped_playback_retargets_when_playmark_selection_is_created_and_resized() {
    let Ok(player) = wavecrate::audio::AudioPlayer::new() else {
        return;
    };
    let mut state = gui_state_for_span_tests();
    state.audio.player = Some(player);
    let sample_path = first_visible_asset_file_path(&state.library.folder_browser);
    state.waveform.current =
        crate::native_app::test_support::WaveformState::load_path(sample_path.into())
            .expect("test sample loads");
    state.audio.loop_playback = true;
    state
        .start_playback_current_span(0.0, 1.0)
        .expect("full sample loop starts");
    assert_player_progress_inside_span(&state, 0.0, 1.0);

    let mut context = ui::UpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::GuiMessage::Waveform(
            WaveformInteraction::BeginSelection {
                kind: WaveformSelectionKind::Play,
                visible_ratio: 0.25,
            },
        ),
        &mut context,
    );
    state.apply_message(
        crate::native_app::test_support::GuiMessage::Waveform(
            WaveformInteraction::UpdateSelection {
                visible_ratio: 0.60,
            },
        ),
        &mut context,
    );
    state.apply_message(
        crate::native_app::test_support::GuiMessage::Waveform(
            WaveformInteraction::FinishSelection {
                visible_ratio: 0.60,
            },
        ),
        &mut context,
    );

    assert_playback_span_state(&state, 0.25, 0.60);
    assert_player_progress_inside_span(&state, 0.25, 0.60);
    assert!(
        state
            .audio
            .player
            .as_ref()
            .is_some_and(|player| player.is_looping())
    );

    state.apply_message(
        crate::native_app::test_support::GuiMessage::Waveform(
            WaveformInteraction::BeginSelectionResize {
                kind: WaveformSelectionKind::Play,
                edge: WaveformSelectionEdge::Start,
                visible_ratio: 0.25,
            },
        ),
        &mut context,
    );
    state.apply_message(
        crate::native_app::test_support::GuiMessage::Waveform(
            WaveformInteraction::UpdateSelection {
                visible_ratio: 0.10,
            },
        ),
        &mut context,
    );
    state.apply_message(
        crate::native_app::test_support::GuiMessage::Waveform(
            WaveformInteraction::FinishSelection {
                visible_ratio: 0.10,
            },
        ),
        &mut context,
    );

    assert_playback_span_state(&state, 0.10, 0.60);
    assert_player_progress_inside_span(&state, 0.10, 0.60);
}

fn assert_playback_span_state(state: &NativeAppState, expected_start: f32, expected_end: f32) {
    let (start, end) = state
        .audio
        .current_playback_span
        .expect("current playback span should be set");
    assert!(
        (start - expected_start).abs() < 0.001,
        "start {start}, expected {expected_start}"
    );
    assert!(
        (end - expected_end).abs() < 0.001,
        "end {end}, expected {expected_end}"
    );
}

fn assert_player_progress_inside_span(state: &NativeAppState, start: f32, end: f32) {
    let progress = state
        .audio
        .player
        .as_ref()
        .and_then(|player| player.progress())
        .expect("audio player progress should be available");
    assert!(
        progress >= start - 0.02 && progress <= end + 0.02,
        "progress {progress}, expected inside {start}..={end}"
    );
}
