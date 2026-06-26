use super::{
    gui_state_for_span_tests, run_command_for_tests, start_deferred_sample_load_for_tests,
    write_test_wav_i16,
};
use radiant::prelude as ui;
use std::fs;

#[test]
fn native_file_open_loads_audio_file_from_configured_source_without_autoplay() {
    let source_root = tempfile::tempdir().expect("source root");
    let sample = source_root.path().join("open.wav");
    write_test_wav_i16(&sample, &[0, 100, -100]);
    let mut state = gui_state_for_span_tests();
    state.library.folder_browser =
        crate::native_app::test_support::state::FolderBrowserState::from_sample_sources(&[
            wavecrate::sample_sources::SampleSource::new(source_root.path().to_path_buf()),
        ]);
    let mut context = ui::UiUpdateContext::default();

    state.open_audio_documents(vec![sample.clone()], &mut context);
    run_command_for_tests(&mut state, context.into_command());
    let mut context = ui::UiUpdateContext::default();

    let sample_id = sample.display().to_string();
    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(sample_id.as_str())
    );
    assert_eq!(state.waveform.load.label.as_deref(), Some("open.wav"));
    start_deferred_sample_load_for_tests(&mut state, sample_id, false, &mut context);
    assert!(state.active_sample_load_task().is_some());
}

#[test]
fn native_file_open_adds_parent_source_before_loading_external_audio_file() {
    let external_root = tempfile::tempdir().expect("external root");
    let sample = external_root.path().join("external.wav");
    write_test_wav_i16(&sample, &[0, 100, -100]);
    let mut state = gui_state_for_span_tests();
    let mut context = ui::UiUpdateContext::default();

    state.open_audio_documents(vec![sample.clone()], &mut context);
    run_command_for_tests(&mut state, context.into_command());
    let mut context = ui::UiUpdateContext::default();

    assert_eq!(
        state.library.pending_audio_document_open_count_for_tests(),
        1
    );
    let progress = state
        .library
        .folder_progress()
        .expect("external document open should scan parent source")
        .clone();
    let result = crate::native_app::sample_library::folder_browser::scan::scan_source_with_progress(
        crate::native_app::sample_library::folder_browser::scan::FolderScanRequest {
            task_id: progress.task_id,
            source_id: progress.source_id,
            label: progress.label,
            root: external_root.path().to_path_buf(),
            database_root: external_root.path().to_path_buf(),
        },
        |_| {},
        |_| {},
    );

    state.finish_folder_scan(result, &mut context);

    let sample_id = sample.display().to_string();
    assert_eq!(
        state.library.pending_audio_document_open_count_for_tests(),
        0
    );
    assert_eq!(
        state.library.folder_browser.selected_file_id(),
        Some(sample_id.as_str())
    );
    assert_eq!(state.waveform.load.label.as_deref(), Some("external.wav"));
    start_deferred_sample_load_for_tests(&mut state, sample_id, false, &mut context);
    assert!(state.active_sample_load_task().is_some());
}

#[test]
fn native_file_open_rejects_unsupported_documents() {
    let external_root = tempfile::tempdir().expect("external root");
    let note = external_root.path().join("note.txt");
    fs::write(&note, "not audio").expect("write note");
    let mut state = gui_state_for_span_tests();
    let mut context = ui::UiUpdateContext::default();

    state.open_audio_documents(vec![note], &mut context);
    run_command_for_tests(&mut state, context.into_command());

    assert_eq!(
        state.library.pending_audio_document_open_count_for_tests(),
        0
    );
    assert!(
        state
            .ui
            .status
            .sample
            .contains("Unsupported audio document"),
        "unsupported file open should tell the user why it was ignored"
    );
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none()
    );
}

#[test]
fn native_file_open_rejects_missing_audio_documents_after_validation() {
    let external_root = tempfile::tempdir().expect("external root");
    let missing = external_root.path().join("missing.wav");
    let mut state = gui_state_for_span_tests();
    let mut context = ui::UiUpdateContext::default();

    state.open_audio_documents(vec![missing], &mut context);
    run_command_for_tests(&mut state, context.into_command());

    assert_eq!(
        state.library.pending_audio_document_open_count_for_tests(),
        0
    );
    assert!(
        state.ui.status.sample.contains("is not a file"),
        "missing file open should be rejected by validation worker"
    );
    assert!(
        state
            .background
            .deferred_sample_load_task
            .active()
            .is_none()
    );
}
