use super::super::*;

pub(super) fn generic_to_product(
    value: runtime_contract::UiAction,
) -> Result<UiAction, runtime_contract::UiAction> {
    match value {
        runtime_contract::UiAction::BeginWaveformSelectionAt { anchor_micros } => {
            Ok(UiAction::BeginWaveformSelectionAt { anchor_micros })
        }
        runtime_contract::UiAction::BeginWaveformSelectionAtPrecise { anchor_nanos } => {
            Ok(UiAction::BeginWaveformSelectionAtPrecise { anchor_nanos })
        }
        runtime_contract::UiAction::SetWaveformSelectionRange {
            start_micros,
            end_micros,
            snap_override,
            preserve_view_edge,
        } => Ok(UiAction::SetWaveformSelectionRange {
            start_micros,
            end_micros,
            snap_override,
            preserve_view_edge,
        }),
        runtime_contract::UiAction::SetWaveformSelectionRangePrecise {
            start_nanos,
            end_nanos,
            snap_override,
            preserve_view_edge,
        } => Ok(UiAction::SetWaveformSelectionRangePrecise {
            start_nanos,
            end_nanos,
            snap_override,
            preserve_view_edge,
        }),
        runtime_contract::UiAction::SetWaveformSelectionRangeSmartScale {
            start_micros,
            end_micros,
        } => Ok(UiAction::SetWaveformSelectionRangeSmartScale {
            start_micros,
            end_micros,
        }),
        runtime_contract::UiAction::SetWaveformSelectionRangeSmartScalePrecise {
            start_nanos,
            end_nanos,
        } => Ok(UiAction::SetWaveformSelectionRangeSmartScalePrecise {
            start_nanos,
            end_nanos,
        }),
        runtime_contract::UiAction::SetWaveformEditSelectionRange {
            start_micros,
            end_micros,
            preserve_view_edge,
        } => Ok(UiAction::SetWaveformEditSelectionRange {
            start_micros,
            end_micros,
            preserve_view_edge,
        }),
        runtime_contract::UiAction::SetWaveformEditSelectionRangePrecise {
            start_nanos,
            end_nanos,
            preserve_view_edge,
        } => Ok(UiAction::SetWaveformEditSelectionRangePrecise {
            start_nanos,
            end_nanos,
            preserve_view_edge,
        }),
        runtime_contract::UiAction::SetWaveformEditFadeInEnd { position_micros } => {
            Ok(UiAction::SetWaveformEditFadeInEnd { position_micros })
        }
        runtime_contract::UiAction::SetWaveformEditFadeInMuteStart { position_micros } => {
            Ok(UiAction::SetWaveformEditFadeInMuteStart { position_micros })
        }
        runtime_contract::UiAction::SetWaveformEditFadeInCurve { curve_milli } => {
            Ok(UiAction::SetWaveformEditFadeInCurve { curve_milli })
        }
        runtime_contract::UiAction::SetWaveformEditFadeOutStart { position_micros } => {
            Ok(UiAction::SetWaveformEditFadeOutStart { position_micros })
        }
        runtime_contract::UiAction::SetWaveformEditFadeOutMuteEnd { position_micros } => {
            Ok(UiAction::SetWaveformEditFadeOutMuteEnd { position_micros })
        }
        runtime_contract::UiAction::SetWaveformEditFadeOutCurve { curve_milli } => {
            Ok(UiAction::SetWaveformEditFadeOutCurve { curve_milli })
        }
        runtime_contract::UiAction::FinishWaveformEditFadeDrag => {
            Ok(UiAction::FinishWaveformEditFadeDrag)
        }
        runtime_contract::UiAction::ClearWaveformSelection => Ok(UiAction::ClearWaveformSelection),
        runtime_contract::UiAction::ClearWaveformEditSelection => {
            Ok(UiAction::ClearWaveformEditSelection)
        }
        runtime_contract::UiAction::ClearWaveformSelections => {
            Ok(UiAction::ClearWaveformSelections)
        }
        other => Err(other),
    }
}

pub(super) fn product_to_generic(value: UiAction) -> Result<runtime_contract::UiAction, UiAction> {
    match value {
        UiAction::BeginWaveformSelectionAt { anchor_micros } => {
            Ok(runtime_contract::UiAction::BeginWaveformSelectionAt { anchor_micros })
        }
        UiAction::BeginWaveformSelectionAtPrecise { anchor_nanos } => {
            Ok(runtime_contract::UiAction::BeginWaveformSelectionAtPrecise { anchor_nanos })
        }
        UiAction::SetWaveformSelectionRange {
            start_micros,
            end_micros,
            snap_override,
            preserve_view_edge,
        } => Ok(runtime_contract::UiAction::SetWaveformSelectionRange {
            start_micros,
            end_micros,
            snap_override,
            preserve_view_edge,
        }),
        UiAction::SetWaveformSelectionRangePrecise {
            start_nanos,
            end_nanos,
            snap_override,
            preserve_view_edge,
        } => Ok(
            runtime_contract::UiAction::SetWaveformSelectionRangePrecise {
                start_nanos,
                end_nanos,
                snap_override,
                preserve_view_edge,
            },
        ),
        UiAction::SetWaveformSelectionRangeSmartScale {
            start_micros,
            end_micros,
        } => Ok(
            runtime_contract::UiAction::SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            },
        ),
        UiAction::SetWaveformSelectionRangeSmartScalePrecise {
            start_nanos,
            end_nanos,
        } => Ok(
            runtime_contract::UiAction::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos,
                end_nanos,
            },
        ),
        UiAction::SetWaveformEditSelectionRange {
            start_micros,
            end_micros,
            preserve_view_edge,
        } => Ok(runtime_contract::UiAction::SetWaveformEditSelectionRange {
            start_micros,
            end_micros,
            preserve_view_edge,
        }),
        UiAction::SetWaveformEditSelectionRangePrecise {
            start_nanos,
            end_nanos,
            preserve_view_edge,
        } => Ok(
            runtime_contract::UiAction::SetWaveformEditSelectionRangePrecise {
                start_nanos,
                end_nanos,
                preserve_view_edge,
            },
        ),
        UiAction::SetWaveformEditFadeInEnd { position_micros } => {
            Ok(runtime_contract::UiAction::SetWaveformEditFadeInEnd { position_micros })
        }
        UiAction::SetWaveformEditFadeInMuteStart { position_micros } => {
            Ok(runtime_contract::UiAction::SetWaveformEditFadeInMuteStart { position_micros })
        }
        UiAction::SetWaveformEditFadeInCurve { curve_milli } => {
            Ok(runtime_contract::UiAction::SetWaveformEditFadeInCurve { curve_milli })
        }
        UiAction::SetWaveformEditFadeOutStart { position_micros } => {
            Ok(runtime_contract::UiAction::SetWaveformEditFadeOutStart { position_micros })
        }
        UiAction::SetWaveformEditFadeOutMuteEnd { position_micros } => {
            Ok(runtime_contract::UiAction::SetWaveformEditFadeOutMuteEnd { position_micros })
        }
        UiAction::SetWaveformEditFadeOutCurve { curve_milli } => {
            Ok(runtime_contract::UiAction::SetWaveformEditFadeOutCurve { curve_milli })
        }
        UiAction::FinishWaveformEditFadeDrag => {
            Ok(runtime_contract::UiAction::FinishWaveformEditFadeDrag)
        }
        UiAction::ClearWaveformSelection => Ok(runtime_contract::UiAction::ClearWaveformSelection),
        UiAction::ClearWaveformEditSelection => {
            Ok(runtime_contract::UiAction::ClearWaveformEditSelection)
        }
        UiAction::ClearWaveformSelections => {
            Ok(runtime_contract::UiAction::ClearWaveformSelections)
        }
        other => Err(other),
    }
}
