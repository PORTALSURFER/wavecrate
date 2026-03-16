use crate::app::controller::library::wav_io::read_samples_for_normalization;
use crate::app::state::{LoopCrossfadeSettings, LoopCrossfadeUnit};
use hound::SampleFormat;
use std::path::Path;

/// Prepared loop-crossfade output before it is written to disk.
pub(super) struct RenderedLoopCrossfade {
    /// Crossfaded sample payload in interleaved channel order.
    pub samples: Vec<f32>,
    /// Output WAV format used for the rewritten file.
    pub spec: hound::WavSpec,
    /// Stable suffix used when generating the destination filename.
    pub suffix: String,
}

/// Decode, crossfade, and prepare the rewritten sample payload.
pub(super) fn render_loop_crossfade(
    absolute_path: &Path,
    settings: &LoopCrossfadeSettings,
) -> Result<RenderedLoopCrossfade, String> {
    let (mut samples, spec) = read_samples_for_normalization(absolute_path)?;
    let (channels, total_frames) = loop_crossfade_layout(&samples, spec.channels)?;
    let fade_frames = loop_crossfade_frames(settings, spec.sample_rate.max(1), total_frames)?;
    apply_loop_crossfade(&mut samples, channels, total_frames, fade_frames)?;
    Ok(RenderedLoopCrossfade {
        samples,
        spec: loop_crossfade_spec(&spec),
        suffix: loop_crossfade_suffix(settings),
    })
}

/// Convert loop-crossfade settings into a clamped frame count for one sample.
fn loop_crossfade_frames(
    settings: &LoopCrossfadeSettings,
    sample_rate: u32,
    total_frames: usize,
) -> Result<usize, String> {
    let frames = match settings.unit {
        LoopCrossfadeUnit::Milliseconds => {
            let ms = settings.depth_ms.max(1) as f32;
            ((sample_rate as f32 * ms / 1000.0).round() as usize).max(1)
        }
        LoopCrossfadeUnit::Samples => settings.depth_samples.max(1) as usize,
    };
    let max_frames = total_frames / 2;
    if max_frames == 0 {
        return Err("Sample is too short for a loop crossfade".into());
    }
    Ok(frames.min(max_frames))
}

/// Validate the channel/frame layout for a decoded sample.
fn loop_crossfade_layout(samples: &[f32], channels: u16) -> Result<(usize, usize), String> {
    if samples.is_empty() {
        return Err("No audio data to crossfade".into());
    }
    let channels = channels.max(1) as usize;
    let total_frames = samples.len() / channels;
    if total_frames < 2 {
        return Err("Sample is too short to crossfade".into());
    }
    Ok((channels, total_frames))
}

/// Rotate the sample so the best loop cut becomes the new seam and crossfade the tail.
pub(super) fn apply_loop_crossfade(
    samples: &mut [f32],
    channels: usize,
    total_frames: usize,
    fade_frames: usize,
) -> Result<(), String> {
    let fade_frames = fade_frames.min(total_frames / 2);
    if fade_frames == 0 {
        return Err("Crossfade depth is too short for this sample".into());
    }
    let channels = channels.max(1);
    let cut_frame = find_crossfade_cut_frame(samples, channels, total_frames, fade_frames);
    let mut output = vec![0.0; samples.len()];
    for frame in 0..total_frames {
        let src_frame = (cut_frame + frame) % total_frames;
        for channel in 0..channels {
            let out_idx = frame * channels + channel;
            let src_idx = src_frame * channels + channel;
            output[out_idx] = samples[src_idx];
        }
    }
    let denom = (fade_frames.saturating_sub(1)).max(1) as f32;
    let body_frames = total_frames.saturating_sub(fade_frames);
    let mut blended = vec![0.0; samples.len()];
    for frame in 0..body_frames {
        for channel in 0..channels {
            let out_idx = frame * channels + channel;
            let src_idx = (frame + fade_frames) * channels + channel;
            blended[out_idx] = output[src_idx];
        }
    }
    for frame in 0..fade_frames {
        let progress = if fade_frames == 1 {
            0.5
        } else {
            frame as f32 / denom
        };
        let (from_gain, to_gain) = equal_power_gains(progress);
        for channel in 0..channels {
            let head_idx = frame * channels + channel;
            let tail_idx = (total_frames - fade_frames + frame) * channels + channel;
            let out_idx = (body_frames + frame) * channels + channel;
            blended[out_idx] = output[tail_idx] * from_gain + output[head_idx] * to_gain;
        }
    }
    samples.copy_from_slice(&blended);
    Ok(())
}

/// Return equal-power gains for one normalized fade step.
fn equal_power_gains(progress: f32) -> (f32, f32) {
    let t = progress.clamp(0.0, 1.0);
    let angle = t * std::f32::consts::FRAC_PI_2;
    (angle.cos(), angle.sin())
}

/// Choose the lowest-delta seam near the natural end of the file.
pub(super) fn find_crossfade_cut_frame(
    samples: &[f32],
    channels: usize,
    total_frames: usize,
    fade_frames: usize,
) -> usize {
    let nominal = total_frames.saturating_sub(fade_frames);
    let search_window = fade_frames.min(1024).min(nominal);
    let min_cut = nominal.saturating_sub(search_window);
    let max_cut = nominal.max(1);
    let mut best_frame = nominal.max(1);
    let mut best_score = f32::INFINITY;
    for frame in min_cut.max(1)..=max_cut {
        let prev = frame - 1;
        let mut score = 0.0;
        for channel in 0..channels {
            let a = samples[prev * channels + channel];
            let b = samples[frame * channels + channel];
            score += (b - a).abs();
        }
        if score < best_score {
            best_score = score;
            best_frame = frame;
        }
    }
    best_frame
}

/// Generate the destination filename suffix implied by the crossfade settings.
fn loop_crossfade_suffix(settings: &LoopCrossfadeSettings) -> String {
    match settings.unit {
        LoopCrossfadeUnit::Milliseconds => format!("fade{}ms", settings.depth_ms.max(1)),
        LoopCrossfadeUnit::Samples => format!("fade{}samp", settings.depth_samples.max(1)),
    }
}

/// Normalize output WAV settings to the controller's floating-point write contract.
fn loop_crossfade_spec(spec: &hound::WavSpec) -> hound::WavSpec {
    hound::WavSpec {
        channels: spec.channels.max(1),
        sample_rate: spec.sample_rate.max(1),
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    }
}
