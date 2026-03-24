//! Waveform cursor, seek, scroll, and zoom action adapters.

use super::*;

/// Equality epsilon used for normalized waveform cursor no-op detection.
const WAVEFORM_CURSOR_NOOP_EPSILON: f32 = 1.0e-6;

impl AppController {
    /// Seek playback to the given normalized position.
    pub fn seek_to(&mut self, position: f64) {
        transport::seek_to(self, position);
    }

    /// Seek waveform/playback using an exact nanounit position from UI actions.
    pub fn seek_waveform_nanos(&mut self, position_nanos: u32) {
        transport::seek_waveform_nanos(self, position_nanos);
    }

    /// Seek waveform/playback using a 0..=1000 milli position from UI actions.
    pub fn seek_waveform_milli(&mut self, position_milli: u16) {
        self.seek_waveform_nanos(nanos_from_milli(position_milli));
    }

    /// Queue a waveform seek from UI actions using exact nanounits.
    pub fn queue_waveform_seek_nanos(&mut self, position_nanos: u32) {
        transport::queue_waveform_seek_nanos(self, position_nanos);
    }

    /// Queue a waveform seek from UI actions and defer commit-side playback work.
    pub fn queue_waveform_seek_milli(&mut self, position_milli: u16) {
        self.queue_waveform_seek_nanos(nanos_from_milli(position_milli));
    }

    /// Set waveform cursor using an exact nanounit position from UI actions.
    pub fn set_waveform_cursor_nanos(&mut self, position_nanos: u32) {
        let normalized = normalized64_from_nanos(position_nanos) as f32;
        let cursor_unchanged =
            self.ui.waveform.cursor.is_some_and(|existing| {
                (existing - normalized).abs() <= WAVEFORM_CURSOR_NOOP_EPSILON
            });
        if cursor_unchanged && waveform_focus_active(self) {
            return;
        }
        self.set_waveform_cursor(normalized);
        self.focus_waveform();
    }

    /// Set waveform cursor using a 0..=1000 milli position from UI actions.
    pub fn set_waveform_cursor_milli(&mut self, position_milli: u16) {
        self.set_waveform_cursor_nanos(nanos_from_milli(position_milli));
    }

    /// Zoom waveform from UI actions using clamped step counts and focus retention.
    pub fn zoom_waveform_steps_from_ui(&mut self, zoom_in: bool, steps: u8) {
        self.zoom_waveform_steps_from_ui_with_anchor(zoom_in, steps, None);
    }

    /// Zoom waveform from UI actions using an optional pointer anchor ratio.
    ///
    /// `anchor_ratio_micros` uses deterministic micros (`0..=1_000_000`) where
    /// `0` is the left edge and `1_000_000` is the right edge of the current view.
    pub fn zoom_waveform_steps_from_ui_with_anchor(
        &mut self,
        zoom_in: bool,
        steps: u8,
        anchor_ratio_micros: Option<u32>,
    ) {
        let before_view = self.ui.waveform.view;
        let focused_before = waveform_focus_active(self);
        let cursor_focus = if let Some(anchor_ratio_micros) = anchor_ratio_micros {
            let ratio =
                f64::from(anchor_ratio_micros.min(1_000_000)) / WAVEFORM_ANCHOR_RATIO_MICROS_SCALE;
            let focus =
                (before_view.start + (before_view.end - before_view.start) * ratio).clamp(0.0, 1.0);
            self.set_waveform_cursor_from_hover(focus as f32);
            focus
        } else if let Some(cursor) = self.ui.waveform.cursor {
            f64::from(cursor)
        } else {
            let center = ((before_view.start + before_view.end) * 0.5).clamp(0.0, 1.0);
            self.set_waveform_cursor(center as f32);
            center
        };
        self.zoom_waveform_steps_with_factor(
            zoom_in,
            zoom_steps_from_ui(steps),
            Some(cursor_focus),
            None,
            false,
            false,
        );
        if focused_before && !waveform_view_changed(before_view, self.ui.waveform.view) {
            return;
        }
        self.focus_waveform_context();
    }

    /// Zoom waveform to current selection while preserving waveform focus.
    pub fn zoom_waveform_to_selection_with_focus(&mut self) {
        self.zoom_waveform_to_selection();
        self.focus_waveform();
    }

    /// Scroll waveform viewport to a normalized center while preserving waveform focus.
    pub fn scroll_waveform_view_with_focus(&mut self, center_micros: u32) {
        self.scroll_waveform_view(
            f64::from(center_micros.min(1_000_000)) / WAVEFORM_ANCHOR_RATIO_MICROS_SCALE,
        );
        self.focus_waveform_context();
    }

    /// Reset waveform zoom to full range while preserving waveform focus.
    pub fn zoom_waveform_full_with_focus(&mut self) {
        self.zoom_waveform_full();
        self.focus_waveform();
    }
}
