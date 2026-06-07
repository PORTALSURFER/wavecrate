use radiant::runtime::{GpuSignalGainPreview, GpuSignalSummary};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

#[cfg(test)]
use super::{SYNTHETIC_SAMPLE_RATE, SYNTHETIC_SECONDS};
use super::{WAVEFORM_HEIGHT, WAVEFORM_WIDTH};

mod downmix;
#[cfg(test)]
pub(super) use downmix::downmix_to_mono;
use downmix::downmix_to_mono_with_progress_and_cancel;

mod extraction;
pub(super) use extraction::{extract_wav_range_to_folder, extract_wav_range_to_sibling};

mod file_io;
use file_io::read_audio_file_with_progress;

mod progress;
pub(super) use progress::{cooperate_with_ui, report_phase_progress_throttled};

mod signal_summary;
use signal_summary::gpu_signal_summary_with_progress_and_cancel;

mod visual_bands;
#[cfg(test)]
pub(super) use visual_bands::split_frequency_bands;
pub(super) use visual_bands::split_frequency_bands_with_progress_and_cancel;

mod wav_decode;
use wav_decode::load_wav_waveform_file_with_progress;

mod waveform_cache;
pub(in crate::native_app) use waveform_cache::{
    cached_waveform_file_exists, cached_waveform_file_playback_ready_exists,
    flush_background_waveform_cache_stores_for_shutdown, load_cached_waveform_file_for_playback,
};
use waveform_cache::{load_cached_waveform_file, store_cached_waveform_file_in_background};
#[cfg(test)]
pub(in crate::native_app) fn store_cached_waveform_file_for_tests(file: &WaveformFile) {
    waveform_cache::store_cached_waveform_file(file);
}

#[cfg(test)]
pub(in crate::native_app) fn store_summary_only_cached_waveform_file_for_tests(
    file: &WaveformFile,
) {
    let mut file = file.clone();
    file.playback_samples = None;
    file.playback_cache_file = None;
    waveform_cache::store_cached_waveform_file(&file);
}

#[cfg(test)]
pub(in crate::native_app) fn test_waveform_file_from_mono_samples(
    path: PathBuf,
    audio_bytes: Arc<[u8]>,
    samples: Vec<f32>,
) -> WaveformFile {
    waveform_file_from_mono_samples(path, audio_bytes, 48_000, 1, samples)
}

#[derive(Clone, Debug)]
pub(in crate::native_app) struct WaveformFile {
    pub(super) path: PathBuf,
    pub(super) audio_bytes: Arc<[u8]>,
    pub(super) playback_samples: Option<Arc<[f32]>>,
    pub(super) playback_cache_file: Option<PersistedPlaybackCacheFile>,
    pub(super) content_revision: u64,
    pub(super) sample_rate: u32,
    pub(super) channels: usize,
    pub(super) frames: usize,
    pub(super) gpu_signal_summary: Arc<GpuSignalSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct PersistedPlaybackCacheFile {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) sample_count: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::native_app) struct WaveformPlaybackReady {
    pub(in crate::native_app) path: PathBuf,
    pub(in crate::native_app) audio_bytes: Arc<[u8]>,
    pub(in crate::native_app) playback_samples: Arc<[f32]>,
    pub(in crate::native_app) sample_rate: u32,
    pub(in crate::native_app) channels: usize,
    pub(in crate::native_app) frames: usize,
}

impl PersistedPlaybackCacheFile {
    pub(in crate::native_app) fn new(path: PathBuf, sample_count: u64) -> Option<Self> {
        (sample_count > 0).then_some(Self { path, sample_count })
    }
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
    load_waveform_file_with_progress_and_cancel(path, progress, || false)
}

pub(super) fn load_waveform_file_with_progress_and_cancel(
    path: PathBuf,
    progress: impl Fn(f32),
    cancelled: impl Fn() -> bool,
) -> Result<WaveformFile, String> {
    load_waveform_file_with_progress_cancel_and_playback_ready(path, progress, cancelled, |_| {})
}

pub(super) fn load_waveform_file_with_progress_cancel_and_playback_ready(
    path: PathBuf,
    progress: impl Fn(f32),
    cancelled: impl Fn() -> bool,
    playback_ready: impl Fn(WaveformPlaybackReady),
) -> Result<WaveformFile, String> {
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    progress(0.0);
    let cache_started_at = Instant::now();
    if let Some(file) = load_cached_waveform_file_for_playback(path.clone()) {
        log_audio_load_timing(
            "browser.audio_file.load.playback_cache",
            &path,
            cache_started_at.elapsed(),
        );
        progress(0.99);
        return Ok(file);
    }
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    let read_started_at = Instant::now();
    let bytes = read_audio_file_with_progress(&path, 0.0, 0.08, &progress, &cancelled)?;
    log_audio_load_timing(
        "browser.audio_file.load.read_bytes",
        &path,
        read_started_at.elapsed(),
    );
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    let summary_cache_started_at = Instant::now();
    if let Some(mut file) = load_cached_waveform_file(path.clone(), Arc::clone(&bytes)) {
        log_audio_load_timing(
            "browser.audio_file.load.summary_cache",
            &path,
            summary_cache_started_at.elapsed(),
        );
        if file.playback_samples.is_none()
            && is_wav_path(&path)
            && let Ok(samples) = wav_decode::read_wav_playback_samples(&bytes)
        {
            let playback_samples = Arc::from(samples);
            playback_ready(WaveformPlaybackReady {
                path: path.clone(),
                audio_bytes: Arc::clone(&bytes),
                playback_samples: Arc::clone(&playback_samples),
                sample_rate: file.sample_rate,
                channels: file.channels,
                frames: file.frames,
            });
            file.playback_samples = Some(playback_samples);
            store_cached_waveform_file_in_background(&file);
        }
        progress(0.99);
        return Ok(file);
    }
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    let wav_decode_started_at = Instant::now();
    if is_wav_path(&path)
        && let Ok(file) = load_wav_waveform_file_with_progress(
            path.clone(),
            Arc::clone(&bytes),
            &progress,
            &cancelled,
            &playback_ready,
        )
    {
        if cancelled() {
            return Err(String::from("cancelled"));
        }
        log_audio_load_timing(
            "browser.audio_file.load.wav_decode",
            &path,
            wav_decode_started_at.elapsed(),
        );
        store_cached_waveform_file_in_background(&file);
        return Ok(file);
    }
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    let decode_started_at = Instant::now();
    let decoded =
        wavecrate::waveform::WaveformRenderer::new(WAVEFORM_WIDTH as u32, WAVEFORM_HEIGHT as u32)
            .decode_from_bytes(&bytes)
            .map_err(|err| format!("failed to decode audio file: {err}"))?;
    log_audio_load_timing(
        "browser.audio_file.load.decode_bytes",
        &path,
        decode_started_at.elapsed(),
    );
    progress(0.48);
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    let channels = decoded.channel_count();
    let frames = decoded.frame_count();
    let downmix_started_at = Instant::now();
    let mono_samples = if decoded.samples.is_empty() {
        decoded.analysis_samples.iter().copied().collect::<Vec<_>>()
    } else {
        downmix_to_mono_with_progress_and_cancel(
            &decoded.samples,
            channels,
            frames,
            0.48,
            0.62,
            &progress,
            &cancelled,
        )?
    };
    log_audio_load_timing(
        "browser.audio_file.load.downmix",
        &path,
        downmix_started_at.elapsed(),
    );
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    if mono_samples.is_empty() {
        return Err(String::from("audio file contains no complete frames"));
    }
    let waveform_started_at = Instant::now();
    let file = waveform_file_from_mono_samples_with_progress_and_cancel(
        path,
        bytes,
        decoded.sample_rate,
        channels,
        mono_samples,
        &progress,
        &cancelled,
    )?;
    log_audio_load_timing(
        "browser.audio_file.load.waveform_summary",
        &file.path,
        waveform_started_at.elapsed(),
    );
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    store_cached_waveform_file_in_background(&file);
    Ok(file)
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
        Arc::from([0_u8]),
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

fn waveform_file_from_mono_samples_with_progress_and_cancel(
    path: PathBuf,
    audio_bytes: Arc<[u8]>,
    sample_rate: u32,
    channels: usize,
    mono_samples: Vec<f32>,
    progress: &impl Fn(f32),
    cancelled: &impl Fn() -> bool,
) -> Result<WaveformFile, String> {
    let gpu_signal_samples = split_frequency_bands_with_progress_and_cancel(
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
        gpu_signal_summary,
    })
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

pub(super) fn log_audio_load_timing(
    event: &'static str,
    path: &std::path::Path,
    elapsed: Duration,
) {
    tracing::info!(
        target: "wavecrate::debug::sample_load",
        event,
        elapsed_ms = elapsed.as_secs_f64() * 1000.0,
        path = %path.display(),
        "Audio file load timing"
    );
}
