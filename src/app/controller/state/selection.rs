//! Selection and waveform view state for the controller.

use super::audio::LoadedAudio;
use crate::app::controller::library::wavs;
use crate::sample_sources::{SampleSource, SourceId};
use crate::selection::SelectionRange;
use crate::selection::SelectionState;
use crate::waveform::{DecodedWaveform, WaveformRenderer};
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) struct WavSelectionState {
    pub(crate) selected_wav: Option<PathBuf>,
    pub(crate) loaded_wav: Option<PathBuf>,
    pub(crate) loaded_audio: Option<LoadedAudio>,
}

impl WavSelectionState {
    pub(crate) fn new() -> Self {
        Self {
            selected_wav: None,
            loaded_wav: None,
            loaded_audio: None,
        }
    }
}

pub(crate) struct ControllerSampleViewState {
    pub(crate) renderer: WaveformRenderer,
    pub(crate) waveform: WaveformState,
    pub(crate) waveform_slide: Option<WaveformSlideState>,
    pub(crate) wav: WavSelectionState,
}

impl ControllerSampleViewState {
    pub(crate) fn new(renderer: WaveformRenderer) -> Self {
        let (waveform_width, waveform_height) = renderer.dimensions();
        Self {
            renderer,
            waveform: WaveformState {
                size: [waveform_width, waveform_height],
                decoded: None,
                render_meta: None,
            },
            waveform_slide: None,
            wav: WavSelectionState::new(),
        }
    }
}

/// Cached state for a circular waveform slide drag.
pub(crate) struct WaveformSlideState {
    pub(crate) source: SampleSource,
    pub(crate) relative_path: PathBuf,
    pub(crate) absolute_path: PathBuf,
    pub(crate) original_samples: Vec<f32>,
    /// Optional preview buffer (e.g. stretched audition) used to keep the render stable.
    pub(crate) preview: Option<WaveformSlidePreview>,
    pub(crate) channels: usize,
    pub(crate) spec_channels: u16,
    pub(crate) sample_rate: u32,
    pub(crate) start_normalized: f32,
    pub(crate) last_offset_frames: isize,
    pub(crate) last_preview_offset_frames: isize,
}

/// Cached waveform preview used during circular slide gestures.
pub(crate) struct WaveformSlidePreview {
    pub(crate) samples: Vec<f32>,
    pub(crate) channels: u16,
    pub(crate) sample_rate: u32,
}

pub(crate) struct SelectionContextState {
    pub(crate) selected_source: Option<SourceId>,
    pub(crate) last_selected_browsable_source: Option<SourceId>,
}

impl SelectionContextState {
    pub(crate) fn new() -> Self {
        Self {
            selected_source: None,
            last_selected_browsable_source: None,
        }
    }
}

pub(crate) struct SelectionUndoState {
    pub(crate) label: String,
    pub(crate) before: Option<SelectionRange>,
}

/// Identifies which edit-fade handle is being dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditFadeDragKind {
    /// Dragging the fade-in end handle.
    InEnd,
    /// Dragging the fade-in bottom handle.
    InMuteStart,
    /// Dragging the fade-in curve control.
    InCurve,
    /// Dragging the fade-out start handle.
    OutStart,
    /// Dragging the fade-out bottom handle.
    OutMuteEnd,
    /// Dragging the fade-out curve control.
    OutCurve,
}

/// Retains the pre-drag edit-selection range while an edit-fade handle moves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct EditFadeDragState {
    /// Drag handle currently driving edit-fade updates.
    pub(crate) kind: EditFadeDragKind,
    /// Edit-selection state captured when the drag began.
    pub(crate) baseline: SelectionRange,
}

pub(crate) struct ControllerSelectionState {
    pub(crate) ctx: SelectionContextState,
    pub(crate) range: SelectionState,
    pub(crate) edit_range: SelectionState,
    pub(crate) edit_fade_drag: Option<EditFadeDragState>,
    pub(crate) pending_undo: Option<SelectionUndoState>,
    pub(crate) pending_edit_undo: Option<SelectionUndoState>,
    pub(crate) suppress_autoplay_once: bool,
    /// Fixed beat count used while a smart-scale resize drag recalculates BPM.
    pub(crate) bpm_scale_beats: Option<f32>,
}

impl ControllerSelectionState {
    pub(crate) fn new() -> Self {
        Self {
            ctx: SelectionContextState::new(),
            range: SelectionState::new(),
            edit_range: SelectionState::new(),
            edit_fade_drag: None,
            pending_undo: None,
            pending_edit_undo: None,
            suppress_autoplay_once: false,
            bpm_scale_beats: None,
        }
    }
}

pub(crate) struct WaveformState {
    pub(crate) size: [u32; 2],
    pub(crate) decoded: Option<Arc<DecodedWaveform>>,
    pub(crate) render_meta: Option<wavs::WaveformRenderMeta>,
}
