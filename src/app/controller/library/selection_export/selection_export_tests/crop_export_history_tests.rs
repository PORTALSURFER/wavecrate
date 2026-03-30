use super::*;

#[test]
fn crop_waveform_selection_to_new_sample_registers_pending_history_and_supports_undo() {
    let temp = tempdir().unwrap();
    let source_root = temp.path().join("source");
    std::fs::create_dir_all(&source_root).unwrap();

    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(source_root.clone());
    controller.library.sources.push(source.clone());
    controller.selection_state.ctx.selected_source = Some(source.id.clone());
    controller.cache_db(&source).unwrap();

    let original_relative = PathBuf::from("crop.wav");
    write_test_wav(&source_root.join(&original_relative), &[0.1, 0.2, 0.3, 0.4]);
    let original_entry = written_entry(&source_root, &original_relative, Rating::NEUTRAL);
    let db = controller.database_for(&source).unwrap();
    db.upsert_file(
        &original_relative,
        original_entry.file_size,
        original_entry.modified_ns,
    )
    .unwrap();
    controller
        .load_waveform_for_selection(&source, Path::new("crop.wav"))
        .unwrap();
    let selection = SelectionRange::new(0.25, 0.75);
    controller.selection_state.range.set_range(Some(selection));
    controller.ui.waveform.selection = Some(selection);

    controller.crop_waveform_selection_to_new_sample().unwrap();

    let history_key = controller
        .history
        .pending_transactions
        .keys()
        .next()
        .cloned()
        .expect("crop export should register pending history");
    assert_eq!(controller.history.pending_transactions.len(), 1);

    let exported_relative = PathBuf::from("crop_crop001.wav");
    pump_background_jobs_until(&mut controller, |controller| {
        controller.history.pending_transactions.is_empty()
            && source_root.join(&exported_relative).is_file()
            && controller
                .sample_view
                .wav
                .loaded_audio
                .as_ref()
                .is_some_and(|audio| audio.relative_path == exported_relative)
    });

    assert!(controller.history.pending_transactions.is_empty());
    controller.undo();

    match controller
        .history
        .pending_undo
        .as_ref()
        .map(|pending| &pending.job)
    {
        Some(UndoFileJob::RemoveSample {
            source_id,
            relative_path,
            ..
        }) => {
            assert_eq!(source_id, &source.id);
            assert_eq!(relative_path, &exported_relative);
        }
        other => panic!("expected crop undo remove job, got {other:?}"),
    }

    assert!(matches!(
        history_key,
        PendingHistoryTransactionKey::SelectionExport { .. }
    ));

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
}

#[test]
fn crop_export_failure_cancels_pending_history_without_leaving_undo_state() {
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let history_key = PendingHistoryTransactionKey::SelectionExport { request_id: 99 };
    controller
        .begin_pending_sample_creation_transaction(history_key.clone(), "Cropped to new sample");

    controller.apply_background_job_message_for_tests(JobMessage::SelectionExport(
        SelectionExportMessage::Finished(SelectionExportResult::CropNewSample {
            request_id: 99,
            result: Err(String::from("Crop export failed")),
        }),
    ));

    assert!(controller.history.pending_transactions.is_empty());
    assert_eq!(controller.ui.status.text, "Crop export failed");

    controller.undo();

    assert!(controller.history.pending_undo.is_none());
    assert_eq!(controller.ui.status.text, "Nothing to undo");
    assert!(
        !controller
            .history
            .pending_transactions
            .contains_key(&history_key)
    );
}
