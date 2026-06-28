use std::io::{Read, Seek, SeekFrom};
use std::ops::Range;

/// Return the normalized playback/export gain for one peak amplitude.
pub fn normalized_gain_from_peak(peak: f32) -> f32 {
    if peak.is_finite() && peak > f32::EPSILON {
        1.0 / peak
    } else {
        1.0
    }
}

/// Return the largest absolute sample amplitude inside one normalized span.
pub fn peak_for_interleaved_span(
    samples: &[f32],
    channels: usize,
    start: f32,
    end: f32,
) -> Option<f32> {
    let bounds = interleaved_span_sample_bounds(samples.len(), channels, start, end)?;
    Some(
        samples[bounds]
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs())),
    )
}

/// Return the largest absolute sample amplitude inside one normalized span read
/// from a little-endian f32 playback cache.
pub fn peak_for_interleaved_f32_reader_span(
    reader: &mut (impl Read + Seek),
    sample_count: usize,
    channels: usize,
    start: f32,
    end: f32,
) -> Option<f32> {
    const F32_SAMPLE_BYTES: usize = std::mem::size_of::<f32>();
    const READ_SAMPLES: usize = 4096;

    let bounds = interleaved_span_sample_bounds(sample_count, channels, start, end)?;
    let offset = bounds.start.checked_mul(F32_SAMPLE_BYTES)?;
    reader.seek(SeekFrom::Start(offset as u64)).ok()?;

    let mut remaining = bounds.end.saturating_sub(bounds.start);
    let mut bytes = vec![0_u8; READ_SAMPLES * F32_SAMPLE_BYTES];
    let mut peak = 0.0_f32;
    while remaining > 0 {
        let samples_to_read = remaining.min(READ_SAMPLES);
        let byte_len = samples_to_read * F32_SAMPLE_BYTES;
        reader.read_exact(&mut bytes[..byte_len]).ok()?;
        for sample in bytes[..byte_len].chunks_exact(F32_SAMPLE_BYTES) {
            peak = peak.max(f32::from_le_bytes(sample.try_into().ok()?).abs());
        }
        remaining -= samples_to_read;
    }
    Some(peak)
}

/// Return interleaved sample bounds for one normalized frame span.
pub fn interleaved_span_sample_bounds(
    sample_count: usize,
    channels: usize,
    start: f32,
    end: f32,
) -> Option<Range<usize>> {
    if sample_count == 0 || !start.is_finite() || !end.is_finite() {
        return None;
    }
    let channels = channels.max(1);
    let total_frames = sample_count / channels;
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
    let end_idx = end_frame.saturating_mul(channels).min(sample_count);
    (start_idx < end_idx).then_some(start_idx..end_idx)
}

/// Return the forced-normalized gain for one interleaved sample span.
pub fn normalized_gain_for_interleaved_span(
    samples: &[f32],
    channels: usize,
    start: f32,
    end: f32,
) -> f32 {
    peak_for_interleaved_span(samples, channels, start, end)
        .map(normalized_gain_from_peak)
        .unwrap_or(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peak_for_interleaved_span_swaps_reversed_bounds() {
        let samples = [0.1, -0.8, 0.3, -0.4];

        let peak = peak_for_interleaved_span(&samples, 1, 0.75, 0.25);

        assert_eq!(peak, Some(0.8));
    }

    #[test]
    fn peak_for_interleaved_span_respects_channel_stride() {
        let samples = [
            0.1, -0.2, //
            0.9, -0.1, //
            0.3, -0.4,
        ];

        let peak = peak_for_interleaved_span(&samples, 2, 0.0, 0.5);

        assert_eq!(peak, Some(0.9));
    }

    #[test]
    fn normalized_gain_for_interleaved_span_targets_peak_one() {
        let samples = [0.1, -0.25, 0.5, -0.2];

        let gain = normalized_gain_for_interleaved_span(&samples, 1, 0.0, 1.0);

        assert!((gain - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn peak_for_interleaved_f32_reader_span_matches_memory_span() {
        let samples = [0.1_f32, -0.2, 0.9, -0.1, 0.3, -0.4];
        let bytes = samples
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect::<Vec<_>>();
        let mut reader = std::io::Cursor::new(bytes);

        let peak = peak_for_interleaved_f32_reader_span(&mut reader, samples.len(), 2, 0.0, 0.5);

        assert_eq!(peak, Some(0.9));
    }
}
