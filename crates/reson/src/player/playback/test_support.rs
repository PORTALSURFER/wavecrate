use std::sync::Arc;
use std::time::Duration;

use crate::SamplesBuffer;
use crate::timebase::{duration_for_frames, seconds_to_frames_floor, seconds_to_frames_round};
use crate::{Source, player::AudioPlayer};

use super::super::super::mixer::{
    decoder_duration, decoder_from_bytes, map_seek_error, wav_header_duration,
};
use super::span::QuantizedSpan;

impl AudioPlayer {
    /// Calculate a frame-aligned span duration that never extends beyond the
    /// original floating-point span request.
    pub(crate) fn aligned_span_duration(span_seconds: f32, sample_rate: u32) -> Duration {
        if sample_rate == 0 {
            return Duration::from_secs_f32(span_seconds.max(0.0));
        }
        let frames = seconds_to_frames_floor(span_seconds.max(0.0), sample_rate).max(1);
        duration_for_frames(frames, sample_rate)
    }

    pub(crate) fn loop_cycle_sample_count_for_tests(
        bytes: Arc<[u8]>,
        start_seconds: f32,
        end_seconds: f32,
        offset_seconds: Option<f32>,
    ) -> Result<(usize, usize, u32, u16), String> {
        let duration = decoder_duration(&bytes)
            .or_else(|| wav_header_duration(&bytes))
            .ok_or_else(|| "Load a .wav file first".to_string())?;
        if duration <= 0.0 {
            return Err("Load a .wav file first".into());
        }

        let mut source = decoder_from_bytes(bytes)?;
        let sample_rate = source.sample_rate().max(1);
        let channels = source.channels().max(1);
        let span = QuantizedSpan::new(start_seconds, end_seconds, duration, sample_rate, channels);

        source
            .try_seek(duration_for_frames(span.start_frame, sample_rate))
            .map_err(map_seek_error)?;

        let samples = read_span_samples(source, span.samples);
        let offset_samples = offset_seconds
            .map(|seconds| seconds_to_frames_round(seconds, sample_rate) % span.frames)
            .map(|frames| frames.saturating_mul(channels as u64) as usize);
        let emitted = looped_sample_count(samples, channels, sample_rate, offset_samples);
        Ok((emitted, span.frames as usize, sample_rate, channels))
    }
}

fn read_span_samples(source: impl Source<Item = f32>, span_samples: u64) -> Vec<f32> {
    let mut limited = source.take_samples(span_samples as usize);
    let mut samples = Vec::with_capacity(span_samples as usize);
    for _ in 0..span_samples {
        if let Some(sample) = limited.next() {
            samples.push(sample);
        } else {
            break;
        }
    }
    samples.resize(span_samples as usize, 0.0);
    samples
}

fn looped_sample_count(
    samples: Vec<f32>,
    channels: u16,
    sample_rate: u32,
    offset_samples: Option<usize>,
) -> usize {
    let span_samples = samples.len();
    let buffer = SamplesBuffer::new(channels, sample_rate, samples);
    let mut looped: Box<dyn Source<Item = f32>> = if let Some(offset_samples) = offset_samples {
        Box::new(buffer.repeat_infinite().skip_samples(offset_samples))
    } else {
        Box::new(buffer.repeat_infinite())
    };

    (0..span_samples)
        .take_while(|_| looped.next().is_some())
        .count()
}
