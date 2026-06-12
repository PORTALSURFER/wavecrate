use super::*;

#[test]
fn waveform_milli_inputs_upgrade_to_precise_actions() {
    assert_eq!(
        upgrade_compatibility_action(CompatibilityAction::SeekWaveform {
            position_milli: 420,
        }),
        UiAction::SeekWaveformPrecise {
            position_nanos: 420_000_000,
        }
    );
    assert_eq!(
        upgrade_compatibility_action(CompatibilityAction::SetWaveformCursor {
            position_milli: 1_200,
        }),
        UiAction::SetWaveformCursorPrecise {
            position_nanos: 1_000_000_000,
        }
    );
}

#[test]
fn column_inputs_remain_review_compatibility_inputs() {
    let select = CompatibilityAction::SelectColumn { index: 2 };
    assert_eq!(select.clone().policy(), CompatibilityPolicy::Review);
    assert_eq!(
        upgrade_compatibility_action(select),
        UiAction::SelectColumn { index: 2 }
    );

    let move_column = CompatibilityAction::MoveColumn { delta: -1 };
    assert_eq!(move_column.clone().policy(), CompatibilityPolicy::Review);
    assert_eq!(
        upgrade_compatibility_action(move_column),
        UiAction::MoveColumn { delta: -1 }
    );
}

#[test]
fn legacy_json_payloads_parse_in_adapter() {
    let action: CompatibilityAction =
        serde_json::from_value(serde_json::json!({ "SeekWaveform": { "position_milli": 333 } }))
            .expect("legacy seek payload parses");

    assert_eq!(
        action.upgrade(),
        UiAction::SeekWaveformPrecise {
            position_nanos: 333_000_000,
        }
    );
}

#[test]
fn ui_action_boundary_normalizes_retained_compatibility_variants() {
    assert_eq!(
        UiAction::SeekWaveform {
            position_milli: 250,
        }
        .upgrade_compatibility(),
        UiAction::SeekWaveformPrecise {
            position_nanos: 250_000_000,
        }
    );

    assert_eq!(
        UiAction::Transport(crate::app_core::actions::NativeTransportAction::PlayFromStart)
            .upgrade_compatibility(),
        UiAction::Transport(crate::app_core::actions::NativeTransportAction::PlayFromStart)
    );
}

#[test]
fn flat_history_update_inputs_upgrade_to_domain_action() {
    assert_eq!(
        UiAction::CheckForUpdates.upgrade_compatibility(),
        UiAction::HistoryAndUpdate(HistoryUpdateAction::CheckForUpdates)
    );
    assert_eq!(
        UiAction::Undo.upgrade_compatibility(),
        UiAction::HistoryAndUpdate(HistoryUpdateAction::Undo)
    );
}
