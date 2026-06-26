use super::*;

#[test]
fn random_audition_span_can_start_near_end_of_long_sample() {
    let (start, end) = crate::native_app::audio::playback::random_audition_span_for_units(
        20.0,
        crate::native_app::audio::playback::RandomAuditionUnits::new(1.0, 0.0),
    );

    assert!((start - 0.9875).abs() < 0.001, "start was {start}");
    assert!((end - 1.0).abs() < 0.001, "end was {end}");
}

#[test]
fn random_audition_span_uses_random_length_inside_sample() {
    let (start, end) = crate::native_app::audio::playback::random_audition_span_for_units(
        10.0,
        crate::native_app::audio::playback::RandomAuditionUnits::new(0.25, 0.5),
    );

    assert!((start - 0.24375).abs() < 0.001, "start was {start}");
    assert!((end - 0.634375).abs() < 0.001, "end was {end}");
}

#[test]
fn random_audition_span_plays_whole_tiny_or_invalid_sample() {
    assert_eq!(
        crate::native_app::audio::playback::random_audition_span_for_units(
            0.1,
            crate::native_app::audio::playback::RandomAuditionUnits::new(0.75, 0.5),
        ),
        (0.0, 1.0)
    );
    assert_eq!(
        crate::native_app::audio::playback::random_audition_span_for_units(
            f32::NAN,
            crate::native_app::audio::playback::RandomAuditionUnits::new(0.75, 0.5),
        ),
        (0.0, 1.0)
    );
}

#[test]
fn random_audition_pans_selected_region_into_view_when_it_fits() {
    let mut state = gui_state_for_span_tests();
    state.waveform.current.set_play_selection_range(0.0, 0.25);
    state.waveform.current.zoom_to_play_selection();

    let viewport = state.waveform.current.viewport();
    let original_width = viewport.end - viewport.start;
    let mut context = ui::UiUpdateContext::default();
    state.play_random_sample_range_with_units(
        crate::native_app::audio::playback::RandomAuditionUnits::new(1.0, 0.0),
        &mut context,
    );

    let viewport = state.waveform.current.viewport();
    assert_eq!(viewport.end - viewport.start, original_width);
    assert_eq!(viewport.start, 36_000);
    assert_eq!(viewport.end, 48_000);
    assert!(
        state
            .waveform
            .current
            .visible_ratio_for_absolute(0.75)
            .is_some()
    );
    assert!(
        state
            .waveform
            .current
            .visible_ratio_for_absolute(1.0)
            .is_some()
    );
}

#[test]
fn random_audition_zooms_selected_region_only_when_current_view_would_clip_it() {
    let mut state = gui_state_for_span_tests();
    state.waveform.current.set_play_selection_range(0.0, 0.25);
    state.waveform.current.zoom_to_play_selection();
    let viewport = state.waveform.current.viewport();
    let original_width = viewport.end - viewport.start;

    let mut context = ui::UiUpdateContext::default();
    state.play_random_sample_range_with_units(
        crate::native_app::audio::playback::RandomAuditionUnits::new(0.25, 0.5),
        &mut context,
    );

    let viewport = state.waveform.current.viewport();
    assert!(viewport.end - viewport.start > original_width);
    assert_eq!(viewport.start, 9_000);
    assert_eq!(viewport.end, 34_500);
    assert!(
        state
            .waveform
            .current
            .visible_ratio_for_absolute(0.1875)
            .is_some()
    );
    assert!(
        state
            .waveform
            .current
            .visible_ratio_for_absolute(0.71875)
            .is_some()
    );
}

#[test]
fn random_audition_ignores_marked_play_ranges_and_samples_entire_waveform() {
    let mut scenario = WaveformPlaybackScenario::synthetic();

    for (start, end) in [(0.10, 0.20), (0.55, 0.70)] {
        scenario.select_play_range(start, end);
    }

    let original_selection = scenario.state.waveform.current.play_selection();
    let span = scenario.state.random_audition_span_for_loaded_waveform(
        crate::native_app::audio::playback::RandomAuditionUnits::new(1.0, 0.0),
    );

    assert_eq!(
        span.source,
        crate::native_app::audio::playback::RandomAuditionSource::WholeSample
    );
    assert!(
        (span.start - 0.75).abs() < 0.001,
        "start was {}",
        span.start
    );
    assert!((span.end - 1.0).abs() < 0.001, "end was {}", span.end);
    assert_eq!(
        scenario.state.waveform.current.play_selection(),
        original_selection
    );
}

#[test]
fn random_audition_loops_random_region_when_loop_is_enabled() {
    let mut state = gui_state_for_span_tests();
    if !install_playback_runtime_for_tests(&mut state) {
        return;
    }
    state.audio.loop_playback = true;

    let mut context = ui::UiUpdateContext::default();
    state.play_random_sample_range_with_units(
        crate::native_app::audio::playback::RandomAuditionUnits::new(0.25, 0.5),
        &mut context,
    );

    assert!(state.audio.loop_playback);
    assert!(state.waveform.current.is_playing());
    assert_eq!(state.audio.current_playback_span, Some((0.1875, 0.71875)));
    assert_eq!(
        state.waveform.current.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.1875, 0.71875))
    );
    assert_eq!(state.waveform.current.play_mark_ratio(), Some(0.1875));
}

#[test]
fn play_from_current_play_start_uses_existing_play_marker() {
    let mut state = gui_state_for_span_tests();
    if !install_playback_runtime_for_tests(&mut state) {
        return;
    }
    state.waveform.current.start_playback(0.37);

    let mut context = ui::UiUpdateContext::default();
    state.play_from_current_play_start(&mut context);

    assert!(state.waveform.current.is_playing());
    assert_eq!(state.audio.current_playback_span, Some((0.37, 1.0)));
    assert_eq!(state.waveform.current.play_mark_ratio(), Some(0.37));
}

#[test]
fn random_audition_loops_random_region_for_loop_tagged_sample() {
    let samples = vec![0_i16; 48_000];
    let Some(mut scenario) =
        WaveformPlaybackScenario::loaded_with_player("loop-tagged-random.wav", &samples)
    else {
        return;
    };
    let file_id = scenario.state.waveform.current.path().display().to_string();
    scenario
        .state
        .metadata
        .tags_by_file
        .insert(file_id, vec![String::from("loop")]);
    scenario.state.audio.loop_playback = true;

    scenario.play_random_range_with_units(0.25, 0.5);

    assert!(scenario.state.audio.loop_playback);
    assert!(scenario.state.waveform.current.is_playing());
    assert_eq!(
        scenario.state.audio.current_playback_span,
        Some((0.1875, 0.71875))
    );
    assert_eq!(
        scenario.state.waveform.current.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.1875, 0.71875))
    );
}

#[test]
fn random_audition_for_unloaded_selection_resumes_after_sample_load() {
    let mut scenario = WaveformPlaybackScenario::with_temp_wav(
        "random-load.wav",
        &[0, 1024, -2048, 4096, -1024, 512],
    )
    .with_unloaded_waveform()
    .with_looping();
    assert!(!scenario.state.waveform.current.has_loaded_sample());

    scenario.play_random_range_with_units(0.5, 0.25);

    assert!(matches!(
        scenario.state.audio.pending_sample_playback,
        Some(
            crate::native_app::test_support::state::PendingSamplePlayback::RandomAudition {
                start_unit,
                length_unit,
            }
        )
            if (start_unit - 0.5).abs() < f32::EPSILON
                && (length_unit - 0.25).abs() < f32::EPSILON
    ));

    scenario.start_deferred_load(false);
    scenario.finish_deferred_load(false);

    assert_eq!(scenario.state.audio.pending_sample_playback, None);
    assert_eq!(
        scenario.state.waveform.current.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.0, 1.0))
    );
    assert_eq!(scenario.state.waveform.current.play_mark_ratio(), Some(0.0));
    assert!(
        scenario.state.audio.pending_playback_start.is_some(),
        "random audition should request playback even when the audio device is still opening"
    );
    assert!(
        scenario.state.audio.loop_playback,
        "random audition should preserve loop mode after the selected sample loads"
    );
    assert!(
        scenario
            .state
            .ui
            .status
            .sample
            .contains("Playback unavailable")
            || scenario.state.ui.status.sample.contains("Random audition"),
        "random load completion should route through random playback, got {}",
        scenario.state.ui.status.sample
    );
}

#[test]
fn random_listed_audition_resolves_region_from_loaded_target_duration() {
    let root = temp_gui_root("wavecrate-random-listed-target-duration");
    let current = root.join("a-current.wav");
    let target_folder = root.join("drums");
    fs::create_dir_all(&target_folder).expect("target folder");
    let target = target_folder.join("z-target.wav");
    let current_id = current.display().to_string();
    let root_id = root.display().to_string();
    let target_id = target.display().to_string();
    write_test_wav_i16(&current, &vec![0_i16; 48_000]);
    write_test_wav_i16(&target, &vec![0_i16; 96_000]);

    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(root.clone()),
        ]);
    state.library.folder_browser.toggle_folder_subtree_listing();
    state.waveform.current =
        crate::native_app::test_support::state::WaveformState::load_path(current.clone())
            .expect("current sample loads");
    state.library.folder_browser.select_file(current_id.clone());

    let mut context = ui::UiUpdateContext::default();
    state.play_random_listed_sample_range_with_units(
        1.0,
        crate::native_app::audio::playback::RandomAuditionUnits::new(0.0, 0.0),
        &mut context,
    );
    run_command_for_tests(&mut state, context.into_command());

    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(target_id.as_str())
    );
    assert_eq!(
        state.library.folder_browser.selected_folder_id(),
        Some(root_id.as_str()),
        "root should stay selected because include-subfolders already keeps the target visible"
    );
    let ticket = active_sample_load_ticket(&state).expect("target sample load queued");
    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SampleLoadFinished(
            sample_load_completion(
                ticket,
                target_id.clone(),
                crate::native_app::test_support::state::WaveformState::load_path(target.clone()),
                false,
            ),
        ),
        &mut context,
    );

    assert_eq!(state.waveform.current.path(), target.as_path());
    assert_eq!(
        state.waveform.current.play_selection(),
        Some(wavecrate::selection::SelectionRange::new(0.0, 0.125))
    );
    assert!(
        state.background.audio_open.active().is_some(),
        "pending random audition should begin opening audio before submitting playback"
    );
    assert!(
        state.audio.pending_playback_start.is_some(),
        "pending random audition should queue playback while audio output is opening"
    );

    let _ = fs::remove_dir_all(root);
}
