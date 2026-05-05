//! Hot-path telemetry counters for async folder-tree projection work.

use crate::hotpath_telemetry;
use std::sync::{
    OnceLock,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

const FOLDER_PROJECTION_TELEMETRY_LOG_EVERY: u64 = 128;

static FOLDER_PROJECTION_TELEMETRY_ENABLED: OnceLock<bool> = OnceLock::new();
static FOLDER_PROJECTION_DISPATCHED: AtomicU64 = AtomicU64::new(0);
static FOLDER_PROJECTION_COMPLETED: AtomicU64 = AtomicU64::new(0);
static FOLDER_PROJECTION_STALE_DROPPED: AtomicU64 = AtomicU64::new(0);
static FOLDER_PROJECTION_WORKER_NS_TOTAL: AtomicU64 = AtomicU64::new(0);
static FOLDER_PROJECTION_APPLY_NS_TOTAL: AtomicU64 = AtomicU64::new(0);
static FOLDER_PROJECTION_FOLDER_TOTAL: AtomicU64 = AtomicU64::new(0);
static FOLDER_PROJECTION_VISIBLE_ROWS_TOTAL: AtomicU64 = AtomicU64::new(0);

fn folder_projection_telemetry_enabled() -> bool {
    hotpath_telemetry::enabled(&FOLDER_PROJECTION_TELEMETRY_ENABLED)
}

pub(super) fn record_folder_projection_dispatch(folder_count: usize) {
    if !folder_projection_telemetry_enabled() {
        return;
    }
    FOLDER_PROJECTION_FOLDER_TOTAL.fetch_add(folder_count as u64, Ordering::Relaxed);
    let tick = FOLDER_PROJECTION_DISPATCHED.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_folder_projection_telemetry(tick);
}

pub(super) fn record_folder_projection_worker(
    duration: Duration,
    folder_count: usize,
    visible_rows: usize,
) {
    if !folder_projection_telemetry_enabled() {
        return;
    }
    hotpath_telemetry::add_duration_ns(&FOLDER_PROJECTION_WORKER_NS_TOTAL, duration);
    FOLDER_PROJECTION_FOLDER_TOTAL.fetch_add(folder_count as u64, Ordering::Relaxed);
    FOLDER_PROJECTION_VISIBLE_ROWS_TOTAL.fetch_add(visible_rows as u64, Ordering::Relaxed);
    let tick = FOLDER_PROJECTION_COMPLETED.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_folder_projection_telemetry(tick);
}

pub(super) fn record_folder_projection_apply(duration: Duration) {
    if !folder_projection_telemetry_enabled() {
        return;
    }
    hotpath_telemetry::add_duration_ns(&FOLDER_PROJECTION_APPLY_NS_TOTAL, duration);
    let tick = FOLDER_PROJECTION_COMPLETED.load(Ordering::Relaxed).max(1);
    maybe_emit_folder_projection_telemetry(tick);
}

pub(super) fn record_folder_projection_stale_drop() {
    if !folder_projection_telemetry_enabled() {
        return;
    }
    let tick = FOLDER_PROJECTION_STALE_DROPPED.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_folder_projection_telemetry(tick);
}

fn maybe_emit_folder_projection_telemetry(sample_tick: u64) {
    if !folder_projection_telemetry_enabled()
        || !hotpath_telemetry::should_emit(sample_tick, FOLDER_PROJECTION_TELEMETRY_LOG_EVERY)
    {
        return;
    }

    let dispatched = FOLDER_PROJECTION_DISPATCHED.load(Ordering::Relaxed);
    let completed = FOLDER_PROJECTION_COMPLETED.load(Ordering::Relaxed);
    let stale_dropped = FOLDER_PROJECTION_STALE_DROPPED.load(Ordering::Relaxed);
    let completed_nonzero = completed.max(1);
    let worker_ns_total = FOLDER_PROJECTION_WORKER_NS_TOTAL.load(Ordering::Relaxed);
    let apply_ns_total = FOLDER_PROJECTION_APPLY_NS_TOTAL.load(Ordering::Relaxed);
    let folder_total = FOLDER_PROJECTION_FOLDER_TOTAL.load(Ordering::Relaxed);
    let visible_rows_total = FOLDER_PROJECTION_VISIBLE_ROWS_TOTAL.load(Ordering::Relaxed);

    tracing::info!(
        target: "perf::hotpath",
        module = "folder_projection",
        dispatched,
        completed,
        stale_dropped,
        avg_worker_ms = worker_ns_total as f64 / completed_nonzero as f64 / 1_000_000.0,
        avg_apply_ms = apply_ns_total as f64 / completed_nonzero as f64 / 1_000_000.0,
        avg_folder_count = folder_total as f64 / completed_nonzero as f64,
        avg_visible_rows = visible_rows_total as f64 / completed_nonzero as f64,
        "Folder projection telemetry snapshot"
    );
}
