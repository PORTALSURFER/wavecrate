//! Durable, versioned source-readiness targets and reconciliation.
//!
//! The source database owns desired targets and exact artifact completions. Actionable deficits
//! reuse the existing `analysis_jobs` table, while the legacy analysis claimant ignores rows owned
//! by the readiness coordinator until the bounded supervisor claims them explicitly.

mod model;
mod snapshot;
mod store;

pub use model::{
    ReadinessArtifact, ReadinessEligibility, ReadinessScopeKind, ReadinessStage, ReadinessTarget,
    SourceAvailability,
};
pub use snapshot::{
    ArtifactPublishOutcome, ReadinessActivity, ReadinessClassification, ReadinessDeficit,
    ReadinessEntry, ReadinessSnapshot, ReadinessStageCounts,
};
pub use store::{
    ReadinessError, persist_readiness_deficits, publish_readiness_artifact, reconcile_readiness,
    replace_readiness_targets,
};

#[cfg(test)]
mod tests;
