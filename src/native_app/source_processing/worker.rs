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
use wavecrate::sample_sources::{SampleSource, SourceDatabase, SourceDatabaseConnectionRole};

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
        run_request_in_child(&request, cancel).map(|result| {
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
    }
}

pub(super) fn run_readiness_feature_stage(
    connection: &mut rusqlite::Connection,
    source: &SampleSource,
    relative_path: &Path,
    content_hash: &str,
    analysis_version: &str,
    cancel: &AtomicBool,
) -> Result<bool, String> {
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
) -> Result<bool, String> {
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
    let result = execute_request(&request)?;
    serde_json::to_string(&result)
        .map(Some)
        .map_err(|error| format!("Encode internal source analysis result failed: {error}"))
}

fn execute_request(request: &SourceAnalysisRequest) -> Result<SourceAnalysisResult, String> {
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
            )?;
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
            )?;
            Ok(SourceAnalysisResult {
                produced,
                processed: usize::from(produced),
                failed: 0,
                retired_cache_refs: 0,
                terminal_offline: false,
            })
        }
        SourceAnalysisTask::RetireDerivedState => {
            let outcome = super::supervisor::retire_source_derived_state(&request.source)?;
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

fn open_source_connection(source: &SampleSource) -> Result<rusqlite::Connection, String> {
    let database_root = source.database_root().map_err(|error| error.to_string())?;
    SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .map_err(|error| error.to_string())
}

#[cfg(not(test))]
fn run_request_in_child(
    request: &SourceAnalysisRequest,
    cancel: &AtomicBool,
) -> Result<Option<SourceAnalysisResult>, String> {
    if cancel.load(std::sync::atomic::Ordering::Acquire) {
        return Ok(None);
    }
    let executable = std::env::current_exe()
        .map_err(|error| format!("Resolve source analysis executable failed: {error}"))?;
    let request_json = serde_json::to_string(request)
        .map_err(|error| format!("Encode internal source analysis request failed: {error}"))?;
    let child = Command::new(executable)
        .arg(INTERNAL_SOURCE_ANALYSIS_ARG)
        .arg(request_json)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Start source analysis process failed: {error}"))?;
    let Some(mut child) =
        child_process::wait_for_cancellable_child(child, cancel, "source analysis")?
    else {
        return Ok(None);
    };
    let mut stdout = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        pipe.read_to_string(&mut stdout)
            .map_err(|error| format!("Read source analysis result failed: {error}"))?;
    }
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        pipe.read_to_string(&mut stderr)
            .map_err(|error| format!("Read source analysis error failed: {error}"))?;
    }
    let status = child
        .wait()
        .map_err(|error| format!("Join source analysis process failed: {error}"))?;
    if !status.success() {
        return Err(format!(
            "Source analysis process failed with {status}: {}",
            stderr.trim()
        ));
    }
    serde_json::from_str::<SourceAnalysisResult>(stdout.trim())
        .map(Some)
        .map_err(|error| format!("Decode source analysis result failed: {error}"))
}
