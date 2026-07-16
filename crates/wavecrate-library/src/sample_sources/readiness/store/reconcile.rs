use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
};

use rusqlite::{Connection, OptionalExtension};

use super::super::{
    model::{
        ReadinessEligibility, ReadinessKey, ReadinessScopeKind, ReadinessStage, ReadinessTarget,
        SourceAvailability,
    },
    snapshot::{
        ReadinessActivity, ReadinessClassification, ReadinessDeficit, ReadinessEntry,
        ReadinessSnapshot, ReadinessStageCounts,
    },
};
use super::error::ReadinessError;

/// Compare desired readiness with persisted artifacts and work without mutating the database.
pub fn reconcile_readiness(
    connection: &Connection,
    source_id: &str,
    now: i64,
) -> Result<ReadinessSnapshot, ReadinessError> {
    reconcile_readiness_inner(connection, source_id, now, None)
}

/// Compare desired readiness with persisted state while honoring cancellation during large scans.
pub fn reconcile_readiness_with_cancel(
    connection: &Connection,
    source_id: &str,
    now: i64,
    cancel: &AtomicBool,
) -> Result<ReadinessSnapshot, ReadinessError> {
    reconcile_readiness_inner(connection, source_id, now, Some(cancel))
}

fn reconcile_readiness_inner(
    connection: &Connection,
    source_id: &str,
    now: i64,
    cancel: Option<&AtomicBool>,
) -> Result<ReadinessSnapshot, ReadinessError> {
    let tx = connection.unchecked_transaction()?;
    let snapshot = reconcile_readiness_snapshot(&tx, source_id, now, || {}, cancel)?;
    tx.commit()?;
    Ok(snapshot)
}

#[cfg(test)]
pub(crate) fn reconcile_readiness_with_hook(
    connection: &Connection,
    source_id: &str,
    now: i64,
    after_source_state: impl FnOnce(),
) -> Result<ReadinessSnapshot, ReadinessError> {
    let tx = connection.unchecked_transaction()?;
    let snapshot = reconcile_readiness_snapshot(&tx, source_id, now, after_source_state, None)?;
    tx.commit()?;
    Ok(snapshot)
}

fn reconcile_readiness_snapshot(
    connection: &Connection,
    source_id: &str,
    now: i64,
    after_source_state: impl FnOnce(),
    cancel: Option<&AtomicBool>,
) -> Result<ReadinessSnapshot, ReadinessError> {
    cancellation_checkpoint(cancel)?;
    if !readiness_schema_available(connection)? {
        return Err(ReadinessError::SchemaUnavailable);
    }
    let (source_generation, readiness_revision, availability) = connection
        .query_row(
            "SELECT source_generation, readiness_revision, availability
             FROM source_readiness_sources
             WHERE source_id = ?1",
            [source_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| ReadinessError::UnknownSource(source_id.to_string()))?;
    let availability = SourceAvailability::from_stored(&availability).ok_or_else(|| {
        ReadinessError::UnknownStoredValue {
            field: "availability",
            value: availability,
        }
    })?;
    let source_state = StoredSourceState {
        generation: source_generation,
        readiness_revision,
        availability,
    };
    after_source_state();
    let targets = load_targets(connection, source_id, cancel)?;
    let artifacts = load_artifacts(connection, source_id, cancel)?;
    let work = load_work(connection, source_id, cancel)?;
    build_snapshot(
        source_id,
        source_state,
        targets,
        artifacts,
        work,
        now,
        cancel,
    )
}

fn readiness_schema_available(connection: &Connection) -> Result<bool, rusqlite::Error> {
    connection.query_row(
        "SELECT COUNT(*) = 3
         FROM sqlite_master
         WHERE type = 'table'
           AND name IN (
               'source_readiness_sources',
               'source_readiness_targets',
               'source_readiness_artifacts'
           )",
        [],
        |row| row.get(0),
    )
}

#[derive(Clone, Debug)]
struct StoredSourceState {
    generation: i64,
    readiness_revision: i64,
    availability: SourceAvailability,
}

#[derive(Clone, Debug)]
struct StoredArtifact {
    artifact_version: String,
    source_generation: i64,
    content_generation: String,
}

#[derive(Clone, Debug)]
struct StoredWork {
    status: String,
    artifact_version: Option<String>,
    source_generation: Option<i64>,
    content_generation: Option<String>,
    retry_at: Option<i64>,
    failure_kind: Option<String>,
    last_error: Option<String>,
    lease_expires_at: Option<i64>,
    created_at: i64,
}

fn load_targets(
    connection: &Connection,
    source_id: &str,
    cancel: Option<&AtomicBool>,
) -> Result<Vec<ReadinessTarget>, ReadinessError> {
    let mut statement = connection.prepare(
        "SELECT scope_kind, scope_id, relative_path, stage, required_version,
                source_generation, content_generation, eligibility
         FROM source_readiness_targets
         WHERE source_id = ?1
         ORDER BY scope_kind, scope_id, stage",
    )?;
    let mut rows = statement.query([source_id])?;
    let mut targets = Vec::new();
    while let Some(row) = rows.next()? {
        cancellation_checkpoint(cancel)?;
        targets.push(ReadinessTarget {
            source_id: source_id.to_string(),
            scope_kind: decode_scope_kind(row.get(0)?)?,
            scope_id: row.get(1)?,
            relative_path: row.get(2)?,
            stage: decode_stage(row.get(3)?)?,
            required_version: row.get(4)?,
            source_generation: row.get(5)?,
            content_generation: row.get(6)?,
            eligibility: decode_eligibility(row.get(7)?)?,
        });
    }
    Ok(targets)
}

fn load_artifacts(
    connection: &Connection,
    source_id: &str,
    cancel: Option<&AtomicBool>,
) -> Result<BTreeMap<ReadinessKey, StoredArtifact>, ReadinessError> {
    let mut statement = connection.prepare(
        "SELECT scope_kind, scope_id, stage, artifact_version,
                source_generation, content_generation
         FROM source_readiness_artifacts
         WHERE source_id = ?1",
    )?;
    let mut rows = statement.query([source_id])?;
    let mut artifacts = BTreeMap::new();
    while let Some(row) = rows.next()? {
        cancellation_checkpoint(cancel)?;
        let key = ReadinessKey {
            source_id: source_id.to_string(),
            scope_kind: decode_scope_kind(row.get(0)?)?,
            scope_id: row.get(1)?,
            stage: decode_stage(row.get(2)?)?,
        };
        artifacts.insert(
            key,
            StoredArtifact {
                artifact_version: row.get(3)?,
                source_generation: row.get(4)?,
                content_generation: row.get(5)?,
            },
        );
    }
    Ok(artifacts)
}

fn load_work(
    connection: &Connection,
    source_id: &str,
    cancel: Option<&AtomicBool>,
) -> Result<BTreeMap<ReadinessKey, StoredWork>, ReadinessError> {
    let mut statement = connection.prepare(
        "SELECT readiness_scope_kind, readiness_scope_id, readiness_stage, status,
                artifact_version, source_generation, content_generation, retry_at,
                failure_kind, last_error, lease_expires_at, created_at
         FROM analysis_jobs
         WHERE source_id = ?1 AND readiness_managed = 1
         ORDER BY id",
    )?;
    let mut rows = statement.query([source_id])?;
    let mut work = BTreeMap::new();
    while let Some(row) = rows.next()? {
        cancellation_checkpoint(cancel)?;
        let key = ReadinessKey {
            source_id: source_id.to_string(),
            scope_kind: decode_scope_kind(row.get(0)?)?,
            scope_id: row.get(1)?,
            stage: decode_stage(row.get(2)?)?,
        };
        work.insert(
            key,
            StoredWork {
                status: row.get(3)?,
                artifact_version: row.get(4)?,
                source_generation: row.get(5)?,
                content_generation: row.get(6)?,
                retry_at: row.get(7)?,
                failure_kind: row.get(8)?,
                last_error: row.get(9)?,
                lease_expires_at: row.get(10)?,
                created_at: row.get(11)?,
            },
        );
    }
    Ok(work)
}

fn cancellation_checkpoint(cancel: Option<&AtomicBool>) -> Result<(), ReadinessError> {
    if cancel.is_some_and(|cancel| cancel.load(Ordering::Acquire)) {
        Err(ReadinessError::Cancelled)
    } else {
        Ok(())
    }
}

fn build_snapshot(
    source_id: &str,
    source_state: StoredSourceState,
    targets: Vec<ReadinessTarget>,
    artifacts: BTreeMap<ReadinessKey, StoredArtifact>,
    work: BTreeMap<ReadinessKey, StoredWork>,
    now: i64,
    cancel: Option<&AtomicBool>,
) -> Result<ReadinessSnapshot, ReadinessError> {
    let mut entries = Vec::with_capacity(targets.len());
    let mut deficits = Vec::new();
    let mut stage_counts = BTreeMap::new();
    for target in targets {
        cancellation_checkpoint(cancel)?;
        let key = target.key();
        let classification = classify_target(
            &target,
            source_state.availability,
            artifacts.get(&key),
            work.get(&key),
            now,
        );
        record_count(&mut stage_counts, target.stage, &classification);
        if classification.is_actionable(now) {
            deficits.push(ReadinessDeficit {
                target: target.clone(),
                reason: classification.clone(),
                enqueued_at: work.get(&key).map(|work| work.created_at),
            });
        }
        entries.push(ReadinessEntry {
            target,
            classification,
        });
    }
    let activity = resolve_activity(&entries, &deficits, now);
    Ok(ReadinessSnapshot {
        source_id: source_id.to_string(),
        source_generation: source_state.generation,
        readiness_revision: source_state.readiness_revision,
        availability: source_state.availability,
        entries,
        deficits,
        stage_counts,
        activity,
    })
}

fn resolve_activity(
    entries: &[ReadinessEntry],
    deficits: &[ReadinessDeficit],
    now: i64,
) -> ReadinessActivity {
    if !deficits.is_empty() {
        ReadinessActivity::Actionable
    } else if entries.iter().any(|entry| {
        matches!(
            entry.classification,
            ReadinessClassification::Running { .. }
        )
    }) {
        ReadinessActivity::Running
    } else if entries.iter().any(|entry| {
        matches!(
            entry.classification,
            ReadinessClassification::RetryableFailure { retry_at, .. } if retry_at > now
        )
    }) {
        ReadinessActivity::WaitingForRetry
    } else {
        ReadinessActivity::Idle
    }
}

fn classify_target(
    target: &ReadinessTarget,
    availability: SourceAvailability,
    artifact: Option<&StoredArtifact>,
    work: Option<&StoredWork>,
    now: i64,
) -> ReadinessClassification {
    match availability {
        SourceAvailability::Offline => return ReadinessClassification::Offline,
        SourceAvailability::Disabled => return ReadinessClassification::Disabled,
        SourceAvailability::Active => {}
    }
    match target.eligibility {
        ReadinessEligibility::Unsupported => return ReadinessClassification::Unsupported,
        ReadinessEligibility::Deleted => return ReadinessClassification::Deleted,
        ReadinessEligibility::Eligible => {}
    }
    if artifact.is_some_and(|artifact| artifact_matches(target, artifact)) {
        return ReadinessClassification::Current;
    }
    if let Some(work) = work {
        if !work_matches(target, work) {
            return ReadinessClassification::StaleByGeneration;
        }
        return classify_matching_work(work, now);
    }
    if artifact.is_some() {
        ReadinessClassification::StaleByGeneration
    } else {
        ReadinessClassification::Pending
    }
}

fn classify_matching_work(work: &StoredWork, now: i64) -> ReadinessClassification {
    match work.status.as_str() {
        "pending" => ReadinessClassification::Pending,
        "running" => match work.lease_expires_at {
            Some(lease_expires_at) if lease_expires_at > now => {
                ReadinessClassification::Running { lease_expires_at }
            }
            _ => ReadinessClassification::RetryableFailure {
                retry_at: now,
                reason: "lease_expired".to_string(),
            },
        },
        "failed" => classify_failure(work, now),
        _ => ReadinessClassification::Pending,
    }
}

fn classify_failure(work: &StoredWork, now: i64) -> ReadinessClassification {
    let reason = work
        .last_error
        .clone()
        .unwrap_or_else(|| "readiness_work_failed".to_string());
    match work.failure_kind.as_deref() {
        Some("permanent") => ReadinessClassification::PermanentFailure { reason },
        Some("unsupported") => ReadinessClassification::Unsupported,
        _ => ReadinessClassification::RetryableFailure {
            retry_at: work.retry_at.unwrap_or(now),
            reason,
        },
    }
}

fn artifact_matches(target: &ReadinessTarget, artifact: &StoredArtifact) -> bool {
    artifact.artifact_version == target.required_version
        && source_generation_matches(
            target.scope_kind,
            target.source_generation,
            artifact.source_generation,
        )
        && artifact.content_generation == target.content_generation
}

fn work_matches(target: &ReadinessTarget, work: &StoredWork) -> bool {
    work.artifact_version.as_deref() == Some(target.required_version.as_str())
        && work.source_generation.is_some_and(|generation| {
            source_generation_matches(target.scope_kind, target.source_generation, generation)
        })
        && work.content_generation.as_deref() == Some(target.content_generation.as_str())
}

fn source_generation_matches(
    scope_kind: ReadinessScopeKind,
    target_generation: i64,
    stored_generation: i64,
) -> bool {
    scope_kind == ReadinessScopeKind::File || stored_generation == target_generation
}

fn record_count(
    counts: &mut BTreeMap<ReadinessStage, ReadinessStageCounts>,
    stage: ReadinessStage,
    classification: &ReadinessClassification,
) {
    let count = counts.entry(stage).or_default();
    match classification {
        ReadinessClassification::Current => count.current += 1,
        ReadinessClassification::Pending => count.pending += 1,
        ReadinessClassification::Running { .. } => count.running += 1,
        ReadinessClassification::RetryableFailure { .. } => count.retryable += 1,
        ReadinessClassification::PermanentFailure { .. } => count.permanent += 1,
        ReadinessClassification::Unsupported => count.unsupported += 1,
        ReadinessClassification::Offline | ReadinessClassification::Disabled => {
            count.offline += 1;
        }
        ReadinessClassification::StaleByGeneration => count.stale += 1,
        ReadinessClassification::Deleted => count.deleted += 1,
    }
}

fn decode_scope_kind(value: String) -> Result<ReadinessScopeKind, ReadinessError> {
    ReadinessScopeKind::from_stored(&value).ok_or(ReadinessError::UnknownStoredValue {
        field: "scope_kind",
        value,
    })
}

fn decode_stage(value: String) -> Result<ReadinessStage, ReadinessError> {
    ReadinessStage::from_stored(&value).ok_or(ReadinessError::UnknownStoredValue {
        field: "stage",
        value,
    })
}

fn decode_eligibility(value: String) -> Result<ReadinessEligibility, ReadinessError> {
    ReadinessEligibility::from_stored(&value).ok_or(ReadinessError::UnknownStoredValue {
        field: "eligibility",
        value,
    })
}
