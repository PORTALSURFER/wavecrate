use super::{InvalidationReason, InvalidationSource};
use crate::app_core::actions::NativeUiAction;

pub(super) fn can_skip_projection_cache_invalidation(action: &NativeUiAction) -> bool {
    matches!(
        action,
        NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SeekWaveformPrecise { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformCursorPrecise { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::BeginWaveformCircularSlide { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::UpdateWaveformCircularSlide { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformCircularSlide)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformViewCenter { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::BeginWaveformSelectionAt { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::BeginWaveformSelectionAtPrecise { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRangePrecise { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRangeSmartScale { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRangeSmartScalePrecise { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditSelectionRange { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditSelectionRangePrecise { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeInEnd { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeInMuteStart { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeInCurve { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeOutStart { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeOutMuteEnd { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeOutCurve { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformEditFadeDrag)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformSelectionRangeDrag)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformEditSelectionDrag)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ClearWaveformSelection)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ClearWaveformEditSelection)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ClearWaveformSelections)
            | NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetWaveformBpmValue { .. })
            | NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::AdjustWaveformBpm { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ZoomWaveform { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ZoomWaveformToSelection)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ZoomWaveformFull)
    )
}

pub(super) fn classify_dirty_source(
    action: &NativeUiAction,
) -> Option<(InvalidationSource, InvalidationReason)> {
    match action {
        NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SeekWaveformPrecise { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformCursorPrecise { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::BeginWaveformSelectionAt { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::BeginWaveformSelectionAtPrecise { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRange { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRangePrecise { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditSelectionRange { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditSelectionRangePrecise { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeInEnd { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeInMuteStart { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeInCurve { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeOutStart { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeOutMuteEnd { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeOutCurve { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformEditFadeDrag)
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformSelectionRangeDrag)
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformEditSelectionDrag)
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ClearWaveformSelection)
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ClearWaveformEditSelection) => Some((
            InvalidationSource::Waveform,
            InvalidationReason::WaveformOverlayAction,
        )),
        NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ClearWaveformSelections) => Some((
            InvalidationSource::Waveform,
            InvalidationReason::WaveformOverlayAction,
        )),
        NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ZoomWaveform { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ZoomWaveformToSelection)
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ZoomWaveformFull)
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformViewCenter { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::BeginWaveformCircularSlide { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::UpdateWaveformCircularSlide { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformCircularSlide)
        | NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetWaveformBpmValue { .. })
        | NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::AdjustWaveformBpm { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRangeSmartScale { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformSelectionRangeSmartScalePrecise { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformSelectionSmartScaleDrag)
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::CommitWaveformEditFades)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::CropWaveformSelection)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::CropWaveformSelectionToNewSample)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::TrimWaveformSelection) => Some((
            InvalidationSource::Waveform,
            InvalidationReason::WaveformViewAction,
        )),
        _ => None,
    }
}

pub(super) fn render_inputs_require_refresh(reason: Option<InvalidationReason>) -> bool {
    !matches!(reason, Some(InvalidationReason::WaveformOverlayAction))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_core::actions::{NativePromptEditAction, NativeWaveformAction};

    #[test]
    fn waveform_overlay_actions_skip_broad_projection_cache_invalidation() {
        let action = NativeUiAction::Waveform(NativeWaveformAction::SeekWaveformPrecise {
            position_nanos: 250_000_000,
        });

        assert!(can_skip_projection_cache_invalidation(&action));
        assert_eq!(
            classify_dirty_source(&action),
            Some((
                InvalidationSource::Waveform,
                InvalidationReason::WaveformOverlayAction
            ))
        );
    }

    #[test]
    fn waveform_edit_commit_actions_dirty_waveform_view() {
        let action = NativeUiAction::PromptsAndEdits(NativePromptEditAction::TrimWaveformSelection);

        assert_eq!(
            classify_dirty_source(&action),
            Some((
                InvalidationSource::Waveform,
                InvalidationReason::WaveformViewAction
            ))
        );
    }
}
