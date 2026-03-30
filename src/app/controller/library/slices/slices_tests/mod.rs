use crate::app::controller::test_support::write_test_wav;
use crate::app::controller::AppController;
use crate::app::state::WaveformSliceBatchProfile;
use crate::sample_sources::SampleSource;
use crate::selection::SelectionRange;
use std::path::Path;
use tempfile::tempdir;

mod cleanup_state;
mod export_paths;
mod review_state;
mod slice_detection;

fn make_controller(root: &std::path::Path) -> (AppController, SampleSource) {
    let renderer = crate::waveform::WaveformRenderer::new(12, 12);
    let mut controller = AppController::new(renderer, None);
    let source = SampleSource::new(root.to_path_buf());
    controller.library.sources.push(source.clone());
    (controller, source)
}

fn prepare_source_dir() -> (tempfile::TempDir, std::path::PathBuf) {
    let temp = tempdir().unwrap();
    let root = temp.path().to_path_buf();
    (temp, root)
}

fn write_clip(root: &std::path::Path, name: &str, samples: &[f32]) -> std::path::PathBuf {
    let path = root.join(name);
    write_test_wav(&path, samples);
    path
}

#[test]
fn next_slice_path_in_dir_skips_existing_suffixes() {
    let (_temp, root) = prepare_source_dir();
    std::fs::write(root.join("clip_slice001.wav"), b"").unwrap();
    std::fs::write(root.join("clip_slice002.wav"), b"").unwrap();

    let (controller, source) = make_controller(&root);

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
    let (_temp, root) = prepare_source_dir();
    std::fs::write(root.join("clip_silence_split_001.wav"), b"").unwrap();
    std::fs::write(root.join("clip_silence_split_002.wav"), b"").unwrap();

    let (controller, source) = make_controller(&root);

    let mut counter = 1usize;
    let candidate = controller.next_slice_path_in_dir(
        &source,
        Path::new("clip.wav"),
        WaveformSliceBatchProfile::SilenceSplit,
        &mut counter,
    );

    assert_eq!(candidate, Path::new("clip_silence_split_003.wav"));
}
