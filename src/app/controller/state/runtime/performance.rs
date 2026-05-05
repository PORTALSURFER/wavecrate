use std::time::Duration;

pub(crate) struct PerformanceGovernorState {
    /// Last user interaction timestamp used for governor hysteresis.
    pub(crate) last_user_activity_at: Option<std::time::Instant>,
    /// Most recent slow-frame timestamp used to raise worker priority.
    pub(crate) last_slow_frame_at: Option<std::time::Instant>,
    /// Last frame timestamp for inter-frame interval sampling.
    pub(crate) last_frame_at: Option<std::time::Instant>,
    /// Smoothed frame time in milliseconds.
    pub(crate) avg_frame_ms: f64,
    /// Number of valid frame samples captured so far.
    pub(crate) frame_sample_count: u32,
    pub(crate) last_worker_count: Option<u32>,
    pub(crate) idle_worker_override: Option<u32>,
}

impl PerformanceGovernorState {
    pub(crate) fn new() -> Self {
        Self {
            last_user_activity_at: None,
            last_slow_frame_at: None,
            last_frame_at: None,
            avg_frame_ms: 0.0,
            frame_sample_count: 0,
            last_worker_count: None,
            idle_worker_override: None,
        }
    }

    /// Update moving-frame metrics from one inter-frame duration sample.
    ///
    /// Uses an EWMA-style filter to keep short-term spikes from dominating the average.
    pub(crate) fn observe_frame_interval(&mut self, frame_interval: Duration) {
        let frame_ms = frame_interval.as_secs_f64() * 1_000.0;
        if frame_ms <= 0.0 {
            return;
        }
        if self.frame_sample_count == 0 {
            self.avg_frame_ms = frame_ms;
            self.frame_sample_count = 1;
            return;
        }
        const FRAME_RATE_ALPHA: f64 = 0.2;
        self.avg_frame_ms = self
            .avg_frame_ms
            .mul_add(1.0 - FRAME_RATE_ALPHA, frame_ms * FRAME_RATE_ALPHA);
        self.frame_sample_count = self.frame_sample_count.saturating_add(1);
    }

    /// Return the averaged frame rate across collected frame-time samples.
    pub(crate) fn average_fps(&self) -> Option<f64> {
        if self.avg_frame_ms <= 0.0 || self.frame_sample_count == 0 {
            return None;
        }
        Some(1_000.0 / self.avg_frame_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::PerformanceGovernorState;
    use std::time::Duration;

    #[test]
    fn average_fps_is_none_before_samples() {
        let state = PerformanceGovernorState::new();
        assert!(state.average_fps().is_none());
        assert_eq!(state.frame_sample_count, 0);
        assert_eq!(state.avg_frame_ms, 0.0);
    }

    #[test]
    fn observe_frame_interval_initializes_average() {
        let mut state = PerformanceGovernorState::new();
        state.observe_frame_interval(Duration::from_millis(16));
        assert_eq!(state.frame_sample_count, 1);
        assert_eq!(state.avg_frame_ms, 16.0);
        assert!((state.average_fps().expect("fps") - 62.5).abs() < f64::EPSILON);
    }

    #[test]
    fn observe_frame_interval_skips_non_positive_samples() {
        let mut state = PerformanceGovernorState::new();
        state.observe_frame_interval(Duration::ZERO);
        state.observe_frame_interval(Duration::from_nanos(500));
        assert_eq!(state.frame_sample_count, 1);
        assert!(state.avg_frame_ms > 0.0);
    }

    #[test]
    fn observe_frame_interval_uses_ewma_update() {
        let mut state = PerformanceGovernorState::new();
        state.observe_frame_interval(Duration::from_millis(10));
        state.observe_frame_interval(Duration::from_millis(20));
        let expected = 12.0;
        assert!((state.avg_frame_ms - expected).abs() < 1e-9);
        assert_eq!(state.frame_sample_count, 2);
    }
}
