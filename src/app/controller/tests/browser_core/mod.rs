use super::super::test_support::{
    dummy_controller, prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
};
use super::common::visible_indices;
use crate::app::state::{TriageFlagColumn, TriageFlagFilter};
use crate::sample_sources::Rating;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

mod filters;
mod loading;
mod marks;
mod selection;
mod tagging;

fn browser_row_is_queued_or_loaded(
    controller: &crate::app::controller::AppController,
    relative_path: &Path,
) -> bool {
    controller
        .runtime
        .jobs
        .pending_audio
        .as_ref()
        .is_some_and(|pending| pending.relative_path == relative_path)
        || controller.ui.waveform.loading.as_deref() == Some(relative_path)
        || controller.sample_view.wav.loaded_wav.as_deref() == Some(relative_path)
}

fn browser_rating_filter_fixture(
    locked_keep: bool,
) -> (
    crate::app::controller::AppController,
    crate::sample_sources::SampleSource,
) {
    let (mut controller, source) = dummy_controller();
    controller.library.sources.push(source.clone());
    let mut locked_keep_entry = sample_entry("locked_keep.wav", Rating::KEEP_3);
    if locked_keep {
        locked_keep_entry.locked = true;
    }
    controller.set_wav_entries_for_tests(vec![
        sample_entry("trash3.wav", Rating::TRASH_3),
        sample_entry("trash2.wav", Rating::new(-2)),
        sample_entry("trash1.wav", Rating::TRASH_1),
        sample_entry("neutral.wav", Rating::NEUTRAL),
        sample_entry("keep1.wav", Rating::KEEP_1),
        sample_entry("keep2.wav", Rating::new(2)),
        sample_entry("keep3.wav", Rating::KEEP_3),
        locked_keep_entry,
    ]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    (controller, source)
}

fn browser_mark_fixture() -> (
    crate::app::controller::AppController,
    crate::sample_sources::SampleSource,
) {
    prepare_with_source_and_wav_entries(vec![
        sample_entry("one.wav", Rating::NEUTRAL),
        sample_entry("two.wav", Rating::NEUTRAL),
        sample_entry("three.wav", Rating::NEUTRAL),
    ])
}

fn write_browser_mark_wavs(source_root: &Path) {
    for name in ["one.wav", "two.wav", "three.wav"] {
        write_test_wav(&source_root.join(name), &[0.0, 0.1]);
    }
}

fn visible_paths(controller: &mut crate::app::controller::AppController) -> Vec<PathBuf> {
    (0..controller.visible_browser_len())
        .filter_map(|row| controller.browser_path_for_visible(row))
        .collect()
}

fn wait_for_waveform_image(controller: &mut crate::app::controller::AppController, path: &Path) {
    for _ in 0..50 {
        controller.poll_background_jobs();
        if controller.sample_view.wav.loaded_wav.as_deref() == Some(path)
            && controller.ui.waveform.loading.is_none()
            && controller.ui.waveform.image.is_some()
        {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
}
