use super::super::{InvalidationReason, InvalidationSource};
use crate::app_core::actions::NativeUiAction;

pub(super) fn classify_dirty_source(
    action: &NativeUiAction,
) -> Option<(InvalidationSource, InvalidationReason)> {
    match action {
        NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::MoveBrowserFocus { .. },
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::SetBrowserViewStart { .. },
        )
        | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusSourcesPanel)
        | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusFolderPanel)
        | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusFolderSearch)
        | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::SetFolderSearch {
            ..
        })
        | NativeUiAction::Shell(
            crate::app_core::actions::NativeShellAction::ToggleShowAllFolders,
        )
        | NativeUiAction::Shell(
            crate::app_core::actions::NativeShellAction::ToggleFolderFlattenedView,
        )
        | NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::FocusFolderRow { .. },
        )
        | NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::ActivateFolderRow { .. },
        )
        | NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::ToggleFolderRowExpanded {
                ..
            },
        )
        | NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::ExpandFocusedFolder,
        )
        | NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::CollapseFocusedFolder,
        )
        | NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::ToggleFocusedFolderSelection,
        )
        | NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::MoveFolderFocus { .. },
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::FocusBrowserRow { .. },
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::AdjustSelectedBrowserRating { .. },
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::CommitFocusedBrowserRow,
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ToggleBrowserRowSelection { .. },
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::StartBrowserSampleDrag { .. },
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::UpdateBrowserSampleDrag { .. },
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::FinishBrowserSampleDrag,
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ExtendBrowserSelectionToRow { .. },
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::AddRangeBrowserSelection { .. },
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ExtendBrowserSelectionFromFocus {
                ..
            },
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::AddRangeBrowserSelectionFromFocus {
                ..
            },
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ToggleFocusedBrowserRowSelection,
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::SelectAllBrowserRows,
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::SetBrowserSearch { .. },
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ToggleBrowserRatingFilter { .. },
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ToggleBrowserPlaybackAgeFilter {
                ..
            },
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ToggleBrowserSampleMark,
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ToggleBrowserMarkedFilter,
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ToggleBrowserTagNamedFilter { .. },
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ToggleBrowserTagSidebar,
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ToggleBrowserTagSidebarAutoRename,
        )
        | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusBrowserPanel)
        | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::FocusBrowserSearch)
        | NativeUiAction::Shell(crate::app_core::actions::NativeShellAction::BlurBrowserSearch)
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::FocusBrowserTagSidebarInput,
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::SetBrowserTagSidebarInput { .. },
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::CommitBrowserTagSidebarInput,
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::SetBrowserSidebarLooped { .. },
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ToggleBrowserSidebarNormalTag { .. },
        )
        | NativeUiAction::Shell(
            crate::app_core::actions::NativeShellAction::FocusLoadedSampleInBrowser,
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ToggleBrowserDuplicateCleanupMode,
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ToggleBrowserDuplicateCleanupKeep {
                ..
            },
        )
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::ConfirmBrowserDuplicateCleanup,
        )
        | NativeUiAction::PromptsAndEdits(
            crate::app_core::actions::NativePromptEditAction::StartBrowserRename,
        )
        | NativeUiAction::PromptsAndEdits(
            crate::app_core::actions::NativePromptEditAction::ConfirmBrowserRename,
        )
        | NativeUiAction::PromptsAndEdits(
            crate::app_core::actions::NativePromptEditAction::CancelBrowserRename,
        )
        | NativeUiAction::PromptsAndEdits(
            crate::app_core::actions::NativePromptEditAction::AutoRenameBrowserSelection { .. },
        )
        | NativeUiAction::PromptsAndEdits(
            crate::app_core::actions::NativePromptEditAction::TagBrowserSelection { .. },
        )
        | NativeUiAction::PromptsAndEdits(
            crate::app_core::actions::NativePromptEditAction::DeleteBrowserSelection,
        )
        | NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::StartNewFolder,
        )
        | NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::StartNewFolderAtFolderRow {
                ..
            },
        )
        | NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::StartNewFolderAtRoot,
        )
        | NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::FocusFolderCreateInput,
        )
        | NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::SetFolderCreateInput { .. },
        )
        | NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::ConfirmFolderCreate,
        )
        | NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::CancelFolderCreate,
        )
        | NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::StartFolderRename,
        )
        | NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::DeleteFocusedFolder,
        )
        | NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::RestoreRetainedFolderDeletes,
        )
        | NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::PurgeRetainedFolderDeletes,
        )
        | NativeUiAction::SourcesAndFolders(
            crate::app_core::actions::NativeSourcesFoldersAction::ClearFolderDeleteRecoveryLog,
        )
        | NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::SetBrowserTab {
            map: false,
        }) => Some((
            InvalidationSource::Browser,
            InvalidationReason::BrowserAction,
        )),
        NativeUiAction::Browser(crate::app_core::actions::NativeBrowserAction::SetBrowserTab {
            map: true,
        })
        | NativeUiAction::Browser(
            crate::app_core::actions::NativeBrowserAction::FocusMapSample { .. },
        ) => Some((InvalidationSource::Map, InvalidationReason::MapAction)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_core::actions::{NativeBrowserAction, NativeShellAction};

    #[test]
    fn browser_navigation_dirties_browser_source() {
        let action = NativeUiAction::Browser(NativeBrowserAction::MoveBrowserFocus { delta: 1 });

        assert_eq!(
            classify_dirty_source(&action),
            Some((
                InvalidationSource::Browser,
                InvalidationReason::BrowserAction
            ))
        );
    }

    #[test]
    fn folder_shell_actions_share_browser_dirty_source() {
        let action = NativeUiAction::Shell(NativeShellAction::SetFolderSearch {
            query: String::from("drums"),
        });

        assert_eq!(
            classify_dirty_source(&action),
            Some((
                InvalidationSource::Browser,
                InvalidationReason::BrowserAction
            ))
        );
    }

    #[test]
    fn map_browser_actions_dirty_map_source() {
        let action = NativeUiAction::Browser(NativeBrowserAction::SetBrowserTab { map: true });

        assert_eq!(
            classify_dirty_source(&action),
            Some((InvalidationSource::Map, InvalidationReason::MapAction))
        );
    }
}
