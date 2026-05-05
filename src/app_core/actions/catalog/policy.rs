//! Policy lookups for GUI action dispatch and history behavior.
//!
//! This module owns policy classification only. It must not duplicate the
//! action catalog row definitions or stable action identifiers.

use super::{GuiActionKind, GuiDispatchPolicy, GuiHistoryPolicy};

/// Resolve whether one cataloged action is publicly dispatchable.
pub(super) const fn gui_dispatch_policy(kind: GuiActionKind) -> GuiDispatchPolicy {
    match kind {
        GuiActionKind::BeginWaveformSelectionShift
        | GuiActionKind::BeginWaveformEditSelectionShift => GuiDispatchPolicy::RuntimeInternal,
        _ => GuiDispatchPolicy::Public,
    }
}

/// Resolve the v1 undo/redo transaction policy for one cataloged action.
pub(super) const fn gui_history_policy(kind: GuiActionKind) -> GuiHistoryPolicy {
    match kind {
        GuiActionKind::FocusSourceRow
        | GuiActionKind::SelectSourceRow
        | GuiActionKind::MoveSourceFocus
        | GuiActionKind::FocusFolderRow
        | GuiActionKind::ActivateFolderRow
        | GuiActionKind::ToggleShowAllFolders
        | GuiActionKind::ToggleFolderFlattenedView
        | GuiActionKind::ToggleFocusedFolderSelection
        | GuiActionKind::MoveFolderFocus
        | GuiActionKind::MoveBrowserFocus
        | GuiActionKind::FocusBrowserRow
        | GuiActionKind::CommitFocusedBrowserRow
        | GuiActionKind::ToggleBrowserRowSelection
        | GuiActionKind::ExtendBrowserSelectionToRow
        | GuiActionKind::AddRangeBrowserSelection
        | GuiActionKind::ExtendBrowserSelectionFromFocus
        | GuiActionKind::AddRangeBrowserSelectionFromFocus
        | GuiActionKind::ToggleFocusedBrowserRowSelection
        | GuiActionKind::SelectAllBrowserRows
        | GuiActionKind::FinishWaveformSelectionDrag
        | GuiActionKind::FinishWaveformCircularSlide
        | GuiActionKind::FinishWaveformSelectionRangeDrag
        | GuiActionKind::FinishWaveformSelectionSmartScaleDrag
        | GuiActionKind::FinishWaveformEditSelectionDrag
        | GuiActionKind::FinishWaveformEditFadeDrag
        | GuiActionKind::ClearWaveformSelection
        | GuiActionKind::ClearWaveformEditSelection
        | GuiActionKind::ClearWaveformSelections
        | GuiActionKind::SlideWaveformSelection
        | GuiActionKind::TagBrowserSelection
        | GuiActionKind::AdjustSelectedBrowserRating
        | GuiActionKind::DeleteFocusedFolder => GuiHistoryPolicy::Immediate,
        GuiActionKind::NormalizeFocusedBrowserSample
        | GuiActionKind::SaveWaveformSelectionToBrowser
        | GuiActionKind::SaveWaveformSelectionToBrowserWithKeep2 => GuiHistoryPolicy::Deferred,
        _ => GuiHistoryPolicy::None,
    }
}
