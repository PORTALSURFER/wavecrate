use radiant::runtime::{GpuSignalGainPreview, GpuSignalSummary};
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    io::Cursor,
    path::PathBuf,
    sync::Arc,
};

use super::{BAND_COUNT, WAVEFORM_HEIGHT, WAVEFORM_WIDTH};
#[cfg(test)]
use super::{SYNTHETIC_SAMPLE_RATE, SYNTHETIC_SECONDS};

mod extraction;
pub(super) use extraction::extract_wav_range_to_sibling;

mod visual_bands;
pub(super) use visual_bands::split_frequency_bands;

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
    let bytes: Arc<[u8]> = fs::read(&path)
        .map_err(|err| format!("failed to read audio file: {err}"))?
        .into();
    if is_wav_path(&path) {
        if let Ok(file) = load_wav_waveform_file(path.clone(), Arc::clone(&bytes)) {
            return Ok(file);
        }
    }
    let decoded =
        wavecrate::waveform::WaveformRenderer::new(WAVEFORM_WIDTH as u32, WAVEFORM_HEIGHT as u32)
            .decode_from_bytes(&bytes)
            .map_err(|err| format!("failed to decode audio file: {err}"))?;
    let channels = decoded.channel_count();
    let frames = decoded.frame_count();
    let mono_samples = if decoded.samples.is_empty() {
        decoded.analysis_samples.iter().copied().collect::<Vec<_>>()
    } else {
        downmix_to_mono(&decoded.samples, channels, frames)
    };
    if mono_samples.is_empty() {
        return Err(String::from("audio file contains no complete frames"));
    }
    Ok(waveform_file_from_mono_samples(
        path,
        bytes,
        decoded.sample_rate,
        channels,
        mono_samples,
    ))
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
    let gpu_signal_samples = split_frequency_bands(&mono_samples, sample_rate);
    let gpu_signal_summary = Arc::new(
        radiant::runtime::GpuSignalSummary::from_interleaved_samples(
            &gpu_signal_samples,
            mono_samples.len(),
            BAND_COUNT,
        ),
    );
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

pub(super) fn load_wav_waveform_file(
    path: PathBuf,
    bytes: Arc<[u8]>,
) -> Result<WaveformFile, String> {
    let cursor = Cursor::new(bytes.as_ref());
    let mut reader =
        hound::WavReader::new(cursor).map_err(|err| format!("failed to open WAV: {err}"))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels).max(1);
    let samples = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|sample| {
                sample
                    .map(|value| value.clamp(-1.0, 1.0))
                    .map_err(|err| format!("failed to read float sample: {err}"))
            })
            .collect::<Result<Vec<_>, _>>()?,
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => {
            let max =
                ((1_i32 << (u32::from(spec.bits_per_sample).saturating_sub(1))) - 1).max(1) as f32;
            reader
                .samples::<i16>()
                .map(|sample| {
                    sample
                        .map(|value| (f32::from(value) / max).clamp(-1.0, 1.0))
                        .map_err(|err| format!("failed to read integer sample: {err}"))
                })
                .collect::<Result<Vec<_>, _>>()?
        }
        hound::SampleFormat::Int => {
            let max =
                ((1_i64 << (u32::from(spec.bits_per_sample).saturating_sub(1))) - 1).max(1) as f32;
            reader
                .samples::<i32>()
                .map(|sample| {
                    sample
                        .map(|value| ((value as f32) / max).clamp(-1.0, 1.0))
                        .map_err(|err| format!("failed to read integer sample: {err}"))
                })
                .collect::<Result<Vec<_>, _>>()?
        }
    };
    if samples.is_empty() {
        return Err(String::from("WAV contains no samples"));
    }

    let frames = samples.len() / channels;
    let mono_samples = downmix_to_mono(&samples, channels, frames);
    if mono_samples.is_empty() {
        return Err(String::from("WAV contains no complete frames"));
    }
    Ok(waveform_file_from_mono_samples(
        path,
        bytes,
        spec.sample_rate,
        channels,
        mono_samples,
    ))
}

pub(super) fn downmix_to_mono(samples: &[f32], channels: usize, frames: usize) -> Vec<f32> {
    let channels = channels.max(1);
    (0..frames)
        .map(|frame| {
            let start = frame * channels;
            let mut peak = 0.0_f32;
            for sample in samples[start..start + channels].iter().copied() {
                if sample.abs() > peak.abs() {
                    peak = sample;
                }
            }
            peak.clamp(-1.0, 1.0)
        })
        .collect()
}
