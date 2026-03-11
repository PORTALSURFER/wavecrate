use super::super::super::test_support::{
    prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
};
use crate::sample_sources::SampleSource;
use std::mem;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

#[test]
fn waveform_image_resizes_to_view() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "resize.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = source.root.join("resize.wav");
    write_test_wav(&wav_path, &[0.0, 0.25, -0.5, 0.75]);

    controller
        .load_waveform_for_selection(&source, Path::new("resize.wav"))
        .unwrap();
    controller.update_waveform_size(24, 8);

    let size = controller.ui.waveform.image.as_ref().unwrap().size;
    assert_eq!(size, [24, 8]);
}

#[test]
fn removing_selected_source_clears_waveform_view() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "one.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = source.root.join("one.wav");
    write_test_wav(&wav_path, &[0.1, -0.1]);
    controller
        .load_waveform_for_selection(&source, Path::new("one.wav"))
        .unwrap();

    controller.remove_source(0);

    assert!(controller.ui.waveform.image.is_none());
    assert!(controller.ui.waveform.selection.is_none());
    assert!(controller.sample_view.wav.loaded_audio.is_none());
    assert!(controller.sample_view.wav.loaded_wav.is_none());
}

#[test]
fn switching_sources_resets_waveform_state() {
    let (mut controller, first) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "a.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = first.root.join("a.wav");
    write_test_wav(&wav_path, &[0.0, 0.1]);
    controller
        .load_waveform_for_selection(&first, Path::new("a.wav"))
        .unwrap();

    let second_dir = tempdir().unwrap();
    let second_root = second_dir.path().join("second");
    std::fs::create_dir_all(&second_root).unwrap();
    mem::forget(second_dir);
    let second = SampleSource::new(second_root);
    controller.library.sources.push(second.clone());

    controller.select_source(Some(second.id.clone()));

    assert!(controller.ui.waveform.image.is_none());
    assert!(controller.ui.waveform.notice.is_none());
    assert!(controller.sample_view.wav.loaded_audio.is_none());
}

#[test]
fn pruning_missing_selection_clears_waveform_view() {
    let (mut controller, source) = prepare_with_source_and_wav_entries(vec![sample_entry(
        "gone.wav",
        crate::sample_sources::Rating::NEUTRAL,
    )]);
    let wav_path = source.root.join("gone.wav");
    write_test_wav(&wav_path, &[0.2, -0.2]);
    controller.sample_view.wav.selected_wav = Some(PathBuf::from("gone.wav"));
    controller
        .load_waveform_for_selection(&source, Path::new("gone.wav"))
        .unwrap();

    controller.set_wav_entries_for_tests(Vec::new());
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();

    assert!(controller.ui.waveform.image.is_none());
    assert!(controller.ui.waveform.selection.is_none());
    assert!(controller.sample_view.wav.loaded_audio.is_none());
    assert!(controller.sample_view.wav.loaded_wav.is_none());
}
