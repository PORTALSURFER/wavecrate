use super::*;

#[test]
fn save_waveform_slices_to_browser_runs_in_background_and_clears_on_success() {
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source");
    std::fs::create_dir_all(&source_root).unwrap();

    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(source_root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    let wav_path = source_root.join("clip.wav");
    write_test_wav(&wav_path, &[0.1, 0.2, 0.3, 0.4, 0.5, 0.6]);
    controller
        .load_waveform_for_selection(&source, Path::new("clip.wav"))
        .unwrap();
    controller.ui.waveform.slices = vec![
        SelectionRange::new(0.0, 0.34),
        SelectionRange::new(0.34, 0.67),
        SelectionRange::new(0.67, 1.0),
    ];
    controller.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::SilenceSplit;

    controller
        .save_waveform_selection_or_slices_to_browser(true)
        .expect("slice batch should queue");

    assert_eq!(controller.ui.status.status_tone, StatusTone::Busy);
    assert_eq!(
        controller.ui.progress.task,
        Some(ProgressTaskKind::SelectionExport)
    );
    assert!(controller.ui.progress.visible);
    assert!(!controller.ui.progress.modal);
    assert!(
        controller
            .runtime
            .jobs
            .pending_slice_batch_export()
            .is_some(),
        "slice batch should be tracked while in flight"
    );

    pump_background_jobs_until(&mut controller, |controller| {
        controller
            .runtime
            .jobs
            .pending_slice_batch_export()
            .is_none()
            && source_root.join("clip_silence_split_003.wav").is_file()
    });

    assert_eq!(controller.ui.status.text, "Saved 3 slices");
    assert_eq!(controller.ui.status.status_tone, StatusTone::Info);
    assert!(!controller.ui.progress.visible);
    assert!(controller.ui.waveform.slices.is_empty());
    assert!(controller.ui.waveform.selected_slices.is_empty());
    assert_eq!(
        controller.ui.waveform.slice_batch_profile,
        WaveformSliceBatchProfile::Manual
    );
}

#[test]
fn save_waveform_slices_to_browser_ignores_duplicate_submit_while_running() {
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source");
    std::fs::create_dir_all(&source_root).unwrap();

    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(source_root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    let wav_path = source_root.join("clip.wav");
    write_test_wav(&wav_path, &[0.1, 0.2, 0.3, 0.4]);
    controller
        .load_waveform_for_selection(&source, Path::new("clip.wav"))
        .unwrap();
    controller.ui.waveform.slices = vec![SelectionRange::new(0.0, 0.5)];

    controller
        .save_waveform_selection_or_slices_to_browser(true)
        .expect("first slice batch should queue");
    controller
        .save_waveform_selection_or_slices_to_browser(true)
        .expect("duplicate submit should be ignored");

    assert_eq!(
        controller.ui.status.text,
        "Slice export already in progress"
    );
    assert_eq!(controller.ui.status.status_tone, StatusTone::Info);

    pump_background_jobs_until(&mut controller, |controller| {
        controller
            .runtime
            .jobs
            .pending_slice_batch_export()
            .is_none()
    });
}
