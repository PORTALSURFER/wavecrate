use super::{
    ReadinessStore, SampleSource, SourceDatabase, SourceDatabaseConnectionRole,
    SourceMetadataStorage, invalidate_persisted_waveform_cache_ref, now_epoch_seconds,
    prune_unreferenced_waveform_cache, retained_waveform_cache_ref_is_owned,
};

pub(in crate::native_app::source_processing) enum SourceRetirementOutcome {
    Retired { retired_cache_refs: usize },
    TerminalOffline,
}

pub(in crate::native_app::source_processing) fn retire_source_derived_state(
    source: &SampleSource,
) -> Result<SourceRetirementOutcome, String> {
    let database_path = source.db_path().map_err(|error| error.to_string())?;
    if !database_path.exists() {
        if source.metadata_storage == SourceMetadataStorage::SourceFolder && !source.root.is_dir() {
            return Ok(SourceRetirementOutcome::TerminalOffline);
        }
        return Ok(SourceRetirementOutcome::Retired {
            retired_cache_refs: 0,
        });
    }
    let database_root = source.database_root().map_err(|error| error.to_string())?;
    let mut connection = SourceDatabase::open_connection_with_role_and_database_root(
        &source.root,
        &database_root,
        SourceDatabaseConnectionRole::JobWorker,
    )
    .map_err(|error| error.to_string())?;
    if connection
        .is_readonly(rusqlite::MAIN_DB)
        .map_err(|error| error.to_string())?
    {
        return Err(format!(
            "source database is read-only: {}",
            database_path.display()
        ));
    }
    if !ReadinessStore::new(&mut connection)
        .schema_available()
        .map_err(|error| error.to_string())?
    {
        return Ok(SourceRetirementOutcome::Retired {
            retired_cache_refs: 0,
        });
    }
    let cleanup = ReadinessStore::new(&mut connection)
        .retire_source(source.id.as_str(), now_epoch_seconds())
        .map_err(|error| error.to_string())?;
    let mut invalidated = 0;
    for cache_ref in &cleanup.retired_artifact_refs {
        match retained_waveform_cache_ref_is_owned(cache_ref) {
            Ok(false) => {
                invalidate_persisted_waveform_cache_ref(std::path::Path::new(cache_ref));
                invalidated += 1;
            }
            Ok(true) => {}
            Err(error) => tracing::warn!(
                target: "wavecrate::source_processing",
                cache_ref,
                error,
                "Retained cache ownership could not be proven; payload was preserved"
            ),
        }
    }
    if let Err(error) = prune_unreferenced_waveform_cache() {
        tracing::warn!(
            target: "wavecrate::source_processing",
            source_id = source.id.as_str(),
            error,
            "Bounded orphan cache collection was deferred"
        );
    }
    Ok(SourceRetirementOutcome::Retired {
        retired_cache_refs: invalidated,
    })
}
