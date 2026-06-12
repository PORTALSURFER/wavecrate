//! Duration accumulators for bridge preparation, projection, and actions.

#[cfg(feature = "native-bridge-metrics")]
use super::registry::{BRIDGE_METRICS, saturating_add_duration};
use std::time::Duration;

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track app-model preparation time before projection.
pub(in crate::app_core::ui_bridge) fn trace_pull_model_preparation(duration: Duration) {
    saturating_add_duration(&BRIDGE_METRICS.pull_model_prep_ns, duration);
}

#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// Ignore app-model preparation time when bridge profiling is compiled out.
pub(in crate::app_core::ui_bridge) fn trace_pull_model_preparation(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track app-model projection time.
pub(in crate::app_core::ui_bridge) fn trace_pull_model_projection(duration: Duration) {
    saturating_add_duration(&BRIDGE_METRICS.pull_model_project_ns, duration);
}

#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// Ignore app-model projection time when bridge profiling is compiled out.
pub(in crate::app_core::ui_bridge) fn trace_pull_model_projection(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track motion preparation time before projection.
pub(in crate::app_core::ui_bridge) fn trace_pull_motion_preparation(duration: Duration) {
    saturating_add_duration(&BRIDGE_METRICS.pull_motion_prep_ns, duration);
}

#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// Ignore motion preparation time when bridge profiling is compiled out.
pub(in crate::app_core::ui_bridge) fn trace_pull_motion_preparation(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track motion projection time.
pub(in crate::app_core::ui_bridge) fn trace_pull_motion_projection(duration: Duration) {
    saturating_add_duration(&BRIDGE_METRICS.pull_motion_project_ns, duration);
}

#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// Ignore motion projection time when bridge profiling is compiled out.
pub(in crate::app_core::ui_bridge) fn trace_pull_motion_projection(_duration: Duration) {}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track one action bridge dispatch duration.
pub(in crate::app_core::ui_bridge) fn trace_action_duration(duration: Duration) {
    saturating_add_duration(&BRIDGE_METRICS.action_duration_ns, duration);
}

#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// Ignore action dispatch duration when bridge profiling is compiled out.
pub(in crate::app_core::ui_bridge) fn trace_action_duration(_duration: Duration) {}

#[cfg(all(test, not(feature = "native-bridge-metrics")))]
mod tests {
    use super::{
        trace_action_duration, trace_pull_model_preparation, trace_pull_model_projection,
        trace_pull_motion_preparation, trace_pull_motion_projection,
    };
    use std::time::Duration;

    #[test]
    fn disabled_feature_duration_tracers_are_noops() {
        let duration = Duration::from_nanos(u64::MAX);

        trace_pull_model_preparation(duration);
        trace_pull_model_projection(duration);
        trace_pull_motion_preparation(duration);
        trace_pull_motion_projection(duration);
        trace_action_duration(duration);
    }
}
