/// Controller messages emitted by readiness-adjacent background work.
#[derive(Clone, Debug)]
pub(crate) enum AnalysisJobMessage {
    /// A targeted manifest reconciliation made current readiness actionable.
    ReadinessReconciliationFinished {
        source_id: crate::sample_sources::SourceId,
        changed: usize,
        /// Whether the completion should surface a status message.
        announce: bool,
    },
    /// A targeted manifest reconciliation failed.
    ReadinessReconciliationFailed(String),
    /// Duration metadata was updated for a source.
    DurationsUpdated {
        source_id: crate::sample_sources::SourceId,
        updated: usize,
    },
}
