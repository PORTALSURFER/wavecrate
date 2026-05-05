use super::*;

/// Return whether one command-click waveform edge adjustment should emit on press.
pub(super) fn should_emit_waveform_range_adjust_immediately<B: NativeAppBridge>(
    runner: &NativeVelloRunner<B>,
    action: &UiAction,
) -> bool {
    let command = runner.modifiers.control_key() || runner.modifiers.super_key();
    if !runner.modifiers.alt_key()
        && matches!(
            action,
            UiAction::SetWaveformSelectionRange { .. }
                | UiAction::SetWaveformEditSelectionRange { .. }
                | UiAction::SetWaveformSelectionRangePrecise { .. }
                | UiAction::SetWaveformEditSelectionRangePrecise { .. }
        )
    {
        if command {
            return true;
        }
        if runner.modifiers.shift_key() {
            return match action {
                UiAction::SetWaveformSelectionRange { .. }
                | UiAction::SetWaveformSelectionRangePrecise { .. } => {
                    runner.model.waveform.selection_milli.is_some()
                }
                UiAction::SetWaveformEditSelectionRange { .. }
                | UiAction::SetWaveformEditSelectionRangePrecise { .. } => {
                    runner.model.waveform.edit_selection_milli.is_some()
                }
                _ => false,
            };
        }
    }
    false
}
