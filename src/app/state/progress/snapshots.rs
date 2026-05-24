/// Snapshot of analysis progress for display.
#[derive(Clone, Debug)]
pub struct AnalysisProgressSnapshot {
    /// Number of pending jobs.
    pub pending: usize,
    /// Number of running jobs.
    pub running: usize,
    /// Number of failed jobs.
    pub failed: usize,
    /// Completed samples count.
    pub samples_completed: usize,
    /// Total samples to process.
    pub samples_total: usize,
    /// Snapshot of running jobs.
    pub running_jobs: Vec<RunningJobSnapshot>,
    /// Staleness threshold in seconds.
    pub stale_after_secs: Option<i64>,
}

/// Summary of a running job heartbeat for display.
#[derive(Clone, Debug)]
pub struct RunningJobSnapshot {
    /// Human-readable job label.
    pub label: String,
    /// Last heartbeat timestamp, epoch seconds.
    pub last_heartbeat_at: Option<i64>,
    /// Whether the job appears stalled.
    pub possibly_stalled: bool,
}

impl RunningJobSnapshot {
    /// Build a snapshot and mark it stalled when the heartbeat is stale.
    pub fn from_heartbeat(
        label: String,
        last_heartbeat_at: Option<i64>,
        stale_after_secs: Option<i64>,
        now_epoch: Option<i64>,
    ) -> Self {
        let possibly_stalled = match (last_heartbeat_at, stale_after_secs, now_epoch) {
            (Some(heartbeat), Some(stale_after), Some(now)) => {
                now.saturating_sub(heartbeat) >= stale_after
            }
            _ => false,
        };
        Self {
            label,
            last_heartbeat_at,
            possibly_stalled,
        }
    }
}
