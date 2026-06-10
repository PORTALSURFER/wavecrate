use super::*;
use crate::app_core::actions::{GUI_ACTION_CATALOG, GuiActionKind, representative_action_for_kind};

#[test]
fn every_catalog_representative_has_a_domain() {
    for entry in GUI_ACTION_CATALOG {
        let action = representative_action_for_kind(entry.kind);
        let _domain = action.domain();
    }
}

#[test]
fn representative_actions_identify_their_primary_domains() {
    let cases = [
        (GuiActionKind::SelectColumn, UiActionDomain::ColumnTriage),
        (GuiActionKind::ToggleTransport, UiActionDomain::Transport),
        (GuiActionKind::OpenOptionsMenu, UiActionDomain::Shell),
        (
            GuiActionKind::FocusSourceRow,
            UiActionDomain::SourcesAndFolders,
        ),
        (GuiActionKind::MoveBrowserFocus, UiActionDomain::Browser),
        (
            GuiActionKind::ConfirmPrompt,
            UiActionDomain::PromptsAndEdits,
        ),
        (
            GuiActionKind::SetInputMonitoringEnabled,
            UiActionDomain::Options,
        ),
        (GuiActionKind::SeekWaveformPrecise, UiActionDomain::Waveform),
        (
            GuiActionKind::CheckForUpdates,
            UiActionDomain::HistoryAndUpdates,
        ),
    ];

    for (kind, expected_domain) in cases {
        let action = representative_action_for_kind(kind);
        assert_eq!(action.domain(), expected_domain, "{kind:?}");
    }
}
