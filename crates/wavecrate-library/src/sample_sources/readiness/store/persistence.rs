use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension, Transaction, params};

use super::super::{
    model::{ReadinessArtifact, ReadinessScopeKind, ReadinessTarget, SourceAvailability},
    snapshot::{ArtifactPublishOutcome, ReadinessDeficit},
};
use super::error::ReadinessError;

/// Atomically replace the complete desired readiness set for one source generation.
pub fn replace_readiness_targets(
    connection: &mut Connection,
    source_id: &str,
    source_generation: i64,
    availability: SourceAvailability,
    targets: &[ReadinessTarget],
    updated_at: i64,
) -> Result<(), ReadinessError> {
    validate_targets(source_id, source_generation, targets)?;
    let tx = connection.transaction()?;
    let current_generation = tx
        .query_row(
            "SELECT source_generation
             FROM source_readiness_sources
             WHERE source_id = ?1",
            [source_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    if let Some(current) = current_generation
        && source_generation < current
    {
        return Err(ReadinessError::StaleSourceGeneration {
            source_id: source_id.to_string(),
            attempted: source_generation,
            current,
        });
    }
    tx.execute(
        "INSERT INTO source_readiness_sources (
            source_id, source_generation, availability, updated_at
         ) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(source_id) DO UPDATE SET
            source_generation = excluded.source_generation,
            availability = excluded.availability,
            updated_at = excluded.updated_at",
        params![
            source_id,
            source_generation,
            availability.as_str(),
            updated_at
        ],
    )?;
    tx.execute(
        "DELETE FROM source_readiness_targets WHERE source_id = ?1",
        [source_id],
    )?;
    for target in targets {
        insert_target(&tx, target, updated_at)?;
    }
    tx.commit()?;
    Ok(())
}

/// Publish a completion only when its version and generations still match the desired target.
pub fn publish_readiness_artifact(
    connection: &mut Connection,
    artifact: &ReadinessArtifact,
) -> Result<ArtifactPublishOutcome, ReadinessError> {
    let tx = connection.transaction()?;
    let is_current = tx.query_row(
        "SELECT EXISTS(
            SELECT 1
            FROM source_readiness_sources AS source
            JOIN source_readiness_targets AS target
              ON target.source_id = source.source_id
            WHERE target.source_id = ?1
              AND target.scope_kind = ?2
              AND target.scope_id = ?3
              AND target.stage = ?4
              AND target.required_version = ?5
              AND (?2 = 'file' OR target.source_generation = ?6)
              AND target.content_generation IS ?7
              AND target.eligibility = 'eligible'
              AND source.source_generation = target.source_generation
              AND source.availability = 'active'
        )",
        params![
            artifact.source_id,
            artifact.scope_kind.as_str(),
            artifact.scope_id,
            artifact.stage.as_str(),
            artifact.artifact_version,
            artifact.source_generation,
            artifact.content_generation,
        ],
        |row| row.get::<_, bool>(0),
    )?;
    if !is_current {
        tx.rollback()?;
        return Ok(ArtifactPublishOutcome::RejectedStale);
    }
    tx.execute(
        "INSERT INTO source_readiness_artifacts (
            source_id,
            scope_kind,
            scope_id,
            stage,
            artifact_version,
            source_generation,
            content_generation,
            completed_at
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

/// Persist actionable deficits into the existing source-local analysis job table.
///
/// These rows are readiness-managed so the legacy analysis claimant ignores them. OPT-1178's
/// supervisor can claim them through the unified readiness contract.
pub fn persist_readiness_deficits(
    connection: &mut Connection,
    deficits: &[ReadinessDeficit],
    created_at: i64,
) -> Result<usize, ReadinessError> {
    let mut unique = BTreeSet::new();
    let tx = connection.transaction()?;
    let mut changed = 0;
    for deficit in deficits {
        let target = &deficit.target;
        if !unique.insert((
            target.key(),
            target.required_version.clone(),
            target.source_generation,
            target.content_generation.clone(),
        )) {
            continue;
        }
        let sample_id = match target.scope_kind {
            ReadinessScopeKind::File => target.scope_id.clone(),
            ReadinessScopeKind::Source => format!("{}::__source__", target.source_id),
        };
        changed += persist_deficit(&tx, target, &sample_id, created_at)?;
    }
    tx.commit()?;
    Ok(changed)
}

fn persist_deficit(
    tx: &Transaction<'_>,
    target: &ReadinessTarget,
    sample_id: &str,
    created_at: i64,
) -> Result<usize, rusqlite::Error> {
    tx.execute(
        "INSERT INTO analysis_jobs (
            sample_id, source_id, relative_path, job_type, content_hash, status, attempts,
            created_at, running_at, last_error, readiness_managed, readiness_scope_kind,
            readiness_scope_id, readiness_stage, artifact_version, source_generation,
            content_generation, retry_at, failure_kind, lease_expires_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, 'pending', 0, ?6, NULL, NULL,
            1, ?7, ?8, ?9, ?10, ?11, ?12, NULL, NULL, NULL
         )
         ON CONFLICT(sample_id, job_type) DO UPDATE SET
            source_id = excluded.source_id,
            relative_path = excluded.relative_path,
            content_hash = excluded.content_hash,
            status = CASE
                WHEN analysis_jobs.artifact_version = excluded.artifact_version
                 AND (
                    excluded.readiness_scope_kind = 'file'
                    OR analysis_jobs.source_generation = excluded.source_generation
                 )
                 AND analysis_jobs.content_generation IS excluded.content_generation
                 AND (
                    analysis_jobs.status = 'pending'
                    OR (
                        analysis_jobs.status = 'running'
                        AND analysis_jobs.lease_expires_at > excluded.created_at
                    )
                 )
                THEN analysis_jobs.status ELSE 'pending' END,
            attempts = CASE
                WHEN analysis_jobs.artifact_version = excluded.artifact_version
                 AND (
                    excluded.readiness_scope_kind = 'file'
                    OR analysis_jobs.source_generation = excluded.source_generation
                 )
                 AND analysis_jobs.content_generation IS excluded.content_generation
                THEN analysis_jobs.attempts ELSE 0 END,
            created_at = CASE
                WHEN analysis_jobs.artifact_version = excluded.artifact_version
                 AND (
                    excluded.readiness_scope_kind = 'file'
                    OR analysis_jobs.source_generation = excluded.source_generation
                 )
                 AND analysis_jobs.content_generation IS excluded.content_generation
                THEN analysis_jobs.created_at ELSE excluded.created_at END,
            running_at = CASE
                WHEN analysis_jobs.artifact_version = excluded.artifact_version
                 AND (
                    excluded.readiness_scope_kind = 'file'
                    OR analysis_jobs.source_generation = excluded.source_generation
                 )
                 AND analysis_jobs.content_generation IS excluded.content_generation
                 AND analysis_jobs.status = 'running'
                 AND analysis_jobs.lease_expires_at > excluded.created_at
                THEN analysis_jobs.running_at ELSE NULL END,
            last_error = NULL,
            readiness_managed = 1,
            readiness_scope_kind = excluded.readiness_scope_kind,
            readiness_scope_id = excluded.readiness_scope_id,
            readiness_stage = excluded.readiness_stage,
            artifact_version = excluded.artifact_version,
            source_generation = excluded.source_generation,
            content_generation = excluded.content_generation,
            retry_at = NULL,
            failure_kind = NULL,
            lease_expires_at = CASE
                WHEN analysis_jobs.artifact_version = excluded.artifact_version
                 AND analysis_jobs.source_generation = excluded.source_generation
                 AND analysis_jobs.content_generation IS excluded.content_generation
                 AND analysis_jobs.status = 'running'
                 AND analysis_jobs.lease_expires_at > excluded.created_at
                THEN analysis_jobs.lease_expires_at ELSE NULL END",
        params![
            sample_id,
            target.source_id,
            target.relative_path.as_deref().unwrap_or(""),
            target.stage.job_type(),
            target.content_generation,
            created_at,
            target.scope_kind.as_str(),
            target.scope_id,
            target.stage.as_str(),
            target.required_version,
            target.source_generation,
            target.content_generation,
        ],
    )
}

fn validate_targets(
    source_id: &str,
    source_generation: i64,
    targets: &[ReadinessTarget],
) -> Result<(), ReadinessError> {
    let mut keys = BTreeSet::new();
    for target in targets {
        if target.source_id != source_id || target.source_generation != source_generation {
            return Err(ReadinessError::TargetGenerationMismatch {
                source_id: source_id.to_string(),
                generation: source_generation,
            });
        }
        if !keys.insert(target.key()) {
            return Err(ReadinessError::DuplicateTarget {
                source_id: target.source_id.clone(),
                scope_id: target.scope_id.clone(),
                stage: target.stage,
            });
        }
    }
    Ok(())
}

fn insert_target(
    tx: &Transaction<'_>,
    target: &ReadinessTarget,
    updated_at: i64,
) -> Result<(), rusqlite::Error> {
    tx.execute(
        "INSERT INTO source_readiness_targets (
            source_id, scope_kind, scope_id, relative_path, stage, required_version,
            source_generation, content_generation, eligibility, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            target.source_id,
            target.scope_kind.as_str(),
            target.scope_id,
            target.relative_path,
            target.stage.as_str(),
            target.required_version,
            target.source_generation,
            target.content_generation,
            target.eligibility.as_str(),
            updated_at,
        ],
    )?;
    Ok(())
}
