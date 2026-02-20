//! Deterministic interaction-step helpers shared by GUI benchmark scenarios.

use sempal::app_core::actions::NativeUiAction;
use sempal::app_core::state::{SampleBrowserSort, TriageFlagFilter};

/// Return a deterministic waveform action for a benchmark step index.
pub(in crate::bench::gui) fn waveform_action_for_step(step: usize) -> NativeUiAction {
    match step % 6 {
        0 => NativeUiAction::SeekWaveform {
            position_milli: 320,
        },
        1 => NativeUiAction::SetWaveformCursor {
            position_milli: 480,
        },
        2 => NativeUiAction::SetWaveformSelectionRange {
            start_milli: 220,
            end_milli: 660,
        },
        3 => NativeUiAction::ZoomWaveform {
            zoom_in: true,
            steps: 2,
        },
        4 => NativeUiAction::ZoomWaveformToSelection,
        _ => NativeUiAction::ZoomWaveformFull,
    }
}

/// Return an adjacent pan/zoom waveform action for a benchmark step index.
pub(in crate::bench::gui) fn adjacent_waveform_action_for_step(step: usize) -> NativeUiAction {
    match step % 4 {
        0 => NativeUiAction::SeekWaveform {
            position_milli: 380,
        },
        1 => NativeUiAction::SeekWaveform {
            position_milli: 410,
        },
        2 => NativeUiAction::ZoomWaveform {
            zoom_in: true,
            steps: 1,
        },
        _ => NativeUiAction::ZoomWaveform {
            zoom_in: false,
            steps: 1,
        },
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
