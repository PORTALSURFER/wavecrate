use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use crate::native_app::waveform::{
    WAVEFORM_HEIGHT, WAVEFORM_WIDTH,
    audio_file::{
        WaveformFile, WaveformPlaybackReady,
        construction::waveform_file_from_mono_samples_with_progress_and_cancel,
        diagnostics::log_audio_load_timing, downmix::downmix_to_mono_with_progress_and_cancel,
        file_io::read_audio_file_with_progress, wav_decode,
        wav_decode::load_wav_waveform_file_with_progress,
        wav_summary::load_wav_waveform_summary_from_path_with_progress, waveform_cache,
    },
};

#[cfg(test)]
pub(in crate::native_app) const FILE_BACKED_WAV_DECODE_MIN_BYTES: u64 = 1024;
#[cfg(not(test))]
pub(in crate::native_app) const FILE_BACKED_WAV_DECODE_MIN_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug)]
pub(in crate::native_app) struct FileBackedWavPlaybackDescriptor {
    pub path: PathBuf,
    pub duration: f32,
    pub sample_rate: u32,
    pub channels: usize,
    #[cfg(test)]
    pub frames: usize,
}

#[cfg(test)]
pub(in crate::native_app::waveform) fn load_waveform_file(
    path: PathBuf,
) -> Result<WaveformFile, String> {
    load_waveform_file_with_progress(path, |_| {})
}

#[cfg(test)]
fn load_waveform_file_with_progress(
    path: PathBuf,
    progress: impl Fn(f32),
) -> Result<WaveformFile, String> {
    load_waveform_file_with_progress_and_cancel(path, progress, || false)
}

pub(in crate::native_app::waveform) fn load_waveform_file_with_progress_and_cancel(
    path: PathBuf,
    progress: impl Fn(f32),
    cancelled: impl Fn() -> bool,
) -> Result<WaveformFile, String> {
    load_waveform_file_with_progress_cancel_and_playback_ready(path, progress, cancelled, |_| {})
}

pub(in crate::native_app) fn ensure_persisted_playback_summary(
    path: PathBuf,
    cancel: &std::sync::atomic::AtomicBool,
) -> Result<(), String> {
    use std::sync::atomic::Ordering;

    if waveform_cache::cached_waveform_file_audition_ready_exists(&path) {
        return Ok(());
    }
    let file = load_waveform_file_with_progress_cancel_playback_ready_and_cache_policy(
        path.clone(),
        |_| {},
        || cancel.load(Ordering::Acquire),
        |_| {},
        true,
        false,
        PlaybackReadyCachePolicy::Allow,
        FileBackedWavPolicy::AllowSummary,
    )?;
    if cancel.load(Ordering::Acquire) {
        return Err(String::from("waveform summary cancelled"));
    }
    waveform_cache::persist_cached_waveform_file(&file)?;
    if waveform_cache::cached_waveform_file_audition_ready_exists(&path) {
        Ok(())
    } else {
        Err(format!(
            "waveform cache did not publish an audition-ready summary: {}",
            path.display()
        ))
    }
}

pub(in crate::native_app::waveform) fn load_waveform_file_with_progress_cancel_and_playback_ready(
    path: PathBuf,
    progress: impl Fn(f32),
    cancelled: impl Fn() -> bool,
    playback_ready: impl Fn(WaveformPlaybackReady),
) -> Result<WaveformFile, String> {
    load_waveform_file_with_progress_cancel_playback_ready_and_cache_policy(
        path,
        progress,
        cancelled,
        playback_ready,
        true,
        true,
        PlaybackReadyCachePolicy::Allow,
        FileBackedWavPolicy::AllowSummary,
    )
}

pub(in crate::native_app::waveform) fn load_waveform_file_for_foreground_audition(
    path: PathBuf,
    progress: impl Fn(f32),
    cancelled: impl Fn() -> bool,
    playback_ready: impl Fn(WaveformPlaybackReady),
) -> Result<WaveformFile, String> {
    load_waveform_file_with_progress_cancel_playback_ready_and_cache_policy(
        path,
        progress,
        cancelled,
        playback_ready,
        true,
        true,
        PlaybackReadyCachePolicy::Allow,
        FileBackedWavPolicy::AllowSummary,
    )
}

pub(in crate::native_app::waveform) fn load_waveform_file_for_instant_audition_display(
    path: PathBuf,
    progress: impl Fn(f32),
    cancelled: impl Fn() -> bool,
) -> Result<WaveformFile, String> {
    load_waveform_file_with_progress_cancel_playback_ready_and_cache_policy(
        path,
        progress,
        cancelled,
        |_| {},
        true,
        true,
        PlaybackReadyCachePolicy::Skip,
        FileBackedWavPolicy::AllowSummary,
    )
}

pub(in crate::native_app::waveform) fn load_waveform_file_for_looped_foreground_audition(
    path: PathBuf,
    progress: impl Fn(f32),
    cancelled: impl Fn() -> bool,
    playback_ready: impl Fn(WaveformPlaybackReady),
) -> Result<WaveformFile, String> {
    load_waveform_file_with_progress_cancel_playback_ready_and_cache_policy(
        path,
        progress,
        cancelled,
        playback_ready,
        true,
        true,
        PlaybackReadyCachePolicy::Allow,
        FileBackedWavPolicy::RequireDecodedPlayback,
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PlaybackReadyCachePolicy {
    Allow,
    Skip,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FileBackedWavPolicy {
    AllowSummary,
    RequireDecodedPlayback,
}

fn load_waveform_file_with_progress_cancel_playback_ready_and_cache_policy(
    path: PathBuf,
    progress: impl Fn(f32),
    cancelled: impl Fn() -> bool,
    playback_ready: impl Fn(WaveformPlaybackReady),
    read_cache: bool,
    persist_cache: bool,
    playback_ready_cache_policy: PlaybackReadyCachePolicy,
    file_backed_wav_policy: FileBackedWavPolicy,
) -> Result<WaveformFile, String> {
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    progress(0.0);
    let allow_file_backed_wav_summary =
        matches!(file_backed_wav_policy, FileBackedWavPolicy::AllowSummary);
    let prefer_file_backed_wav_summary =
        allow_file_backed_wav_summary && should_use_file_backed_wav_decode(&path);
    let skip_playback_ready_cache =
        matches!(playback_ready_cache_policy, PlaybackReadyCachePolicy::Skip);
    if read_cache
        && (prefer_file_backed_wav_summary || skip_playback_ready_cache)
        && let Some(file) = waveform_cache::load_cached_waveform_file_summary(path.clone())
    {
        progress(0.99);
        return Ok(file);
    }
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    if prefer_file_backed_wav_summary {
        let stream_started_at = Instant::now();
        let file =
            load_wav_waveform_summary_from_path_with_progress(path.clone(), &progress, &cancelled)?;
        log_audio_load_timing(
            "browser.audio_file.load.file_backed_wav_summary",
            &path,
            stream_started_at.elapsed(),
        );
        if persist_cache {
            waveform_cache::store_cached_waveform_file_in_background(&file);
        }
        return Ok(file);
    }
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    if read_cache && matches!(playback_ready_cache_policy, PlaybackReadyCachePolicy::Allow) {
        let cache_started_at = Instant::now();
        if let Some(file) = waveform_cache::load_cached_waveform_file_for_playback(path.clone()) {
            log_audio_load_timing(
                "browser.audio_file.load.playback_cache",
                &path,
                cache_started_at.elapsed(),
            );
            progress(0.99);
            return Ok(file);
        }
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
    if read_cache {
        let summary_cache_started_at = Instant::now();
        if let Some(mut file) =
            waveform_cache::load_cached_waveform_file(path.clone(), Arc::clone(&bytes))
        {
            log_audio_load_timing(
                "browser.audio_file.load.summary_cache",
                &path,
                summary_cache_started_at.elapsed(),
            );
            complete_wav_playback_ready_from_summary_cache(
                &mut file,
                &path,
                &bytes,
                &playback_ready,
                persist_cache,
            );
            progress(0.99);
            return Ok(file);
        }
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
        if persist_cache {
            waveform_cache::store_cached_waveform_file_in_background(&file);
        }
        return Ok(file);
    }
    load_with_fallback_decoder(path, bytes, progress, cancelled, persist_cache)
}

fn complete_wav_playback_ready_from_summary_cache(
    file: &mut WaveformFile,
    path: &Path,
    bytes: &Arc<[u8]>,
    playback_ready: &impl Fn(WaveformPlaybackReady),
    persist_cache: bool,
) {
    if !summary_cache_can_attempt_wav_playback_ready(file, path) {
        return;
    }
    if let Ok(samples) = wav_decode::read_wav_playback_samples(bytes) {
        let source_modified = path
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok());
        let playback_samples = Arc::from(samples);
        playback_ready(WaveformPlaybackReady {
            path: path.to_path_buf(),
            audio_bytes: Arc::clone(bytes),
            playback_samples: Arc::clone(&playback_samples),
            sample_rate: file.sample_rate,
            channels: file.channels,
            frames: file.frames,
            source_len: bytes.len() as u64,
            source_modified,
        });
        file.playback_samples = Some(playback_samples);
        if persist_cache {
            waveform_cache::store_cached_waveform_file_in_background(file);
        }
    }
}

fn summary_cache_can_attempt_wav_playback_ready(file: &WaveformFile, path: &Path) -> bool {
    file.playback_samples.is_none() && is_wav_path(path)
}

pub(in crate::native_app) fn should_use_file_backed_wav_decode(path: &Path) -> bool {
    is_wav_path(path)
        && path
            .metadata()
            .is_ok_and(|metadata| metadata.len() > FILE_BACKED_WAV_DECODE_MIN_BYTES)
}

pub(in crate::native_app) fn should_use_file_backed_wav_decode_for_entry(
    extension: &str,
    size_bytes: u64,
) -> bool {
    extension.eq_ignore_ascii_case("wav") && size_bytes > FILE_BACKED_WAV_DECODE_MIN_BYTES
}

pub(in crate::native_app) fn file_backed_wav_playback_descriptor(
    path: &Path,
) -> Option<FileBackedWavPlaybackDescriptor> {
    if !should_use_file_backed_wav_decode(path) {
        return None;
    }
    let reader = hound::WavReader::open(path).ok()?;
    let spec = reader.spec();
    let sample_rate = spec.sample_rate.max(1);
    let channels = usize::from(spec.channels).max(1);
    let frames = reader.duration() as usize;
    if frames == 0 {
        return None;
    }
    Some(FileBackedWavPlaybackDescriptor {
        path: path.to_path_buf(),
        duration: frames as f32 / sample_rate as f32,
        sample_rate,
        channels,
        #[cfg(test)]
        frames,
    })
}

fn load_with_fallback_decoder(
    path: PathBuf,
    bytes: Arc<[u8]>,
    progress: impl Fn(f32),
    cancelled: impl Fn() -> bool,
    persist_cache: bool,
) -> Result<WaveformFile, String> {
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
    let playback_samples = (!decoded.samples.is_empty()).then(|| Arc::clone(&decoded.samples));
    let mut file = waveform_file_from_mono_samples_with_progress_and_cancel(
        path,
        bytes,
        decoded.sample_rate,
        channels,
        mono_samples,
        &progress,
        &cancelled,
    )?;
    file.playback_samples = playback_samples;
    log_audio_load_timing(
        "browser.audio_file.load.waveform_summary",
        &file.path,
        waveform_started_at.elapsed(),
    );
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    if persist_cache {
        waveform_cache::store_cached_waveform_file_in_background(&file);
    }
    Ok(file)
}

pub(in crate::native_app::waveform) fn is_wav_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_app::waveform::{BAND_COUNT, audio_file::waveform_file_from_mono_samples};

    #[test]
    fn summary_cache_playback_ready_policy_requires_wav_without_samples() {
        let bytes = Arc::<[u8]>::from([1, 2, 3, 4]);
        let mut file = waveform_file_from_mono_samples(
            PathBuf::from("kick.wav"),
            Arc::clone(&bytes),
            48_000,
            1,
            vec![0.0, 0.25],
        );

        assert!(summary_cache_can_attempt_wav_playback_ready(
            &file,
            Path::new("kick.wav")
        ));
        assert!(!summary_cache_can_attempt_wav_playback_ready(
            &file,
            Path::new("kick.aif")
        ));

        file.playback_samples = Some(Arc::from([0.0_f32, 0.25]));
        assert!(!summary_cache_can_attempt_wav_playback_ready(
            &file,
            Path::new("kick.wav")
        ));
    }

    #[test]
    fn instant_audition_display_uses_file_backed_summary_for_large_wav() {
        let source_root = tempfile::tempdir().expect("source root");
        let sample_path = source_root.path().join("large-instant-display.wav");
        write_test_wav_i16(&sample_path, 700);

        let file =
            load_waveform_file_for_instant_audition_display(sample_path.clone(), |_| {}, || false)
                .expect("summary display state");

        assert_eq!(file.path, sample_path);
        assert!(file.audio_bytes.is_empty());
        assert!(file.playback_samples.is_none());
        assert!(file.playback_cache_file.is_none());
        assert_eq!(file.sample_rate, 48_000);
        assert_eq!(file.channels, 1);
        assert_eq!(file.frames, 700);
        assert_eq!(file.gpu_signal_summary.frames, 700);
        assert_eq!(file.gpu_signal_summary.band_count, BAND_COUNT);
        assert!(file.gpu_signal_summary.levels.len() > 1);
        assert!(
            signal_summary_peak(&file) > 0.0,
            "large display summary should retain visible signal data"
        );
    }

    fn signal_summary_peak(file: &WaveformFile) -> f32 {
        file.gpu_signal_summary
            .levels
            .iter()
            .flat_map(|level| level.buckets.iter())
            .fold(0.0_f32, |peak, bucket| {
                peak.max(bucket.min.abs()).max(bucket.max.abs())
            })
    }

    fn write_test_wav_i16(path: &Path, frames: usize) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
        for frame in 0..frames {
            writer
                .write_sample::<i16>((frame as i16).wrapping_mul(97))
                .expect("write sample");
        }
        writer.finalize().expect("finalize wav");
    }
}
