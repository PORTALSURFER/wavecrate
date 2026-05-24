use super::super::*;

pub(super) fn generic_to_product(
    value: runtime_contract::UiAction,
) -> Result<UiAction, runtime_contract::UiAction> {
    match value {
        runtime_contract::UiAction::SeekWaveformPrecise { position_nanos } => {
            Ok(UiAction::SeekWaveformPrecise { position_nanos })
        }
        runtime_contract::UiAction::SetWaveformCursorPrecise { position_nanos } => {
            Ok(UiAction::SetWaveformCursorPrecise { position_nanos })
        }
        runtime_contract::UiAction::SeekWaveform { position_milli } => {
            Ok(UiAction::SeekWaveform { position_milli })
        }
        runtime_contract::UiAction::SetWaveformCursor { position_milli } => {
            Ok(UiAction::SetWaveformCursor { position_milli })
        }
        runtime_contract::UiAction::BeginWaveformCircularSlide { anchor_micros } => {
            Ok(UiAction::BeginWaveformCircularSlide { anchor_micros })
        }
        runtime_contract::UiAction::UpdateWaveformCircularSlide { position_micros } => {
            Ok(UiAction::UpdateWaveformCircularSlide { position_micros })
        }
        runtime_contract::UiAction::FinishWaveformCircularSlide => {
            Ok(UiAction::FinishWaveformCircularSlide)
        }
        runtime_contract::UiAction::SetWaveformViewCenter {
            center_micros,
            center_nanos,
        } => Ok(UiAction::SetWaveformViewCenter {
            center_micros,
            center_nanos,
        }),
        runtime_contract::UiAction::ZoomWaveform {
            zoom_in,
            steps,
            anchor_ratio_micros,
        } => Ok(UiAction::ZoomWaveform {
            zoom_in,
            steps,
            anchor_ratio_micros,
        }),
        runtime_contract::UiAction::ZoomWaveformToSelection => {
            Ok(UiAction::ZoomWaveformToSelection)
        }
        runtime_contract::UiAction::ZoomWaveformFull => Ok(UiAction::ZoomWaveformFull),
        other => Err(other),
    }
}

pub(super) fn product_to_generic(value: UiAction) -> Result<runtime_contract::UiAction, UiAction> {
    match value {
        UiAction::SeekWaveformPrecise { position_nanos } => {
            Ok(runtime_contract::UiAction::SeekWaveformPrecise { position_nanos })
        }
        UiAction::SetWaveformCursorPrecise { position_nanos } => {
            Ok(runtime_contract::UiAction::SetWaveformCursorPrecise { position_nanos })
        }
        UiAction::SeekWaveform { position_milli } => {
            Ok(runtime_contract::UiAction::SeekWaveform { position_milli })
        }
        UiAction::SetWaveformCursor { position_milli } => {
            Ok(runtime_contract::UiAction::SetWaveformCursor { position_milli })
        }
        UiAction::BeginWaveformCircularSlide { anchor_micros } => {
            Ok(runtime_contract::UiAction::BeginWaveformCircularSlide { anchor_micros })
        }
        UiAction::UpdateWaveformCircularSlide { position_micros } => {
            Ok(runtime_contract::UiAction::UpdateWaveformCircularSlide { position_micros })
        }
        UiAction::FinishWaveformCircularSlide => {
            Ok(runtime_contract::UiAction::FinishWaveformCircularSlide)
        }
        UiAction::SetWaveformViewCenter {
            center_micros,
            center_nanos,
        } => Ok(runtime_contract::UiAction::SetWaveformViewCenter {
            center_micros,
            center_nanos,
        }),
        UiAction::ZoomWaveform {
            zoom_in,
            steps,
            anchor_ratio_micros,
        } => Ok(runtime_contract::UiAction::ZoomWaveform {
            zoom_in,
            steps,
            anchor_ratio_micros,
        }),
        UiAction::ZoomWaveformToSelection => {
            Ok(runtime_contract::UiAction::ZoomWaveformToSelection)
        }
        UiAction::ZoomWaveformFull => Ok(runtime_contract::UiAction::ZoomWaveformFull),
        other => Err(other),
    }
}
