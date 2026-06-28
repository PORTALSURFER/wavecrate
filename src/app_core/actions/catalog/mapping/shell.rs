use super::shared::{GuiActionKind, Kind, NativeShellAction};

pub(super) fn shell_action_kind(action: &NativeShellAction) -> GuiActionKind {
    match action {
        NativeShellAction::FocusBrowserPanel => Kind::FocusBrowserPanel,
        NativeShellAction::FocusSourcesPanel => Kind::FocusSourcesPanel,
        NativeShellAction::FocusWaveformPanel => Kind::FocusWaveformPanel,
        NativeShellAction::FocusFolderPanel => Kind::FocusFolderPanel,
        NativeShellAction::FocusLoadedSampleInBrowser => Kind::FocusLoadedSampleInBrowser,
        NativeShellAction::OpenAddSourceDialog => Kind::OpenAddSourceDialog,
        NativeShellAction::FocusFolderSearch => Kind::FocusFolderSearch,
        NativeShellAction::SetFolderSearch { .. } => Kind::SetFolderSearch,
        NativeShellAction::ToggleShowAllFolders => Kind::ToggleShowAllFolders,
        NativeShellAction::ToggleFolderFlattenedView => Kind::ToggleFolderFlattenedView,
    }
}
