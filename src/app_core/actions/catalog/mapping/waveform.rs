use super::shared::{GuiActionKind, Kind, NativeWaveformAction};

pub(super) fn waveform_action_kind(action: &NativeWaveformAction) -> GuiActionKind {
    match action {
        NativeWaveformAction::SeekWaveformPrecise { .. } => Kind::SeekWaveformPrecise,
        NativeWaveformAction::SetWaveformCursorPrecise { .. } => Kind::SetWaveformCursorPrecise,
        NativeWaveformAction::BeginWaveformSelectionAt { .. } => Kind::BeginWaveformSelectionAt,
        NativeWaveformAction::BeginWaveformSelectionAtPrecise { .. } => {
            Kind::BeginWaveformSelectionAtPrecise
        }
        NativeWaveformAction::BeginWaveformCircularSlide { .. } => Kind::BeginWaveformCircularSlide,
        NativeWaveformAction::UpdateWaveformCircularSlide { .. } => {
            Kind::UpdateWaveformCircularSlide
        }
        NativeWaveformAction::FinishWaveformCircularSlide => Kind::FinishWaveformCircularSlide,
        NativeWaveformAction::SetWaveformSelectionRange { .. } => Kind::SetWaveformSelectionRange,
        NativeWaveformAction::SetWaveformSelectionRangePrecise { .. } => {
            Kind::SetWaveformSelectionRangePrecise
        }
        NativeWaveformAction::SetWaveformSelectionRangeSmartScale { .. } => {
            Kind::SetWaveformSelectionRangeSmartScale
        }
        NativeWaveformAction::SetWaveformSelectionRangeSmartScalePrecise { .. } => {
            Kind::SetWaveformSelectionRangeSmartScalePrecise
        }
        NativeWaveformAction::SetWaveformEditSelectionRange { .. } => {
            Kind::SetWaveformEditSelectionRange
        }
        NativeWaveformAction::SetWaveformEditSelectionRangePrecise { .. } => {
            Kind::SetWaveformEditSelectionRangePrecise
        }
        NativeWaveformAction::SetWaveformEditFadeInEnd { .. } => Kind::SetWaveformEditFadeInEnd,
        NativeWaveformAction::SetWaveformEditFadeInMuteStart { .. } => {
            Kind::SetWaveformEditFadeInMuteStart
        }
        NativeWaveformAction::SetWaveformEditFadeInCurve { .. } => Kind::SetWaveformEditFadeInCurve,
        NativeWaveformAction::SetWaveformEditFadeOutStart { .. } => {
            Kind::SetWaveformEditFadeOutStart
        }
        NativeWaveformAction::SetWaveformEditFadeOutMuteEnd { .. } => {
            Kind::SetWaveformEditFadeOutMuteEnd
        }
        NativeWaveformAction::SetWaveformEditFadeOutCurve { .. } => {
            Kind::SetWaveformEditFadeOutCurve
        }
        NativeWaveformAction::FinishWaveformEditFadeDrag => Kind::FinishWaveformEditFadeDrag,
        NativeWaveformAction::StartWaveformSelectionDrag { .. } => Kind::StartWaveformSelectionDrag,
        NativeWaveformAction::UpdateWaveformSelectionDrag { .. } => {
            Kind::UpdateWaveformSelectionDrag
        }
        NativeWaveformAction::FinishWaveformSelectionDrag => Kind::FinishWaveformSelectionDrag,
        NativeWaveformAction::FinishWaveformSelectionRangeDrag => {
            Kind::FinishWaveformSelectionRangeDrag
        }
        NativeWaveformAction::FinishWaveformSelectionSmartScaleDrag => {
            Kind::FinishWaveformSelectionSmartScaleDrag
        }
        NativeWaveformAction::BeginWaveformSelectionShift { .. } => {
            Kind::BeginWaveformSelectionShift
        }
        NativeWaveformAction::BeginWaveformSelectionShiftPrecise { .. } => {
            Kind::BeginWaveformSelectionShiftPrecise
        }
        NativeWaveformAction::BeginWaveformEditSelectionShift { .. } => {
            Kind::BeginWaveformEditSelectionShift
        }
        NativeWaveformAction::BeginWaveformEditSelectionShiftPrecise { .. } => {
            Kind::BeginWaveformEditSelectionShiftPrecise
        }
        NativeWaveformAction::FinishWaveformEditSelectionDrag => {
            Kind::FinishWaveformEditSelectionDrag
        }
        NativeWaveformAction::ClearWaveformSelection => Kind::ClearWaveformSelection,
        NativeWaveformAction::ClearWaveformEditSelection => Kind::ClearWaveformEditSelection,
        NativeWaveformAction::ClearWaveformSelections => Kind::ClearWaveformSelections,
        NativeWaveformAction::SetWaveformViewCenter { .. } => Kind::SetWaveformViewCenter,
        NativeWaveformAction::ZoomWaveform { .. } => Kind::ZoomWaveform,
        NativeWaveformAction::ZoomWaveformToSelection => Kind::ZoomWaveformToSelection,
        NativeWaveformAction::ZoomWaveformFull => Kind::ZoomWaveformFull,
    }
}
