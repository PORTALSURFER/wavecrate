use std::path::PathBuf;

use crate::sample_sources::{
    HarvestFileIdentity, HarvestFileKey, SourceDatabase, SourceId, harvest_file_ops, library,
};

/// Result of persisting a harvest file's seen/touched identity in the library database.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HarvestSeenPersistResult {
    /// Absolute file id/path that the native browser scheduled for persistence.
    pub file_id: String,
    /// Persistence outcome with a user-loggable error string on failure.
    pub result: Result<(), String>,
}

/// Background request for marking a harvest file as seen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HarvestSeenPersistRequest {
    /// Absolute file id/path used to correlate the background result.
    pub file_id: String,
    /// Source id that owns the harvest-relative sample path.
    pub source_id: SourceId,
    /// Filesystem root of the owning source.
    pub source_root: PathBuf,
    /// Metadata database root for the owning source.
    pub source_database_root: PathBuf,
    /// Path to the sample relative to the owning source root.
    pub relative_path: PathBuf,
}

/// Persist a harvest-seen marker using file metadata and the owning source database.
pub fn persist_harvest_seen(request: HarvestSeenPersistRequest) -> HarvestSeenPersistResult {
    let result = persist_harvest_seen_inner(&request);
    HarvestSeenPersistResult {
        file_id: request.file_id,
        result,
    }
}

fn persist_harvest_seen_inner(request: &HarvestSeenPersistRequest) -> Result<(), String> {
    let path = request.source_root.join(&request.relative_path);
    let (file_size, modified_ns) = harvest_file_ops::file_identity_metadata(&path);
    let entry = SourceDatabase::open_read_only_with_database_root(
        &request.source_root,
        &request.source_database_root,
    )
    .ok()
    .and_then(|db| db.entry_for_path(&request.relative_path).ok().flatten());
    let identity = HarvestFileIdentity {
        key: HarvestFileKey::new(request.source_id.clone(), request.relative_path.clone()),
        file_size: file_size.or_else(|| entry.as_ref().map(|entry| entry.file_size)),
        modified_ns: modified_ns.or_else(|| entry.as_ref().map(|entry| entry.modified_ns)),
        content_hash: entry.and_then(|entry| entry.content_hash),
    };
    library::mark_harvest_seen(&identity)
        .map(|_| ())
        .map_err(|err| err.to_string())
}
