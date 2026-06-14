use super::*;

#[test]
fn waveform_milli_inputs_upgrade_to_precise_actions() {
    assert_eq!(
        CompatibilityAction::SeekWaveform {
            position_milli: 420,
        }
        .upgrade(),
        UiAction::Waveform(
            crate::app_core::actions::NativeWaveformAction::SeekWaveformPrecise {
                position_nanos: 420_000_000,
            }
        )
    );
    assert_eq!(
        CompatibilityAction::SetWaveformCursor {
            position_milli: 1_200,
        }
        .upgrade(),
        UiAction::Waveform(
            crate::app_core::actions::NativeWaveformAction::SetWaveformCursorPrecise {
                position_nanos: 1_000_000_000,
            }
        )
    );
}

#[test]
fn column_inputs_upgrade_to_current_column_triage_actions() {
    let select = CompatibilityAction::SelectColumn { index: 2 };
    assert_eq!(
        select.upgrade(),
        UiAction::ColumnTriage(ColumnTriageAction::SelectColumn { index: 2 })
    );

    let move_column = CompatibilityAction::MoveColumn { delta: -1 };
    assert_eq!(
        move_column.upgrade(),
        UiAction::ColumnTriage(ColumnTriageAction::MoveColumn { delta: -1 })
    );
}

#[test]
fn current_column_catalog_samples_use_column_triage_actions() {
    assert!(matches!(
        crate::app_core::actions::representative_action_for_kind(
            crate::app_core::actions::GuiActionKind::SelectColumn
        ),
        UiAction::ColumnTriage(ColumnTriageAction::SelectColumn { index: 1 })
    ));
    assert!(matches!(
        crate::app_core::actions::representative_action_for_kind(
            crate::app_core::actions::GuiActionKind::MoveColumn
        ),
        UiAction::ColumnTriage(ColumnTriageAction::MoveColumn { delta: 1 })
    ));
}

#[test]
fn legacy_json_payloads_parse_in_adapter() {
    let action: CompatibilityAction =
        serde_json::from_value(serde_json::json!({ "SeekWaveform": { "position_milli": 333 } }))
            .expect("legacy seek payload parses");

    assert_eq!(
        action.upgrade(),
        UiAction::Waveform(
            crate::app_core::actions::NativeWaveformAction::SeekWaveformPrecise {
                position_nanos: 333_000_000,
            }
        )
    );
}

#[test]
fn ui_action_boundary_normalizes_retained_compatibility_variants() {
    assert_eq!(
        RetainedUiAction::Compatibility(CompatibilityAction::SeekWaveform {
            position_milli: 250,
        })
        .into_current(),
        UiAction::Waveform(
            crate::app_core::actions::NativeWaveformAction::SeekWaveformPrecise {
                position_nanos: 250_000_000,
            }
        )
    );

    assert_eq!(
        RetainedUiAction::Current(UiAction::Transport(
            crate::app_core::actions::NativeTransportAction::PlayFromStart
        ))
        .into_current(),
        UiAction::Transport(crate::app_core::actions::NativeTransportAction::PlayFromStart)
    );
}

#[test]
fn flat_history_update_inputs_upgrade_to_domain_action() {
    assert_eq!(
        RetainedUiAction::Compatibility(CompatibilityAction::CheckForUpdates).into_current(),
        UiAction::HistoryAndUpdate(HistoryUpdateAction::CheckForUpdates)
    );
    assert_eq!(
        RetainedUiAction::Compatibility(CompatibilityAction::Undo).into_current(),
        UiAction::HistoryAndUpdate(HistoryUpdateAction::Undo)
    );
}

#[test]
fn flat_history_update_inputs_are_adapter_owned() {
    let parsed: RetainedUiAction =
        serde_json::from_value(serde_json::json!("DismissUpdate")).expect("parse flat update");

    assert_eq!(
        parsed,
        RetainedUiAction::Compatibility(CompatibilityAction::DismissUpdate)
    );
    assert_eq!(
        parsed.into_current(),
        UiAction::HistoryAndUpdate(HistoryUpdateAction::DismissUpdate)
    );

    let root_source = include_str!("../../ui_projection_actions.rs");
    for variant in [
        "\n    SelectColumn {\n",
        "\n    MoveColumn {\n",
        "\n    SeekWaveform {\n",
        "\n    SetWaveformCursor {\n",
        "\n    Undo,\n",
        "\n    Redo,\n",
        "\n    CheckForUpdates,\n",
        "\n    OpenUpdateLink,\n",
        "\n    InstallUpdate,\n",
        "\n    DismissUpdate,\n",
    ] {
        assert!(
            !root_source.contains(variant),
            "compatibility-only variant leaked back into UiAction: {variant:?}"
        );
    }
}

#[test]
fn active_domain_variants_are_not_root_owned() {
    let root_source = include_str!("../../ui_projection_actions.rs");
    for variant in [
        "\n    FocusBrowserPanel,\n",
        "\n    SetFolderSearch {\n",
        "\n    FocusSourceRow {\n",
        "\n    MoveBrowserFocus {\n",
        "\n    SetPromptInput {\n",
        "\n    ToggleLoopPlayback,\n",
        "\n    SetWaveformChannelView {\n",
        "\n    SeekWaveformPrecise {\n",
        "\n    SetWaveformSelectionRange {\n",
        "\n    ZoomWaveformFull,\n",
    ] {
        assert!(
            !root_source.contains(variant),
            "domain-owned variant leaked back into root UiAction: {variant:?}"
        );
    }
}
