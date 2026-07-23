//! Durable, versioned source-readiness targets and reconciliation.
//!
//! The source database owns desired targets and exact artifact completions. Actionable deficits
//! reuse the existing `analysis_jobs` table, while the legacy analysis claimant ignores rows owned
//! by the readiness coordinator until the bounded supervisor claims them explicitly.

mod model;
mod snapshot;
mod store;

pub use model::{
    ClaimedReadinessWork, ReadinessArtifact, ReadinessClaimOrigin, ReadinessEligibility,
    ReadinessFailureClassification, ReadinessFailureOutcome, ReadinessLeaseRenewalOutcome,
    ReadinessMembership, ReadinessRetryPolicy, ReadinessScopeKind, ReadinessStage, ReadinessTarget,
    ReadinessWorkMutationOutcome, ReadinessWorkStats, SourceAvailability,
};
pub use snapshot::{
    ArtifactPublishOutcome, ReadinessActivity, ReadinessClassification, ReadinessDeficit,
    ReadinessEntry, ReadinessProgress, ReadinessSnapshot, ReadinessStageCounts,
};
#[cfg(test)]
pub(crate) use store::reconcile_readiness_with_hook;
pub use store::{
    ReadinessCompatibilityCleanup, ReadinessDeltaPublicationOutcome,
    ReadinessEmbeddingArtifactTarget, ReadinessError, ReadinessSimilarityManifest,
    ReadinessSimilarityManifestRequest, ReadinessSimilarityManifestRow,
    ReadinessSimilarityPayloadContract, ReadinessSourceState, ReadinessStore,
    ReadinessTargetDeltaPublication, ReadinessTargetPublication, ReadinessView,
};
#[cfg(test)]
pub(crate) use store::{
    cancel_readiness_work, claim_readiness_target, complete_readiness_work,
    complete_readiness_work_with_artifact_ref, fail_readiness_work, invalidate_readiness_artifact,
    persist_readiness_deficits, publish_readiness_artifact, readiness_work_stats,
    reconcile_readiness, release_readiness_work, renew_readiness_lease, replace_readiness_targets,
};

#[cfg(test)]
mod tests;
