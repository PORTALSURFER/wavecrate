use super::{
    AtomicBool, Cancellable, META_WAV_PATHS_REVISION, Ordering, READINESS_MANIFEST_VERSION,
    READINESS_MEMBERSHIP_VERSION, ReadinessEligibility, ReadinessMembership, ReadinessStage,
    ReadinessStore, ReadinessTarget, ReadinessTargetPublication, SampleSource, SourceAvailability,
    invalidate_persisted_waveform_cache_ref, native_similarity_artifact_version,
    retained_waveform_cache_ref_is_owned,
};

pub(super) fn file_readiness_targets(
    source_id: &str,
    identity: &str,
    path: &str,
    source_generation: i64,
    content_generation: &str,
    embedding_version: &str,
    unsupported: bool,
) -> [ReadinessTarget; 3] {
    let indexed = ReadinessTarget::file(
        source_id,
        identity,
        path,
        ReadinessStage::IndexedIdentity,
        READINESS_MANIFEST_VERSION,
        source_generation,
        content_generation,
    );
    let analysis = ReadinessTarget::file(
        source_id,
        identity,
        path,
        ReadinessStage::AnalysisFeatures,
        wavecrate_analysis::analysis_version(),
        source_generation,
        content_generation,
    );
    let embedding = ReadinessTarget::file(
        source_id,
        identity,
        path,
        ReadinessStage::EmbeddingAspects,
        embedding_version,
        source_generation,
        content_generation,
    );
    if unsupported {
        [
            indexed,
            analysis.with_eligibility(ReadinessEligibility::Unsupported),
            embedding.with_eligibility(ReadinessEligibility::Unsupported),
        ]
    } else {
        [indexed, analysis, embedding]
    }
}

pub(super) fn publish_current_readiness_targets_with_cancel_and_checkpoint(
    connection: &mut rusqlite::Connection,
    source_id: &str,
    now: i64,
    cancel: &AtomicBool,
    allow_revision_noop: bool,
    checkpoint: &mut impl FnMut(),
) -> Result<Cancellable<bool>, String> {
    checkpoint();
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
    let current_source = ReadinessStore::new(connection)
        .source_state(source_id)
        .map_err(|error| error.to_string())?;
    if allow_revision_noop
        && current_source.as_ref().is_some_and(|state| {
            state.source_generation == source_generation
                && state.availability == SourceAvailability::Active
                && state.contract_version == contract_version
        })
    {
        return Ok(Cancellable::Completed(false));
    }
    let rows = {
        let mut statement = connection
            .prepare(
                "SELECT path, file_identity, content_hash, file_size, modified_ns
                 FROM wav_files
                 WHERE missing = 0
                 ORDER BY path",
            )
            .map_err(|error| error.to_string())?;
        let mut query = statement.query([]).map_err(|error| error.to_string())?;
        let mut rows = Vec::new();
        while let Some(row) = query.next().map_err(|error| error.to_string())? {
            checkpoint();
            if cancelled(cancel) {
                return Ok(Cancellable::Cancelled);
            }
            rows.push((
                row.get::<_, String>(0).map_err(|error| error.to_string())?,
                row.get::<_, Option<String>>(1)
                    .map_err(|error| error.to_string())?,
                row.get::<_, Option<String>>(2)
                    .map_err(|error| error.to_string())?,
                row.get::<_, i64>(3).map_err(|error| error.to_string())?,
                row.get::<_, i64>(4).map_err(|error| error.to_string())?,
            ));
        }
        rows
    };
    let unsupported_generations = ReadinessStore::new(connection)
        .unsupported_content_generations(source_id)
        .map_err(|error| error.to_string())?;
    let mut manifest = Vec::with_capacity(rows.len());
    for (path, identity, content_hash, file_size, modified_ns) in rows {
        checkpoint();
        if cancelled(cancel) {
            return Ok(Cancellable::Cancelled);
        }
        if !wavecrate_library::sample_sources::is_supported_audio(std::path::Path::new(&path)) {
            continue;
        }
        let Some(identity) = identity.filter(|value| !value.trim().is_empty()) else {
            ReadinessStore::new(connection)
                .mark_temporarily_unavailable(source_id, now)
                .map_err(|error| error.to_string())?;
            return Ok(Cancellable::Completed(false));
        };
        let content_hash = content_hash.filter(|value| !value.trim().is_empty());
        let content_generation = content_hash
            .clone()
            .unwrap_or_else(|| format!("pending-{identity}-{file_size}-{modified_ns}"));
        manifest.push((path, identity, content_hash, content_generation, file_size));
    }
    let embedding_version = format!(
        "{}+{}",
        wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
        wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
    );
    let similarity_artifact_version = native_similarity_artifact_version();
    let mut membership = ReadinessMembership::default();
    let mut targets = Vec::with_capacity(manifest.len().saturating_mul(3).saturating_add(1));
    for (path, identity, content_hash, content_generation, file_size) in &manifest {
        checkpoint();
        if cancelled(cancel) {
            return Ok(Cancellable::Cancelled);
        }
        let analyzable = *file_size > 0;
        let unsupported = content_hash.as_ref().is_some_and(|content_hash| {
            unsupported_generations.contains(&(identity.clone(), content_hash.clone()))
        });
        if analyzable && !unsupported {
            membership.add(identity, content_generation);
        }
        targets.push(ReadinessTarget::file(
            source_id,
            identity,
            path,
            ReadinessStage::IndexedIdentity,
            READINESS_MANIFEST_VERSION,
            source_generation,
            content_generation,
        ));
        for (stage, version) in [
            (
                ReadinessStage::AnalysisFeatures,
                wavecrate_analysis::analysis_version(),
            ),
            (ReadinessStage::EmbeddingAspects, embedding_version.as_str()),
        ] {
            let mut target = ReadinessTarget::file(
                source_id,
                identity,
                path,
                stage,
                version,
                source_generation,
                content_generation,
            );
            if !analyzable || unsupported {
                target = target.with_eligibility(ReadinessEligibility::Unsupported);
            }
            targets.push(target);
        }
    }
    let membership_generation = membership.generation();
    targets.push(ReadinessTarget::source(
        source_id,
        ReadinessStage::SimilarityLayout,
        &similarity_artifact_version,
        source_generation,
        membership_generation.as_str(),
    ));
    let readiness_revision = current_source
        .map(|state| state.readiness_revision.saturating_add(1))
        .unwrap_or(1);
    let publication = ReadinessTargetPublication::new(
        source_id,
        source_generation,
        readiness_revision,
        SourceAvailability::Active,
        contract_version.as_str(),
        &targets,
        now,
    );
    match ReadinessStore::new(connection).publish_targets_with_cancel(&publication, cancel) {
        Ok(()) => {}
        Err(wavecrate::sample_sources::readiness::ReadinessError::Cancelled) => {
            return Ok(Cancellable::Cancelled);
        }
        Err(error) => return Err(error.to_string()),
    }
    if cancelled(cancel) {
        return Ok(Cancellable::Cancelled);
    }
    let similarity_state = serde_json::json!({
        "state": "dirty",
        "source_generation": source_generation,
        "membership_generation": membership_generation,
        "artifact_version": similarity_artifact_version,
    })
    .to_string();
    wavecrate_analysis::ann_index::mark_artifacts_dirty(connection, &similarity_state)?;
    Ok(Cancellable::Completed(true))
}

/// Retire rows written by the removed durable playback-summary readiness stage.
///
/// Current waveform and playback caches are managed by the independent cache lifecycle. This
/// compatibility pass exists only so writable source databases from older builds cannot keep stale
/// readiness work or reverse-ownership rows alive. Read-only reconciliation filters the same legacy
/// stage without mutating it.
pub(super) fn retire_legacy_playback_readiness(
    source: &SampleSource,
    connection: &mut rusqlite::Connection,
    cancel: &AtomicBool,
) -> Result<Cancellable<usize>, String> {
    retire_legacy_playback_readiness_with_post_commit_hook(source, connection, cancel, || {})
}

pub(super) fn retire_legacy_playback_readiness_with_post_commit_hook(
    source: &SampleSource,
    connection: &mut rusqlite::Connection,
    cancel: &AtomicBool,
    post_commit: impl FnOnce(),
) -> Result<Cancellable<usize>, String> {
    if cancelled(cancel) {
        return Ok(Cancellable::Cancelled);
    }
    let cleanup = ReadinessStore::new(connection)
        .retire_legacy_playback(source.id.as_str())
        .map_err(|error| error.to_string())?;
    post_commit();

    for cache_ref in cleanup.retired_artifact_refs {
        match retained_waveform_cache_ref_is_owned(&cache_ref) {
            Ok(false) => invalidate_persisted_waveform_cache_ref(std::path::Path::new(&cache_ref)),
            Ok(true) => {}
            Err(error) => tracing::warn!(
                target: "wavecrate::source_processing",
                source_id = source.id.as_str(),
                cache_ref,
                error,
                "Legacy playback cache ownership could not be proven; payload was preserved"
            ),
        }
    }
    Ok(Cancellable::Completed(cleanup.changed))
}

pub(super) fn cancelled(cancel: &AtomicBool) -> bool {
    cancel.load(Ordering::Acquire)
}

pub(super) fn readiness_contract_version() -> String {
    let mut hash = blake3::Hasher::new();
    let similarity_artifact_version = native_similarity_artifact_version();
    for component in [
        READINESS_MANIFEST_VERSION,
        wavecrate_analysis::analysis_version(),
        wavecrate_analysis::similarity::SIMILARITY_MODEL_ID,
        wavecrate_analysis::aspects::ASPECT_DESCRIPTOR_MODEL_ID,
        similarity_artifact_version.as_str(),
        READINESS_MEMBERSHIP_VERSION,
    ] {
        hash.update(component.as_bytes());
        hash.update(&[0]);
    }
    format!("readiness-contract-v2:{}", hash.finalize().to_hex())
}
