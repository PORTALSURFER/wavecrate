use super::telemetry;
use super::types::SampleMetadata;
use rusqlite::Connection;
use rusqlite::params_from_iter;
use rusqlite::types::Value;

pub(crate) fn upsert_samples(
    conn: &mut Connection,
    samples: &[SampleMetadata],
) -> Result<usize, String> {
    if samples.is_empty() {
        return Ok(0);
    }
    let tx = telemetry::begin_immediate_transaction(conn, "analysis_samples_upsert")
        .map_err(|err| format!("Failed to start samples upsert transaction: {err}"))?;
    let changed = upsert_samples_in_tx(&tx, samples)?;
    telemetry::commit_transaction(tx, "analysis_samples_upsert")
        .map_err(|err| format!("Failed to commit samples upsert transaction: {err}"))?;
    Ok(changed)
}

/// Upsert sample rows inside an existing write transaction.
pub(crate) fn upsert_samples_in_tx(
    tx: &rusqlite::Transaction<'_>,
    samples: &[SampleMetadata],
) -> Result<usize, String> {
    let mut changed = 0usize;
    const BATCH_SIZE: usize = 200;
    for chunk in samples.chunks(BATCH_SIZE) {
        let mut sql = String::from(
            "INSERT INTO samples (sample_id, content_hash, size, mtime_ns, duration_seconds, sr_used, analysis_version, bpm, long_sample_mark) VALUES ",
        );
        let mut params: Vec<Value> = Vec::with_capacity(chunk.len() * 4);
        for (idx, sample) in chunk.iter().enumerate() {
            if idx > 0 {
                sql.push_str(", ");
            }
            let base = idx * 4;
            sql.push_str(&format!(
                "(?{}, ?{}, ?{}, ?{}, NULL, NULL, NULL, NULL, NULL)",
                base + 1,
                base + 2,
                base + 3,
                base + 4
            ));
            params.push(Value::from(sample.sample_id.clone()));
            params.push(Value::from(sample.content_hash.clone()));
            params.push(Value::from(sample.size as i64));
            params.push(Value::from(sample.mtime_ns));
        }
        sql.push_str(
            " ON CONFLICT(sample_id) DO UPDATE SET
                content_hash = excluded.content_hash,
                size = excluded.size,
                mtime_ns = excluded.mtime_ns,
                duration_seconds = CASE
                    WHEN samples.content_hash != excluded.content_hash
                         AND NOT (samples.content_hash LIKE 'fast-%'
                                  AND samples.size = excluded.size
                                  AND samples.mtime_ns = excluded.mtime_ns)
                    THEN NULL
                    ELSE samples.duration_seconds
                END,
                sr_used = CASE
                    WHEN samples.content_hash != excluded.content_hash
                         AND NOT (samples.content_hash LIKE 'fast-%'
                                  AND samples.size = excluded.size
                                  AND samples.mtime_ns = excluded.mtime_ns)
                    THEN NULL
                    ELSE samples.sr_used
                END,
                analysis_version = CASE
                    WHEN samples.content_hash != excluded.content_hash
                         AND NOT (samples.content_hash LIKE 'fast-%'
                                  AND samples.size = excluded.size
                                  AND samples.mtime_ns = excluded.mtime_ns)
                    THEN NULL
                    ELSE samples.analysis_version
                END,
                long_sample_mark = CASE
                    WHEN samples.content_hash != excluded.content_hash
                         AND NOT (samples.content_hash LIKE 'fast-%'
                                  AND samples.size = excluded.size
                                  AND samples.mtime_ns = excluded.mtime_ns)
                    THEN NULL
                    ELSE samples.long_sample_mark
                END,
                bpm = samples.bpm",
        );
        let batch_changed = tx
            .execute(&sql, params_from_iter(params))
            .map_err(|err| format!("Failed to upsert sample metadata: {err}"))?;
        changed += batch_changed as usize;
    }
    Ok(changed)
}
