//! Hot-path telemetry counters for async source hydration.

use crate::hotpath_telemetry;
use std::sync::{
    OnceLock,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

const SOURCE_HYDRATION_TELEMETRY_LOG_EVERY: u64 = 64;

static SOURCE_HYDRATION_TELEMETRY_ENABLED: OnceLock<bool> = OnceLock::new();
static SOURCE_HYDRATION_DISPATCHED: AtomicU64 = AtomicU64::new(0);
static SOURCE_HYDRATION_COMPLETED: AtomicU64 = AtomicU64::new(0);
static SOURCE_HYDRATION_FAILED: AtomicU64 = AtomicU64::new(0);
static SOURCE_HYDRATION_STALE_DROPPED: AtomicU64 = AtomicU64::new(0);
static SOURCE_HYDRATION_WORKER_NS_TOTAL: AtomicU64 = AtomicU64::new(0);
static SOURCE_HYDRATION_APPLY_NS_TOTAL: AtomicU64 = AtomicU64::new(0);

pub(super) fn source_hydration_telemetry_enabled() -> bool {
    hotpath_telemetry::enabled(&SOURCE_HYDRATION_TELEMETRY_ENABLED)
}

pub(super) fn record_source_hydration_dispatch() {
    if !source_hydration_telemetry_enabled() {
        return;
    }
    let sample_tick = SOURCE_HYDRATION_DISPATCHED.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_source_hydration_telemetry(sample_tick);
}

pub(super) fn record_source_hydration_worker(success: bool, duration: Duration) {
    if !source_hydration_telemetry_enabled() {
        return;
    }
    hotpath_telemetry::add_duration_ns(&SOURCE_HYDRATION_WORKER_NS_TOTAL, duration);
    let sample_tick = if success {
        SOURCE_HYDRATION_COMPLETED.fetch_add(1, Ordering::Relaxed) + 1
    } else {
        SOURCE_HYDRATION_FAILED.fetch_add(1, Ordering::Relaxed) + 1
    };
    maybe_emit_source_hydration_telemetry(sample_tick);
}

pub(super) fn record_source_hydration_apply(duration: Duration) {
    if !source_hydration_telemetry_enabled() {
        return;
    }
    hotpath_telemetry::add_duration_ns(&SOURCE_HYDRATION_APPLY_NS_TOTAL, duration);
    let sample_tick = SOURCE_HYDRATION_COMPLETED.load(Ordering::Relaxed).max(1);
    maybe_emit_source_hydration_telemetry(sample_tick);
}

pub(super) fn record_source_hydration_stale_drop() {
    if !source_hydration_telemetry_enabled() {
        return;
    }
    let sample_tick = SOURCE_HYDRATION_STALE_DROPPED.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_source_hydration_telemetry(sample_tick);
}

fn maybe_emit_source_hydration_telemetry(sample_tick: u64) {
    if !source_hydration_telemetry_enabled()
        || !hotpath_telemetry::should_emit(sample_tick, SOURCE_HYDRATION_TELEMETRY_LOG_EVERY)
    {
        return;
    }

    let dispatched = SOURCE_HYDRATION_DISPATCHED.load(Ordering::Relaxed);
    let completed = SOURCE_HYDRATION_COMPLETED.load(Ordering::Relaxed);
    let failed = SOURCE_HYDRATION_FAILED.load(Ordering::Relaxed);
    let stale_dropped = SOURCE_HYDRATION_STALE_DROPPED.load(Ordering::Relaxed);
    let completed_nonzero = completed.max(1);
    let worker_ns_total = SOURCE_HYDRATION_WORKER_NS_TOTAL.load(Ordering::Relaxed);
    let apply_ns_total = SOURCE_HYDRATION_APPLY_NS_TOTAL.load(Ordering::Relaxed);

    tracing::info!(
        target: "perf::hotpath",
        module = "source_hydration",
        dispatched,
        completed,
        failed,
        stale_dropped,
        avg_worker_ms = worker_ns_total as f64 / completed_nonzero as f64 / 1_000_000.0,
        avg_apply_ms = apply_ns_total as f64 / completed_nonzero as f64 / 1_000_000.0,
        "Source hydration telemetry snapshot"
    );
}
