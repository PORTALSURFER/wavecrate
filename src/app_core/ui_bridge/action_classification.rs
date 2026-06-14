#[cfg(test)]
use crate::app_core::actions::{GuiActionKind, representative_action_for_kind};
use crate::app_core::actions::{NativeOptionsAction, NativeUiAction};

/// Interaction classes tracked by UI bridge profiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InteractionActionClass {
    /// Wheel-like browser row movement actions.
    Wheel,
    /// Map interaction actions flowing through the bridge.
    MapPanProxy,
    /// Waveform seek/cursor/selection/edit/fade/zoom actions.
    Waveform,
    /// Volume slider interaction actions.
    Volume,
}

/// Classify UI actions into focused interaction profile groups.
pub(super) fn classify_action_interaction(
    action: &NativeUiAction,
) -> Option<InteractionActionClass> {
    match action {
        NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::MoveBrowserFocus { .. }) | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::SetBrowserViewStart { .. }) => {
            Some(InteractionActionClass::Wheel)
        }
        NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::SetBrowserTab { map: true }) | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::FocusMapSample { .. }) => {
            Some(InteractionActionClass::MapPanProxy)
        }
        NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SeekWaveformPrecise { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformCursorPrecise { .. })
        | NativeUiAction::Transport(
            crate::app_core::actions::NativeTransportAction::PlayFromWaveformCursor,
        )
        | NativeUiAction::Transport(
            crate::app_core::actions::NativeTransportAction::PlayWaveformAtPrecise { .. },
        )
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
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::StartWaveformSelectionDrag { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::UpdateWaveformSelectionDrag { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformSelectionDrag)
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformSelectionRangeDrag)
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformSelectionSmartScaleDrag)
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformEditSelectionDrag)
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ClearWaveformEditSelection)
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ClearWaveformSelections)
        | NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetWaveformBpmValue { .. })
        | NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::AdjustWaveformBpm { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ClearWaveformSelection)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::CropWaveformSelection)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::CropWaveformSelectionToNewSample)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::TrimWaveformSelection)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::ReverseWaveformSelection)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::FadeWaveformSelectionLeftToRight)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::FadeWaveformSelectionRightToLeft)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::MuteWaveformSelection)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::DeleteSelectedSliceMarkers)
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::DetectWaveformSilenceSlices)
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::DetectWaveformExactDuplicateSlices)
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::CleanWaveformExactDuplicateSlices)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::ToggleWaveformSliceSelection { .. })
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::AuditionWaveformDuplicateSlice { .. })
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::ToggleWaveformDuplicateSliceExemption { .. })
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::MoveWaveformSliceFocus { .. })
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::ToggleFocusedWaveformSliceExportMark)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::AlignWaveformStartToMarker)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::DeleteLoadedWaveformSample)
        | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::SlideWaveformSelection { .. })
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::CommitWaveformEditFades)
        | NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::ToggleTransientMarkers)
        | NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::ToggleBpmSnap)
        | NativeUiAction::Options(crate::app_core::actions::NativeOptionsAction::SetRelativeBpmGridEnabled { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ZoomWaveform { .. })
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ZoomWaveformToSelection)
        | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ZoomWaveformFull) => Some(InteractionActionClass::Waveform),
        NativeUiAction::Options(NativeOptionsAction::SetVolume { .. })
        | NativeUiAction::Options(NativeOptionsAction::CommitVolumeSetting) => {
            Some(InteractionActionClass::Volume)
        }
        _ => None,
    }
}

/// Return whether a waveform action should apply immediately for smooth preview.
///
/// These actions update overlay state frequently (cursor and selection edits) and
/// benefit from immediate feedback more than queue coalescing.
pub(super) fn is_immediate_waveform_preview_action(action: &NativeUiAction) -> bool {
    matches!(
        action,
        NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::BeginWaveformCircularSlide { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::UpdateWaveformCircularSlide { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformCircularSlide)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::BeginWaveformSelectionAt { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::BeginWaveformSelectionAtPrecise { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditSelectionRange { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditSelectionRangePrecise { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeInEnd { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeInMuteStart { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeInCurve { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeOutStart { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeOutMuteEnd { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeOutCurve { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformEditFadeDrag)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::StartWaveformSelectionDrag { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::UpdateWaveformSelectionDrag { .. })
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformSelectionDrag)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformSelectionRangeDrag)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformSelectionSmartScaleDrag)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::FinishWaveformEditSelectionDrag)
            | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::ToggleWaveformSliceSelection { .. })
            | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::AuditionWaveformDuplicateSlice { .. })
            | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::ToggleWaveformDuplicateSliceExemption { .. })
            | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::MoveWaveformSliceFocus { .. })
            | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::ToggleFocusedWaveformSliceExportMark)
            | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::DetectWaveformExactDuplicateSlices)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ClearWaveformEditSelection)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ClearWaveformSelections)
    )
}

/// Return whether an action only mutates native-visible UI state and can use a
/// one-shot local model-pull fast path.
///
/// These actions already update controller UI state directly and do not rely on
/// derived recomputation, background maintenance, or transport ticking to make
/// the next projected frame correct.
pub(super) fn uses_local_model_pull_fast_path(action: &NativeUiAction) -> bool {
    matches!(
        action,
        NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::MoveBrowserFocus { .. })
            | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::SetBrowserViewStart { .. })
            | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusBrowserPanel)
            | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusSourcesPanel)
            | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusWaveformPanel)
            | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusFolderPanel)
            | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusBrowserSearch)
            | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::BlurBrowserSearch)
            | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusFolderSearch)
            | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::SetFolderSearch { .. })
            | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::ToggleShowAllFolders)
            | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::ToggleFolderFlattenedView)
            | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::FocusFolderRow { .. })
            | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::ActivateFolderRow { .. })
            | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::ToggleFolderRowExpanded { .. })
            | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::ExpandFocusedFolder)
            | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::CollapseFocusedFolder)
            | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::ToggleFocusedFolderSelection)
            | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::MoveFolderFocus { .. })
            | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::StartNewFolder)
            | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::StartNewFolderAtFolderRow { .. })
            | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::StartNewFolderAtRoot)
            | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::FocusFolderCreateInput)
            | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::SetFolderCreateInput { .. })
            | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::ConfirmFolderCreate)
            | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::CancelFolderCreate)
            | NativeUiAction::SourcesAndFolders(crate::app_core::actions::NativeSourcesFoldersAction::StartFolderRename)
            | NativeUiAction::Options(NativeOptionsAction::OpenOptionsMenu)
            | NativeUiAction::Options(NativeOptionsAction::CloseOptionsPanel)
            | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::SetPromptInput { .. })
            | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::ToggleBrowserDuplicateCleanupMode)
            | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::ToggleBrowserDuplicateCleanupKeep { .. })
            | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::ToggleWaveformSliceSelection { .. })
            | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::ToggleWaveformDuplicateSliceExemption { .. })
            | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::MoveWaveformSliceFocus { .. })
            | NativeUiAction::PromptsAndEdits(crate::app_core::actions::NativePromptEditAction::ToggleFocusedWaveformSliceExportMark)
            | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::DetectWaveformExactDuplicateSlices)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ClearWaveformEditSelection)
            | NativeUiAction::Waveform(crate::app_core::actions::NativeWaveformAction::ClearWaveformSelections)
    )
}

/// Resolve the interaction profile class for one catalog action kind.
#[cfg(test)]
pub(crate) fn catalog_interaction_class(kind: GuiActionKind) -> Option<InteractionActionClass> {
    classify_action_interaction(&representative_action_for_kind(kind))
}

/// Return whether one catalog action kind should apply immediate waveform preview updates.
#[cfg(test)]
pub(crate) fn catalog_is_immediate_waveform_preview_action(kind: GuiActionKind) -> bool {
    is_immediate_waveform_preview_action(&representative_action_for_kind(kind))
}

/// Return whether one catalog action kind qualifies for the local model-pull fast path.
#[cfg(test)]
pub(crate) fn catalog_uses_local_model_pull_fast_path(kind: GuiActionKind) -> bool {
    uses_local_model_pull_fast_path(&representative_action_for_kind(kind))
}
