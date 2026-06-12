use super::PendingWaveformActions;
use crate::app_core::actions::NativeUiAction;

impl PendingWaveformActions {
    /// Emit queued waveform actions in deterministic application order.
    pub(in crate::app_core::ui_bridge) fn emit_actions(
        &self,
        mut emit: impl FnMut(NativeUiAction),
    ) -> u64 {
        let mut emitted_actions = 0u64;
        let mut ordered_actions = [
            self.zoom_order
                .map(|order| (order, self.zoom_action()))
                .and_then(|(order, action)| action.map(|action| (order, action))),
            self.selection_order
                .map(|order| (order, self.selection_action()))
                .and_then(|(order, action)| action.map(|action| (order, action))),
            self.view_center_order.and_then(|order| {
                self.view_center_micros.map(|center_micros| {
                    (
                        order,
                        NativeUiAction::SetWaveformViewCenter {
                            center_micros,
                            center_nanos: self.view_center_nanos,
                        },
                    )
                })
            }),
        ];
        ordered_actions.sort_by_key(|entry| entry.as_ref().map(|(order, _)| *order));
        for (_, action) in ordered_actions.into_iter().flatten() {
            emit(action);
            emitted_actions = emitted_actions.saturating_add(1);
        }
        if let Some(position_nanos) = self.deduped_cursor_nanos() {
            emit(NativeUiAction::SetWaveformCursorPrecise { position_nanos });
            emitted_actions = emitted_actions.saturating_add(1);
        }
        if let Some(position_nanos) = self.seek_nanos {
            emit(NativeUiAction::SeekWaveformPrecise { position_nanos });
            emitted_actions = emitted_actions.saturating_add(1);
        }
        emitted_actions
    }
}
