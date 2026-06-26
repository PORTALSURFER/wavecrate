use std::sync::Arc;

use wavecrate::selection::SelectionRange;

use super::{WaveformDrag, WaveformFile, WaveformViewport, similar_sections::SimilarSectionsState};

#[derive(Clone, Debug)]
pub(in crate::native_app) struct WaveformState {
    pub(in crate::native_app::waveform) file: Arc<WaveformFile>,
    pub(in crate::native_app::waveform) viewport: WaveformViewport,
    pub(in crate::native_app::waveform) zoom_anchor_ratio: f32,
    pub(in crate::native_app::waveform) playing: bool,
    pub(in crate::native_app::waveform) playhead_ratio: Option<f32>,
    pub(in crate::native_app::waveform) play_mark_ratio: Option<f32>,
    pub(in crate::native_app::waveform) edit_mark_ratio: Option<f32>,
    pub(in crate::native_app::waveform) play_selection: Option<SelectionRange>,
    pub(in crate::native_app::waveform) edit_selection: Option<SelectionRange>,
    pub(in crate::native_app::waveform) zero_crossing_snap_enabled: bool,
    pub(in crate::native_app::waveform) marked_play_ranges: Vec<SelectionRange>,
    pub(in crate::native_app::waveform) extracted_ranges: Vec<SelectionRange>,
    pub(in crate::native_app::waveform) similar_sections: SimilarSectionsState,
    pub(in crate::native_app::waveform) play_selection_flash_frames: u8,
    pub(in crate::native_app::waveform) edit_selection_flash_frames: u8,
    pub(in crate::native_app::waveform) play_selection_denied_flash_frames: u8,
    pub(in crate::native_app::waveform) edit_selection_denied_flash_frames: u8,
    pub(in crate::native_app::waveform) active_drag: Option<WaveformDrag>,
    pub(in crate::native_app::waveform) pending_playback_start: Option<f32>,
}
