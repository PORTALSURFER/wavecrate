use super::super::*;

#[test]
/// Saving from the waveform should accept deep, narrow selections on long files.
fn save_waveform_selection_to_browser_exports_narrow_deep_selection() {
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source");
    std::fs::create_dir_all(&source_root).unwrap();

    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(source_root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    let wav_path = source_root.join("long.wav");
    let samples = vec![0.25; 4096];
    write_test_wav(&wav_path, &samples);
    controller
        .load_waveform_for_selection(&source, Path::new("long.wav"))
        .unwrap();
    let narrow_deep_selection = SelectionRange::new(0.995, 0.9955);
    controller
        .selection_state
        .range
        .set_range(Some(narrow_deep_selection));
    controller.ui.waveform.selection = Some(narrow_deep_selection);

    controller
        .save_waveform_selection_to_browser(true)
        .expect("narrow selection should queue");

    assert_eq!(controller.ui.status.status_tone, StatusTone::Busy);
    assert_eq!(
        controller
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .map(|audio| &audio.relative_path),
        Some(&PathBuf::from("long.wav"))
    );
    pump_background_jobs_until(&mut controller, |controller| {
        source_root.join("long_selection_001.wav").is_file()
            && controller.ui.status.text.contains("Saved clip")
    });
    assert!(source_root.join("long_selection_001.wav").is_file());
    assert!(controller.ui.status.text.contains("Saved clip"));
    assert_eq!(
        controller
            .sample_view
            .wav
            .loaded_audio
            .as_ref()
            .map(|audio| &audio.relative_path),
        Some(&PathBuf::from("long.wav"))
    );
}

#[test]
/// Queued waveform selection exports should raise one optimistic native-shell flash token.
fn save_waveform_selection_to_browser_records_flash_nonce_immediately() {
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source");
    std::fs::create_dir_all(&source_root).unwrap();

    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(source_root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    let wav_path = source_root.join("flash.wav");
    write_test_wav(&wav_path, &[0.1, 0.2, 0.3, 0.4]);
    controller
        .load_waveform_for_selection(&source, Path::new("flash.wav"))
        .unwrap();
    let selection = SelectionRange::new(0.25, 0.75);
    controller.selection_state.range.set_range(Some(selection));
    controller.ui.waveform.selection = Some(selection);

    let before = controller.ui.waveform.selection_export_flash_nonce;
    controller
        .save_waveform_selection_to_browser(true)
        .expect("selection export should queue");

    assert_eq!(controller.ui.status.status_tone, StatusTone::Busy);
    assert_eq!(
        controller.ui.waveform.selection_export_flash_nonce,
        before + 1
    );
    pump_background_jobs_until(&mut controller, |_| {
        source_root.join("flash_selection_001.wav").is_file()
    });

    assert_eq!(
        controller.ui.waveform.selection_export_flash_nonce,
        before + 1
    );
}

#[test]
fn save_waveform_selection_to_browser_does_not_commit_pending_edit_fades() {
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source");
    std::fs::create_dir_all(&source_root).unwrap();

    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(source_root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    let wav_path = source_root.join("pending_edit.wav");
    write_test_wav(&wav_path, &[0.1, 0.2, 0.3, 0.4]);
    controller
        .load_waveform_for_selection(&source, Path::new("pending_edit.wav"))
        .unwrap();
    let selection = SelectionRange::new(0.25, 0.75);
    controller.selection_state.range.set_range(Some(selection));
    controller.ui.waveform.selection = Some(selection);
    controller.set_edit_selection_range(selection.with_fade_out(0.5, 0.0));

    let export_before = controller.ui.waveform.selection_export_flash_nonce;
    let apply_before = controller.ui.waveform.edit_selection_apply_flash_nonce;

    controller
        .save_waveform_selection_to_browser(true)
        .expect("selection export should queue");

    assert_eq!(
        controller.ui.waveform.selection_export_flash_nonce,
        export_before + 1
    );
    assert_eq!(
        controller.ui.waveform.edit_selection_apply_flash_nonce,
        apply_before
    );
    assert!(
        controller
            .ui
            .waveform
            .edit_selection
            .is_some_and(|selection| selection.has_edit_effects())
    );

    pump_background_jobs_until(&mut controller, |_| {
        source_root.join("pending_edit_selection_001.wav").is_file()
    });
    assert!(source_root.join("pending_edit_selection_001.wav").is_file());
}

#[test]
fn save_waveform_selection_to_browser_action_persists_keep1_tag() {
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
    let selection = SelectionRange::new(0.25, 0.75);
    controller.selection_state.range.set_range(Some(selection));
    controller.ui.waveform.selection = Some(selection);

    controller
        .save_waveform_selection_or_slices_to_browser_action_with_tag(true, Some(Rating::KEEP_1));

    pump_background_jobs_until(&mut controller, |controller| {
        source_root.join("clip_selection_001.wav").is_file()
            && controller.ui.status.text.contains("Saved clip")
    });

    let rows = controller
        .database_for(&source)
        .unwrap()
        .list_files()
        .unwrap();
    let exported = rows
        .iter()
        .find(|row| row.relative_path == PathBuf::from("clip_selection_001.wav"))
        .expect("exported clip should be registered");
    assert_eq!(exported.tag, Rating::KEEP_1);
}

#[test]
fn save_waveform_selection_to_browser_with_keep2_persists_keep2_tag() {
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
    let selection = SelectionRange::new(0.25, 0.75);
    controller.selection_state.range.set_range(Some(selection));
    controller.ui.waveform.selection = Some(selection);

    controller
        .save_waveform_selection_or_slices_to_browser_action_with_tag(true, Some(Rating::new(2)));

    pump_background_jobs_until(&mut controller, |controller| {
        source_root.join("clip_selection_001.wav").is_file()
            && controller.ui.status.text.contains("Saved clip")
    });

    let rows = controller
        .database_for(&source)
        .unwrap()
        .list_files()
        .unwrap();
    let exported = rows
        .iter()
        .find(|row| row.relative_path == PathBuf::from("clip_selection_001.wav"))
        .expect("exported clip should be registered");
    assert_eq!(exported.tag, Rating::new(2));
}

#[test]
/// Failed queued waveform selection exports should raise one deferred error flash token.
fn save_waveform_selection_to_browser_records_failure_flash_when_worker_fails() {
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let failure_before = controller.ui.waveform.selection_export_failure_flash_nonce;
    controller.apply_background_job_message_for_tests(JobMessage::SelectionExport(
        SelectionExportMessage::Finished(SelectionExportResult::Clip {
            request_id: 99,
            result: Err(String::from("Selection export failed")),
        }),
    ));

    assert_eq!(
        controller.ui.waveform.selection_export_failure_flash_nonce,
        failure_before + 1
    );
    assert_eq!(controller.ui.status.status_tone, StatusTone::Error);
    assert_eq!(controller.ui.status.text, "Selection export failed");
}
