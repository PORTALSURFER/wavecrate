use super::super::test_support::{
    dummy_controller, prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
};
use super::common::visible_indices;
use crate::app::state::{TriageFlagColumn, TriageFlagFilter};
use crate::sample_sources::Rating;
use std::path::{Path, PathBuf};

mod filters;
mod loading;
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
