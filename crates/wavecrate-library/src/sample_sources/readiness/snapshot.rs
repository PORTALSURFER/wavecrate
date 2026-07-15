use std::collections::BTreeMap;

use super::model::{ReadinessEligibility, ReadinessStage, ReadinessTarget, SourceAvailability};

/// Authoritative classification for one target at one reconciliation instant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadinessClassification {
    /// The exact artifact version and generations are present.
    Current,
    /// Work is required but has not been claimed.
    Pending,
    /// Matching work owns an unexpired lease.
    Running {
        /// Durable lease deadline.
        lease_expires_at: i64,
    },
    /// Work failed transiently or its lease expired.
    RetryableFailure {
        /// Earliest retry time.
        retry_at: i64,
        /// Stable diagnostic reason.
        reason: String,
    },
    /// The current generation cannot complete without a content or product change.
    PermanentFailure {
        /// Stable diagnostic reason.
        reason: String,
    },
    /// The stage is unsupported for this identity.
    Unsupported,
    /// The configured source is temporarily unavailable.
    Offline,
    /// The configured source is disabled.
    Disabled,
    /// Persisted work or artifacts belong to a different generation or version.
    StaleByGeneration,
    /// The identity is no longer part of the current eligible manifest.
    Deleted,
}

impl ReadinessClassification {
    pub(crate) fn is_actionable(&self, now: i64) -> bool {
        match self {
            Self::Pending | Self::StaleByGeneration => true,
            Self::RetryableFailure { retry_at, .. } => *retry_at <= now,
            _ => false,
        }
    }

    pub(crate) fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::PermanentFailure { .. } | Self::Unsupported | Self::Deleted
        )
    }
}

/// One classified readiness target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadinessEntry {
    /// Desired target.
    pub target: ReadinessTarget,
    /// Current durable classification.
    pub classification: ReadinessClassification,
}

/// One deduplicated unit of work required to converge a target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadinessDeficit {
    /// Desired target that needs work.
    pub target: ReadinessTarget,
    /// Classification that made the target actionable.
    pub reason: ReadinessClassification,
}

/// Aggregate counts for one readiness stage.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReadinessStageCounts {
    /// Exact current targets.
    pub current: usize,
    /// Pending targets.
    pub pending: usize,
    /// Running targets.
    pub running: usize,
    /// Retryable failures, including future retry deadlines.
    pub retryable: usize,
    /// Permanent failures.
    pub permanent: usize,
    /// Unsupported targets.
    pub unsupported: usize,
    /// Offline or disabled targets.
    pub offline: usize,
    /// Stale version or generation targets.
    pub stale: usize,
    /// Deleted identities retained for terminal diagnostics.
    pub deleted: usize,
}

/// High-level coordinator activity derived from the durable snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadinessActivity {
    /// No actionable, running, or delayed-retry work remains.
    Idle,
    /// At least one deficit can be scheduled immediately.
    Actionable,
    /// Matching work is currently leased.
    Running,
    /// Only future retry deadlines remain.
    WaitingForRetry,
}

/// Reconciled source readiness plus observable per-stage diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadinessSnapshot {
    /// Source whose desired and observed state was reconciled.
    pub source_id: String,
    /// Current committed source generation.
    pub source_generation: i64,
    /// Monotonic desired-state publication revision used to fence concurrent writers.
    pub readiness_revision: i64,
    /// Durable source availability.
    pub availability: SourceAvailability,
    /// Every desired target with its authoritative classification.
    pub entries: Vec<ReadinessEntry>,
    /// Deduplicated actionable deficits.
    pub deficits: Vec<ReadinessDeficit>,
    /// Per-stage diagnostic counts.
    pub stage_counts: BTreeMap<ReadinessStage, ReadinessStageCounts>,
    /// Current coordinator activity.
    pub activity: ReadinessActivity,
}

impl ReadinessSnapshot {
    /// Whether the coordinator has no immediately actionable deficit.
    pub fn is_idle(&self) -> bool {
        self.activity == ReadinessActivity::Idle
    }

    /// Whether all active work has reached a current or terminal classification.
    pub fn is_converged(&self) -> bool {
        self.availability == SourceAvailability::Active
            && self.entries.iter().all(|entry| {
                entry.classification == ReadinessClassification::Current
                    || entry.classification.is_terminal()
            })
    }

    /// Whether every eligible target is current and the source is active.
    pub fn is_fully_ready(&self) -> bool {
        self.availability == SourceAvailability::Active
            && self
                .entries
                .iter()
                .all(|entry| match entry.target.eligibility {
                    ReadinessEligibility::Eligible => {
                        entry.classification == ReadinessClassification::Current
                    }
                    ReadinessEligibility::Unsupported | ReadinessEligibility::Deleted => {
                        entry.classification.is_terminal()
                    }
                })
    }
}

/// Result of attempting to publish a generation-fenced artifact completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactPublishOutcome {
    /// The completion exactly matched the current target and was persisted.
    Recorded,
    /// The target changed, disappeared, or became terminal before completion.
    RejectedStale,
}
