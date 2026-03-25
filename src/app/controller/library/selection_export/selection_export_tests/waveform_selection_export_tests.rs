use super::*;

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
fn save_waveform_selection_to_browser_success_finishes_pending_history_and_supports_undo() {
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
        .save_waveform_selection_to_browser(true)
        .expect("selection export should queue");

    let history_key = controller
        .history
        .pending_transactions
        .keys()
        .next()
        .cloned()
        .expect("selection export should register pending history");
    let request_id = match history_key {
        PendingHistoryTransactionKey::SelectionExport { request_id } => request_id,
        other => panic!("unexpected history key: {other:?}"),
    };
    assert_eq!(controller.history.pending_transactions.len(), 1);

    let exported_relative = PathBuf::from("clip_selection_001.wav");
    pump_background_jobs_until(&mut controller, |controller| {
        controller.history.pending_transactions.is_empty()
            && source_root.join(&exported_relative).is_file()
    });

    assert!(controller.history.pending_transactions.is_empty());
    assert!(controller.wav_index_for_path(&exported_relative).is_some());

    controller.undo();

    match controller.history.pending_undo.as_ref().map(|pending| &pending.job) {
        Some(UndoFileJob::RemoveSample {
            source_id,
            relative_path,
            ..
        }) => {
            assert_eq!(source_id, &source.id);
            assert_eq!(relative_path, &exported_relative);
        }
        other => panic!("expected deferred remove undo job, got {other:?}"),
    }
    assert!(
        controller
            .ui
            .status
            .text
            .contains("Undoing Saved selection clip"),
        "status was {:?}",
        controller.ui.status.text
    );

    std::fs::remove_file(source_root.join(&exported_relative)).unwrap();
    controller
        .database_for(&source)
        .unwrap()
        .remove_file(&exported_relative)
        .unwrap();
    controller.apply_file_op_result(FileOpResult::UndoFile(UndoFileOpResult {
        result: Ok(UndoFileOutcome::Removed {
            source_id: source.id.clone(),
            relative_path: exported_relative.clone(),
        }),
        cancelled: false,
    }));

    assert!(controller.history.pending_undo.is_none());
    assert!(controller.wav_index_for_path(&exported_relative).is_none());
    assert_eq!(controller.ui.status.text, "Undid Saved selection clip");
    assert_eq!(
        PendingHistoryTransactionKey::SelectionExport { request_id },
        history_key
    );
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

#[test]
fn selection_export_failure_cancels_pending_history_without_leaving_undo_state() {
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let history_key = PendingHistoryTransactionKey::SelectionExport { request_id: 99 };
    controller.begin_pending_sample_creation_transaction(history_key.clone(), "Saved selection clip");

    controller.apply_background_job_message_for_tests(JobMessage::SelectionExport(
        SelectionExportMessage::Finished(SelectionExportResult::Clip {
            request_id: 99,
            result: Err(String::from("Selection export failed")),
        }),
    ));

    assert!(controller.history.pending_transactions.is_empty());
    assert_eq!(controller.ui.status.text, "Selection export failed");

    controller.undo();

    assert!(controller.history.pending_undo.is_none());
    assert_eq!(controller.ui.status.text, "Nothing to undo");
    assert!(!controller.history.pending_transactions.contains_key(&history_key));
}

#[test]
fn apply_selection_crop_export_success_restores_focus_playback_and_undo_state() {
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source");
    std::fs::create_dir_all(&source_root).unwrap();

    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(source_root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    let original_path = source_root.join("clip.wav");
    write_test_wav(&original_path, &[0.1, 0.2, 0.3, 0.4]);
    controller
        .load_waveform_for_selection(&source, Path::new("clip.wav"))
        .unwrap();
    controller
        .set_wav_entries_for_tests(vec![written_entry(&source_root, Path::new("clip.wav"), Rating::NEUTRAL)]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.ui.focus.context = FocusContext::SampleBrowser;

    let cropped_relative = PathBuf::from("clip_crop_001.wav");
    let cropped_absolute = source_root.join(&cropped_relative);
    write_test_wav(&cropped_absolute, &[0.2, 0.3]);
    let entry = written_entry(&source_root, &cropped_relative, Rating::KEEP_1);
    let db = controller.database_for(&source).unwrap();
    db.upsert_file(&cropped_relative, entry.file_size, entry.modified_ns)
        .unwrap();
    db.set_tag(&cropped_relative, entry.tag).unwrap();

    controller.apply_selection_crop_export_success(SelectionCropExportSuccess {
        request_id: 7,
        source_id: source.id.clone(),
        source_root: source.root.clone(),
        source_relative_path: PathBuf::from("clip.wav"),
        entry: entry.clone(),
        absolute_path: cropped_absolute,
        tag: Rating::KEEP_1,
        playback: SelectionExportPlaybackState {
            was_playing: true,
            was_looping: true,
            start_override: Some(0.25),
        },
        timings: SelectionExportTimings::default(),
    });

    assert_eq!(controller.ui.focus.context, FocusContext::Waveform);
    assert_eq!(
        controller.sample_view.wav.selected_wav.as_deref(),
        Some(cropped_relative.as_path())
    );
    match controller.runtime.jobs.pending_playback.as_ref() {
        Some(pending) => {
            assert_eq!(pending.source_id, source.id);
            assert_eq!(pending.relative_path, cropped_relative);
            assert!(pending.looped);
            assert_eq!(pending.start_override, Some(0.25));
        }
        None => panic!("expected crop completion to queue playback resume"),
    }

    controller.undo();

    match controller.history.pending_undo.as_ref().map(|pending| &pending.job) {
        Some(UndoFileJob::RemoveSample {
            source_id,
            relative_path,
            ..
        }) => {
            assert_eq!(source_id, &source.id);
            assert_eq!(relative_path, &PathBuf::from("clip_crop_001.wav"));
        }
        other => panic!("expected crop undo remove job, got {other:?}"),
    }
}
