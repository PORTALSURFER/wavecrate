use super::{
    AudioLoadJob, AudioLoadResult, AudioLoaderHandle,
    latest::drain_to_latest_job,
    pending::PendingVisualCompute,
    primary::{AudioLoadExecution, load_audio},
    telemetry::{StaleDropStage, record_job_completion, record_job_received, stale_and_record},
    visual::build_visual_result,
};
use crate::waveform::WaveformRenderer;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{Receiver, Sender},
    },
    thread,
    time::Duration,
};

const AUDIO_LOADER_POLL_INTERVAL: Duration = Duration::from_millis(200);

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
                        result.as_ref().ok().map(|outcome| PendingVisualCompute {
                            request_id: job.request_id,
                            source_id: job.source_id.clone(),
                            relative_path: job.relative_path.clone(),
                            metadata: outcome.metadata,
                            cache_token: outcome.decoded.cache_token,
                            decoded: Arc::clone(&outcome.decoded),
                            render_spec: job.render_spec,
                            known_transients: outcome.transients.as_ref().map(Arc::clone).or_else(
                                || {
                                    job.prepared
                                        .as_ref()
                                        .map(|prepared| Arc::clone(&prepared.transients))
                                },
                            ),
                            stretched: outcome.stretched,
                        });
                    let _ = result_tx.send(AudioLoadResult::Primary {
                        request_id: job.request_id,
                        source_id: job.source_id.clone(),
                        relative_path: job.relative_path.clone(),
                        result,
                    });
                    if let Some(transient_compute) = transient_compute
                        && let Some(transients_result) = build_visual_result(
                            &renderer,
                            transient_compute,
                            &latest_request_id_worker,
                        )
                    {
                        let _ = result_tx.send(AudioLoadResult::Visual(transients_result));
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
