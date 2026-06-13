use super::*;

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
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    state
        .library
        .folder_browser
        .select_file(first_path_string.clone());
    let mut context = ui::UiUpdateContext::default();
    state.apply_message(
        crate::native_app::test_support::state::GuiMessage::SelectSampleWithModifiers {
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
        crate::native_app::test_support::state::GuiMessage::SamplePlaybackReady(
            ui::TaskCompletion {
                ticket,
                output: crate::native_app::test_support::state::SamplePlaybackReady {
                    path: first_path_string.clone(),
                    audio: crate::native_app::waveform::WaveformPlaybackReady {
                        path: first_path,
                        audio_bytes: std::sync::Arc::from(Vec::<u8>::new()),
                        playback_samples: std::sync::Arc::from(vec![0.0_f32, 0.25, -0.25, 0.5]),
                        sample_rate: 48_000,
                        channels: 1,
                        frames: 4,
                    },
                    autoplay: true,
                },
            },
        ),
        &mut context,
    );

    assert_eq!(state.audio.early_sample_playback_path, None);
    assert_eq!(state.audio.current_playback_span, None);
    assert!(
        !state.ui.status.sample.contains("Playing"),
        "stale playback-ready messages must not start old selection audio"
    );
}
