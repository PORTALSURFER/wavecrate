use super::super::retry_policy::DEFERRED_MAINTENANCE_SCHEMA_TOKEN;

/// Return whether deferred source-db maintenance markers match the current revision/schema.
pub(super) fn deferred_maintenance_is_up_to_date(
    db: &crate::sample_sources::SourceDatabase,
    revision: u64,
) -> Result<bool, String> {
    let revision_marker = db
        .get_metadata(crate::sample_sources::db::META_DEFERRED_MAINTENANCE_REVISION)
        .map_err(|err| format!("Read deferred maintenance revision failed: {err}"))?;
    let schema_marker = db
        .get_metadata(crate::sample_sources::db::META_DEFERRED_MAINTENANCE_SCHEMA)
        .map_err(|err| format!("Read deferred maintenance schema marker failed: {err}"))?;
    let revision_string = revision.to_string();
    let schema_string = DEFERRED_MAINTENANCE_SCHEMA_TOKEN.to_string();
    Ok(revision_marker.as_deref() == Some(revision_string.as_str())
        && schema_marker.as_deref() == Some(schema_string.as_str()))
}

/// Persist deferred source-db maintenance revision/schema markers.
pub(super) fn update_deferred_maintenance_markers(
    conn: &rusqlite::Connection,
    revision: u64,
) -> Result<(), String> {
    update_metadata_value(
        conn,
        crate::sample_sources::db::META_DEFERRED_MAINTENANCE_REVISION,
        revision.to_string(),
        "revision",
    )?;
    update_metadata_value(
        conn,
        crate::sample_sources::db::META_DEFERRED_MAINTENANCE_SCHEMA,
        DEFERRED_MAINTENANCE_SCHEMA_TOKEN.to_string(),
        "schema marker",
    )
}

fn update_metadata_value(
    conn: &rusqlite::Connection,
    key: &str,
    value: String,
    label: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO metadata (key, value)
         VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )
    .map_err(|err| format!("Update deferred maintenance {label} failed: {err}"))?;
    Ok(())
}
