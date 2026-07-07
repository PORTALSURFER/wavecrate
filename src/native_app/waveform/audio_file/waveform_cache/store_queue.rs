use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::{Arc, Condvar, LazyLock, Mutex},
    thread,
    time::{Duration, Instant},
};

use super::{
    identity::{cache_path_for_identity, CacheIdentity},
    invalidation::current_path_generation,
    write::store_cached_waveform_file_now,
    BACKGROUND_STORE_SHUTDOWN_WAIT,
};
use crate::native_app::waveform::audio_file::WaveformFile;
use diagnostics::{log_slow_cache_shutdown_flush, log_store_completion};

mod diagnostics;

const BACKGROUND_STORE_QUEUE_CAPACITY: usize = 128;

static BACKGROUND_STORE_QUEUE: LazyLock<Arc<BackgroundStoreQueue>> =
    LazyLock::new(|| BackgroundStoreQueue::start(BACKGROUND_STORE_QUEUE_CAPACITY));

#[cfg(test)]
pub(in crate::native_app::waveform::audio_file) fn store_cached_waveform_file(file: &WaveformFile) {
    let Some(job) = CachedWaveformStoreJob::new(file) else {
        return;
    };
    let _ = store_cached_waveform_file_now(job);
}

pub(in crate::native_app::waveform::audio_file) fn store_cached_waveform_file_in_background(
    file: &WaveformFile,
) {
    let Some(job) = CachedWaveformStoreJob::new(file) else {
        return;
    };
    let path = job.file.path.clone();
    let cache_path = job.cache_path.clone();
    match BACKGROUND_STORE_QUEUE.enqueue(job) {
        StoreEnqueueOutcome::Enqueued => {}
        StoreEnqueueOutcome::ReplacedQueued => {
            tracing::debug!(
                target: "wavecrate::debug::sample_cache",
                event = "browser.sample_cache.store_replaced_queued",
                path = %path.display(),
                cache_path = %cache_path.display(),
                "Replaced queued waveform cache persistence"
            );
        }
        StoreEnqueueOutcome::DeferredForActive => {
            tracing::debug!(
                target: "wavecrate::debug::sample_cache",
                event = "browser.sample_cache.store_deferred_for_active",
                path = %path.display(),
                cache_path = %cache_path.display(),
                "Deferred waveform cache persistence until active write finishes"
            );
        }
        StoreEnqueueOutcome::QueueFull => {
            tracing::warn!(
                target: "wavecrate::debug::sample_cache",
                event = "browser.sample_cache.store_dropped_queue_full",
                path = %path.display(),
                cache_path = %cache_path.display(),
                capacity = BACKGROUND_STORE_QUEUE.capacity(),
                "Dropped waveform cache persistence because the writer queue is full"
            );
        }
        StoreEnqueueOutcome::WorkerUnavailable => {
            tracing::warn!(
                target: "wavecrate::debug::sample_cache",
                event = "browser.sample_cache.store_dropped_worker_unavailable",
                path = %path.display(),
                cache_path = %cache_path.display(),
                "Dropped waveform cache persistence because the writer worker is unavailable"
            );
        }
    }
}

pub(in crate::native_app) fn flush_background_waveform_cache_stores_for_shutdown() {
    BACKGROUND_STORE_QUEUE.flush_for_shutdown(BACKGROUND_STORE_SHUTDOWN_WAIT);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(test, allow(dead_code))]
pub(super) enum StoreEnqueueOutcome {
    Enqueued,
    ReplacedQueued,
    DeferredForActive,
    QueueFull,
    WorkerUnavailable,
}

pub(super) struct BackgroundStoreQueue {
    capacity: usize,
    state: Mutex<StoreQueueState>,
    available: Condvar,
    drained: Condvar,
}

impl BackgroundStoreQueue {
    fn start(capacity: usize) -> Arc<Self> {
        let queue = Arc::new(Self::new(capacity, true));
        let worker_queue = Arc::clone(&queue);
        if let Err(err) = thread::Builder::new()
            .name(String::from("waveform-cache-store"))
            .spawn(move || worker_queue.run_worker())
        {
            if let Ok(mut state) = queue.state.lock() {
                state.worker_available = false;
            }
            tracing::warn!(
                target: "wavecrate::debug::sample_cache",
                event = "browser.sample_cache.store_worker_spawn_error",
                error = %err,
                "Failed to spawn waveform cache persistence worker"
            );
        }
        queue
    }

    fn new(capacity: usize, worker_available: bool) -> Self {
        Self {
            capacity,
            state: Mutex::new(StoreQueueState {
                worker_available,
                ..StoreQueueState::default()
            }),
            available: Condvar::new(),
            drained: Condvar::new(),
        }
    }

    fn capacity(&self) -> usize {
        self.capacity
    }

    pub(super) fn enqueue(&self, job: CachedWaveformStoreJob) -> StoreEnqueueOutcome {
        let Ok(mut state) = self.state.lock() else {
            return StoreEnqueueOutcome::WorkerUnavailable;
        };
        if !state.worker_available {
            return StoreEnqueueOutcome::WorkerUnavailable;
        }
        if state.queued_paths.contains(&job.cache_path) {
            replace_queued_job(&mut state.queued, job);
            return StoreEnqueueOutcome::ReplacedQueued;
        }
        if state.active_paths.contains(&job.cache_path) {
            state.active_successors.insert(job.cache_path.clone(), job);
            return StoreEnqueueOutcome::DeferredForActive;
        }
        if state.queued.len() >= self.capacity {
            return StoreEnqueueOutcome::QueueFull;
        }
        state.queued_paths.insert(job.cache_path.clone());
        state.queued.push_back(job);
        self.available.notify_one();
        StoreEnqueueOutcome::Enqueued
    }

    fn run_worker(&self) {
        loop {
            let job = self.next_job();
            let cache_path = job.cache_path.clone();
            let outcome = store_cached_waveform_file_now(job);
            log_store_completion(&cache_path, outcome);
            self.finish_job(&cache_path);
        }
    }

    fn next_job(&self) -> CachedWaveformStoreJob {
        let mut state = self.state.lock().expect("waveform cache queue lock");
        loop {
            if let Some(job) = state.queued.pop_front() {
                state.queued_paths.remove(&job.cache_path);
                state.active_paths.insert(job.cache_path.clone());
                return job;
            }
            state = self
                .available
                .wait(state)
                .expect("waveform cache queue condvar");
        }
    }

    pub(super) fn finish_job(&self, cache_path: &Path) {
        if let Ok(mut state) = self.state.lock() {
            state.active_paths.remove(cache_path);
            if let Some(successor) = state.active_successors.remove(cache_path) {
                let successor_cache_path = successor.cache_path.clone();
                if state.queued_paths.contains(&successor_cache_path) {
                    replace_queued_job(&mut state.queued, successor);
                } else {
                    state.queued_paths.insert(successor_cache_path);
                    state.queued.push_front(successor);
                }
                self.available.notify_one();
            }
            if state.is_drained() {
                self.drained.notify_all();
            }
        }
    }

    pub(super) fn flush_for_shutdown(&self, wait: Duration) {
        let started_at = Instant::now();
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        while !state.is_drained() {
            let remaining = wait.saturating_sub(started_at.elapsed());
            if remaining.is_zero() {
                break;
            }
            let Ok((next_state, timeout)) = self.drained.wait_timeout(state, remaining) else {
                return;
            };
            state = next_state;
            if timeout.timed_out() {
                break;
            }
        }
        if !state.is_drained() {
            tracing::warn!(
                target: "wavecrate::debug::sample_cache",
                event = "browser.sample_cache.shutdown_flush_timeout",
                queued = state.queued.len(),
                active = state.active_paths.len(),
                active_successors = state.active_successors.len(),
                elapsed_ms = started_at.elapsed().as_secs_f64() * 1000.0,
                "Timed out waiting for waveform cache persistence during shutdown"
            );
        } else {
            log_slow_cache_shutdown_flush(started_at);
        }
    }

    #[cfg(test)]
    pub(super) fn pop_next_for_test(&self) -> Option<CachedWaveformStoreJob> {
        let mut state = self.state.lock().expect("waveform cache queue lock");
        let job = state.queued.pop_front()?;
        state.queued_paths.remove(&job.cache_path);
        state.active_paths.insert(job.cache_path.clone());
        Some(job)
    }

    #[cfg(test)]
    pub(super) fn pending_for_test(&self) -> usize {
        let state = self.state.lock().expect("waveform cache queue lock");
        state.queued.len() + state.active_paths.len() + state.active_successors.len()
    }
}

#[derive(Default)]
struct StoreQueueState {
    worker_available: bool,
    queued: VecDeque<CachedWaveformStoreJob>,
    queued_paths: HashSet<PathBuf>,
    active_paths: HashSet<PathBuf>,
    active_successors: HashMap<PathBuf, CachedWaveformStoreJob>,
}

impl StoreQueueState {
    fn is_drained(&self) -> bool {
        self.queued.is_empty() && self.active_paths.is_empty() && self.active_successors.is_empty()
    }
}

#[derive(Clone)]
pub(super) struct CachedWaveformStoreJob {
    pub(super) file: WaveformFile,
    pub(super) identity: CacheIdentity,
    pub(super) cache_path: PathBuf,
    pub(super) path_generation: u64,
}

impl CachedWaveformStoreJob {
    pub(super) fn new(file: &WaveformFile) -> Option<Self> {
        if file.path.as_os_str().is_empty()
            || file.sample_rate == 0
            || file.channels == 0
            || file.frames == 0
        {
            return None;
        }
        let identity = CacheIdentity::for_path(&file.path).ok()?;
        let cache_path = cache_path_for_identity(&file.path, &identity).ok()?;
        let path_generation = current_path_generation(&file.path);
        Some(Self {
            file: file.clone(),
            identity,
            cache_path,
            path_generation,
        })
    }
}

fn replace_queued_job(queued: &mut VecDeque<CachedWaveformStoreJob>, job: CachedWaveformStoreJob) {
    if let Some(existing) = queued
        .iter_mut()
        .find(|existing| existing.cache_path == job.cache_path)
    {
        *existing = job;
    }
}

#[cfg(test)]
pub(super) fn test_store_queue(capacity: usize) -> BackgroundStoreQueue {
    BackgroundStoreQueue::new(capacity, true)
}
