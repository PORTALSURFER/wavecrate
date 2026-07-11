use std::path::PathBuf;

use wavecrate::selection::SelectionRange;

use crate::native_app::waveform::WaveformState;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::native_app) struct PendingPlaySelectionRetargetCycle {
    pub(in crate::native_app) end_ratio: f32,
    pub(in crate::native_app) last_progress_ratio: Option<f32>,
}

impl PendingPlaySelectionRetargetCycle {
    pub(in crate::native_app) fn new(end_ratio: f32, last_progress_ratio: Option<f32>) -> Self {
        Self {
            end_ratio,
            last_progress_ratio,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct WaveformEditSelectionSnapshot {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) edit_selection: Option<SelectionRange>,
}

impl WaveformEditSelectionSnapshot {
    pub(in crate::native_app) fn from_waveform(waveform: &WaveformState) -> Self {
        Self {
            path: waveform.path(),
            edit_selection: waveform.edit_selection(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct WaveformPlaySelectionSnapshot {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) play_mark_ratio: Option<f32>,
    pub(in crate::native_app) play_selection: Option<SelectionRange>,
    pub(in crate::native_app) marked_play_ranges: Vec<SelectionRange>,
}

impl WaveformPlaySelectionSnapshot {
    pub(in crate::native_app) fn from_waveform(waveform: &WaveformState) -> Self {
        Self {
            path: waveform.path(),
            play_mark_ratio: waveform.play_mark_ratio(),
            play_selection: waveform.play_selection(),
            marked_play_ranges: waveform.marked_play_ranges().to_vec(),
        }
    }
}
