//! Classified interaction action counters and timing accumulators.

use super::super::action_classification::InteractionActionClass;
#[cfg(feature = "native-bridge-metrics")]
use super::registry::{BRIDGE_METRICS, saturating_add_duration};
#[cfg(feature = "native-bridge-metrics")]
use std::sync::atomic::Ordering;
use std::time::Duration;

#[cfg(any(test, feature = "native-bridge-metrics"))]
#[cfg_attr(test, derive(Clone, Copy, Debug, Eq, PartialEq))]
enum InteractionMetric {
    Wheel,
    MapPanProxy,
    Waveform,
    Volume,
}

#[cfg(any(test, feature = "native-bridge-metrics"))]
fn interaction_metric(kind: InteractionActionClass) -> InteractionMetric {
    match kind {
        InteractionActionClass::Wheel => InteractionMetric::Wheel,
        InteractionActionClass::MapPanProxy => InteractionMetric::MapPanProxy,
        InteractionActionClass::Waveform => InteractionMetric::Waveform,
        InteractionActionClass::Volume => InteractionMetric::Volume,
    }
}

#[cfg(feature = "native-bridge-metrics")]
#[inline(always)]
/// Track classified interaction action timings for bridge profiling logs.
pub(in crate::app_core::ui_bridge) fn trace_action_interaction(
    kind: InteractionActionClass,
    duration: Duration,
) {
    match interaction_metric(kind) {
        InteractionMetric::Wheel => {
            BRIDGE_METRICS
                .action_wheel_count
                .fetch_add(1, Ordering::Relaxed);
            saturating_add_duration(&BRIDGE_METRICS.action_wheel_duration_ns, duration);
        }
        InteractionMetric::MapPanProxy => {
            BRIDGE_METRICS
                .action_map_proxy_count
                .fetch_add(1, Ordering::Relaxed);
            saturating_add_duration(&BRIDGE_METRICS.action_map_proxy_duration_ns, duration);
        }
        InteractionMetric::Waveform => {
            BRIDGE_METRICS
                .action_waveform_count
                .fetch_add(1, Ordering::Relaxed);
            saturating_add_duration(&BRIDGE_METRICS.action_waveform_duration_ns, duration);
        }
        InteractionMetric::Volume => {
            BRIDGE_METRICS
                .action_volume_count
                .fetch_add(1, Ordering::Relaxed);
            saturating_add_duration(&BRIDGE_METRICS.action_volume_duration_ns, duration);
        }
    }
}

#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
/// No-op classified interaction recorder for non-profiling builds.
pub(in crate::app_core::ui_bridge) fn trace_action_interaction(
    _kind: InteractionActionClass,
    _duration: Duration,
) {
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "native-bridge-metrics"))]
    use super::trace_action_interaction;
    use super::{InteractionMetric, interaction_metric};
    use crate::app_core::ui_bridge::action_classification::InteractionActionClass;
    #[cfg(not(feature = "native-bridge-metrics"))]
    use std::time::Duration;

    #[test]
    fn interaction_metric_mapping_covers_action_classes() {
        let cases = [
            (InteractionActionClass::Wheel, InteractionMetric::Wheel),
            (
                InteractionActionClass::MapPanProxy,
                InteractionMetric::MapPanProxy,
            ),
            (
                InteractionActionClass::Waveform,
                InteractionMetric::Waveform,
            ),
            (InteractionActionClass::Volume, InteractionMetric::Volume),
        ];

        for (kind, expected_metric) in cases {
            assert_eq!(interaction_metric(kind), expected_metric);
        }
    }

    #[cfg(not(feature = "native-bridge-metrics"))]
    #[test]
    fn disabled_feature_interaction_tracer_is_noop() {
        trace_action_interaction(InteractionActionClass::Wheel, Duration::from_millis(1));
        trace_action_interaction(InteractionActionClass::Waveform, Duration::from_millis(1));
    }
}
