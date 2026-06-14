mod browser;
mod column_triage;
mod history_update;
mod options;
mod prompt_edit;
mod shell;
mod sources_folders;
mod transport;
mod waveform;

use self::browser::browser_action_kind;
use self::column_triage::column_triage_action_kind;
use self::history_update::history_update_action_kind;
use self::options::options_action_kind;
use self::prompt_edit::prompt_edit_action_kind;
use self::shell::shell_action_kind;
use self::sources_folders::sources_folders_action_kind;
use self::transport::transport_action_kind;
use self::waveform::waveform_action_kind;
use super::super::NativeUiAction;
use super::data::gui_action_rows;
use super::{GuiActionKind, GuiActionKind as Kind};

macro_rules! build_representative_action_mapping {
    ($($kind:ident $pattern:tt => {
        id: $id:literal, surface: $surface:ident, effect: $effect:ident,
        coverage: [$($coverage:ident),+ $(,)?],
        fixtures: [$($fixture:literal),* $(,)?], sample: $sample:expr
    }),+ $(,)?) => {
        /// Return a representative action payload for the provided kind.
        pub fn representative_action_for_kind(kind: GuiActionKind) -> NativeUiAction {
            match kind {
                $(Kind::$kind => $sample,)+
            }
        }
    };
}

/// Return the payload-free kind for one concrete UI action.
pub fn action_kind(action: &NativeUiAction) -> GuiActionKind {
    match action {
        NativeUiAction::ColumnTriage(action) => column_triage_action_kind(action),
        NativeUiAction::Transport(action) => transport_action_kind(action),
        NativeUiAction::HistoryAndUpdate(action) => history_update_action_kind(action),
        NativeUiAction::Shell(action) => shell_action_kind(action),
        NativeUiAction::SourcesAndFolders(action) => sources_folders_action_kind(action),
        NativeUiAction::Browser(action) => browser_action_kind(action),
        NativeUiAction::PromptsAndEdits(action) => prompt_edit_action_kind(action),
        NativeUiAction::Options(action) => options_action_kind(action),
        NativeUiAction::Waveform(action) => waveform_action_kind(action),
    }
}

gui_action_rows!(build_representative_action_mapping);

mod shared {
    pub(super) use super::super::super::{
        NativeBrowserAction, NativeColumnTriageAction, NativeHistoryUpdateAction,
        NativeOptionsAction, NativePromptEditAction, NativeShellAction, NativeSourcesFoldersAction,
        NativeTransportAction, NativeWaveformAction,
    };
    pub(super) use super::super::{GuiActionKind, GuiActionKind as Kind};
}
