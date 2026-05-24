use super::super::*;

pub(super) fn generic_to_product(
    value: runtime_contract::UiAction,
) -> Result<UiAction, runtime_contract::UiAction> {
    match value {
        runtime_contract::UiAction::ConfirmPrompt => Ok(UiAction::ConfirmPrompt),
        runtime_contract::UiAction::CancelPrompt => Ok(UiAction::CancelPrompt),
        runtime_contract::UiAction::CancelProgress => Ok(UiAction::CancelProgress),
        runtime_contract::UiAction::CopySelectionToClipboard => {
            Ok(UiAction::CopySelectionToClipboard)
        }
        runtime_contract::UiAction::ToggleHotkeyOverlay => Ok(UiAction::ToggleHotkeyOverlay),
        runtime_contract::UiAction::CopyStatusLog => Ok(UiAction::CopyStatusLog),
        runtime_contract::UiAction::OpenFeedbackIssuePrompt => {
            Ok(UiAction::OpenFeedbackIssuePrompt)
        }
        runtime_contract::UiAction::MoveDiscardedItemsToFolder => {
            Ok(UiAction::MoveTrashedSamplesToFolder)
        }
        runtime_contract::UiAction::Undo => Ok(UiAction::Undo),
        runtime_contract::UiAction::Redo => Ok(UiAction::Redo),
        runtime_contract::UiAction::CheckForUpdates => Ok(UiAction::CheckForUpdates),
        runtime_contract::UiAction::OpenUpdateLink => Ok(UiAction::OpenUpdateLink),
        runtime_contract::UiAction::InstallUpdate => Ok(UiAction::InstallUpdate),
        runtime_contract::UiAction::DismissUpdate => Ok(UiAction::DismissUpdate),
        other => Err(other),
    }
}

pub(super) fn product_to_generic(value: UiAction) -> Result<runtime_contract::UiAction, UiAction> {
    match value {
        UiAction::ConfirmPrompt => Ok(runtime_contract::UiAction::ConfirmPrompt),
        UiAction::CancelPrompt => Ok(runtime_contract::UiAction::CancelPrompt),
        UiAction::CancelProgress => Ok(runtime_contract::UiAction::CancelProgress),
        UiAction::CopySelectionToClipboard => {
            Ok(runtime_contract::UiAction::CopySelectionToClipboard)
        }
        UiAction::ToggleHotkeyOverlay => Ok(runtime_contract::UiAction::ToggleHotkeyOverlay),
        UiAction::CopyStatusLog => Ok(runtime_contract::UiAction::CopyStatusLog),
        UiAction::OpenFeedbackIssuePrompt => {
            Ok(runtime_contract::UiAction::OpenFeedbackIssuePrompt)
        }
        UiAction::MoveTrashedSamplesToFolder => {
            Ok(runtime_contract::UiAction::MoveDiscardedItemsToFolder)
        }
        UiAction::Undo => Ok(runtime_contract::UiAction::Undo),
        UiAction::Redo => Ok(runtime_contract::UiAction::Redo),
        UiAction::CheckForUpdates => Ok(runtime_contract::UiAction::CheckForUpdates),
        UiAction::OpenUpdateLink => Ok(runtime_contract::UiAction::OpenUpdateLink),
        UiAction::InstallUpdate => Ok(runtime_contract::UiAction::InstallUpdate),
        UiAction::DismissUpdate => Ok(runtime_contract::UiAction::DismissUpdate),
        other => Err(other),
    }
}
