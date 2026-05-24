use super::super::*;

pub(super) fn generic_to_product(
    value: runtime_contract::UiAction,
) -> Result<UiAction, runtime_contract::UiAction> {
    match value {
        runtime_contract::UiAction::NormalizeFocusedContentItem => {
            Ok(UiAction::NormalizeFocusedBrowserSample)
        }
        runtime_contract::UiAction::NormalizeWaveformSelectionOrLoadedContent => {
            Ok(UiAction::NormalizeWaveformSelectionOrSample)
        }
        runtime_contract::UiAction::CropWaveformSelection => Ok(UiAction::CropWaveformSelection),
        runtime_contract::UiAction::CropWaveformSelectionToNewContentItem => {
            Ok(UiAction::CropWaveformSelectionToNewSample)
        }
        runtime_contract::UiAction::TrimWaveformSelection => Ok(UiAction::TrimWaveformSelection),
        runtime_contract::UiAction::ReverseWaveformSelection => {
            Ok(UiAction::ReverseWaveformSelection)
        }
        runtime_contract::UiAction::FadeWaveformSelectionLeftToRight => {
            Ok(UiAction::FadeWaveformSelectionLeftToRight)
        }
        runtime_contract::UiAction::FadeWaveformSelectionRightToLeft => {
            Ok(UiAction::FadeWaveformSelectionRightToLeft)
        }
        runtime_contract::UiAction::MuteWaveformSelection => Ok(UiAction::MuteWaveformSelection),
        runtime_contract::UiAction::DeleteSelectedSliceMarkers => {
            Ok(UiAction::DeleteSelectedSliceMarkers)
        }
        runtime_contract::UiAction::ToggleWaveformSliceSelection { index } => {
            Ok(UiAction::ToggleWaveformSliceSelection { index })
        }
        runtime_contract::UiAction::AuditionWaveformDuplicateSlice { index } => {
            Ok(UiAction::AuditionWaveformDuplicateSlice { index })
        }
        runtime_contract::UiAction::ToggleWaveformDuplicateSliceExemption { index } => {
            Ok(UiAction::ToggleWaveformDuplicateSliceExemption { index })
        }
        runtime_contract::UiAction::MoveWaveformSliceFocus { delta } => {
            Ok(UiAction::MoveWaveformSliceFocus { delta })
        }
        runtime_contract::UiAction::ToggleFocusedWaveformSliceExportMark => {
            Ok(UiAction::ToggleFocusedWaveformSliceExportMark)
        }
        runtime_contract::UiAction::AlignWaveformStartToMarker => {
            Ok(UiAction::AlignWaveformStartToMarker)
        }
        runtime_contract::UiAction::DeleteLoadedWaveformContent => {
            Ok(UiAction::DeleteLoadedWaveformSample)
        }
        runtime_contract::UiAction::SlideWaveformSelection { delta, fine } => {
            Ok(UiAction::SlideWaveformSelection { delta, fine })
        }
        other => Err(other),
    }
}

pub(super) fn product_to_generic(value: UiAction) -> Result<runtime_contract::UiAction, UiAction> {
    match value {
        UiAction::NormalizeFocusedBrowserSample => {
            Ok(runtime_contract::UiAction::NormalizeFocusedContentItem)
        }
        UiAction::NormalizeWaveformSelectionOrSample => {
            Ok(runtime_contract::UiAction::NormalizeWaveformSelectionOrLoadedContent)
        }
        UiAction::CropWaveformSelection => Ok(runtime_contract::UiAction::CropWaveformSelection),
        UiAction::CropWaveformSelectionToNewSample => {
            Ok(runtime_contract::UiAction::CropWaveformSelectionToNewContentItem)
        }
        UiAction::TrimWaveformSelection => Ok(runtime_contract::UiAction::TrimWaveformSelection),
        UiAction::ReverseWaveformSelection => {
            Ok(runtime_contract::UiAction::ReverseWaveformSelection)
        }
        UiAction::FadeWaveformSelectionLeftToRight => {
            Ok(runtime_contract::UiAction::FadeWaveformSelectionLeftToRight)
        }
        UiAction::FadeWaveformSelectionRightToLeft => {
            Ok(runtime_contract::UiAction::FadeWaveformSelectionRightToLeft)
        }
        UiAction::MuteWaveformSelection => Ok(runtime_contract::UiAction::MuteWaveformSelection),
        UiAction::DeleteSelectedSliceMarkers => {
            Ok(runtime_contract::UiAction::DeleteSelectedSliceMarkers)
        }
        UiAction::ToggleWaveformSliceSelection { index } => {
            Ok(runtime_contract::UiAction::ToggleWaveformSliceSelection { index })
        }
        UiAction::AuditionWaveformDuplicateSlice { index } => {
            Ok(runtime_contract::UiAction::AuditionWaveformDuplicateSlice { index })
        }
        UiAction::ToggleWaveformDuplicateSliceExemption { index } => {
            Ok(runtime_contract::UiAction::ToggleWaveformDuplicateSliceExemption { index })
        }
        UiAction::MoveWaveformSliceFocus { delta } => {
            Ok(runtime_contract::UiAction::MoveWaveformSliceFocus { delta })
        }
        UiAction::ToggleFocusedWaveformSliceExportMark => {
            Ok(runtime_contract::UiAction::ToggleFocusedWaveformSliceExportMark)
        }
        UiAction::AlignWaveformStartToMarker => {
            Ok(runtime_contract::UiAction::AlignWaveformStartToMarker)
        }
        UiAction::DeleteLoadedWaveformSample => {
            Ok(runtime_contract::UiAction::DeleteLoadedWaveformContent)
        }
        UiAction::SlideWaveformSelection { delta, fine } => {
            Ok(runtime_contract::UiAction::SlideWaveformSelection { delta, fine })
        }
        other => Err(other),
    }
}
