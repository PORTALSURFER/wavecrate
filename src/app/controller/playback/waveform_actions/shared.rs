//! Shared waveform-action value conversions and small UI helpers.

use super::*;

/// Equality epsilon used for smart-scale BPM no-op detection.
const WAVEFORM_BPM_NOOP_EPSILON: f32 = 1.0e-3;

/// Convert one UI waveform milli value (`0..=1000`) into normalized `[0.0, 1.0]`.
pub(in crate::app::controller::playback) fn normalized_from_milli(value: u16) -> f32 {
    (value.min(1000) as f32) / 1000.0
}

/// Convert one normalized waveform position into UI milli space (`0..=1000`).
pub(in crate::app::controller::playback) fn normalized_to_milli(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

/// Convert one normalized waveform position into UI micro space (`0..=1_000_000`).
pub(in crate::app::controller::playback) fn normalized_to_micros(value: f32) -> u32 {
    (value.clamp(0.0, 1.0) * 1_000_000.0).round() as u32
}

/// Convert one UI waveform micro value (`0..=1_000_000`) back into normalized space.
pub(in crate::app::controller::playback) fn normalized_from_micros(value: u32) -> f32 {
    (value.min(1_000_000) as f32) / 1_000_000.0
}

/// Convert one UI waveform milli value (`0..=1000`) into micro space.
pub(in crate::app::controller::playback) fn micros_from_milli(value: u16) -> u32 {
    u32::from(value.min(1000)) * 1000
}

/// Build a normalized selection range from two UI waveform milli values (`0..=1000`).
pub(in crate::app::controller::playback) fn selection_range_from_milli(
    start_milli: u16,
    end_milli: u16,
) -> SelectionRange {
    selection_range_from_micros(micros_from_milli(start_milli), micros_from_milli(end_milli))
}

/// Build a normalized selection range from two UI waveform micro values (`0..=1_000_000`).
pub(in crate::app::controller::playback) fn selection_range_from_micros(
    start_micros: u32,
    end_micros: u32,
) -> SelectionRange {
    SelectionRange::new(
        normalized_from_micros(start_micros),
        normalized_from_micros(end_micros),
    )
}

/// Clamp UI-provided waveform zoom steps to at least one step.
pub(in crate::app::controller::playback) fn zoom_steps_from_ui(steps: u8) -> u32 {
    u32::from(steps.max(1))
}

/// Return whether waveform focus is already active.
pub(in crate::app::controller::playback) fn waveform_focus_active(
    controller: &AppController,
) -> bool {
    controller.ui.focus.context == crate::app::state::FocusContext::Waveform
}

/// Return whether two waveform views differ enough to warrant follow-up focus work.
pub(in crate::app::controller::playback) fn waveform_view_changed(
    before: crate::app::state::WaveformView,
    after: crate::app::state::WaveformView,
) -> bool {
    (before.start - after.start).abs() > WAVEFORM_VIEW_NOOP_EPSILON
        || (before.end - after.end).abs() > WAVEFORM_VIEW_NOOP_EPSILON
}

/// Return whether two optional BPM values are equal within smart-scale drag tolerance.
pub(in crate::app::controller::playback::waveform_actions) fn bpm_matches(
    current: Option<f32>,
    next: Option<f32>,
) -> bool {
    match (current, next) {
        (Some(current), Some(next)) => (current - next).abs() <= WAVEFORM_BPM_NOOP_EPSILON,
        (None, None) => true,
        _ => false,
    }
}
