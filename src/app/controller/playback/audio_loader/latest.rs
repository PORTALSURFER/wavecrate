use super::{AudioLoadJob, telemetry};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc::Receiver,
};
use std::thread;

/// Join handle and shutdown signal for the audio loader thread.
pub(crate) struct AudioLoaderHandle {
    pub(super) shutdown: Arc<AtomicBool>,
    pub(super) latest_request_id: Arc<AtomicU64>,
    pub(super) join_handle: Option<thread::JoinHandle<()>>,
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

    /// Signal the loader thread to exit without waiting for the backing thread.
    pub(crate) fn request_shutdown_detached(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        let _ = self.join_handle.take();
    }
}

pub(super) fn drain_to_latest_job(
    mut latest_job: AudioLoadJob,
    rx: &Receiver<AudioLoadJob>,
) -> AudioLoadJob {
    let mut coalesced = 0u64;
    while let Ok(next_job) = rx.try_recv() {
        latest_job = next_job;
        coalesced = coalesced.saturating_add(1);
    }
    if telemetry::audio_loader_telemetry_enabled() && coalesced > 0 {
        telemetry::record_jobs_coalesced(coalesced);
    }
    latest_job
}

pub(super) fn is_stale_request(request_id: u64, latest_request_id: &AtomicU64) -> bool {
    let latest = latest_request_id.load(Ordering::Relaxed);
    latest != 0 && latest != request_id
}
