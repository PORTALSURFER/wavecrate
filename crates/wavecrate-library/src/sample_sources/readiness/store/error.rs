use thiserror::Error;

use super::super::model::ReadinessStage;

/// Errors returned by the durable readiness contract.
#[derive(Debug, Error)]
pub enum ReadinessError {
    /// SQLite persistence or query failed.
    #[error("Readiness database operation failed: {0}")]
    Sql(#[from] rusqlite::Error),
    /// A stored enum value is not part of the versioned readiness contract.
    #[error("Unknown stored readiness value for {field}: {value}")]
    UnknownStoredValue {
        /// Field containing the invalid value.
        field: &'static str,
        /// Invalid stored value.
        value: String,
    },
    /// A target did not match the source or generation being replaced.
    #[error("Readiness target does not match source {source_id} generation {generation}")]
    TargetGenerationMismatch {
        /// Expected source identity.
        source_id: String,
        /// Expected source generation.
        generation: i64,
    },
    /// Two desired targets used the same durable key.
    #[error("Duplicate readiness target for {source_id}:{scope_id}:{stage:?}")]
    DuplicateTarget {
        /// Source identity.
        source_id: String,
        /// File or source scope identity.
        scope_id: String,
        /// Duplicated readiness stage.
        stage: ReadinessStage,
    },
    /// A caller attempted to replace newer desired state with an older generation.
    #[error(
        "Stale readiness generation {attempted} for {source_id}; current generation is {current}"
    )]
    StaleSourceGeneration {
        /// Source identity.
        source_id: String,
        /// Generation supplied by the caller.
        attempted: i64,
        /// Current persisted generation.
        current: i64,
    },
    /// A caller attempted to replace desired state using an already-consumed publication revision.
    #[error("Stale readiness revision {attempted} for {source_id}; current revision is {current}")]
    StaleReadinessRevision {
        /// Source identity.
        source_id: String,
        /// Revision supplied by the caller.
        attempted: i64,
        /// Current persisted revision.
        current: i64,
    },
    /// A target or artifact omitted its exact content or membership generation.
    #[error("Readiness generation must be non-empty for {source_id}:{scope_id}:{stage:?}")]
    InvalidContentGeneration {
        /// Source identity.
        source_id: String,
        /// File or source scope identity.
        scope_id: String,
        /// Readiness stage requiring the generation.
        stage: ReadinessStage,
    },
    /// No desired readiness state has been published for the source.
    #[error("No readiness state exists for source {0}")]
    UnknownSource(String),
    /// The read-only database predates the additive readiness schema.
    #[error("Source database does not contain the readiness schema")]
    SchemaUnavailable,
}
