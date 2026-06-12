use super::PendingWaveformActions;
use crate::app_core::actions::NativeUiAction;
use crate::app_core::app_api::controller_state::DirtyReason;

impl PendingWaveformActions {
    /// Return the derived-graph dirty reason represented by this pending batch.
    pub(in crate::app_core::ui_bridge) fn dirty_reason(&self) -> DirtyReason {
        if self.zoom_full
            || self.zoom_to_selection
            || self.zoom_steps_delta != 0
            || self.view_center_micros.is_some()
            || self.view_center_nanos.is_some()
            || self.selection_smart_scale
        {
            DirtyReason::WaveformViewAction
        } else {
            DirtyReason::WaveformOverlayAction
        }
    }

    /// Return the cursor update after removing redundant cursor+seek pairs.
    ///
    /// A queued seek already updates cursor position, so sending both actions at
    /// the same normalized target adds no behavior but does add apply cost.
    pub(in crate::app_core::ui_bridge) fn deduped_cursor_nanos(&self) -> Option<u32> {
        if self.cursor_nanos.is_some() && self.cursor_nanos == self.seek_nanos {
            None
        } else {
            self.cursor_nanos
        }
    }

    /// Build the highest-priority zoom action for this pending batch, if any.
    pub(in crate::app_core::ui_bridge) fn zoom_action(&self) -> Option<NativeUiAction> {
        if self.zoom_full {
            return Some(NativeUiAction::ZoomWaveformFull);
        }
        if self.zoom_to_selection {
            return Some(NativeUiAction::ZoomWaveformToSelection);
        }
        if self.zoom_steps_delta == 0 {
            return None;
        }
        let zoom_in = self.zoom_steps_delta.is_positive();
        let steps = self.zoom_steps_delta.unsigned_abs().min(u16::from(u8::MAX)) as u8;
        Some(NativeUiAction::ZoomWaveform {
            zoom_in,
            steps,
            anchor_ratio_micros: self.zoom_anchor_ratio_micros,
        })
    }

    /// Build the highest-priority selection action for this pending batch, if any.
    pub(in crate::app_core::ui_bridge) fn selection_action(&self) -> Option<NativeUiAction> {
        if let Some((start_nanos, end_nanos)) = self.selection_range_nanos {
            return Some(if self.selection_smart_scale {
                NativeUiAction::SetWaveformSelectionRangeSmartScalePrecise {
                    start_nanos,
                    end_nanos,
                }
            } else {
                NativeUiAction::SetWaveformSelectionRangePrecise {
                    start_nanos,
                    end_nanos,
                    snap_override: self.selection_snap_override,
                    preserve_view_edge: self.selection_preserve_view_edge,
                }
            });
        }
        if let Some((start_micros, end_micros)) = self.selection_range_micros {
            return Some(if self.selection_smart_scale {
                NativeUiAction::SetWaveformSelectionRangeSmartScale {
                    start_micros,
                    end_micros,
                }
            } else {
                NativeUiAction::SetWaveformSelectionRange {
                    start_micros,
                    end_micros,
                    snap_override: self.selection_snap_override,
                    preserve_view_edge: self.selection_preserve_view_edge,
                }
            });
        }
        self.clear_selection
            .then_some(NativeUiAction::ClearWaveformSelection)
    }

    /// Return true when queued actions mutate waveform static rendering content.
    ///
    /// Zoom actions change the waveform viewport and image payload, so native
    /// runtime must pull a full projected model instead of motion-only state.
    pub(in crate::app_core::ui_bridge) fn requires_full_model_pull(&self) -> bool {
        self.zoom_steps_delta != 0
            || self.zoom_to_selection
            || self.zoom_full
            || self.view_center_micros.is_some()
            || self.view_center_nanos.is_some()
            || self.selection_smart_scale
    }
}
