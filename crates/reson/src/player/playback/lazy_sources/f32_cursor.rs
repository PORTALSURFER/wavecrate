use std::{
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use crate::telemetry;

use super::{RepeatReadRequest, SourceFormat};

const F32_SAMPLE_BYTES: u64 = std::mem::size_of::<f32>() as u64;

pub(super) struct F32FileCursor {
    path: PathBuf,
    reader: Option<BufReader<File>>,
    position: u64,
}

impl F32FileCursor {
    pub(super) fn new(path: PathBuf, position: u64) -> Self {
        Self {
            path,
            reader: None,
            position,
        }
    }

    pub(super) fn position(&self) -> u64 {
        self.position
    }

    pub(super) fn seek_logically_to(&mut self, position: u64) {
        self.position = position;
    }

    pub(super) fn close(&mut self) {
        self.reader = None;
    }

    pub(super) fn is_closed(&self) -> bool {
        self.reader.is_none()
    }

    pub(super) fn read_next(&mut self, open_stage: &'static str) -> Result<f32, String> {
        self.reader(open_stage)?;
        let mut bytes = [0_u8; 4];
        let reader = self.reader.as_mut().expect("reader opened above");
        reader
            .read_exact(&mut bytes)
            .map_err(|err| format!("Failed to read f32 playback cache: {err}"))?;
        self.position = self.position.saturating_add(1);
        Ok(f32::from_le_bytes(bytes).clamp(-1.0, 1.0))
    }

    fn reader(&mut self, open_stage: &'static str) -> Result<(), String> {
        if self.reader.is_none() {
            self.open_at(self.position, open_stage)?;
        }
        Ok(())
    }

    pub(super) fn open_at(&mut self, sample: u64, stage: &'static str) -> Result<(), String> {
        let started_at = telemetry::playback_telemetry_enabled().then(std::time::Instant::now);
        let reader = open_f32_reader_at(&self.path, sample)?;
        self.reader = Some(reader);
        self.position = sample;
        if let Some(started_at) = started_at {
            tracing::info!(
                target: "perf::audio_start",
                module = "reson_f32_file_source",
                stage,
                path = %self.path.display(),
                start_sample = sample,
                elapsed_ms = telemetry::elapsed_ms(started_at.elapsed()),
                "Interleaved f32 file playback stage"
            );
        }
        Ok(())
    }
}

pub(super) struct F32RepeatCycle {
    start_sample: u64,
    span_samples: u64,
    samples_into_cycle: u64,
    initial_offset_samples: u64,
}

impl F32RepeatCycle {
    pub(super) fn new(
        request: RepeatReadRequest,
        format: SourceFormat,
        total_samples: u64,
    ) -> Self {
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

    pub(super) fn initial_sample(&self) -> u64 {
        self.sample_for_cycle_offset(self.initial_offset_samples)
    }

    pub(super) fn initial_offset(&self) -> u64 {
        self.initial_offset_samples
    }

    pub(super) fn is_complete(&self) -> bool {
        self.samples_into_cycle >= self.span_samples
    }

    pub(super) fn sample_for_cycle_offset(&self, cycle_sample_offset: u64) -> u64 {
        self.start_sample
            .saturating_add(cycle_sample_offset.min(self.span_samples))
    }

    pub(super) fn seek_to(&mut self, cycle_sample_offset: u64) {
        self.samples_into_cycle = cycle_sample_offset.min(self.span_samples);
    }

    pub(super) fn advance(&mut self) {
        self.samples_into_cycle = self.samples_into_cycle.saturating_add(1);
    }
}

fn open_f32_reader_at(path: &Path, sample: u64) -> Result<BufReader<File>, String> {
    let mut file = File::open(path).map_err(|err| {
        format!(
            "Failed to open f32 playback cache {}: {err}",
            path.display()
        )
    })?;
    file.seek(SeekFrom::Start(sample.saturating_mul(F32_SAMPLE_BYTES)))
        .map_err(|err| {
            format!(
                "Failed to seek f32 playback cache {}: {err}",
                path.display()
            )
        })?;
    Ok(BufReader::new(file))
}
