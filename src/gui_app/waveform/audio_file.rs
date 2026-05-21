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

pub(super) fn split_frequency_bands(samples: &[f32], sample_rate: u32) -> Arc<[f32]> {
    if samples.is_empty() {
        return Arc::from([]);
    }
    let alpha_low = lowpass_alpha(sample_rate, 150.0);
    let alpha_mid = lowpass_alpha(sample_rate, 2_200.0);
    let mut low = 0.0_f32;
    let mut mid_low = 0.0_f32;
    let mut low_envelope = 0.0_f32;
    let mut mid_envelope = 0.0_f32;
    let mut high_envelope = 0.0_f32;
    let low_release = envelope_release_alpha(sample_rate, 12.0);
    let mid_release = envelope_release_alpha(sample_rate, 5.5);
    let high_release = envelope_release_alpha(sample_rate, 2.2);
    let mut bands = Vec::with_capacity(samples.len().saturating_mul(BAND_COUNT));
    for sample in samples {
        let sample = sample.clamp(-1.0, 1.0);
        low += alpha_low * (sample - low);
        mid_low += alpha_mid * (sample - mid_low);
        let low_band = (low * 1.08).clamp(-1.0, 1.0);
        let mid_band = ((mid_low - low) * 1.45).clamp(-1.0, 1.0);
        let high_band = ((sample - mid_low) * 2.15).clamp(-1.0, 1.0);
        low_envelope = follow_visual_envelope(low_envelope, low_band.abs(), low_release);
        mid_envelope = follow_visual_envelope(mid_envelope, mid_band.abs(), mid_release);
        high_envelope = follow_visual_envelope(high_envelope, high_band.abs(), high_release);
        bands.push(low_envelope);
        bands.push(mid_envelope);
        bands.push(high_envelope);
        bands.push(sample);
    }
    normalize_visual_band_peaks(&mut bands);
    bands.into()
}

pub(super) fn normalize_visual_band_peaks(bands: &mut [f32]) {
    let raw_peak = bands
        .chunks_exact(BAND_COUNT)
        .map(|frame| frame[3].abs())
        .fold(0.0_f32, f32::max);
    if raw_peak <= 0.000_01 || !raw_peak.is_finite() {
        return;
    }
    let peaks = [
        visual_band_peak(bands, 0),
        visual_band_peak(bands, 1),
        visual_band_peak(bands, 2),
    ];
    let spectral_total = peaks.iter().copied().sum::<f32>().max(0.000_01);
    let targets = [raw_peak * 0.98, raw_peak * 0.74, raw_peak * 0.34];
    let boost_thresholds = [raw_peak * 0.08, raw_peak * 0.05, raw_peak * 0.035];
    let max_gains = [2.6_f32, 2.8, 2.4];
    for band in 0..3 {
        let peak = peaks[band];
        if peak <= 0.000_01 || !peak.is_finite() {
            continue;
        }
        let energy_share = peak / spectral_total;
        let target = targets[band] * smoothstep_scalar(0.12, 0.55, energy_share);
        let max_gain = if peak < boost_thresholds[band] {
            1.0
        } else {
            max_gains[band]
        };
        let gain = (target / peak).clamp(0.25, max_gain);
        for frame in bands.chunks_exact_mut(BAND_COUNT) {
            frame[band] = (frame[band] * gain).clamp(-1.0, 1.0);
        }
    }
}

pub(super) fn visual_band_peak(bands: &[f32], band: usize) -> f32 {
    bands
        .chunks_exact(BAND_COUNT)
        .map(|frame| frame[band].abs())
        .fold(0.0_f32, f32::max)
}

pub(super) fn smoothstep_scalar(edge0: f32, edge1: f32, value: f32) -> f32 {
    let range = (edge1 - edge0).abs().max(0.000_01);
    let t = ((value - edge0) / range).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub(super) fn follow_visual_envelope(previous: f32, value: f32, release_alpha: f32) -> f32 {
    if value >= previous {
        value
    } else {
        previous + release_alpha * (value - previous)
    }
}

pub(super) fn envelope_release_alpha(sample_rate: u32, release_ms: f32) -> f32 {
    let samples = sample_rate.max(1) as f32 * (release_ms.max(0.1) / 1_000.0);
    (1.0 - (-1.0 / samples).exp()).clamp(0.0, 1.0)
}

pub(super) fn lowpass_alpha(sample_rate: u32, cutoff_hz: f32) -> f32 {
    (1.0 - (-std::f32::consts::TAU * cutoff_hz / sample_rate.max(1) as f32).exp()).clamp(0.0, 1.0)
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
