use crate::selection::{SelectionRange, fade_gain_at_position};

pub(super) fn fade_intersects_view(
    view_start: f32,
    view_end: f32,
    edit_fade: Option<SelectionRange>,
) -> bool {
    let Some(selection) = edit_fade else {
        return false;
    };
    selection.has_edit_effects() && selection.end() >= view_start && selection.start() <= view_end
}

pub(super) fn apply_fade_to_columns(
    columns: &mut [(f32, f32)],
    view_start: f32,
    view_end: f32,
    edit_fade: Option<SelectionRange>,
) {
    if !fade_intersects_view(view_start, view_end, edit_fade) {
        return;
    }
    let Some(selection) = edit_fade else {
        return;
    };
    let column_count = columns.len();
    for (index, column) in columns.iter_mut().enumerate() {
        let position = preview_position_for_index(index, column_count, view_start, view_end);
        let gain = fade_gain_at_position(
            position,
            selection.start(),
            selection.end(),
            selection.gain(),
            selection.fade_in(),
            selection.fade_out(),
            0.0,
        );
        if (gain - 1.0).abs() > f32::EPSILON {
            column.0 *= gain;
            column.1 *= gain;
        }
    }
}

pub(super) fn apply_fade_to_samples(
    samples: &[f32],
    channels: usize,
    frame_count: usize,
    view_start: f32,
    view_end: f32,
    edit_fade: Option<SelectionRange>,
) -> Vec<f32> {
    let Some(selection) = edit_fade else {
        return samples.to_vec();
    };
    let mut faded = samples.to_vec();
    for frame in 0..frame_count {
        let position = preview_position_for_index(frame, frame_count, view_start, view_end);
        let gain = fade_gain_at_position(
            position,
            selection.start(),
            selection.end(),
            selection.gain(),
            selection.fade_in(),
            selection.fade_out(),
            0.0,
        );
        if (gain - 1.0).abs() > f32::EPSILON {
            let base = frame * channels;
            for ch in 0..channels {
                if let Some(sample) = faded.get_mut(base + ch) {
                    *sample *= gain;
                }
            }
        }
    }
    faded
}

fn preview_position_for_index(
    index: usize,
    sample_count: usize,
    view_start: f32,
    view_end: f32,
) -> f32 {
    let fraction = (view_end - view_start).max(1e-6);
    if sample_count <= 1 {
        return view_end;
    }
    let t = index as f32 / (sample_count.saturating_sub(1)) as f32;
    view_start + fraction * t.clamp(0.0, 1.0)
}
