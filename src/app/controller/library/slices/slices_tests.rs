use crate::app::controller::AppController;
use crate::app::controller::test_support::write_test_wav;
use crate::app::state::WaveformSliceBatchProfile;
use crate::sample_sources::SampleSource;
use crate::selection::SelectionRange;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn next_slice_path_in_dir_skips_existing_suffixes() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    std::fs::write(root.join("clip_slice001.wav"), b"").unwrap();
    std::fs::write(root.join("clip_slice002.wav"), b"").unwrap();

    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.to_path_buf());
    controller.library.sources.push(source.clone());

    let mut counter = 1usize;
    let candidate = controller.next_slice_path_in_dir(
        &source,
        Path::new("clip.wav"),
        WaveformSliceBatchProfile::Manual,
        &mut counter,
    );

    assert_eq!(candidate, Path::new("clip_slice003.wav"));
}

#[test]
fn next_slice_path_in_dir_uses_silence_split_suffix() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    std::fs::write(root.join("clip_silence_split_001.wav"), b"").unwrap();
    std::fs::write(root.join("clip_silence_split_002.wav"), b"").unwrap();

    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.to_path_buf());
    controller.library.sources.push(source.clone());

    let mut counter = 1usize;
    let candidate = controller.next_slice_path_in_dir(
        &source,
        Path::new("clip.wav"),
        WaveformSliceBatchProfile::SilenceSplit,
        &mut counter,
    );

    assert_eq!(candidate, Path::new("clip_silence_split_003.wav"));
}

#[test]
fn accept_waveform_slices_exports_files() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();

    let wav_path = root.join("clip.wav");
    write_test_wav(&wav_path, &[0.2, 0.2, 0.0, 0.0, 0.3, 0.3]);
    controller
        .load_waveform_for_selection(&source, Path::new("clip.wav"))
        .unwrap();
    controller.ui.waveform.slices =
        vec![SelectionRange::new(0.0, 0.5), SelectionRange::new(0.5, 1.0)];

    let count = controller.accept_waveform_slices().unwrap();

    assert_eq!(count, 2);
    assert!(root.join("clip_slice001.wav").exists());
    assert!(root.join("clip_slice002.wav").exists());
    assert_eq!(
        controller.ui.waveform.slice_batch_profile,
        WaveformSliceBatchProfile::Manual
    );
}

#[test]
fn accept_waveform_slices_uses_silence_split_suffix() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();

    let wav_path = root.join("clip.wav");
    write_test_wav(&wav_path, &[0.0, 0.6, 0.6, 0.0, 0.6, 0.6, 0.0, 0.0]);
    controller
        .load_waveform_for_selection(&source, Path::new("clip.wav"))
        .unwrap();
    controller.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::SilenceSplit;
    controller.ui.waveform.slices = vec![
        SelectionRange::new(0.0, 0.25),
        SelectionRange::new(0.5, 0.75),
    ];

    let count = controller.accept_waveform_slices().unwrap();

    assert_eq!(count, 2);
    assert!(root.join("clip_silence_split_001.wav").exists());
    assert!(root.join("clip_silence_split_002.wav").exists());
    assert_eq!(
        controller.ui.waveform.slice_batch_profile,
        WaveformSliceBatchProfile::Manual
    );
    assert!(controller.ui.waveform.slices.is_empty());
    assert!(controller.ui.waveform.selected_slices.is_empty());
}

#[test]
fn detect_waveform_slices_ignores_transient_settings() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();

    let wav_path = root.join("clip.wav");
    write_test_wav(&wav_path, &[0.0, 0.6, 0.6, 0.0, 0.6, 0.6, 0.0, 0.0]);
    controller
        .load_waveform_for_selection(&source, Path::new("clip.wav"))
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
    let temp = tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();

    let wav_path = root.join("clip.wav");
    write_test_wav(
        &wav_path,
        &[0.8, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0, 0.0, 0.4, 0.0, 0.0, 0.0],
    );
    controller
        .load_waveform_for_selection(&source, Path::new("clip.wav"))
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
    assert!(controller.ui.status.text.contains("duplicate windows"));
}

#[test]
fn detect_waveform_exact_duplicate_slices_uses_selection_window_anchor() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();

    let wav_path = root.join("clip.wav");
    write_test_wav(
        &wav_path,
        &[
            9.0, 0.0, 1.0, 0.0, 0.0, 0.0, 7.0, 0.0, 1.0, 0.0, 0.0, 0.0, 5.0,
        ],
    );
    controller
        .load_waveform_for_selection(&source, Path::new("clip.wav"))
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
    let temp = tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();

    let wav_path = root.join("clip.wav");
    write_test_wav(
        &wav_path,
        &[
            0.0, 0.8, 0.0, 0.0, 0.0, 0.6, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0, 0.0, 0.4, 0.0, 0.0, 0.0,
            0.6, 0.0, 0.0, 0.0, 0.3, 0.0, 0.0, 0.0, 0.2, 0.0, 0.0, 0.0, 0.3, 0.0, 0.0, 0.0, 0.4,
            0.0, 0.0, 0.0, 0.2, 0.0, 0.0,
        ],
    );
    controller
        .load_waveform_for_selection(&source, Path::new("clip.wav"))
        .unwrap();
    controller.ui.waveform.selection = Some(SelectionRange::new(0.0, 4.0 / 40.0));

    let count = controller
        .detect_waveform_exact_duplicate_slices_from_selection()
        .unwrap();

    assert_eq!(count, 3);
    assert_eq!(controller.ui.waveform.slices.len(), 3);
    assert_eq!(controller.ui.waveform.slice_batch_beat_count, 5);
}

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

    assert_eq!(
        err,
        "Exact duplicate cleanup batches cannot be export-marked"
    );
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
    assert_eq!(
        controller.ui.waveform.slice_batch_profile,
        WaveformSliceBatchProfile::Manual
    );
}

#[test]
fn delete_selected_slices_removes_marked_ranges() {
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
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
    let temp = tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir_all(&root).unwrap();
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.clone());
    controller.library.sources.push(source.clone());
    controller.cache_db(&source).unwrap();

    let wav_path = root.join("clip.wav");
    write_test_wav(
        &wav_path,
        &[
            0.8, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0, 0.0, 0.4, 0.0, 0.0, 0.0, 0.4, 0.0, 0.0, 0.0,
        ],
    );
    controller
        .load_waveform_for_selection(&source, Path::new("clip.wav"))
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

    assert_eq!(err, "Use Clean Dups to apply exact duplicate cleanup");
}
