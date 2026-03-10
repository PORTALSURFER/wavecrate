//! Shared projection-key encoding utilities used by native-bridge key builders.
//!
//! This module centralizes stable enum-to-byte encodings and normalized scalar
//! conversion so all projection keys use the same representation.

use crate::app_core::app_api::state::FocusContext;
use crate::app_core::state::{SampleBrowserSort, SampleBrowserTab, TriageFlagFilter, UpdateStatus};

/// Encode browser triage filter state into a stable byte for key comparisons.
pub(super) const fn encode_browser_filter(filter: TriageFlagFilter) -> u8 {
    match filter {
        TriageFlagFilter::All => 0,
        TriageFlagFilter::Keep => 1,
        TriageFlagFilter::Trash => 2,
        TriageFlagFilter::Untagged => 3,
    }
}

/// Encode browser sort mode into a stable byte for key comparisons.
pub(super) const fn encode_browser_sort(sort: SampleBrowserSort) -> u8 {
    match sort {
        SampleBrowserSort::ListOrder => 0,
        SampleBrowserSort::Similarity => 1,
        SampleBrowserSort::PlaybackAgeAsc => 2,
        SampleBrowserSort::PlaybackAgeDesc => 3,
    }
}

/// Encode browser active-tab state into a stable byte for key comparisons.
pub(super) const fn encode_browser_tab(tab: SampleBrowserTab) -> u8 {
    match tab {
        SampleBrowserTab::List => 0,
        SampleBrowserTab::Map => 1,
    }
}

/// Encode update status into a stable byte for key comparisons.
pub(super) const fn encode_update_status(status: &UpdateStatus) -> u8 {
    match status {
        UpdateStatus::Idle => 0,
        UpdateStatus::Checking => 1,
        UpdateStatus::UpdateAvailable => 2,
        UpdateStatus::Error => 3,
    }
}

/// Encode focus context into a stable byte for key comparisons.
pub(super) const fn encode_focus_context(context: FocusContext) -> u8 {
    match context {
        FocusContext::None => 0,
        FocusContext::Waveform => 1,
        FocusContext::SampleBrowser => 2,
        FocusContext::SourceFolders => 3,
        FocusContext::SourcesList => 4,
    }
}

/// Convert a normalized `f32` scalar into clamped milli-space (`0..=1000`).
pub(super) fn normalized_f32_to_milli(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

/// Convert a normalized `f64` scalar into clamped milli-space (`0..=1000`).
pub(super) fn normalized_f64_to_milli(value: f64) -> u16 {
    (value.clamp(0.0, 1.0) * 1000.0).round() as u16
}

/// Convert a normalized `f32` scalar into clamped micro-space (`0..=1_000_000`).
pub(super) fn normalized_f32_to_micros(value: f32) -> u32 {
    (value.clamp(0.0, 1.0) * 1_000_000.0).round() as u32
}

/// Convert a normalized `f64` scalar into clamped micro-space (`0..=1_000_000`).
pub(super) fn normalized_f64_to_micros(value: f64) -> u32 {
    (value.clamp(0.0, 1.0) * 1_000_000.0).round() as u32
}
