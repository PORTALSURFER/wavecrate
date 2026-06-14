//! Wavecrate-owned UI runtime action DTOs.
//!
//! These actions describe Wavecrate user intent inside app-core. Retained
//! legacy input shapes are owned by the sibling compatibility adapter so the
//! primary action contract can stay focused on active behavior.

use serde::{Deserialize, Serialize};

mod browser;
mod column_triage;
mod compatibility;
mod domain;
mod history_update;
mod options;
#[cfg(test)]
mod precision_eq;
mod prompt_edit;
mod shell;
mod sources_folders;
mod transport;
mod waveform;

pub use self::browser::{BrowserAction, BrowserTagTarget};
pub use self::column_triage::ColumnTriageAction;
pub use self::compatibility::RetainedUiAction;
pub use self::domain::UiActionDomain;
pub use self::history_update::HistoryUpdateAction;
pub use self::options::OptionsAction;
pub use self::prompt_edit::PromptEditAction;
pub use self::shell::ShellAction;
pub use self::sources_folders::SourcesFoldersAction;
pub use self::transport::TransportAction;
pub use self::waveform::WaveformAction;

#[cfg_attr(not(test), derive(PartialEq, Eq))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum UiAction {
    ColumnTriage(ColumnTriageAction),
    Transport(TransportAction),
    HistoryAndUpdate(HistoryUpdateAction),
    #[serde(untagged)]
    Shell(ShellAction),
    #[serde(untagged)]
    SourcesAndFolders(SourcesFoldersAction),
    #[serde(untagged)]
    Browser(BrowserAction),
    #[serde(untagged)]
    PromptsAndEdits(PromptEditAction),
    #[serde(untagged)]
    Options(OptionsAction),
    #[serde(untagged)]
    Waveform(WaveformAction),
}

#[allow(non_upper_case_globals)]
impl UiAction {
    pub const Undo: Self = Self::HistoryAndUpdate(HistoryUpdateAction::Undo);
    pub const Redo: Self = Self::HistoryAndUpdate(HistoryUpdateAction::Redo);
    pub const CheckForUpdates: Self = Self::HistoryAndUpdate(HistoryUpdateAction::CheckForUpdates);
    pub const OpenUpdateLink: Self = Self::HistoryAndUpdate(HistoryUpdateAction::OpenUpdateLink);
    pub const InstallUpdate: Self = Self::HistoryAndUpdate(HistoryUpdateAction::InstallUpdate);
    pub const DismissUpdate: Self = Self::HistoryAndUpdate(HistoryUpdateAction::DismissUpdate);
}
