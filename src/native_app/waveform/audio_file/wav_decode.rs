use radiant::runtime::GpuSignalSummaryBucket;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    io::Cursor,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Instant, SystemTime},
};

use super::super::BAND_COUNT;
use super::{
    WaveformFile, WaveformPlaybackReady, downmix_to_mono_with_progress_and_cancel,
    report_phase_progress_throttled,
    signal_summary::gpu_signal_summary_from_base_buckets_with_progress_and_cancel,
    visual_bands::{VisualBandFrameProcessor, normalize_visual_band_summary_buckets},
};
use crate::native_app::waveform::audio_file::diagnostics::log_audio_load_timing;

const STREAMING_WAV_SUMMARY_READ_END: f32 = 0.88;
const STREAMING_WAV_SUMMARY_BUILD_END: f32 = 0.99;
#[cfg(test)]
const MAX_STREAMING_WAV_SUMMARY_BUCKETS: usize = 128;
#[cfg(not(test))]
const MAX_STREAMING_WAV_SUMMARY_BUCKETS: usize = 65_536;

pub(in crate::native_app::waveform) fn load_wav_waveform_file_with_progress(
    path: PathBuf,
    bytes: Arc<[u8]>,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
    playback_ready: &impl Fn(WaveformPlaybackReady),
) -> Result<WaveformFile, String> {
    let cursor = Cursor::new(bytes.as_ref());
    let mut reader =
        hound::WavReader::new(cursor).map_err(|err| format!("failed to open WAV: {err}"))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels).max(1);
    let sample_started_at = Instant::now();
    let samples = read_wav_samples_with_progress(&mut reader, spec, channels, progress, cancelled)?;
    log_audio_load_timing(
        "browser.audio_file.wav.read_samples",
        &path,
        sample_started_at.elapsed(),
    );
    if samples.is_empty() {
        return Err(String::from("WAV contains no samples"));
    }

    let frames = samples.len() / channels;
    let playback_samples = Arc::from(samples);
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    playback_ready(WaveformPlaybackReady {
        path: path.clone(),
        audio_bytes: Arc::clone(&bytes),
        playback_samples: Arc::clone(&playback_samples),
        sample_rate: spec.sample_rate,
        channels,
        frames,
    });
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    let downmix_started_at = Instant::now();
    let mono_samples = downmix_to_mono_with_progress_and_cancel(
        &playback_samples,
        channels,
        frames,
        0.46,
        0.62,
        progress,
        cancelled,
    )?;
    log_audio_load_timing(
        "browser.audio_file.wav.downmix",
        &path,
        downmix_started_at.elapsed(),
    );
    if mono_samples.is_empty() {
        return Err(String::from("WAV contains no complete frames"));
    }
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    let waveform_started_at = Instant::now();
    let mut file = super::waveform_file_from_mono_samples_with_progress_and_cancel(
        path,
        bytes,
        spec.sample_rate,
        channels,
        mono_samples,
        progress,
        cancelled,
    )?;
    log_audio_load_timing(
        "browser.audio_file.wav.waveform_summary",
        &file.path,
        waveform_started_at.elapsed(),
    );
    file.playback_samples = Some(playback_samples);
    Ok(file)
}

pub(in crate::native_app::waveform) fn load_wav_waveform_summary_from_path_with_progress(
    path: PathBuf,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<WaveformFile, String> {
    let mut reader =
        hound::WavReader::open(&path).map_err(|err| format!("failed to open WAV: {err}"))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels).max(1);
    let total_frames = reader.duration() as usize;
    let bucket_frames = streaming_summary_bucket_frames(total_frames);
    let mut builder = StreamingWavSummaryBuilder::new(spec.sample_rate, bucket_frames);
    let read_started_at = Instant::now();
    read_wav_summary_with_progress(
        &mut reader,
        spec,
        channels,
        &mut builder,
        progress,
        cancelled,
    )?;
    log_audio_load_timing(
        "browser.audio_file.wav.stream_summary_read",
        &path,
        read_started_at.elapsed(),
    );
    if builder.frames() == 0 {
        return Err(String::from("WAV contains no complete frames"));
    }
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    let summary_started_at = Instant::now();
    let summary = builder.finish(
        STREAMING_WAV_SUMMARY_READ_END,
        STREAMING_WAV_SUMMARY_BUILD_END,
        progress,
        cancelled,
    )?;
    log_audio_load_timing(
        "browser.audio_file.wav.stream_summary_build",
        &path,
        summary_started_at.elapsed(),
    );
    let metadata = std::fs::metadata(&path)
        .map_err(|err| format!("failed to read WAV metadata {}: {err}", path.display()))?;
    let frames = summary.frames;
    Ok(WaveformFile {
        path: path.clone(),
        content_revision: content_revision_for_path_metadata(
            &path,
            &metadata,
            spec.sample_rate,
            channels,
            frames,
        ),
        audio_bytes: Arc::from([]),
        playback_samples: None,
        playback_cache_file: None,
        sample_rate: spec.sample_rate,
        channels,
        frames,
        gpu_signal_summary: Arc::new(summary),
    })
}

pub(super) fn read_wav_playback_samples(bytes: &Arc<[u8]>) -> Result<Vec<f32>, String> {
    let cursor = Cursor::new(bytes.as_ref());
    let mut reader =
        hound::WavReader::new(cursor).map_err(|err| format!("failed to open WAV: {err}"))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels).max(1);
    read_wav_samples_with_progress(&mut reader, spec, channels, &|_| {}, &|| false)
}

fn streaming_summary_bucket_frames(total_frames: usize) -> usize {
    total_frames
        .div_ceil(MAX_STREAMING_WAV_SUMMARY_BUCKETS)
        .max(1)
}

struct StreamingWavSummaryBuilder {
    processor: VisualBandFrameProcessor,
    bucket_frames: usize,
    current_bucket_frame_count: usize,
    current_bucket: Vec<GpuSignalSummaryBucket>,
    buckets: Vec<GpuSignalSummaryBucket>,
    frames: usize,
}

impl StreamingWavSummaryBuilder {
    fn new(sample_rate: u32, bucket_frames: usize) -> Self {
        Self {
            processor: VisualBandFrameProcessor::new(sample_rate),
            bucket_frames: bucket_frames.max(1),
            current_bucket_frame_count: 0,
            current_bucket: empty_summary_bucket(),
            buckets: Vec::new(),
            frames: 0,
        }
    }

    fn push_peak(&mut self, mono: f32) {
        let bands = self.processor.process(mono);
        for (bucket, value) in self.current_bucket.iter_mut().zip(bands) {
            bucket.min = bucket.min.min(value);
            bucket.max = bucket.max.max(value);
        }
        self.frames = self.frames.saturating_add(1);
        self.current_bucket_frame_count = self.current_bucket_frame_count.saturating_add(1);
        if self.current_bucket_frame_count >= self.bucket_frames {
            self.flush_current_bucket();
        }
    }

    fn frames(&self) -> usize {
        self.frames
    }

    fn finish(
        mut self,
        start: f32,
        end: f32,
        progress: &impl Fn(f32),
        cancelled: &impl Fn() -> bool,
    ) -> Result<radiant::runtime::GpuSignalSummary, String> {
        self.flush_current_bucket();
        normalize_visual_band_summary_buckets(&mut self.buckets, BAND_COUNT, cancelled)?;
        let base = Arc::<[GpuSignalSummaryBucket]>::from(self.buckets);
        gpu_signal_summary_from_base_buckets_with_progress_and_cancel(
            self.frames,
            BAND_COUNT,
            self.bucket_frames,
            base,
            start,
            end,
            progress,
            cancelled,
        )
    }

    fn flush_current_bucket(&mut self) {
        if self.current_bucket_frame_count == 0 {
            return;
        }
        self.buckets
            .extend(self.current_bucket.iter().map(|bucket| {
                if bucket.min.is_finite() && bucket.max.is_finite() {
                    *bucket
                } else {
                    GpuSignalSummaryBucket::default()
                }
            }));
        self.current_bucket = empty_summary_bucket();
        self.current_bucket_frame_count = 0;
    }
}

fn empty_summary_bucket() -> Vec<GpuSignalSummaryBucket> {
    vec![
        GpuSignalSummaryBucket {
            min: f32::INFINITY,
            max: f32::NEG_INFINITY,
        };
        BAND_COUNT
    ]
}

fn read_wav_summary_with_progress<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    spec: hound::WavSpec,
    channels: usize,
    builder: &mut StreamingWavSummaryBuilder,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<(), String> {
    let total_frames = reader.duration() as usize;
    match spec.sample_format {
        hound::SampleFormat::Float => {
            read_float_summary(reader, channels, builder, total_frames, progress, cancelled)
        }
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => read_i16_summary(
            reader,
            channels,
            spec.bits_per_sample,
            builder,
            total_frames,
            progress,
            cancelled,
        ),
        hound::SampleFormat::Int => read_i32_summary(
            reader,
            channels,
            spec.bits_per_sample,
            builder,
            total_frames,
            progress,
            cancelled,
        ),
    }
}

fn read_float_summary<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    channels: usize,
    builder: &mut StreamingWavSummaryBuilder,
    total_frames: usize,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<(), String> {
    let mut samples = reader.samples::<f32>();
    for frame_index in 0..total_frames {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
        let peak = read_float_frame_peak(&mut samples, channels)?;
        builder.push_peak(peak);
        report_phase_progress_throttled(
            0.0,
            STREAMING_WAV_SUMMARY_READ_END,
            frame_index + 1,
            total_frames,
            progress,
        );
    }
    progress(STREAMING_WAV_SUMMARY_READ_END);
    Ok(())
}

fn read_i16_summary<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    channels: usize,
    bits_per_sample: u16,
    builder: &mut StreamingWavSummaryBuilder,
    total_frames: usize,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<(), String> {
    let max = integer_sample_max_i32(bits_per_sample);
    let mut samples = reader.samples::<i16>();
    for frame_index in 0..total_frames {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
        let peak = read_i16_frame_peak(&mut samples, channels, max)?;
        builder.push_peak(peak);
        report_phase_progress_throttled(
            0.0,
            STREAMING_WAV_SUMMARY_READ_END,
            frame_index + 1,
            total_frames,
            progress,
        );
    }
    progress(STREAMING_WAV_SUMMARY_READ_END);
    Ok(())
}

fn read_i32_summary<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    channels: usize,
    bits_per_sample: u16,
    builder: &mut StreamingWavSummaryBuilder,
    total_frames: usize,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<(), String> {
    let max = integer_sample_max_i64(bits_per_sample);
    let mut samples = reader.samples::<i32>();
    for frame_index in 0..total_frames {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
        let peak = read_i32_frame_peak(&mut samples, channels, max)?;
        builder.push_peak(peak);
        report_phase_progress_throttled(
            0.0,
            STREAMING_WAV_SUMMARY_READ_END,
            frame_index + 1,
            total_frames,
            progress,
        );
    }
    progress(STREAMING_WAV_SUMMARY_READ_END);
    Ok(())
}

fn read_float_frame_peak<R: std::io::Read>(
    samples: &mut hound::WavSamples<'_, R, f32>,
    channels: usize,
) -> Result<f32, String> {
    let mut peak = 0.0_f32;
    for _ in 0..channels {
        let sample = samples
            .next()
            .transpose()
            .map_err(|err| format!("failed to read float sample: {err}"))?
            .ok_or_else(|| String::from("WAV ended before declared frame count"))?
            .clamp(-1.0, 1.0);
        if sample.abs() > peak.abs() {
            peak = sample;
        }
    }
    Ok(peak.clamp(-1.0, 1.0))
}

fn read_i16_frame_peak<R: std::io::Read>(
    samples: &mut hound::WavSamples<'_, R, i16>,
    channels: usize,
    max: f32,
) -> Result<f32, String> {
    let mut peak = 0.0_f32;
    for _ in 0..channels {
        let sample = samples
            .next()
            .transpose()
            .map_err(|err| format!("failed to read integer sample: {err}"))?
            .ok_or_else(|| String::from("WAV ended before declared frame count"))
            .map(|value| (f32::from(value) / max).clamp(-1.0, 1.0))?;
        if sample.abs() > peak.abs() {
            peak = sample;
        }
    }
    Ok(peak.clamp(-1.0, 1.0))
}

fn read_i32_frame_peak<R: std::io::Read>(
    samples: &mut hound::WavSamples<'_, R, i32>,
    channels: usize,
    max: f32,
) -> Result<f32, String> {
    let mut peak = 0.0_f32;
    for _ in 0..channels {
        let sample = samples
            .next()
            .transpose()
            .map_err(|err| format!("failed to read integer sample: {err}"))?
            .ok_or_else(|| String::from("WAV ended before declared frame count"))
            .map(|value| ((value as f32) / max).clamp(-1.0, 1.0))?;
        if sample.abs() > peak.abs() {
            peak = sample;
        }
    }
    Ok(peak.clamp(-1.0, 1.0))
}

fn content_revision_for_path_metadata(
    path: &Path,
    metadata: &std::fs::Metadata,
    sample_rate: u32,
    channels: usize,
    frames: usize,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.hash(&mut hasher);
    metadata.len().hash(&mut hasher);
    modified_ns(metadata).hash(&mut hasher);
    sample_rate.hash(&mut hasher);
    channels.hash(&mut hasher);
    frames.hash(&mut hasher);
    hasher.finish().max(1)
}

fn modified_ns(metadata: &std::fs::Metadata) -> u128 {
    metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

fn read_wav_samples_with_progress<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    spec: hound::WavSpec,
    channels: usize,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<Vec<f32>, String> {
    let total_samples = reader.duration() as usize * channels;
    match spec.sample_format {
        hound::SampleFormat::Float => {
            read_float_samples(reader, total_samples, progress, cancelled)
        }
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => read_i16_samples(
            reader,
            spec.bits_per_sample,
            total_samples,
            progress,
            cancelled,
        ),
        hound::SampleFormat::Int => read_i32_samples(
            reader,
            spec.bits_per_sample,
            total_samples,
            progress,
            cancelled,
        ),
    }
}

fn read_float_samples<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    total_samples: usize,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<Vec<f32>, String> {
    let mut samples = Vec::with_capacity(total_samples);
    for (index, sample) in reader.samples::<f32>().enumerate() {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
        let sample = sample
            .map(|value| value.clamp(-1.0, 1.0))
            .map_err(|err| format!("failed to read float sample: {err}"))?;
        samples.push(sample);
        report_phase_progress_throttled(0.08, 0.46, index + 1, total_samples, progress);
    }
    progress(0.46);
    Ok(samples)
}

fn read_i16_samples<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    bits_per_sample: u16,
    total_samples: usize,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<Vec<f32>, String> {
    let max = integer_sample_max_i32(bits_per_sample);
    let mut samples = Vec::with_capacity(total_samples);
    for (index, sample) in reader.samples::<i16>().enumerate() {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
        let sample = sample
            .map(|value| (f32::from(value) / max).clamp(-1.0, 1.0))
            .map_err(|err| format!("failed to read integer sample: {err}"))?;
        samples.push(sample);
        report_phase_progress_throttled(0.08, 0.46, index + 1, total_samples, progress);
    }
    progress(0.46);
    Ok(samples)
}

fn read_i32_samples<R: std::io::Read>(
    reader: &mut hound::WavReader<R>,
    bits_per_sample: u16,
    total_samples: usize,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<Vec<f32>, String> {
    let max = integer_sample_max_i64(bits_per_sample);
    let mut samples = Vec::with_capacity(total_samples);
    for (index, sample) in reader.samples::<i32>().enumerate() {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
        let sample = sample
            .map(|value| ((value as f32) / max).clamp(-1.0, 1.0))
            .map_err(|err| format!("failed to read integer sample: {err}"))?;
        samples.push(sample);
        report_phase_progress_throttled(0.08, 0.46, index + 1, total_samples, progress);
    }
    progress(0.46);
    Ok(samples)
}

fn integer_sample_max_i32(bits_per_sample: u16) -> f32 {
    ((1_i32 << (u32::from(bits_per_sample).saturating_sub(1))) - 1).max(1) as f32
}

fn integer_sample_max_i64(bits_per_sample: u16) -> f32 {
    ((1_i64 << (u32::from(bits_per_sample).saturating_sub(1))) - 1).max(1) as f32
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        path::Path,
        path::PathBuf,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };

    use super::*;

    fn wav_bytes_i16(channels: u16, samples: &[i16]) -> Arc<[u8]> {
        let spec = hound::WavSpec {
            channels,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut cursor, spec)
                .expect("test wav writer should be created");
            for &sample in samples {
                writer
                    .write_sample(sample)
                    .expect("test wav sample should be written");
            }
            writer.finalize().expect("test wav should be finalized");
        }
        Arc::from(cursor.into_inner())
    }

    fn write_wav_i16(path: &Path, channels: u16, samples: &[i16]) {
        let spec = hound::WavSpec {
            channels,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).expect("test wav writer");
        for sample in samples {
            writer.write_sample(*sample).expect("write sample");
        }
        writer.finalize().expect("finalize wav");
    }

    #[test]
    fn cancellation_after_playback_ready_stops_waveform_summary() {
        let bytes = wav_bytes_i16(2, &[0, 0, 1000, -1000, 2000, -2000, 0, 0]);
        let cancelled = AtomicBool::new(false);
        let playback_ready_called = AtomicBool::new(false);

        let result = load_wav_waveform_file_with_progress(
            PathBuf::from("cancelled-after-playback-ready.wav"),
            bytes,
            &|_| {},
            &|| cancelled.load(Ordering::Relaxed),
            &|_| {
                playback_ready_called.store(true, Ordering::Relaxed);
                cancelled.store(true, Ordering::Relaxed);
            },
        );

        assert!(matches!(result, Err(error) if error == "cancelled"));
        assert!(playback_ready_called.load(Ordering::Relaxed));
    }

    #[test]
    fn streaming_wav_summary_keeps_large_file_playback_file_backed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("large-summary.wav");
        write_wav_i16(
            &path,
            2,
            &[0, 0, 1_000, -4_000, 6_000, -2_000, 0, 0, -8_000, 4_000],
        );
        let progress_values = RefCell::new(Vec::new());

        let file = load_wav_waveform_summary_from_path_with_progress(
            path.clone(),
            &|progress| progress_values.borrow_mut().push(progress),
            &|| false,
        )
        .expect("streaming summary");

        assert_eq!(file.path, path);
        assert_eq!(file.sample_rate, 48_000);
        assert_eq!(file.channels, 2);
        assert_eq!(file.frames, 5);
        assert!(file.audio_bytes.is_empty());
        assert!(file.playback_samples.is_none());
        assert!(file.playback_cache_file.is_none());
        assert_eq!(file.gpu_signal_summary.frames, 5);
        assert!(!file.gpu_signal_summary.levels.is_empty());
        assert!(progress_values.borrow().iter().any(|value| *value >= 0.99));
    }
}
