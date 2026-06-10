use std::{path::PathBuf, time::Duration};

use crate::Source;

use super::super::super::PlaybackSpanPlan;
use super::f32_cursor::{F32FileCursor, F32RepeatCycle};
use super::{RepeatReadRequest, SourceFormat, SpanReadRequest};

pub(in crate::player::playback) struct InterleavedF32FileSpanSource {
    cursor: F32FileCursor,
    format: SourceFormat,
    end_sample: u64,
    total_duration: Duration,
    last_error: Option<String>,
}

impl InterleavedF32FileSpanSource {
    pub(in crate::player::playback) fn new(
        path: PathBuf,
        plan: &PlaybackSpanPlan,
        total_samples: u64,
    ) -> Self {
        let format = SourceFormat::from_plan(plan);
        let request = SpanReadRequest::from_plan(plan);
        let start_sample = request
            .start_frame
            .saturating_mul(format.channels() as u64)
            .min(total_samples);
        let end_sample = start_sample
            .saturating_add(request.span_samples)
            .min(total_samples);
        Self {
            cursor: F32FileCursor::new(path, start_sample),
            format,
            end_sample,
            total_duration: request.total_duration,
            last_error: None,
        }
    }
}

impl Iterator for InterleavedF32FileSpanSource {
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

impl Source for InterleavedF32FileSpanSource {
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

pub(in crate::player::playback) struct InterleavedF32FileRepeatingSpanSource {
    cursor: F32FileCursor,
    format: SourceFormat,
    cycle: F32RepeatCycle,
    last_error: Option<String>,
}

impl InterleavedF32FileRepeatingSpanSource {
    pub(in crate::player::playback) fn new(
        path: PathBuf,
        plan: &PlaybackSpanPlan,
        total_samples: u64,
    ) -> Self {
        let format = SourceFormat::from_plan(plan);
        let request = RepeatReadRequest::from_plan(plan);
        let cycle = F32RepeatCycle::new(request, format, total_samples);
        Self {
            cursor: F32FileCursor::new(path, cycle.initial_sample()),
            format,
            cycle,
            last_error: None,
        }
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

impl Iterator for InterleavedF32FileRepeatingSpanSource {
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

impl Source for InterleavedF32FileRepeatingSpanSource {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::{
        PlaybackChannelLayout, PlaybackSeekBehavior, PlaybackSourceIdentity, PlaybackSourceKind,
        PlaybackSpanRequest,
    };
    use std::{fs, path::Path};

    #[test]
    fn interleaved_f32_span_reads_requested_samples() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("samples.pcm");
        write_samples(&path, &[0.0, 0.25, 0.5, 0.75, 1.0, -0.5]);
        let plan = span_plan(2, 48_000, 1, 3, 0, 0.0000625);

        let source = InterleavedF32FileSpanSource::new(path, &plan, 6);
        let samples = source.collect::<Vec<_>>();

        assert_eq!(samples, vec![0.5, 0.75, 1.0, -0.5]);
    }

    #[test]
    fn interleaved_f32_repeating_source_starts_at_offset_and_wraps() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("loop.pcm");
        write_samples(&path, &[0.0, 0.25, 0.5, 0.75]);
        let plan = span_plan(1, 48_000, 0, 4, 2, 0.0000834);

        let source = InterleavedF32FileRepeatingSpanSource::new(path, &plan, 4);
        let samples = source.take(5).collect::<Vec<_>>();

        assert_eq!(samples, vec![0.5, 0.75, 0.0, 0.25, 0.5]);
    }

    #[test]
    fn interleaved_f32_span_reports_truncated_file_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("truncated.pcm");
        fs::write(&path, [0_u8, 0, 0]).expect("write truncated sample");
        let plan = span_plan(1, 48_000, 0, 1, 0, 1.0);

        let mut source = InterleavedF32FileSpanSource::new(path, &plan, 1);

        assert!(source.next().is_none());
        assert!(source.last_error().is_some());
    }

    fn write_samples(path: &Path, samples: &[f32]) {
        let mut bytes = Vec::new();
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        fs::write(path, bytes).expect("write samples");
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
            PlaybackSourceIdentity::new(PlaybackSourceKind::InterleavedF32File, None),
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
