use super::*;

#[test]
fn exact_duplicate_cleanup_request_prompts_without_selection() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "dups.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = source.root.join("dups.wav");
    write_test_wav(
        &wav_path,
        &[0.8, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0, 0.0, 0.4, 0.0, 0.0, 0.0],
    );
    controller
        .load_waveform_for_selection(&source, Path::new("dups.wav"))
        .unwrap();
    controller.ui.waveform.selection = Some(SelectionRange::new(0.0, 4.0 / 12.0));
    controller
        .detect_waveform_exact_duplicate_slices_from_selection()
        .unwrap();
    controller.ui.waveform.selection = None;
    controller.set_destructive_yolo_mode(false);

    let outcome = controller
        .request_destructive_selection_edit(DestructiveSelectionEdit::CleanExactDuplicateBeats)
        .unwrap();

    assert!(matches!(outcome, SelectionEditRequest::Prompted));
    let prompt = controller
        .ui
        .waveform
        .pending_destructive
        .as_ref()
        .expect("pending destructive prompt");
    assert_eq!(
        prompt.edit,
        DestructiveSelectionEdit::CleanExactDuplicateBeats
    );
}

#[test]
fn clean_exact_duplicate_beats_overwrites_file_and_clears_cleanup_batch() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "dups.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = source.root.join("dups.wav");
    write_test_wav(
        &wav_path,
        &[0.8, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0, 0.0, 0.4, 0.0, 0.0, 0.0],
    );
    controller
        .load_waveform_for_selection(&source, Path::new("dups.wav"))
        .unwrap();
    controller.ui.waveform.selection = Some(SelectionRange::new(0.0, 4.0 / 12.0));
    controller
        .detect_waveform_exact_duplicate_slices_from_selection()
        .unwrap();

    controller.clean_exact_duplicate_beats().unwrap();

    let samples: Vec<f32> = WavReader::open(&wav_path)
        .unwrap()
        .samples::<f32>()
        .map(|sample| sample.unwrap())
        .collect();
    assert_eq!(samples, vec![0.8, 0.0, 0.0, 0.0, 0.4, 0.0, 0.0, 0.0]);
    assert!(controller.ui.waveform.slices.is_empty());
    assert_eq!(controller.ui.waveform.slice_batch_beat_count, 0);
    assert!(
        controller
            .ui
            .status
            .text
            .contains("Removed 1 duplicate window(s)")
    );
}
