/// Stable readiness stages required for a usable source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadinessStage {
    /// The source manifest contains the current file identity.
    IndexedIdentity,
    /// A bounded playback descriptor and compact waveform summary are current.
    PlaybackSummary,
    /// Versioned analysis features are current.
    AnalysisFeatures,
    /// Versioned similarity embedding and aspect descriptors are current.
    EmbeddingAspects,
    /// Source-level ANN membership and layout generation are current.
    SimilarityLayout,
}

impl ReadinessStage {
    pub(crate) const ALL: [Self; 5] = [
        Self::IndexedIdentity,
        Self::PlaybackSummary,
        Self::AnalysisFeatures,
        Self::EmbeddingAspects,
        Self::SimilarityLayout,
    ];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::IndexedIdentity => "indexed_identity",
            Self::PlaybackSummary => "playback_summary",
            Self::AnalysisFeatures => "analysis_features",
            Self::EmbeddingAspects => "embedding_aspects",
            Self::SimilarityLayout => "similarity_layout",
        }
    }

    pub(crate) fn from_stored(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|stage| stage.as_str() == value)
    }

    pub(crate) fn job_type(self) -> &'static str {
        match self {
            Self::IndexedIdentity => "readiness_indexed_identity_v1",
            Self::PlaybackSummary => "readiness_playback_summary_v1",
            Self::AnalysisFeatures => "readiness_analysis_features_v1",
            Self::EmbeddingAspects => "readiness_embedding_aspects_v1",
            Self::SimilarityLayout => "readiness_similarity_layout_v1",
        }
    }
}

/// Whether one readiness stage belongs to a file identity or to the source as a whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadinessScopeKind {
    /// One current file identity.
    File,
    /// One source-level generation.
    Source,
}

impl ReadinessScopeKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Source => "source",
        }
    }

    pub(crate) fn from_stored(value: &str) -> Option<Self> {
        match value {
            "file" => Some(Self::File),
            "source" => Some(Self::Source),
            _ => None,
        }
    }
}

/// Durable availability of a configured source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceAvailability {
    /// The enabled source root is available for convergence.
    Active,
    /// The enabled source root is temporarily unavailable.
    Offline,
    /// The source is intentionally disabled and must not be processed.
    Disabled,
}

impl SourceAvailability {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Offline => "offline",
            Self::Disabled => "disabled",
        }
    }

    pub(crate) fn from_stored(value: &str) -> Option<Self> {
        match value {
            "active" => Some(Self::Active),
            "offline" => Some(Self::Offline),
            "disabled" => Some(Self::Disabled),
            _ => None,
        }
    }
}

/// Eligibility of an identity for one desired readiness stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadinessEligibility {
    /// The stage is required and may produce work.
    Eligible,
    /// The file type or shape is unsupported for this stage.
    Unsupported,
    /// The identity is no longer current and must not produce work.
    Deleted,
}

impl ReadinessEligibility {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Eligible => "eligible",
            Self::Unsupported => "unsupported",
            Self::Deleted => "deleted",
        }
    }

    pub(crate) fn from_stored(value: &str) -> Option<Self> {
        match value {
            "eligible" => Some(Self::Eligible),
            "unsupported" => Some(Self::Unsupported),
            "deleted" => Some(Self::Deleted),
            _ => None,
        }
    }
}

/// Durable desired state for one versioned readiness stage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadinessTarget {
    /// Configured source identity.
    pub source_id: String,
    /// File-scoped or source-scoped ownership.
    pub scope_kind: ReadinessScopeKind,
    /// Stable file identity, or the source ID for source-scoped work.
    pub scope_id: String,
    /// Current source-relative path when the scope is a file.
    pub relative_path: Option<String>,
    /// Required readiness stage.
    pub stage: ReadinessStage,
    /// Algorithm, format, or artifact contract version required by the stage.
    pub required_version: String,
    /// Committed source-manifest generation that owns this target.
    pub source_generation: i64,
    /// Non-empty content generation for file-scoped work, or membership generation for source work.
    pub content_generation: String,
    /// Whether this target may produce actionable work.
    pub eligibility: ReadinessEligibility,
}

impl ReadinessTarget {
    /// Build one eligible file-stage target.
    pub fn file(
        source_id: impl Into<String>,
        file_identity: impl Into<String>,
        relative_path: impl Into<String>,
        stage: ReadinessStage,
        required_version: impl Into<String>,
        source_generation: i64,
        content_generation: impl Into<String>,
    ) -> Self {
        Self {
            source_id: source_id.into(),
            scope_kind: ReadinessScopeKind::File,
            scope_id: file_identity.into(),
            relative_path: Some(relative_path.into()),
            stage,
            required_version: required_version.into(),
            source_generation,
            content_generation: content_generation.into(),
            eligibility: ReadinessEligibility::Eligible,
        }
    }

    /// Build one eligible source-stage target.
    pub fn source(
        source_id: impl Into<String>,
        stage: ReadinessStage,
        required_version: impl Into<String>,
        source_generation: i64,
        membership_generation: impl Into<String>,
    ) -> Self {
        let source_id = source_id.into();
        Self {
            scope_kind: ReadinessScopeKind::Source,
            scope_id: source_id.clone(),
            source_id,
            relative_path: None,
            stage,
            required_version: required_version.into(),
            source_generation,
            content_generation: membership_generation.into(),
            eligibility: ReadinessEligibility::Eligible,
        }
    }

    /// Return a copy with terminal eligibility.
    pub fn with_eligibility(mut self, eligibility: ReadinessEligibility) -> Self {
        self.eligibility = eligibility;
        self
    }

    pub(crate) fn key(&self) -> ReadinessKey {
        ReadinessKey {
            source_id: self.source_id.clone(),
            scope_kind: self.scope_kind,
            scope_id: self.scope_id.clone(),
            stage: self.stage,
        }
    }
}

/// Persisted completion marker for one exact readiness generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadinessArtifact {
    /// Configured source identity.
    pub source_id: String,
    /// File-scoped or source-scoped ownership.
    pub scope_kind: ReadinessScopeKind,
    /// Stable file identity, or source identity for source-scoped work.
    pub scope_id: String,
    /// Completed stage.
    pub stage: ReadinessStage,
    /// Version produced by the completed work.
    pub artifact_version: String,
    /// Source generation captured by the work.
    pub source_generation: i64,
    /// Non-empty file content or source membership generation captured by the work.
    pub content_generation: String,
    /// Completion timestamp used for diagnostics only.
    pub completed_at: i64,
}

impl ReadinessArtifact {
    /// Build a completion marker that exactly matches a desired target.
    pub fn for_target(target: &ReadinessTarget, completed_at: i64) -> Self {
        Self {
            source_id: target.source_id.clone(),
            scope_kind: target.scope_kind,
            scope_id: target.scope_id.clone(),
            stage: target.stage,
            artifact_version: target.required_version.clone(),
            source_generation: target.source_generation,
            content_generation: target.content_generation.clone(),
            completed_at,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ReadinessKey {
    pub(crate) source_id: String,
    pub(crate) scope_kind: ReadinessScopeKind,
    pub(crate) scope_id: String,
    pub(crate) stage: ReadinessStage,
}
