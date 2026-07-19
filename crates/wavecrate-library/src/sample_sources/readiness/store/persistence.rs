use std::{
    collections::{BTreeMap, BTreeSet},
    sync::atomic::{AtomicBool, Ordering},
};

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};

use super::super::{
    model::{
        ReadinessArtifact, ReadinessEligibility, ReadinessMembership, ReadinessScopeKind,
        ReadinessStage, ReadinessTarget, SourceAvailability,
    },
    snapshot::{ArtifactPublishOutcome, ReadinessDeficit},
};
use super::{ReadinessDeltaPublicationOutcome, error::ReadinessError};

#[cfg(test)]
const TEST_READINESS_CONTRACT_VERSION: &str = "readiness-test-contract-v1";

/// Atomically replace the complete desired readiness set for one source generation.
#[cfg(test)]
pub fn replace_readiness_targets(
    connection: &mut Connection,
    source_id: &str,
    source_generation: i64,
    readiness_revision: i64,
    availability: SourceAvailability,
    targets: &[ReadinessTarget],
    updated_at: i64,
) -> Result<(), ReadinessError> {
    replace_readiness_targets_inner(
        connection,
        source_id,
        source_generation,
        readiness_revision,
        availability,
        TEST_READINESS_CONTRACT_VERSION,
        targets,
        updated_at,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn replace_readiness_targets_inner(
    connection: &mut Connection,
    source_id: &str,
    source_generation: i64,
    readiness_revision: i64,
    availability: SourceAvailability,
    contract_version: &str,
    targets: &[ReadinessTarget],
    updated_at: i64,
    cancel: Option<&AtomicBool>,
) -> Result<(), ReadinessError> {
    validate_contract_version(contract_version)?;
    validate_targets(source_id, source_generation, targets, cancel)?;
    let membership = membership_from_targets(targets);
    cancellation_checkpoint(cancel)?;
    let tx = connection.transaction()?;
    let current_state = tx
        .query_row(
            "SELECT source_generation, readiness_revision
             FROM source_readiness_sources
             WHERE source_id = ?1",
            [source_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()?;
    if let Some((current_generation, current_revision)) = current_state {
        if source_generation < current_generation {
            return Err(ReadinessError::StaleSourceGeneration {
                source_id: source_id.to_string(),
                attempted: source_generation,
                current: current_generation,
            });
        }
        if readiness_revision <= current_revision {
            return Err(ReadinessError::StaleReadinessRevision {
                source_id: source_id.to_string(),
                attempted: readiness_revision,
                current: current_revision,
            });
        }
    }
    validate_manifest_membership(&tx, targets, cancel)?;
    cancellation_checkpoint(cancel)?;
    tx.execute(
        "INSERT INTO source_readiness_sources (
            source_id, source_generation, readiness_revision, availability, contract_version,
            membership_digest, membership_count, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(source_id) DO UPDATE SET
            source_generation = excluded.source_generation,
            readiness_revision = excluded.readiness_revision,
            availability = excluded.availability,
            contract_version = excluded.contract_version,
            membership_digest = excluded.membership_digest,
            membership_count = excluded.membership_count,
            updated_at = excluded.updated_at",
        params![
            source_id,
            source_generation,
            readiness_revision,
            availability.as_str(),
            contract_version,
            membership.digest.as_slice(),
            i64::try_from(membership.count).unwrap_or(i64::MAX),
            updated_at
        ],
    )?;
    cancellation_checkpoint(cancel)?;
    tx.execute(
        "DELETE FROM source_readiness_targets WHERE source_id = ?1",
        [source_id],
    )?;
    for target in targets {
        cancellation_checkpoint(cancel)?;
        insert_target(&tx, target, updated_at)?;
        if target.scope_kind == ReadinessScopeKind::File {
            refresh_work_metadata(&tx, target)?;
        }
    }
    cancellation_checkpoint(cancel)?;
    tx.execute(
        "DELETE FROM analysis_jobs
         WHERE source_id = ?1
           AND readiness_managed = 1
           AND NOT EXISTS (
               SELECT 1
               FROM source_readiness_targets AS target
               WHERE target.source_id = analysis_jobs.source_id
                 AND target.scope_kind = analysis_jobs.readiness_scope_kind
                 AND target.scope_id = analysis_jobs.readiness_scope_id
                 AND target.stage = analysis_jobs.readiness_stage
                 AND target.required_version = analysis_jobs.artifact_version
                 AND target.content_generation = analysis_jobs.content_generation
                 AND (
                     target.scope_kind = 'file'
                     OR target.source_generation = analysis_jobs.source_generation
                 )
           )",
        [source_id],
    )?;
    tx.commit()?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_readiness_target_delta_inner(
    connection: &mut Connection,
    source_id: &str,
    source_generation: i64,
    readiness_revision: i64,
    availability: SourceAvailability,
    contract_version: &str,
    upsert_targets: &[ReadinessTarget],
    deleted_file_scope_ids: &[String],
    similarity_artifact_version: &str,
    updated_at: i64,
    cancel: Option<&AtomicBool>,
) -> Result<ReadinessDeltaPublicationOutcome, ReadinessError> {
    validate_contract_version(contract_version)?;
    if similarity_artifact_version.trim().is_empty() {
        return Err(ReadinessError::InvalidArtifactVersion {
            source_id: source_id.to_string(),
            scope_id: source_id.to_string(),
            stage: ReadinessStage::SimilarityLayout,
        });
    }
    validate_delta_targets(source_id, source_generation, upsert_targets, cancel)?;
    cancellation_checkpoint(cancel)?;
    let tx = connection.transaction()?;
    let Some((
        current_generation,
        current_revision,
        current_contract,
        raw_membership_digest,
        raw_membership_count,
    )) = tx
        .query_row(
            "SELECT source_generation, readiness_revision, contract_version,
                    membership_digest, membership_count
             FROM source_readiness_sources
             WHERE source_id = ?1",
            [source_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )
        .optional()?
    else {
        tx.rollback()?;
        return Ok(ReadinessDeltaPublicationOutcome::RequiresFullPublication);
    };
    if source_generation < current_generation {
        return Err(ReadinessError::StaleSourceGeneration {
            source_id: source_id.to_string(),
            attempted: source_generation,
            current: current_generation,
        });
    }
    if readiness_revision <= current_revision {
        return Err(ReadinessError::StaleReadinessRevision {
            source_id: source_id.to_string(),
            attempted: readiness_revision,
            current: current_revision,
        });
    }
    if source_generation == current_generation {
        tx.rollback()?;
        let digest = decode_membership_digest(&raw_membership_digest)?;
        return Ok(ReadinessDeltaPublicationOutcome::Applied {
            membership_generation: ReadinessMembership::from_parts(
                digest,
                raw_membership_count.max(0) as u64,
            )
            .generation(),
            changed: 0,
        });
    }
    if source_generation != current_generation.saturating_add(1)
        || current_contract != contract_version
        || raw_membership_digest.len() != 32
        || raw_membership_count < 0
    {
        tx.rollback()?;
        return Ok(ReadinessDeltaPublicationOutcome::RequiresFullPublication);
    }
    validate_delta_manifest_membership(&tx, upsert_targets, deleted_file_scope_ids, cancel)?;

    let mut membership = ReadinessMembership::from_parts(
        decode_membership_digest(&raw_membership_digest)?,
        raw_membership_count.max(0) as u64,
    );
    let upsert_scope_ids = upsert_targets
        .iter()
        .map(|target| target.scope_id.clone())
        .collect::<BTreeSet<_>>();
    let affected_scope_ids = upsert_scope_ids
        .iter()
        .cloned()
        .chain(deleted_file_scope_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    for scope_id in &affected_scope_ids {
        cancellation_checkpoint(cancel)?;
        let old_member = tx
            .query_row(
                "SELECT content_generation
                 FROM source_readiness_targets
                 WHERE source_id = ?1
                   AND scope_kind = 'file'
                   AND scope_id = ?2
                   AND stage = 'embedding_aspects'
                   AND eligibility = 'eligible'",
                params![source_id, scope_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(content_generation) = old_member {
            if membership.count == 0 {
                tx.rollback()?;
                return Ok(ReadinessDeltaPublicationOutcome::RequiresFullPublication);
            }
            membership.remove(scope_id, &content_generation);
        }
    }
    for target in upsert_targets.iter().filter(|target| {
        target.stage == ReadinessStage::EmbeddingAspects
            && target.eligibility == ReadinessEligibility::Eligible
    }) {
        membership.add(&target.scope_id, &target.content_generation);
    }
    let membership_generation = membership.generation();
    let similarity_target = ReadinessTarget::source(
        source_id,
        ReadinessStage::SimilarityLayout,
        similarity_artifact_version,
        source_generation,
        membership_generation.as_str(),
    );

    let mut changed = 0_usize;
    for scope_id in deleted_file_scope_ids {
        cancellation_checkpoint(cancel)?;
        if upsert_scope_ids.contains(scope_id) {
            continue;
        }
        changed = changed.saturating_add(tx.execute(
            "DELETE FROM source_readiness_targets
             WHERE source_id = ?1 AND scope_kind = 'file' AND scope_id = ?2",
            params![source_id, scope_id],
        )?);
        changed = changed.saturating_add(tx.execute(
            "DELETE FROM analysis_jobs
             WHERE source_id = ?1
               AND readiness_managed = 1
               AND readiness_scope_kind = 'file'
               AND readiness_scope_id = ?2",
            params![source_id, scope_id],
        )?);
    }
    for target in upsert_targets {
        cancellation_checkpoint(cancel)?;
        changed = changed.saturating_add(upsert_target(&tx, target, updated_at)?);
    }
    changed = changed.saturating_add(upsert_target(&tx, &similarity_target, updated_at)?);
    for scope_id in &upsert_scope_ids {
        cancellation_checkpoint(cancel)?;
        changed = changed.saturating_add(tx.execute(
            "DELETE FROM analysis_jobs
             WHERE source_id = ?1
               AND readiness_managed = 1
               AND readiness_scope_kind = 'file'
               AND readiness_scope_id = ?2
               AND NOT EXISTS (
                   SELECT 1
                   FROM source_readiness_targets AS target
                   WHERE target.source_id = analysis_jobs.source_id
                     AND target.scope_kind = analysis_jobs.readiness_scope_kind
                     AND target.scope_id = analysis_jobs.readiness_scope_id
                     AND target.stage = analysis_jobs.readiness_stage
                     AND target.required_version = analysis_jobs.artifact_version
                     AND target.content_generation = analysis_jobs.content_generation
               )",
            params![source_id, scope_id],
        )?);
    }
    changed = changed.saturating_add(tx.execute(
        "DELETE FROM analysis_jobs
         WHERE source_id = ?1
           AND readiness_managed = 1
           AND readiness_scope_kind = 'source'
           AND NOT EXISTS (
               SELECT 1
               FROM source_readiness_targets AS target
               WHERE target.source_id = analysis_jobs.source_id
                 AND target.scope_kind = analysis_jobs.readiness_scope_kind
                 AND target.scope_id = analysis_jobs.readiness_scope_id
                 AND target.stage = analysis_jobs.readiness_stage
                 AND target.required_version = analysis_jobs.artifact_version
                 AND target.source_generation = analysis_jobs.source_generation
                 AND target.content_generation = analysis_jobs.content_generation
           )",
        [source_id],
    )?);
    for target in upsert_targets {
        changed = changed.saturating_add(refresh_work_metadata(&tx, target)?);
    }
    changed = changed.saturating_add(tx.execute(
        "INSERT INTO source_readiness_sources (
            source_id, source_generation, readiness_revision, availability, contract_version,
            membership_digest, membership_count, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(source_id) DO UPDATE SET
            source_generation = excluded.source_generation,
            readiness_revision = excluded.readiness_revision,
            availability = excluded.availability,
            contract_version = excluded.contract_version,
            membership_digest = excluded.membership_digest,
            membership_count = excluded.membership_count,
            updated_at = excluded.updated_at",
        params![
            source_id,
            source_generation,
            readiness_revision,
            availability.as_str(),
            contract_version,
            membership.digest.as_slice(),
            i64::try_from(membership.count).unwrap_or(i64::MAX),
            updated_at,
        ],
    )?);
    cancellation_checkpoint(cancel)?;
    tx.commit()?;
    Ok(ReadinessDeltaPublicationOutcome::Applied {
        membership_generation,
        changed,
    })
}

/// Publish a completion only when its version and generations still match the desired target.
pub(crate) fn publish_readiness_artifact(
    connection: &mut Connection,
    artifact: &ReadinessArtifact,
) -> Result<ArtifactPublishOutcome, ReadinessError> {
    if artifact.artifact_version.trim().is_empty() {
        return Err(ReadinessError::InvalidArtifactVersion {
            source_id: artifact.source_id.clone(),
            scope_id: artifact.scope_id.clone(),
            stage: artifact.stage,
        });
    }
    if artifact.content_generation.trim().is_empty() {
        return Err(ReadinessError::InvalidContentGeneration {
            source_id: artifact.source_id.clone(),
            scope_id: artifact.scope_id.clone(),
            stage: artifact.stage,
        });
    }
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
              AND (
                  target.scope_kind = 'file'
                  OR source.source_generation = target.source_generation
              )
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
            relative_path,
            stage,
            artifact_version,
            source_generation,
            content_generation,
            artifact_ref,
            completed_at
         ) VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, NULL, ?8)
         ON CONFLICT(source_id, scope_kind, scope_id, stage) DO UPDATE SET
            relative_path = NULL,
            artifact_version = excluded.artifact_version,
            source_generation = excluded.source_generation,
            content_generation = excluded.content_generation,
            artifact_ref = NULL,
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

/// Remove an exact artifact whose backing payload has been proven missing or corrupt.
///
/// The desired target must still match the supplied generation and version. This fence prevents
/// a stale worker from invalidating a replacement artifact published for newer content.
pub(crate) fn invalidate_readiness_artifact(
    connection: &mut Connection,
    target: &ReadinessTarget,
) -> Result<bool, ReadinessError> {
    let changed = connection.execute(
        "DELETE FROM source_readiness_artifacts AS artifact
         WHERE artifact.source_id = ?1
           AND artifact.scope_kind = ?2
           AND artifact.scope_id = ?3
           AND artifact.stage = ?4
           AND artifact.artifact_version = ?5
           AND artifact.content_generation = ?6
           AND (?2 = 'file' OR artifact.source_generation = ?7)
           AND EXISTS (
               SELECT 1
               FROM source_readiness_sources AS source
            JOIN source_readiness_targets AS target
              ON target.source_id = source.source_id
             AND (
                 target.scope_kind = 'file'
                 OR target.source_generation = source.source_generation
             )
               WHERE target.source_id = artifact.source_id
                 AND target.scope_kind = artifact.scope_kind
                 AND target.scope_id = artifact.scope_id
                 AND target.stage = artifact.stage
                 AND target.required_version = artifact.artifact_version
                 AND target.content_generation = artifact.content_generation
                 AND (?2 = 'file' OR target.source_generation = ?7)
                 AND target.eligibility = 'eligible'
                 AND source.availability = 'active'
           )",
        params![
            target.source_id,
            target.scope_kind.as_str(),
            target.scope_id,
            target.stage.as_str(),
            target.required_version,
            target.content_generation,
            target.source_generation,
        ],
    )?;
    Ok(changed == 1)
}

/// Fence in-flight claims, retire current derived rows, and make the exact
/// current analysis targets actionable again.
pub(crate) fn requeue_source_analysis(
    connection: &mut Connection,
    source_id: &str,
    requested_at: i64,
) -> Result<usize, ReadinessError> {
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let analysis_stages = "('analysis_features', 'embedding_aspects', 'similarity_layout')";
    let sample_target = "EXISTS (
        SELECT 1
        FROM source_readiness_targets AS target
        WHERE target.source_id = ?1
          AND target.scope_kind = 'file'
          AND target.relative_path IS NOT NULL
          AND target.stage IN ('analysis_features', 'embedding_aspects')
          AND sample_id = target.source_id || '::' || target.relative_path
    )";
    let mut changed = 0usize;
    for table in [
        "analysis_features",
        "features",
        "embeddings",
        "similarity_aspect_descriptors",
    ] {
        changed = changed.saturating_add(tx.execute(
            &format!("DELETE FROM {table} WHERE {sample_target}"),
            [source_id],
        )?);
    }
    let source_sample_prefix = format!("{source_id}::");
    for table in ["layout_umap", "hdbscan_clusters"] {
        changed = changed.saturating_add(tx.execute(
            &format!(
                "DELETE FROM {table}
                 WHERE substr(sample_id, 1, length(?1)) = ?1"
            ),
            [&source_sample_prefix],
        )?);
    }
    changed = changed.saturating_add(tx.execute(
        &format!(
            "DELETE FROM source_readiness_artifacts
             WHERE source_id = ?1
               AND stage IN {analysis_stages}"
        ),
        [source_id],
    )?);
    changed = changed.saturating_add(tx.execute(
        &format!(
            "UPDATE analysis_jobs
             SET status = 'pending',
                 attempts = 0,
                 running_at = NULL,
                 last_error = NULL,
                 readiness_claim_generation = readiness_claim_generation + 1,
                 retry_at = NULL,
                 failure_kind = NULL,
                 failure_code = NULL,
                 lease_expires_at = NULL,
                 created_at = ?2
             WHERE source_id = ?1
               AND readiness_managed = 1
               AND readiness_stage IN {analysis_stages}"
        ),
        params![source_id, requested_at],
    )?);
    tx.execute(
        "UPDATE source_readiness_sources
         SET readiness_revision = readiness_revision + 1,
             updated_at = ?2
         WHERE source_id = ?1",
        params![source_id, requested_at],
    )?;
    tx.commit()?;
    Ok(changed)
}

/// Persist actionable deficits into the existing source-local analysis job table.
///
/// These rows are readiness-managed so the legacy analysis claimant ignores them. OPT-1178's
/// supervisor can claim them through the unified readiness contract.
#[cfg(test)]
pub fn persist_readiness_deficits(
    connection: &mut Connection,
    deficits: &[ReadinessDeficit],
    created_at: i64,
) -> Result<usize, ReadinessError> {
    persist_readiness_deficits_inner(connection, deficits, created_at, None, &mut || {})
}

pub(super) fn persist_readiness_deficits_inner(
    connection: &mut Connection,
    deficits: &[ReadinessDeficit],
    created_at: i64,
    cancel: Option<&AtomicBool>,
    progress: &mut dyn FnMut(),
) -> Result<usize, ReadinessError> {
    cancellation_checkpoint(cancel)?;
    let mut unique = BTreeSet::new();
    let tx = connection.transaction()?;
    let mut changed = 0;
    for deficit in deficits {
        cancellation_checkpoint(cancel)?;
        progress();
        let target = &deficit.target;
        if !unique.insert((
            target.key(),
            target.required_version.clone(),
            target.source_generation,
            target.content_generation.clone(),
        )) {
            continue;
        }
        let Some(current_target) = current_actionable_target(&tx, target)? else {
            continue;
        };
        let sample_id = match current_target.scope_kind {
            ReadinessScopeKind::File => current_target.scope_id.clone(),
            ReadinessScopeKind::Source => format!("{}::__source__", current_target.source_id),
        };
        changed += match target_persistence_action(&tx, &current_target, created_at)? {
            TargetPersistenceAction::Persist => {
                persist_deficit(&tx, &current_target, &sample_id, created_at)?
            }
            TargetPersistenceAction::RefreshMetadata => {
                refresh_work_metadata(&tx, &current_target)?
            }
            TargetPersistenceAction::Skip => 0,
        };
    }
    tx.commit()?;
    Ok(changed)
}

fn current_actionable_target(
    tx: &Transaction<'_>,
    target: &ReadinessTarget,
) -> Result<Option<ReadinessTarget>, rusqlite::Error> {
    let current = tx
        .query_row(
            "SELECT current.relative_path, current.source_generation
            FROM source_readiness_sources AS source
            JOIN source_readiness_targets AS current
              ON current.source_id = source.source_id
            WHERE current.source_id = ?1
              AND current.scope_kind = ?2
              AND current.scope_id = ?3
              AND current.stage = ?4
              AND current.required_version = ?5
              AND current.content_generation = ?6
              AND current.eligibility = 'eligible'
              AND source.availability = 'active'
              AND (
                  current.scope_kind = 'file'
                  OR source.source_generation = current.source_generation
              )
              AND (?2 = 'file' OR current.source_generation = ?7)",
            params![
                target.source_id,
                target.scope_kind.as_str(),
                target.scope_id,
                target.stage.as_str(),
                target.required_version,
                target.content_generation,
                target.source_generation,
            ],
            |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()?;
    Ok(current.map(|(relative_path, source_generation)| {
        let mut current_target = target.clone();
        current_target.relative_path = relative_path;
        current_target.source_generation = source_generation;
        current_target
    }))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TargetPersistenceAction {
    Persist,
    RefreshMetadata,
    Skip,
}

fn target_persistence_action(
    tx: &Transaction<'_>,
    target: &ReadinessTarget,
    now: i64,
) -> Result<TargetPersistenceAction, rusqlite::Error> {
    let artifact_is_current = tx.query_row(
        "SELECT EXISTS(
            SELECT 1
            FROM source_readiness_artifacts
            WHERE source_id = ?1
              AND scope_kind = ?2
              AND scope_id = ?3
              AND stage = ?4
              AND artifact_version = ?5
              AND content_generation = ?6
              AND (?2 = 'file' OR source_generation = ?7)
        )",
        params![
            target.source_id,
            target.scope_kind.as_str(),
            target.scope_id,
            target.stage.as_str(),
            target.required_version,
            target.content_generation,
            target.source_generation,
        ],
        |row| row.get::<_, bool>(0),
    )?;
    if artifact_is_current {
        return Ok(TargetPersistenceAction::Skip);
    }

    let matching_work = tx
        .query_row(
            "SELECT status, retry_at, failure_kind, lease_expires_at,
                    relative_path, source_generation
             FROM analysis_jobs
             WHERE source_id = ?1
               AND readiness_managed = 1
               AND readiness_scope_kind = ?2
               AND readiness_scope_id = ?3
               AND readiness_stage = ?4
               AND artifact_version = ?5
               AND content_generation = ?6
               AND (?2 = 'file' OR source_generation = ?7)",
            params![
                target.source_id,
                target.scope_kind.as_str(),
                target.scope_id,
                target.stage.as_str(),
                target.required_version,
                target.content_generation,
                target.source_generation,
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                ))
            },
        )
        .optional()?;
    let Some((status, retry_at, failure_kind, lease_expires_at, relative_path, source_generation)) =
        matching_work
    else {
        return Ok(TargetPersistenceAction::Persist);
    };
    let is_actionable = match status.as_str() {
        "pending" => false,
        "running" => lease_expires_at.is_none_or(|deadline| deadline <= now),
        "failed" => match failure_kind.as_deref() {
            Some("permanent" | "unsupported") => false,
            _ => retry_at.is_none_or(|deadline| deadline <= now),
        },
        _ => true,
    };
    if is_actionable {
        return Ok(TargetPersistenceAction::Persist);
    }
    let metadata_is_current = relative_path == target.relative_path.as_deref().unwrap_or("")
        && source_generation == Some(target.source_generation);
    Ok(if metadata_is_current {
        TargetPersistenceAction::Skip
    } else {
        TargetPersistenceAction::RefreshMetadata
    })
}

fn refresh_work_metadata(
    tx: &Transaction<'_>,
    target: &ReadinessTarget,
) -> Result<usize, rusqlite::Error> {
    tx.execute(
        "UPDATE analysis_jobs
         SET relative_path = ?1,
             source_generation = ?2
         WHERE source_id = ?3
           AND readiness_managed = 1
           AND readiness_scope_kind = ?4
           AND readiness_scope_id = ?5
           AND readiness_stage = ?6
           AND artifact_version = ?7
           AND content_generation = ?8
           AND (?4 = 'file' OR source_generation = ?2)",
        params![
            target.relative_path.as_deref().unwrap_or(""),
            target.source_generation,
            target.source_id,
            target.scope_kind.as_str(),
            target.scope_id,
            target.stage.as_str(),
            target.required_version,
            target.content_generation,
        ],
    )
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
                 AND (
                    excluded.readiness_scope_kind = 'file'
                    OR analysis_jobs.source_generation = excluded.source_generation
                 )
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
    cancel: Option<&AtomicBool>,
) -> Result<(), ReadinessError> {
    let mut keys = BTreeSet::new();
    let mut file_stages = BTreeMap::<String, BTreeSet<ReadinessStage>>::new();
    let mut has_similarity_layout = false;
    for target in targets {
        cancellation_checkpoint(cancel)?;
        if target.source_id != source_id || target.source_generation != source_generation {
            return Err(ReadinessError::TargetGenerationMismatch {
                source_id: source_id.to_string(),
                generation: source_generation,
            });
        }
        if target.content_generation.trim().is_empty() {
            return Err(ReadinessError::InvalidContentGeneration {
                source_id: target.source_id.clone(),
                scope_id: target.scope_id.clone(),
                stage: target.stage,
            });
        }
        if target.required_version.trim().is_empty() {
            return Err(ReadinessError::InvalidArtifactVersion {
                source_id: target.source_id.clone(),
                scope_id: target.scope_id.clone(),
                stage: target.stage,
            });
        }
        let scope_is_valid = match target.stage {
            ReadinessStage::SimilarityLayout => target.scope_kind == ReadinessScopeKind::Source,
            _ => target.scope_kind == ReadinessScopeKind::File,
        };
        if !scope_is_valid {
            return Err(ReadinessError::InvalidStageScope {
                source_id: target.source_id.clone(),
                scope_id: target.scope_id.clone(),
                stage: target.stage,
                scope_kind: target.scope_kind,
            });
        }
        match target.scope_kind {
            ReadinessScopeKind::File => {
                if target.scope_id.trim().is_empty() {
                    return Err(ReadinessError::InvalidScopeIdentity {
                        source_id: target.source_id.clone(),
                        scope_id: target.scope_id.clone(),
                        scope_kind: target.scope_kind,
                    });
                }
                if target.eligibility == super::super::model::ReadinessEligibility::Eligible
                    && target
                        .relative_path
                        .as_deref()
                        .is_none_or(|path| path.trim().is_empty())
                {
                    return Err(ReadinessError::InvalidRelativePath {
                        source_id: target.source_id.clone(),
                        scope_id: target.scope_id.clone(),
                        stage: target.stage,
                    });
                }
                file_stages
                    .entry(target.scope_id.clone())
                    .or_default()
                    .insert(target.stage);
            }
            ReadinessScopeKind::Source => {
                if target.scope_id != source_id || target.relative_path.is_some() {
                    return Err(ReadinessError::InvalidScopeIdentity {
                        source_id: target.source_id.clone(),
                        scope_id: target.scope_id.clone(),
                        scope_kind: target.scope_kind,
                    });
                }
                has_similarity_layout = true;
            }
        }
        if !keys.insert(target.key()) {
            return Err(ReadinessError::DuplicateTarget {
                source_id: target.source_id.clone(),
                scope_id: target.scope_id.clone(),
                stage: target.stage,
            });
        }
    }
    for (scope_id, stages) in file_stages {
        cancellation_checkpoint(cancel)?;
        for stage in [
            ReadinessStage::IndexedIdentity,
            ReadinessStage::AnalysisFeatures,
            ReadinessStage::EmbeddingAspects,
        ] {
            if !stages.contains(&stage) {
                return Err(ReadinessError::IncompleteTargetMatrix { scope_id, stage });
            }
        }
    }
    if !has_similarity_layout {
        return Err(ReadinessError::IncompleteTargetMatrix {
            scope_id: source_id.to_string(),
            stage: ReadinessStage::SimilarityLayout,
        });
    }
    Ok(())
}

fn validate_contract_version(contract_version: &str) -> Result<(), ReadinessError> {
    if contract_version.trim().is_empty() {
        Err(ReadinessError::InvalidContractVersion)
    } else {
        Ok(())
    }
}

fn membership_from_targets(targets: &[ReadinessTarget]) -> ReadinessMembership {
    let mut membership = ReadinessMembership::default();
    for target in targets.iter().filter(|target| {
        target.stage == ReadinessStage::EmbeddingAspects
            && target.eligibility == ReadinessEligibility::Eligible
    }) {
        membership.add(&target.scope_id, &target.content_generation);
    }
    membership
}

fn validate_delta_targets(
    source_id: &str,
    source_generation: i64,
    targets: &[ReadinessTarget],
    cancel: Option<&AtomicBool>,
) -> Result<(), ReadinessError> {
    let mut keys = BTreeSet::new();
    let mut file_stages = BTreeMap::<String, BTreeSet<ReadinessStage>>::new();
    for target in targets {
        cancellation_checkpoint(cancel)?;
        if target.source_id != source_id
            || target.source_generation != source_generation
            || target.scope_kind != ReadinessScopeKind::File
        {
            return Err(ReadinessError::TargetGenerationMismatch {
                source_id: source_id.to_string(),
                generation: source_generation,
            });
        }
        if target.content_generation.trim().is_empty() {
            return Err(ReadinessError::InvalidContentGeneration {
                source_id: target.source_id.clone(),
                scope_id: target.scope_id.clone(),
                stage: target.stage,
            });
        }
        if target.required_version.trim().is_empty() {
            return Err(ReadinessError::InvalidArtifactVersion {
                source_id: target.source_id.clone(),
                scope_id: target.scope_id.clone(),
                stage: target.stage,
            });
        }
        if target.scope_id.trim().is_empty()
            || (target.eligibility == ReadinessEligibility::Eligible
                && target
                    .relative_path
                    .as_deref()
                    .is_none_or(|path| path.trim().is_empty()))
        {
            return Err(ReadinessError::InvalidRelativePath {
                source_id: target.source_id.clone(),
                scope_id: target.scope_id.clone(),
                stage: target.stage,
            });
        }
        if !keys.insert(target.key()) {
            return Err(ReadinessError::DuplicateTarget {
                source_id: target.source_id.clone(),
                scope_id: target.scope_id.clone(),
                stage: target.stage,
            });
        }
        file_stages
            .entry(target.scope_id.clone())
            .or_default()
            .insert(target.stage);
    }
    for (scope_id, stages) in file_stages {
        for stage in [
            ReadinessStage::IndexedIdentity,
            ReadinessStage::AnalysisFeatures,
            ReadinessStage::EmbeddingAspects,
        ] {
            if !stages.contains(&stage) {
                return Err(ReadinessError::IncompleteTargetMatrix { scope_id, stage });
            }
        }
    }
    Ok(())
}

fn validate_delta_manifest_membership(
    tx: &Transaction<'_>,
    upsert_targets: &[ReadinessTarget],
    deleted_scope_ids: &[String],
    cancel: Option<&AtomicBool>,
) -> Result<(), ReadinessError> {
    let mut desired_paths = BTreeMap::<String, String>::new();
    for target in upsert_targets {
        cancellation_checkpoint(cancel)?;
        let path = target.relative_path.as_deref().unwrap_or_default();
        if let Some(existing) = desired_paths.insert(target.scope_id.clone(), path.to_string())
            && existing != path
        {
            return Err(ReadinessError::InconsistentTargetPath {
                identity: target.scope_id.clone(),
            });
        }
    }
    for (identity, path) in &desired_paths {
        let current = tx.query_row(
            "SELECT MIN(path), COUNT(*)
                 FROM wav_files
                 WHERE missing = 0 AND file_identity = ?1",
            [identity],
            |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, i64>(1)?)),
        )?;
        match current {
            (Some(current), 1) if current == *path => {}
            (Some(first_path), count) if count > 1 => {
                return Err(ReadinessError::DuplicateManifestIdentity {
                    identity: identity.clone(),
                    first_path,
                    second_path: path.clone(),
                });
            }
            (Some(current), _) => {
                return Err(ReadinessError::ManifestPathMismatch {
                    identity: identity.clone(),
                    expected: current,
                    supplied: path.clone(),
                });
            }
            (None, _) => {
                return Err(ReadinessError::ManifestMembershipMismatch {
                    missing: vec![identity.clone()],
                    unexpected: Vec::new(),
                });
            }
        }
    }
    for identity in deleted_scope_ids {
        cancellation_checkpoint(cancel)?;
        if desired_paths.contains_key(identity) {
            continue;
        }
        let still_current = tx.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM wav_files
                WHERE missing = 0 AND file_identity = ?1
            )",
            [identity],
            |row| row.get::<_, bool>(0),
        )?;
        if still_current {
            return Err(ReadinessError::ManifestMembershipMismatch {
                missing: Vec::new(),
                unexpected: vec![identity.clone()],
            });
        }
    }
    Ok(())
}

fn decode_membership_digest(raw: &[u8]) -> Result<[u8; 32], ReadinessError> {
    raw.try_into()
        .map_err(|_| ReadinessError::SimilarityMembershipMismatch)
}

fn validate_manifest_membership(
    tx: &Transaction<'_>,
    targets: &[ReadinessTarget],
    cancel: Option<&AtomicBool>,
) -> Result<(), ReadinessError> {
    let mut desired = BTreeMap::<String, String>::new();
    for target in targets.iter().filter(|target| {
        target.scope_kind == ReadinessScopeKind::File
            && target.eligibility != super::super::model::ReadinessEligibility::Deleted
    }) {
        cancellation_checkpoint(cancel)?;
        let path = target
            .relative_path
            .as_deref()
            .filter(|path| !path.trim().is_empty())
            .ok_or_else(|| ReadinessError::InvalidRelativePath {
                source_id: target.source_id.clone(),
                scope_id: target.scope_id.clone(),
                stage: target.stage,
            })?;
        if desired
            .insert(target.scope_id.clone(), path.to_string())
            .is_some_and(|current| current != path)
        {
            return Err(ReadinessError::InconsistentTargetPath {
                identity: target.scope_id.clone(),
            });
        }
    }
    let filter = crate::sample_sources::supported_audio_where_clause();
    let mut statement = tx.prepare(&format!(
        "SELECT path, file_identity
         FROM wav_files
         WHERE missing = 0 AND {filter}
         ORDER BY path"
    ))?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    })?;
    let mut manifest = BTreeMap::<String, String>::new();
    for row in rows {
        cancellation_checkpoint(cancel)?;
        let (path, identity) = row?;
        let Some(identity) = identity.filter(|identity| !identity.trim().is_empty()) else {
            return Err(ReadinessError::ManifestIdentityUnavailable { path });
        };
        if let Some(first_path) = manifest.insert(identity.clone(), path.clone()) {
            return Err(ReadinessError::DuplicateManifestIdentity {
                identity,
                first_path,
                second_path: path,
            });
        }
    }
    let manifest_identities = manifest.keys().cloned().collect::<BTreeSet<_>>();
    let desired_identities = desired.keys().cloned().collect::<BTreeSet<_>>();
    if manifest_identities != desired_identities {
        return Err(ReadinessError::ManifestMembershipMismatch {
            missing: manifest_identities
                .difference(&desired_identities)
                .cloned()
                .collect(),
            unexpected: desired_identities
                .difference(&manifest_identities)
                .cloned()
                .collect(),
        });
    }
    for (identity, expected) in manifest {
        cancellation_checkpoint(cancel)?;
        let supplied = desired
            .get(&identity)
            .expect("identity sets were compared before paths");
        if supplied != &expected {
            return Err(ReadinessError::ManifestPathMismatch {
                identity,
                expected,
                supplied: supplied.clone(),
            });
        }
    }
    Ok(())
}

fn cancellation_checkpoint(cancel: Option<&AtomicBool>) -> Result<(), ReadinessError> {
    if cancel.is_some_and(|cancel| cancel.load(Ordering::Acquire)) {
        Err(ReadinessError::Cancelled)
    } else {
        Ok(())
    }
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

fn upsert_target(
    tx: &Transaction<'_>,
    target: &ReadinessTarget,
    updated_at: i64,
) -> Result<usize, rusqlite::Error> {
    tx.execute(
        "INSERT INTO source_readiness_targets (
            source_id, scope_kind, scope_id, relative_path, stage, required_version,
            source_generation, content_generation, eligibility, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(source_id, scope_kind, scope_id, stage) DO UPDATE SET
            relative_path = excluded.relative_path,
            required_version = excluded.required_version,
            source_generation = excluded.source_generation,
            content_generation = excluded.content_generation,
            eligibility = excluded.eligibility,
            updated_at = excluded.updated_at
         WHERE source_readiness_targets.relative_path IS NOT excluded.relative_path
            OR source_readiness_targets.required_version != excluded.required_version
            OR source_readiness_targets.source_generation != excluded.source_generation
            OR source_readiness_targets.content_generation != excluded.content_generation
            OR source_readiness_targets.eligibility != excluded.eligibility",
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
    )
}
