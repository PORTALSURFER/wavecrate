#![allow(missing_docs)]

use radiant::prelude as ui;
use std::sync::Arc;

use crate::native_app::ui::ids as widget_ids;

const WAVEFORM_WIDTH: usize = 1200;
const WAVEFORM_HEIGHT: usize = 320;
pub(in crate::native_app) const WAVEFORM_SIGNAL_WIDGET_ID: u64 =
    widget_ids::WAVEFORM_SIGNAL_WIDGET_ID;
pub(in crate::native_app) const WAVEFORM_WIDGET_ID: u64 = widget_ids::WAVEFORM_WIDGET_ID;
const MIN_VISIBLE_FRAMES: usize = 256;
const BAND_COUNT: usize = 4;
const SELECTION_DRAG_EPSILON: f32 = 0.001;
const SELECTION_FLASH_FRAMES: u8 = 12;
#[cfg(test)]
const SYNTHETIC_SAMPLE_RATE: u32 = 48_000;
#[cfg(test)]
const SYNTHETIC_SECONDS: usize = 1;

mod types;
pub(super) use types::{
    WaveformActiveDragKind, WaveformEditFadeHandle, WaveformInteraction, WaveformSelectionEdge,
    WaveformSelectionKind,
};

mod interaction;
use interaction::{WaveformDrag, edit_preview_for_selection};

mod state_extraction;
mod state_file;
mod state_interaction;
mod state_loading;
mod state_marked_ranges;
mod state_playback;
mod state_selection;
mod state_transient;
mod state_viewport;
mod state_viewport_access;
#[cfg(test)]
pub(in crate::native_app) use state_marked_ranges::random_marked_play_range_for_unit;

mod audio_file;
pub(super) use audio_file::WaveformFile;
#[cfg(test)]
pub(super) use audio_file::store_cached_waveform_file_for_tests;
#[cfg(test)]
pub(super) use audio_file::store_summary_only_cached_waveform_file_for_tests;
#[cfg(test)]
pub(super) use audio_file::test_waveform_file_from_mono_samples;
pub(in crate::native_app) use audio_file::{
    WaveformPlaybackReady, cached_waveform_file_exists, cached_waveform_file_playback_ready_exists,
    flush_background_waveform_cache_stores_for_shutdown, load_cached_waveform_file_for_playback,
};
#[cfg(test)]
use audio_file::{downmix_to_mono, split_frequency_bands, waveform_file_from_mono_samples};

mod widget;
#[cfg(test)]
pub(super) use widget::WaveformWidgetProps;
#[cfg(test)]
pub(in crate::native_app::waveform) use widget::waveform_signal_surface_view;
pub(super) use widget::{WaveformWidget, waveform_viewport_view};

mod widget_geometry;
mod widget_input;

mod edit_fade_curve_paint;
mod edit_fade_geometry;
mod edit_fade_paint;
mod selection_paint;

pub(super) type WaveformViewport = ui::IndexViewport;

#[derive(Clone, Debug)]
pub(super) struct WaveformState {
    file: Arc<WaveformFile>,
    viewport: WaveformViewport,
    zoom_anchor_ratio: f32,
    playing: bool,
    playhead_ratio: Option<f32>,
    play_mark_ratio: Option<f32>,
    edit_mark_ratio: Option<f32>,
    play_selection: Option<wavecrate::selection::SelectionRange>,
    edit_selection: Option<wavecrate::selection::SelectionRange>,
    marked_play_ranges: Vec<wavecrate::selection::SelectionRange>,
    extracted_ranges: Vec<wavecrate::selection::SelectionRange>,
    play_selection_flash_frames: u8,
    active_drag: Option<WaveformDrag>,
    pending_playback_start: Option<f32>,
}

#[cfg(test)]
mod tests;
