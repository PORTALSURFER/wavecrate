use std::time::{Duration, Instant};

const BULK_NORMALIZATION_PACE_INTERVAL: Duration = Duration::from_millis(16);
const BULK_NORMALIZATION_PACE_SLEEP: Duration = Duration::from_millis(1);

pub(super) struct NormalizationWorkerPacer {
    enabled: bool,
    last_pause: Instant,
}

impl NormalizationWorkerPacer {
    pub(super) fn new(enabled: bool) -> Self {
        Self {
            enabled,
            last_pause: Instant::now(),
        }
    }

    pub(super) fn pause_if_due(&mut self) {
        if !self.enabled || self.last_pause.elapsed() < BULK_NORMALIZATION_PACE_INTERVAL {
            return;
        }
        std::thread::sleep(BULK_NORMALIZATION_PACE_SLEEP);
        self.last_pause = Instant::now();
    }
}
