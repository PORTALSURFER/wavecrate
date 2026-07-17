//! Durable, versioned source-readiness targets and reconciliation.
//!
//! The source database owns desired targets and exact artifact completions. Actionable deficits
//! reuse the existing `analysis_jobs` table, while the legacy analysis claimant ignores rows owned
//! by the readiness coordinator until the bounded supervisor claims them explicitly.

mod model;
mod snapshot;
mod store;

pub use model::{
    ClaimedReadinessWork, ReadinessArtifact, ReadinessEligibility, ReadinessFailureClassification,
    ReadinessFailureOutcome, ReadinessLeaseRenewalOutcome, ReadinessRetryPolicy,
    ReadinessScopeKind, ReadinessStage, ReadinessTarget, ReadinessWorkMutationOutcome,
    ReadinessWorkStats, SourceAvailability,
};
pub use snapshot::{
    ArtifactPublishOutcome, ReadinessActivity, ReadinessClassification, ReadinessDeficit,
    ReadinessEntry, ReadinessSnapshot, ReadinessStageCounts,
};
#[cfg(test)]
pub(crate) use store::reconcile_readiness_with_hook;
pub use store::{
    ReadinessError, cancel_readiness_work, claim_readiness_target, complete_readiness_work,
    fail_readiness_work, invalidate_readiness_artifact, persist_readiness_deficits,
    persist_readiness_deficits_with_cancel, publish_readiness_artifact, readiness_work_stats,
    reconcile_readiness, reconcile_readiness_with_cancel, release_readiness_work,
    renew_readiness_lease, replace_readiness_targets, replace_readiness_targets_with_cancel,
};

#[cfg(test)]
mod tests;
