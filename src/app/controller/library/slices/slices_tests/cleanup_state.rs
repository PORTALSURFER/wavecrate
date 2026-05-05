use super::*;

#[test]
fn duplicate_cleanup_exemption_keeps_preview_but_reduces_cleanup_count() {
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
    controller
        .detect_waveform_exact_duplicate_slices_from_selection()
        .unwrap();

    let exempted = controller
        .toggle_duplicate_cleanup_preview_exemption(0)
        .expect("duplicate preview should toggle");

    assert!(exempted);
    assert_eq!(controller.ui.waveform.slices.len(), 1);
    assert_eq!(controller.ui.waveform.slice_batch_beat_count, 0);
    assert!(
        controller
            .ui
            .waveform
            .duplicate_cleanup
            .as_ref()
            .is_some_and(|state| state.previews[0].exempted)
    );
}

#[test]
fn delete_selected_slices_removes_marked_ranges() {
    let (_temp, root) = prepare_source_dir();
    let source_root = root.join("source");
    std::fs::create_dir_all(&source_root).unwrap();
    let (mut controller, _source) = make_controller(&source_root);

    controller.ui.waveform.slices = vec![
        SelectionRange::new(0.0, 0.2),
        SelectionRange::new(0.3, 0.4),
        SelectionRange::new(0.6, 0.7),
    ];
    controller.ui.waveform.selected_slices = vec![0, 2];

    let removed = controller.delete_selected_slices();

    assert_eq!(removed, 2);
    assert_eq!(controller.ui.waveform.slices.len(), 1);
    assert_eq!(
        controller.ui.waveform.slices[0],
        SelectionRange::new(0.3, 0.4)
    );
    assert!(controller.ui.waveform.selected_slices.is_empty());
    assert_eq!(
        controller.ui.waveform.slice_batch_profile,
        WaveformSliceBatchProfile::Manual
    );
}

#[test]
fn delete_selected_slices_preserves_duplicate_cleanup_profile_and_recounts_windows() {
    let (_temp, root) = prepare_source_dir();
    let source_root = root.join("source");
    std::fs::create_dir_all(&source_root).unwrap();
    let (mut controller, source) = make_controller(&source_root);
    controller.cache_db(&source).unwrap();

    let wav_path = write_clip(
        &source_root,
        "clip.wav",
        &[
            0.8, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0, 0.0, 0.4, 0.0, 0.0, 0.0, 0.4, 0.0, 0.0, 0.0,
        ],
    );
    controller
        .load_waveform_for_selection(&source, wav_path.strip_prefix(&source_root).unwrap())
        .unwrap();
    controller
        .ui
        .waveform
        .selection
        .replace(SelectionRange::new(0.0, 4.0 / 16.0));
    controller
        .detect_waveform_exact_duplicate_slices_from_selection()
        .unwrap();
    controller.ui.waveform.selected_slices = vec![0];

    let removed = controller.delete_selected_slices();

    assert_eq!(removed, 1);
    assert_eq!(controller.ui.waveform.slices.len(), 1);
    assert_eq!(
        controller.ui.waveform.slice_batch_profile,
        WaveformSliceBatchProfile::ExactDuplicateBeats
    );
    assert_eq!(controller.ui.waveform.slice_batch_beat_count, 1);
}
