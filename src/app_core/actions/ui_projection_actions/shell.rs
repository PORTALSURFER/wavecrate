use serde::{Deserialize, Serialize};

/// Focus and shell-surface actions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShellAction {
    FocusBrowserPanel,
    FocusSourcesPanel,
    FocusWaveformPanel,
    FocusFolderPanel,
    FocusLoadedSampleInBrowser,
    OpenAddSourceDialog,
    FocusFolderSearch,
    SetFolderSearch { query: String },
    ToggleShowAllFolders,
    ToggleFolderFlattenedView,
}
