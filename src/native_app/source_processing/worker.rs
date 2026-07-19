//! Killable process boundary for supervisor-owned decode, DSP, and embedding work.

#[cfg(not(test))]
mod child_process;

#[cfg(not(test))]
pub(in crate::native_app) use child_process::wait_for_cancellable_child;

use std::{path::Path, sync::atomic::AtomicBool};

#[cfg(not(test))]
use std::{
    io::Read,
    process::{Command, Stdio},
};

use serde::{Deserialize, Serialize};
use wavecrate::sample_sources::{
    SampleSource, SourceDatabase, SourceDatabaseConnectionRole, db::SourceDbError,
};

const INTERNAL_SOURCE_ANALYSIS_ARG: &str = "--wavecrate-internal-source-analysis-v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
enum SourceAnalysisTask {
    ReadinessFeature {
        relative_path: String,
        content_hash: String,
        analysis_version: String,
    },
    ReadinessEmbedding {
        relative_path: String,
        content_hash: String,
        analysis_version: String,
    },
    RetireDerivedState,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SourceAnalysisRequest {
    source: SampleSource,
    task: SourceAnalysisTask,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct SourceAnalysisResult {
    produced: bool,
    processed: usize,
    failed: usize,
    retired_cache_refs: usize,
    terminal_offline: bool,
}

/// Stable, serializable classification of an execution failure.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum SourceProcessingFailureClass {
    Retryable,
    Permanent,
    Unsupported,
}

/// Stable execution-failure code persisted independently from display text.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum SourceProcessingFailureCode {
    DecoderUnsupported,
    SqliteBusy,
    WorkerProcessFailed,
    WorkerProtocol,
    ExecutionUnclassified,
}

impl SourceProcessingFailureCode {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::DecoderUnsupported => "decoder_unsupported",
            Self::SqliteBusy => "sqlite_busy",
            Self::WorkerProcessFailed => "worker_process_failed",
            Self::WorkerProtocol => "worker_protocol",
            Self::ExecutionUnclassified => "execution_unclassified",
        }
    }
}

/// Typed execution failure crossing the worker/supervisor boundary.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct SourceProcessingFailure {
    pub(super) class: SourceProcessingFailureClass,
    pub(super) code: SourceProcessingFailureCode,
    /// Sanitized context suitable for user-facing readiness diagnostics.
    pub(super) context: String,
    /// Original error context retained for logs without making it part of policy.
    pub(super) source_error: Option<String>,
}

impl SourceProcessingFailure {
    pub(super) const fn readiness_failure_classification(
        &self,
    ) -> wavecrate::sample_sources::readiness::ReadinessFailureClassification {
        match self.class {
            SourceProcessingFailureClass::Retryable => {
                wavecrate::sample_sources::readiness::ReadinessFailureClassification::Retryable
            }
            SourceProcessingFailureClass::Permanent => {
                wavecrate::sample_sources::readiness::ReadinessFailureClassification::Permanent
            }
            SourceProcessingFailureClass::Unsupported => {
                wavecrate::sample_sources::readiness::ReadinessFailureClassification::Unsupported
            }
        }
    }

    fn retryable(code: SourceProcessingFailureCode, context: impl Into<String>) -> Self {
        Self {
            class: SourceProcessingFailureClass::Retryable,
            code,
            context: context.into(),
            source_error: None,
        }
    }

    fn permanent(
        code: SourceProcessingFailureCode,
        context: impl Into<String>,
        source_error: Option<String>,
    ) -> Self {
        Self {
            class: SourceProcessingFailureClass::Permanent,
            code,
            context: context.into(),
            source_error,
        }
    }
}

impl From<String> for SourceProcessingFailure {
    fn from(source_error: String) -> Self {
        // Unknown failures fail closed: new text must never silently become retryable.
        Self::permanent(
            SourceProcessingFailureCode::ExecutionUnclassified,
            "Source processing execution failed",
            Some(source_error),
        )
    }
}

impl From<rusqlite::Error> for SourceProcessingFailure {
    fn from(error: rusqlite::Error) -> Self {
        if matches!(
            error,
            rusqlite::Error::SqliteFailure(
                ref sqlite_error,
                _
            ) if matches!(
                sqlite_error.code,
                rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
            )
        ) {
            return Self::retryable(
                SourceProcessingFailureCode::SqliteBusy,
                "Source database is busy",
            );
        }
        Self::permanent(
            SourceProcessingFailureCode::ExecutionUnclassified,
            "Source database operation failed",
            Some(error.to_string()),
        )
    }
}

impl From<wavecrate::internal_analysis_jobs::ReadinessStageError> for SourceProcessingFailure {
    fn from(error: wavecrate::internal_analysis_jobs::ReadinessStageError) -> Self {
        match error {
            wavecrate::internal_analysis_jobs::ReadinessStageError::Decode(
                wavecrate_analysis::AnalysisDecodeError::Unsupported(detail),
            ) => Self {
                class: SourceProcessingFailureClass::Unsupported,
                code: SourceProcessingFailureCode::DecoderUnsupported,
                context: "Audio codec is unsupported".to_string(),
                source_error: Some(detail),
            },
            wavecrate::internal_analysis_jobs::ReadinessStageError::Decode(error) => {
                Self::permanent(
                    SourceProcessingFailureCode::ExecutionUnclassified,
                    "Audio decoding failed",
                    Some(error.to_string()),
                )
            }
            wavecrate::internal_analysis_jobs::ReadinessStageError::Other(error) => {
                Self::from(error)
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
enum SourceAnalysisResponse {
    Completed(SourceAnalysisResult),
    Failed(SourceProcessingFailure),
}

pub(super) fn run_source_retirement(
    source: &SampleSource,
    cancel: &AtomicBool,
) -> Result<Option<super::supervisor::SourceRetirementOutcome>, String> {
    #[cfg(test)]
    {
        if cancel.load(std::sync::atomic::Ordering::Acquire) {
            return Ok(None);
        }
        super::supervisor::retire_source_derived_state(source).map(Some)
    }
    #[cfg(not(test))]
    {
        let request = SourceAnalysisRequest {
            source: source.clone(),
            task: SourceAnalysisTask::RetireDerivedState,
        };
        run_request_in_child(&request, cancel)
            .map(|result| {
                result.map(|result| {
                    if result.terminal_offline {
                        super::supervisor::SourceRetirementOutcome::TerminalOffline
                    } else {
                        super::supervisor::SourceRetirementOutcome::Retired {
                            retired_cache_refs: result.retired_cache_refs,
                        }
                    }
                })
            })
            .map_err(|failure| failure.context)
    }
}

pub(super) fn run_readiness_feature_stage(
    connection: &mut rusqlite::Connection,
    source: &SampleSource,
    relative_path: &Path,
    content_hash: &str,
    analysis_version: &str,
    cancel: &AtomicBool,
) -> Result<bool, SourceProcessingFailure> {
    #[cfg(test)]
    {
        wavecrate::internal_analysis_jobs::run_readiness_feature_stage(
            connection,
            &source.root,
            source.id.as_str(),
            relative_path,
            content_hash,
            analysis_version,
            cancel,
        )
        .map_err(Into::into)
    }
    #[cfg(not(test))]
    {
        let _ = connection;
        let request = SourceAnalysisRequest {
            source: source.clone(),
            task: SourceAnalysisTask::ReadinessFeature {
                relative_path: relative_path.to_string_lossy().replace('\\', "/"),
                content_hash: content_hash.to_string(),
                analysis_version: analysis_version.to_string(),
            },
        };
        run_request_in_child(&request, cancel)
            .map(|result| result.is_some_and(|result| result.produced))
    }
}

pub(super) fn run_readiness_embedding_stage(
    connection: &mut rusqlite::Connection,
    source: &SampleSource,
    relative_path: &Path,
    content_hash: &str,
    analysis_version: &str,
    cancel: &AtomicBool,
) -> Result<bool, SourceProcessingFailure> {
    #[cfg(test)]
    {
        wavecrate::internal_analysis_jobs::run_readiness_embedding_stage(
            connection,
            &source.root,
            source.id.as_str(),
            relative_path,
            content_hash,
            analysis_version,
            cancel,
        )
        .map_err(Into::into)
    }
    #[cfg(not(test))]
    {
        let _ = connection;
        let request = SourceAnalysisRequest {
            source: source.clone(),
            task: SourceAnalysisTask::ReadinessEmbedding {
                relative_path: relative_path.to_string_lossy().replace('\\', "/"),
                content_hash: content_hash.to_string(),
                analysis_version: analysis_version.to_string(),
            },
        };
        run_request_in_child(&request, cancel)
            .map(|result| result.is_some_and(|result| result.produced))
    }
}

pub(in crate::native_app) fn run_internal_source_analysis_from_args()
-> Result<Option<String>, String> {
    let mut args = std::env::args();
    let _executable = args.next();
    if args.next().as_deref() != Some(INTERNAL_SOURCE_ANALYSIS_ARG) {
        return Ok(None);
    }
    let request_json = args
        .next()
        .ok_or_else(|| "Internal source analysis is missing its request".to_string())?;
    if args.next().is_some() {
        return Err("Internal source analysis received unexpected arguments".to_string());
    }
    let request = serde_json::from_str::<SourceAnalysisRequest>(&request_json)
        .map_err(|error| format!("Decode internal source analysis request failed: {error}"))?;
    let response = match execute_request(&request) {
        Ok(result) => SourceAnalysisResponse::Completed(result),
        Err(failure) => SourceAnalysisResponse::Failed(failure),
    };
    serde_json::to_string(&response)
        .map(Some)
        .map_err(|error| format!("Encode internal source analysis result failed: {error}"))
}

fn execute_request(
    request: &SourceAnalysisRequest,
) -> Result<SourceAnalysisResult, SourceProcessingFailure> {
    let cancel = AtomicBool::new(false);
    match &request.task {
        SourceAnalysisTask::ReadinessFeature {
            relative_path,
            content_hash,
            analysis_version,
        } => {
            let mut connection = open_source_connection(&request.source)?;
            let produced = wavecrate::internal_analysis_jobs::run_readiness_feature_stage(
                &mut connection,
                &request.source.root,
                request.source.id.as_str(),
                Path::new(relative_path),
                content_hash,
                analysis_version,
                &cancel,
            )
            .map_err(SourceProcessingFailure::from)?;
            Ok(SourceAnalysisResult {
                produced,
                processed: usize::from(produced),
                failed: 0,
                retired_cache_refs: 0,
                terminal_offline: false,
            })
        }
        SourceAnalysisTask::ReadinessEmbedding {
            relative_path,
            content_hash,
            analysis_version,
        } => {
            let mut connection = open_source_connection(&request.source)?;
            let produced = wavecrate::internal_analysis_jobs::run_readiness_embedding_stage(
                &mut connection,
                &request.source.root,
                request.source.id.as_str(),
                Path::new(relative_path),
                content_hash,
                analysis_version,
                &cancel,
            )
            .map_err(SourceProcessingFailure::from)?;
            Ok(SourceAnalysisResult {
                produced,
                processed: usize::from(produced),
                failed: 0,
                retired_cache_refs: 0,
                terminal_offline: false,
            })
        }
        SourceAnalysisTask::RetireDerivedState => {
            let outcome = super::supervisor::retire_source_derived_state(&request.source)
                .map_err(SourceProcessingFailure::from)?;
            let (retired_cache_refs, terminal_offline) = match outcome {
                super::supervisor::SourceRetirementOutcome::Retired { retired_cache_refs } => {
                    (retired_cache_refs, false)
                }
                super::supervisor::SourceRetirementOutcome::TerminalOffline => (0, true),
            };
            Ok(SourceAnalysisResult {
                produced: false,
                processed: 0,
                failed: 0,
                retired_cache_refs,
                terminal_offline,
            })
        }
    }
}

fn open_source_connection(
    source: &SampleSource,
) -> Result<rusqlite::Connection, SourceProcessingFailure> {
    let database_root = source.database_root().map_err(|error| {
        SourceProcessingFailure::permanent(
            SourceProcessingFailureCode::ExecutionUnclassified,
            "Resolve source database root failed",
            Some(error.to_string()),
        )
    })?;
    SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .map_err(source_database_failure)
}

pub(super) fn source_database_failure(error: SourceDbError) -> SourceProcessingFailure {
    match error {
        SourceDbError::Busy => SourceProcessingFailure::retryable(
            SourceProcessingFailureCode::SqliteBusy,
            "Source database is busy",
        ),
        error => SourceProcessingFailure::permanent(
            SourceProcessingFailureCode::ExecutionUnclassified,
            "Open source database failed",
            Some(error.to_string()),
        ),
    }
}

#[cfg(not(test))]
fn run_request_in_child(
    request: &SourceAnalysisRequest,
    cancel: &AtomicBool,
) -> Result<Option<SourceAnalysisResult>, SourceProcessingFailure> {
    if cancel.load(std::sync::atomic::Ordering::Acquire) {
        return Ok(None);
    }
    let executable = std::env::current_exe().map_err(|error| {
        SourceProcessingFailure::permanent(
            SourceProcessingFailureCode::WorkerProtocol,
            "Resolve source analysis executable failed",
            Some(error.to_string()),
        )
    })?;
    let request_json = serde_json::to_string(request).map_err(|error| {
        SourceProcessingFailure::permanent(
            SourceProcessingFailureCode::WorkerProtocol,
            "Encode source analysis request failed",
            Some(error.to_string()),
        )
    })?;
    let child = Command::new(executable)
        .arg(INTERNAL_SOURCE_ANALYSIS_ARG)
        .arg(request_json)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            SourceProcessingFailure::retryable(
                SourceProcessingFailureCode::WorkerProcessFailed,
                format!("Start source analysis process failed: {error}"),
            )
        })?;
    let Some(mut child) =
        child_process::wait_for_cancellable_child(child, cancel, "source analysis")
            .map_err(SourceProcessingFailure::from)?
    else {
        return Ok(None);
    };
    let mut stdout = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        pipe.read_to_string(&mut stdout).map_err(|error| {
            SourceProcessingFailure::permanent(
                SourceProcessingFailureCode::WorkerProtocol,
                "Read source analysis result failed",
                Some(error.to_string()),
            )
        })?;
    }
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        pipe.read_to_string(&mut stderr).map_err(|error| {
            SourceProcessingFailure::permanent(
                SourceProcessingFailureCode::WorkerProtocol,
                "Read source analysis error failed",
                Some(error.to_string()),
            )
        })?;
    }
    let status = child.wait().map_err(|error| {
        SourceProcessingFailure::retryable(
            SourceProcessingFailureCode::WorkerProcessFailed,
            format!("Join source analysis process failed: {error}"),
        )
    })?;
    if !status.success() {
        return Err(SourceProcessingFailure::retryable(
            SourceProcessingFailureCode::WorkerProcessFailed,
            format!(
                "Source analysis process failed with {status}: {}",
                stderr.trim()
            ),
        ));
    }
    match serde_json::from_str::<SourceAnalysisResponse>(stdout.trim()).map_err(|error| {
        SourceProcessingFailure::permanent(
            SourceProcessingFailureCode::WorkerProtocol,
            "Decode source analysis result failed",
            Some(error.to_string()),
        )
    })? {
        SourceAnalysisResponse::Completed(result) => Ok(Some(result)),
        SourceAnalysisResponse::Failed(failure) => Err(failure),
    }
}
