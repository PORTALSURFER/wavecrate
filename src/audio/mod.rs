//! Compatibility exports for Wavecrate's reusable audio foundation.
//!
//! Generic audio infrastructure lives in the `reson` crate. This module keeps
//! existing Wavecrate imports stable and owns conversion from Wavecrate-specific
//! selection state into neutral realtime audio fade ranges.

use std::time::Duration;

pub use reson::{
    AudioDeviceSummary, AudioHostSummary, AudioInputConfig, AudioInputError, AudioOutputConfig,
    AudioOutputError, AudioPlayer, AudioRecorder, EditFadeRange, FadeParams, InputMonitor,
    PlaybackMetronomeConfig, PlaybackRequestId, PlaybackRuntime, PlaybackRuntimeCancellation,
    PlaybackRuntimeConfig, PlaybackRuntimeEvent, PlaybackRuntimeHandle, PlaybackRuntimeMode,
    PlaybackRuntimeProgress, PlaybackRuntimeRequest, PlaybackRuntimeSource,
    PlaybackRuntimeSpanUpdate, PlaybackRuntimeStarted, PlaybackRuntimeSubmitError,
    RecordingOutcome, ResolvedInput, ResolvedInputConfig, ResolvedOutput, SamplesBuffer, Source,
    Wsola, available_devices, available_hosts, available_input_channel_count,
    available_input_devices, available_input_hosts, decoder, input, open_output_stream, output,
    recording, resolve_input_stream_config, supported_input_sample_rates, supported_sample_rates,
    wav_sanitize,
};

use crate::selection::SelectionRange;

/// Default anti-click fade used at hard playback/export/extraction boundaries.
pub const DEFAULT_SHORT_EDGE_FADE: Duration = Duration::from_millis(2);

/// Convert a Wavecrate waveform selection into a reusable `reson` edit-fade range.
pub fn edit_fade_range_from_selection(range: Option<SelectionRange>) -> Option<EditFadeRange> {
    range.map(|range| {
        EditFadeRange::new(
            range.start(),
            range.end(),
            range.gain(),
            range.fade_in().map(|fade| {
                FadeParams::with_outer_gain(fade.length, fade.curve, fade.mute, fade.outer_gain)
            }),
            range.fade_out().map(|fade| {
                FadeParams::with_outer_gain(fade.length, fade.curve, fade.mute, fade.outer_gain)
            }),
        )
    })
}

/// Apply short fade-in/out ramps across an entire interleaved clip.
pub fn apply_short_edge_fades_to_clip(
    samples: &mut [f32],
    channels: usize,
    sample_rate: u32,
    fade_duration: Duration,
) -> bool {
    let channels = channels.max(1);
    let total_frames = samples.len() / channels;
    let fade_frames = short_edge_fade_frame_count(sample_rate, total_frames, fade_duration);
    if fade_frames == 0 {
        return false;
    }
    for frame in 0..total_frames {
        let gain = short_edge_fade_gain(frame, total_frames, fade_frames);
        if (gain - 1.0).abs() <= f32::EPSILON {
            continue;
        }
        let offset = frame.saturating_mul(channels);
        for sample in samples[offset..offset + channels].iter_mut() {
            *sample *= gain;
        }
    }
    true
}

/// Resolve the fade length in frames for a short anti-click clip edge fade.
pub fn short_edge_fade_frame_count(
    sample_rate: u32,
    total_frames: usize,
    fade_duration: Duration,
) -> usize {
    if total_frames < 2 || fade_duration.is_zero() {
        return 0;
    }
    let requested = (fade_duration.as_secs_f64() * f64::from(sample_rate.max(1))).round() as usize;
    requested.min(total_frames / 2)
}

/// Gain for a frame inside a short clip edge fade.
pub fn short_edge_fade_gain(frame: usize, total_frames: usize, fade_frames: usize) -> f32 {
    if total_frames == 0 || fade_frames == 0 {
        return 1.0;
    }
    let fade_frames = fade_frames.min(total_frames / 2).max(1);
    let fade_in = if frame < fade_frames {
        ramp_up_gain(frame, fade_frames)
    } else {
        1.0
    };
    let frames_from_end = total_frames.saturating_sub(frame + 1);
    let fade_out = if frames_from_end < fade_frames {
        ramp_up_gain(frames_from_end, fade_frames)
    } else {
        1.0
    };
    fade_in.min(fade_out)
}

fn ramp_up_gain(offset: usize, fade_frames: usize) -> f32 {
    if fade_frames <= 1 {
        return 0.0;
    }
    (offset as f32 / (fade_frames - 1) as f32).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_edge_fades_ramp_clip_boundaries() {
        let mut samples = vec![1.0_f32; 8];

        let applied =
            apply_short_edge_fades_to_clip(&mut samples, 1, 1_000, Duration::from_millis(2));

        assert!(applied);
        assert_eq!(samples, vec![0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0]);
    }

    #[test]
    fn short_edge_fades_preserve_channel_pairs() {
        let mut samples = vec![1.0_f32; 12];

        apply_short_edge_fades_to_clip(&mut samples, 2, 1_000, Duration::from_millis(2));

        assert_eq!(samples[0], samples[1]);
        assert_eq!(samples[2], samples[3]);
        assert_eq!(samples[8], samples[9]);
        assert_eq!(samples[10], samples[11]);
        assert_eq!(samples[0], 0.0);
        assert_eq!(samples[10], 0.0);
    }
}
