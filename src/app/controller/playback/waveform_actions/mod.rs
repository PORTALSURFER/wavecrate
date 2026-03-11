//! Waveform and selection action facade methods for [`AppController`].

use super::*;

mod edit;
mod edit_fades;
mod edit_selection;
mod selection;
mod selection_updates;
mod shared;
mod state;
mod view;

pub(super) use edit_selection::clear_edit_fade_drag;
use shared::bpm_matches;
use state::{apply_edit_selection_update, current_edit_selection, current_playback_selection};

/// Convert one UI waveform milli value (`0..=1000`) into normalized `[0.0, 1.0]`.
pub(in crate::app::controller::playback) fn normalized_from_milli(value: u16) -> f32 {
    shared::normalized_from_milli(value)
}

/// Convert one UI waveform micro value (`0..=1_000_000`) back into normalized space.
pub(in crate::app::controller::playback) fn normalized_from_micros(value: u32) -> f32 {
    shared::normalized_from_micros(value)
}

/// Convert one normalized waveform position into UI micro space (`0..=1_000_000`).
pub(in crate::app::controller::playback) fn normalized_to_micros(value: f32) -> u32 {
    shared::normalized_to_micros(value)
}

/// Convert one UI waveform milli value (`0..=1000`) into micro space.
pub(in crate::app::controller::playback) fn micros_from_milli(value: u16) -> u32 {
    shared::micros_from_milli(value)
}

/// Build a normalized selection range from two UI waveform milli values (`0..=1000`).
pub(in crate::app::controller::playback) fn selection_range_from_milli(
    start_milli: u16,
    end_milli: u16,
) -> SelectionRange {
    shared::selection_range_from_milli(start_milli, end_milli)
}

/// Build a normalized selection range from two UI waveform micro values (`0..=1_000_000`).
pub(in crate::app::controller::playback) fn selection_range_from_micros(
    start_micros: u32,
    end_micros: u32,
) -> SelectionRange {
    shared::selection_range_from_micros(start_micros, end_micros)
}

/// Clamp UI-provided waveform zoom steps to at least one step.
pub(in crate::app::controller::playback) fn zoom_steps_from_ui(steps: u8) -> u32 {
    shared::zoom_steps_from_ui(steps)
}

/// Return whether waveform focus is already active.
pub(in crate::app::controller::playback) fn waveform_focus_active(
    controller: &AppController,
) -> bool {
    shared::waveform_focus_active(controller)
}

/// Return whether two waveform views differ enough to warrant follow-up focus work.
pub(in crate::app::controller::playback) fn waveform_view_changed(
    before: crate::app::state::WaveformView,
    after: crate::app::state::WaveformView,
) -> bool {
    shared::waveform_view_changed(before, after)
}
