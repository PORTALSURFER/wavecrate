use crate::app::controller::jobs::{MetadataMutationJob, SourceMetadataMutationOp};
use crate::sample_sources::{SourceDatabase, SourceDbError};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tracing::info;

pub(super) fn run_source_metadata_ops(job: &MetadataMutationJob) -> Result<(), String> {
    let started_at = Instant::now();
    let db =
        SourceDatabase::open_for_background_job(&job.source_root).map_err(|err| err.to_string())?;
    let resolved_ops = job
        .source_ops
        .iter()
        .map(|op| {
            let original_path = source_metadata_op_path(op);
            resolve_stale_browser_rename_path_with_db(job, &db, original_path)
                .map(|resolved_path| (op, original_path, resolved_path))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let log_records = source_metadata_log_records(&resolved_ops);
    let mut batch = db.write_batch().map_err(|err| err.to_string())?;
    for (op, original_path, resolved_path) in resolved_ops {
        let result = match op {
            SourceMetadataMutationOp::SetTagAndLocked { tag, locked, .. } => batch
                .set_tag(&resolved_path, *tag)
                .and_then(|_| batch.set_locked(&resolved_path, *locked)),
            SourceMetadataMutationOp::SetLooped { looped, .. } => {
                batch.set_looped(&resolved_path, *looped)
            }
            SourceMetadataMutationOp::SetSoundType { sound_type, .. } => {
                batch.set_sound_type(&resolved_path, *sound_type)
            }
            SourceMetadataMutationOp::SetUserTag { user_tag, .. } => {
                batch.set_user_tag(&resolved_path, user_tag.as_deref())
            }
            SourceMetadataMutationOp::SetTagNamed { tag_named, .. } => {
                batch.set_tag_named(&resolved_path, *tag_named)
            }
            SourceMetadataMutationOp::AssignNormalTag { label, .. } => {
                batch.assign_tag_to_path(&resolved_path, label).map(|_| ())
            }
            SourceMetadataMutationOp::RemoveNormalTag { label, .. } => batch
                .remove_tag_from_path(&resolved_path, label)
                .map(|_| ()),
            SourceMetadataMutationOp::SetLastPlayedAt { played_at, .. } => {
                batch.set_last_played_at(&resolved_path, *played_at)
            }
        };
        if let Err(err) = result {
            let message = source_metadata_op_error(op, original_path, &resolved_path, err);
            log_source_metadata_mutation_batch(
                job,
                &log_records,
                "error",
                started_at.elapsed(),
                Some(&message),
            );
            return Err(message);
        }
    }
    match batch.commit() {
        Ok(()) => {
            log_source_metadata_mutation_batch(job, &log_records, "ok", started_at.elapsed(), None);
            Ok(())
        }
        Err(err) => {
            let message = err.to_string();
            log_source_metadata_mutation_batch(
                job,
                &log_records,
                "error",
                started_at.elapsed(),
                Some(&message),
            );
            Err(message)
        }
    }
}

struct SourceMetadataMutationLogRecord {
    op_name: &'static str,
    original_path: PathBuf,
    resolved_path: PathBuf,
}

fn source_metadata_log_records(
    resolved_ops: &[(&SourceMetadataMutationOp, &Path, PathBuf)],
) -> Vec<SourceMetadataMutationLogRecord> {
    resolved_ops
        .iter()
        .map(
            |(op, original_path, resolved_path)| SourceMetadataMutationLogRecord {
                op_name: source_metadata_op_name(op),
                original_path: (*original_path).to_path_buf(),
                resolved_path: resolved_path.clone(),
            },
        )
        .collect()
}

fn log_source_metadata_mutation_batch(
    job: &MetadataMutationJob,
    records: &[SourceMetadataMutationLogRecord],
    result: &'static str,
    elapsed: Duration,
    error: Option<&str>,
) {
    info!(
        source_id = %job.source_id,
        request_id = job.request_id,
        op_count = records.len(),
        result,
        elapsed_ms = elapsed.as_millis() as u64,
        ops = %format_source_metadata_mutation_records(records),
        error = error.unwrap_or(""),
        "source metadata mutation: source ops resolved"
    );
}

fn format_source_metadata_mutation_records(records: &[SourceMetadataMutationLogRecord]) -> String {
    const MAX_ITEMS: usize = 8;
    let mut parts = records
        .iter()
        .take(MAX_ITEMS)
        .map(|record| {
            if record.original_path == record.resolved_path {
                format!("{} {}", record.op_name, record.original_path.display())
            } else {
                format!(
                    "{} {} -> {} remapped=true",
                    record.op_name,
                    record.original_path.display(),
                    record.resolved_path.display()
                )
            }
        })
        .collect::<Vec<_>>();
    if records.len() > MAX_ITEMS {
        parts.push(format!("... +{} more", records.len() - MAX_ITEMS));
    }
    parts.join("; ")
}

fn source_metadata_op_path(op: &SourceMetadataMutationOp) -> &Path {
    match op {
        SourceMetadataMutationOp::SetTagAndLocked { relative_path, .. }
        | SourceMetadataMutationOp::SetLooped { relative_path, .. }
        | SourceMetadataMutationOp::SetSoundType { relative_path, .. }
        | SourceMetadataMutationOp::SetUserTag { relative_path, .. }
        | SourceMetadataMutationOp::SetTagNamed { relative_path, .. }
        | SourceMetadataMutationOp::AssignNormalTag { relative_path, .. }
        | SourceMetadataMutationOp::RemoveNormalTag { relative_path, .. }
        | SourceMetadataMutationOp::SetLastPlayedAt { relative_path, .. } => relative_path,
    }
}

fn source_metadata_op_name(op: &SourceMetadataMutationOp) -> &'static str {
    match op {
        SourceMetadataMutationOp::SetTagAndLocked { .. } => "SetTagAndLocked",
        SourceMetadataMutationOp::SetLooped { .. } => "SetLooped",
        SourceMetadataMutationOp::SetSoundType { .. } => "SetSoundType",
        SourceMetadataMutationOp::SetUserTag { .. } => "SetUserTag",
        SourceMetadataMutationOp::SetTagNamed { .. } => "SetTagNamed",
        SourceMetadataMutationOp::AssignNormalTag { .. } => "AssignNormalTag",
        SourceMetadataMutationOp::RemoveNormalTag { .. } => "RemoveNormalTag",
        SourceMetadataMutationOp::SetLastPlayedAt { .. } => "SetLastPlayedAt",
    }
}

fn source_metadata_op_error(
    op: &SourceMetadataMutationOp,
    original_path: &Path,
    resolved_path: &Path,
    err: SourceDbError,
) -> String {
    if original_path == resolved_path {
        format!(
            "Source metadata {} failed for {}: {err}",
            source_metadata_op_name(op),
            original_path.display()
        )
    } else {
        format!(
            "Source metadata {} failed for {} (resolved to {}): {err}",
            source_metadata_op_name(op),
            original_path.display(),
            resolved_path.display()
        )
    }
}

fn resolve_stale_browser_rename_path_with_db(
    job: &MetadataMutationJob,
    db: &SourceDatabase,
    relative_path: &Path,
) -> Result<PathBuf, String> {
    if job.source_root.join(relative_path).exists() {
        return Ok(relative_path.to_path_buf());
    }
    let Some(new_relative) =
        crate::app::controller::library::source_write_priority::completed_browser_rename_target(
            &job.source_id,
            relative_path,
        )
    else {
        return Ok(relative_path.to_path_buf());
    };
    if db
        .entry_for_path(&new_relative)
        .map_err(|err| format!("Failed to resolve renamed metadata target: {err}"))?
        .is_some()
    {
        return Ok(new_relative);
    }
    Ok(relative_path.to_path_buf())
}
