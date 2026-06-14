use super::*;
use crate::analysis::audio::DetectedDuplicateWindow;
use crate::app::state::{WaveformDuplicateCleanupPreview, WaveformDuplicateCleanupState};

mod detection_workflow;
mod exemption;
mod inputs;
mod preview_review;

#[derive(Clone, Copy, Default)]
pub(crate) struct DuplicateCleanupCounts {
    pub(crate) group_count: usize,
    pub(crate) marked_windows: usize,
    pub(crate) exempted_windows: usize,
}

fn build_duplicate_cleanup_state(
    windows: &[DetectedDuplicateWindow],
    duplicate_group_count: usize,
    total_frames: usize,
) -> WaveformDuplicateCleanupState {
    WaveformDuplicateCleanupState {
        group_count: duplicate_group_count,
        previews: windows
            .iter()
            .map(|window| WaveformDuplicateCleanupPreview {
                range: SelectionRange::new(
                    window.start_frame as f32 / total_frames as f32,
                    window.end_frame as f32 / total_frames as f32,
                ),
                group_id: window.group_id,
                exempted: false,
                represented_window_count: 1,
            })
            .collect(),
    }
}
