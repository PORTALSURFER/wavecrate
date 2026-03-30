use super::*;
use crate::app::controller::history::PendingHistoryTransactionKey;
use crate::app::controller::jobs::{
    FileOpResult, JobMessage, SelectionCropExportSuccess, SelectionExportAudioPayload,
    SelectionExportMessage, SelectionExportPlaybackState, SelectionExportResult,
    SelectionExportTimings, UndoFileJob, UndoFileOpResult, UndoFileOutcome,
};
use crate::app::controller::library::analysis_jobs;
use crate::app::controller::test_support::write_test_wav;
use crate::app::state::{FocusContext, ProgressTaskKind, WaveformSliceBatchProfile};
use crate::app_core::state::StatusTone;
use crate::sample_sources::Rating;
use crate::waveform::{DecodedWaveform, WaveformPeaks, next_cache_token};
use hound::WavReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::tempdir;

fn pump_background_jobs_until(
    controller: &mut AppController,
    mut predicate: impl FnMut(&mut AppController) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        controller.poll_background_jobs();
        if predicate(controller) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!(
        "timed out waiting for background job condition; status='{}' tone={:?}",
        controller.ui.status.text, controller.ui.status.status_tone
    );
}

fn written_entry(root: &Path, relative_path: &Path, tag: Rating) -> WavEntry {
    let metadata = std::fs::metadata(root.join(relative_path)).expect("selection export fixture");
    let modified_ns = metadata
        .modified()
        .expect("modified time")
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .expect("after epoch")
        .as_nanos() as i64;
    WavEntry {
        relative_path: relative_path.to_path_buf(),
        file_size: metadata.len(),
        modified_ns,
        content_hash: None,
        tag,
        looped: false,
        locked: false,
        missing: false,
        last_played_at: None,
    }
}

mod audio_payload_tests;
mod clip_export_tests;
mod crop_export_history_tests;
mod slice_batch_export_tests;
mod waveform_selection_export_tests;
