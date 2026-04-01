use super::super::test_support::{
    dummy_controller, prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
};
use crate::app::controller::AppController;
use crate::app::controller::state::history::RandomHistoryEntry;
use crate::app::state::TriageFlagFilter;
use crate::sample_sources::{Rating, SampleSource, WavEntry};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

fn set_single_selected_entry(controller: &mut AppController, name: &str, rating: Rating) {
    controller.set_wav_entries_for_tests(vec![sample_entry(name, rating)]);
    controller.rebuild_wav_lookup();
    controller.rebuild_browser_lists();
    controller.sample_view.wav.selected_wav = Some(PathBuf::from(name));
}

fn persisted_row(controller: &mut AppController, source: &SampleSource, name: &str) -> WavEntry {
    controller
        .database_for(source)
        .unwrap()
        .list_files()
        .unwrap()
        .into_iter()
        .find(|row| row.relative_path.to_string_lossy() == name)
        .unwrap_or_else(|| panic!("missing row {name}"))
}

fn wait_for_loaded_waveform(controller: &mut AppController, relative_path: &Path) {
    for _ in 0..50 {
        controller.poll_background_jobs();
        if controller.sample_view.wav.loaded_wav.as_deref() == Some(relative_path)
            && controller.ui.waveform.loading.is_none()
            && controller.ui.waveform.image.is_some()
        {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

mod random_navigation;
mod rating_transitions;
mod undo_behavior;
