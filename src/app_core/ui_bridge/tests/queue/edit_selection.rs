use super::*;

/// Edit-selection actions are applied immediately and must not be coalesced.
#[test]
fn waveform_action_queue_does_not_absorb_edit_selection_actions() {
    let mut queue = PendingWaveformActions::default();
    assert!(!queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformEditSelectionRange {
            start_micros: 140_000,
            end_micros: 460_000,
            preserve_view_edge: false,
        }
    )));
    assert!(!queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeInEnd {
            position_micros: 300_000,
        }
    )));
    assert!(!queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::SetWaveformEditFadeOutStart {
            position_micros: 690_000,
        }
    )));
    assert!(!queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::FinishWaveformEditFadeDrag
    )));
    assert!(!queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::FinishWaveformSelectionRangeDrag
    )));
    assert!(!queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::FinishWaveformEditSelectionDrag
    )));
    assert!(!queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::ClearWaveformEditSelection
    )));
    assert!(!queue.enqueue(&NativeUiAction::Waveform(
        crate::app_core::actions::NativeWaveformAction::ClearWaveformSelections
    )));
    assert!(!queue.has_pending());
}
