//! Latest-only search queue, worker lifecycle, and queue telemetry plumbing.

use super::cache::SearchWorkerCache;
use super::pipeline::process_search_job;
use super::telemetry::{
    record_search_queue_lock_wait, record_search_queue_send, record_search_queue_take,
    record_search_queue_wait, search_queue_telemetry_enabled,
};
use super::*;

#[derive(Default)]
pub(super) struct SearchJobQueueState {
    pending: Option<QueuedSearchJob>,
    poisoned_recovered: bool,
    shutdown: bool,
}

pub(super) struct QueuedSearchJob {
    pub(super) job: SearchJob,
    pub(super) generation: u64,
}

/// Latest-only queue for browser search jobs.
pub(super) struct SearchJobQueue {
    pub(super) state: Mutex<SearchJobQueueState>,
    generation: AtomicU64,
    ready: Condvar,
}

impl SearchJobQueue {
    pub(super) fn new() -> Self {
        Self {
            state: Mutex::new(SearchJobQueueState::default()),
            generation: AtomicU64::new(0),
            ready: Condvar::new(),
        }
    }

    pub(super) fn send(&self, job: SearchJob) {
        let mut state = self.lock_state();
        if state.shutdown {
            return;
        }
        record_search_queue_send(state.pending.is_some());
        state.pending = Some(QueuedSearchJob {
            job,
            generation: self.next_generation(),
        });
        self.ready.notify_one();
    }

    pub(super) fn shutdown(&self) {
        let mut state = self.lock_state();
        state.shutdown = true;
        state.pending = None;
        self.next_generation();
        self.ready.notify_all();
    }

    pub(super) fn take_blocking(&self) -> Option<QueuedSearchJob> {
        let mut state = self.lock_state();
        loop {
            if state.shutdown {
                return None;
            }
            if let Some(job) = state.pending.take() {
                record_search_queue_take();
                return Some(job);
            }
            state = self.wait_ready(state);
        }
    }

    #[cfg(test)]
    pub(super) fn try_take(&self) -> Option<QueuedSearchJob> {
        let mut state = self.lock_state();
        state.pending.take()
    }

    pub(super) fn is_generation_current(&self, generation: u64) -> bool {
        self.generation.load(AtomicOrdering::Relaxed) == generation
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, SearchJobQueueState> {
        let lock_start = search_queue_telemetry_enabled().then(Instant::now);
        let guard = match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => self.recover_state("lock", poisoned),
        };
        if let Some(start) = lock_start {
            record_search_queue_lock_wait(start.elapsed());
        }
        guard
    }

    fn wait_ready<'a>(
        &self,
        guard: std::sync::MutexGuard<'a, SearchJobQueueState>,
    ) -> std::sync::MutexGuard<'a, SearchJobQueueState> {
        let wait_start = search_queue_telemetry_enabled().then(Instant::now);
        let guard = self
            .ready
            .wait(guard)
            .unwrap_or_else(|poisoned| self.recover_state("condvar", poisoned));
        if let Some(start) = wait_start {
            record_search_queue_wait(start.elapsed());
        }
        guard
    }

    fn recover_state<'a>(
        &self,
        context: &'static str,
        poisoned: std::sync::PoisonError<std::sync::MutexGuard<'a, SearchJobQueueState>>,
    ) -> std::sync::MutexGuard<'a, SearchJobQueueState> {
        let mut guard = poisoned.into_inner();
        if !guard.poisoned_recovered {
            warn!("Search job queue {context} poisoned; recovering and clearing pending job.");
            guard.pending = None;
            guard.poisoned_recovered = true;
        }
        guard
    }

    fn next_generation(&self) -> u64 {
        self.generation
            .fetch_add(1, AtomicOrdering::Relaxed)
            .wrapping_add(1)
    }
}

/// Sender handle for coalesced search jobs.
#[derive(Clone)]
pub(crate) struct SearchJobSender {
    pub(super) queue: Arc<SearchJobQueue>,
}

impl SearchJobSender {
    /// Replace any pending search job with the latest request.
    pub(crate) fn send(&self, job: SearchJob) {
        self.queue.send(job);
    }
}

/// Join handle and shutdown signal for the browser search worker thread.
pub(crate) struct SearchWorkerHandle {
    queue: Arc<SearchJobQueue>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl SearchWorkerHandle {
    /// Signal the worker thread to exit and wait for it to finish.
    pub(crate) fn shutdown(&mut self) {
        self.queue.shutdown();
        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Spawn a background worker that processes the latest pending search job.
/// Returns the sender, result channel, and a shutdown handle.
pub(crate) fn spawn_search_worker() -> (SearchJobSender, Receiver<SearchResult>, SearchWorkerHandle)
{
    let queue = Arc::new(SearchJobQueue::new());
    let sender = SearchJobSender {
        queue: Arc::clone(&queue),
    };
    let (result_tx, result_rx) = std::sync::mpsc::channel::<SearchResult>();
    let queue_worker = Arc::clone(&queue);
    let handle = thread::spawn(move || {
        let matcher = SkimMatcherV2::default();
        let mut cache = SearchWorkerCache::default();
        while let Some(queued) = queue_worker.take_blocking() {
            if let Some(result) = process_search_job(
                queued.job,
                &matcher,
                &mut cache,
                &queue_worker,
                queued.generation,
            ) {
                let _ = result_tx.send(result);
            }
        }
    });
    (
        sender,
        result_rx,
        SearchWorkerHandle {
            queue,
            join_handle: Some(handle),
        },
    )
}
