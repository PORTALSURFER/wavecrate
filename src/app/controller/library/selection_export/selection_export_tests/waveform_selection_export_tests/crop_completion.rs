use super::super::*;

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
    controller.set_wav_entries_for_tests(vec![written_entry(
        &source_root,
        Path::new("clip.wav"),
        Rating::NEUTRAL,
    )]);
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
    let history_key = PendingHistoryTransactionKey::SelectionExport { request_id: 7 };
    controller
        .begin_pending_sample_creation_transaction(history_key.clone(), "Cropped to new sample");

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
            assert_eq!(relative_path, &PathBuf::from("clip_crop_001.wav"));
        }
        other => panic!("expected crop undo remove job, got {other:?}"),
    }
    assert!(controller.history.pending_transactions.is_empty());
    assert_eq!(
        history_key,
        PendingHistoryTransactionKey::SelectionExport { request_id: 7 }
    );
}
