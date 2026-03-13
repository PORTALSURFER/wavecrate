use crate::hotpath_telemetry;
use std::{
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

const ZOOM_CACHE_TELEMETRY_LOG_EVERY: u64 = 1_024;
static ZOOM_CACHE_TELEMETRY_ENABLED: OnceLock<bool> = OnceLock::new();
static ZOOM_CACHE_LOCK_ACQUIRE_COUNT: AtomicU64 = AtomicU64::new(0);
static ZOOM_CACHE_LOCK_WAIT_NS: AtomicU64 = AtomicU64::new(0);
static ZOOM_CACHE_LOCK_POISON_RECOVERY_COUNT: AtomicU64 = AtomicU64::new(0);
static ZOOM_CACHE_HIT_COUNT: AtomicU64 = AtomicU64::new(0);
static ZOOM_CACHE_MISS_COUNT: AtomicU64 = AtomicU64::new(0);
static ZOOM_CACHE_INSERT_COUNT: AtomicU64 = AtomicU64::new(0);
static ZOOM_CACHE_EVICT_COUNT: AtomicU64 = AtomicU64::new(0);
static ZOOM_CACHE_COMPACT_COUNT: AtomicU64 = AtomicU64::new(0);
static ZOOM_CACHE_RESIDENT_BYTES: AtomicU64 = AtomicU64::new(0);
static ZOOM_CACHE_PEAK_RESIDENT_BYTES: AtomicU64 = AtomicU64::new(0);

pub(super) fn zoom_cache_telemetry_enabled() -> bool {
    hotpath_telemetry::enabled(&ZOOM_CACHE_TELEMETRY_ENABLED)
}

pub(super) fn record_zoom_cache_lock_wait(duration: Duration) {
    if !zoom_cache_telemetry_enabled() {
        return;
    }
    let sample_tick = ZOOM_CACHE_LOCK_ACQUIRE_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    hotpath_telemetry::add_duration_ns(&ZOOM_CACHE_LOCK_WAIT_NS, duration);
    maybe_emit_zoom_cache_telemetry(sample_tick);
}

pub(super) fn record_zoom_cache_hit() {
    if !zoom_cache_telemetry_enabled() {
        return;
    }
    let sample_tick = ZOOM_CACHE_HIT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_zoom_cache_telemetry(sample_tick);
}

pub(super) fn record_zoom_cache_miss() {
    if !zoom_cache_telemetry_enabled() {
        return;
    }
    let sample_tick = ZOOM_CACHE_MISS_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_zoom_cache_telemetry(sample_tick);
}

pub(super) fn record_zoom_cache_insert() {
    if !zoom_cache_telemetry_enabled() {
        return;
    }
    let sample_tick = ZOOM_CACHE_INSERT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_zoom_cache_telemetry(sample_tick);
}

pub(super) fn record_zoom_cache_evict() {
    if !zoom_cache_telemetry_enabled() {
        return;
    }
    let sample_tick = ZOOM_CACHE_EVICT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_zoom_cache_telemetry(sample_tick);
}

pub(super) fn record_zoom_cache_compaction() {
    if !zoom_cache_telemetry_enabled() {
        return;
    }
    let sample_tick = ZOOM_CACHE_COMPACT_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_zoom_cache_telemetry(sample_tick);
}

pub(super) fn add_zoom_cache_resident_bytes(bytes: usize) {
    if !zoom_cache_telemetry_enabled() {
        return;
    }
    let added = bytes.min(u64::MAX as usize) as u64;
    let resident = ZOOM_CACHE_RESIDENT_BYTES
        .fetch_add(added, Ordering::Relaxed)
        .saturating_add(added);
    ZOOM_CACHE_PEAK_RESIDENT_BYTES.fetch_max(resident, Ordering::Relaxed);
}

pub(super) fn subtract_zoom_cache_resident_bytes(bytes: usize) {
    if !zoom_cache_telemetry_enabled() {
        return;
    }
    let removed = bytes.min(u64::MAX as usize) as u64;
    let _ = ZOOM_CACHE_RESIDENT_BYTES.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
        Some(value.saturating_sub(removed))
    });
}

pub(super) fn record_zoom_cache_poison_recovery(stale_bytes: usize) {
    ZOOM_CACHE_LOCK_POISON_RECOVERY_COUNT.fetch_add(1, Ordering::Relaxed);
    subtract_zoom_cache_resident_bytes(stale_bytes);
}

fn maybe_emit_zoom_cache_telemetry(sample_tick: u64) {
    if !zoom_cache_telemetry_enabled()
        || !hotpath_telemetry::should_emit(sample_tick, ZOOM_CACHE_TELEMETRY_LOG_EVERY)
    {
        return;
    }

    let lock_acquires = ZOOM_CACHE_LOCK_ACQUIRE_COUNT.load(Ordering::Relaxed);
    let lock_wait_ns = ZOOM_CACHE_LOCK_WAIT_NS.load(Ordering::Relaxed);
    let lock_poison_recoveries = ZOOM_CACHE_LOCK_POISON_RECOVERY_COUNT.load(Ordering::Relaxed);
    let hits = ZOOM_CACHE_HIT_COUNT.load(Ordering::Relaxed);
    let misses = ZOOM_CACHE_MISS_COUNT.load(Ordering::Relaxed);
    let inserts = ZOOM_CACHE_INSERT_COUNT.load(Ordering::Relaxed);
    let evicts = ZOOM_CACHE_EVICT_COUNT.load(Ordering::Relaxed);
    let compactions = ZOOM_CACHE_COMPACT_COUNT.load(Ordering::Relaxed);
    let resident_bytes = ZOOM_CACHE_RESIDENT_BYTES.load(Ordering::Relaxed);
    let peak_resident_bytes = ZOOM_CACHE_PEAK_RESIDENT_BYTES.load(Ordering::Relaxed);
    let avg_lock_wait_us = if lock_acquires == 0 {
        0.0
    } else {
        lock_wait_ns as f64 / lock_acquires as f64 / 1_000.0
    };

    tracing::info!(
        target: "perf::hotpath",
        module = "waveform_zoom_cache",
        lock_acquires,
        avg_lock_wait_us,
        lock_poison_recoveries,
        hits,
        misses,
        inserts,
        evicts,
        compactions,
        resident_bytes,
        peak_resident_bytes,
        "Waveform zoom cache telemetry snapshot"
    );
}
