use rusqlite::params;

/// Purge orphaned analysis rows inside an existing write transaction.
pub(crate) fn purge_orphaned_samples_in_tx(
    tx: &rusqlite::Transaction<'_>,
) -> Result<usize, String> {
    let mut removed = 0usize;
    for table in ["analysis_features", "features", "embeddings", "samples"] {
        let sql = format!(
            "DELETE FROM {table}
             WHERE NOT EXISTS (
                SELECT 1
                FROM wav_files wf
                WHERE wf.path = substr({table}.sample_id, instr({table}.sample_id, '::') + 2)
             )"
        );
        removed += tx
            .execute(&sql, params![])
            .map_err(|err| format!("Failed to purge {table}: {err}"))?;
    }
    Ok(removed)
}
