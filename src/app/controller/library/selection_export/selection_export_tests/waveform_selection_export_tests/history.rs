use super::super::*;

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
fn selection_export_failure_cancels_pending_history_without_leaving_undo_state() {
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let history_key = PendingHistoryTransactionKey::SelectionExport { request_id: 99 };
    controller
        .begin_pending_sample_creation_transaction(history_key.clone(), "Saved selection clip");

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
    assert!(
        !controller
            .history
            .pending_transactions
            .contains_key(&history_key)
    );
}
