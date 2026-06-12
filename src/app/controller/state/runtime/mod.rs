//! Runtime state and job coordination for the controller.

/// Browser and deferred metadata/projection runtime state.
mod browser_runtime;
/// Live configuration persistence runtime state.
mod config_persistence;
mod deferred;
/// Incremental derived-state dirty graph model used by UI projection paths.
mod derived_graph;
/// Retained-delete recovery runtime state.
mod file_recovery;
/// Cached map-query runtime state.
mod map_runtime;
mod performance;
/// Projection revision and derived-state runtime state.
mod projection_runtime;
/// Similarity prep and query runtime state.
mod similarity_runtime;
mod source_lane;
/// Source scan and watcher synchronization runtime state.
mod source_sync_runtime;
/// Startup-deferred runtime state.
mod startup_runtime;
/// Test-only failure injection runtime state.
#[cfg(test)]
mod test_faults;
/// Waveform refresh, render, and seek runtime state.
mod waveform_runtime;

use crate::app::controller::jobs;
use crate::app::controller::library::analysis_jobs;
use crate::sample_sources::db::SourceDbError;
use crate::sample_sources::{ScanMode, SourceId, WavEntry};
pub(crate) use browser_runtime::BrowserRuntimeState;
pub(crate) use config_persistence::{ConfigPersistenceRuntimeState, PendingConfigPersist};
pub(crate) use deferred::{
    AnalysisProgressUiCache, BrowserSelectionCommitRequest, BrowserSelectionCommitStage,
    BrowserSelectionLoadState, BrowserSelectionTransition, LoadedSimilarityQueryCache,
    LoadedSimilarityQueryData, LoadedSimilaritySourceCandidate, LoadedSimilaritySourceSnapshot,
    PendingBrowserFeatureCacheRefresh, PendingFocusedSimilarityQuery,
    PendingFocusedSimilarityRefresh, PendingLoadedDurationMetadata, PendingLoadedSimilarityQuery,
    PendingSimilarityFilterRebuild,
};
pub(crate) use derived_graph::{DerivedNodeId, DirtyReason};
pub(crate) use file_recovery::FileRecoveryRuntimeState;
pub(crate) use map_runtime::MapRuntimeState;
pub(crate) use performance::PerformanceGovernorState;
pub(crate) use projection_runtime::{ProjectionRevisionDirtyMask, ProjectionRuntimeState};
pub(crate) use similarity_runtime::{
    SimilarityPrepStage, SimilarityPrepState, SimilarityRuntimeState,
};
#[cfg(test)]
pub(crate) use source_lane::AutoRenameBatchRowSnapshot;
pub(crate) use source_lane::{ActiveAutoRenameBatchSnapshot, AutoRenameBatchRowState};
pub(crate) use source_lane::{
    BrowserRenameBusyDecision, BrowserRenameIntentKey, MetadataRollback,
    PendingBrowserAutoRenameIntent, PendingMetadataMutation, PendingSourceHydration,
    SourceLaneRuntimeState,
};
pub(crate) use source_sync_runtime::SourceSyncRuntimeState;
pub(crate) use startup_runtime::StartupRuntimeState;
use std::path::PathBuf;
use std::time::Duration;
#[cfg(test)]
pub(crate) use test_faults::TestFaultRuntimeState;
pub(crate) use waveform_runtime::{
    PendingWaveformRender, PendingWaveformTransientCompute, WaveformRefreshReason,
    WaveformRuntimeState,
};

pub(crate) struct ControllerRuntimeState {
    pub(crate) jobs: jobs::ControllerJobs,
    pub(crate) analysis: analysis_jobs::AnalysisWorkerPool,
    pub(crate) performance: PerformanceGovernorState,
    pub(crate) config_persistence: ConfigPersistenceRuntimeState,
    pub(crate) waveform: WaveformRuntimeState,
    pub(crate) projection: ProjectionRuntimeState,
    pub(crate) browser: BrowserRuntimeState,
    pub(crate) similarity: SimilarityRuntimeState,
    pub(crate) map: MapRuntimeState,
    pub(crate) recovery: FileRecoveryRuntimeState,
    pub(crate) startup: StartupRuntimeState,
    pub(crate) source_sync: SourceSyncRuntimeState,
    /// Source-specific runtime state for hydration, folder projection, and mutations.
    pub(crate) source_lane: SourceLaneRuntimeState,
    #[cfg(test)]
    pub(crate) test_faults: TestFaultRuntimeState,
}

impl ControllerRuntimeState {
    pub(crate) fn new(
        jobs: jobs::ControllerJobs,
        analysis: analysis_jobs::AnalysisWorkerPool,
    ) -> Self {
        Self {
            jobs,
            analysis,
            performance: PerformanceGovernorState::new(),
            config_persistence: ConfigPersistenceRuntimeState::default(),
            waveform: WaveformRuntimeState::default(),
            projection: ProjectionRuntimeState::default(),
            browser: BrowserRuntimeState::default(),
            similarity: SimilarityRuntimeState::default(),
            map: MapRuntimeState::default(),
            recovery: FileRecoveryRuntimeState::default(),
            startup: StartupRuntimeState::default(),
            source_sync: SourceSyncRuntimeState::default(),
            source_lane: SourceLaneRuntimeState::default(),
            #[cfg(test)]
            test_faults: TestFaultRuntimeState::default(),
        }
    }

    /// Begin a waveform-refresh batch where refresh requests are coalesced.
    pub(crate) fn begin_waveform_refresh_batch(&mut self) {
        self.waveform.begin_refresh_batch();
    }

    /// End the current waveform-refresh batch, saturating at zero depth.
    pub(crate) fn end_waveform_refresh_batch(&mut self) {
        self.waveform.end_refresh_batch();
    }

    /// Return true when waveform refresh requests should be deferred.
    pub(crate) fn waveform_refresh_batch_active(&self) -> bool {
        self.waveform.refresh_batch_active()
    }
}

pub(crate) struct WavLoadJob {
    pub(crate) source_id: SourceId,
    pub(crate) root: PathBuf,
    pub(crate) page_size: usize,
}

#[derive(Debug)]
pub(crate) struct WavLoadResult {
    pub(crate) source_id: SourceId,
    pub(crate) result: Result<Vec<WavEntry>, LoadEntriesError>,
    pub(crate) elapsed: Duration,
    pub(crate) total: usize,
    pub(crate) page_index: usize,
}

#[derive(Debug)]
pub(crate) struct ScanResult {
    pub(crate) source_id: SourceId,
    pub(crate) mode: ScanMode,
    pub(crate) kind: ScanKind,
    pub(crate) result: Result<
        crate::sample_sources::scanner::ScanStats,
        crate::sample_sources::scanner::ScanError,
    >,
}

/// Indicates whether a scan was triggered by the user or automatically in the background.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScanKind {
    Manual,
    Auto,
}

#[derive(Debug)]
pub(crate) enum ScanJobMessage {
    Progress {
        completed: usize,
        detail: Option<String>,
    },
    Finished(ScanResult),
}

#[derive(Clone, Debug)]
pub(crate) struct UpdateCheckResult {
    pub(crate) result: Result<crate::updater::UpdateCheckOutcome, String>,
}

#[derive(Debug)]
pub(crate) enum LoadEntriesError {
    Db(SourceDbError),
    Message(String),
}

impl std::fmt::Display for LoadEntriesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadEntriesError::Db(err) => write!(f, "{err}"),
            LoadEntriesError::Message(msg) => f.write_str(msg),
        }
    }
}

impl From<String> for LoadEntriesError {
    fn from(value: String) -> Self {
        LoadEntriesError::Message(value)
    }
}
