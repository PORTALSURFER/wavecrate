use super::super::super::test_support::{
    prepare_with_source_and_wav_entries, sample_entry, write_test_wav,
};
use crate::app::controller::jobs::{
    ActiveRetainedDeleteResolution, RetainedDeleteBusyEntry, RetainedDeleteResolutionMode,
};
use crate::sample_sources::Rating;
use std::path::PathBuf;

mod retained_recovery;
mod selection_tagging;
mod wav_only_edits;

fn visible_browser_paths(controller: &mut crate::app::controller::AppController) -> Vec<PathBuf> {
    (0..controller.visible_browser_len())
        .filter_map(|row| controller.browser_path_for_visible(row))
        .collect()
}
