use super::{apply_muted_selection, clamped_selection_span};
use crate::app::controller::library::selection_edits::FadeDirection;

pub(crate) fn apply_directional_fade(
    samples: &mut [f32],
    channels: usize,
    start_frame: usize,
    end_frame: usize,
    direction: FadeDirection,
) {
    let channels = channels.max(1);
    let total_frames = samples.len() / channels;
    let (clamped_start, clamped_end) = clamped_selection_span(total_frames, start_frame, end_frame);
    if clamped_end <= clamped_start {
        return;
    }
    apply_fade_ramp(
        samples,
        channels,
        clamped_start,
        clamped_end,
        direction,
        0.5,
    );
    match direction {
        FadeDirection::LeftToRight => {
            apply_muted_selection(samples, channels, clamped_end, total_frames);
        }
        FadeDirection::RightToLeft => {
            apply_muted_selection(samples, channels, 0, clamped_start);
        }
    }
}

/// Apply fade-in and fade-out ramps at the edges of the selected span.
pub(crate) fn apply_edge_fades(
    samples: &mut [f32],
    channels: usize,
    start_frame: usize,
    end_frame: usize,
    fade_frames: usize,
) {
    let channels = channels.max(1);
    let total_frames = samples.len() / channels;
    let (clamped_start, clamped_end) = clamped_selection_span(total_frames, start_frame, end_frame);
    if clamped_end <= clamped_start {
        return;
    }
    let selection_frames = clamped_end - clamped_start;
    let fade_frames = fade_frames.min(selection_frames / 2);
    if fade_frames == 0 {
        return;
    }
    apply_edge_fade_ramp(samples, channels, clamped_start, fade_frames, true);
    apply_edge_fade_ramp(
        samples,
        channels,
        clamped_end.saturating_sub(fade_frames),
        fade_frames,
        false,
    );
}

fn apply_edge_fade_ramp(
    samples: &mut [f32],
    channels: usize,
    start_frame: usize,
    fade_frames: usize,
    fade_in: bool,
) {
    let denom = (fade_frames.saturating_sub(1)).max(1) as f32;
    let direction = if fade_in {
        FadeDirection::RightToLeft
    } else {
        FadeDirection::LeftToRight
    };
    for i in 0..fade_frames {
        let t = if fade_frames == 1 && !fade_in {
            1.0
        } else {
            i as f32 / denom
        };
        let factor = fade_factor(fade_frames, t, direction, 0.5);
        let frame = start_frame + i;
        for ch in 0..channels {
            let idx = frame * channels + ch;
            if let Some(sample) = samples.get_mut(idx) {
                *sample *= factor;
            }
        }
    }
}

pub(super) fn apply_fade_ramp(
    samples: &mut [f32],
    channels: usize,
    clamped_start: usize,
    clamped_end: usize,
    direction: FadeDirection,
    curve: f32,
) {
    let frame_count = clamped_end - clamped_start;
    let denom = (frame_count.saturating_sub(1)).max(1) as f32;
    for i in 0..frame_count {
        let progress = i as f32 / denom;
        let factor = fade_factor(frame_count, progress, direction, curve);
        let frame = clamped_start + i;
        for ch in 0..channels {
            let idx = frame * channels + ch;
            if let Some(sample) = samples.get_mut(idx) {
                *sample *= factor;
            }
        }
    }
}

pub(crate) fn fade_factor(
    frame_count: usize,
    progress: f32,
    direction: FadeDirection,
    curve: f32,
) -> f32 {
    if frame_count == 1 {
        return 0.0;
    }
    let smoothed = apply_s_curve(progress.clamp(0.0, 1.0), curve);
    let factor = match direction {
        FadeDirection::LeftToRight => 1.0 - smoothed,
        FadeDirection::RightToLeft => smoothed,
    };
    factor.clamp(0.0, 1.0)
}

fn apply_s_curve(t: f32, curve: f32) -> f32 {
    if curve <= 0.0 {
        return t;
    }
    let t2 = t * t;
    let t3 = t2 * t;
    let smootherstep = t3 * (t * (t * 6.0 - 15.0) + 10.0);
    t * (1.0 - curve) + smootherstep * curve
}
