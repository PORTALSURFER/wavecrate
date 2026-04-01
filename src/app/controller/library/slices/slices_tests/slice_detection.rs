use super::*;

#[test]
fn detect_waveform_slices_ignores_transient_settings() {
    let (_temp, root) = prepare_source_dir();
    let source_root = root.join("source");
    std::fs::create_dir_all(&source_root).unwrap();
    let (mut controller, source) = make_controller(&source_root);
    controller.cache_db(&source).unwrap();

    let wav_path = write_clip(
        &source_root,
        "clip.wav",
        &[0.0, 0.6, 0.6, 0.0, 0.6, 0.6, 0.0, 0.0],
    );
    controller
        .load_waveform_for_selection(&source, wav_path.strip_prefix(&source_root).unwrap())
        .unwrap();
    controller.ui.waveform.transient_markers_enabled = true;
    controller.ui.waveform.transient_snap_enabled = true;
    controller.ui.waveform.transients = vec![0.25, 0.5, 0.75].into();

    let count = controller.detect_waveform_slices_from_silence().unwrap();

    assert_eq!(count, 2);
    assert_eq!(controller.ui.waveform.slices.len(), 2);
    let first = controller.ui.waveform.slices[0];
    let second = controller.ui.waveform.slices[1];
    assert!((first.start() - 0.125).abs() < 1.0e-6);
    assert!((first.end() - 0.375).abs() < 1.0e-6);
    assert!((second.start() - 0.5).abs() < 1.0e-6);
    assert!((second.end() - 0.75).abs() < 1.0e-6);
    assert_eq!(
        controller.ui.waveform.slice_batch_profile,
        WaveformSliceBatchProfile::SilenceSplit
    );
    assert!(controller.ui.waveform.slice_mode_enabled);
    assert!(controller.ui.waveform.selected_slices.is_empty());
    assert!(controller.ui.waveform.slice_review.active);
    assert_eq!(controller.ui.waveform.slice_review.focused_index, Some(0));
    assert!(
        controller
            .ui
            .waveform
            .slice_review
            .marked_indices
            .is_empty()
    );
    assert!(controller.ui.status.text.contains("Space audition"));
}

#[test]
fn detect_waveform_exact_duplicate_slices_from_selection_keeps_first_window() {
    let (_temp, root) = prepare_source_dir();
    let source_root = root.join("source");
    std::fs::create_dir_all(&source_root).unwrap();
    let (mut controller, source) = make_controller(&source_root);
    controller.cache_db(&source).unwrap();

    let wav_path = write_clip(
        &source_root,
        "clip.wav",
        &[0.8, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0, 0.0, 0.4, 0.0, 0.0, 0.0],
    );
    controller
        .load_waveform_for_selection(&source, wav_path.strip_prefix(&source_root).unwrap())
        .unwrap();
    controller.ui.waveform.selection = Some(SelectionRange::new(0.0, 4.0 / 12.0));

    let count = controller
        .detect_waveform_exact_duplicate_slices_from_selection()
        .unwrap();

    assert_eq!(count, 1);
    assert_eq!(controller.ui.waveform.slices.len(), 1);
    let duplicate = controller.ui.waveform.slices[0];
    assert!((duplicate.start() - (4.0 / 12.0)).abs() < 1.0e-6);
    assert!((duplicate.end() - (8.0 / 12.0)).abs() < 1.0e-6);
    assert_eq!(
        controller.ui.waveform.slice_batch_profile,
        WaveformSliceBatchProfile::ExactDuplicateBeats
    );
    assert_eq!(controller.ui.waveform.slice_batch_beat_count, 1);
    assert_eq!(
        controller
            .ui
            .waveform
            .duplicate_cleanup
            .as_ref()
            .map(|state| state.group_count),
        Some(1)
    );
    assert!(controller.ui.status.text.contains("Left-click audition"));
}

#[test]
fn detect_waveform_exact_duplicate_slices_uses_selection_window_anchor() {
    let (_temp, root) = prepare_source_dir();
    let source_root = root.join("source");
    std::fs::create_dir_all(&source_root).unwrap();
    let (mut controller, source) = make_controller(&source_root);
    controller.cache_db(&source).unwrap();

    let wav_path = write_clip(
        &source_root,
        "clip.wav",
        &[
            9.0, 0.0, 1.0, 0.0, 0.0, 0.0, 7.0, 0.0, 1.0, 0.0, 0.0, 0.0, 5.0,
        ],
    );
    controller
        .load_waveform_for_selection(&source, wav_path.strip_prefix(&source_root).unwrap())
        .unwrap();
    controller.ui.waveform.selection = Some(SelectionRange::new(1.0 / 13.0, 5.0 / 13.0));

    let count = controller
        .detect_waveform_exact_duplicate_slices_from_selection()
        .unwrap();

    assert_eq!(count, 1);
    assert_eq!(controller.ui.waveform.slices.len(), 1);
    let duplicate = controller.ui.waveform.slices[0];
    assert!((duplicate.start() - (7.0 / 13.0)).abs() < 1.0e-6);
    assert!((duplicate.end() - (11.0 / 13.0)).abs() < 1.0e-6);
}

#[test]
fn detect_waveform_exact_duplicate_slices_marks_all_duplicate_groups_for_selection_size() {
    let (_temp, root) = prepare_source_dir();
    let source_root = root.join("source");
    std::fs::create_dir_all(&source_root).unwrap();
    let (mut controller, source) = make_controller(&source_root);
    controller.cache_db(&source).unwrap();

    let wav_path = write_clip(
        &source_root,
        "clip.wav",
        &[
            0.0, 0.8, 0.0, 0.0, 0.0, 0.6, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0, 0.0, 0.4, 0.0, 0.0, 0.0,
            0.6, 0.0, 0.0, 0.0, 0.3, 0.0, 0.0, 0.0, 0.2, 0.0, 0.0, 0.0, 0.3, 0.0, 0.0, 0.0, 0.4,
            0.0, 0.0, 0.0, 0.2, 0.0, 0.0,
        ],
    );
    controller
        .load_waveform_for_selection(&source, wav_path.strip_prefix(&source_root).unwrap())
        .unwrap();
    controller.ui.waveform.selection = Some(SelectionRange::new(0.0, 4.0 / 40.0));

    let count = controller
        .detect_waveform_exact_duplicate_slices_from_selection()
        .unwrap();

    assert_eq!(count, 5);
    assert_eq!(controller.ui.waveform.slices.len(), 5);
    assert_eq!(controller.ui.waveform.slice_batch_beat_count, 5);
}
