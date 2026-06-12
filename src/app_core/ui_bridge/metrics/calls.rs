//! Call counters for app-model, motion, and action bridge entrypoints.

#[cfg(feature = "native-bridge-metrics")]
use super::registry::BRIDGE_METRICS;
#[cfg(feature = "native-bridge-metrics")]
use std::sync::atomic::Ordering;
#[cfg(not(feature = "native-bridge-metrics"))]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(not(feature = "native-bridge-metrics"))]
static FALLBACK_PULL_MODEL_CALL_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(not(feature = "native-bridge-metrics"))]
static FALLBACK_PULL_MOTION_CALL_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(not(feature = "native-bridge-metrics"))]
static FALLBACK_ACTION_CALL_COUNT: AtomicU64 = AtomicU64::new(0);

#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
fn trace_fallback_call(counter: &AtomicU64) -> u64 {
    counter.fetch_add(1, Ordering::Relaxed) + 1
}

#[cfg(feature = "native-bridge-metrics")]
/// Count one app-model bridge projection call.
pub(in crate::app_core::ui_bridge) fn trace_pull_model_call() -> u64 {
    BRIDGE_METRICS
        .pull_model_count
        .fetch_add(1, Ordering::Relaxed)
        + 1
}

#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// Count one app-model bridge projection call in fallback builds.
pub(in crate::app_core::ui_bridge) fn trace_pull_model_call() -> u64 {
    trace_fallback_call(&FALLBACK_PULL_MODEL_CALL_COUNT)
}

#[cfg(feature = "native-bridge-metrics")]
/// Count one motion bridge projection call.
pub(in crate::app_core::ui_bridge) fn trace_pull_motion_call() -> u64 {
    BRIDGE_METRICS
        .pull_motion_count
        .fetch_add(1, Ordering::Relaxed)
        + 1
}

#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// Count one motion bridge projection call in fallback builds.
pub(in crate::app_core::ui_bridge) fn trace_pull_motion_call() -> u64 {
    trace_fallback_call(&FALLBACK_PULL_MOTION_CALL_COUNT)
}

#[cfg(feature = "native-bridge-metrics")]
/// Count one action bridge dispatch call.
pub(in crate::app_core::ui_bridge) fn trace_action_call() -> u64 {
    BRIDGE_METRICS.action_count.fetch_add(1, Ordering::Relaxed) + 1
}

#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// Count one action bridge dispatch call in fallback builds.
pub(in crate::app_core::ui_bridge) fn trace_action_call() -> u64 {
    trace_fallback_call(&FALLBACK_ACTION_CALL_COUNT)
}

#[cfg(test)]
mod tests {
    use super::{trace_action_call, trace_pull_model_call, trace_pull_motion_call};

    fn assert_monotonic_increase(mut trace_call: impl FnMut() -> u64) {
        let first = trace_call();
        let second = trace_call();
        let third = trace_call();

        assert!(second > first);
        assert!(third > second);
    }

    #[test]
    fn pull_model_call_trace_is_monotonic() {
        assert_monotonic_increase(trace_pull_model_call);
    }

    #[test]
    fn pull_motion_call_trace_is_monotonic() {
        assert_monotonic_increase(trace_pull_motion_call);
    }

    #[test]
    fn action_call_trace_is_monotonic() {
        assert_monotonic_increase(trace_action_call);
    }
}
