use radiant::runtime::GpuSignalGainPreview;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Arc,
};

use crate::native_app::waveform::audio_file::{
    WaveformFile, signal_summary::gpu_signal_summary_with_progress_and_cancel,
    visual_bands::split_frequency_bands_with_progress_and_cancel,
};
#[cfg(test)]
use crate::native_app::waveform::{SYNTHETIC_SAMPLE_RATE, SYNTHETIC_SECONDS};

#[cfg(test)]
pub(in crate::native_app) fn test_waveform_file_from_mono_samples(
    path: PathBuf,
    audio_bytes: Arc<[u8]>,
    samples: Vec<f32>,
) -> WaveformFile {
    waveform_file_from_mono_samples(path, audio_bytes, 48_000, 1, samples)
}

#[cfg(test)]
pub(in crate::native_app) fn test_file_backed_waveform_file_from_mono_samples(
    path: PathBuf,
    samples: Vec<f32>,
) -> WaveformFile {
    let mut file = waveform_file_from_mono_samples(path, Arc::from([]), 48_000, 1, samples);
    file.playback_samples = None;
    file.playback_cache_file = None;
    file
}

#[cfg(test)]
pub(in crate::native_app) fn test_decoded_waveform_file_from_mono_samples(
    path: PathBuf,
    samples: Vec<f32>,
) -> WaveformFile {
    let mut file =
        waveform_file_from_mono_samples(path, Arc::from([1_u8]), 48_000, 1, samples.clone());
    file.playback_samples = Some(Arc::from(samples));
    file.playback_cache_file = None;
    file
}

#[cfg(test)]
pub(in crate::native_app::waveform) fn synthetic_waveform_file() -> WaveformFile {
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
        Arc::from([0_u8]),
        SYNTHETIC_SAMPLE_RATE,
        1,
        samples,
    )
}

pub(in crate::native_app::waveform) fn empty_waveform_file() -> WaveformFile {
    waveform_file_from_mono_samples(PathBuf::new(), Arc::from([]), 0, 1, vec![0.0])
}

pub(in crate::native_app::waveform) fn waveform_file_from_mono_samples(
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

pub(in crate::native_app::waveform) fn waveform_file_from_mono_samples_with_progress(
    path: PathBuf,
    audio_bytes: Arc<[u8]>,
    sample_rate: u32,
    channels: usize,
    mono_samples: Vec<f32>,
    progress: &impl Fn(f32),
) -> WaveformFile {
    waveform_file_from_mono_samples_with_progress_and_cancel(
        path,
        audio_bytes,
        sample_rate,
        channels,
        mono_samples,
        progress,
        &|| false,
    )
    .expect("non-cancellable waveform construction cannot be cancelled")
}

pub(in crate::native_app::waveform) fn waveform_file_from_mono_samples_with_progress_and_cancel(
    path: PathBuf,
    audio_bytes: Arc<[u8]>,
    sample_rate: u32,
    channels: usize,
    mono_samples: Vec<f32>,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<WaveformFile, String> {
    let (gpu_signal_samples, visual_band_normalization) =
        split_frequency_bands_with_progress_and_cancel(
            &mono_samples,
            sample_rate,
            0.62,
            0.9,
            progress,
            cancelled,
        )?;
    let gpu_signal_summary = Arc::new(gpu_signal_summary_with_progress_and_cancel(
        &gpu_signal_samples,
        mono_samples.len(),
        0.9,
        0.99,
        progress,
        cancelled,
    )?);
    Ok(WaveformFile {
        path,
        content_revision: content_revision_for_audio_bytes(&audio_bytes),
        audio_bytes,
        playback_samples: None,
        playback_cache_file: None,
        sample_rate,
        channels,
        frames: mono_samples.len(),
        visual_band_normalization,
        gpu_signal_summary,
    })
}

pub(in crate::native_app::waveform) fn gain_preview_for_selection(
    selection: Option<wavecrate::selection::SelectionRange>,
) -> Option<GpuSignalGainPreview> {
    let selection = selection.filter(|selection| selection.has_edit_effects())?;
    Some(gain_preview(selection, selection.gain()))
}

pub(in crate::native_app::waveform) fn gain_preview_for_range_with_gain(
    selection: wavecrate::selection::SelectionRange,
    gain: f32,
) -> Option<GpuSignalGainPreview> {
    if !gain.is_finite() || gain <= 0.0 || (gain - 1.0).abs() <= f32::EPSILON {
        return None;
    }
    Some(gain_preview(selection, gain))
}

fn gain_preview(
    selection: wavecrate::selection::SelectionRange,
    gain: f32,
) -> GpuSignalGainPreview {
    let fade_in = selection.fade_in();
    let fade_out = selection.fade_out();
    GpuSignalGainPreview {
        start: selection.start(),
        end: selection.end(),
        gain,
        fade_in_length: fade_in.map(|fade| fade.length).unwrap_or(0.0),
        fade_in_curve: fade_in.map(|fade| fade.curve).unwrap_or(0.5),
        fade_in_mute: fade_in.map(|fade| fade.mute).unwrap_or(0.0),
        fade_in_outer_gain: fade_in.map(|fade| fade.outer_gain).unwrap_or(1.0),
        fade_out_length: fade_out.map(|fade| fade.length).unwrap_or(0.0),
        fade_out_curve: fade_out.map(|fade| fade.curve).unwrap_or(0.5),
        fade_out_mute: fade_out.map(|fade| fade.mute).unwrap_or(0.0),
        fade_out_outer_gain: fade_out.map(|fade| fade.outer_gain).unwrap_or(1.0),
    }
}

pub(in crate::native_app::waveform) fn content_revision_for_audio_bytes(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish().max(1)
}
