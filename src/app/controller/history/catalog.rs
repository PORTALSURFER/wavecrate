//! Catalog-to-controller history compatibility helpers.
//!
//! This stays separate from the snapshot/deferred-history core so the
//! action-catalog guard tests can validate controller support without
//! inflating the main history implementation module.

use crate::app_core::actions::{GuiActionKind, GuiHistoryPolicy};

/// Return whether one catalog history policy is backed by a controller transaction handler.
pub(crate) const fn catalog_history_handler_supported(
    kind: GuiActionKind,
    policy: GuiHistoryPolicy,
) -> bool {
    match policy {
        GuiHistoryPolicy::None => true,
        GuiHistoryPolicy::Immediate => matches!(
            kind,
            GuiActionKind::FocusSourceRow
                | GuiActionKind::SelectSourceRow
                | GuiActionKind::MoveSourceFocus
                | GuiActionKind::FocusFolderRow
                | GuiActionKind::ActivateFolderRow
                | GuiActionKind::ToggleFocusedFolderSelection
                | GuiActionKind::ToggleShowAllFolders
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
                | GuiActionKind::DeleteFocusedFolder
        ),
        GuiHistoryPolicy::Deferred => matches!(
            kind,
            GuiActionKind::NormalizeFocusedBrowserSample
                | GuiActionKind::SaveWaveformSelectionToBrowser
        ),
    }
}
