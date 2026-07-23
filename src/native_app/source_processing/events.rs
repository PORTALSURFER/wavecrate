use std::{path::PathBuf, sync::mpsc::Sender};

use wavecrate::sample_sources::{readiness::ReadinessStage, scanner::CommittedSourceDelta};

/// Identifies one configured lifetime of a source.
///
/// Source identifiers may be reused after removal, so consumers must use the
/// generation as part of the identity when applying asynchronous events.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(in crate::native_app) struct SourceProcessingLifecycle {
    pub(in crate::native_app) source_id: String,
    pub(in crate::native_app) generation: u64,
}

impl SourceProcessingLifecycle {
    pub(in crate::native_app) fn new(source_id: impl Into<String>, generation: u64) -> Self {
        Self {
            source_id: source_id.into(),
            generation,
        }
    }
}

/// Semantic source work that a consumer may present or record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum SourceProcessingActivity {
    Discovering {
        phase: SourceDiscoveryPhase,
    },
    Readiness {
        stage: ReadinessStage,
        relative_path: Option<String>,
    },
    ManifestAudit {
        checked: Option<usize>,
        relative_path: Option<PathBuf>,
    },
    WaitingForPrerequisites {
        retry_at: Option<i64>,
    },
    WaitingForRetry {
        retry_at: i64,
    },
}

/// Stable user-facing phases within source readiness discovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::native_app) enum SourceDiscoveryPhase {
    Preparing,
    InspectingManifest,
    PreparingTargets,
    ComparingReadiness,
    ComparingChangedReadiness,
    QueueingWork,
}

/// Backend-neutral progress for one source lifecycle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::native_app) struct SourceProcessingProgressEvent {
    pub(in crate::native_app) lifecycle: SourceProcessingLifecycle,
    pub(in crate::native_app) source_row_active: bool,
    pub(in crate::native_app) completed: usize,
    pub(in crate::native_app) total: usize,
    pub(in crate::native_app) activity: SourceProcessingActivity,
}

/// Ordered outward observations produced by the source-processing supervisor.
#[derive(Clone, Debug)]
pub(in crate::native_app) enum SourceProcessingEvent {
    Progress(SourceProcessingProgressEvent),
    SimilarityReadinessAdvanced {
        lifecycle: SourceProcessingLifecycle,
    },
    ManifestAuditCommitted {
        lifecycle: SourceProcessingLifecycle,
        committed_delta: CommittedSourceDelta,
    },
    Completed,
}

impl SourceProcessingEvent {
    pub(super) fn lifecycle(&self) -> Option<&SourceProcessingLifecycle> {
        match self {
            Self::Progress(progress) => Some(&progress.lifecycle),
            Self::SimilarityReadinessAdvanced { lifecycle }
            | Self::ManifestAuditCommitted { lifecycle, .. } => Some(lifecycle),
            Self::Completed => None,
        }
    }
}

/// A narrow, non-blocking consumer of source-processing events.
///
/// Implementations must return promptly. The supervisor deliberately owns
/// throttling and lifecycle fencing before invoking the sink.
pub(in crate::native_app) trait SourceProcessingEventSink:
    Send + Sync
{
    fn try_publish(&self, event: SourceProcessingEvent) -> bool;
}

impl SourceProcessingEventSink for Sender<SourceProcessingEvent> {
    fn try_publish(&self, event: SourceProcessingEvent) -> bool {
        self.send(event).is_ok()
    }
}
