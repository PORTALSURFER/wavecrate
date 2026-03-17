use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
use tracing::warn;

use crate::app::controller::library::analysis_jobs::pool::job_claim::lease;

type AllowedSourceIds = std::sync::Arc<
    std::sync::RwLock<Option<std::collections::HashSet<crate::sample_sources::SourceId>>>,
>;

type ImmediateJob = (analysis_db::ClaimedJob, Result<(), String>);

pub(super) struct BatchSettings {
    pub(super) use_cache: bool,
    pub(super) max_analysis_duration_seconds: f32,
    pub(super) analysis_sample_rate: u32,
    pub(super) analysis_version: String,
}

pub(super) type DecodedBatchMap = HashMap<
    std::path::PathBuf,
    Vec<(
        analysis_db::ClaimedJob,
        crate::analysis::audio::AnalysisAudio,
    )>,
>;

struct BatchProcessContext<'a> {
    connections: &'a mut HashMap<std::path::PathBuf, Connection>,
    allowed_source_ids: &'a AllowedSourceIds,
    log_jobs: bool,
    settings: &'a BatchSettings,
    decode_queue: &'a super::super::DecodedQueue,
    decoded_batches: &'a mut DecodedBatchMap,
    immediate_jobs: &'a mut Vec<ImmediateJob>,
}

pub(super) fn current_batch_settings(
    use_cache: &std::sync::atomic::AtomicBool,
    max_duration_bits: &std::sync::atomic::AtomicU32,
    analysis_sample_rate: &std::sync::atomic::AtomicU32,
    analysis_version_override: &std::sync::Arc<std::sync::RwLock<Option<String>>>,
) -> BatchSettings {
    BatchSettings {
        use_cache: use_cache.load(std::sync::atomic::Ordering::Relaxed),
        max_analysis_duration_seconds: f32::from_bits(
            max_duration_bits.load(std::sync::atomic::Ordering::Relaxed),
        ),
        analysis_sample_rate: analysis_sample_rate
            .load(std::sync::atomic::Ordering::Relaxed)
            .max(1),
        analysis_version: analysis_version_override
            .read()
            .ok()
            .and_then(|guard| guard.clone())
            .unwrap_or_else(|| crate::analysis::version::analysis_version().to_string()),
    }
}

pub(super) fn process_batch(
    batch: Vec<DecodedWork>,
    connections: &mut HashMap<std::path::PathBuf, Connection>,
    allowed_source_ids: &AllowedSourceIds,
    log_jobs: bool,
    settings: &BatchSettings,
    decode_queue: &super::super::DecodedQueue,
) -> (DecodedBatchMap, Vec<ImmediateJob>) {
    let mut decoded_batches: DecodedBatchMap = HashMap::new();
    let mut immediate_jobs = Vec::new();
    let mut context = BatchProcessContext {
        connections,
        allowed_source_ids,
        log_jobs,
        settings,
        decode_queue,
        decoded_batches: &mut decoded_batches,
        immediate_jobs: &mut immediate_jobs,
    };
    for work in batch {
        context.process_work(work);
    }
    (decoded_batches, immediate_jobs)
}

impl BatchProcessContext<'_> {
    fn process_work(&mut self, work: DecodedWork) {
        let allowed = self
            .allowed_source_ids
            .read()
            .ok()
            .and_then(|guard| guard.clone());
        if !lease::job_allowed(&work.job, allowed.as_ref()) {
            super::finalization::release_disallowed_work(
                self.connections,
                &work,
                self.log_jobs,
                self.decode_queue,
            );
            return;
        }
        super::finalization::log_run_start(self.log_jobs, &work.job);
        let mut batch_job = None;
        let mut immediate_job = None;
        let job_fallback = work.job.clone();
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            run_work_item(
                work,
                self.connections,
                self.settings,
                &mut batch_job,
                &mut immediate_job,
            )
        }))
        .unwrap_or_else(|payload| Err(logging::panic_to_string(payload)));

        if let Err(err) = outcome {
            immediate_job = Some((job_fallback, Err(err)));
        }
        if let Some((job, decoded)) = batch_job {
            self.decoded_batches
                .entry(job.source_root.clone())
                .or_default()
                .push((job, decoded));
        }
        if let Some(entry) = immediate_job {
            self.immediate_jobs.push(entry);
        }
    }
}

fn run_work_item(
    work: DecodedWork,
    connections: &mut HashMap<std::path::PathBuf, Connection>,
    settings: &BatchSettings,
    batch_job: &mut Option<(
        analysis_db::ClaimedJob,
        crate::analysis::audio::AnalysisAudio,
    )>,
    immediate_job: &mut Option<(analysis_db::ClaimedJob, Result<(), String>)>,
) -> Result<(), String> {
    let conn = match db::open_connection_with_retry(connections, &work.job.source_root) {
        Ok(conn) => conn,
        Err(err) => {
            *immediate_job = Some((work.job, Err(err)));
            return Ok(());
        }
    };
    match work.job.job_type.as_str() {
        analysis_db::ANALYZE_SAMPLE_JOB_TYPE => handle_analysis_work(
            work,
            conn,
            &settings.analysis_version,
            batch_job,
            immediate_job,
        ),
        _ => {
            let result = super::super::super::job_execution::run_job(
                conn,
                &work.job,
                settings.use_cache,
                settings.max_analysis_duration_seconds,
                settings.analysis_sample_rate,
                &settings.analysis_version,
            );
            *immediate_job = Some((work.job, result));
            Ok(())
        }
    }
}

fn handle_analysis_work(
    work: DecodedWork,
    conn: &Connection,
    analysis_version: &str,
    batch_job: &mut Option<(
        analysis_db::ClaimedJob,
        crate::analysis::audio::AnalysisAudio,
    )>,
    immediate_job: &mut Option<(analysis_db::ClaimedJob, Result<(), String>)>,
) -> Result<(), String> {
    match work.outcome {
        DecodeOutcome::Decoded(decoded) => {
            *batch_job = Some((work.job, decoded));
            Ok(())
        }
        DecodeOutcome::Skipped {
            duration_seconds,
            sample_rate,
        } => {
            let result = analysis_db::update_analysis_metadata(
                conn,
                analysis_db::AnalysisMetadataUpdate {
                    sample_id: &work.job.sample_id,
                    content_hash: work.job.content_hash.as_deref(),
                    duration_seconds,
                    sr_used: sample_rate,
                    analysis_version,
                },
            );
            *immediate_job = Some((work.job, result));
            Ok(())
        }
        DecodeOutcome::Failed(err) => {
            *immediate_job = Some((work.job, Err(err)));
            Ok(())
        }
        DecodeOutcome::NotNeeded => {
            *immediate_job = Some((work.job, Err("Decode missing for analysis job".to_string())));
            Ok(())
        }
    }
}

pub(super) fn immediate_jobs_with_decoded_batches(
    decoded_batches: DecodedBatchMap,
    connections: &mut HashMap<std::path::PathBuf, Connection>,
    settings: &BatchSettings,
) -> Vec<ImmediateJob> {
    let mut immediate_jobs = Vec::new();
    for (source_root, jobs) in decoded_batches {
        run_decoded_batch(
            source_root,
            jobs,
            connections,
            settings,
            &mut immediate_jobs,
        );
    }
    immediate_jobs
}

fn run_decoded_batch(
    source_root: std::path::PathBuf,
    jobs: Vec<(
        analysis_db::ClaimedJob,
        crate::analysis::audio::AnalysisAudio,
    )>,
    connections: &mut HashMap<std::path::PathBuf, Connection>,
    settings: &BatchSettings,
    immediate_jobs: &mut Vec<ImmediateJob>,
) {
    let conn = match db::open_connection_with_retry(connections, &source_root) {
        Ok(conn) => conn,
        Err(err) => {
            for (job, _) in jobs {
                immediate_jobs.push((job, Err(err.clone())));
            }
            return;
        }
    };
    let jobs_for_failure: Vec<analysis_db::ClaimedJob> =
        jobs.iter().map(|(job, _)| job.clone()).collect();
    let analysis_context = AnalysisContext {
        use_cache: settings.use_cache,
        max_analysis_duration_seconds: settings.max_analysis_duration_seconds,
        analysis_sample_rate: settings.analysis_sample_rate,
        analysis_version: settings.analysis_version.as_str(),
    };
    let batch_outcomes = catch_unwind(AssertUnwindSafe(|| {
        super::super::super::job_execution::run_analysis_jobs_with_decoded_batch(
            conn,
            jobs,
            &analysis_context,
        )
    }))
    .unwrap_or_else(|payload| {
        let err = logging::panic_to_string(payload);
        warn!(error = %err, "Analysis batch panicked");
        jobs_for_failure
            .into_iter()
            .map(|job| (job, Err(err.clone())))
            .collect()
    });
    immediate_jobs.extend(batch_outcomes);
}
