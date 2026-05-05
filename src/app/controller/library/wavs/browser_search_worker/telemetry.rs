//! Hot-path telemetry counters for browser-search queue and worker processing.

use super::*;

const SEARCH_QUEUE_TELEMETRY_LOG_EVERY: u64 = 2_048;
static SEARCH_QUEUE_TELEMETRY_ENABLED: OnceLock<bool> = OnceLock::new();
static SEARCH_QUEUE_LOCK_ACQUIRE_COUNT: AtomicU64 = AtomicU64::new(0);
static SEARCH_QUEUE_LOCK_WAIT_NS: AtomicU64 = AtomicU64::new(0);
static SEARCH_QUEUE_WAIT_COUNT: AtomicU64 = AtomicU64::new(0);
static SEARCH_QUEUE_WAIT_NS: AtomicU64 = AtomicU64::new(0);
static SEARCH_QUEUE_SEND_COUNT: AtomicU64 = AtomicU64::new(0);
static SEARCH_QUEUE_PENDING_REPLACED_COUNT: AtomicU64 = AtomicU64::new(0);
static SEARCH_QUEUE_TAKE_COUNT: AtomicU64 = AtomicU64::new(0);
static SEARCH_QUEUE_CANCEL_COUNT: AtomicU64 = AtomicU64::new(0);
static SEARCH_WORKER_SCORE_ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static SEARCH_WORKER_SCRATCH_ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static SEARCH_WORKER_SIMILAR_LOOKUP_ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
static SEARCH_WORKER_VISIBLE_ROWS_TOTAL: AtomicU64 = AtomicU64::new(0);

pub(super) fn search_queue_telemetry_enabled() -> bool {
    hotpath_telemetry::enabled(&SEARCH_QUEUE_TELEMETRY_ENABLED)
}

pub(super) fn record_search_queue_lock_wait(duration: Duration) {
    if !search_queue_telemetry_enabled() {
        return;
    }
    let sample_tick = SEARCH_QUEUE_LOCK_ACQUIRE_COUNT.fetch_add(1, AtomicOrdering::Relaxed) + 1;
    hotpath_telemetry::add_duration_ns(&SEARCH_QUEUE_LOCK_WAIT_NS, duration);
    maybe_emit_search_worker_telemetry(sample_tick);
}

pub(super) fn record_search_queue_wait(duration: Duration) {
    if !search_queue_telemetry_enabled() {
        return;
    }
    let sample_tick = SEARCH_QUEUE_WAIT_COUNT.fetch_add(1, AtomicOrdering::Relaxed) + 1;
    hotpath_telemetry::add_duration_ns(&SEARCH_QUEUE_WAIT_NS, duration);
    maybe_emit_search_worker_telemetry(sample_tick);
}

pub(super) fn record_search_queue_send(replaced_pending: bool) {
    if !search_queue_telemetry_enabled() {
        return;
    }
    SEARCH_QUEUE_SEND_COUNT.fetch_add(1, AtomicOrdering::Relaxed);
    if replaced_pending {
        SEARCH_QUEUE_PENDING_REPLACED_COUNT.fetch_add(1, AtomicOrdering::Relaxed);
    }
}

pub(super) fn record_search_queue_take() {
    if !search_queue_telemetry_enabled() {
        return;
    }
    let sample_tick = SEARCH_QUEUE_TAKE_COUNT.fetch_add(1, AtomicOrdering::Relaxed) + 1;
    maybe_emit_search_worker_telemetry(sample_tick);
}

pub(super) fn record_search_worker_score_alloc(bytes: usize) {
    record_search_worker_allocation(&SEARCH_WORKER_SCORE_ALLOC_BYTES, bytes);
}

pub(super) fn record_search_worker_scratch_alloc(bytes: usize) {
    record_search_worker_allocation(&SEARCH_WORKER_SCRATCH_ALLOC_BYTES, bytes);
}

pub(super) fn record_search_worker_similar_lookup_alloc(bytes: usize) {
    record_search_worker_allocation(&SEARCH_WORKER_SIMILAR_LOOKUP_ALLOC_BYTES, bytes);
}

pub(super) fn record_search_worker_visible_rows(count: usize) {
    if !search_queue_telemetry_enabled() || count == 0 {
        return;
    }
    SEARCH_WORKER_VISIBLE_ROWS_TOTAL.fetch_add(count as u64, AtomicOrdering::Relaxed);
}

pub(super) fn record_search_job_cancel() {
    if !search_queue_telemetry_enabled() {
        return;
    }
    let sample_tick = SEARCH_QUEUE_CANCEL_COUNT.fetch_add(1, AtomicOrdering::Relaxed) + 1;
    maybe_emit_search_worker_telemetry(sample_tick);
}

fn record_search_worker_allocation(counter: &AtomicU64, bytes: usize) {
    if !search_queue_telemetry_enabled() || bytes == 0 {
        return;
    }
    hotpath_telemetry::add_bytes(counter, bytes);
}

fn maybe_emit_search_worker_telemetry(sample_tick: u64) {
    if !search_queue_telemetry_enabled()
        || !hotpath_telemetry::should_emit(sample_tick, SEARCH_QUEUE_TELEMETRY_LOG_EVERY)
    {
        return;
    }

    let lock_acquires = SEARCH_QUEUE_LOCK_ACQUIRE_COUNT.load(AtomicOrdering::Relaxed);
    let lock_wait_ns = SEARCH_QUEUE_LOCK_WAIT_NS.load(AtomicOrdering::Relaxed);
    let wait_count = SEARCH_QUEUE_WAIT_COUNT.load(AtomicOrdering::Relaxed);
    let wait_ns = SEARCH_QUEUE_WAIT_NS.load(AtomicOrdering::Relaxed);
    let send_count = SEARCH_QUEUE_SEND_COUNT.load(AtomicOrdering::Relaxed);
    let replaced_count = SEARCH_QUEUE_PENDING_REPLACED_COUNT.load(AtomicOrdering::Relaxed);
    let take_count = SEARCH_QUEUE_TAKE_COUNT.load(AtomicOrdering::Relaxed);
    let cancel_count = SEARCH_QUEUE_CANCEL_COUNT.load(AtomicOrdering::Relaxed);
    let score_alloc_bytes = SEARCH_WORKER_SCORE_ALLOC_BYTES.load(AtomicOrdering::Relaxed);
    let scratch_alloc_bytes = SEARCH_WORKER_SCRATCH_ALLOC_BYTES.load(AtomicOrdering::Relaxed);
    let similar_lookup_alloc_bytes =
        SEARCH_WORKER_SIMILAR_LOOKUP_ALLOC_BYTES.load(AtomicOrdering::Relaxed);
    let visible_rows_total = SEARCH_WORKER_VISIBLE_ROWS_TOTAL.load(AtomicOrdering::Relaxed);

    let avg_lock_wait_us = if lock_acquires == 0 {
        0.0
    } else {
        lock_wait_ns as f64 / lock_acquires as f64 / 1_000.0
    };
    let avg_wait_ms = if wait_count == 0 {
        0.0
    } else {
        wait_ns as f64 / wait_count as f64 / 1_000_000.0
    };

    tracing::info!(
        target: "perf::hotpath",
        module = "browser_search_worker",
        lock_acquires,
        avg_lock_wait_us,
        condvar_waits = wait_count,
        avg_condvar_wait_ms = avg_wait_ms,
        sends = send_count,
        pending_replaced = replaced_count,
        takes = take_count,
        cancels = cancel_count,
        score_alloc_bytes,
        scratch_alloc_bytes,
        similar_lookup_alloc_bytes,
        visible_rows_total,
        "Search worker queue telemetry snapshot"
    );
}
