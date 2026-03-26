use super::*;

pub(crate) trait WaveformActions {
    fn focus_waveform(&mut self);
    fn zoom_waveform_steps_with_factor(
        &mut self,
        zoom_in: bool,
        steps: u32,
        focus: Option<f64>,
        factor_override: Option<f32>,
        playhead_focus_when_playing: bool,
        keep_playhead_visible: bool,
    );
    fn nudge_selection_range(&mut self, steps: isize, fine: bool);
    fn slide_selection_range(&mut self, steps: isize);
    fn scroll_waveform_view(&mut self, center: f64);
}

impl WaveformActions for WaveformController<'_> {
    fn focus_waveform(&mut self) {
        if self.waveform_ready() {
            self.focus_waveform_context();
            self.ensure_playhead_visible_in_view();
        } else if self.sample_view.wav.selected_wav.is_some() || self.ui.waveform.loading.is_some()
        {
            self.focus_waveform_context();
        } else {
            self.set_status("Load a sample to focus the waveform", StatusTone::Info);
        }
    }

    fn zoom_waveform_steps_with_factor(
        &mut self,
        zoom_in: bool,
        steps: u32,
        focus: Option<f64>,
        factor_override: Option<f32>,
        playhead_focus_when_playing: bool,
        keep_playhead_visible: bool,
    ) {
        if !self.waveform_ready() {
            return;
        }
        let changed = self.apply_zoom_steps(
            zoom_in,
            steps.max(1),
            focus,
            factor_override,
            playhead_focus_when_playing,
            keep_playhead_visible,
        );
        if changed {
            self.refresh_waveform_image();
        }
    }

    fn nudge_selection_range(&mut self, steps: isize, fine: bool) {
        if !self.waveform_ready() {
            return;
        }
        let step = if fine {
            self.waveform_step_size(true)
        } else {
            self.bpm_snap_step()
                .unwrap_or_else(|| self.waveform_step_size(false))
        };
        if step <= 0.0 {
            return;
        }
        let Some(selection) = self
            .selection_state
            .range
            .range()
            .or(self.ui.waveform.selection)
        else {
            self.set_status("Create a selection first", StatusTone::Info);
            return;
        };
        let before = Some(selection);
        let delta = step * steps as f32;
        let range = selection.shift(delta);
        self.selection_state.range.set_range(Some(range));
        self.apply_selection(Some(range));
        self.ensure_selection_visible_in_view(range);
        self.refresh_loop_after_selection_change(range);
        self.push_selection_undo("Selection", before, Some(range));
    }

    fn slide_selection_range(&mut self, steps: isize) {
        if !self.waveform_ready() {
            return;
        }
        let Some(selection) = self
            .selection_state
            .range
            .range()
            .or(self.ui.waveform.selection)
        else {
            self.set_status("Create a selection first", StatusTone::Info);
            return;
        };
        let before = Some(selection);
        let delta = slide_selection_delta(selection, steps, self.bpm_snap_step());
        let range = selection.shift(delta);
        self.selection_state.range.set_range(Some(range));
        self.apply_selection(Some(range));
        self.ensure_selection_visible_in_view(range);
        self.refresh_loop_after_selection_change(range);
        self.push_selection_undo("Selection", before, Some(range));
    }

    fn scroll_waveform_view(&mut self, center: f64) {
        let view = self.ui.waveform.view; // Use actual view, not display_view
        let width = view.width();
        if width >= 1.0 {
            self.ui.waveform.view = WaveformView {
                start: 0.0,
                end: 1.0,
            };
            self.refresh_waveform_image();
            return;
        }
        let half = width * 0.5;
        let start = (center - half).clamp(0.0, 1.0 - width);
        self.ui.waveform.view = WaveformView {
            start,
            end: (start + width).min(1.0),
        }
        .clamp();
        self.refresh_waveform_image();
    }
}

/// Resolve the normalized translation delta for one selection slide request.
fn slide_selection_delta(
    selection: SelectionRange,
    steps: isize,
    bpm_snap_step: Option<f32>,
) -> f32 {
    let width = selection.width();
    if steps == 0 || !width.is_finite() || width <= 0.0 {
        return 0.0;
    }
    let requested_start = selection.start() + (width * steps as f32);
    let clamped_start = requested_start.clamp(0.0, (1.0 - width).max(0.0));
    let snapped_start = bpm_snap_step
        .filter(|step| step.is_finite() && *step > 0.0)
        .map(|step| {
            let snapped_delta = crate::app::controller::playback::snap_waveform_delta_to_bpm_step(
                clamped_start - selection.start(),
                step,
            );
            (selection.start() + snapped_delta).clamp(0.0, (1.0 - width).max(0.0))
        })
        .unwrap_or(clamped_start);
    snapped_start - selection.start()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slide_selection_delta_moves_by_full_selection_width_without_snap() {
        let selection = SelectionRange::new(0.2, 0.35);

        let delta = slide_selection_delta(selection, 1, None);

        assert!((delta - selection.width()).abs() < 1.0e-6);
    }

    #[test]
    fn slide_selection_delta_snaps_translated_start_instead_of_raw_delta() {
        let selection = SelectionRange::new(0.2, 0.4);

        let delta = slide_selection_delta(selection, 1, Some(0.125));

        assert!((delta - 0.25).abs() < 1.0e-6);
        let shifted = selection.shift(delta);
        assert!((shifted.start() - 0.45).abs() < 1.0e-6);
        assert!((shifted.width() - selection.width()).abs() < 1.0e-6);
    }
}
