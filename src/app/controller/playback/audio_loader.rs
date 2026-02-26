use super::*;
use crate::app::controller::playback::audio_cache::FileMetadata;
use crate::waveform::{DecodedWaveform, WaveformRenderer};
use std::{
    fs,
    path::{Component, Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{Receiver, Sender},
    },
    thread,
    time::Duration,
};

const AUDIO_LOADER_POLL_INTERVAL: Duration = Duration::from_millis(200);

pub(crate) struct AudioLoadJob {
    pub request_id: u64,
    pub source_id: SourceId,
    pub root: PathBuf,
    pub relative_path: PathBuf,
    pub stretch_ratio: Option<f64>,
}

#[derive(Debug)]
pub(crate) struct AudioLoadOutcome {
    pub decoded: Arc<DecodedWaveform>,
    pub bytes: Arc<[u8]>,
    pub metadata: FileMetadata,
    pub transients: Arc<[f32]>,
    pub stretched: bool,
}

#[derive(Debug)]
pub(crate) enum AudioLoadError {
    Missing(String),
    Failed(String),
}

#[derive(Debug)]
pub(crate) struct AudioLoadResult {
    pub request_id: u64,
    pub source_id: SourceId,
    pub relative_path: PathBuf,
    pub result: Result<AudioLoadOutcome, AudioLoadError>,
}

/// Join handle and shutdown signal for the audio loader thread.
pub(crate) struct AudioLoaderHandle {
    shutdown: Arc<AtomicBool>,
    latest_request_id: Arc<AtomicU64>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl AudioLoaderHandle {
    /// Publish the most recent queued request id so stale decode work can abort early.
    pub(crate) fn publish_latest_request_id(&self, request_id: u64) {
        self.latest_request_id.store(request_id, Ordering::Relaxed);
    }

    /// Signal the loader thread to exit and wait for it to finish.
    pub(crate) fn shutdown(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Spawn the audio loader worker and return its job channel plus shutdown handle.
pub(crate) fn spawn_audio_loader(
    renderer: WaveformRenderer,
) -> (
    Sender<AudioLoadJob>,
    Receiver<AudioLoadResult>,
    AudioLoaderHandle,
) {
    let (tx, rx) = std::sync::mpsc::channel::<AudioLoadJob>();
    let (result_tx, result_rx) = std::sync::mpsc::channel::<AudioLoadResult>();
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_worker = Arc::clone(&shutdown);
    let latest_request_id = Arc::new(AtomicU64::new(0));
    let latest_request_id_worker = Arc::clone(&latest_request_id);
    let handle = thread::spawn(move || {
        while !shutdown_worker.load(Ordering::Relaxed) {
            match rx.recv_timeout(AUDIO_LOADER_POLL_INTERVAL) {
                Ok(job) => {
                    let job = drain_to_latest_job(job, &rx);
                    let outcome = load_audio(&renderer, &job, &latest_request_id_worker);
                    if !matches!(outcome, AudioLoadExecution::Completed(_)) {
                        continue;
                    }
                    if is_stale_request(job.request_id, &latest_request_id_worker) {
                        continue;
                    }
                    let AudioLoadExecution::Completed(result) = outcome else {
                        continue;
                    };
                    let _ = result_tx.send(AudioLoadResult {
                        request_id: job.request_id,
                        source_id: job.source_id.clone(),
                        relative_path: job.relative_path.clone(),
                        result,
                    });
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });
    (
        tx,
        result_rx,
        AudioLoaderHandle {
            shutdown,
            latest_request_id,
            join_handle: Some(handle),
        },
    )
}

enum AudioLoadExecution {
    Completed(Result<AudioLoadOutcome, AudioLoadError>),
    DroppedStale,
}

fn drain_to_latest_job(mut latest_job: AudioLoadJob, rx: &Receiver<AudioLoadJob>) -> AudioLoadJob {
    while let Ok(next_job) = rx.try_recv() {
        latest_job = next_job;
    }
    latest_job
}

fn is_stale_request(request_id: u64, latest_request_id: &AtomicU64) -> bool {
    let latest = latest_request_id.load(Ordering::Relaxed);
    latest != 0 && latest != request_id
}

fn load_audio(
    renderer: &WaveformRenderer,
    job: &AudioLoadJob,
    latest_request_id: &AtomicU64,
) -> AudioLoadExecution {
    if is_stale_request(job.request_id, latest_request_id) {
        return AudioLoadExecution::DroppedStale;
    }
    let result = load_audio_inner(renderer, job, latest_request_id);
    match result {
        Ok(Some(outcome)) => AudioLoadExecution::Completed(Ok(outcome)),
        Ok(None) => AudioLoadExecution::DroppedStale,
        Err(err) => AudioLoadExecution::Completed(Err(err)),
    }
}

fn load_audio_inner(
    renderer: &WaveformRenderer,
    job: &AudioLoadJob,
    latest_request_id: &AtomicU64,
) -> Result<Option<AudioLoadOutcome>, AudioLoadError> {
    ensure_safe_relative_path(&job.relative_path)?;
    if is_stale_request(job.request_id, latest_request_id) {
        return Ok(None);
    }
    let full_path = job.root.join(&job.relative_path);
    let metadata = fs::metadata(&full_path).map_err(|err| {
        let missing = err.kind() == std::io::ErrorKind::NotFound;
        if missing {
            AudioLoadError::Missing(format!("File missing: {} ({err})", full_path.display()))
        } else {
            AudioLoadError::Failed(format!(
                "Failed to read metadata for {}: {err}",
                full_path.display()
            ))
        }
    })?;
    let bytes = fs::read(&full_path).map_err(|err| {
        let missing = err.kind() == std::io::ErrorKind::NotFound;
        if missing {
            AudioLoadError::Missing(format!("File missing: {} ({err})", full_path.display()))
        } else {
            AudioLoadError::Failed(format!("Failed to read {}: {err}", full_path.display()))
        }
    })?;
    if is_stale_request(job.request_id, latest_request_id) {
        return Ok(None);
    }
    let bytes = crate::wav_sanitize::sanitize_wav_bytes(bytes);
    let modified_ns = metadata
        .modified()
        .map_err(|err| {
            AudioLoadError::Failed(format!(
                "Missing modified time for {}: {err}",
                full_path.display()
            ))
        })?
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map_err(|_| {
            AudioLoadError::Failed(format!(
                "File modified time is before epoch: {}",
                full_path.display()
            ))
        })?
        .as_nanos() as i64;
    if is_stale_request(job.request_id, latest_request_id) {
        return Ok(None);
    }
    let mut decoded = Arc::new(
        renderer
            .decode_from_bytes(&bytes)
            .map_err(|err| AudioLoadError::Failed(err.to_string()))?,
    );
    if is_stale_request(job.request_id, latest_request_id) {
        return Ok(None);
    }

    let mut stretched = false;
    let mut final_bytes: Arc<[u8]> = bytes.into();

    if let Some(ratio) = job.stretch_ratio {
        if is_stale_request(job.request_id, latest_request_id) {
            return Ok(None);
        }
        let wsola = crate::audio::Wsola::new(decoded.sample_rate);
        let stretched_samples = wsola.stretch(&decoded.samples, decoded.channel_count(), ratio);
        if is_stale_request(job.request_id, latest_request_id) {
            return Ok(None);
        }
        match crate::app::controller::playback::audio_samples::wav_bytes_from_samples(
            &stretched_samples,
            decoded.sample_rate,
            decoded.channels,
        ) {
            Ok(b) => {
                final_bytes = b.into();
                stretched = true;
                // Decode the stretched bytes to get the correct duration and cache token
                if let Ok(d) = renderer.decode_from_bytes(&final_bytes) {
                    decoded = Arc::new(d);
                }
            }
            Err(err) => {
                tracing::warn!("Failed to stretch audio in background: {err}");
            }
        }
    }

    if is_stale_request(job.request_id, latest_request_id) {
        return Ok(None);
    }
    let transients: Arc<[f32]> = crate::waveform::transients::detect_transients(
        decoded.as_ref(),
        crate::app::controller::library::wavs::waveform_rendering::DEFAULT_TRANSIENT_SENSITIVITY,
    )
    .into();
    if is_stale_request(job.request_id, latest_request_id) {
        return Ok(None);
    }

    Ok(Some(AudioLoadOutcome {
        decoded,
        bytes: final_bytes,
        metadata: FileMetadata {
            file_size: metadata.len(),
            modified_ns,
        },
        transients,
        stretched,
    }))
}

fn ensure_safe_relative_path(path: &Path) -> Result<(), AudioLoadError> {
    let mut saw_component = false;
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(_) => {
                saw_component = true;
            }
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(AudioLoadError::Failed(format!(
                    "Invalid relative path: {}",
                    path.display()
                )));
            }
        }
    }
    if !saw_component {
        return Err(AudioLoadError::Failed(format!(
            "Invalid relative path: {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{AudioLoadJob, drain_to_latest_job, ensure_safe_relative_path, is_stale_request};
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::atomic::AtomicU64;

    fn test_job(request_id: u64, relative_path: &str) -> AudioLoadJob {
        AudioLoadJob {
            request_id,
            source_id: crate::sample_sources::SourceId::from_string("source"),
            root: PathBuf::from("/tmp"),
            relative_path: PathBuf::from(relative_path),
            stretch_ratio: None,
        }
    }

    #[test]
    fn ensure_safe_relative_path_rejects_parent_dir() {
        let err = ensure_safe_relative_path(Path::new("../escape.wav")).unwrap_err();
        assert!(matches!(err, super::AudioLoadError::Failed(_)));
    }

    #[test]
    fn ensure_safe_relative_path_accepts_normal_relative_paths() {
        ensure_safe_relative_path(Path::new("folder/./file.wav")).unwrap();
    }

    #[test]
    fn drain_to_latest_job_keeps_most_recent_request() {
        let (tx, rx) = std::sync::mpsc::channel::<AudioLoadJob>();
        tx.send(test_job(2, "two.wav")).unwrap();
        tx.send(test_job(3, "three.wav")).unwrap();
        let drained = drain_to_latest_job(test_job(1, "one.wav"), &rx);
        assert_eq!(drained.request_id, 3);
        assert_eq!(drained.relative_path, Path::new("three.wav"));
    }

    #[test]
    fn stale_request_detection_ignores_zero_and_matches_latest_only() {
        let latest = AtomicU64::new(0);
        assert!(!is_stale_request(1, &latest));
        latest.store(5, std::sync::atomic::Ordering::Relaxed);
        assert!(is_stale_request(4, &latest));
        assert!(!is_stale_request(5, &latest));
    }
}
