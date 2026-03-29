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

#[test]
fn active_slice_review_requires_marks_before_export() {
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
    ];
    controller.start_slice_review();

    controller.save_waveform_selection_or_slices_to_browser_action(true);

    assert_eq!(controller.ui.status.text, "Mark slices to export first");
    assert!(
        controller
            .runtime
            .jobs
            .pending_slice_batch_export()
            .is_none()
    );
}

#[test]
fn slice_review_exports_only_marked_slices() {
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
    controller.start_slice_review();
    controller.move_slice_review_focus(1);
    controller.toggle_focused_slice_export_mark().unwrap();
    controller.move_slice_review_focus(1);
    controller.toggle_focused_slice_export_mark().unwrap();

    controller
        .save_waveform_selection_or_slices_to_browser(true)
        .expect("marked slice batch should queue");

    pump_background_jobs_until(&mut controller, |controller| {
        controller
            .runtime
            .jobs
            .pending_slice_batch_export()
            .is_none()
            && source_root.join("clip_silence_split_002.wav").is_file()
    });

    assert_eq!(controller.ui.status.text, "Saved 2 slices");
    assert!(source_root.join("clip_silence_split_001.wav").is_file());
    assert!(source_root.join("clip_silence_split_002.wav").is_file());
    assert!(!source_root.join("clip_silence_split_003.wav").exists());
}

#[test]
fn save_waveform_slices_to_browser_with_keep2_persists_keep2_on_each_entry() {
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
    ];
    controller.start_slice_review();
    controller.toggle_focused_slice_export_mark().unwrap();
    controller.move_slice_review_focus(1);
    controller.toggle_focused_slice_export_mark().unwrap();

    controller
        .save_waveform_selection_or_slices_to_browser_action_with_tag(true, Some(Rating::new(2)));

    pump_background_jobs_until(&mut controller, |controller| {
        controller
            .runtime
            .jobs
            .pending_slice_batch_export()
            .is_none()
            && source_root.join("clip_slice002.wav").is_file()
    });

    let rows = controller
        .database_for(&source)
        .unwrap()
        .list_files()
        .unwrap();
    let exported: Vec<_> = rows
        .iter()
        .filter(|row| {
            row.relative_path == PathBuf::from("clip_slice001.wav")
                || row.relative_path == PathBuf::from("clip_slice002.wav")
        })
        .collect();
    assert_eq!(exported.len(), 2);
    assert!(exported.iter().all(|row| row.tag == Rating::new(2)));
}
