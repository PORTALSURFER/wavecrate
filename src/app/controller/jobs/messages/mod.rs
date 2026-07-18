//! Job message and DTO types shared across controller workers.

use super::*;

mod issue_gateway_types;
mod metadata_types;
mod normalization_types;
mod search_types;
mod similarity_types;
mod source_db_maintenance_types;
mod source_lane_types;
mod waveform_types;

pub(crate) use self::issue_gateway_types::*;
pub(crate) use self::metadata_types::*;
pub(crate) use self::normalization_types::*;
pub(crate) use self::search_types::*;
pub(crate) use self::similarity_types::*;
pub(crate) use self::source_db_maintenance_types::*;
pub(crate) use self::source_lane_types::*;
pub(crate) use self::waveform_types::*;

/// Messages emitted by background lanes back to the controller thread.
#[derive(Debug)]
pub(crate) enum JobMessage {
    WavLoaded(WavLoadResult),
    SourceAddPrepared(SourceAddPreparedResult),
    SourceRemapPrepared(SourceRemapPreparedResult),
    SourceHydrated(SourceHydrationResult),
    BrowserFeatureCacheRefreshed(BrowserFeatureCacheRefreshResult),
    FolderProjected(FolderProjectionResult),
    MetadataMutationFinished(MetadataMutationResult),
    ConfigPersistFinished(ConfigPersistResult),
    WaveformRendered(WaveformRenderResult),
    WaveformTransientsComputed(WaveformTransientResult),
    AudioLoaded(AudioLoadResult),
    RecordingWaveformLoaded(RecordingWaveformLoadResult),
    Scan(ScanJobMessage),
    FolderScanFinished(FolderScanResult),
    SourceWatch(SourceWatchEvent),
    TrashMove(trash_move::TrashMoveMessage),
    FolderDeleteRecoveryFinished(DeleteRecoveryReport),
    FileOps(FileOpMessage),
    Analysis(AnalysisJobMessage),
    AnalysisFailuresLoaded(AnalysisFailuresResult),
    FocusedSimilarityLoaded(FocusedSimilarityResult),
    LoadedSimilarityQueryBuilt(LoadedSimilarityQueryResult),
    UpdateChecked(UpdateCheckResult),
    IssueGatewayCreated(IssueGatewayCreateResult),
    IssueGatewayAuthed(IssueGatewayAuthResult),
    IssueTokenLoaded(IssueTokenLoadResult),
    IssueTokenSaved(IssueTokenSaveResult),
    IssueTokenDeleted(IssueTokenDeleteResult),
    BrowserSearchFinished(SearchResult),
    SourceDbMaintenanceFinished(SourceDbMaintenanceResult),
    SelectionExport(SelectionExportMessage),
    Normalized(NormalizationResult),
}
