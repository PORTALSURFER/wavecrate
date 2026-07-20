use super::*;
pub(crate) use super::{AppController, BPM_MIN_SELECTION_DIVISOR, StatusTone};
pub(crate) use crate::sample_sources::*;
pub(crate) use crate::selection::SelectionRange;

use std::time::Duration;

pub(crate) mod audio_cache;
pub(crate) mod audio_loader;
pub(crate) mod audio_options;
pub(crate) mod audio_samples;
mod bpm_policy;
pub(crate) mod loop_crossfade;
/// Disk-backed waveform cache persistence used to reuse decoded waveforms across app restarts.
pub(crate) mod persistent_waveform_cache;
mod playback_age;
pub(crate) mod recording;

mod browser_nav;
mod compare_anchor;
mod facade_browser;
mod facade_loop;
mod facade_player;
mod facade_transport;
mod facade_volume;
mod facade_waveform;
mod formatting;
mod player;
mod playhead_trail;
mod random_nav;
mod random_nav_facade;
mod tagging;
pub(crate) mod telemetry;
mod transport;
/// Waveform selection/cursor/zoom action facade methods.
mod waveform_actions;

#[cfg(test)]
mod audio_options_tests;
#[cfg(test)]
/// Playback facade behavior tests.
mod tests;
#[cfg(test)]
/// UI waveform action regressions for playback behavior.
mod ui_action_tests;
#[cfg(test)]
/// Waveform action behavior tests.
mod waveform_action_tests;

pub(crate) use crate::ui_formatting::format_selection_duration;
pub(crate) use bpm_policy::{
    bpm_min_selection_seconds, selection_meets_bpm_min_for_playback,
    snap_waveform_delta_to_bpm_step, snap_waveform_micros_to_bpm_anchor, waveform_bpm_snap_step,
};
pub(crate) use compare_anchor::play_loaded_audio_for_path;
use formatting::format_timestamp_hms_ms;

#[cfg(test)]
const SHOULD_PLAY_RANDOM_SAMPLE: bool = false;
#[cfg(not(test))]
const SHOULD_PLAY_RANDOM_SAMPLE: bool = true;
const PLAYHEAD_COMPLETION_EPSILON: f32 = 0.001;
/// Equality epsilon used for waveform view no-op detection.
const WAVEFORM_VIEW_NOOP_EPSILON: f64 = 1.0e-9;
/// Integer precision used for pointer-anchored waveform zoom ratios.
const WAVEFORM_ANCHOR_RATIO_MICROS_SCALE: f64 = 1_000_000.0;
/// Debounce duration for deferred playback-age database writes.
const DEFERRED_PLAYBACK_AGE_COMMIT_DELAY: Duration = Duration::from_millis(160);
