use super::helpers::{CursorUpdateSource, MIN_VIEW_WIDTH_BASE, VIEW_EPSILON, views_differ};
use super::*;

impl WaveformController<'_> {
    pub(crate) fn apply_zoom_step(
        &mut self,
        zoom_in: bool,
        focus: Option<f64>,
        factor_override: Option<f32>,
        playhead_focus_when_playing: bool,
        keep_playhead_visible: bool,
    ) -> bool {
        let width_factor = self.zoom_width_factor(zoom_in, 1, factor_override);
        self.apply_zoom_width_factor(
            width_factor,
            focus,
            playhead_focus_when_playing,
            keep_playhead_visible,
        )
    }

    /// Apply zoom with an explicit step count in a single width-factor solve.
    pub(crate) fn apply_zoom_steps(
        &mut self,
        zoom_in: bool,
        steps: u32,
        focus: Option<f64>,
        factor_override: Option<f32>,
        playhead_focus_when_playing: bool,
        keep_playhead_visible: bool,
    ) -> bool {
        let width_factor = self.zoom_width_factor(zoom_in, steps.max(1), factor_override);
        self.apply_zoom_width_factor(
            width_factor,
            focus,
            playhead_focus_when_playing,
            keep_playhead_visible,
        )
    }

    /// Resolve the multiplicative view-width factor for one or more zoom steps.
    fn zoom_width_factor(&self, zoom_in: bool, steps: u32, factor_override: Option<f32>) -> f64 {
        let default_factor = self.ui.controls.keyboard_zoom_factor.max(0.01);
        let base = factor_override.unwrap_or(default_factor).max(0.01) as f64;
        let steps = steps.max(1);
        let factor = if zoom_in {
            base.powf(steps as f64)
        } else {
            base.powf(-(steps as f64))
        };
        if factor.is_finite() {
            factor
        } else if zoom_in {
            0.0
        } else {
            f64::INFINITY
        }
    }

    /// Apply a precomputed width factor while preserving existing zoom focus behavior.
    fn apply_zoom_width_factor(
        &mut self,
        width_factor: f64,
        focus: Option<f64>,
        playhead_focus_when_playing: bool,
        keep_playhead_visible: bool,
    ) -> bool {
        if !self.waveform_ready() {
            return false;
        }
        let focus_from_pointer = focus.is_some();
        let original = self.ui.waveform.view;
        let focus = if playhead_focus_when_playing && self.is_playing() {
            self.ui.waveform.playhead.visible = true;
            self.ui.waveform.playhead.position as f64
        } else {
            focus.unwrap_or_else(|| self.waveform_focus_point())
        };
        let width = (original.width() * width_factor.max(0.0)).clamp(MIN_VIEW_WIDTH_BASE, 1.0);
        if (width - original.width()).abs() <= VIEW_EPSILON {
            return false;
        }
        self.ui.waveform.suppress_hover_cursor = !focus_from_pointer;
        if focus.is_finite() && focus_from_pointer {
            self.set_waveform_cursor_with_source(focus as f32, CursorUpdateSource::Hover);
        }
        let mut view = original;
        if focus_from_pointer {
            let ratio = ((focus - original.start) / original.width()).clamp(0.0, 1.0);
            view.start = focus - width * ratio;
            view.end = view.start + width;
        } else {
            view.start = focus - width * 0.5;
            view.end = focus + width * 0.5;
        }
        self.ui.waveform.view = view.clamp();
        if keep_playhead_visible && self.ui.waveform.cursor.is_none() {
            self.ensure_playhead_visible_in_view();
        }
        views_differ(original, self.ui.waveform.view)
    }

    pub(crate) fn zoom_to_selection(&mut self) {
        if !self.waveform_ready() {
            return;
        }
        let Some(selection) = self
            .selection_state
            .range
            .range()
            .or(self.ui.waveform.selection)
        else {
            self.zoom_waveform_steps_with_factor(true, 1, None, None, true, true);
            return;
        };
        if (selection.width() as f64) <= VIEW_EPSILON {
            self.zoom_waveform_steps_with_factor(
                true,
                1,
                Some(selection.start() as f64),
                None,
                true,
                true,
            );
            return;
        }

        let width = (selection.width() as f64).max(self.min_view_width());
        let center = ((selection.start() + selection.end()) * 0.5) as f64;
        let start = (center - width * 0.5).clamp(0.0, 1.0 - width);
        let view = WaveformView {
            start,
            end: (start + width).min(1.0),
        }
        .clamp();
        if views_differ(self.ui.waveform.view, view) {
            self.ui.waveform.view = view;
            self.refresh_waveform_image();
        }
    }

    pub(crate) fn zoom_out_full(&mut self) {
        if !self.waveform_ready() {
            return;
        }
        let view = WaveformView {
            start: 0.0,
            end: 1.0,
        };
        if views_differ(self.ui.waveform.view, view) {
            self.ui.waveform.view = view;
            self.refresh_waveform_image();
        }
    }
}
