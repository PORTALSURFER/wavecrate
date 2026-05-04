//! Cursor-state and redraw-pacing helpers for the native Vello runner.

use super::super::*;
use crate::gui::focus::FocusSurface;

impl<Bridge> NativeVelloRunner<Bridge>
where
    Bridge: NativeAppBridge,
{
    pub(crate) fn queue_cursor(&mut self, point: Point) {
        self.pending_cursor = Some(point);
    }

    /// Update the native cursor icon only when it changed.
    pub(crate) fn set_cursor_icon(&mut self, icon: CursorIcon) {
        if self.cursor_icon == icon {
            return;
        }
        if let Some(window) = self.window.as_ref() {
            window.set_cursor(icon);
        }
        self.cursor_icon = icon;
    }

    /// Resolve waveform-resize hover cursor state for the current pointer.
    pub(crate) fn update_waveform_resize_cursor(&mut self, point: Point) {
        let icon = if let Some(layout) = self.shell_layout.as_deref() {
            if waveform_selection_drag_handle_hovered(&layout, &self.model, point) {
                CursorIcon::Grab
            } else if waveform_resize_handle_hovered(&layout, &self.model, point) {
                CursorIcon::EwResize
            } else if self
                .shell_state
                .prompt_input_at_point(&layout, &self.model, point)
                || self
                    .shell_state
                    .waveform_bpm_input_at_point(&layout, &self.model, point)
            {
                CursorIcon::Text
            } else {
                CursorIcon::Default
            }
        } else {
            CursorIcon::Default
        };
        self.set_cursor_icon(icon);
    }

    /// Keep one stable cursor icon for the currently captured waveform drag.
    pub(crate) fn update_cursor_for_active_waveform_drag(&mut self) {
        let icon = self
            .waveform_drag_mode
            .map(cursor_icon_for_waveform_drag_mode)
            .unwrap_or(CursorIcon::Default);
        self.set_cursor_icon(icon);
    }

    /// Record recent pointer activity for short-lived high-frequency redraw pacing.
    pub(crate) fn note_cursor_activity(&mut self, now: Instant) {
        self.cursor_activity_redraw_until = Some(now + CURSOR_ACTIVITY_REDRAW_WINDOW);
    }

    /// Return the next redraw deadline while recent cursor activity is active.
    pub(crate) fn next_cursor_activity_redraw_deadline(&mut self, now: Instant) -> Option<Instant> {
        let until = self.cursor_activity_redraw_until?;
        if now >= until {
            self.cursor_activity_redraw_until = None;
            return None;
        }
        let mut next_redraw_at = self.last_redraw + self.cursor_activity_redraw_interval;
        if next_redraw_at < now {
            next_redraw_at = now;
        }
        if next_redraw_at > until {
            next_redraw_at = until;
        }
        Some(next_redraw_at)
    }

    /// Process one cursor move immediately when layout state is available.
    ///
    /// Returns `(processed, handled)` where:
    /// - `processed` indicates whether layout/model state was available now.
    /// - `handled` indicates whether hover state changed and triggered redraw.
    pub(crate) fn process_cursor_move_immediately(&mut self, point: Point) -> (bool, bool) {
        let Some(layout) = self.shell_layout.as_ref() else {
            return (false, false);
        };
        let profile_start = self.profiler.now_if_enabled();
        let effect = self
            .shell_state
            .handle_cursor_move_effect(layout, &self.model, point);
        let handled = effect != CursorMoveEffect::None;
        if handled {
            if let Some(start) = profile_start {
                let kind = if self.model.map.active {
                    InteractionProfileKind::SpatialPanProxy
                } else {
                    InteractionProfileKind::Hover
                };
                self.profiler.add_interaction_latency(kind, start.elapsed());
            }
            match effect {
                CursorMoveEffect::WaveformHoverOnly => {
                    self.apply_invalidation_scope(RuntimeInvalidationScope::OverlayMotionOnly);
                }
                CursorMoveEffect::GeneralOverlay => self.rebuild_overlay_and_request_redraw(),
                CursorMoveEffect::None => {}
            }
        }
        (true, handled)
    }

    /// Return whether one held-key repeat should be processed for navigation.
    pub(crate) fn allows_key_repeat(&self, key: KeyCode) -> bool {
        if self.text_input_target == TextInputTarget::WaveformBpm {
            if self.modifiers.control_key()
                || self.modifiers.super_key()
                || self.modifiers.alt_key()
            {
                return false;
            }
            return matches!(key, KeyCode::ArrowUp | KeyCode::ArrowDown);
        }
        if self.text_input_target != TextInputTarget::None {
            return false;
        }
        if self.modifiers.control_key() || self.modifiers.super_key() {
            return false;
        }
        if self.modifiers.alt_key() {
            return !self.modifiers.shift_key()
                && self.model.focus_context == FocusSurface::Timeline
                && matches!(key, KeyCode::ArrowLeft | KeyCode::ArrowRight);
        }
        if self.modifiers.shift_key() {
            return false;
        }
        matches!(key, KeyCode::ArrowUp | KeyCode::ArrowDown)
    }
}

fn cursor_icon_for_waveform_drag_mode(mode: WaveformPointerDragMode) -> CursorIcon {
    match mode {
        WaveformPointerDragMode::CircularSlide { .. } => CursorIcon::Grab,
        WaveformPointerDragMode::Selection { .. }
        | WaveformPointerDragMode::SelectionSmartScale { .. }
        | WaveformPointerDragMode::EditSelection { .. }
        | WaveformPointerDragMode::EditFadeInEnd
        | WaveformPointerDragMode::EditFadeInMuteStart
        | WaveformPointerDragMode::EditFadeOutStart
        | WaveformPointerDragMode::EditFadeOutMuteEnd => CursorIcon::EwResize,
        WaveformPointerDragMode::SelectionShift { .. }
        | WaveformPointerDragMode::EditSelectionShift { .. } => CursorIcon::Grab,
        WaveformPointerDragMode::Seek
        | WaveformPointerDragMode::Cursor
        | WaveformPointerDragMode::EditFadeInCurve
        | WaveformPointerDragMode::EditFadeOutCurve => CursorIcon::Default,
    }
}
