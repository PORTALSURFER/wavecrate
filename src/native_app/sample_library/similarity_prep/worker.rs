#![cfg_attr(test, allow(dead_code))]

use std::{
    collections::HashSet,
    path::PathBuf,
    process::Child,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

#[cfg(not(test))]
use std::{
    io::Read,
    process::{Command, Stdio},
};

use wavecrate::sample_sources::config::{self, AnalysisSettings};
use wavecrate::sample_sources::{
    SampleSource, SourceDatabase, SourceDatabaseConnectionRole,
    db::{META_LAST_SCAN_COMPLETED_AT, META_LAST_SIMILARITY_PREP_SCAN_AT, META_WAV_PATHS_REVISION},
    readiness::{ReadinessScopeKind, ReadinessStage, ReadinessTarget},
    scanner::{self, ScanMode},
};
use wavecrate_analysis::{
    self as analysis,
    aspects::{ASPECT_DESCRIPTOR_DIM, ASPECT_DESCRIPTOR_DTYPE_F32, ASPECT_DESCRIPTOR_MODEL_ID},
    similarity::SIMILARITY_MODEL_ID,
};

use super::{NativeSimilarityPrepStatus, SimilarityPrepEnqueueSummary};

mod analysis_enqueue;

use analysis_enqueue::enqueue_analysis_backfill;

pub(in crate::native_app) use wavecrate::sample_sources::STARMAP_LAYOUT_UMAP_VERSION as NATIVE_SIMILARITY_UMAP_VERSION;
const NATIVE_SIMILARITY_CLUSTER_MIN_SIZE: usize = 10;
const ANALYZE_SAMPLE_JOB_TYPE: &str = "wav_metadata_v1";
const EMBEDDING_BACKFILL_JOB_TYPE: &str = "embedding_backfill_v1";
const SIMILARITY_BUSY_RETRY_ATTEMPTS: usize = 3;
const SIMILARITY_BUSY_RETRY_DELAY: Duration = Duration::from_millis(250);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::native_app) struct SimilarityPrepJobDrainSummary {
    pub(in crate::native_app) processed: usize,
    pub(in crate::native_app) failed: usize,
}

const INTERNAL_SIMILARITY_FINALIZER_ARG: &str = "--wavecrate-internal-similarity-finalizer-v1";
const FINALIZER_POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub(in crate::native_app) enum SimilarityPublicationFence {
    Readiness {
        source_id: String,
        source_generation: i64,
        membership_generation: String,
        artifact_version: String,
    },
    LegacyPathsRevision(i64),
}

impl SimilarityPublicationFence {
    pub(in crate::native_app) fn for_readiness_target(
        target: &ReadinessTarget,
    ) -> Result<Self, String> {
        if target.scope_kind != ReadinessScopeKind::Source
            || target.stage != ReadinessStage::SimilarityLayout
        {
            return Err("similarity publication requires a source-level layout target".to_string());
        }
        Ok(Self::Readiness {
            source_id: target.source_id.clone(),
            source_generation: target.source_generation,
            membership_generation: target.content_generation.clone(),
            artifact_version: target.required_version.clone(),
        })
    }

    pub(in crate::native_app) fn legacy_paths_revision(revision: i64) -> Self {
        Self::LegacyPathsRevision(revision)
    }

    fn is_current(&self, connection: &rusqlite::Connection) -> Result<bool, String> {
        match self {
            Self::Readiness {
                source_id,
                source_generation,
                membership_generation,
                artifact_version,
            } => connection
                .query_row(
                    "SELECT EXISTS(
                        SELECT 1
                        FROM source_readiness_sources AS source
                        JOIN source_readiness_targets AS target
                          ON target.source_id = source.source_id
                        WHERE source.source_id = ?1
                          AND source.source_generation = ?2
                          AND source.availability = 'active'
                          AND target.scope_kind = 'source'
                          AND target.scope_id = ?1
                          AND target.stage = 'similarity_layout'
                          AND target.required_version = ?3
                          AND target.source_generation = ?2
                          AND target.content_generation = ?4
                          AND target.eligibility = 'eligible'
                    )",
                    rusqlite::params![
                        source_id,
                        source_generation,
                        artifact_version,
                        membership_generation,
                    ],
                    |row| row.get(0),
                )
                .map_err(|error| {
                    format!("Validate similarity readiness generation failed: {error}")
                }),
            Self::LegacyPathsRevision(revision) => connection
                .query_row(
                    "SELECT COALESCE(
                        (SELECT CAST(value AS INTEGER) FROM metadata WHERE key = ?1),
                        0
                    ) = ?2",
                    rusqlite::params![META_WAV_PATHS_REVISION, revision],
                    |row| row.get(0),
                )
                .map_err(|error| format!("Validate source paths revision failed: {error}")),
        }
    }
}

#[cfg(test)]
pub(super) fn enqueue_similarity_prep_inner(
    source: &SampleSource,
    automatic: bool,
) -> Result<SimilarityPrepEnqueueSummary, String> {
    enqueue_similarity_prep_inner_with_cancel(source, automatic, None)
}

pub(super) fn enqueue_similarity_prep_inner_with_cancel(
    source: &SampleSource,
    automatic: bool,
    cancel: Option<&AtomicBool>,
) -> Result<SimilarityPrepEnqueueSummary, String> {
    retry_similarity_prep_busy(source, || {
        enqueue_similarity_prep_once(source, automatic, cancel)
    })
}

fn enqueue_similarity_prep_once(
    source: &SampleSource,
    automatic: bool,
    cancel: Option<&AtomicBool>,
) -> Result<SimilarityPrepEnqueueSummary, String> {
    ensure_not_cancelled(cancel)?;
    ensure_source_database_scanned(source, cancel)?;
    ensure_not_cancelled(cancel)?;
    let initial_status = resolve_similarity_prep_status(source)?;
    if initial_status == NativeSimilarityPrepStatus::UpToDate
        || (automatic
            && matches!(initial_status, NativeSimilarityPrepStatus::Blocked { .. })
            && !source_has_active_similarity_prep_jobs(source)?)
    {
        return Ok(SimilarityPrepEnqueueSummary {
            analysis_inserted: 0,
            embedding_inserted: 0,
            jobs_processed: 0,
            jobs_failed: 0,
            finalized: false,
            status: initial_status,
        });
    }
    let analysis_inserted = enqueue_analysis_backfill(source)?;
    ensure_not_cancelled(cancel)?;
    let embedding_inserted = enqueue_embedding_backfill(source)?;
    ensure_not_cancelled(cancel)?;
    let status = resolve_similarity_prep_status(source)?;
    Ok(SimilarityPrepEnqueueSummary {
        analysis_inserted,
        embedding_inserted,
        jobs_processed: 0,
        jobs_failed: 0,
        finalized: false,
        status,
    })
}

fn ensure_not_cancelled(cancel: Option<&AtomicBool>) -> Result<(), String> {
    if cancel.is_some_and(|cancel| cancel.load(Ordering::Acquire)) {
        Err(String::from("Similarity preparation canceled"))
    } else {
        Ok(())
    }
}

fn retry_similarity_prep_busy<T>(
    source: &SampleSource,
    mut operation: impl FnMut() -> Result<T, String>,
) -> Result<T, String> {
    let started_at = Instant::now();
    for attempt in 0..SIMILARITY_BUSY_RETRY_ATTEMPTS {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error)
                if attempt + 1 < SIMILARITY_BUSY_RETRY_ATTEMPTS
                    && is_transient_database_busy(&error) =>
            {
                tracing::debug!(
                    source = %source.root.display(),
                    attempt = attempt + 1,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    error = %error,
                    "similarity prep source database busy; retrying"
                );
                std::thread::sleep(SIMILARITY_BUSY_RETRY_DELAY);
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("similarity prep retry loop must return from every attempt")
}

fn is_transient_database_busy(error: &str) -> bool {
    let lowered = error.to_ascii_lowercase();
    lowered.contains("database is busy")
        || lowered.contains("database is locked")
        || lowered.contains("sqlite_busy")
}

pub(in crate::native_app) fn finalize_similarity_prep_if_ready(
    source: &SampleSource,
    publication_fence: &SimilarityPublicationFence,
    cancel: &AtomicBool,
) -> Result<bool, String> {
    #[cfg(test)]
    {
        return finalize_if_ready(source, publication_fence, cancel);
    }
    #[cfg(not(test))]
    {
        finalize_similarity_prep_in_child(source, publication_fence, cancel)
    }
}

pub(in crate::native_app) fn run_internal_similarity_finalizer_from_args()
-> Result<Option<bool>, String> {
    let mut args = std::env::args();
    let _executable = args.next();
    if args.next().as_deref() != Some(INTERNAL_SIMILARITY_FINALIZER_ARG) {
        return Ok(None);
    }
    let source_json = args
        .next()
        .ok_or_else(|| "Internal similarity finalizer is missing its source".to_string())?;
    let fence_json = args.next().ok_or_else(|| {
        "Internal similarity finalizer is missing its publication fence".to_string()
    })?;
    if args.next().is_some() {
        return Err("Internal similarity finalizer received unexpected arguments".to_string());
    }
    let source = serde_json::from_str::<SampleSource>(&source_json)
        .map_err(|error| format!("Decode internal similarity source failed: {error}"))?;
    let publication_fence = serde_json::from_str::<SimilarityPublicationFence>(&fence_json)
        .map_err(|error| format!("Decode internal similarity fence failed: {error}"))?;
    let cancel = AtomicBool::new(false);
    finalize_if_ready(&source, &publication_fence, &cancel).map(Some)
}

#[cfg(not(test))]
fn finalize_similarity_prep_in_child(
    source: &SampleSource,
    publication_fence: &SimilarityPublicationFence,
    cancel: &AtomicBool,
) -> Result<bool, String> {
    if cancel.load(Ordering::Acquire) {
        return Ok(false);
    }
    let executable = std::env::current_exe()
        .map_err(|error| format!("Resolve similarity finalizer executable failed: {error}"))?;
    let source_json = serde_json::to_string(source)
        .map_err(|error| format!("Encode internal similarity source failed: {error}"))?;
    let fence_json = serde_json::to_string(publication_fence)
        .map_err(|error| format!("Encode internal similarity fence failed: {error}"))?;
    let child = Command::new(executable)
        .arg(INTERNAL_SIMILARITY_FINALIZER_ARG)
        .arg(source_json)
        .arg(fence_json)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Start similarity finalizer process failed: {error}"))?;
    let Some(mut child) = wait_for_cancellable_child(child, cancel)? else {
        return Ok(false);
    };
    let mut stdout = String::new();
    if let Some(mut pipe) = child.stdout.take() {
        pipe.read_to_string(&mut stdout)
            .map_err(|error| format!("Read similarity finalizer result failed: {error}"))?;
    }
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        pipe.read_to_string(&mut stderr)
            .map_err(|error| format!("Read similarity finalizer error failed: {error}"))?;
    }
    let status = child
        .wait()
        .map_err(|error| format!("Join similarity finalizer process failed: {error}"))?;
    if !status.success() {
        return Err(format!(
            "Similarity finalizer process failed with {status}: {}",
            stderr.trim()
        ));
    }
    serde_json::from_str::<bool>(stdout.trim())
        .map_err(|error| format!("Decode similarity finalizer result failed: {error}"))
}

fn wait_for_cancellable_child(
    mut child: Child,
    cancel: &AtomicBool,
) -> Result<Option<Child>, String> {
    loop {
        if cancel.load(Ordering::Acquire) {
            if let Err(error) = child.kill()
                && child
                    .try_wait()
                    .map_err(|poll_error| {
                        format!(
                            "Cancel similarity finalizer process failed: {error}; poll failed: {poll_error}"
                        )
                    })?
                    .is_none()
            {
                return Err(format!(
                    "Cancel similarity finalizer process failed: {error}"
                ));
            }
            child
                .wait()
                .map_err(|error| format!("Join cancelled similarity finalizer failed: {error}"))?;
            return Ok(None);
        }
        if child
            .try_wait()
            .map_err(|error| format!("Poll similarity finalizer process failed: {error}"))?
            .is_some()
        {
            return Ok(Some(child));
        }
        std::thread::sleep(FINALIZER_POLL_INTERVAL);
    }
}

fn finalize_if_ready(
    source: &SampleSource,
    publication_fence: &SimilarityPublicationFence,
    cancel: &AtomicBool,
) -> Result<bool, String> {
    if cancel.load(Ordering::Acquire) {
        return Ok(false);
    }
    if source_has_active_similarity_prep_jobs(source)? {
        return Ok(false);
    }
    if current_similarity_sample_ids(source)?.is_empty() {
        if let Some(scan_completed_at) = read_source_scan_timestamp(source)? {
            return set_source_prep_timestamp_if_current(
                source,
                scan_completed_at,
                publication_fence,
            );
        }
        return Ok(false);
    }
    if !source_has_embeddings(source)? || !source_has_aspect_descriptors(source)? {
        return Ok(false);
    }
    let mut conn = open_source_db(source)?;
    let fence = |connection: &rusqlite::Connection| publication_fence.is_current(connection);
    if analysis::build_map_layout_with_cancel_and_publication_fence(
        &mut conn,
        SIMILARITY_MODEL_ID,
        NATIVE_SIMILARITY_UMAP_VERSION,
        0,
        0.95,
        cancel,
        &fence,
    )?
    .is_none()
    {
        return Ok(false);
    }
    if cancel.load(Ordering::Acquire) {
        return Ok(false);
    }
    let sample_id_prefix = format!("{}::%", source.id.as_str());
    let layout_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM layout_umap
             WHERE model_id = ?1 AND umap_version = ?2 AND sample_id LIKE ?3",
            rusqlite::params![
                SIMILARITY_MODEL_ID,
                NATIVE_SIMILARITY_UMAP_VERSION,
                sample_id_prefix
            ],
            |row| row.get(0),
        )
        .map_err(|err| format!("Count similarity layout rows failed: {err}"))?;
    if layout_rows == 0 {
        return Ok(false);
    }
    if analysis::hdbscan::build_hdbscan_clusters_for_sample_id_prefix_with_cancel_and_publication_fence(
        &mut conn,
        SIMILARITY_MODEL_ID,
        analysis::hdbscan::HdbscanMethod::Umap,
        Some(NATIVE_SIMILARITY_UMAP_VERSION),
        Some(sample_id_prefix.as_str()),
        analysis::hdbscan::HdbscanConfig {
            min_cluster_size: NATIVE_SIMILARITY_CLUSTER_MIN_SIZE,
            min_samples: None,
            allow_single_cluster: false,
        },
        cancel,
        &fence,
    )?
    .is_none()
    {
        return Ok(false);
    }
    if cancel.load(Ordering::Acquire) {
        return Ok(false);
    }
    if !analysis::flush_ann_index_with_publication_fence(&mut conn, &fence)? {
        return Ok(false);
    }
    if cancel.load(Ordering::Acquire) {
        return Ok(false);
    }
    if let Some(scan_completed_at) = read_source_scan_timestamp(source)? {
        return set_source_prep_timestamp_if_current(source, scan_completed_at, publication_fence);
    }
    Ok(true)
}

pub(in crate::native_app) fn reset_interrupted_similarity_prep_jobs(
    source: &SampleSource,
) -> Result<usize, String> {
    let conn = open_source_db(source)?;
    wavecrate::internal_analysis_jobs::reset_running_to_pending(&conn)
}

#[cfg(test)]
pub(in crate::native_app) fn run_similarity_prep_job_batch(
    source: &SampleSource,
    limit: usize,
    cancel: &AtomicBool,
) -> Result<SimilarityPrepJobDrainSummary, String> {
    let mut conn = open_source_db(source)?;
    let settings = load_analysis_settings();
    let runtime = SimilarityPrepJobRuntime::from_settings(&settings);
    let mut summary = SimilarityPrepJobDrainSummary::default();
    let jobs = wavecrate::internal_analysis_jobs::claim_next_jobs(&mut conn, &source.root, limit)?;
    for job in jobs {
        if cancel.load(Ordering::Acquire) {
            wavecrate::internal_analysis_jobs::release(&conn, &job)?;
            continue;
        }
        let outcome = wavecrate::internal_analysis_jobs::run_claimed_job(
            &mut conn,
            &job,
            true,
            runtime.max_analysis_duration_seconds,
            runtime.analysis_sample_rate,
            runtime.analysis_version.as_str(),
            cancel,
        );
        if let Err(error) = outcome.as_ref() {
            summary.failed += 1;
            wavecrate::internal_analysis_jobs::mark_failed_with_reason(&conn, &job, error)?;
        } else {
            wavecrate::internal_analysis_jobs::mark_done(&conn, &job)?;
        }
        summary.processed += 1;
    }
    Ok(summary)
}

pub(in crate::native_app) fn run_similarity_prep_job(
    source: &SampleSource,
    job_id: i64,
    cancel: &AtomicBool,
    embedding_worker_limit: usize,
) -> Result<SimilarityPrepJobDrainSummary, String> {
    let mut conn = open_source_db(source)?;
    let settings = load_analysis_settings();
    let runtime = SimilarityPrepJobRuntime::from_settings(&settings);
    let Some(job) =
        wavecrate::internal_analysis_jobs::claim_job_by_id(&mut conn, &source.root, job_id)?
    else {
        return Ok(SimilarityPrepJobDrainSummary::default());
    };
    if cancel.load(Ordering::Acquire) {
        wavecrate::internal_analysis_jobs::release(&conn, &job)?;
        return Ok(SimilarityPrepJobDrainSummary::default());
    }
    let outcome = wavecrate::internal_analysis_jobs::run_claimed_job_with_embedding_worker_limit(
        &mut conn,
        &job,
        true,
        runtime.max_analysis_duration_seconds,
        runtime.analysis_sample_rate,
        runtime.analysis_version.as_str(),
        cancel,
        embedding_worker_limit,
    );
    if cancel.load(Ordering::Acquire) {
        wavecrate::internal_analysis_jobs::release(&conn, &job)?;
        return Ok(SimilarityPrepJobDrainSummary::default());
    }
    let mut summary = SimilarityPrepJobDrainSummary {
        processed: 1,
        failed: usize::from(outcome.is_err()),
    };
    if let Err(error) = outcome {
        wavecrate::internal_analysis_jobs::mark_failed_with_reason(&conn, &job, &error)?;
    } else {
        wavecrate::internal_analysis_jobs::mark_done(&conn, &job)?;
    }
    if cancel.load(Ordering::Acquire) {
        summary.processed = 0;
    }
    Ok(summary)
}

#[cfg(test)]
pub(in crate::native_app) fn similarity_prep_has_pending_jobs(
    source: &SampleSource,
) -> Result<bool, String> {
    source_has_active_similarity_prep_jobs(source)
}

pub(in crate::native_app) fn similarity_prep_needs_finalization(
    source: &SampleSource,
) -> Result<bool, String> {
    if source_has_active_similarity_prep_jobs(source)? {
        return Ok(false);
    }
    if resolve_similarity_prep_status(source)? == NativeSimilarityPrepStatus::UpToDate {
        return Ok(false);
    }
    Ok((read_source_scan_timestamp(source)?.is_some()
        && current_similarity_sample_ids(source)?.is_empty())
        || (source_has_embeddings(source)? && source_has_aspect_descriptors(source)?))
}

pub(super) fn source_has_active_similarity_prep_jobs(
    source: &SampleSource,
) -> Result<bool, String> {
    let conn = open_source_db(source)?;
    let active: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM analysis_jobs
             WHERE source_id = ?1
               AND job_type IN (?2, ?3)
               AND status IN ('pending','running')",
            rusqlite::params![
                source.id.as_str(),
                ANALYZE_SAMPLE_JOB_TYPE,
                EMBEDDING_BACKFILL_JOB_TYPE,
            ],
            |row| row.get(0),
        )
        .map_err(|err| format!("Count active similarity prep jobs failed: {err}"))?;
    Ok(active > 0)
}

#[derive(Clone, Debug, PartialEq)]
struct SimilarityPrepJobRuntime {
    max_analysis_duration_seconds: f32,
    analysis_sample_rate: u32,
    analysis_version: String,
}

impl SimilarityPrepJobRuntime {
    fn from_settings(settings: &AnalysisSettings) -> Self {
        let max_analysis_duration_seconds = if settings.limit_similarity_prep_duration {
            settings.max_analysis_duration_seconds
        } else {
            0.0
        };
        let analysis_sample_rate = if settings.fast_similarity_prep {
            settings
                .fast_similarity_prep_sample_rate
                .clamp(1, wavecrate_analysis::ANALYSIS_SAMPLE_RATE)
        } else {
            wavecrate_analysis::ANALYSIS_SAMPLE_RATE
        };
        let analysis_version = if settings.fast_similarity_prep {
            wavecrate_analysis::analysis_version_for_sample_rate(analysis_sample_rate)
        } else {
            wavecrate_analysis::analysis_version().to_string()
        };
        Self {
            max_analysis_duration_seconds,
            analysis_sample_rate,
            analysis_version,
        }
    }
}

fn load_analysis_settings() -> AnalysisSettings {
    config::load_or_default()
        .map(|config| config.core.analysis)
        .unwrap_or_default()
}

fn ensure_source_database_scanned(
    source: &SampleSource,
    cancel: Option<&AtomicBool>,
) -> Result<(), String> {
    let db = open_fast_source_db(source).map_err(|err| err.to_string())?;
    let has_scan_timestamp = db
        .get_metadata(META_LAST_SCAN_COMPLETED_AT)
        .map_err(|err| err.to_string())?
        .is_some();
    if has_scan_timestamp {
        return Ok(());
    }
    let stats = scanner::scan_with_progress(&db, ScanMode::Quick, cancel, &mut |_, _| {})
        .map_err(|err| format!("Sync source index failed: {err}"))?;
    scanner::complete_deferred_rename_candidates_with_cancel(&db, stats, cancel)
        .map_err(|err| format!("Finish deferred rename reconciliation failed: {err}"))?;
    Ok(())
}

pub(in crate::native_app) fn resolve_similarity_prep_status(
    source: &SampleSource,
) -> Result<NativeSimilarityPrepStatus, String> {
    let facts = SimilarityPrepFacts {
        scan_completed_at: read_source_scan_timestamp(source)?,
        prep_completed_at: read_source_prep_timestamp(source)?,
        has_embeddings: source_has_embeddings(source)?,
        has_aspects: source_has_aspect_descriptors(source)?,
        has_layout: source_has_layout(source)?,
        has_active_jobs: source_has_active_similarity_prep_jobs(source)?,
        failures: similarity_failure_counts(source)?,
    };
    Ok(resolve_similarity_prep_facts(facts))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SimilarityPrepFacts {
    scan_completed_at: Option<i64>,
    prep_completed_at: Option<i64>,
    has_embeddings: bool,
    has_aspects: bool,
    has_layout: bool,
    has_active_jobs: bool,
    failures: Option<SimilarityPrepFailureCounts>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SimilarityPrepFailureCounts {
    failed_count: usize,
    unsupported_count: usize,
}

fn resolve_similarity_prep_facts(facts: SimilarityPrepFacts) -> NativeSimilarityPrepStatus {
    if facts.scan_completed_at.is_some()
        && facts.scan_completed_at == facts.prep_completed_at
        && facts.has_embeddings
        && facts.has_aspects
        && facts.has_layout
        && !facts.has_active_jobs
    {
        return NativeSimilarityPrepStatus::UpToDate;
    }
    if let Some(failures) = facts.failures
        && failures.failed_count > 0
    {
        return NativeSimilarityPrepStatus::Blocked {
            failed_count: failures.failed_count,
            unsupported_count: failures.unsupported_count,
        };
    }
    if facts.has_active_jobs
        || (facts.scan_completed_at.is_some() && facts.scan_completed_at != facts.prep_completed_at)
    {
        return NativeSimilarityPrepStatus::Outdated;
    }
    NativeSimilarityPrepStatus::MissingArtifacts {
        missing_embeddings: !facts.has_embeddings,
        missing_aspects: !facts.has_aspects,
        missing_layout: !facts.has_layout,
    }
}

fn similarity_failure_counts(
    source: &SampleSource,
) -> Result<Option<SimilarityPrepFailureCounts>, String> {
    let failures = failed_samples_for_source(source)?;
    let unsupported_count = failures
        .values()
        .filter(|message| message.to_ascii_lowercase().contains("unsupported"))
        .count();
    Ok(Some(SimilarityPrepFailureCounts {
        failed_count: failures.len(),
        unsupported_count,
    }))
}

fn read_source_scan_timestamp(source: &SampleSource) -> Result<Option<i64>, String> {
    read_source_timestamp(source, META_LAST_SCAN_COMPLETED_AT)
}

fn read_source_prep_timestamp(source: &SampleSource) -> Result<Option<i64>, String> {
    read_source_timestamp(source, META_LAST_SIMILARITY_PREP_SCAN_AT)
}

fn read_source_timestamp(source: &SampleSource, key: &str) -> Result<Option<i64>, String> {
    let db = open_fast_source_db(source).map_err(|err| err.to_string())?;
    db.get_metadata(key)
        .map_err(|err| err.to_string())?
        .map(|value| {
            value
                .parse::<i64>()
                .map_err(|err| format!("Invalid {key} metadata: {err}"))
        })
        .transpose()
}

fn set_source_prep_timestamp_if_current(
    source: &SampleSource,
    value: i64,
    publication_fence: &SimilarityPublicationFence,
) -> Result<bool, String> {
    let mut connection = open_source_db(source)?;
    let tx = connection
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|error| format!("Start similarity completion transaction failed: {error}"))?;
    if !publication_fence.is_current(&tx)? {
        tx.rollback()
            .map_err(|error| format!("Roll back stale similarity completion failed: {error}"))?;
        return Ok(false);
    }
    tx.execute(
        "INSERT INTO metadata (key, value)
         VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![META_LAST_SIMILARITY_PREP_SCAN_AT, value.to_string()],
    )
    .map_err(|error| format!("Write similarity completion timestamp failed: {error}"))?;
    tx.execute(
        "INSERT INTO metadata (key, value)
         VALUES ('revision', '1')
         ON CONFLICT(key) DO UPDATE SET value = CAST(CAST(value AS INTEGER) + 1 AS TEXT)",
        [],
    )
    .map_err(|error| format!("Advance source metadata revision failed: {error}"))?;
    tx.commit()
        .map_err(|error| format!("Commit similarity completion failed: {error}"))?;
    Ok(true)
}

fn source_has_embeddings(source: &SampleSource) -> Result<bool, String> {
    let sample_ids = current_similarity_sample_ids(source)?;
    if sample_ids.is_empty() {
        return Ok(true);
    }
    let conn = open_source_db(source)?;
    let sample_id_prefix = format!("{}::%", source.id.as_str());
    sample_ids_covered(
        &conn,
        "SELECT sample_id FROM embeddings WHERE model_id = ?1 AND sample_id LIKE ?2",
        rusqlite::params![SIMILARITY_MODEL_ID, sample_id_prefix],
        &sample_ids,
    )
}

fn source_has_layout(source: &SampleSource) -> Result<bool, String> {
    let sample_ids = current_similarity_sample_ids(source)?;
    if sample_ids.is_empty() {
        return Ok(true);
    }
    let conn = open_source_db(source)?;
    let sample_id_prefix = format!("{}::%", source.id.as_str());
    sample_ids_covered(
        &conn,
        "SELECT sample_id FROM layout_umap
         WHERE model_id = ?1 AND umap_version = ?2 AND sample_id LIKE ?3",
        rusqlite::params![
            SIMILARITY_MODEL_ID,
            NATIVE_SIMILARITY_UMAP_VERSION,
            sample_id_prefix
        ],
        &sample_ids,
    )
}

fn source_has_aspect_descriptors(source: &SampleSource) -> Result<bool, String> {
    let sample_ids = current_similarity_sample_ids(source)?;
    if sample_ids.is_empty() {
        return Ok(true);
    }
    let conn = open_source_db(source)?;
    let sample_id_prefix = format!("{}::%", source.id.as_str());
    sample_ids_covered(
        &conn,
        "SELECT sample_id FROM similarity_aspect_descriptors
         WHERE model_id = ?1
           AND dim = ?2
           AND dtype = ?3
           AND l2_normed = 1
           AND sample_id LIKE ?4",
        rusqlite::params![
            ASPECT_DESCRIPTOR_MODEL_ID,
            ASPECT_DESCRIPTOR_DIM as i64,
            ASPECT_DESCRIPTOR_DTYPE_F32,
            sample_id_prefix,
        ],
        &sample_ids,
    )
}

fn current_present_sample_ids(source: &SampleSource) -> Result<Vec<String>, String> {
    let db = open_fast_source_db(source).map_err(|err| err.to_string())?;
    let entries = db.list_files().map_err(|err| err.to_string())?;
    Ok(entries
        .into_iter()
        .filter(|entry| !entry.missing)
        .map(|entry| build_sample_id(source.id.as_str(), &entry.relative_path))
        .collect())
}

fn current_similarity_sample_ids(source: &SampleSource) -> Result<Vec<String>, String> {
    let unsupported = unsupported_sample_ids_for_source(source)?;
    let mut sample_ids = current_present_sample_ids(source)?;
    sample_ids.retain(|sample_id| !unsupported.contains(sample_id));
    Ok(sample_ids)
}

fn sample_ids_covered<P>(
    conn: &rusqlite::Connection,
    sql: &str,
    params: P,
    sample_ids: &[String],
) -> Result<bool, String>
where
    P: rusqlite::Params,
{
    let covered = sample_id_set(conn, sql, params)?;
    Ok(sample_ids
        .iter()
        .all(|sample_id| covered.contains(sample_id)))
}

fn enqueue_embedding_backfill(source: &SampleSource) -> Result<usize, String> {
    let mut conn = open_source_db(source)?;
    if active_jobs_exist(&conn, source.id.as_str(), EMBEDDING_BACKFILL_JOB_TYPE)? {
        return Ok(0);
    }
    let sample_ids = sample_ids_missing_similarity_artifacts(&conn, source)?;
    if sample_ids.is_empty() {
        return Ok(0);
    }
    let created_at = now_epoch_seconds();
    let tx = conn
        .transaction()
        .map_err(|err| format!("Start embedding enqueue transaction failed: {err}"))?;
    let mut inserted = 0usize;
    for (idx, chunk) in sample_ids.chunks(32).enumerate() {
        let job_sample_id = format!("{}::embed_backfill::{}", source.id.as_str(), idx);
        let payload =
            serde_json::to_string(chunk).map_err(|err| format!("Encode embedding job: {err}"))?;
        upsert_analysis_job(
            &tx,
            &job_sample_id,
            source.id.as_str(),
            std::path::Path::new("embed_backfill"),
            EMBEDDING_BACKFILL_JOB_TYPE,
            &payload,
            created_at,
        )?;
        inserted += 1;
    }
    tx.commit()
        .map_err(|err| format!("Commit embedding enqueue failed: {err}"))?;
    Ok(inserted)
}

fn active_jobs_exist(
    conn: &rusqlite::Connection,
    source_id: &str,
    job_type: &str,
) -> Result<bool, String> {
    let active: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM analysis_jobs
             WHERE source_id = ?1 AND job_type = ?2 AND status IN ('pending','running')",
            rusqlite::params![source_id, job_type],
            |row| row.get(0),
        )
        .map_err(|err| format!("Count active analysis jobs failed: {err}"))?;
    Ok(active > 0)
}

fn upsert_analysis_job(
    tx: &rusqlite::Transaction<'_>,
    sample_id: &str,
    source_id: &str,
    relative_path: &std::path::Path,
    job_type: &str,
    content_hash: &str,
    created_at: i64,
) -> Result<(), String> {
    tx.execute(
        "INSERT INTO analysis_jobs
         (sample_id, source_id, relative_path, job_type, content_hash, status, attempts, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'pending', 0, ?6)
         ON CONFLICT(sample_id, job_type) DO UPDATE SET
           source_id = excluded.source_id,
           relative_path = excluded.relative_path,
           content_hash = excluded.content_hash,
           status = 'pending',
           attempts = 0,
           created_at = excluded.created_at,
           running_at = NULL,
           last_error = NULL",
        rusqlite::params![
            sample_id,
            source_id,
            relative_path.to_string_lossy().replace('\\', "/"),
            job_type,
            content_hash,
            created_at,
        ],
    )
    .map(|_| ())
    .map_err(|err| format!("Enqueue analysis job failed: {err}"))
}

fn sample_ids_missing_similarity_artifacts(
    conn: &rusqlite::Connection,
    source: &SampleSource,
) -> Result<Vec<String>, String> {
    let sample_ids = current_similarity_sample_ids(source)?;
    if sample_ids.is_empty() {
        return Ok(Vec::new());
    }
    let sample_id_prefix = format!("{}::%", source.id.as_str());
    let embeddings = sample_id_set(
        conn,
        "SELECT sample_id FROM embeddings WHERE model_id = ?1 AND sample_id LIKE ?2",
        rusqlite::params![SIMILARITY_MODEL_ID, sample_id_prefix.as_str()],
    )?;
    let aspects = sample_id_set(
        conn,
        "SELECT sample_id FROM similarity_aspect_descriptors
         WHERE model_id = ?1
           AND dim = ?2
           AND dtype = ?3
           AND l2_normed = 1
           AND sample_id LIKE ?4",
        rusqlite::params![
            ASPECT_DESCRIPTOR_MODEL_ID,
            ASPECT_DESCRIPTOR_DIM as i64,
            ASPECT_DESCRIPTOR_DTYPE_F32,
            sample_id_prefix.as_str(),
        ],
    )?;
    Ok(sample_ids
        .into_iter()
        .filter(|sample_id| !embeddings.contains(sample_id) || !aspects.contains(sample_id))
        .collect())
}

fn sample_id_set<P>(
    conn: &rusqlite::Connection,
    sql: &str,
    params: P,
) -> Result<HashSet<String>, String>
where
    P: rusqlite::Params,
{
    let mut stmt = conn
        .prepare(sql)
        .map_err(|err| format!("Prepare similarity sample id query failed: {err}"))?;
    stmt.query_map(params, |row| row.get::<_, String>(0))
        .map_err(|err| format!("Query similarity sample ids failed: {err}"))?
        .collect::<Result<HashSet<_>, _>>()
        .map_err(|err| format!("Decode similarity sample ids failed: {err}"))
}

fn unsupported_sample_ids_for_source(source: &SampleSource) -> Result<HashSet<String>, String> {
    let conn = open_source_db(source)?;
    let mut stmt = conn
        .prepare(
            "SELECT aj.sample_id
             FROM analysis_jobs aj
             JOIN wav_files wf
               ON wf.path = aj.relative_path
              AND wf.missing = 0
             WHERE aj.source_id = ?1
               AND aj.job_type = ?2
               AND aj.status = 'failed'
               AND aj.content_hash = COALESCE(
                   wf.content_hash,
                   'fast-' || wf.file_size || '-' || wf.modified_ns
               )
               AND (
                   lower(COALESCE(aj.last_error, '')) LIKE '%unsupported%'
                   OR lower(COALESCE(aj.last_error, '')) LIKE '%no suitable format reader%'
               )",
        )
        .map_err(|err| format!("Prepare unsupported similarity skip query failed: {err}"))?;
    stmt.query_map(
        rusqlite::params![source.id.as_str(), ANALYZE_SAMPLE_JOB_TYPE],
        |row| row.get::<_, String>(0),
    )
    .map_err(|err| format!("Query unsupported similarity skips failed: {err}"))?
    .collect::<Result<HashSet<_>, _>>()
    .map_err(|err| format!("Decode unsupported similarity skips failed: {err}"))
}

fn failed_samples_for_source(
    source: &SampleSource,
) -> Result<std::collections::HashMap<PathBuf, String>, String> {
    let conn = open_source_db(source)?;
    let mut stmt = conn
        .prepare(
            "SELECT aj.relative_path, aj.last_error
             FROM analysis_jobs aj
             JOIN wav_files wf
               ON wf.path = aj.relative_path
              AND wf.missing = 0
             LEFT JOIN samples s ON s.sample_id = aj.sample_id
             LEFT JOIN features f
               ON f.sample_id = aj.sample_id AND f.feat_version = ?2
             LEFT JOIN embeddings e
               ON e.sample_id = aj.sample_id AND e.model_id = ?3
             LEFT JOIN similarity_aspect_descriptors a
               ON a.sample_id = aj.sample_id
              AND a.model_id = ?5
              AND a.dim = ?6
              AND a.dtype = ?7
              AND a.l2_normed = 1
             WHERE aj.status = 'failed'
               AND aj.source_id = ?1
               AND aj.job_type = ?8
               AND aj.content_hash = COALESCE(
                   wf.content_hash,
                   'fast-' || wf.file_size || '-' || wf.modified_ns
               )
               AND lower(COALESCE(aj.last_error, '')) NOT LIKE '%unsupported%'
               AND lower(COALESCE(aj.last_error, '')) NOT LIKE '%no suitable format reader%'
               AND (
                  f.sample_id IS NULL
                  OR s.analysis_version IS NULL
                  OR s.analysis_version != ?4
                  OR e.sample_id IS NULL
                  OR a.sample_id IS NULL
               )
             UNION ALL
             SELECT aj.relative_path, aj.last_error
             FROM analysis_jobs aj
             WHERE aj.status = 'failed'
               AND aj.source_id = ?1
               AND aj.job_type = ?9
               AND lower(COALESCE(aj.last_error, '')) NOT LIKE '%unsupported%'
               AND lower(COALESCE(aj.last_error, '')) NOT LIKE '%no suitable format reader%'
             ORDER BY 1 ASC",
        )
        .map_err(|err| format!("Failed to query failed analysis jobs: {err}"))?;
    let rows = stmt
        .query_map(
            rusqlite::params![
                source.id.as_str(),
                1i64,
                SIMILARITY_MODEL_ID,
                wavecrate_analysis::analysis_version(),
                ASPECT_DESCRIPTOR_MODEL_ID,
                ASPECT_DESCRIPTOR_DIM as i64,
                ASPECT_DESCRIPTOR_DTYPE_F32,
                ANALYZE_SAMPLE_JOB_TYPE,
                EMBEDDING_BACKFILL_JOB_TYPE,
            ],
            |row| {
                let relative_path: String = row.get(0)?;
                let last_error: Option<String> = row.get(1)?;
                Ok((relative_path, last_error))
            },
        )
        .map_err(|err| format!("Failed to query failed analysis jobs: {err}"))?;
    let mut failures = std::collections::HashMap::new();
    for row in rows {
        let (relative_path, last_error) =
            row.map_err(|err| format!("Failed to decode failed analysis job row: {err}"))?;
        failures.insert(
            PathBuf::from(relative_path),
            last_error.unwrap_or_else(|| String::from("Analysis failed")),
        );
    }
    Ok(failures)
}

fn build_sample_id(source_id: &str, relative_path: &std::path::Path) -> String {
    format!(
        "{}::{}",
        source_id,
        relative_path.to_string_lossy().replace('\\', "/")
    )
}

fn now_epoch_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::ZERO)
        .as_secs() as i64
}

pub(super) fn open_fast_source_db(source: &SampleSource) -> Result<SourceDatabase, String> {
    let database_root = source
        .database_root()
        .map_err(|err| format!("Resolve source metadata location failed: {err}"))?;
    SourceDatabase::open_for_background_job_with_database_root(&source.root, database_root)
        .map_err(|err| format!("Open source DB failed: {err}"))
}

pub(super) fn open_source_db(source: &SampleSource) -> Result<rusqlite::Connection, String> {
    let database_root = source
        .database_root()
        .map_err(|err| format!("Resolve source metadata location failed: {err}"))?;
    SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .map_err(|err| format!("Open source DB failed: {err}"))
}

#[cfg(test)]
mod tests;
