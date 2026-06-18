use std::{path::PathBuf, time::Duration};

use crate::Source;

use super::super::super::PlaybackSpanPlan;
use super::pcm_wav_cursor::{PcmWavFileCursor, validate_pcm_wav_file};
use super::{RepeatReadRequest, SourceFormat, SpanReadRequest};

pub(in crate::player::playback) struct PcmWavFileSpanSource {
    cursor: PcmWavFileCursor,
    format: SourceFormat,
    end_sample: u64,
    total_duration: Duration,
    last_error: Option<String>,
}

impl PcmWavFileSpanSource {
    pub(in crate::player::playback) fn try_new(
        path: PathBuf,
        plan: &PlaybackSpanPlan,
    ) -> Result<Self, String> {
        let format = SourceFormat::from_plan(plan);
        let total_frames = validate_pcm_wav_file(&path, format)?;
        let total_samples = total_frames.saturating_mul(format.channels() as u64);
        let request = SpanReadRequest::from_plan(plan);
        let start_sample = request
            .start_frame
            .saturating_mul(format.channels() as u64)
            .min(total_samples);
        let end_sample = start_sample
            .saturating_add(request.span_samples)
            .min(total_samples);
        Ok(Self {
            cursor: PcmWavFileCursor::new(path, format, start_sample),
            format,
            end_sample,
            total_duration: request.total_duration,
            last_error: None,
        })
    }
}

impl Iterator for PcmWavFileSpanSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor.position() >= self.end_sample {
            return None;
        }
        match self.cursor.read_next("span_open_seek") {
            Ok(sample) => Some(sample),
            Err(error) => {
                self.last_error = Some(error);
                self.cursor.seek_logically_to(self.end_sample);
                None
            }
        }
    }
}

impl Source for PcmWavFileSpanSource {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.end_sample.saturating_sub(self.cursor.position()) as usize)
    }

    fn channels(&self) -> u16 {
        self.format.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.format.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(self.total_duration)
    }

    fn last_error(&self) -> Option<String> {
        self.last_error.clone()
    }
}

pub(in crate::player::playback) struct PcmWavFileRepeatingSpanSource {
    cursor: PcmWavFileCursor,
    format: SourceFormat,
    cycle: PcmWavRepeatCycle,
    last_error: Option<String>,
}

impl PcmWavFileRepeatingSpanSource {
    pub(in crate::player::playback) fn try_new(
        path: PathBuf,
        plan: &PlaybackSpanPlan,
    ) -> Result<Self, String> {
        let format = SourceFormat::from_plan(plan);
        let total_frames = validate_pcm_wav_file(&path, format)?;
        let request = RepeatReadRequest::from_plan(plan);
        let cycle = PcmWavRepeatCycle::new(request, format, total_frames);
        Ok(Self {
            cursor: PcmWavFileCursor::new(path, format, cycle.initial_sample()),
            format,
            cycle,
            last_error: None,
        })
    }

    fn seek_to_cycle_position(&mut self, cycle_sample_offset: u64) -> Result<(), ()> {
        self.cursor
            .open_at(
                self.cycle.sample_for_cycle_offset(cycle_sample_offset),
                "repeat_open_seek",
            )
            .map_err(|error| {
                self.last_error = Some(error);
                self.cursor.close();
            })?;
        self.cycle.seek_to(cycle_sample_offset);
        Ok(())
    }

    fn reader_ready(&mut self) -> Option<()> {
        if self.cursor.is_closed()
            && self
                .seek_to_cycle_position(self.cycle.initial_offset())
                .is_err()
        {
            return None;
        }
        Some(())
    }

    fn restart_cycle(&mut self) -> Option<()> {
        self.seek_to_cycle_position(0).ok()
    }
}

impl Iterator for PcmWavFileRepeatingSpanSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cycle.is_complete() {
            self.restart_cycle()?;
        }
        self.reader_ready()?;
        match self.cursor.read_next("repeat_open_seek") {
            Ok(sample) => {
                self.cycle.advance();
                Some(sample)
            }
            Err(error) => {
                self.last_error = Some(error);
                self.restart_cycle()?;
                let sample = self.cursor.read_next("repeat_open_seek").ok()?;
                self.cycle.advance();
                Some(sample)
            }
        }
    }
}

impl Source for PcmWavFileRepeatingSpanSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.format.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.format.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }

    fn last_error(&self) -> Option<String> {
        self.last_error.clone()
    }
}

struct PcmWavRepeatCycle {
    start_sample: u64,
    span_samples: u64,
    samples_into_cycle: u64,
    initial_offset_samples: u64,
}

impl PcmWavRepeatCycle {
    fn new(request: RepeatReadRequest, format: SourceFormat, total_frames: u64) -> Self {
        let total_samples = total_frames.saturating_mul(format.channels() as u64);
        let start_sample = request
            .start_frame
            .saturating_mul(format.channels() as u64)
            .min(total_samples);
        let span_samples = request
            .span_samples
            .max(format.channels() as u64)
            .min(total_samples.saturating_sub(start_sample));
        let initial_offset_samples = request
            .offset_frames
            .saturating_mul(format.channels() as u64)
            % span_samples.max(1);
        Self {
            start_sample,
            span_samples,
            samples_into_cycle: initial_offset_samples,
            initial_offset_samples,
        }
    }

    fn initial_sample(&self) -> u64 {
        self.sample_for_cycle_offset(self.initial_offset_samples)
    }

    fn initial_offset(&self) -> u64 {
        self.initial_offset_samples
    }

    fn is_complete(&self) -> bool {
        self.samples_into_cycle >= self.span_samples
    }

    fn sample_for_cycle_offset(&self, cycle_sample_offset: u64) -> u64 {
        self.start_sample
            .saturating_add(cycle_sample_offset.min(self.span_samples))
    }

    fn seek_to(&mut self, cycle_sample_offset: u64) {
        self.samples_into_cycle = cycle_sample_offset.min(self.span_samples);
    }

    fn advance(&mut self) {
        self.samples_into_cycle = self.samples_into_cycle.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::{
        PlaybackChannelLayout, PlaybackSeekBehavior, PlaybackSourceIdentity, PlaybackSourceKind,
        PlaybackSpanRequest,
    };
    use std::path::Path;

    #[test]
    fn pcm_wav_span_reads_requested_samples() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("samples.wav");
        write_i16_wav(&path, 2, &[0, 1_000, 2_000, 3_000, 4_000, -4_000]);
        let plan = span_plan(2, 48_000, 1, 3, 0, 0.0000625);

        let source = PcmWavFileSpanSource::try_new(path, &plan).expect("pcm wav source");
        let samples = source.collect::<Vec<_>>();

        assert_eq!(samples.len(), 4);
        assert!((samples[0] - 2_000.0 / 32_767.0).abs() < 0.000_01);
        assert!((samples[1] - 3_000.0 / 32_767.0).abs() < 0.000_01);
        assert!((samples[2] - 4_000.0 / 32_767.0).abs() < 0.000_01);
        assert!((samples[3] + 4_000.0 / 32_767.0).abs() < 0.000_01);
    }

    #[test]
    fn pcm_wav_repeating_source_starts_at_offset_and_wraps() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("loop.wav");
        write_i16_wav(&path, 1, &[0, 1_000, 2_000, 3_000]);
        let plan = span_plan(1, 48_000, 0, 4, 2, 0.0000834);

        let source = PcmWavFileRepeatingSpanSource::try_new(path, &plan).expect("pcm wav source");
        let samples = source.take(5).collect::<Vec<_>>();

        assert_eq!(samples.len(), 5);
        assert!((samples[0] - 2_000.0 / 32_767.0).abs() < 0.000_01);
        assert!((samples[1] - 3_000.0 / 32_767.0).abs() < 0.000_01);
        assert!((samples[2] - 0.0).abs() < 0.000_01);
        assert!((samples[3] - 1_000.0 / 32_767.0).abs() < 0.000_01);
        assert!((samples[4] - 2_000.0 / 32_767.0).abs() < 0.000_01);
    }

    #[test]
    fn pcm_wav_source_rejects_layout_mismatch() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("mismatch.wav");
        write_i16_wav(&path, 2, &[0, 0, 1_000, -1_000]);
        let plan = span_plan(1, 48_000, 0, 2, 0, 0.0000417);

        let error = match PcmWavFileSpanSource::try_new(path, &plan) {
            Ok(_) => panic!("layout mismatch should fail"),
            Err(error) => error,
        };

        assert!(error.contains("channel count changed"));
    }

    fn write_i16_wav(path: &Path, channels: u16, samples: &[i16]) {
        let spec = hound::WavSpec {
            channels,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).expect("writer");
        for sample in samples {
            writer.write_sample(*sample).expect("write sample");
        }
        writer.finalize().expect("finalize wav");
    }

    fn span_plan(
        channels: u16,
        sample_rate: u32,
        start_frame: u64,
        end_frame: u64,
        offset_frame: u64,
        duration_seconds: f32,
    ) -> PlaybackSpanPlan {
        let start_seconds = start_frame as f32 / sample_rate as f32;
        let end_seconds = end_frame as f32 / sample_rate as f32;
        PlaybackSpanPlan::new(
            PlaybackSourceIdentity::new(PlaybackSourceKind::File, None),
            PlaybackChannelLayout::new(channels, sample_rate).expect("valid layout"),
            PlaybackSpanRequest::new(
                start_seconds,
                end_seconds,
                duration_seconds,
                offset_frame > 0,
                PlaybackSeekBehavior::FrameOffset(offset_frame),
            ),
        )
        .expect("span plan")
    }
}
