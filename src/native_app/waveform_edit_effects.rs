use wavecrate::selection::{FadeParams, SelectionRange, fade_curve_value};

#[derive(Clone, Copy)]
enum FadeRampDirection {
    Down,
    Up,
}

const MIN_MUTE_FADE_SECS: f32 = 0.002;

pub(in crate::native_app) fn apply_edit_selection_effects(
    samples: &mut [f32],
    channels: usize,
    sample_rate: u32,
    selection: SelectionRange,
    start_frame: usize,
    end_frame: usize,
) {
    let channels = channels.max(1);
    let total_frames = samples.len() / channels;
    let (start_frame, end_frame) = clamped_frame_span(total_frames, start_frame, end_frame);
    if end_frame <= start_frame {
        return;
    }
    let selection_frames = end_frame - start_frame;
    let min_fade_frames = ((sample_rate.max(1) as f32) * MIN_MUTE_FADE_SECS)
        .round()
        .max(1.0) as usize;

    apply_selection_gain(samples, channels, start_frame, end_frame, selection.gain());
    apply_fade_in_effect(
        samples,
        channels,
        start_frame,
        end_frame,
        selection_frames,
        min_fade_frames,
        selection.fade_in(),
    );
    apply_fade_out_effect(
        samples,
        channels,
        FadeOutEffect {
            start_frame,
            end_frame,
            total_frames,
            selection_frames,
            min_fade_frames,
            fade: selection.fade_out(),
        },
    );
}

fn apply_selection_gain(
    samples: &mut [f32],
    channels: usize,
    start_frame: usize,
    end_frame: usize,
    gain: f32,
) {
    if (gain - 1.0).abs() <= f32::EPSILON {
        return;
    }
    for frame in start_frame..end_frame {
        let base = frame * channels;
        for channel in 0..channels {
            if let Some(sample) = samples.get_mut(base + channel) {
                *sample *= gain;
            }
        }
    }
}

fn apply_fade_in_effect(
    samples: &mut [f32],
    channels: usize,
    start_frame: usize,
    end_frame: usize,
    selection_frames: usize,
    min_fade_frames: usize,
    fade: Option<FadeParams>,
) {
    let Some(fade) = fade else {
        return;
    };
    let fade_frames = fade_frame_count(selection_frames, min_fade_frames, fade);
    let mute_frames = ((selection_frames as f32) * fade.mute).round().max(0.0) as usize;
    if mute_frames > 0 {
        let mute_start = start_frame.saturating_sub(mute_frames);
        if mute_start < start_frame {
            apply_scaled_fade_ramp(
                samples,
                channels,
                mute_start,
                start_frame,
                FadeRampDirection::Down,
                fade.curve,
                fade.outer_gain,
            );
        }
    }
    let fade_end = start_frame.saturating_add(fade_frames).min(end_frame);
    if fade_end > start_frame {
        apply_fade_ramp(
            samples,
            channels,
            start_frame,
            fade_end,
            FadeRampDirection::Up,
            fade.curve,
        );
    }
}

struct FadeOutEffect {
    start_frame: usize,
    end_frame: usize,
    total_frames: usize,
    selection_frames: usize,
    min_fade_frames: usize,
    fade: Option<FadeParams>,
}

fn apply_fade_out_effect(samples: &mut [f32], channels: usize, effect: FadeOutEffect) {
    let Some(fade) = effect.fade else {
        return;
    };
    let fade_frames = fade_frame_count(effect.selection_frames, effect.min_fade_frames, fade);
    let mute_frames = ((effect.selection_frames as f32) * fade.mute)
        .round()
        .max(0.0) as usize;
    if mute_frames > 0 {
        let mute_end = effect
            .end_frame
            .saturating_add(mute_frames)
            .min(effect.total_frames);
        if effect.end_frame < mute_end {
            apply_scaled_fade_ramp(
                samples,
                channels,
                effect.end_frame,
                mute_end,
                FadeRampDirection::Up,
                fade.curve,
                fade.outer_gain,
            );
        }
    }
    let fade_start = effect
        .end_frame
        .saturating_sub(fade_frames)
        .max(effect.start_frame);
    if fade_start < effect.end_frame {
        apply_fade_ramp(
            samples,
            channels,
            fade_start,
            effect.end_frame,
            FadeRampDirection::Down,
            fade.curve,
        );
    }
}

fn apply_fade_ramp(
    samples: &mut [f32],
    channels: usize,
    start_frame: usize,
    end_frame: usize,
    direction: FadeRampDirection,
    curve: f32,
) {
    apply_scaled_fade_ramp(
        samples,
        channels,
        start_frame,
        end_frame,
        direction,
        curve,
        1.0,
    );
}

fn apply_scaled_fade_ramp(
    samples: &mut [f32],
    channels: usize,
    start_frame: usize,
    end_frame: usize,
    direction: FadeRampDirection,
    curve: f32,
    scale: f32,
) {
    let frame_count = end_frame.saturating_sub(start_frame);
    if frame_count == 0 {
        return;
    }
    let denom = frame_count.saturating_sub(1).max(1) as f32;
    let scale = scale.clamp(0.0, 1.0);
    for i in 0..frame_count {
        let progress = i as f32 / denom;
        let factor = fade_factor(frame_count, progress, direction, curve) * scale;
        let frame = start_frame + i;
        let base = frame * channels;
        for channel in 0..channels {
            if let Some(sample) = samples.get_mut(base + channel) {
                *sample *= factor;
            }
        }
    }
}

fn fade_factor(frame_count: usize, progress: f32, direction: FadeRampDirection, curve: f32) -> f32 {
    if frame_count == 1 {
        return 0.0;
    }
    let eased = fade_curve_value(progress, curve);
    match direction {
        FadeRampDirection::Down => 1.0 - eased,
        FadeRampDirection::Up => eased,
    }
    .clamp(0.0, 1.0)
}

fn fade_frame_count(selection_frames: usize, min_fade_frames: usize, fade: FadeParams) -> usize {
    let base_fade_frames = ((selection_frames as f32) * fade.length)
        .round()
        .clamp(0.0, selection_frames as f32) as usize;
    if base_fade_frames > 0 {
        base_fade_frames
    } else if fade.mute > 0.0 {
        min_fade_frames.min(selection_frames)
    } else {
        0
    }
}

fn clamped_frame_span(total_frames: usize, start_frame: usize, end_frame: usize) -> (usize, usize) {
    (start_frame.min(total_frames), end_frame.min(total_frames))
}
