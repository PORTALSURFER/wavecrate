use super::*;
use crate::app::controller::playback::audio_samples::decode_samples_from_bytes;

pub(super) fn normalized_audition_gain(controller: &AppController, start: f32, end: f32) -> f32 {
    if !controller.ui.waveform.normalized_audition_enabled {
        return 1.0;
    }
    let Some(peak) = normalized_audition_peak(controller, start, end) else {
        return 1.0;
    };
    if peak <= f32::EPSILON {
        return 1.0;
    }
    1.0 / peak
}

/// Resolve the peak amplitude used for normalized audition over one playback span.
///
/// The retained decoded waveform is the fast path, but it is not guaranteed to
/// be present for every loaded sample. Plain transport playback should still
/// honor normalized audition in that state, so fall back to the loaded audio
/// bytes when the waveform decode cache is unavailable.
fn normalized_audition_peak(controller: &AppController, start: f32, end: f32) -> Option<f32> {
    if let Some(decoded) = controller.sample_view.waveform.decoded.as_ref() {
        return decoded.max_abs_in_span(start, end);
    }
    let loaded = controller.sample_view.wav.loaded_audio.as_ref()?;
    let decoded = decode_samples_from_bytes(&loaded.bytes).ok()?;
    max_abs_from_samples_span(
        &decoded.samples,
        decoded.channels.max(1) as usize,
        start,
        end,
    )
}

/// Compute the largest absolute sample amplitude inside one normalized span.
fn max_abs_from_samples_span(
    samples: &[f32],
    channels: usize,
    start: f32,
    end: f32,
) -> Option<f32> {
    if samples.is_empty() || !start.is_finite() || !end.is_finite() {
        return None;
    }
    let channels = channels.max(1);
    let total_frames = samples.len() / channels;
    if total_frames == 0 {
        return None;
    }
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    let start_frame = (start.clamp(0.0, 1.0) * total_frames as f32).floor() as usize;
    let mut end_frame = (end.clamp(0.0, 1.0) * total_frames as f32).ceil() as usize;
    if end_frame <= start_frame {
        end_frame = (start_frame + 1).min(total_frames);
    }
    let start_idx = start_frame.saturating_mul(channels);
    let end_idx = end_frame.saturating_mul(channels).min(samples.len());
    (start_idx < end_idx).then(|| {
        samples[start_idx..end_idx]
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()))
    })
}

#[cfg(test)]
mod tests {
    use super::max_abs_from_samples_span;

    #[test]
    fn max_abs_from_samples_span_swaps_reversed_bounds() {
        let samples = [0.1, -0.8, 0.3, -0.4];

        let peak = max_abs_from_samples_span(&samples, 1, 0.75, 0.25);

        assert_eq!(peak, Some(0.8));
    }

    #[test]
    fn max_abs_from_samples_span_respects_channel_stride() {
        let samples = [
            0.1, -0.2, //
            0.9, -0.1, //
            0.3, -0.4,
        ];

        let peak = max_abs_from_samples_span(&samples, 2, 0.0, 0.5);

        assert_eq!(peak, Some(0.9));
    }
}
