use super::types::{MetadataTagPersistRequest, MetadataTagPersistResult};
#[cfg(test)]
use std::path::PathBuf;
use std::{collections::HashMap, path::Path, time::SystemTime};
use wavecrate::sample_sources::{SourceDatabase, SourceDbError};

pub(super) fn persist_metadata_tag_assignment(
    request: MetadataTagPersistRequest,
) -> MetadataTagPersistResult {
    let result = persist_metadata_tag_assignment_inner(&request);
    MetadataTagPersistResult {
        tags: request.tags,
        assigned: request.assigned,
        result,
    }
}

pub(super) fn persist_metadata_tag_deletions(
    requests: Vec<MetadataTagPersistRequest>,
) -> MetadataTagPersistResult {
    let tags = requests
        .first()
        .map(|request| request.tags.clone())
        .unwrap_or_default();
    let result = requests
        .iter()
        .try_for_each(persist_metadata_tag_assignment_inner);
    MetadataTagPersistResult {
        tags,
        assigned: false,
        result,
    }
}

fn persist_metadata_tag_assignment_inner(
    request: &MetadataTagPersistRequest,
) -> Result<(), String> {
    let (file_size, modified_ns) = file_metadata(&request.absolute_path)?;
    let db = SourceDatabase::open_for_user_metadata_write(&request.source_root)
        .map_err(|err| err.to_string())?;
    let mut batch = db.write_batch().map_err(|err| err.to_string())?;
    batch
        .upsert_file(&request.relative_path, file_size, modified_ns)
        .map_err(|err| err.to_string())?;
    for tag in &request.tags {
        if request.assigned {
            batch
                .assign_tag_to_path(&request.relative_path, tag)
                .map(|_| ())
        } else {
            batch
                .remove_tag_from_path(&request.relative_path, tag)
                .map(|_| ())
        }
        .map_err(|err| err.to_string())?;
    }
    batch.commit().map_err(|err| err.to_string())
}

#[cfg(test)]
pub(in crate::gui_app) fn persist_metadata_tag_additions_for_tests(
    absolute_path: PathBuf,
    source_root: PathBuf,
    relative_path: PathBuf,
    tags: Vec<String>,
) -> Result<(), String> {
    persist_metadata_tag_assignment_inner(&MetadataTagPersistRequest {
        absolute_path,
        source_root,
        relative_path,
        tags,
        assigned: true,
    })
}

#[cfg(test)]
pub(in crate::gui_app) fn persist_metadata_tag_removals_for_tests(
    absolute_path: PathBuf,
    source_root: PathBuf,
    relative_path: PathBuf,
    tags: Vec<String>,
) -> Result<(), String> {
    persist_metadata_tag_assignment_inner(&MetadataTagPersistRequest {
        absolute_path,
        source_root,
        relative_path,
        tags,
        assigned: false,
    })
}

pub(super) fn load_persisted_metadata_tags_for_source(
    source_root: &Path,
    tags_by_file: &mut HashMap<String, Vec<String>>,
) -> Result<(), String> {
    let db = match SourceDatabase::open_read_only(source_root) {
        Ok(db) => db,
        Err(SourceDbError::ReadOnlyDatabaseMissing(_)) => return Ok(()),
        Err(err) => return Err(err.to_string()),
    };
    for entry in db.list_files().map_err(|err| err.to_string())? {
        if entry.normal_tags.is_empty() {
            continue;
        }
        let absolute_path = source_root.join(entry.relative_path);
        tags_by_file.insert(
            absolute_path.to_string_lossy().to_string(),
            entry.normal_tags,
        );
    }
    Ok(())
}

fn file_metadata(path: &Path) -> Result<(u64, i64), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("Sample metadata unavailable for {}: {err}", path.display()))?;
    let modified_ns = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(SystemTime::UNIX_EPOCH).ok())
        .and_then(|duration| i64::try_from(duration.as_nanos()).ok())
        .unwrap_or_default();
    Ok((metadata.len(), modified_ns))
}
