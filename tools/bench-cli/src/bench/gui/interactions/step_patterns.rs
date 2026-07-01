//! Deterministic interaction-step helpers shared by GUI benchmark scenarios.

use wavecrate::app_core::actions::{NativeUiAction, NativeWaveformAction};
use wavecrate::app_core::state::{SampleBrowserSort, TriageFlagFilter};

/// Return a deterministic waveform action for a benchmark step index.
pub(in crate::bench::gui) fn waveform_action_for_step(step: usize) -> NativeUiAction {
    match step % 6 {
        0 => NativeUiAction::Waveform(NativeWaveformAction::SeekWaveformPrecise {
            position_nanos: 320_000_000,
        }),
        1 => NativeUiAction::Waveform(NativeWaveformAction::SetWaveformCursorPrecise {
            position_nanos: 480_000_000,
        }),
        2 => NativeUiAction::Waveform(NativeWaveformAction::SetWaveformSelectionRange {
            start_micros: 220_000,
            end_micros: 660_000,
            snap_override: false,
            preserve_view_edge: false,
        }),
        3 => NativeUiAction::Waveform(NativeWaveformAction::ZoomWaveform {
            zoom_in: true,
            steps: 2,
            anchor_ratio_micros: None,
        }),
        4 => NativeUiAction::Waveform(NativeWaveformAction::ZoomWaveformToSelection),
        _ => NativeUiAction::Waveform(NativeWaveformAction::ZoomWaveformFull),
    }
}

/// Return an adjacent pan/zoom waveform action for a benchmark step index.
pub(in crate::bench::gui) fn adjacent_waveform_action_for_step(step: usize) -> NativeUiAction {
    match step % 4 {
        0 => NativeUiAction::Waveform(NativeWaveformAction::SeekWaveformPrecise {
            position_nanos: 380_000_000,
        }),
        1 => NativeUiAction::Waveform(NativeWaveformAction::SeekWaveformPrecise {
            position_nanos: 410_000_000,
        }),
        2 => NativeUiAction::Waveform(NativeWaveformAction::ZoomWaveform {
            zoom_in: true,
            steps: 1,
            anchor_ratio_micros: None,
        }),
        _ => NativeUiAction::Waveform(NativeWaveformAction::ZoomWaveform {
            zoom_in: false,
            steps: 1,
            anchor_ratio_micros: None,
        }),
    }
}

/// Return a deterministic volume slider milli value for one benchmark step.
pub(in crate::bench::gui) fn volume_milli_for_step(step: usize) -> u16 {
    ((step.saturating_mul(37)) % 1001) as u16
}

/// Return a deterministic query string for one benchmark step.
pub(in crate::bench::gui) fn interaction_query_for_step(step: usize) -> &'static str {
    const SEARCH_QUERIES: [&str; 4] = ["sample_", "sample_00", "sample_000", "sample_001"];
    SEARCH_QUERIES[step % SEARCH_QUERIES.len()]
}

/// Return a deterministic triage filter for one benchmark step.
pub(in crate::bench::gui) fn interaction_filter_for_step(step: usize) -> TriageFlagFilter {
    match step % 3 {
        0 => TriageFlagFilter::All,
        1 => TriageFlagFilter::Keep,
        _ => TriageFlagFilter::Trash,
    }
}

/// Return a deterministic browser sort mode for one benchmark step.
pub(in crate::bench::gui) fn interaction_sort_for_step(step: usize) -> SampleBrowserSort {
    if step.is_multiple_of(2) {
        SampleBrowserSort::ListOrder
    } else {
        SampleBrowserSort::PlaybackAgeDesc
    }
}
