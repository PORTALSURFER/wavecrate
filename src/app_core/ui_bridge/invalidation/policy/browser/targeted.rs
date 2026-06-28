use crate::app_core::actions::NativeUiAction;

pub(super) fn prefers_targeted_invalidation(action: &NativeUiAction) -> bool {
    matches!(
        action,
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::MoveBrowserFocus { .. }
        ) | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::SetBrowserViewStart { .. }
        ) | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusSourcesPanel)
            | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusFolderPanel)
            | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusFolderSearch)
            | NativeUiAction::Shell(
                crate::app_core::actions::NativeShellAction::SetFolderSearch { .. }
            )
            | NativeUiAction::Shell(
                crate::app_core::actions::NativeShellAction::ToggleShowAllFolders
            )
            | NativeUiAction::Shell(
                crate::app_core::actions::NativeShellAction::ToggleFolderFlattenedView
            )
            | NativeUiAction::SourcesAndFolders(
                crate::app_core::actions::NativeSourcesFoldersAction::FocusFolderRow { .. }
            )
            | NativeUiAction::SourcesAndFolders(
                crate::app_core::actions::NativeSourcesFoldersAction::ActivateFolderRow { .. }
            )
            | NativeUiAction::SourcesAndFolders(
                crate::app_core::actions::NativeSourcesFoldersAction::ToggleFolderRowExpanded { .. }
            )
            | NativeUiAction::SourcesAndFolders(
                crate::app_core::actions::NativeSourcesFoldersAction::ExpandFocusedFolder
            )
            | NativeUiAction::SourcesAndFolders(
                crate::app_core::actions::NativeSourcesFoldersAction::CollapseFocusedFolder
            )
            | NativeUiAction::SourcesAndFolders(
                crate::app_core::actions::NativeSourcesFoldersAction::ToggleFocusedFolderSelection
            )
            | NativeUiAction::SourcesAndFolders(
                crate::app_core::actions::NativeSourcesFoldersAction::MoveFolderFocus { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::FocusBrowserRow { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::AdjustSelectedBrowserRating { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleBrowserRowSelection { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::StartBrowserSampleDrag { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::UpdateBrowserSampleDrag { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::FinishBrowserSampleDrag
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ExtendBrowserSelectionToRow { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::AddRangeBrowserSelection { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ExtendBrowserSelectionFromFocus { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::AddRangeBrowserSelectionFromFocus { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleFocusedBrowserRowSelection
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::SelectAllBrowserRows
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::SetBrowserSearch { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleBrowserRatingFilter { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleBrowserPlaybackAgeFilter { .. }
            )
            | NativeUiAction::PromptsAndEdits(
                crate::app_core::actions::NativePromptEditAction::AutoRenameBrowserSelection { .. }
            )
            | NativeUiAction::PromptsAndEdits(
                crate::app_core::actions::NativePromptEditAction::TagBrowserSelection { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleBrowserTagNamedFilter { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleBrowserTagSidebar
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleBrowserTagSidebarAutoRename
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleBrowserDuplicateCleanupMode
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleBrowserDuplicateCleanupKeep { .. }
            )
            | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusBrowserPanel)
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::FocusBrowserTagSidebarInput
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::SetBrowserTagSidebarInput { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::CommitBrowserTagSidebarInput
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::SetBrowserSidebarLooped { .. }
            )
            | NativeUiAction::Browser(
                crate::app_core::actions::NativeBrowserAction::ToggleBrowserSidebarNormalTag { .. }
            )
            | NativeUiAction::Shell(
                crate::app_core::actions::NativeShellAction::FocusLoadedSampleInBrowser
            )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_core::actions::{NativeBrowserAction, NativeShellAction};

    #[test]
    fn browser_navigation_prefers_targeted_invalidation() {
        let action = NativeUiAction::Browser(NativeBrowserAction::MoveBrowserFocus { delta: 1 });

        assert!(prefers_targeted_invalidation(&action));
    }

    #[test]
    fn folder_shell_actions_prefer_targeted_invalidation() {
        let action = NativeUiAction::Shell(NativeShellAction::SetFolderSearch {
            query: String::from("drums"),
        });

        assert!(prefers_targeted_invalidation(&action));
    }
}
