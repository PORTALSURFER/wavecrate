use super::clamped_selection_span;
use super::fades::{apply_fade_ramp, apply_scaled_fade_ramp};
use crate::app::controller::library::selection_edits::FadeDirection;
use crate::selection::FadeParams;

const MIN_MUTE_FADE_SECS: f32 = 0.002;

/// Inputs for applying gain and fade envelopes within a selected frame span.
pub(crate) struct SelectionFadeRequest<'a> {
    /// Audio samples to update in place.
    pub samples: &'a mut [f32],
    /// Channel count for interleaved sample layout.
    pub channels: usize,
    /// Sample rate used to translate minimum mute-fade durations into frames.
    pub sample_rate: u32,
    /// Inclusive selection start frame.
    pub start_frame: usize,
    /// Exclusive selection end frame.
    pub end_frame: usize,
    /// Gain applied uniformly across the selected span before edge fades.
    pub selection_gain: f32,
    /// Optional fade-in parameters for the left edge.
    pub fade_in: Option<FadeParams>,
    /// Optional fade-out parameters for the right edge.
    pub fade_out: Option<FadeParams>,
}

pub(crate) fn apply_selection_fades(request: SelectionFadeRequest<'_>) {
    let SelectionFadeRequest {
        samples,
        channels,
        sample_rate,
        start_frame,
        end_frame,
        selection_gain,
        fade_in,
        fade_out,
    } = request;
    let channels = channels.max(1);
    let total_frames = samples.len() / channels;
    let (clamped_start, clamped_end) = clamped_selection_span(total_frames, start_frame, end_frame);
    if clamped_end <= clamped_start {
        return;
    }
    let selection_frames = clamped_end - clamped_start;
    let min_fade_frames = ((sample_rate.max(1) as f32) * MIN_MUTE_FADE_SECS)
        .round()
        .max(1.0) as usize;

    apply_selection_gain(
        samples,
        channels,
        clamped_start,
        clamped_end,
        selection_gain,
    );
    apply_fade_in(
        samples,
        channels,
        clamped_start,
        clamped_end,
        selection_frames,
        min_fade_frames,
        fade_in,
    );
    apply_fade_out(
        samples,
        channels,
        FadeOutSelection {
            clamped_start,
            clamped_end,
            total_frames,
            selection_frames,
            min_fade_frames,
            fade_out,
        },
    );
}

fn apply_selection_gain(
    samples: &mut [f32],
    channels: usize,
    clamped_start: usize,
    clamped_end: usize,
    selection_gain: f32,
) {
    if (selection_gain - 1.0).abs() <= f32::EPSILON {
        return;
    }
    for frame in clamped_start..clamped_end {
        let base = frame * channels;
        for ch in 0..channels {
            if let Some(sample) = samples.get_mut(base + ch) {
                *sample *= selection_gain;
            }
        }
    }
}

fn apply_fade_in(
    samples: &mut [f32],
    channels: usize,
    clamped_start: usize,
    clamped_end: usize,
    selection_frames: usize,
    min_fade_frames: usize,
    fade_in: Option<FadeParams>,
) {
    let Some(fade_in) = fade_in else {
        return;
    };
    let fade_frames = fade_frame_count(selection_frames, min_fade_frames, fade_in);
    if fade_frames == 0 {
        return;
    }
    let mute_frames = ((selection_frames as f32) * fade_in.mute).round().max(0.0) as usize;
    if mute_frames > 0 {
        let mute_start = clamped_start.saturating_sub(mute_frames);
        if mute_start < clamped_start {
            apply_scaled_fade_ramp(
                samples,
                channels,
                mute_start,
                clamped_start,
                FadeDirection::LeftToRight,
                fade_in.curve,
                fade_in.outer_gain,
            );
        }
    }
    let fade_end = clamped_start.saturating_add(fade_frames).min(clamped_end);
    if fade_end > clamped_start {
        apply_fade_ramp(
            samples,
            channels,
            clamped_start,
            fade_end,
            FadeDirection::RightToLeft,
            fade_in.curve,
        );
    }
}

struct FadeOutSelection {
    clamped_start: usize,
    clamped_end: usize,
    total_frames: usize,
    selection_frames: usize,
    min_fade_frames: usize,
    fade_out: Option<FadeParams>,
}

fn apply_fade_out(samples: &mut [f32], channels: usize, selection: FadeOutSelection) {
    let Some(fade_out) = selection.fade_out else {
        return;
    };
    let fade_frames = fade_frame_count(
        selection.selection_frames,
        selection.min_fade_frames,
        fade_out,
    );
    if fade_frames == 0 {
        return;
    }
    let mute_frames = ((selection.selection_frames as f32) * fade_out.mute)
        .round()
        .max(0.0) as usize;
    if mute_frames > 0 {
        let mute_end = selection
            .clamped_end
            .saturating_add(mute_frames)
            .min(selection.total_frames);
        if selection.clamped_end < mute_end {
            apply_scaled_fade_ramp(
                samples,
                channels,
                selection.clamped_end,
                mute_end,
                FadeDirection::RightToLeft,
                fade_out.curve,
                fade_out.outer_gain,
            );
        }
    }
    let fade_start = selection
        .clamped_end
        .saturating_sub(fade_frames)
        .max(selection.clamped_start);
    if fade_start < selection.clamped_end {
        apply_fade_ramp(
            samples,
            channels,
            fade_start,
            selection.clamped_end,
            FadeDirection::LeftToRight,
            fade_out.curve,
        );
    }
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
