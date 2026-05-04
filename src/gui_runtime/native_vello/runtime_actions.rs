use super::*;

impl<B: NativeAppBridge> NativeVelloRunner<B> {
    pub(super) fn classify_action_scope(action: &UiAction) -> RuntimeInvalidationScope {
        match action {
            UiAction::SetVolume { .. }
            | UiAction::CommitVolumeSetting
            | UiAction::SetFolderSearch { .. }
            | UiAction::ReloadSourceRow { .. }
            | UiAction::HardSyncSourceRow { .. }
            | UiAction::OpenSourceFolderRow { .. }
            | UiAction::RemoveSourceRow { .. }
            | UiAction::FocusFolderRow { .. }
            | UiAction::ActivateFolderRow { .. }
            | UiAction::ToggleFolderRowExpanded { .. }
            | UiAction::ExpandFocusedFolder
            | UiAction::CollapseFocusedFolder
            | UiAction::MoveFolderFocus { .. }
            | UiAction::SetBrowserSearch { .. }
            | UiAction::BlurBrowserSearch
            | UiAction::SetBrowserTab { .. }
            | UiAction::FocusSpatialContentItem { .. }
            | UiAction::SetPromptInput { .. }
            | UiAction::SetWaveformBpmValue { .. }
            | UiAction::AdjustWaveformBpm { .. }
            | UiAction::BeginWaveformCircularSlide { .. }
            | UiAction::UpdateWaveformCircularSlide { .. }
            | UiAction::FinishWaveformCircularSlide
            | UiAction::BeginWaveformSelectionAt { .. }
            | UiAction::BeginWaveformSelectionAtPrecise { .. }
            | UiAction::SetWaveformSelectionRange { .. }
            | UiAction::SetWaveformSelectionRangePrecise { .. }
            | UiAction::SetWaveformSelectionRangeSmartScale { .. }
            | UiAction::SetWaveformSelectionRangeSmartScalePrecise { .. }
            | UiAction::SetWaveformEditSelectionRange { .. }
            | UiAction::SetWaveformEditSelectionRangePrecise { .. }
            | UiAction::DetectWaveformSilenceSlices
            | UiAction::DetectWaveformExactDuplicateSlices
            | UiAction::CleanWaveformExactDuplicateSlices
            | UiAction::ToggleWaveformSliceSelection { .. }
            | UiAction::AuditionWaveformDuplicateSlice { .. }
            | UiAction::ToggleWaveformDuplicateSliceExemption { .. }
            | UiAction::MoveWaveformSliceFocus { .. }
            | UiAction::ToggleFocusedWaveformSliceExportMark
            | UiAction::SetWaveformEditFadeInEnd { .. }
            | UiAction::SetWaveformEditFadeInMuteStart { .. }
            | UiAction::SetWaveformEditFadeInCurve { .. }
            | UiAction::SetWaveformEditFadeOutStart { .. }
            | UiAction::SetWaveformEditFadeOutMuteEnd { .. }
            | UiAction::SetWaveformEditFadeOutCurve { .. }
            | UiAction::FinishWaveformEditFadeDrag
            | UiAction::StartContentItemDrag { .. }
            | UiAction::UpdateContentItemDrag { .. }
            | UiAction::FinishContentItemDrag
            | UiAction::StartWaveformSelectionDrag { .. }
            | UiAction::UpdateWaveformSelectionDrag { .. }
            | UiAction::FinishWaveformSelectionDrag
            | UiAction::FinishWaveformSelectionRangeDrag
            | UiAction::FinishWaveformSelectionSmartScaleDrag
            | UiAction::FinishWaveformEditSelectionDrag
            | UiAction::CommitWaveformEditFades
            | UiAction::ClearWaveformSelection
            | UiAction::ClearWaveformEditSelection
            | UiAction::ClearWaveformSelections => RuntimeInvalidationScope::ModelAndOverlays,
            UiAction::MoveBrowserFocus { .. }
            | UiAction::FocusBrowserRow { .. }
            | UiAction::ToggleBrowserRowSelection { .. }
            | UiAction::ExtendBrowserSelectionToRow { .. }
            | UiAction::AddRangeBrowserSelection { .. }
            | UiAction::ExtendBrowserSelectionFromFocus { .. }
            | UiAction::AddRangeBrowserSelectionFromFocus { .. }
            | UiAction::ToggleFocusedBrowserRowSelection
            | UiAction::SelectAllBrowserRows
            | UiAction::SetBrowserViewStart { .. } => RuntimeInvalidationScope::ModelAndOverlays,
            UiAction::SeekWaveformPrecise { .. }
            | UiAction::SetWaveformCursorPrecise { .. }
            | UiAction::SeekWaveform { .. }
            | UiAction::SetWaveformCursor { .. } => RuntimeInvalidationScope::OverlayMotionOnly,
            UiAction::PlayFromStart
            | UiAction::PlayFromCurrentPlayhead
            | UiAction::PlayWaveformAtPrecise { .. }
            | UiAction::PlayFromWaveformCursor => RuntimeInvalidationScope::ModelAndOverlays,
            UiAction::ZoomWaveform { .. }
            | UiAction::SetWaveformViewCenter { .. }
            | UiAction::ZoomWaveformToSelection
            | UiAction::ZoomWaveformFull => RuntimeInvalidationScope::StaticAndOverlays,
            _ => RuntimeInvalidationScope::StaticAndOverlays,
        }
    }

    /// Classify bridge actions into tracked interaction profile groups.
    pub(super) fn classify_action_interaction(action: &UiAction) -> Option<InteractionProfileKind> {
        match action {
            UiAction::SetBrowserTab { map: true } | UiAction::FocusSpatialContentItem { .. } => {
                Some(InteractionProfileKind::SpatialPanProxy)
            }
            UiAction::StartContentItemDrag { .. }
            | UiAction::UpdateContentItemDrag { .. }
            | UiAction::FinishContentItemDrag => Some(InteractionProfileKind::Hover),
            UiAction::SeekWaveformPrecise { .. }
            | UiAction::SeekWaveform { .. }
            | UiAction::PlayFromStart
            | UiAction::PlayFromCurrentPlayhead
            | UiAction::PlayWaveformAtPrecise { .. }
            | UiAction::PlayFromWaveformCursor
            | UiAction::SetWaveformCursorPrecise { .. }
            | UiAction::SetWaveformCursor { .. }
            | UiAction::BeginWaveformCircularSlide { .. }
            | UiAction::UpdateWaveformCircularSlide { .. }
            | UiAction::FinishWaveformCircularSlide
            | UiAction::SetWaveformViewCenter { .. }
            | UiAction::BeginWaveformSelectionAt { .. }
            | UiAction::BeginWaveformSelectionAtPrecise { .. }
            | UiAction::SetWaveformSelectionRange { .. }
            | UiAction::SetWaveformSelectionRangePrecise { .. }
            | UiAction::SetWaveformSelectionRangeSmartScale { .. }
            | UiAction::SetWaveformSelectionRangeSmartScalePrecise { .. }
            | UiAction::SetWaveformBpmValue { .. }
            | UiAction::AdjustWaveformBpm { .. }
            | UiAction::SetWaveformEditSelectionRange { .. }
            | UiAction::SetWaveformEditSelectionRangePrecise { .. }
            | UiAction::SetWaveformEditFadeInEnd { .. }
            | UiAction::SetWaveformEditFadeInMuteStart { .. }
            | UiAction::SetWaveformEditFadeInCurve { .. }
            | UiAction::SetWaveformEditFadeOutStart { .. }
            | UiAction::SetWaveformEditFadeOutMuteEnd { .. }
            | UiAction::SetWaveformEditFadeOutCurve { .. }
            | UiAction::FinishWaveformEditFadeDrag
            | UiAction::StartWaveformSelectionDrag { .. }
            | UiAction::UpdateWaveformSelectionDrag { .. }
            | UiAction::FinishWaveformSelectionDrag
            | UiAction::FinishWaveformSelectionRangeDrag
            | UiAction::FinishWaveformSelectionSmartScaleDrag
            | UiAction::FinishWaveformEditSelectionDrag
            | UiAction::ClearWaveformSelection
            | UiAction::ClearWaveformEditSelection
            | UiAction::ClearWaveformSelections
            | UiAction::DetectWaveformSilenceSlices
            | UiAction::DetectWaveformExactDuplicateSlices
            | UiAction::CleanWaveformExactDuplicateSlices
            | UiAction::ToggleWaveformSliceSelection { .. }
            | UiAction::AuditionWaveformDuplicateSlice { .. }
            | UiAction::ToggleWaveformDuplicateSliceExemption { .. }
            | UiAction::MoveWaveformSliceFocus { .. }
            | UiAction::ToggleFocusedWaveformSliceExportMark
            | UiAction::CommitWaveformEditFades
            | UiAction::ZoomWaveform { .. }
            | UiAction::ZoomWaveformToSelection
            | UiAction::ZoomWaveformFull => Some(InteractionProfileKind::Timeline),
            UiAction::SetVolume { .. } => Some(InteractionProfileKind::Volume),
            _ => None,
        }
    }

    /// Apply one model action and optionally record interaction latency.
    pub(super) fn emit_model_action_with_profile(
        &mut self,
        action: UiAction,
        profile_kind: Option<InteractionProfileKind>,
    ) {
        self.apply_invalidation_scope(Self::classify_action_scope(&action));
        let profile_start = profile_kind.and_then(|_| self.profiler.now_if_enabled());
        self.bridge.reduce_action(action);
        if let (Some(kind), Some(start)) = (profile_kind, profile_start) {
            self.profiler.add_interaction_latency(kind, start.elapsed());
        }
    }

    /// Apply one model action with default interaction profiling classification.
    pub(super) fn emit_model_action(&mut self, action: UiAction) {
        let profile_kind = Self::classify_action_interaction(&action);
        self.emit_model_action_with_profile(action, profile_kind);
    }
}
