use radiant::runtime::{GpuSignalGainPreview, GpuSignalSummary};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Arc,
};

#[cfg(test)]
use super::{SYNTHETIC_SAMPLE_RATE, SYNTHETIC_SECONDS};
use super::{WAVEFORM_HEIGHT, WAVEFORM_WIDTH};

mod downmix;
#[cfg(test)]
pub(super) use downmix::downmix_to_mono;
use downmix::downmix_to_mono_with_progress;

mod extraction;
pub(super) use extraction::{extract_wav_range_to_folder, extract_wav_range_to_sibling};

mod file_io;
use file_io::read_audio_file_with_progress;

mod progress;
pub(super) use progress::report_phase_progress_throttled;

mod signal_summary;
use signal_summary::gpu_signal_summary_with_progress;

mod visual_bands;
#[cfg(test)]
pub(super) use visual_bands::split_frequency_bands;
pub(super) use visual_bands::split_frequency_bands_with_progress;

mod wav_decode;
use wav_decode::load_wav_waveform_file_with_progress;

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
