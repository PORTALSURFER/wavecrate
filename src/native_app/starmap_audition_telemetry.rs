use std::{
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

const HOTPATH_TELEMETRY_ENV: &str = "WAVECRATE_HOTPATH_TELEMETRY";

const STARMAP_AUDITION_TELEMETRY_LOG_EVERY: u64 = 32;

static STARMAP_AUDITION_TELEMETRY_ENABLED: OnceLock<bool> = OnceLock::new();
static STARMAP_AUDITION_EVENTS_TOTAL: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_DRAG_BEGIN: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_DRAG_UPDATE: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_DRAG_FINISH: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_WIDGET_POINT_HIT: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_WIDGET_POINT_MISS: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_WIDGET_SEGMENT_HIT: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_WIDGET_SEGMENT_MISS: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_HITS_QUEUED: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_HITS_STARTED: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_DUPLICATE_ACTIVE: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_DUPLICATE_QUEUED: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_ACTIVE_REPLACED: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_ADVANCE_SCHEDULED: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_ADVANCE_STALE: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_LOADED_CURRENT: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_READY_STARTED: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_READY_PENDING: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_READY_UNAVAILABLE: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_VALIDATION_QUEUED: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_RUNTIME_STARTED: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_RUNTIME_FAILED: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_RUNTIME_CANCELLED: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_RUNTIME_STALE: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_FOCUS_NS_TOTAL: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_FOCUS_COUNT: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_WIDGET_HIT_TEST_NS_TOTAL: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_WIDGET_HIT_TEST_COUNT: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_READY_SOURCE_NS_TOTAL: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_READY_SOURCE_COUNT: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_RUNTIME_START_NS_TOTAL: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_RUNTIME_START_COUNT: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_START_TOTAL_NS_TOTAL: AtomicU64 = AtomicU64::new(0);
static STARMAP_AUDITION_START_TOTAL_COUNT: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_app) enum StarmapAuditionCounter {
    DragBegin,
    DragUpdate,
    DragFinish,
    WidgetPointHit,
    WidgetPointMiss,
    WidgetSegmentHit,
    WidgetSegmentMiss,
    HitQueued,
    HitStarted,
    DuplicateActive,
    DuplicateQueued,
    ActiveReplaced,
    AdvanceScheduled,
    AdvanceStale,
    LoadedCurrent,
    ReadyStarted,
    ReadyPending,
    ReadyUnavailable,
    ValidationQueued,
    RuntimeStarted,
    RuntimeFailed,
    RuntimeCancelled,
    RuntimeStale,
}

impl StarmapAuditionCounter {
    fn as_str(self) -> &'static str {
        match self {
            Self::DragBegin => "drag_begin",
            Self::DragUpdate => "drag_update",
            Self::DragFinish => "drag_finish",
            Self::WidgetPointHit => "widget_point_hit",
            Self::WidgetPointMiss => "widget_point_miss",
            Self::WidgetSegmentHit => "widget_segment_hit",
            Self::WidgetSegmentMiss => "widget_segment_miss",
            Self::HitQueued => "hit_queued",
            Self::HitStarted => "hit_started",
            Self::DuplicateActive => "duplicate_active",
            Self::DuplicateQueued => "duplicate_queued",
            Self::ActiveReplaced => "active_replaced",
            Self::AdvanceScheduled => "advance_scheduled",
            Self::AdvanceStale => "advance_stale",
            Self::LoadedCurrent => "loaded_current",
            Self::ReadyStarted => "ready_started",
            Self::ReadyPending => "ready_pending",
            Self::ReadyUnavailable => "ready_unavailable",
            Self::ValidationQueued => "validation_queued",
            Self::RuntimeStarted => "runtime_started",
            Self::RuntimeFailed => "runtime_failed",
            Self::RuntimeCancelled => "runtime_cancelled",
            Self::RuntimeStale => "runtime_stale",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_app) enum StarmapAuditionDuration {
    Focus,
    WidgetHitTest,
    ReadySource,
    RuntimeStart,
    StartTotal,
}

pub(in crate::native_app) fn enabled() -> bool {
    *STARMAP_AUDITION_TELEMETRY_ENABLED.get_or_init(|| env_var_truthy(HOTPATH_TELEMETRY_ENV))
}

pub(in crate::native_app) fn stage_timer() -> Option<Instant> {
    enabled().then(Instant::now)
}

pub(in crate::native_app) fn elapsed_since(started_at: Option<Instant>) -> Option<Duration> {
    started_at.map(|started_at| started_at.elapsed())
}

pub(in crate::native_app) fn record_duration(counter: StarmapAuditionDuration, duration: Duration) {
    if !enabled() {
        return;
    }
    match counter {
        StarmapAuditionDuration::Focus => {
            add_duration_ns(&STARMAP_AUDITION_FOCUS_NS_TOTAL, duration);
            STARMAP_AUDITION_FOCUS_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionDuration::WidgetHitTest => {
            add_duration_ns(&STARMAP_AUDITION_WIDGET_HIT_TEST_NS_TOTAL, duration);
            STARMAP_AUDITION_WIDGET_HIT_TEST_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionDuration::ReadySource => {
            add_duration_ns(&STARMAP_AUDITION_READY_SOURCE_NS_TOTAL, duration);
            STARMAP_AUDITION_READY_SOURCE_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionDuration::RuntimeStart => {
            add_duration_ns(&STARMAP_AUDITION_RUNTIME_START_NS_TOTAL, duration);
            STARMAP_AUDITION_RUNTIME_START_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionDuration::StartTotal => {
            add_duration_ns(&STARMAP_AUDITION_START_TOTAL_NS_TOTAL, duration);
            STARMAP_AUDITION_START_TOTAL_COUNT.fetch_add(1, Ordering::Relaxed);
        }
    }
}

pub(in crate::native_app) fn record_event(
    counter: Option<StarmapAuditionCounter>,
    stage: &'static str,
    outcome: &'static str,
    path: Option<&str>,
    hit_count: usize,
    queue_len: usize,
    active: bool,
    elapsed: Option<Duration>,
) {
    if !enabled() {
        return;
    }
    let sample_tick = STARMAP_AUDITION_EVENTS_TOTAL.fetch_add(1, Ordering::Relaxed) + 1;
    let counter_name = counter.map(StarmapAuditionCounter::as_str).unwrap_or("");
    if let Some(counter) = counter {
        record_counter(counter);
    }
    tracing::info!(
        target: "perf::starmap_drag",
        module = "starmap_audition",
        stage,
        outcome,
        counter = counter_name,
        path = path.unwrap_or_default(),
        hit_count,
        queue_len,
        active,
        elapsed_ms = elapsed.map(duration_ms).unwrap_or(0.0),
        "Starmap audition telemetry event"
    );
    maybe_emit_starmap_audition_telemetry(sample_tick);
}

fn record_counter(counter: StarmapAuditionCounter) {
    match counter {
        StarmapAuditionCounter::DragBegin => {
            STARMAP_AUDITION_DRAG_BEGIN.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::DragUpdate => {
            STARMAP_AUDITION_DRAG_UPDATE.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::DragFinish => {
            STARMAP_AUDITION_DRAG_FINISH.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::WidgetPointHit => {
            STARMAP_AUDITION_WIDGET_POINT_HIT.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::WidgetPointMiss => {
            STARMAP_AUDITION_WIDGET_POINT_MISS.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::WidgetSegmentHit => {
            STARMAP_AUDITION_WIDGET_SEGMENT_HIT.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::WidgetSegmentMiss => {
            STARMAP_AUDITION_WIDGET_SEGMENT_MISS.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::HitQueued => {
            STARMAP_AUDITION_HITS_QUEUED.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::HitStarted => {
            STARMAP_AUDITION_HITS_STARTED.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::DuplicateActive => {
            STARMAP_AUDITION_DUPLICATE_ACTIVE.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::DuplicateQueued => {
            STARMAP_AUDITION_DUPLICATE_QUEUED.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::ActiveReplaced => {
            STARMAP_AUDITION_ACTIVE_REPLACED.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::AdvanceScheduled => {
            STARMAP_AUDITION_ADVANCE_SCHEDULED.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::AdvanceStale => {
            STARMAP_AUDITION_ADVANCE_STALE.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::LoadedCurrent => {
            STARMAP_AUDITION_LOADED_CURRENT.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::ReadyStarted => {
            STARMAP_AUDITION_READY_STARTED.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::ReadyPending => {
            STARMAP_AUDITION_READY_PENDING.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::ReadyUnavailable => {
            STARMAP_AUDITION_READY_UNAVAILABLE.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::ValidationQueued => {
            STARMAP_AUDITION_VALIDATION_QUEUED.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::RuntimeStarted => {
            STARMAP_AUDITION_RUNTIME_STARTED.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::RuntimeFailed => {
            STARMAP_AUDITION_RUNTIME_FAILED.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::RuntimeCancelled => {
            STARMAP_AUDITION_RUNTIME_CANCELLED.fetch_add(1, Ordering::Relaxed);
        }
        StarmapAuditionCounter::RuntimeStale => {
            STARMAP_AUDITION_RUNTIME_STALE.fetch_add(1, Ordering::Relaxed);
        }
    }
}

fn maybe_emit_starmap_audition_telemetry(sample_tick: u64) {
    if !should_emit(sample_tick, STARMAP_AUDITION_TELEMETRY_LOG_EVERY) {
        return;
    }
    let focus_count = STARMAP_AUDITION_FOCUS_COUNT.load(Ordering::Relaxed).max(1);
    let widget_hit_test_count = STARMAP_AUDITION_WIDGET_HIT_TEST_COUNT
        .load(Ordering::Relaxed)
        .max(1);
    let ready_source_count = STARMAP_AUDITION_READY_SOURCE_COUNT
        .load(Ordering::Relaxed)
        .max(1);
    let runtime_start_count = STARMAP_AUDITION_RUNTIME_START_COUNT
        .load(Ordering::Relaxed)
        .max(1);
    let start_total_count = STARMAP_AUDITION_START_TOTAL_COUNT
        .load(Ordering::Relaxed)
        .max(1);

    tracing::info!(
        target: "perf::hotpath",
        module = "starmap_audition",
        events_total = STARMAP_AUDITION_EVENTS_TOTAL.load(Ordering::Relaxed),
        drag_begin = STARMAP_AUDITION_DRAG_BEGIN.load(Ordering::Relaxed),
        drag_update = STARMAP_AUDITION_DRAG_UPDATE.load(Ordering::Relaxed),
        drag_finish = STARMAP_AUDITION_DRAG_FINISH.load(Ordering::Relaxed),
        widget_point_hit = STARMAP_AUDITION_WIDGET_POINT_HIT.load(Ordering::Relaxed),
        widget_point_miss = STARMAP_AUDITION_WIDGET_POINT_MISS.load(Ordering::Relaxed),
        widget_segment_hit = STARMAP_AUDITION_WIDGET_SEGMENT_HIT.load(Ordering::Relaxed),
        widget_segment_miss = STARMAP_AUDITION_WIDGET_SEGMENT_MISS.load(Ordering::Relaxed),
        hits_queued = STARMAP_AUDITION_HITS_QUEUED.load(Ordering::Relaxed),
        hits_started = STARMAP_AUDITION_HITS_STARTED.load(Ordering::Relaxed),
        duplicate_active = STARMAP_AUDITION_DUPLICATE_ACTIVE.load(Ordering::Relaxed),
        duplicate_queued = STARMAP_AUDITION_DUPLICATE_QUEUED.load(Ordering::Relaxed),
        active_replaced = STARMAP_AUDITION_ACTIVE_REPLACED.load(Ordering::Relaxed),
        advance_scheduled = STARMAP_AUDITION_ADVANCE_SCHEDULED.load(Ordering::Relaxed),
        advance_stale = STARMAP_AUDITION_ADVANCE_STALE.load(Ordering::Relaxed),
        loaded_current = STARMAP_AUDITION_LOADED_CURRENT.load(Ordering::Relaxed),
        ready_started = STARMAP_AUDITION_READY_STARTED.load(Ordering::Relaxed),
        ready_pending = STARMAP_AUDITION_READY_PENDING.load(Ordering::Relaxed),
        ready_unavailable = STARMAP_AUDITION_READY_UNAVAILABLE.load(Ordering::Relaxed),
        validation_queued = STARMAP_AUDITION_VALIDATION_QUEUED.load(Ordering::Relaxed),
        runtime_started = STARMAP_AUDITION_RUNTIME_STARTED.load(Ordering::Relaxed),
        runtime_failed = STARMAP_AUDITION_RUNTIME_FAILED.load(Ordering::Relaxed),
        runtime_cancelled = STARMAP_AUDITION_RUNTIME_CANCELLED.load(Ordering::Relaxed),
        runtime_stale = STARMAP_AUDITION_RUNTIME_STALE.load(Ordering::Relaxed),
        avg_focus_ms = avg_ms(
            STARMAP_AUDITION_FOCUS_NS_TOTAL.load(Ordering::Relaxed),
            focus_count
        ),
        avg_widget_hit_test_ms = avg_ms(
            STARMAP_AUDITION_WIDGET_HIT_TEST_NS_TOTAL.load(Ordering::Relaxed),
            widget_hit_test_count
        ),
        avg_ready_source_ms = avg_ms(
            STARMAP_AUDITION_READY_SOURCE_NS_TOTAL.load(Ordering::Relaxed),
            ready_source_count
        ),
        avg_runtime_start_ms = avg_ms(
            STARMAP_AUDITION_RUNTIME_START_NS_TOTAL.load(Ordering::Relaxed),
            runtime_start_count
        ),
        avg_start_total_ms = avg_ms(
            STARMAP_AUDITION_START_TOTAL_NS_TOTAL.load(Ordering::Relaxed),
            start_total_count
        ),
        "Starmap audition telemetry snapshot"
    );
}

fn avg_ms(total_ns: u64, count: u64) -> f64 {
    total_ns as f64 / count.max(1) as f64 / 1_000_000.0
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn add_duration_ns(counter: &AtomicU64, duration: Duration) {
    let dur_ns = duration.as_nanos().min(u64::MAX as u128) as u64;
    counter.fetch_add(dur_ns, Ordering::Relaxed);
}

fn should_emit(sample_tick: u64, every: u64) -> bool {
    sample_tick != 0 && sample_tick.is_multiple_of(every)
}

fn env_var_truthy(key: &str) -> bool {
    std::env::var(key)
        .ok()
        .is_some_and(|value| is_truthy(&value))
}

fn is_truthy(value: &str) -> bool {
    let normalized = value.trim();
    normalized.eq_ignore_ascii_case("1")
        || normalized.eq_ignore_ascii_case("true")
        || normalized.eq_ignore_ascii_case("yes")
        || normalized.eq_ignore_ascii_case("on")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starmap_audition_counter_names_are_stable() {
        assert_eq!(StarmapAuditionCounter::HitQueued.as_str(), "hit_queued");
        assert_eq!(
            StarmapAuditionCounter::ReadyUnavailable.as_str(),
            "ready_unavailable"
        );
    }

    #[test]
    fn starmap_audition_duration_ms_uses_fractional_precision() {
        assert_eq!(duration_ms(Duration::from_micros(1_500)), 1.5);
    }

    #[test]
    fn starmap_audition_should_emit_requires_non_zero_multiple() {
        assert!(!should_emit(0, 32));
        assert!(!should_emit(31, 32));
        assert!(should_emit(32, 32));
    }

    #[test]
    fn starmap_audition_truthy_parser_accepts_common_tokens() {
        assert!(is_truthy("1"));
        assert!(is_truthy(" true "));
        assert!(is_truthy("YES"));
        assert!(is_truthy("on"));
        assert!(!is_truthy("0"));
        assert!(!is_truthy("false"));
    }
}
