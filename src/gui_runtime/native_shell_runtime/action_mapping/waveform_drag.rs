use super::super::*;

pub(super) fn generic_to_product(
    value: runtime_contract::UiAction,
) -> Result<UiAction, runtime_contract::UiAction> {
    match value {
        runtime_contract::UiAction::StartWaveformSelectionDrag {
            pointer_x,
            pointer_y,
        } => Ok(UiAction::StartWaveformSelectionDrag {
            pointer_x,
            pointer_y,
        }),
        runtime_contract::UiAction::UpdateWaveformSelectionDrag {
            pointer_x,
            pointer_y,
            hovered_folder_pane,
            hovered_folder_row,
            over_folder_panel,
            over_browser_list,
            shift_down,
            alt_down,
        } => Ok(UiAction::UpdateWaveformSelectionDrag {
            pointer_x,
            pointer_y,
            hovered_folder_pane,
            hovered_folder_row,
            over_folder_panel,
            over_browser_list,
            shift_down,
            alt_down,
        }),
        runtime_contract::UiAction::FinishWaveformSelectionDrag => {
            Ok(UiAction::FinishWaveformSelectionDrag)
        }
        runtime_contract::UiAction::FinishWaveformSelectionRangeDrag => {
            Ok(UiAction::FinishWaveformSelectionRangeDrag)
        }
        runtime_contract::UiAction::FinishWaveformSelectionSmartScaleDrag => {
            Ok(UiAction::FinishWaveformSelectionSmartScaleDrag)
        }
        runtime_contract::UiAction::BeginWaveformSelectionShift {
            pointer_micros,
            start_micros,
            end_micros,
        } => Ok(UiAction::BeginWaveformSelectionShift {
            pointer_micros,
            start_micros,
            end_micros,
        }),
        runtime_contract::UiAction::BeginWaveformSelectionShiftPrecise {
            pointer_nanos,
            start_nanos,
            end_nanos,
        } => Ok(UiAction::BeginWaveformSelectionShiftPrecise {
            pointer_nanos,
            start_nanos,
            end_nanos,
        }),
        runtime_contract::UiAction::BeginWaveformEditSelectionShift {
            pointer_micros,
            start_micros,
            end_micros,
        } => Ok(UiAction::BeginWaveformEditSelectionShift {
            pointer_micros,
            start_micros,
            end_micros,
        }),
        runtime_contract::UiAction::BeginWaveformEditSelectionShiftPrecise {
            pointer_nanos,
            start_nanos,
            end_nanos,
        } => Ok(UiAction::BeginWaveformEditSelectionShiftPrecise {
            pointer_nanos,
            start_nanos,
            end_nanos,
        }),
        runtime_contract::UiAction::FinishWaveformEditSelectionDrag => {
            Ok(UiAction::FinishWaveformEditSelectionDrag)
        }
        other => Err(other),
    }
}

pub(super) fn product_to_generic(value: UiAction) -> Result<runtime_contract::UiAction, UiAction> {
    match value {
        UiAction::StartWaveformSelectionDrag {
            pointer_x,
            pointer_y,
        } => Ok(runtime_contract::UiAction::StartWaveformSelectionDrag {
            pointer_x,
            pointer_y,
        }),
        UiAction::UpdateWaveformSelectionDrag {
            pointer_x,
            pointer_y,
            hovered_folder_pane,
            hovered_folder_row,
            over_folder_panel,
            over_browser_list,
            shift_down,
            alt_down,
        } => Ok(runtime_contract::UiAction::UpdateWaveformSelectionDrag {
            pointer_x,
            pointer_y,
            hovered_folder_pane,
            hovered_folder_row,
            over_folder_panel,
            over_browser_list,
            shift_down,
            alt_down,
        }),
        UiAction::FinishWaveformSelectionDrag => {
            Ok(runtime_contract::UiAction::FinishWaveformSelectionDrag)
        }
        UiAction::FinishWaveformSelectionRangeDrag => {
            Ok(runtime_contract::UiAction::FinishWaveformSelectionRangeDrag)
        }
        UiAction::FinishWaveformSelectionSmartScaleDrag => {
            Ok(runtime_contract::UiAction::FinishWaveformSelectionSmartScaleDrag)
        }
        UiAction::BeginWaveformSelectionShift {
            pointer_micros,
            start_micros,
            end_micros,
        } => Ok(runtime_contract::UiAction::BeginWaveformSelectionShift {
            pointer_micros,
            start_micros,
            end_micros,
        }),
        UiAction::BeginWaveformSelectionShiftPrecise {
            pointer_nanos,
            start_nanos,
            end_nanos,
        } => Ok(
            runtime_contract::UiAction::BeginWaveformSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            },
        ),
        UiAction::BeginWaveformEditSelectionShift {
            pointer_micros,
            start_micros,
            end_micros,
        } => Ok(
            runtime_contract::UiAction::BeginWaveformEditSelectionShift {
                pointer_micros,
                start_micros,
                end_micros,
            },
        ),
        UiAction::BeginWaveformEditSelectionShiftPrecise {
            pointer_nanos,
            start_nanos,
            end_nanos,
        } => Ok(
            runtime_contract::UiAction::BeginWaveformEditSelectionShiftPrecise {
                pointer_nanos,
                start_nanos,
                end_nanos,
            },
        ),
        UiAction::FinishWaveformEditSelectionDrag => {
            Ok(runtime_contract::UiAction::FinishWaveformEditSelectionDrag)
        }
        other => Err(other),
    }
}
