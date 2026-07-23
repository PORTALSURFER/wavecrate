#[cfg(test)]
use super::publish_current_readiness_targets_with_cancel_and_checkpoint;
use super::{
    AtomicBool, Cancellable, META_WAV_PATHS_REVISION, PendingReadinessDelta,
    ReadinessDeltaPublicationOutcome, ReadinessStore, ReadinessTargetDeltaPublication,
    SourceAvailability, cancelled, file_readiness_targets, native_similarity_artifact_version,
    readiness_contract_version,
};

#[cfg(test)]
pub(super) fn publish_current_readiness_targets(
    connection: &mut rusqlite::Connection,
    source_id: &str,
    now: i64,
) -> Result<bool, String> {
    let cancel = AtomicBool::new(false);
    match publish_current_readiness_targets_with_cancel(connection, source_id, now, &cancel)? {
        Cancellable::Completed(changed) => Ok(changed),
        Cancellable::Cancelled => unreachable!("an uncancelled publication cannot be cancelled"),
    }
}

#[cfg(test)]
pub(super) fn publish_current_readiness_targets_with_cancel(
    connection: &mut rusqlite::Connection,
    source_id: &str,
    now: i64,
    cancel: &AtomicBool,
) -> Result<Cancellable<bool>, String> {
    publish_current_readiness_targets_with_cancel_and_checkpoint(
        connection,
        source_id,
        now,
        cancel,
        false,
        &mut |_| {},
    )
}

pub(super) fn publish_current_readiness_delta_with_cancel(
    connection: &mut rusqlite::Connection,
    source_id: &str,
    delta: &PendingReadinessDelta,
    now: i64,
    cancel: &AtomicBool,
) -> Result<Cancellable<Option<usize>>, String> {
    if cancelled(cancel) {
        return Ok(Cancellable::Cancelled);
    }
    let source_generation = connection
        .query_row(
            "SELECT COALESCE(
                (SELECT CAST(value AS INTEGER) FROM metadata WHERE key = ?1),
                0
             )",
            [META_WAV_PATHS_REVISION],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| error.to_string())?;
    let contract_version = readiness_contract_version();
    let Some(current_source) = ReadinessStore::new(connection)
        .source_state(source_id)
        .map_err(|error| error.to_string())?
    else {
        return Ok(Cancellable::Completed(None));
    };
    if current_source.contract_version != contract_version {
        return Ok(Cancellable::Completed(None));
    }
    if current_source.source_generation == source_generation {
        return Ok(Cancellable::Completed(Some(0)));
    }

    let embedding_version = format!(
        "{}+{}",
        wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
        wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
    );
    let mut targets = Vec::with_capacity(delta.scope_ids.len().saturating_mul(3));
    let mut deleted_scope_ids = Vec::new();
    for scope_id in &delta.scope_ids {
        if cancelled(cancel) {
            return Ok(Cancellable::Cancelled);
        }
        let mut statement = connection
            .prepare(
                "SELECT path, content_hash, file_size, modified_ns
                 FROM wav_files
                 WHERE missing = 0 AND file_identity = ?1
                 ORDER BY path
                 LIMIT 2",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([scope_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        drop(statement);
        match rows.as_slice() {
            [] => deleted_scope_ids.push(scope_id.clone()),
            [(path, content_hash, file_size, modified_ns)] => {
                if !wavecrate_library::sample_sources::is_supported_audio(std::path::Path::new(
                    path,
                )) {
                    return Ok(Cancellable::Completed(None));
                }
                let content_generation = content_hash
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("pending-{scope_id}-{file_size}-{modified_ns}"));
                let unsupported = *file_size <= 0
                    || ReadinessStore::new(connection)
                        .generation_is_known_unsupported(source_id, scope_id, &content_generation)
                        .map_err(|error| error.to_string())?;
                targets.extend(file_readiness_targets(
                    source_id,
                    scope_id,
                    path,
                    source_generation,
                    &content_generation,
                    embedding_version.as_str(),
                    unsupported,
                ));
            }
            _ => return Ok(Cancellable::Completed(None)),
        }
    }
    let readiness_revision = current_source.readiness_revision.saturating_add(1);
    let similarity_artifact_version = native_similarity_artifact_version();
    let publication = ReadinessTargetDeltaPublication::new(
        source_id,
        source_generation,
        readiness_revision,
        SourceAvailability::Active,
        contract_version.as_str(),
        &targets,
        &deleted_scope_ids,
        similarity_artifact_version.as_str(),
        now,
    );
    let outcome = match ReadinessStore::new(connection)
        .publish_target_delta_with_cancel(&publication, cancel)
    {
        Ok(outcome) => outcome,
        Err(wavecrate::sample_sources::readiness::ReadinessError::Cancelled) => {
            return Ok(Cancellable::Cancelled);
        }
        Err(error) => return Err(error.to_string()),
    };
    let ReadinessDeltaPublicationOutcome::Applied {
        membership_generation,
        changed,
    } = outcome
    else {
        return Ok(Cancellable::Completed(None));
    };
    let similarity_state = serde_json::json!({
        "state": "dirty",
        "source_generation": source_generation,
        "membership_generation": membership_generation,
        "artifact_version": similarity_artifact_version,
    })
    .to_string();
    wavecrate_analysis::ann_index::mark_artifacts_dirty(connection, &similarity_state)?;
    tracing::debug!(
        target: "wavecrate::source_processing",
        event = "source_processing.readiness_delta_reconciled",
        source_id,
        source_generation,
        identities = delta.scope_ids.len(),
        target_upserts = targets.len(),
        target_deletes = deleted_scope_ids.len(),
        changed,
        "Applied committed readiness target delta"
    );
    Ok(Cancellable::Completed(Some(changed)))
}
