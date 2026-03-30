use super::*;

#[test]
fn slice_review_navigation_and_marking_use_dedicated_state() {
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    controller.ui.waveform.slices = vec![
        SelectionRange::new(0.0, 0.2),
        SelectionRange::new(0.3, 0.4),
        SelectionRange::new(0.6, 0.8),
    ];

    assert!(controller.start_slice_review());
    assert_eq!(controller.ui.waveform.slice_review.focused_index, Some(0));

    assert!(controller.move_slice_review_focus(1));
    assert_eq!(controller.ui.waveform.slice_review.focused_index, Some(1));

    let marked = controller.toggle_focused_slice_export_mark().unwrap();
    assert!(marked);
    assert_eq!(controller.ui.waveform.slice_review.marked_indices, vec![1]);
    assert!(controller.ui.waveform.selected_slices.is_empty());

    assert!(controller.exit_slice_review());
    assert!(!controller.ui.waveform.slice_review.active);
    assert!(controller.ui.waveform.slice_review.focused_index.is_none());
    assert_eq!(controller.ui.waveform.slice_review.marked_indices, vec![1]);
}

#[test]
fn toggle_focused_slice_export_mark_rejects_duplicate_cleanup_batches() {
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    controller.ui.waveform.slices = vec![SelectionRange::new(0.25, 0.5)];
    controller.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::ExactDuplicateBeats;
    controller.start_slice_review();

    let err = controller
        .toggle_focused_slice_export_mark()
        .expect_err("duplicate cleanup batches should not be export-marked");

    assert_eq!(err, "Duplicate cleanup batches cannot be export-marked");
}

#[test]
fn apply_painted_slice_cuts_existing_ranges() {
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    controller.ui.waveform.slices = vec![SelectionRange::new(0.2, 0.8)];
    controller.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::SilenceSplit;

    let added = controller.apply_painted_slice(SelectionRange::new(0.4, 0.6));

    assert!(added);
    assert_eq!(controller.ui.waveform.slices.len(), 3);
    assert_eq!(
        controller.ui.waveform.slices[0],
        SelectionRange::new(0.2, 0.4)
    );
    assert_eq!(
        controller.ui.waveform.slices[1],
        SelectionRange::new(0.4, 0.6)
    );
    assert_eq!(
        controller.ui.waveform.slices[2],
        SelectionRange::new(0.6, 0.8)
    );
    assert_eq!(
        controller.ui.waveform.slice_batch_profile,
        WaveformSliceBatchProfile::Manual
    );
}

#[test]
fn clear_waveform_slices_resets_batch_profile() {
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    controller.ui.waveform.slices = vec![SelectionRange::new(0.1, 0.2)];
    controller.ui.waveform.selected_slices = vec![0];
    controller.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::SilenceSplit;

    controller.clear_waveform_slices();

    assert!(controller.ui.waveform.slices.is_empty());
    assert!(controller.ui.waveform.selected_slices.is_empty());
    assert!(controller.ui.waveform.duplicate_cleanup.is_none());
    assert_eq!(
        controller.ui.waveform.slice_batch_profile,
        WaveformSliceBatchProfile::Manual
    );
}

#[test]
fn merge_selected_slices_spans_between_markers() {
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    controller.ui.waveform.slices = vec![
        SelectionRange::new(0.1, 0.2),
        SelectionRange::new(0.35, 0.45),
        SelectionRange::new(0.7, 0.8),
    ];
    controller.ui.waveform.selected_slices = vec![0, 2];

    let merged = controller.merge_selected_slices();

    assert_eq!(merged, Some(SelectionRange::new(0.1, 0.8)));
    assert_eq!(controller.ui.waveform.slices.len(), 1);
    assert_eq!(
        controller.ui.waveform.slices[0],
        SelectionRange::new(0.1, 0.8)
    );
    assert_eq!(controller.ui.waveform.selected_slices, vec![0]);
    assert_eq!(
        controller.ui.waveform.slice_batch_profile,
        WaveformSliceBatchProfile::Manual
    );
}

#[test]
fn accept_waveform_slices_rejects_duplicate_cleanup_batches() {
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    controller.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::ExactDuplicateBeats;
    controller.ui.waveform.slices = vec![SelectionRange::new(0.25, 0.5)];

    let err = controller
        .accept_waveform_slices()
        .expect_err("duplicate cleanup batches should not export");

    assert_eq!(err, "Use Clean Dups to apply duplicate cleanup");
}
