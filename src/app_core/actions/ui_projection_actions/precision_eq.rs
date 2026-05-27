use super::UiAction;

impl PartialEq for UiAction {
    fn eq(&self, other: &Self) -> bool {
        waveform_precision_equivalent(self, other)
            || serde_json::to_value(self).ok() == serde_json::to_value(other).ok()
    }
}

impl Eq for UiAction {}

fn waveform_precision_equivalent(left: &UiAction, right: &UiAction) -> bool {
    use UiAction::*;

    match (left, right) {
        (
            BeginWaveformSelectionAt { anchor_micros },
            BeginWaveformSelectionAtPrecise { anchor_nanos },
        )
        | (
            BeginWaveformSelectionAtPrecise { anchor_nanos },
            BeginWaveformSelectionAt { anchor_micros },
        ) => nanos_match_micros(*anchor_nanos, *anchor_micros),
        (
            SetWaveformSelectionRange {
                start_micros,
                end_micros,
                snap_override,
                preserve_view_edge,
            },
            SetWaveformSelectionRangePrecise {
                start_nanos,
                end_nanos,
                snap_override: precise_snap_override,
                preserve_view_edge: precise_preserve_view_edge,
            },
        )
        | (
            SetWaveformSelectionRangePrecise {
                start_nanos,
                end_nanos,
                snap_override: precise_snap_override,
                preserve_view_edge: precise_preserve_view_edge,
            },
            SetWaveformSelectionRange {
                start_micros,
                end_micros,
                snap_override,
                preserve_view_edge,
            },
        ) => {
            snap_override == precise_snap_override
                && preserve_view_edge == precise_preserve_view_edge
                && nanos_match_micros(*start_nanos, *start_micros)
                && nanos_match_micros(*end_nanos, *end_micros)
        }
        (
            SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            },
            SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos,
                end_nanos,
            },
        )
        | (
            SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos,
                end_nanos,
            },
            SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            },
        ) => {
            nanos_match_micros(*start_nanos, *start_micros)
                && nanos_match_micros(*end_nanos, *end_micros)
        }
        (
            BeginWaveformSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            },
            BeginWaveformSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            },
        )
        | (
            BeginWaveformSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            },
            BeginWaveformSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            },
        ) => {
            nanos_match_micros(*pointer_nanos, *pointer_micros)
                && nanos_match_micros(*start_nanos, *start_micros)
                && nanos_match_micros(*end_nanos, *end_micros)
        }
        (
            SetWaveformEditSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge,
            },
            SetWaveformEditSelectionRangePrecise {
                start_nanos,
                end_nanos,
                preserve_view_edge: precise_preserve_view_edge,
            },
        )
        | (
            SetWaveformEditSelectionRangePrecise {
                start_nanos,
                end_nanos,
                preserve_view_edge: precise_preserve_view_edge,
            },
            SetWaveformEditSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge,
            },
        ) => {
            preserve_view_edge == precise_preserve_view_edge
                && nanos_match_micros(*start_nanos, *start_micros)
                && nanos_match_micros(*end_nanos, *end_micros)
        }
        (
            BeginWaveformEditSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            },
            BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            },
        )
        | (
            BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            },
            BeginWaveformEditSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            },
        ) => {
            nanos_match_micros(*pointer_nanos, *pointer_micros)
                && nanos_match_micros(*start_nanos, *start_micros)
                && nanos_match_micros(*end_nanos, *end_micros)
        }
        _ => false,
    }
}

fn nanos_match_micros(nanos: u32, micros: u32) -> bool {
    nanos == micros
        || ((nanos.min(1_000_000_000) + 500) / 1000).min(1_000_000) == micros.min(1_000_000)
}
