use super::WaveformEditFadeHandle;

#[derive(Clone, Copy, Debug)]
pub(in crate::gui_app) struct WaveformEditFadeDrag {
    pub(in crate::gui_app) handle: WaveformEditFadeHandle,
    pub(in crate::gui_app) fixed_ratio: f32,
    pub(in crate::gui_app) curve: f32,
    pub(in crate::gui_app) baseline: wavecrate::selection::SelectionRange,
}

impl WaveformEditFadeDrag {
    pub(in crate::gui_app) fn new(
        handle: WaveformEditFadeHandle,
        selection: wavecrate::selection::SelectionRange,
    ) -> Self {
        let curve = match handle {
            WaveformEditFadeHandle::FadeInEnd
            | WaveformEditFadeHandle::FadeInStart
            | WaveformEditFadeHandle::FadeInOuterStart => {
                selection.fade_in().map(|fade| fade.curve).unwrap_or(0.5)
            }
            WaveformEditFadeHandle::FadeOutStart
            | WaveformEditFadeHandle::FadeOutEnd
            | WaveformEditFadeHandle::FadeOutOuterEnd => {
                selection.fade_out().map(|fade| fade.curve).unwrap_or(0.5)
            }
        };
        let fixed_ratio = match handle {
            WaveformEditFadeHandle::FadeInStart => selection
                .fade_in()
                .map(|fade| selection.start() + selection.width() * fade.length)
                .unwrap_or(selection.start()),
            WaveformEditFadeHandle::FadeOutEnd => selection
                .fade_out()
                .map(|fade| selection.end() - selection.width() * fade.length)
                .unwrap_or(selection.end()),
            WaveformEditFadeHandle::FadeInEnd
            | WaveformEditFadeHandle::FadeOutStart
            | WaveformEditFadeHandle::FadeInOuterStart
            | WaveformEditFadeHandle::FadeOutOuterEnd => 0.0,
        };
        Self {
            handle,
            fixed_ratio,
            curve,
            baseline: selection,
        }
    }

    pub(in crate::gui_app) fn apply(
        self,
        selection: wavecrate::selection::SelectionRange,
        ratio: f32,
    ) -> wavecrate::selection::SelectionRange {
        let ratio = ratio.clamp(0.0, 1.0);
        match self.handle {
            WaveformEditFadeHandle::FadeInEnd => {
                resize_fade_in_end_with_collision(selection, self.baseline, ratio, self.curve)
            }
            WaveformEditFadeHandle::FadeOutStart => {
                resize_fade_out_start_with_collision(selection, self.baseline, ratio, self.curve)
            }
            WaveformEditFadeHandle::FadeInStart => {
                resize_fade_in_start(self.baseline, self.fixed_ratio, ratio, self.curve)
            }
            WaveformEditFadeHandle::FadeOutEnd => {
                resize_fade_out_end(self.baseline, self.fixed_ratio, ratio, self.curve)
            }
            WaveformEditFadeHandle::FadeInOuterStart => {
                resize_fade_in_outer_start(selection, ratio)
            }
            WaveformEditFadeHandle::FadeOutOuterEnd => resize_fade_out_outer_end(selection, ratio),
        }
    }
}

fn fade_in_length_for_end(selection: wavecrate::selection::SelectionRange, end_ratio: f32) -> f32 {
    if selection.width() <= f32::EPSILON {
        return 0.0;
    }
    ((end_ratio.clamp(selection.start(), selection.end()) - selection.start()) / selection.width())
        .clamp(0.0, 1.0)
}

fn fade_out_length_for_start(
    selection: wavecrate::selection::SelectionRange,
    start_ratio: f32,
) -> f32 {
    if selection.width() <= f32::EPSILON {
        return 0.0;
    }
    ((selection.end() - start_ratio.clamp(selection.start(), selection.end())) / selection.width())
        .clamp(0.0, 1.0)
}

fn resize_fade_in_end_with_collision(
    selection: wavecrate::selection::SelectionRange,
    baseline: wavecrate::selection::SelectionRange,
    end_ratio: f32,
    curve: f32,
) -> wavecrate::selection::SelectionRange {
    let width = selection.width();
    if width <= f32::EPSILON {
        return selection;
    }
    let start = selection.start();
    let end = selection.end();
    let fade_in_end = end_ratio.clamp(start, end);
    let fade_in_abs = fade_in_end - start;
    let baseline_fade_out_abs = baseline.fade_out().map_or(0.0, |fade| {
        (baseline.end() - (baseline.end() - baseline.width() * fade.length)).max(0.0)
    });
    let baseline_fade_out_start = end - baseline_fade_out_abs;
    let fade_out_abs = if fade_in_end > baseline_fade_out_start {
        (end - fade_in_end).max(0.0)
    } else {
        baseline_fade_out_abs
    };
    rebuild_edit_fades_for_same_range(
        selection,
        Some((fade_in_abs / width, curve)),
        fade_out_for_same_width(selection, baseline, fade_out_abs).map(|length| {
            (
                length,
                baseline.fade_out().map(|fade| fade.curve).unwrap_or(0.5),
            )
        }),
    )
}

fn resize_fade_out_start_with_collision(
    selection: wavecrate::selection::SelectionRange,
    baseline: wavecrate::selection::SelectionRange,
    start_ratio: f32,
    curve: f32,
) -> wavecrate::selection::SelectionRange {
    let width = selection.width();
    if width <= f32::EPSILON {
        return selection;
    }
    let start = selection.start();
    let end = selection.end();
    let fade_out_start = start_ratio.clamp(start, end);
    let fade_out_abs = end - fade_out_start;
    let baseline_fade_in_abs = baseline.fade_in().map_or(0.0, |fade| {
        ((baseline.start() + baseline.width() * fade.length) - baseline.start()).max(0.0)
    });
    let baseline_fade_in_end = start + baseline_fade_in_abs;
    let fade_in_abs = if fade_out_start < baseline_fade_in_end {
        (fade_out_start - start).max(0.0)
    } else {
        baseline_fade_in_abs
    };
    rebuild_edit_fades_for_same_range(
        selection,
        fade_in_for_same_width(selection, baseline, fade_in_abs).map(|length| {
            (
                length,
                baseline.fade_in().map(|fade| fade.curve).unwrap_or(0.5),
            )
        }),
        Some((fade_out_abs / width, curve)),
    )
}

fn resize_fade_in_outer_start(
    selection: wavecrate::selection::SelectionRange,
    outer_start_ratio: f32,
) -> wavecrate::selection::SelectionRange {
    let Some(fade) = selection.fade_in() else {
        return selection;
    };
    let width = selection.width();
    if width <= f32::EPSILON {
        return selection;
    }
    let outer_start = outer_start_ratio.clamp(0.0, selection.start());
    let mute =
        ((selection.start() - outer_start) / width).clamp(0.0, selection.max_fade_in_mute_length());
    selection
        .with_fade_in(fade.length, fade.curve)
        .with_fade_in_mute(mute)
}

fn resize_fade_out_outer_end(
    selection: wavecrate::selection::SelectionRange,
    outer_end_ratio: f32,
) -> wavecrate::selection::SelectionRange {
    let Some(fade) = selection.fade_out() else {
        return selection;
    };
    let width = selection.width();
    if width <= f32::EPSILON {
        return selection;
    }
    let outer_end = outer_end_ratio.clamp(selection.end(), 1.0);
    let mute =
        ((outer_end - selection.end()) / width).clamp(0.0, selection.max_fade_out_mute_length());
    selection
        .with_fade_out(fade.length, fade.curve)
        .with_fade_out_mute(mute)
}

fn rebuild_edit_fades_for_same_range(
    selection: wavecrate::selection::SelectionRange,
    fade_in: Option<(f32, f32)>,
    fade_out: Option<(f32, f32)>,
) -> wavecrate::selection::SelectionRange {
    let mut rebuilt = wavecrate::selection::SelectionRange::new(selection.start(), selection.end())
        .with_gain(selection.gain());
    if let Some((length, curve)) = fade_in {
        let mute = selection.fade_in().map(|fade| fade.mute).unwrap_or(0.0);
        rebuilt = rebuilt.with_fade_in_and_mute(length.clamp(0.0, 1.0), curve, mute);
    }
    if let Some((length, curve)) = fade_out {
        let mute = selection.fade_out().map(|fade| fade.mute).unwrap_or(0.0);
        rebuilt = rebuilt.with_fade_out_and_mute(length.clamp(0.0, 1.0), curve, mute);
    }
    rebuilt
}

fn fade_in_for_same_width(
    selection: wavecrate::selection::SelectionRange,
    baseline: wavecrate::selection::SelectionRange,
    fade_in_abs: f32,
) -> Option<f32> {
    baseline.fade_in()?;
    Some((fade_in_abs / selection.width().max(f32::EPSILON)).clamp(0.0, 1.0))
}

fn fade_out_for_same_width(
    selection: wavecrate::selection::SelectionRange,
    baseline: wavecrate::selection::SelectionRange,
    fade_out_abs: f32,
) -> Option<f32> {
    baseline.fade_out()?;
    Some((fade_out_abs / selection.width().max(f32::EPSILON)).clamp(0.0, 1.0))
}

fn resize_fade_in_start(
    selection: wavecrate::selection::SelectionRange,
    fade_end: f32,
    start_ratio: f32,
    curve: f32,
) -> wavecrate::selection::SelectionRange {
    let new_start = start_ratio.clamp(0.0, selection.end());
    let old_width = selection.width();
    let mut resized = wavecrate::selection::SelectionRange::new(new_start, selection.end());
    if let Some(fade_out) = selection.fade_out() {
        let fade_out_abs = old_width * fade_out.length;
        let length = if resized.width() <= f32::EPSILON {
            0.0
        } else {
            (fade_out_abs / resized.width()).clamp(0.0, 1.0)
        };
        let old_outer_end = selection.end() + old_width * fade_out.mute;
        let mute = if fade_out.mute <= 0.0 || resized.width() <= f32::EPSILON {
            0.0
        } else {
            fade_out_mute_for_outer_end(resized, old_outer_end)
        };
        resized = resized.with_fade_out_and_mute(length, fade_out.curve, mute);
    }
    let length = fade_in_length_for_end(resized, fade_end);
    let mut resized = resized.with_fade_in(length, curve);
    if let Some(fade_in) = selection.fade_in() {
        let old_outer_start = selection.start() - old_width * fade_in.mute;
        let mute = if fade_in.mute <= 0.0 || resized.width() <= f32::EPSILON {
            0.0
        } else {
            fade_in_mute_for_outer_start(resized, old_outer_start)
        };
        resized = resized.with_fade_in_and_mute(length, curve, mute);
    }
    resized
}

fn resize_fade_out_end(
    selection: wavecrate::selection::SelectionRange,
    fade_start: f32,
    end_ratio: f32,
    curve: f32,
) -> wavecrate::selection::SelectionRange {
    let new_end = end_ratio.clamp(selection.start(), 1.0);
    let old_width = selection.width();
    let mut resized = wavecrate::selection::SelectionRange::new(selection.start(), new_end);
    if let Some(fade_in) = selection.fade_in() {
        let fade_in_abs = old_width * fade_in.length;
        let length = if resized.width() <= f32::EPSILON {
            0.0
        } else {
            (fade_in_abs / resized.width()).clamp(0.0, 1.0)
        };
        let old_outer_start = selection.start() - old_width * fade_in.mute;
        let mute = if fade_in.mute <= 0.0 || resized.width() <= f32::EPSILON {
            0.0
        } else {
            fade_in_mute_for_outer_start(resized, old_outer_start)
        };
        resized = resized.with_fade_in_and_mute(length, fade_in.curve, mute);
    }
    let length = fade_out_length_for_start(resized, fade_start);
    let mut resized = resized.with_fade_out(length, curve);
    if let Some(fade_out) = selection.fade_out() {
        let old_outer_end = selection.end() + old_width * fade_out.mute;
        let mute = if fade_out.mute <= 0.0 || resized.width() <= f32::EPSILON {
            0.0
        } else {
            fade_out_mute_for_outer_end(resized, old_outer_end)
        };
        resized = resized.with_fade_out_and_mute(length, curve, mute);
    }
    resized
}

fn fade_in_mute_for_outer_start(
    selection: wavecrate::selection::SelectionRange,
    outer_start: f32,
) -> f32 {
    if selection.width() <= f32::EPSILON {
        return 0.0;
    }
    let outer_start = snap_to_sample_edge(outer_start).clamp(0.0, selection.start());
    ((selection.start() - outer_start) / selection.width()).max(0.0)
}

fn fade_out_mute_for_outer_end(
    selection: wavecrate::selection::SelectionRange,
    outer_end: f32,
) -> f32 {
    if selection.width() <= f32::EPSILON {
        return 0.0;
    }
    let outer_end = snap_to_sample_edge(outer_end).clamp(selection.end(), 1.0);
    ((outer_end - selection.end()) / selection.width()).max(0.0)
}

fn snap_to_sample_edge(position: f32) -> f32 {
    const EDGE_EPSILON: f32 = 1.0e-6;
    if position <= EDGE_EPSILON {
        0.0
    } else if position >= 1.0 - EDGE_EPSILON {
        1.0
    } else {
        position
    }
}
