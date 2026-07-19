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
        self.update_performance_governor_at(Instant::now(), user_active);
    }

    fn update_performance_governor_at(&mut self, now: Instant, user_active: bool) {
        self.observe_frame_timing_for_fps(now, user_active);
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
