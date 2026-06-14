//! Domain-family classification for Wavecrate UI actions.
//!
//! This module names action families explicitly so dispatch, catalog,
//! invalidation, and tests can migrate by domain without treating the root
//! action enum as one undifferentiated API.

use super::UiAction;

/// Stable domain family for a Wavecrate UI action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UiActionDomain {
    /// Column and triage actions.
    ColumnTriage,
    /// Transport actions.
    Transport,
    /// Shell actions.
    Shell,
    /// Source and folder-tree actions.
    SourcesAndFolders,
    /// Browser actions.
    Browser,
    /// Prompt, rename, file-edit, and confirmation actions.
    PromptsAndEdits,
    /// Options actions.
    Options,
    /// Waveform actions.
    Waveform,
    /// History and update actions.
    HistoryAndUpdates,
}

impl UiAction {
    /// Return the domain family that owns this action's behavior.
    pub fn domain(&self) -> UiActionDomain {
        match self {
            UiAction::Transport(_) => UiActionDomain::Transport,
            UiAction::ColumnTriage(_) => UiActionDomain::ColumnTriage,
            UiAction::HistoryAndUpdate(_) => UiActionDomain::HistoryAndUpdates,
            UiAction::Shell(_) => UiActionDomain::Shell,
            UiAction::SourcesAndFolders(_) => UiActionDomain::SourcesAndFolders,
            UiAction::Browser(_) => UiActionDomain::Browser,
            UiAction::PromptsAndEdits(_) => UiActionDomain::PromptsAndEdits,
            UiAction::Options(_) => UiActionDomain::Options,
            UiAction::Waveform(_) => UiActionDomain::Waveform,
        }
    }
}

#[cfg(test)]
mod tests;
