use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use crate::telemetry;

use super::SourceFormat;

pub(super) struct PcmWavFileCursor {
    path: PathBuf,
    format: SourceFormat,
    reader: Option<PcmWavSampleStream>,
    position: u64,
}

impl PcmWavFileCursor {
    pub(super) fn new(path: PathBuf, format: SourceFormat, position: u64) -> Self {
        Self {
            path,
            format,
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
        let reader = self.reader.as_mut().expect("reader opened above");
        let sample = reader.read_next()?;
        self.position = self.position.saturating_add(1);
        Ok(sample)
    }

    fn reader(&mut self, open_stage: &'static str) -> Result<(), String> {
        if self.reader.is_none() {
            self.open_at(self.position, open_stage)?;
        }
        Ok(())
    }

    pub(super) fn open_at(&mut self, sample: u64, stage: &'static str) -> Result<(), String> {
        let started_at = telemetry::playback_telemetry_enabled().then(std::time::Instant::now);
        let reader = open_pcm_wav_reader_at(&self.path, self.format, sample)?;
        self.reader = Some(reader);
        self.position = sample;
        if let Some(started_at) = started_at {
            tracing::info!(
                target: "perf::audio_start",
                module = "reson_pcm_wav_file_source",
                stage,
                path = %self.path.display(),
                start_sample = sample,
                elapsed_ms = telemetry::elapsed_ms(started_at.elapsed()),
                "PCM WAV file playback stage"
            );
        }
        Ok(())
    }
}

enum PcmWavSampleStream {
    Float(hound::WavIntoSamples<BufReader<File>, f32>),
    I16(hound::WavIntoSamples<BufReader<File>, i16>, f32),
    I32(hound::WavIntoSamples<BufReader<File>, i32>, f32),
}

impl PcmWavSampleStream {
    fn read_next(&mut self) -> Result<f32, String> {
        match self {
            Self::Float(samples) => samples
                .next()
                .transpose()
                .map_err(|err| format!("Failed to read float WAV sample: {err}"))?
                .map(|sample| sample.clamp(-1.0, 1.0))
                .ok_or_else(|| String::from("Reached end of WAV file")),
            Self::I16(samples, max) => samples
                .next()
                .transpose()
                .map_err(|err| format!("Failed to read integer WAV sample: {err}"))?
                .map(|sample| (f32::from(sample) / *max).clamp(-1.0, 1.0))
                .ok_or_else(|| String::from("Reached end of WAV file")),
            Self::I32(samples, max) => samples
                .next()
                .transpose()
                .map_err(|err| format!("Failed to read integer WAV sample: {err}"))?
                .map(|sample| ((sample as f32) / *max).clamp(-1.0, 1.0))
                .ok_or_else(|| String::from("Reached end of WAV file")),
        }
    }
}

pub(super) fn validate_pcm_wav_file(path: &Path, expected: SourceFormat) -> Result<u64, String> {
    let reader = hound::WavReader::open(path)
        .map_err(|err| format!("Failed to open PCM WAV file {}: {err}", path.display()))?;
    let spec = reader.spec();
    validate_pcm_wav_spec(path, spec, expected)?;
    Ok(reader.duration() as u64)
}

fn validate_pcm_wav_spec(
    path: &Path,
    spec: hound::WavSpec,
    expected: SourceFormat,
) -> Result<(), String> {
    let channels = spec.channels.max(1);
    if channels != expected.channels() {
        return Err(format!(
            "WAV channel count changed for {}: expected {}, got {}",
            path.display(),
            expected.channels(),
            channels
        ));
    }
    if spec.sample_rate != expected.sample_rate() {
        return Err(format!(
            "WAV sample rate changed for {}: expected {}, got {}",
            path.display(),
            expected.sample_rate(),
            spec.sample_rate
        ));
    }
    match spec.sample_format {
        hound::SampleFormat::Float if spec.bits_per_sample == 32 => Ok(()),
        hound::SampleFormat::Int if (1..=32).contains(&spec.bits_per_sample) => Ok(()),
        hound::SampleFormat::Float => Err(format!(
            "Unsupported WAV float bit depth for direct playback: {}",
            spec.bits_per_sample
        )),
        hound::SampleFormat::Int => Err(format!(
            "Unsupported WAV integer bit depth for direct playback: {}",
            spec.bits_per_sample
        )),
    }
}

fn open_pcm_wav_reader_at(
    path: &Path,
    expected: SourceFormat,
    sample: u64,
) -> Result<PcmWavSampleStream, String> {
    let mut reader = hound::WavReader::open(path)
        .map_err(|err| format!("Failed to open PCM WAV file {}: {err}", path.display()))?;
    let spec = reader.spec();
    validate_pcm_wav_spec(path, spec, expected)?;
    let channels = u64::from(spec.channels.max(1));
    if !sample.is_multiple_of(channels) {
        return Err(format!(
            "Direct WAV playback seek is not frame-aligned: sample {sample}, channels {channels}"
        ));
    }
    let frame = u32::try_from(sample / channels)
        .map_err(|_| format!("Direct WAV playback seek is beyond RIFF WAV range: {sample}"))?;
    reader.seek(frame).map_err(|err| {
        format!(
            "Failed to seek PCM WAV file {} to frame {frame}: {err}",
            path.display()
        )
    })?;
    match spec.sample_format {
        hound::SampleFormat::Float => Ok(PcmWavSampleStream::Float(reader.into_samples::<f32>())),
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => Ok(PcmWavSampleStream::I16(
            reader.into_samples::<i16>(),
            integer_sample_max_i32(spec.bits_per_sample),
        )),
        hound::SampleFormat::Int => Ok(PcmWavSampleStream::I32(
            reader.into_samples::<i32>(),
            integer_sample_max_i64(spec.bits_per_sample),
        )),
    }
}

fn integer_sample_max_i32(bits_per_sample: u16) -> f32 {
    ((1_i32 << (u32::from(bits_per_sample).saturating_sub(1))) - 1).max(1) as f32
}

fn integer_sample_max_i64(bits_per_sample: u16) -> f32 {
    ((1_i64 << (u32::from(bits_per_sample).saturating_sub(1))) - 1).max(1) as f32
}
