use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};

use super::super::model::{
    ClaimedReadinessWork, ReadinessArtifact, ReadinessEligibility, ReadinessFailureClassification,
    ReadinessFailureOutcome, ReadinessLeaseRenewalOutcome, ReadinessRetryPolicy, ReadinessTarget,
    ReadinessWorkMutationOutcome, ReadinessWorkStats,
};
use super::super::snapshot::ArtifactPublishOutcome;
use super::error::ReadinessError;

/// Claim one exact desired target with a durable lease.
///
/// File targets remain claimable across unrelated source-generation changes when their stable
/// identity, stage, artifact version, and content generation are unchanged. The returned target is
/// always refreshed from the current desired-state row.
pub fn claim_readiness_target(
    connection: &mut Connection,
    requested: &ReadinessTarget,
    now: i64,
    lease_duration_seconds: i64,
) -> Result<Option<ClaimedReadinessWork>, ReadinessError> {
    let lease_expires_at = lease_deadline(now, lease_duration_seconds)?;
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let candidate = tx
        .query_row(
            "SELECT job.id, job.readiness_claim_generation, job.attempts,
                    target.relative_path, target.source_generation, target.eligibility
             FROM source_readiness_targets AS target
             JOIN source_readiness_sources AS source
               ON source.source_id = target.source_id
              AND source.source_generation = target.source_generation
             JOIN analysis_jobs AS job
               ON job.source_id = target.source_id
              AND job.readiness_managed = 1
              AND job.readiness_scope_kind = target.scope_kind
              AND job.readiness_scope_id = target.scope_id
              AND job.readiness_stage = target.stage
              AND job.artifact_version = target.required_version
              AND job.content_generation = target.content_generation
              AND (target.scope_kind = 'file' OR job.source_generation = target.source_generation)
             WHERE target.source_id = ?1
               AND target.scope_kind = ?2
               AND target.scope_id = ?3
               AND target.stage = ?4
               AND target.required_version = ?5
               AND target.content_generation = ?6
               AND (?2 = 'file' OR target.source_generation = ?7)
               AND target.eligibility = 'eligible'
               AND source.availability = 'active'
               AND (
                    job.status = 'pending'
                    OR (
                        job.status = 'failed'
                        AND (job.failure_kind IS NULL OR job.failure_kind = 'retryable')
                        AND (job.retry_at IS NULL OR job.retry_at <= ?8)
                    )
                    OR (
                        job.status = 'running'
                        AND (job.lease_expires_at IS NULL OR job.lease_expires_at <= ?8)
                    )
               )
               AND NOT EXISTS (
                    SELECT 1
                    FROM source_readiness_artifacts AS artifact
                    WHERE artifact.source_id = target.source_id
                      AND artifact.scope_kind = target.scope_kind
                      AND artifact.scope_id = target.scope_id
                      AND artifact.stage = target.stage
                      AND artifact.artifact_version = target.required_version
                      AND artifact.content_generation = target.content_generation
                      AND (
                          target.scope_kind = 'file'
                          OR artifact.source_generation = target.source_generation
                      )
               )",
            params![
                requested.source_id,
                requested.scope_kind.as_str(),
                requested.scope_id,
                requested.stage.as_str(),
                requested.required_version,
                requested.content_generation,
                requested.source_generation,
                now,
            ],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()?;
    let Some((
        job_id,
        stored_claim_generation,
        stored_failure_attempts,
        relative_path,
        source_generation,
        eligibility,
    )) = candidate
    else {
        tx.commit()?;
        return Ok(None);
    };
    let previous_claim_generation =
        decode_counter("readiness_claim_generation", stored_claim_generation)?;
    let claim_generation = previous_claim_generation
        .checked_add(1)
        .ok_or(ReadinessError::TimestampOverflow)?;
    let failure_attempts = decode_counter("attempts", stored_failure_attempts)?;
    let changed = tx.execute(
        "UPDATE analysis_jobs
         SET status = 'running',
             readiness_claim_generation = ?1,
             running_at = ?2,
             retry_at = NULL,
             failure_kind = NULL,
             last_error = NULL,
             lease_expires_at = ?3,
             relative_path = ?4,
             source_generation = ?5
         WHERE id = ?6
           AND readiness_claim_generation = ?7",
        params![
            claim_generation,
            now,
            lease_expires_at,
            relative_path.as_deref().unwrap_or(""),
            source_generation,
            job_id,
            previous_claim_generation,
        ],
    )?;
    if changed != 1 {
        tx.rollback()?;
        return Ok(None);
    }
    let target = ReadinessTarget {
        source_id: requested.source_id.clone(),
        scope_kind: requested.scope_kind,
        scope_id: requested.scope_id.clone(),
        relative_path,
        stage: requested.stage,
        required_version: requested.required_version.clone(),
        source_generation,
        content_generation: requested.content_generation.clone(),
        eligibility: decode_eligibility(eligibility)?,
    };
    tx.commit()?;
    Ok(Some(ClaimedReadinessWork {
        target,
        claim_generation,
        failure_attempts,
        lease_expires_at,
    }))
}

/// Renew an unexpired claim without changing its claim generation.
pub fn renew_readiness_lease(
    connection: &mut Connection,
    claim: &ClaimedReadinessWork,
    now: i64,
    lease_duration_seconds: i64,
) -> Result<ReadinessLeaseRenewalOutcome, ReadinessError> {
    let requested_deadline = lease_deadline(now, lease_duration_seconds)?;
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let changed = tx.execute(
        "UPDATE analysis_jobs
         SET lease_expires_at = MAX(lease_expires_at, ?1)
         WHERE status = 'running'
           AND lease_expires_at > ?2
           AND readiness_claim_generation = ?3
           AND source_id = ?4
           AND readiness_managed = 1
           AND readiness_scope_kind = ?5
           AND readiness_scope_id = ?6
           AND readiness_stage = ?7
           AND artifact_version = ?8
           AND content_generation = ?9
           AND (?5 = 'file' OR source_generation = ?10)
           AND EXISTS (
               SELECT 1
               FROM source_readiness_targets AS target
               JOIN source_readiness_sources AS source
                 ON source.source_id = target.source_id
                AND source.source_generation = target.source_generation
               WHERE target.source_id = analysis_jobs.source_id
                 AND target.scope_kind = analysis_jobs.readiness_scope_kind
                 AND target.scope_id = analysis_jobs.readiness_scope_id
                 AND target.stage = analysis_jobs.readiness_stage
                 AND target.required_version = analysis_jobs.artifact_version
                 AND target.content_generation = analysis_jobs.content_generation
                 AND (?5 = 'file' OR target.source_generation = ?10)
                 AND target.eligibility = 'eligible'
                 AND source.availability = 'active'
           )",
        params![
            requested_deadline,
            now,
            claim.claim_generation,
            claim.target.source_id,
            claim.target.scope_kind.as_str(),
            claim.target.scope_id,
            claim.target.stage.as_str(),
            claim.target.required_version,
            claim.target.content_generation,
            claim.target.source_generation,
        ],
    )?;
    if changed != 1 {
        tx.rollback()?;
        return Ok(ReadinessLeaseRenewalOutcome::RejectedStale);
    }
    let lease_expires_at = tx.query_row(
        "SELECT lease_expires_at
         FROM analysis_jobs
         WHERE source_id = ?1
           AND readiness_managed = 1
           AND readiness_scope_kind = ?2
           AND readiness_scope_id = ?3
           AND readiness_stage = ?4
           AND status = 'running'
           AND readiness_claim_generation = ?5
           AND artifact_version = ?6
           AND content_generation = ?7
           AND (?2 = 'file' OR source_generation = ?8)",
        params![
            claim.target.source_id,
            claim.target.scope_kind.as_str(),
            claim.target.scope_id,
            claim.target.stage.as_str(),
            claim.claim_generation,
            claim.target.required_version,
            claim.target.content_generation,
            claim.target.source_generation,
        ],
        |row| row.get(0),
    )?;
    tx.commit()?;
    Ok(ReadinessLeaseRenewalOutcome::Renewed { lease_expires_at })
}

/// Atomically publish the exact claimed artifact and mark its work row complete.
pub fn complete_readiness_work(
    connection: &mut Connection,
    claim: &ClaimedReadinessWork,
    completed_at: i64,
) -> Result<ArtifactPublishOutcome, ReadinessError> {
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let changed = finish_claim_update(
        &tx,
        claim,
        completed_at,
        "status = 'done', running_at = NULL, retry_at = NULL, failure_kind = NULL, \
         last_error = NULL, lease_expires_at = NULL",
        None,
        None,
    )?;
    if changed != 1 {
        tx.rollback()?;
        return Ok(ArtifactPublishOutcome::RejectedStale);
    }
    let artifact = ReadinessArtifact::for_target(&claim.target, completed_at);
    tx.execute(
        "INSERT INTO source_readiness_artifacts (
            source_id, scope_kind, scope_id, stage, artifact_version,
            source_generation, content_generation, completed_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(source_id, scope_kind, scope_id, stage) DO UPDATE SET
            artifact_version = excluded.artifact_version,
            source_generation = excluded.source_generation,
            content_generation = excluded.content_generation,
            completed_at = excluded.completed_at",
        params![
            artifact.source_id,
            artifact.scope_kind.as_str(),
            artifact.scope_id,
            artifact.stage.as_str(),
            artifact.artifact_version,
            artifact.source_generation,
            artifact.content_generation,
            artifact.completed_at,
        ],
    )?;
    tx.commit()?;
    Ok(ArtifactPublishOutcome::Recorded)
}

/// Record a classified failure, applying bounded backoff to retryable failures.
pub fn fail_readiness_work(
    connection: &mut Connection,
    claim: &ClaimedReadinessWork,
    classification: ReadinessFailureClassification,
    reason: &str,
    failed_at: i64,
    retry_policy: ReadinessRetryPolicy,
) -> Result<ReadinessFailureOutcome, ReadinessError> {
    let failure_attempt = match classification {
        ReadinessFailureClassification::Retryable => Some(
            claim
                .failure_attempts
                .checked_add(1)
                .ok_or(ReadinessError::TimestampOverflow)?,
        ),
        ReadinessFailureClassification::Permanent | ReadinessFailureClassification::Unsupported => {
            None
        }
    };
    let (stored_classification, retry_at, outcome) = match classification {
        ReadinessFailureClassification::Retryable
            if failure_attempt.expect("retryable failure attempt")
                >= retry_policy.max_attempts() =>
        {
            (
                ReadinessFailureClassification::Permanent,
                None,
                ReadinessFailureOutcome::AttemptsExhausted,
            )
        }
        ReadinessFailureClassification::Retryable => {
            let failure_attempt = failure_attempt.expect("retryable failure attempt");
            let retry_at = failed_at
                .checked_add(retry_policy.delay_for_attempt(failure_attempt))
                .ok_or(ReadinessError::TimestampOverflow)?;
            (
                classification,
                Some(retry_at),
                ReadinessFailureOutcome::RetryScheduled { retry_at },
            )
        }
        ReadinessFailureClassification::Permanent => {
            (classification, None, ReadinessFailureOutcome::Permanent)
        }
        ReadinessFailureClassification::Unsupported => {
            (classification, None, ReadinessFailureOutcome::Unsupported)
        }
    };
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let changed = finish_claim_update(
        &tx,
        claim,
        failed_at,
        "status = 'failed', running_at = NULL, failure_kind = ?10, retry_at = ?11, \
         last_error = ?12, lease_expires_at = NULL",
        Some((
            stored_classification.as_str(),
            retry_at,
            normalized_reason(reason),
        )),
        failure_attempt,
    )?;
    if changed != 1 {
        tx.rollback()?;
        return Ok(ReadinessFailureOutcome::RejectedStale);
    }
    tx.commit()?;
    Ok(outcome)
}

/// Voluntarily release a current claim back to pending without deleting durable work.
pub fn release_readiness_work(
    connection: &mut Connection,
    claim: &ClaimedReadinessWork,
    released_at: i64,
) -> Result<ReadinessWorkMutationOutcome, ReadinessError> {
    return_claim_to_pending(connection, claim, released_at, None)
}

/// Cancel in-flight execution back to pending while retaining a cancellation diagnostic.
pub fn cancel_readiness_work(
    connection: &mut Connection,
    claim: &ClaimedReadinessWork,
    reason: &str,
    cancelled_at: i64,
) -> Result<ReadinessWorkMutationOutcome, ReadinessError> {
    return_claim_to_pending(
        connection,
        claim,
        cancelled_at,
        Some(normalized_reason(reason)),
    )
}

/// Read queue, retry, lease-recovery, cancellation, and terminal counts for telemetry.
pub fn readiness_work_stats(
    connection: &Connection,
    now: i64,
) -> Result<ReadinessWorkStats, ReadinessError> {
    let values = connection.query_row(
        "SELECT
            COUNT(*),
            SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END),
            SUM(CASE WHEN status = 'running' AND lease_expires_at > ?1 THEN 1 ELSE 0 END),
            SUM(CASE WHEN status = 'running' AND (
                lease_expires_at IS NULL OR lease_expires_at <= ?1
            ) THEN 1 ELSE 0 END),
            SUM(CASE WHEN status = 'failed' AND failure_kind = 'retryable'
                AND (retry_at IS NULL OR retry_at <= ?1) THEN 1 ELSE 0 END),
            SUM(CASE WHEN status = 'failed' AND failure_kind = 'retryable'
                AND retry_at > ?1 THEN 1 ELSE 0 END),
            MIN(CASE WHEN status = 'failed' AND failure_kind = 'retryable'
                AND retry_at > ?1 THEN retry_at ELSE NULL END),
            SUM(CASE WHEN status = 'failed' AND failure_kind = 'permanent'
                THEN 1 ELSE 0 END),
            SUM(CASE WHEN status = 'failed' AND failure_kind = 'unsupported'
                THEN 1 ELSE 0 END),
            SUM(CASE WHEN status = 'pending' AND failure_kind = 'cancelled'
                THEN 1 ELSE 0 END),
            SUM(CASE WHEN status = 'done' THEN 1 ELSE 0 END)
         FROM analysis_jobs AS job
         JOIN source_readiness_targets AS target
           ON target.source_id = job.source_id
          AND target.scope_kind = job.readiness_scope_kind
          AND target.scope_id = job.readiness_scope_id
          AND target.stage = job.readiness_stage
          AND target.required_version = job.artifact_version
          AND target.content_generation = job.content_generation
         JOIN source_readiness_sources AS source
           ON source.source_id = target.source_id
          AND source.source_generation = target.source_generation
         WHERE job.readiness_managed = 1
           AND target.eligibility = 'eligible'
           AND source.availability = 'active'
           AND (
               target.scope_kind = 'file'
               OR job.source_generation = target.source_generation
           )",
        [now],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<i64>>(1)?.unwrap_or(0),
                row.get::<_, Option<i64>>(2)?.unwrap_or(0),
                row.get::<_, Option<i64>>(3)?.unwrap_or(0),
                row.get::<_, Option<i64>>(4)?.unwrap_or(0),
                row.get::<_, Option<i64>>(5)?.unwrap_or(0),
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, Option<i64>>(7)?.unwrap_or(0),
                row.get::<_, Option<i64>>(8)?.unwrap_or(0),
                row.get::<_, Option<i64>>(9)?.unwrap_or(0),
                row.get::<_, Option<i64>>(10)?.unwrap_or(0),
            ))
        },
    )?;
    Ok(ReadinessWorkStats {
        total: count(values.0),
        pending: count(values.1),
        running: count(values.2),
        expired_leases: count(values.3),
        retries_due: count(values.4),
        retries_waiting: count(values.5),
        earliest_retry_at: values.6,
        permanent_failures: count(values.7),
        unsupported: count(values.8),
        cancelled: count(values.9),
        completed: count(values.10),
    })
}

fn return_claim_to_pending(
    connection: &mut Connection,
    claim: &ClaimedReadinessWork,
    now: i64,
    cancellation_reason: Option<&str>,
) -> Result<ReadinessWorkMutationOutcome, ReadinessError> {
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let (set_clause, extras) = if let Some(reason) = cancellation_reason {
        (
            "status = 'pending', running_at = NULL, retry_at = NULL, \
             failure_kind = ?10, last_error = ?12, lease_expires_at = NULL",
            Some(("cancelled", None, reason)),
        )
    } else {
        (
            "status = 'pending', running_at = NULL, retry_at = NULL, \
             failure_kind = NULL, last_error = NULL, lease_expires_at = NULL",
            None,
        )
    };
    let changed = finish_claim_update(&tx, claim, now, set_clause, extras, None)?;
    if changed != 1 {
        tx.rollback()?;
        return Ok(ReadinessWorkMutationOutcome::RejectedStale);
    }
    tx.commit()?;
    Ok(ReadinessWorkMutationOutcome::Recorded)
}

fn finish_claim_update(
    tx: &Transaction<'_>,
    claim: &ClaimedReadinessWork,
    now: i64,
    set_clause: &str,
    extras: Option<(&str, Option<i64>, &str)>,
    failure_attempt: Option<u32>,
) -> Result<usize, rusqlite::Error> {
    let sql = format!(
        "UPDATE analysis_jobs
         SET {set_clause},
             attempts = COALESCE(?13, attempts)
         WHERE status = 'running'
           AND lease_expires_at > ?1
           AND readiness_claim_generation = ?2
           AND source_id = ?3
           AND readiness_managed = 1
           AND readiness_scope_kind = ?4
           AND readiness_scope_id = ?5
           AND readiness_stage = ?6
           AND artifact_version = ?7
           AND content_generation = ?8
           AND (?4 = 'file' OR source_generation = ?9)
           AND EXISTS (
               SELECT 1
               FROM source_readiness_targets AS target
               JOIN source_readiness_sources AS source
                 ON source.source_id = target.source_id
                AND source.source_generation = target.source_generation
               WHERE target.source_id = analysis_jobs.source_id
                 AND target.scope_kind = analysis_jobs.readiness_scope_kind
                 AND target.scope_id = analysis_jobs.readiness_scope_id
                 AND target.stage = analysis_jobs.readiness_stage
                 AND target.required_version = analysis_jobs.artifact_version
                 AND target.content_generation = analysis_jobs.content_generation
                 AND (?4 = 'file' OR target.source_generation = ?9)
                 AND target.eligibility = 'eligible'
                 AND source.availability = 'active'
           )"
    );
    let (failure_kind, retry_at, reason) = extras
        .map(|(kind, retry_at, reason)| (Some(kind), retry_at, Some(reason)))
        .unwrap_or((None, None, None));
    tx.execute(
        &sql,
        params![
            now,
            claim.claim_generation,
            claim.target.source_id,
            claim.target.scope_kind.as_str(),
            claim.target.scope_id,
            claim.target.stage.as_str(),
            claim.target.required_version,
            claim.target.content_generation,
            claim.target.source_generation,
            failure_kind,
            retry_at,
            reason,
            failure_attempt,
        ],
    )
}

fn lease_deadline(now: i64, duration_seconds: i64) -> Result<i64, ReadinessError> {
    if duration_seconds <= 0 {
        return Err(ReadinessError::InvalidLeaseDuration(duration_seconds));
    }
    now.checked_add(duration_seconds)
        .ok_or(ReadinessError::TimestampOverflow)
}

fn decode_counter(field: &'static str, value: i64) -> Result<u32, ReadinessError> {
    u32::try_from(value).map_err(|_| ReadinessError::UnknownStoredValue {
        field,
        value: value.to_string(),
    })
}

fn decode_eligibility(value: String) -> Result<ReadinessEligibility, ReadinessError> {
    ReadinessEligibility::from_stored(&value).ok_or(ReadinessError::UnknownStoredValue {
        field: "eligibility",
        value,
    })
}

fn normalized_reason(reason: &str) -> &str {
    if reason.trim().is_empty() {
        "readiness_work_failed"
    } else {
        reason
    }
}

fn count(value: i64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}
