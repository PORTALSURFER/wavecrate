use radiant::runtime::{
    GpuSignalGainPreview, GpuSignalSummary, GpuSignalSummaryBucket, GpuSignalSummaryLevel,
};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    io::{Cursor, Read},
    path::PathBuf,
    sync::Arc,
};

use super::{BAND_COUNT, WAVEFORM_HEIGHT, WAVEFORM_WIDTH};
#[cfg(test)]
use super::{SYNTHETIC_SAMPLE_RATE, SYNTHETIC_SECONDS};

mod extraction;
pub(super) use extraction::extract_wav_range_to_sibling;

mod visual_bands;
#[cfg(test)]
pub(super) use visual_bands::split_frequency_bands;
pub(super) use visual_bands::split_frequency_bands_with_progress;

#[derive(Clone, Debug)]
pub(in crate::gui_app) struct WaveformFile {
    pub(super) path: PathBuf,
    pub(super) audio_bytes: Arc<[u8]>,
    pub(super) content_revision: u64,
    pub(super) sample_rate: u32,
    pub(super) channels: usize,
    pub(super) frames: usize,
    pub(super) gpu_signal_summary: Arc<GpuSignalSummary>,
}

impl WaveformFile {
    pub(super) fn path_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.path.hash(&mut hasher);
        self.frames.hash(&mut hasher);
        self.sample_rate.hash(&mut hasher);
        self.channels.hash(&mut hasher);
        hasher.finish()
    }

    pub(super) fn content_revision(&self) -> u64 {
        self.content_revision
    }
}

pub(super) fn load_waveform_file(path: PathBuf) -> Result<WaveformFile, String> {
    load_waveform_file_with_progress(path, |_| {})
}

pub(super) fn load_waveform_file_with_progress(
    path: PathBuf,
    progress: impl Fn(f32),
) -> Result<WaveformFile, String> {
    progress(0.0);
    let bytes = read_audio_file_with_progress(&path, 0.0, 0.08, &progress)?;
    if is_wav_path(&path) {
        if let Ok(file) =
            load_wav_waveform_file_with_progress(path.clone(), Arc::clone(&bytes), &progress)
        {
            return Ok(file);
        }
    }
    let decoded =
        wavecrate::waveform::WaveformRenderer::new(WAVEFORM_WIDTH as u32, WAVEFORM_HEIGHT as u32)
            .decode_from_bytes(&bytes)
            .map_err(|err| format!("failed to decode audio file: {err}"))?;
    progress(0.48);
    let channels = decoded.channel_count();
    let frames = decoded.frame_count();
    let mono_samples = if decoded.samples.is_empty() {
        decoded.analysis_samples.iter().copied().collect::<Vec<_>>()
    } else {
        downmix_to_mono_with_progress(&decoded.samples, channels, frames, 0.48, 0.62, &progress)
    };
    if mono_samples.is_empty() {
        return Err(String::from("audio file contains no complete frames"));
    }
    Ok(waveform_file_from_mono_samples_with_progress(
        path,
        bytes,
        decoded.sample_rate,
        channels,
        mono_samples,
        &progress,
    ))
}

fn read_audio_file_with_progress(
    path: &std::path::Path,
    start: f32,
    end: f32,
    progress: &impl Fn(f32),
) -> Result<Arc<[u8]>, String> {
    let mut file =
        std::fs::File::open(path).map_err(|err| format!("failed to read audio file: {err}"))?;
    let total = file.metadata().ok().map(|metadata| metadata.len() as usize);
    let mut bytes = Vec::with_capacity(total.unwrap_or_default().min(64 * 1024 * 1024));
    let mut buffer = [0_u8; 256 * 1024];
    let mut read = 0usize;
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|err| format!("failed to read audio file: {err}"))?;
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..count]);
        read = read.saturating_add(count);
        if let Some(total) = total.filter(|total| *total > 0) {
            report_phase_progress(start, end, read, total, progress);
        }
    }
    progress(end);
    Ok(bytes.into())
}

#[cfg(test)]
pub(super) fn synthetic_waveform_file() -> WaveformFile {
    let frames = SYNTHETIC_SAMPLE_RATE as usize * SYNTHETIC_SECONDS;
    let samples = (0..frames)
        .map(|frame| {
            let t = frame as f32 / SYNTHETIC_SAMPLE_RATE as f32;
            let envelope = (1.0 - t / SYNTHETIC_SECONDS as f32).clamp(0.18, 1.0);
            let low = (std::f32::consts::TAU * 72.0 * t).sin() * 0.48;
            let mid = (std::f32::consts::TAU * 220.0 * t).sin() * 0.24;
            let high = (std::f32::consts::TAU * 1_760.0 * t).sin() * 0.1;
            ((low + mid + high) * envelope).clamp(-1.0, 1.0)
        })
        .collect::<Vec<_>>();
    waveform_file_from_mono_samples(
        PathBuf::from("synthetic-waveform"),
        Arc::from([]),
        SYNTHETIC_SAMPLE_RATE,
        1,
        samples,
    )
}

pub(super) fn empty_waveform_file() -> WaveformFile {
    waveform_file_from_mono_samples(PathBuf::new(), Arc::from([]), 0, 1, vec![0.0])
}

pub(super) fn waveform_file_from_mono_samples(
    path: PathBuf,
    audio_bytes: Arc<[u8]>,
    sample_rate: u32,
    channels: usize,
    mono_samples: Vec<f32>,
) -> WaveformFile {
    waveform_file_from_mono_samples_with_progress(
        path,
        audio_bytes,
        sample_rate,
        channels,
        mono_samples,
        &|_| {},
    )
}

fn waveform_file_from_mono_samples_with_progress(
    path: PathBuf,
    audio_bytes: Arc<[u8]>,
    sample_rate: u32,
    channels: usize,
    mono_samples: Vec<f32>,
    progress: &impl Fn(f32),
) -> WaveformFile {
    let gpu_signal_samples =
        split_frequency_bands_with_progress(&mono_samples, sample_rate, 0.62, 0.9, progress);
    let gpu_signal_summary = Arc::new(gpu_signal_summary_with_progress(
        &gpu_signal_samples,
        mono_samples.len(),
        BAND_COUNT,
        0.9,
        0.99,
        progress,
    ));
    WaveformFile {
        path,
        content_revision: content_revision_for_audio_bytes(&audio_bytes),
        audio_bytes,
        sample_rate,
        channels,
        frames: mono_samples.len(),
        gpu_signal_summary,
    }
}

fn gpu_signal_summary_with_progress(
    samples: &[f32],
    frames: usize,
    band_count: usize,
    start: f32,
    end: f32,
    progress: &impl Fn(f32),
) -> GpuSignalSummary {
    let frames = frames.min(samples.len() / band_count.max(1));
    let band_count = band_count.max(1);
    let mut levels = Vec::with_capacity(signal_summary_level_count(frames));
    let mut bucket_frames = 1usize;
    let mut previous_buckets: Option<Arc<[GpuSignalSummaryBucket]>> = None;
    let total_levels = signal_summary_level_count(frames).max(1);
    while bucket_frames <= frames.max(1) {
        let level_index = levels.len();
        let level_start = start + (end - start) * (level_index as f32 / total_levels as f32);
        let level_end = start + (end - start) * ((level_index + 1) as f32 / total_levels as f32);
        let buckets = match previous_buckets.as_deref() {
            Some(previous) => merge_signal_summary_level_with_progress(
                previous,
                frames,
                band_count,
                bucket_frames,
                level_start,
                level_end,
                progress,
            ),
            None => build_signal_summary_base_level_with_progress(
                samples,
                frames,
                band_count,
                level_start,
                level_end,
                progress,
            ),
        };
        levels.push(GpuSignalSummaryLevel {
            bucket_frames,
            buckets: Arc::clone(&buckets),
        });
        previous_buckets = Some(buckets);
        if bucket_frames >= frames.max(1) {
            break;
        }
        bucket_frames = bucket_frames.saturating_mul(2).max(bucket_frames + 1);
    }
    progress(end);
    GpuSignalSummary {
        frames,
        band_count,
        levels,
    }
}

fn signal_summary_level_count(frames: usize) -> usize {
    let frames = frames.max(1);
    usize::BITS as usize - frames.leading_zeros() as usize
}

fn build_signal_summary_base_level_with_progress(
    samples: &[f32],
    frames: usize,
    band_count: usize,
    start: f32,
    end: f32,
    progress: &impl Fn(f32),
) -> Arc<[GpuSignalSummaryBucket]> {
    if frames == 0 {
        return vec![GpuSignalSummaryBucket::default(); band_count].into();
    }
    let sample_count = frames.saturating_mul(band_count);
    let mut buckets = Vec::with_capacity(sample_count);
    for (index, value) in samples.iter().copied().take(sample_count).enumerate() {
        if value.is_finite() {
            buckets.push(GpuSignalSummaryBucket {
                min: value,
                max: value,
            });
        } else {
            buckets.push(GpuSignalSummaryBucket::default());
        }
        report_phase_progress_throttled(start, end, index + 1, sample_count, progress);
    }
    progress(end);
    buckets.into()
}

fn merge_signal_summary_level_with_progress(
    previous: &[GpuSignalSummaryBucket],
    frames: usize,
    band_count: usize,
    bucket_frames: usize,
    start: f32,
    end: f32,
    progress: &impl Fn(f32),
) -> Arc<[GpuSignalSummaryBucket]> {
    let bucket_count = frames.div_ceil(bucket_frames.max(1)).max(1);
    let previous_bucket_count = previous.len() / band_count.max(1);
    let mut buckets = Vec::with_capacity(bucket_count.saturating_mul(band_count));
    for bucket in 0..bucket_count {
        let first = bucket.saturating_mul(2);
        let second = first + 1;
        for band in 0..band_count {
            let mut summary = previous
                .get(first.saturating_mul(band_count).saturating_add(band))
                .copied()
                .unwrap_or_default();
            if second < previous_bucket_count
                && let Some(next) =
                    previous.get(second.saturating_mul(band_count).saturating_add(band))
            {
                summary.min = summary.min.min(next.min);
                summary.max = summary.max.max(next.max);
            }
            buckets.push(summary);
        }
        report_phase_progress_throttled(start, end, bucket + 1, bucket_count, progress);
    }
    progress(end);
    buckets.into()
}

pub(super) fn gain_preview_for_selection(
    selection: Option<wavecrate::selection::SelectionRange>,
) -> Option<GpuSignalGainPreview> {
    let selection = selection.filter(|selection| selection.has_edit_effects())?;
    let fade_in = selection.fade_in();
    let fade_out = selection.fade_out();
    Some(GpuSignalGainPreview {
        start: selection.start(),
        end: selection.end(),
        gain: selection.gain(),
        fade_in_length: fade_in.map(|fade| fade.length).unwrap_or(0.0),
        fade_in_curve: fade_in.map(|fade| fade.curve).unwrap_or(0.5),
        fade_in_mute: fade_in.map(|fade| fade.mute).unwrap_or(0.0),
        fade_out_length: fade_out.map(|fade| fade.length).unwrap_or(0.0),
        fade_out_curve: fade_out.map(|fade| fade.curve).unwrap_or(0.5),
        fade_out_mute: fade_out.map(|fade| fade.mute).unwrap_or(0.0),
    })
}

pub(super) fn is_wav_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
}

pub(super) fn content_revision_for_audio_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish().max(1)
}

pub(super) fn load_wav_waveform_file_with_progress(
    path: PathBuf,
    bytes: Arc<[u8]>,
    progress: &impl Fn(f32),
) -> Result<WaveformFile, String> {
    let cursor = Cursor::new(bytes.as_ref());
    let mut reader =
        hound::WavReader::new(cursor).map_err(|err| format!("failed to open WAV: {err}"))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels).max(1);
    let total_samples = reader.duration() as usize * channels;
    let samples = match spec.sample_format {
        hound::SampleFormat::Float => {
            let mut samples = Vec::with_capacity(total_samples);
            for (index, sample) in reader.samples::<f32>().enumerate() {
                let sample = sample
                    .map(|value| value.clamp(-1.0, 1.0))
                    .map_err(|err| format!("failed to read float sample: {err}"))?;
                samples.push(sample);
                report_phase_progress_throttled(0.08, 0.46, index + 1, total_samples, progress);
            }
            progress(0.46);
            samples
        }
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => {
            let max =
                ((1_i32 << (u32::from(spec.bits_per_sample).saturating_sub(1))) - 1).max(1) as f32;
            let mut samples = Vec::with_capacity(total_samples);
            for (index, sample) in reader.samples::<i16>().enumerate() {
                let sample = sample
                    .map(|value| (f32::from(value) / max).clamp(-1.0, 1.0))
                    .map_err(|err| format!("failed to read integer sample: {err}"))?;
                samples.push(sample);
                report_phase_progress_throttled(0.08, 0.46, index + 1, total_samples, progress);
            }
            progress(0.46);
            samples
        }
        hound::SampleFormat::Int => {
            let max =
                ((1_i64 << (u32::from(spec.bits_per_sample).saturating_sub(1))) - 1).max(1) as f32;
            let mut samples = Vec::with_capacity(total_samples);
            for (index, sample) in reader.samples::<i32>().enumerate() {
                let sample = sample
                    .map(|value| ((value as f32) / max).clamp(-1.0, 1.0))
                    .map_err(|err| format!("failed to read integer sample: {err}"))?;
                samples.push(sample);
                report_phase_progress_throttled(0.08, 0.46, index + 1, total_samples, progress);
            }
            progress(0.46);
            samples
        }
    };
    if samples.is_empty() {
        return Err(String::from("WAV contains no samples"));
    }

    let frames = samples.len() / channels;
    let mono_samples =
        downmix_to_mono_with_progress(&samples, channels, frames, 0.46, 0.62, progress);
    if mono_samples.is_empty() {
        return Err(String::from("WAV contains no complete frames"));
    }
    Ok(waveform_file_from_mono_samples_with_progress(
        path,
        bytes,
        spec.sample_rate,
        channels,
        mono_samples,
        progress,
    ))
}

#[cfg(test)]
pub(super) fn downmix_to_mono(samples: &[f32], channels: usize, frames: usize) -> Vec<f32> {
    downmix_to_mono_with_progress(samples, channels, frames, 0.0, 1.0, &|_| {})
}

fn downmix_to_mono_with_progress(
    samples: &[f32],
    channels: usize,
    frames: usize,
    start: f32,
    end: f32,
    progress: &impl Fn(f32),
) -> Vec<f32> {
    let channels = channels.max(1);
    let mut mono = Vec::with_capacity(frames);
    for frame in 0..frames {
        let sample_start = frame * channels;
        let mut peak = 0.0_f32;
        for sample in samples[sample_start..sample_start + channels]
            .iter()
            .copied()
        {
            if sample.abs() > peak.abs() {
                peak = sample;
            }
        }
        mono.push(peak.clamp(-1.0, 1.0));
        report_phase_progress_throttled(start, end, frame + 1, frames, progress);
    }
    progress(end);
    mono
}

pub(super) fn report_phase_progress_throttled(
    start: f32,
    end: f32,
    completed: usize,
    total: usize,
    progress: &impl Fn(f32),
) {
    if completed == total || completed % 16_384 == 0 {
        report_phase_progress(start, end, completed, total, progress);
    }
}

fn report_phase_progress(
    start: f32,
    end: f32,
    completed: usize,
    total: usize,
    progress: &impl Fn(f32),
) {
    if total == 0 {
        return;
    }
    let ratio = completed as f32 / total as f32;
    progress(start + (end - start) * ratio.clamp(0.0, 1.0));
}
