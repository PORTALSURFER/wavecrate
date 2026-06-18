//! App-core-owned projection fixture builders for tests.
//!
//! These helpers keep projection tests focused on behavior while the underlying
//! DTOs are still sourced through `app_api` during migration.

use std::path::PathBuf;

use crate::app_core::state::{
    ActiveAudioOutput, AudioDeviceView, AudioHostView, BrowserDuplicateCleanupState,
    ProgressTaskKind, SampleBrowserIndex, SimilarQuery, TriageFlagColumn, VisibleRows,
};
use crate::sample_sources::SourceId;

/// Build an all-rows browser viewport fixture.
pub(crate) fn visible_rows_all(total: usize) -> VisibleRows {
    VisibleRows::All { total }
}

/// Build a list-backed browser viewport fixture.
pub(crate) fn visible_rows_list(rows: impl IntoIterator<Item = usize>) -> VisibleRows {
    VisibleRows::List(rows.into_iter().collect::<Vec<_>>().into())
}

/// Build a selected browser index fixture.
pub(crate) fn sample_browser_index(column: TriageFlagColumn, row: usize) -> SampleBrowserIndex {
    SampleBrowserIndex { column, row }
}

/// Build an active audio-output fixture.
pub(crate) fn active_audio_output(
    host_id: impl Into<String>,
    device_name: impl Into<String>,
    sample_rate: u32,
    buffer_size_frames: Option<u32>,
    channel_count: u16,
) -> ActiveAudioOutput {
    ActiveAudioOutput {
        host_id: host_id.into(),
        device_name: device_name.into(),
        sample_rate,
        buffer_size_frames,
        channel_count,
    }
}

/// Build an audio-host option fixture.
pub(crate) fn audio_host_view(
    id: impl Into<String>,
    label: impl Into<String>,
    is_default: bool,
) -> AudioHostView {
    AudioHostView {
        id: id.into(),
        label: label.into(),
        is_default,
    }
}

/// Build an audio-device option fixture.
pub(crate) fn audio_device_view(
    host_id: impl Into<String>,
    name: impl Into<String>,
    is_default: bool,
) -> AudioDeviceView {
    AudioDeviceView {
        host_id: host_id.into(),
        name: name.into(),
        is_default,
    }
}

/// Progress task fixture for analysis progress.
pub(crate) fn analysis_progress_task() -> ProgressTaskKind {
    ProgressTaskKind::Analysis
}

/// Progress task fixture for normalization progress.
pub(crate) fn normalization_progress_task() -> ProgressTaskKind {
    ProgressTaskKind::Normalization
}

/// Build a similar-query fixture for browser projection tests.
pub(crate) fn similar_query(
    sample_id: impl Into<String>,
    label: impl Into<String>,
    indices: Vec<usize>,
    scores: Vec<f32>,
    anchor_index: Option<usize>,
) -> SimilarQuery {
    SimilarQuery {
        sample_id: sample_id.into(),
        label: label.into(),
        indices,
        aspect_scores: empty_similarity_aspect_score_rows(scores.len()),
        scores,
        anchor_index,
    }
}

fn empty_similarity_aspect_score_rows(
    len: usize,
) -> Vec<[Option<f32>; wavecrate_analysis::aspects::ASPECT_COUNT]> {
    vec![[None; wavecrate_analysis::aspects::ASPECT_COUNT]; len]
}

/// Build a duplicate-cleanup fixture for browser projection tests.
pub(crate) fn browser_duplicate_cleanup(
    source_id: SourceId,
    sample_id: impl Into<String>,
    anchor_path: impl Into<PathBuf>,
    anchor_label: impl Into<String>,
    candidate_indices: Vec<usize>,
    scores: Vec<f32>,
    anchor_index: usize,
) -> BrowserDuplicateCleanupState {
    BrowserDuplicateCleanupState::new(
        source_id,
        sample_id.into(),
        anchor_path.into(),
        anchor_label.into(),
        candidate_indices,
        scores,
        anchor_index,
    )
}
