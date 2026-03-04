use super::*;
use crate::app::controller::playback::audio_cache::FileMetadata;
use crate::waveform::{DecodedWaveform, WaveformRenderer};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{Receiver, Sender},
    },
    thread,
    time::Duration,
};

mod stages;
mod telemetry;

#[cfg(test)]
mod tests;

use self::stages::{build_transient_result, load_audio_inner};
use self::telemetry::{
    StaleDropStage, audio_loader_telemetry_enabled, record_job_completion, record_job_received,
    record_jobs_coalesced, stale_and_record,
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
    pub stretched: bool,
}

#[derive(Debug)]
pub(crate) enum AudioLoadError {
    Missing(String),
    Failed(String),
}

#[derive(Debug)]
/// Deferred transient-marker payload for an already-delivered audio load.
pub(crate) struct AudioTransientResult {
    pub request_id: u64,
    pub source_id: SourceId,
    pub relative_path: PathBuf,
    pub metadata: FileMetadata,
    pub cache_token: u64,
    pub transients: Arc<[f32]>,
    pub stretched: bool,
}

#[derive(Debug)]
/// Audio loader worker message stream: primary load completion plus deferred transients.
pub(crate) enum AudioLoadResult {
    Primary {
        request_id: u64,
        source_id: SourceId,
        relative_path: PathBuf,
        result: Result<AudioLoadOutcome, AudioLoadError>,
    },
    Transients(AudioTransientResult),
}

#[derive(Clone)]
/// Inputs required to compute deferred transient markers after primary load delivery.
struct PendingTransientCompute {
    request_id: u64,
    source_id: SourceId,
    relative_path: PathBuf,
    metadata: FileMetadata,
    cache_token: u64,
    decoded: Arc<DecodedWaveform>,
    stretched: bool,
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
                    record_job_received();
                    let job = drain_to_latest_job(job, &rx);
                    let outcome = load_audio(&renderer, &job, &latest_request_id_worker);
                    let AudioLoadExecution::Completed(result) = outcome else {
                        continue;
                    };
                    if stale_and_record(
                        job.request_id,
                        &latest_request_id_worker,
                        StaleDropStage::PreSend,
                    ) {
                        continue;
                    }
                    record_job_completion(result.is_ok());
                    let transient_compute =
                        result.as_ref().ok().map(|outcome| PendingTransientCompute {
                            request_id: job.request_id,
                            source_id: job.source_id.clone(),
                            relative_path: job.relative_path.clone(),
                            metadata: outcome.metadata,
                            cache_token: outcome.decoded.cache_token,
                            decoded: Arc::clone(&outcome.decoded),
                            stretched: outcome.stretched,
                        });
                    let _ = result_tx.send(AudioLoadResult::Primary {
                        request_id: job.request_id,
                        source_id: job.source_id.clone(),
                        relative_path: job.relative_path.clone(),
                        result,
                    });
                    if let Some(transient_compute) = transient_compute
                        && let Some(transients_result) =
                            build_transient_result(transient_compute, &latest_request_id_worker)
                    {
                        let _ = result_tx.send(AudioLoadResult::Transients(transients_result));
                    }
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
    let mut coalesced = 0u64;
    while let Ok(next_job) = rx.try_recv() {
        latest_job = next_job;
        coalesced = coalesced.saturating_add(1);
    }
    if audio_loader_telemetry_enabled() && coalesced > 0 {
        record_jobs_coalesced(coalesced);
    }
    latest_job
}

pub(super) fn is_stale_request(request_id: u64, latest_request_id: &AtomicU64) -> bool {
    let latest = latest_request_id.load(Ordering::Relaxed);
    latest != 0 && latest != request_id
}

fn load_audio(
    renderer: &WaveformRenderer,
    job: &AudioLoadJob,
    latest_request_id: &AtomicU64,
) -> AudioLoadExecution {
    if stale_and_record(job.request_id, latest_request_id, StaleDropStage::Dispatch) {
        return AudioLoadExecution::DroppedStale;
    }
    let result = load_audio_inner(renderer, job, latest_request_id);
    match result {
        Ok(Some(outcome)) => AudioLoadExecution::Completed(Ok(outcome)),
        Ok(None) => AudioLoadExecution::DroppedStale,
        Err(err) => AudioLoadExecution::Completed(Err(err)),
    }
}
