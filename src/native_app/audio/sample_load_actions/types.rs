use radiant::prelude as ui;
use std::{
    path::Path,
    time::{Duration, Instant},
};

pub(in crate::native_app) struct NormalizedWaveformReload<'a> {
    pub(in crate::native_app) path: &'a Path,
    pub(in crate::native_app) playback: Option<WaveformPlaybackResume>,
}

pub(in crate::native_app) struct WaveformPlaybackResume {
    pub(in crate::native_app) start_ratio: f32,
    pub(in crate::native_app) span: Option<(f32, f32)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SampleLoadStrategy {
    Decode,
    PersistedPlaybackCacheOnly,
    PreferPersistedPlaybackCache,
}

#[derive(Clone, Debug)]
pub(super) struct SampleLoadRequest {
    path: String,
    autoplay: bool,
    priority: ui::TaskPriority,
    strategy: SampleLoadStrategy,
    queued_at: Instant,
}

impl SampleLoadRequest {
    pub(super) fn new(
        path: String,
        autoplay: bool,
        priority: ui::TaskPriority,
        strategy: SampleLoadStrategy,
    ) -> Self {
        Self {
            path,
            autoplay,
            priority,
            strategy,
            queued_at: Instant::now(),
        }
    }

    pub(super) fn path(&self) -> &str {
        self.path.as_str()
    }

    pub(super) fn into_path(self) -> String {
        self.path
    }

    pub(super) fn autoplay(&self) -> bool {
        self.autoplay
    }

    pub(super) fn priority(&self) -> ui::TaskPriority {
        self.priority
    }

    pub(super) fn strategy(&self) -> SampleLoadStrategy {
        self.strategy
    }

    pub(super) fn queue_wait(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.queued_at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_load_request_preserves_worker_plan_inputs() {
        let request = SampleLoadRequest::new(
            String::from("kick.wav"),
            true,
            ui::TaskPriority::Interactive,
            SampleLoadStrategy::PreferPersistedPlaybackCache,
        );

        assert_eq!(request.path(), "kick.wav");
        assert!(request.autoplay());
        assert_eq!(request.priority(), ui::TaskPriority::Interactive);
        assert_eq!(
            request.strategy(),
            SampleLoadStrategy::PreferPersistedPlaybackCache
        );
    }
}
