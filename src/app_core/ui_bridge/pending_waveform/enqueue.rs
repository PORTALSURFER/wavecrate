use super::PendingWaveformActions;
use crate::app_core::actions::NativeUiAction;

impl PendingWaveformActions {
    /// Queue a coalescable waveform action and return true when absorbed.
    pub(in crate::app_core::ui_bridge) fn enqueue(&mut self, action: &NativeUiAction) -> bool {
        match action {
            NativeUiAction::SeekWaveformPrecise { position_nanos } => {
                self.seek_nanos = Some(*position_nanos);
                true
            }
            NativeUiAction::SeekWaveform { position_milli } => {
                self.seek_nanos = Some(u32::from((*position_milli).min(1000)) * 1_000_000);
                true
            }
            NativeUiAction::SetWaveformCursorPrecise { position_nanos } => {
                self.cursor_nanos = Some(*position_nanos);
                true
            }
            NativeUiAction::SetWaveformCursor { position_milli } => {
                self.cursor_nanos = Some(u32::from((*position_milli).min(1000)) * 1_000_000);
                true
            }
            NativeUiAction::SetWaveformSelectionRange {
                start_micros,
                end_micros,
                snap_override,
                preserve_view_edge,
            } => {
                self.note_selection_action();
                self.selection_range_micros = Some((*start_micros, *end_micros));
                self.selection_range_nanos = None;
                self.selection_preserve_view_edge = *preserve_view_edge;
                self.selection_snap_override = *snap_override;
                self.selection_smart_scale = false;
                self.clear_selection = false;
                true
            }
            NativeUiAction::SetWaveformSelectionRangePrecise {
                start_nanos,
                end_nanos,
                snap_override,
                preserve_view_edge,
            } => {
                self.note_selection_action();
                let start = (*start_nanos).min(1_000_000_000);
                let end = (*end_nanos).min(1_000_000_000);
                self.selection_range_micros = Some((start / 1000, end / 1000));
                self.selection_range_nanos = Some((start, end));
                self.selection_preserve_view_edge = *preserve_view_edge;
                self.selection_snap_override = *snap_override;
                self.selection_smart_scale = false;
                self.clear_selection = false;
                true
            }
            NativeUiAction::SetWaveformSelectionRangeSmartScale {
                start_micros,
                end_micros,
            } => {
                self.note_selection_action();
                self.selection_range_micros = Some((*start_micros, *end_micros));
                self.selection_range_nanos = None;
                self.selection_preserve_view_edge = false;
                self.selection_snap_override = false;
                self.selection_smart_scale = true;
                self.clear_selection = false;
                true
            }
            NativeUiAction::SetWaveformSelectionRangeSmartScalePrecise {
                start_nanos,
                end_nanos,
            } => {
                self.note_selection_action();
                let start = (*start_nanos).min(1_000_000_000);
                let end = (*end_nanos).min(1_000_000_000);
                self.selection_range_micros = Some((start / 1000, end / 1000));
                self.selection_range_nanos = Some((start, end));
                self.selection_preserve_view_edge = false;
                self.selection_snap_override = false;
                self.selection_smart_scale = true;
                self.clear_selection = false;
                true
            }
            NativeUiAction::ClearWaveformSelection => {
                self.note_selection_action();
                self.selection_range_micros = None;
                self.selection_range_nanos = None;
                self.selection_preserve_view_edge = false;
                self.selection_snap_override = false;
                self.selection_smart_scale = false;
                self.clear_selection = true;
                true
            }
            NativeUiAction::SetWaveformViewCenter {
                center_micros,
                center_nanos,
            } => {
                self.note_view_center_action();
                self.view_center_micros = Some((*center_micros).min(1_000_000));
                self.view_center_nanos = center_nanos.map(|nanos| nanos.min(1_000_000_000));
                true
            }
            NativeUiAction::ZoomWaveform {
                zoom_in,
                steps,
                anchor_ratio_micros,
            } => {
                if self.zoom_full || self.zoom_to_selection {
                    return true;
                }
                self.note_zoom_action();
                let signed_steps = if *zoom_in {
                    i16::from(*steps)
                } else {
                    -i16::from(*steps)
                };
                self.zoom_steps_delta = self.zoom_steps_delta.saturating_add(signed_steps);
                self.zoom_anchor_ratio_micros = if self.zoom_steps_delta == 0 {
                    None
                } else {
                    anchor_ratio_micros.map(|micros| micros.min(1_000_000))
                };
                true
            }
            NativeUiAction::ZoomWaveformToSelection => {
                self.note_zoom_action();
                self.zoom_steps_delta = 0;
                self.zoom_anchor_ratio_micros = None;
                self.zoom_to_selection = true;
                self.zoom_full = false;
                true
            }
            NativeUiAction::ZoomWaveformFull => {
                self.note_zoom_action();
                self.zoom_steps_delta = 0;
                self.zoom_anchor_ratio_micros = None;
                self.zoom_to_selection = false;
                self.zoom_full = true;
                true
            }
            _ => false,
        }
    }
}
