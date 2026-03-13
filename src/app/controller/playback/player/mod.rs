use super::*;

/// Playback/player lifecycle helpers grouped by transport, playhead, and waveform sync.
mod lifecycle;
/// Transport-facing playback start/progress helpers.
mod playback_start;
/// Playhead/runtime follow-up helpers.
mod playhead;
/// Waveform/UI state synchronization helpers driven by playback changes.
mod waveform_ui;

/// Smallest normalized selection width treated as an intentional playback span.
///
/// Zero-width click markers are represented as selection ranges where
/// `start == end`; those should not clamp non-loop playback to an instant blip.
const PLAYBACK_SELECTION_MIN_WIDTH: f32 = 1.0e-6;

pub(crate) use lifecycle::{
    defer_loop_disable_after_cycle, defer_loop_retarget_after_cycle, ensure_player,
};
pub(crate) use playback_start::{is_playing, live_progress, play_audio};
#[cfg(test)]
pub(crate) use playhead::update_playhead_from_progress;
pub(crate) use playhead::{hide_waveform_playhead, tick_playhead};
#[cfg(test)]
pub(crate) use playhead::{hide_waveform_playhead_for_tests, playhead_completed_span_for_tests};
#[cfg(test)]
pub(crate) use waveform_ui::selection_duration_label;
pub(crate) use waveform_ui::{
    apply_edit_selection, apply_selection, apply_volume, update_waveform_hover_time,
};
