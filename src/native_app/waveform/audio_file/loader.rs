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
        wav_decode::load_wav_waveform_file_with_progress, waveform_cache,
    },
};

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
        false,
        false,
    )
}

fn load_waveform_file_with_progress_cancel_playback_ready_and_cache_policy(
    path: PathBuf,
    progress: impl Fn(f32),
    cancelled: impl Fn() -> bool,
    playback_ready: impl Fn(WaveformPlaybackReady),
    read_cache: bool,
    persist_cache: bool,
) -> Result<WaveformFile, String> {
    if cancelled() {
        return Err(String::from("cancelled"));
    }
    progress(0.0);
    if read_cache {
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
        let playback_samples = Arc::from(samples);
        playback_ready(WaveformPlaybackReady {
            path: path.to_path_buf(),
            audio_bytes: Arc::clone(bytes),
            playback_samples: Arc::clone(&playback_samples),
            sample_rate: file.sample_rate,
            channels: file.channels,
            frames: file.frames,
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
    use crate::native_app::waveform::audio_file::waveform_file_from_mono_samples;

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
}
