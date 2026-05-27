use crate::app_core::actions::NativeFrameBuildResult;

#[cfg(feature = "native-bridge-metrics")]
use super::registry::BRIDGE_METRICS;
#[cfg(feature = "native-bridge-metrics")]
use std::sync::atomic::Ordering;

#[cfg(feature = "native-bridge-metrics")]
pub(in crate::app_core::ui_bridge) fn trace_frame_result(result: &NativeFrameBuildResult) -> u64 {
    let frame_count = BRIDGE_METRICS
        .frame_result_count
        .fetch_add(1, Ordering::Relaxed)
        + 1;
    if result.needs_animation {
        BRIDGE_METRICS
            .frame_result_animation_count
            .fetch_add(1, Ordering::Relaxed);
    }
    if result.presented {
        BRIDGE_METRICS
            .frame_result_presented_count
            .fetch_add(1, Ordering::Relaxed);
    }
    if result.missed_present {
        BRIDGE_METRICS
            .frame_result_missed_present_count
            .fetch_add(1, Ordering::Relaxed);
    }
    if result.jank {
        BRIDGE_METRICS
            .frame_result_jank_count
            .fetch_add(1, Ordering::Relaxed);
    }
    BRIDGE_METRICS
        .frame_result_total_us
        .fetch_add(result.frame_total_us as u64, Ordering::Relaxed);
    BRIDGE_METRICS
        .frame_result_present_us_total
        .fetch_add(result.present_us as u64, Ordering::Relaxed);
    BRIDGE_METRICS
        .frame_result_frame_budget_us
        .store(result.frame_budget_us as u64, Ordering::Relaxed);
    BRIDGE_METRICS
        .frame_result_primitives_total
        .fetch_add(result.primitive_count as u64, Ordering::Relaxed);
    BRIDGE_METRICS
        .frame_result_text_runs_total
        .fetch_add(result.text_run_count as u64, Ordering::Relaxed);
    if result.layout_rebuild {
        BRIDGE_METRICS
            .frame_result_layout_rebuild_count
            .fetch_add(1, Ordering::Relaxed);
    }
    if result.static_rebuild {
        BRIDGE_METRICS
            .frame_result_static_rebuild_count
            .fetch_add(1, Ordering::Relaxed);
    }
    if result.state_overlay_rebuild {
        BRIDGE_METRICS
            .frame_result_state_overlay_rebuild_count
            .fetch_add(1, Ordering::Relaxed);
    }
    if result.motion_overlay_rebuild {
        BRIDGE_METRICS
            .frame_result_motion_overlay_rebuild_count
            .fetch_add(1, Ordering::Relaxed);
    }
    if !result.static_rebuild && (result.state_overlay_rebuild || result.motion_overlay_rebuild) {
        BRIDGE_METRICS
            .frame_result_overlay_only_count
            .fetch_add(1, Ordering::Relaxed);
    }
    frame_count
}

#[cfg(not(feature = "native-bridge-metrics"))]
#[inline(always)]
pub(in crate::app_core::ui_bridge) fn trace_frame_result(_result: &NativeFrameBuildResult) -> u64 {
    1
}

#[cfg(all(test, feature = "native-bridge-metrics"))]
mod tests {
    use super::super::registry::BRIDGE_METRICS;
    use super::trace_frame_result;
    use crate::app_core::actions::NativeFrameBuildResult;
    use std::sync::atomic::Ordering;

    #[test]
    fn frame_result_trace_counts_rebuild_attribution() {
        let before_layout = BRIDGE_METRICS
            .frame_result_layout_rebuild_count
            .load(Ordering::Relaxed);
        let before_static = BRIDGE_METRICS
            .frame_result_static_rebuild_count
            .load(Ordering::Relaxed);
        let before_state_overlay = BRIDGE_METRICS
            .frame_result_state_overlay_rebuild_count
            .load(Ordering::Relaxed);
        let before_motion_overlay = BRIDGE_METRICS
            .frame_result_motion_overlay_rebuild_count
            .load(Ordering::Relaxed);
        let before_overlay_only = BRIDGE_METRICS
            .frame_result_overlay_only_count
            .load(Ordering::Relaxed);

        trace_frame_result(&NativeFrameBuildResult {
            layout_rebuild: true,
            static_rebuild: true,
            state_overlay_rebuild: true,
            motion_overlay_rebuild: false,
            ..NativeFrameBuildResult::default()
        });
        trace_frame_result(&NativeFrameBuildResult {
            layout_rebuild: false,
            static_rebuild: false,
            state_overlay_rebuild: false,
            motion_overlay_rebuild: true,
            ..NativeFrameBuildResult::default()
        });

        assert!(
            BRIDGE_METRICS
                .frame_result_layout_rebuild_count
                .load(Ordering::Relaxed)
                >= before_layout + 1
        );
        assert!(
            BRIDGE_METRICS
                .frame_result_static_rebuild_count
                .load(Ordering::Relaxed)
                >= before_static + 1
        );
        assert!(
            BRIDGE_METRICS
                .frame_result_state_overlay_rebuild_count
                .load(Ordering::Relaxed)
                >= before_state_overlay + 1
        );
        assert!(
            BRIDGE_METRICS
                .frame_result_motion_overlay_rebuild_count
                .load(Ordering::Relaxed)
                >= before_motion_overlay + 1
        );
        assert!(
            BRIDGE_METRICS
                .frame_result_overlay_only_count
                .load(Ordering::Relaxed)
                >= before_overlay_only + 1
        );
    }
}
