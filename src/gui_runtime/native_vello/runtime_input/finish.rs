//! Pointer-release and deferred-input helpers for the native Vello runner.

use super::super::*;
use tracing::info;

/// Horizontal click slop used to distinguish waveform clicks from drags.
const WAVEFORM_CLICK_SEEK_SLOP_PX: f32 = 3.0;

impl<Bridge> NativeVelloRunner<Bridge>
where
    Bridge: NativeAppBridge,
{
    pub(crate) fn finish_volume_drag(&mut self, released_button: Option<MouseButton>) {
        let finish_edit_fade_drag = self
            .waveform_drag_mode
            .is_some_and(waveform_drag_mode_is_edit_fade);
        let finish_circular_slide = matches!(released_button, Some(MouseButton::Left))
            && self
                .waveform_drag_mode
                .is_some_and(|mode| matches!(mode, WaveformPointerDragMode::CircularSlide { .. }));
        let finish_selection_range_drag = matches!(released_button, Some(MouseButton::Left))
            && self.waveform_drag_mode.is_some_and(|mode| {
                matches!(
                    mode,
                    WaveformPointerDragMode::Selection { .. }
                        | WaveformPointerDragMode::SelectionShift { .. }
                )
            })
            && self.last_emitted_waveform_drag_action.is_some();
        let finish_edit_selection_drag = matches!(released_button, Some(MouseButton::Left))
            && self.waveform_drag_mode.is_some_and(|mode| {
                matches!(
                    mode,
                    WaveformPointerDragMode::EditSelection { .. }
                        | WaveformPointerDragMode::EditSelectionShift { .. }
                )
            })
            && self.last_emitted_waveform_drag_action.is_some();
        let finish_selection_drag =
            self.selection_drag_active && matches!(released_button, Some(MouseButton::Left));
        let finish_content_item_drag =
            self.content_item_drag.is_some() && matches!(released_button, Some(MouseButton::Left));
        let finish_selection_smart_scale_drag = matches!(released_button, Some(MouseButton::Left))
            && self.waveform_drag_mode.is_some_and(|mode| {
                matches!(mode, WaveformPointerDragMode::SelectionSmartScale { .. })
            });
        let click_seek_press = self.waveform_click_seek_press;
        let seek_on_waveform_click_release = matches!(released_button, Some(MouseButton::Left))
            && self.last_emitted_waveform_drag_action.is_none()
            && click_seek_press.is_some()
            && self
                .waveform_drag_mode
                .is_none_or(|mode| matches!(mode, WaveformPointerDragMode::Selection { .. }))
            && self.last_cursor.is_some_and(|point| {
                click_seek_press.is_some_and(|press| {
                    (point.x - press.press_x).abs() <= WAVEFORM_CLICK_SEEK_SLOP_PX
                })
            });
        let _ = self.flush_pending_volume_action();
        if self.volume_drag_active {
            self.emit_model_action(UiAction::CommitVolumeSetting);
        }
        self.volume_drag_active = false;
        self.last_emitted_volume_milli = None;
        if finish_edit_fade_drag {
            self.emit_model_action(UiAction::FinishWaveformEditFadeDrag);
        }
        if finish_circular_slide {
            self.emit_model_action(UiAction::FinishWaveformCircularSlide);
        }
        if finish_selection_range_drag {
            self.emit_model_action(UiAction::FinishWaveformSelectionRangeDrag);
        }
        if finish_selection_drag {
            self.emit_model_action(UiAction::FinishWaveformSelectionDrag);
        }
        if finish_content_item_drag {
            info!(
                last_cursor_present = self.last_cursor.is_some(),
                "radiant external drag: releasing browser-item drag without external handoff"
            );
            self.emit_model_action(UiAction::FinishContentItemDrag);
        }
        if finish_selection_smart_scale_drag {
            self.emit_model_action(UiAction::FinishWaveformSelectionSmartScaleDrag);
        }
        if finish_edit_selection_drag {
            self.emit_model_action(UiAction::FinishWaveformEditSelectionDrag);
        }
        let browser_row_click_release = matches!(released_button, Some(MouseButton::Left))
            && !finish_content_item_drag
            && self.pending_browser_row_press.is_some();
        let pending_browser_row_press = if browser_row_click_release {
            self.pending_browser_row_press.take()
        } else {
            self.pending_browser_row_press = None;
            None
        };
        self.clear_pointer_drag_session();
        if let Some(point) = self.last_cursor {
            let _ = self.process_cursor_move_immediately(point);
            self.update_waveform_resize_cursor(point);
        }
        if let Some(pending_browser_row_press) = pending_browser_row_press {
            let _ =
                self.emit_pointer_press_action_now(pending_browser_row_press.action, false, None);
        }
        if seek_on_waveform_click_release && let Some(click_seek_press) = click_seek_press {
            if click_seek_press.clear_selection_on_release {
                self.emit_model_action(UiAction::ClearWaveformSelection);
            }
            self.sync_model_after_waveform_click_release();
            self.emit_waveform_click_release_playback(click_seek_press.position_nanos);
            self.sync_model_after_waveform_click_release();
        }
    }

    pub(crate) fn flush_pending_input(&mut self) -> bool {
        let mut pending_action = false;
        if self.flush_pending_volume_action() {
            pending_action = true;
        }
        if let Some(point) = self.pending_cursor.take() {
            let (_, handled) = self.process_cursor_move_immediately(point);
            if handled {
                pending_action = true;
            }
        }
        pending_action
    }

    pub(crate) fn mark_idle_status_refresh_if_due(&mut self, now: Instant) -> bool {
        if now < self.next_idle_status_refresh {
            return false;
        }
        let mut next_refresh = self.next_idle_status_refresh;
        while next_refresh <= now {
            next_refresh += self.idle_status_refresh_interval;
        }
        self.next_idle_status_refresh = next_refresh;
        self.frame_state.mark_motion_overlay_dirty();
        true
    }

    /// Pull the latest host model after click-to-seek release so queued bridge
    /// waveform actions become visible and audible immediately.
    fn sync_model_after_waveform_click_release(&mut self) {
        self.model = self.bridge.pull_model_arc();
        self.waveform_view_refresh_pending = false;
        self.shell_state.sync_from_model(&self.model);
        self.refresh_motion_model_from_model();
        self.sync_text_input_target();
    }

    /// Apply plain waveform click-release playback from one exact pointer point.
    ///
    /// Plain clicks should always audition from the clicked location while
    /// drag gestures still create or resize selections. Emit one exact
    /// click-play action so the host can seek and start transport in the same
    /// reduction path instead of inferring playback from cursor state.
    fn emit_waveform_click_release_playback(&mut self, position_nanos: u32) {
        self.emit_model_action(UiAction::PlayWaveformAtPrecise { position_nanos });
    }
}
