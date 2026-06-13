//! Wavecrate-owned UI runtime action DTOs.
//!
//! These actions describe Wavecrate user intent inside app-core. Retained
//! legacy input shapes are owned by the sibling compatibility adapter so the
//! primary action contract can stay focused on active behavior.

use serde::{Deserialize, Serialize};

mod browser;
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
pub use self::compatibility::{CompatibilityAction, upgrade_compatibility_action};
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
    #[serde(untagged)]
    Compatibility(CompatibilityAction),
}

#[allow(non_upper_case_globals)]
impl UiAction {
    pub const Undo: Self = Self::Compatibility(CompatibilityAction::Undo);
    pub const Redo: Self = Self::Compatibility(CompatibilityAction::Redo);
    pub const CheckForUpdates: Self = Self::Compatibility(CompatibilityAction::CheckForUpdates);
    pub const OpenUpdateLink: Self = Self::Compatibility(CompatibilityAction::OpenUpdateLink);
    pub const InstallUpdate: Self = Self::Compatibility(CompatibilityAction::InstallUpdate);
    pub const DismissUpdate: Self = Self::Compatibility(CompatibilityAction::DismissUpdate);
}
