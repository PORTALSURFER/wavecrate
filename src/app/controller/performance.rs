//! Controller-side frame timing and performance governor helpers.

use super::*;

impl AppController {
    fn observe_frame_timing_for_fps(&mut self, now: Instant, user_active: bool) {
        const SLOW_FRAME_THRESHOLD: Duration = Duration::from_millis(40);
        if let Some(last_frame) = self.runtime.performance.last_frame_at {
            let frame_delta = now.saturating_duration_since(last_frame);
            self.runtime.performance.observe_frame_interval(frame_delta);
            if frame_delta >= SLOW_FRAME_THRESHOLD {
                self.runtime.performance.last_slow_frame_at = Some(now);
            }
        }
        self.runtime.performance.last_frame_at = Some(now);
        if user_active {
            self.runtime.performance.last_user_activity_at = Some(now);
        }
    }

    pub(crate) fn update_performance_governor(&mut self, user_active: bool) {
        const ACTIVE_WINDOW: Duration = Duration::from_millis(300);
        const IDLE_WINDOW: Duration = Duration::from_secs(2);
        let now = Instant::now();
        self.observe_frame_timing_for_fps(now, user_active);
        let recent_input = self
            .runtime
            .performance
            .last_user_activity_at
            .is_some_and(|time| now.saturating_duration_since(time) <= ACTIVE_WINDOW);
        let recent_slow_frame = self
            .runtime
            .performance
            .last_slow_frame_at
            .is_some_and(|time| now.saturating_duration_since(time) <= ACTIVE_WINDOW);
        let busy = self.is_playing() || recent_input || recent_slow_frame;
        let analysis_active = self
            .ui
            .progress
            .analysis
            .as_ref()
            .is_some_and(|snapshot| snapshot.pending > 0 || snapshot.running > 0);
        let pause_claiming = (self.is_playing() || recent_input) && !analysis_active;
        let last_activity_at = match (
            self.runtime.performance.last_user_activity_at,
            self.runtime.performance.last_slow_frame_at,
        ) {
            (Some(input), Some(slow)) => Some(input.max(slow)),
            (Some(input), None) => Some(input),
            (None, Some(slow)) => Some(slow),
            (None, None) => None,
        };
        let idle = !self.is_playing()
            && last_activity_at
                .is_some_and(|time| now.saturating_duration_since(time) >= IDLE_WINDOW);
        let base_worker_count = if self.settings.analysis.analysis_worker_count == 0 {
            crate::app::controller::library::analysis_jobs::default_worker_count()
        } else {
            self.settings.analysis.analysis_worker_count
        };
        let idle_target = self
            .runtime
            .performance
            .idle_worker_override
            .unwrap_or(base_worker_count);
        let target = if busy || !idle { 1 } else { idle_target };
        if pause_claiming {
            self.runtime.analysis.pause_claiming();
        } else {
            self.runtime.analysis.resume_claiming();
        }
        if self.runtime.performance.last_worker_count != Some(target) {
            self.runtime.analysis.set_worker_count(target);
            self.runtime.performance.last_worker_count = Some(target);
        }
    }

    /// Record the latest inter-frame timing sample used by the FPS counter.
    pub(crate) fn record_frame_timing_for_fps(&mut self) {
        let now = Instant::now();
        self.observe_frame_timing_for_fps(now, false);
    }

    /// Current exponentially weighted average FPS estimated from recent frame intervals.
    pub(crate) fn average_fps(&self) -> Option<f64> {
        self.runtime.performance.average_fps()
    }
}
