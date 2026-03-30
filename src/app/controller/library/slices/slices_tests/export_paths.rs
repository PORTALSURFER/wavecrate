use super::*;

#[test]
fn accept_waveform_slices_exports_files() {
    let (_temp, root) = prepare_source_dir();
    let source_root = root.join("source");
    std::fs::create_dir_all(&source_root).unwrap();
    let (mut controller, source) = make_controller(&source_root);
    controller.cache_db(&source).unwrap();

    let wav_path = write_clip(&source_root, "clip.wav", &[0.2, 0.2, 0.0, 0.0, 0.3, 0.3]);
    controller
        .load_waveform_for_selection(&source, wav_path.strip_prefix(&source_root).unwrap())
        .unwrap();
    controller.ui.waveform.slices =
        vec![SelectionRange::new(0.0, 0.5), SelectionRange::new(0.5, 1.0)];

    let count = controller.accept_waveform_slices().unwrap();

    assert_eq!(count, 2);
    assert!(source_root.join("clip_slice001.wav").exists());
    assert!(source_root.join("clip_slice002.wav").exists());
    assert_eq!(
        controller.ui.waveform.slice_batch_profile,
        WaveformSliceBatchProfile::Manual
    );
}

#[test]
fn accept_waveform_slices_uses_silence_split_suffix() {
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
    controller.ui.waveform.slice_batch_profile = WaveformSliceBatchProfile::SilenceSplit;
    controller.ui.waveform.slices = vec![
        SelectionRange::new(0.0, 0.25),
        SelectionRange::new(0.5, 0.75),
    ];

    let count = controller.accept_waveform_slices().unwrap();

    assert_eq!(count, 2);
    assert!(source_root.join("clip_silence_split_001.wav").exists());
    assert!(source_root.join("clip_silence_split_002.wav").exists());
    assert_eq!(
        controller.ui.waveform.slice_batch_profile,
        WaveformSliceBatchProfile::Manual
    );
    assert!(controller.ui.waveform.slices.is_empty());
    assert!(controller.ui.waveform.selected_slices.is_empty());
}
