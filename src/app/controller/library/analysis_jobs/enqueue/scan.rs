use super::enqueue_helpers::fast_content_hash;
use crate::app::controller::library::analysis_jobs::db;
use crate::sample_sources::scanner::ChangedSample;
use std::path::Path;

pub(crate) fn sample_metadata_for_changed_samples(
    source: &crate::sample_sources::SampleSource,
    changed_samples: &[ChangedSample],
) -> Vec<db::SampleMetadata> {
    changed_samples
        .iter()
        .map(|sample| db::SampleMetadata {
            sample_id: db::build_sample_id(source.id.as_str(), &sample.relative_path),
            content_hash: sample.content_hash.clone(),
            size: sample.file_size,
            mtime_ns: sample.modified_ns,
        })
        .collect()
}

pub(crate) fn stage_samples_for_source(
    source: &crate::sample_sources::SampleSource,
    _include_missing_entries: bool,
) -> Result<Vec<db::SampleMetadata>, String> {
    let source_db =
        crate::sample_sources::SourceDatabase::open(&source.root).map_err(|err| err.to_string())?;
    let entries = source_db.list_files().map_err(|err| err.to_string())?;
    if entries.is_empty() {
        return Ok(Vec::new());
    }
    stage_samples_from_entries(source, &source_db, &entries)
}

fn sample_metadata_from_entry(
    source_id: &str,
    relative_path: &Path,
    content_hash: Option<String>,
    file_size: u64,
    modified_ns: i64,
) -> Option<db::SampleMetadata> {
    let content_hash = match content_hash {
        Some(hash) if !hash.trim().is_empty() => hash,
        _ => fast_content_hash(file_size, modified_ns),
    };
    if content_hash.trim().is_empty() {
        return None;
    }
    Some(db::SampleMetadata {
        sample_id: db::build_sample_id(source_id, relative_path),
        content_hash,
        size: file_size,
        mtime_ns: modified_ns,
    })
}

fn stage_samples_from_entries(
    source: &crate::sample_sources::SampleSource,
    source_db: &crate::sample_sources::SourceDatabase,
    entries: &[crate::sample_sources::WavEntry],
) -> Result<Vec<db::SampleMetadata>, String> {
    let mut staged_samples = Vec::with_capacity(entries.len());
    for entry in entries {
        let absolute = source.root.join(&entry.relative_path);
        if !absolute.exists() {
            if let Some(db_entry) = source_db
                .entry_for_path(&entry.relative_path)
                .map_err(|err| err.to_string())?
            {
                let mut batch = source_db.write_batch().map_err(|err| err.to_string())?;
                batch
                    .stage_pending_rename(&db_entry)
                    .map_err(|err| err.to_string())?;
                batch
                    .remove_file(&entry.relative_path)
                    .map_err(|err| err.to_string())?;
                batch.commit().map_err(|err| err.to_string())?;
            }
            continue;
        }
        if let Some(metadata) = sample_metadata_from_entry(
            source.id.as_str(),
            &entry.relative_path,
            entry.content_hash.clone(),
            entry.file_size,
            entry.modified_ns,
        ) {
            staged_samples.push(metadata);
        }
    }
    Ok(staged_samples)
}
