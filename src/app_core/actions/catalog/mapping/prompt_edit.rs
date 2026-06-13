use super::shared::{GuiActionKind, Kind, NativePromptEditAction};

pub(super) fn prompt_edit_action_kind(action: &NativePromptEditAction) -> GuiActionKind {
    match action {
        NativePromptEditAction::SetPromptInput { .. } => Kind::SetPromptInput,
        NativePromptEditAction::StartBrowserRename => Kind::StartBrowserRename,
        NativePromptEditAction::ConfirmBrowserRename => Kind::ConfirmBrowserRename,
        NativePromptEditAction::CancelBrowserRename => Kind::CancelBrowserRename,
        NativePromptEditAction::AutoRenameBrowserSelection { .. } => {
            Kind::AutoRenameBrowserSelection
        }
        NativePromptEditAction::TagBrowserSelection { .. } => Kind::TagBrowserSelection,
        NativePromptEditAction::DeleteBrowserSelection => Kind::DeleteBrowserSelection,
        NativePromptEditAction::NormalizeFocusedBrowserSample => {
            Kind::NormalizeFocusedBrowserSample
        }
        NativePromptEditAction::NormalizeWaveformSelectionOrSample => {
            Kind::NormalizeWaveformSelectionOrSample
        }
        NativePromptEditAction::CropWaveformSelection => Kind::CropWaveformSelection,
        NativePromptEditAction::CropWaveformSelectionToNewSample => {
            Kind::CropWaveformSelectionToNewSample
        }
        NativePromptEditAction::TrimWaveformSelection => Kind::TrimWaveformSelection,
        NativePromptEditAction::ReverseWaveformSelection => Kind::ReverseWaveformSelection,
        NativePromptEditAction::FadeWaveformSelectionLeftToRight => {
            Kind::FadeWaveformSelectionLeftToRight
        }
        NativePromptEditAction::FadeWaveformSelectionRightToLeft => {
            Kind::FadeWaveformSelectionRightToLeft
        }
        NativePromptEditAction::MuteWaveformSelection => Kind::MuteWaveformSelection,
        NativePromptEditAction::DeleteSelectedSliceMarkers => Kind::DeleteSelectedSliceMarkers,
        NativePromptEditAction::ToggleWaveformSliceSelection { .. } => {
            Kind::ToggleWaveformSliceSelection
        }
        NativePromptEditAction::AuditionWaveformDuplicateSlice { .. } => {
            Kind::AuditionWaveformDuplicateSlice
        }
        NativePromptEditAction::ToggleWaveformDuplicateSliceExemption { .. } => {
            Kind::ToggleWaveformDuplicateSliceExemption
        }
        NativePromptEditAction::MoveWaveformSliceFocus { .. } => Kind::MoveWaveformSliceFocus,
        NativePromptEditAction::ToggleFocusedWaveformSliceExportMark => {
            Kind::ToggleFocusedWaveformSliceExportMark
        }
        NativePromptEditAction::AlignWaveformStartToMarker => Kind::AlignWaveformStartToMarker,
        NativePromptEditAction::DeleteLoadedWaveformSample => Kind::DeleteLoadedWaveformSample,
        NativePromptEditAction::SlideWaveformSelection { .. } => Kind::SlideWaveformSelection,
        NativePromptEditAction::ConfirmPrompt => Kind::ConfirmPrompt,
        NativePromptEditAction::CancelPrompt => Kind::CancelPrompt,
        NativePromptEditAction::CancelProgress => Kind::CancelProgress,
        NativePromptEditAction::CopySelectionToClipboard => Kind::CopySelectionToClipboard,
        NativePromptEditAction::ToggleHotkeyOverlay => Kind::ToggleHotkeyOverlay,
        NativePromptEditAction::CopyStatusLog => Kind::CopyStatusLog,
        NativePromptEditAction::OpenFeedbackIssuePrompt => Kind::OpenFeedbackIssuePrompt,
        NativePromptEditAction::MoveTrashedSamplesToFolder => Kind::MoveTrashedSamplesToFolder,
    }
}
