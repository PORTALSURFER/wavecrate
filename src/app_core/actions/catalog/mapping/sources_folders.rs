use super::shared::{GuiActionKind, Kind, NativeSourcesFoldersAction};

pub(super) fn sources_folders_action_kind(action: &NativeSourcesFoldersAction) -> GuiActionKind {
    match action {
        NativeSourcesFoldersAction::FocusSourceRow { .. } => Kind::FocusSourceRow,
        NativeSourcesFoldersAction::SelectSourceRow { .. } => Kind::SelectSourceRow,
        NativeSourcesFoldersAction::MoveSourceFocus { .. } => Kind::MoveSourceFocus,
        NativeSourcesFoldersAction::ReloadFocusedSourceRow => Kind::ReloadFocusedSourceRow,
        NativeSourcesFoldersAction::HardSyncFocusedSourceRow => Kind::HardSyncFocusedSourceRow,
        NativeSourcesFoldersAction::OpenFocusedSourceFolder => Kind::OpenFocusedSourceFolder,
        NativeSourcesFoldersAction::RemoveFocusedSourceRow => Kind::RemoveFocusedSourceRow,
        NativeSourcesFoldersAction::ReloadSourceRow { .. } => Kind::ReloadSourceRow,
        NativeSourcesFoldersAction::HardSyncSourceRow { .. } => Kind::HardSyncSourceRow,
        NativeSourcesFoldersAction::OpenSourceFolderRow { .. } => Kind::OpenSourceFolderRow,
        NativeSourcesFoldersAction::RemoveSourceRow { .. } => Kind::RemoveSourceRow,
        NativeSourcesFoldersAction::FocusFolderRow { .. } => Kind::FocusFolderRow,
        NativeSourcesFoldersAction::ActivateFolderRow { .. } => Kind::ActivateFolderRow,
        NativeSourcesFoldersAction::ToggleFolderRowExpanded { .. } => Kind::ToggleFolderRowExpanded,
        NativeSourcesFoldersAction::ExpandFocusedFolder => Kind::ExpandFocusedFolder,
        NativeSourcesFoldersAction::CollapseFocusedFolder => Kind::CollapseFocusedFolder,
        NativeSourcesFoldersAction::ToggleFocusedFolderSelection => {
            Kind::ToggleFocusedFolderSelection
        }
        NativeSourcesFoldersAction::MoveFolderFocus { .. } => Kind::MoveFolderFocus,
        NativeSourcesFoldersAction::StartNewFolder => Kind::StartNewFolder,
        NativeSourcesFoldersAction::StartNewFolderAtFolderRow { .. } => {
            Kind::StartNewFolderAtFolderRow
        }
        NativeSourcesFoldersAction::StartNewFolderAtRoot => Kind::StartNewFolderAtRoot,
        NativeSourcesFoldersAction::FocusFolderCreateInput => Kind::FocusFolderCreateInput,
        NativeSourcesFoldersAction::SetFolderCreateInput { .. } => Kind::SetFolderCreateInput,
        NativeSourcesFoldersAction::ConfirmFolderCreate => Kind::ConfirmFolderCreate,
        NativeSourcesFoldersAction::CancelFolderCreate => Kind::CancelFolderCreate,
        NativeSourcesFoldersAction::StartFolderRename => Kind::StartFolderRename,
        NativeSourcesFoldersAction::DeleteFocusedFolder => Kind::DeleteFocusedFolder,
        NativeSourcesFoldersAction::RestoreRetainedFolderDeletes => {
            Kind::RestoreRetainedFolderDeletes
        }
        NativeSourcesFoldersAction::PurgeRetainedFolderDeletes => Kind::PurgeRetainedFolderDeletes,
        NativeSourcesFoldersAction::ClearFolderDeleteRecoveryLog => {
            Kind::ClearFolderDeleteRecoveryLog
        }
    }
}
