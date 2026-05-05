use super::is_stale_request;
use crate::hotpath_telemetry;
use std::sync::{
    OnceLock,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

const AUDIO_LOADER_TELEMETRY_LOG_EVERY: u64 = 128;

static AUDIO_LOADER_TELEMETRY_ENABLED: OnceLock<bool> = OnceLock::new();
static AUDIO_LOADER_JOBS_RECEIVED: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_JOBS_COALESCED: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_JOBS_COMPLETED: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_JOBS_FAILED: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_STALE_DROPPED_TOTAL: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_STALE_DISPATCH: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_STALE_PRE_IO: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_STALE_POST_IO: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_STALE_POST_DECODE: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_STALE_POST_STRETCH: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_STALE_POST_TRANSIENTS: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_STALE_PRE_SEND: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_IO_NS_TOTAL: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_DECODE_NS_TOTAL: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_STRETCH_NS_TOTAL: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_TRANSIENT_NS_TOTAL: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_READ_BYTES_TOTAL: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_OUTPUT_BYTES_TOTAL: AtomicU64 = AtomicU64::new(0);
static AUDIO_LOADER_ALLOC_ESTIMATE_BYTES_TOTAL: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
pub(super) enum StaleDropStage {
    Dispatch,
    PreIo,
    PostIo,
    PostDecode,
    PostStretch,
    PostTransients,
    PreSend,
}

pub(super) fn audio_loader_telemetry_enabled() -> bool {
    hotpath_telemetry::enabled(&AUDIO_LOADER_TELEMETRY_ENABLED)
}

fn record_audio_loader_duration(counter: &AtomicU64, duration: Duration) {
    if !audio_loader_telemetry_enabled() {
        return;
    }
    hotpath_telemetry::add_duration_ns(counter, duration);
}

fn record_audio_loader_bytes(counter: &AtomicU64, bytes: usize) {
    if !audio_loader_telemetry_enabled() || bytes == 0 {
        return;
    }
    hotpath_telemetry::add_bytes(counter, bytes);
}

fn record_audio_loader_stale(stage: StaleDropStage) {
    if !audio_loader_telemetry_enabled() {
        return;
    }
    let sample_tick = AUDIO_LOADER_STALE_DROPPED_TOTAL.fetch_add(1, Ordering::Relaxed) + 1;
    match stage {
        StaleDropStage::Dispatch => {
            AUDIO_LOADER_STALE_DISPATCH.fetch_add(1, Ordering::Relaxed);
        }
        StaleDropStage::PreIo => {
            AUDIO_LOADER_STALE_PRE_IO.fetch_add(1, Ordering::Relaxed);
        }
        StaleDropStage::PostIo => {
            AUDIO_LOADER_STALE_POST_IO.fetch_add(1, Ordering::Relaxed);
        }
        StaleDropStage::PostDecode => {
            AUDIO_LOADER_STALE_POST_DECODE.fetch_add(1, Ordering::Relaxed);
        }
        StaleDropStage::PostStretch => {
            AUDIO_LOADER_STALE_POST_STRETCH.fetch_add(1, Ordering::Relaxed);
        }
        StaleDropStage::PostTransients => {
            AUDIO_LOADER_STALE_POST_TRANSIENTS.fetch_add(1, Ordering::Relaxed);
        }
        StaleDropStage::PreSend => {
            AUDIO_LOADER_STALE_PRE_SEND.fetch_add(1, Ordering::Relaxed);
        }
    }
    maybe_emit_audio_loader_telemetry(sample_tick);
}

pub(super) fn stale_and_record(
    request_id: u64,
    latest_request_id: &AtomicU64,
    stage: StaleDropStage,
) -> bool {
    if is_stale_request(request_id, latest_request_id) {
        record_audio_loader_stale(stage);
        return true;
    }
    false
}

pub(super) fn record_job_received() {
    if !audio_loader_telemetry_enabled() {
        return;
    }
    let sample_tick = AUDIO_LOADER_JOBS_RECEIVED.fetch_add(1, Ordering::Relaxed) + 1;
    maybe_emit_audio_loader_telemetry(sample_tick);
}

pub(super) fn record_jobs_coalesced(coalesced: u64) {
    if !audio_loader_telemetry_enabled() || coalesced == 0 {
        return;
    }
    let sample_tick = AUDIO_LOADER_JOBS_COALESCED.fetch_add(coalesced, Ordering::Relaxed) + 1;
    maybe_emit_audio_loader_telemetry(sample_tick);
}

pub(super) fn record_job_completion(success: bool) {
    if !audio_loader_telemetry_enabled() {
        return;
    }
    let sample_tick = if success {
        AUDIO_LOADER_JOBS_COMPLETED.fetch_add(1, Ordering::Relaxed) + 1
    } else {
        AUDIO_LOADER_JOBS_FAILED.fetch_add(1, Ordering::Relaxed) + 1
    };
    maybe_emit_audio_loader_telemetry(sample_tick);
}

pub(super) fn record_io_duration(duration: Duration) {
    record_audio_loader_duration(&AUDIO_LOADER_IO_NS_TOTAL, duration);
}

pub(super) fn record_decode_duration(duration: Duration) {
    record_audio_loader_duration(&AUDIO_LOADER_DECODE_NS_TOTAL, duration);
}

pub(super) fn record_stretch_duration(duration: Duration) {
    record_audio_loader_duration(&AUDIO_LOADER_STRETCH_NS_TOTAL, duration);
}

pub(super) fn record_transient_duration(duration: Duration) {
    record_audio_loader_duration(&AUDIO_LOADER_TRANSIENT_NS_TOTAL, duration);
}

pub(super) fn record_read_bytes(bytes: usize) {
    record_audio_loader_bytes(&AUDIO_LOADER_READ_BYTES_TOTAL, bytes);
}

pub(super) fn record_output_bytes(bytes: usize) {
    record_audio_loader_bytes(&AUDIO_LOADER_OUTPUT_BYTES_TOTAL, bytes);
}

pub(super) fn record_alloc_estimate_bytes(bytes: usize) {
    record_audio_loader_bytes(&AUDIO_LOADER_ALLOC_ESTIMATE_BYTES_TOTAL, bytes);
}

pub(super) fn maybe_emit_audio_loader_telemetry(sample_tick: u64) {
    if !audio_loader_telemetry_enabled()
        || !hotpath_telemetry::should_emit(sample_tick, AUDIO_LOADER_TELEMETRY_LOG_EVERY)
    {
        return;
    }

    let jobs_received = AUDIO_LOADER_JOBS_RECEIVED.load(Ordering::Relaxed);
    let jobs_coalesced = AUDIO_LOADER_JOBS_COALESCED.load(Ordering::Relaxed);
    let jobs_completed = AUDIO_LOADER_JOBS_COMPLETED.load(Ordering::Relaxed);
    let jobs_failed = AUDIO_LOADER_JOBS_FAILED.load(Ordering::Relaxed);
    let stale_total = AUDIO_LOADER_STALE_DROPPED_TOTAL.load(Ordering::Relaxed);
    let stale_dispatch = AUDIO_LOADER_STALE_DISPATCH.load(Ordering::Relaxed);
    let stale_pre_io = AUDIO_LOADER_STALE_PRE_IO.load(Ordering::Relaxed);
    let stale_post_io = AUDIO_LOADER_STALE_POST_IO.load(Ordering::Relaxed);
    let stale_post_decode = AUDIO_LOADER_STALE_POST_DECODE.load(Ordering::Relaxed);
    let stale_post_stretch = AUDIO_LOADER_STALE_POST_STRETCH.load(Ordering::Relaxed);
    let stale_post_transients = AUDIO_LOADER_STALE_POST_TRANSIENTS.load(Ordering::Relaxed);
    let stale_pre_send = AUDIO_LOADER_STALE_PRE_SEND.load(Ordering::Relaxed);
    let io_ns_total = AUDIO_LOADER_IO_NS_TOTAL.load(Ordering::Relaxed);
    let decode_ns_total = AUDIO_LOADER_DECODE_NS_TOTAL.load(Ordering::Relaxed);
    let stretch_ns_total = AUDIO_LOADER_STRETCH_NS_TOTAL.load(Ordering::Relaxed);
    let transient_ns_total = AUDIO_LOADER_TRANSIENT_NS_TOTAL.load(Ordering::Relaxed);
    let read_bytes_total = AUDIO_LOADER_READ_BYTES_TOTAL.load(Ordering::Relaxed);
    let output_bytes_total = AUDIO_LOADER_OUTPUT_BYTES_TOTAL.load(Ordering::Relaxed);
    let alloc_estimate_total = AUDIO_LOADER_ALLOC_ESTIMATE_BYTES_TOTAL.load(Ordering::Relaxed);
    let completed_nonzero = jobs_completed.max(1);

    tracing::info!(
        target: "perf::hotpath",
        module = "audio_loader",
        jobs_received,
        jobs_coalesced,
        jobs_completed,
        jobs_failed,
        stale_total,
        stale_dispatch,
        stale_pre_io,
        stale_post_io,
        stale_post_decode,
        stale_post_stretch,
        stale_post_transients,
        stale_pre_send,
        avg_io_ms = io_ns_total as f64 / completed_nonzero as f64 / 1_000_000.0,
        avg_decode_ms = decode_ns_total as f64 / completed_nonzero as f64 / 1_000_000.0,
        avg_stretch_ms = stretch_ns_total as f64 / completed_nonzero as f64 / 1_000_000.0,
        avg_transient_ms = transient_ns_total as f64 / completed_nonzero as f64 / 1_000_000.0,
        read_bytes_total,
        output_bytes_total,
        alloc_estimate_total,
        "Audio loader telemetry snapshot"
    );
}
