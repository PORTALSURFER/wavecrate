use thiserror::Error;

use super::super::model::{ReadinessScopeKind, ReadinessStage};

/// Errors returned by the durable readiness contract.
#[derive(Debug, Error)]
pub enum ReadinessError {
    /// The caller cancelled a potentially long readiness database operation.
    #[error("Readiness operation cancelled")]
    Cancelled,
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
    /// A target or artifact omitted its versioned readiness contract identity.
    #[error("Readiness artifact version must be non-empty for {source_id}:{scope_id}:{stage:?}")]
    InvalidArtifactVersion {
        /// Source identity.
        source_id: String,
        /// File or source scope identity.
        scope_id: String,
        /// Readiness stage requiring the version.
        stage: ReadinessStage,
    },
    /// A cache-backed artifact omitted its durable reverse-ownership reference.
    #[error("Readiness artifact reference must be non-empty for {source_id}:{scope_id}:{stage:?}")]
    InvalidArtifactReference {
        /// Source identity.
        source_id: String,
        /// File or source scope identity.
        scope_id: String,
        /// Readiness stage requiring the reference.
        stage: ReadinessStage,
    },
    /// A stage was assigned to the wrong durable ownership scope.
    #[error("Invalid readiness scope {scope_kind:?} for {source_id}:{scope_id}:{stage:?}")]
    InvalidStageScope {
        /// Source identity.
        source_id: String,
        /// File or source scope identity.
        scope_id: String,
        /// Readiness stage whose ownership is invalid.
        stage: ReadinessStage,
        /// Supplied ownership scope.
        scope_kind: ReadinessScopeKind,
    },
    /// A target's durable identity did not match its ownership scope.
    #[error("Invalid readiness identity {scope_id} for source {source_id} scope {scope_kind:?}")]
    InvalidScopeIdentity {
        /// Source identity.
        source_id: String,
        /// Supplied scope identity.
        scope_id: String,
        /// Supplied ownership scope.
        scope_kind: ReadinessScopeKind,
    },
    /// Eligible file work omitted its executable source-relative path.
    #[error("Eligible readiness target has no path for {source_id}:{scope_id}:{stage:?}")]
    InvalidRelativePath {
        /// Source identity.
        source_id: String,
        /// File identity.
        scope_id: String,
        /// Readiness stage requiring the path.
        stage: ReadinessStage,
    },
    /// A complete desired-state publication omitted one required readiness stage.
    #[error("Readiness target matrix for {scope_id} is missing {stage:?}")]
    IncompleteTargetMatrix {
        /// File identity, or source identity for source-level work.
        scope_id: String,
        /// Required stage missing from the publication.
        stage: ReadinessStage,
    },
    /// The desired file identities did not exactly match the committed source manifest.
    #[error("Readiness identities do not match the current source manifest")]
    ManifestMembershipMismatch {
        /// Current manifest identities omitted from desired readiness state.
        missing: Vec<String>,
        /// Desired identities absent from the current manifest.
        unexpected: Vec<String>,
    },
    /// A current supported manifest row did not yet have a stable file identity.
    #[error("Current source file has no stable identity: {path}")]
    ManifestIdentityUnavailable {
        /// Source-relative manifest path lacking an identity.
        path: String,
    },
    /// Two current manifest paths claimed the same stable file identity.
    #[error(
        "Current source manifest has duplicate identity {identity}: {first_path}, {second_path}"
    )]
    DuplicateManifestIdentity {
        /// Duplicated stable identity.
        identity: String,
        /// First source-relative path.
        first_path: String,
        /// Second source-relative path.
        second_path: String,
    },
    /// Desired stages for one current identity disagreed about its executable path.
    #[error("Readiness targets disagree on the path for identity {identity}")]
    InconsistentTargetPath {
        /// Stable file identity.
        identity: String,
    },
    /// A desired file path did not match the current manifest path for its identity.
    #[error("Readiness path for {identity} is stale; expected {expected}, got {supplied}")]
    ManifestPathMismatch {
        /// Stable file identity.
        identity: String,
        /// Current authoritative source-relative path.
        expected: String,
        /// Supplied source-relative path.
        supplied: String,
    },
    /// No desired readiness state has been published for the source.
    #[error("No readiness state exists for source {0}")]
    UnknownSource(String),
    /// The read-only database predates the additive readiness schema.
    #[error("Source database does not contain the readiness schema")]
    SchemaUnavailable,
    /// A lease must advance time by a positive bounded duration.
    #[error("Readiness lease duration must be positive: {0}")]
    InvalidLeaseDuration(i64),
    /// A supplied timestamp and duration could not be represented durably.
    #[error("Readiness timestamp overflow")]
    TimestampOverflow,
}
