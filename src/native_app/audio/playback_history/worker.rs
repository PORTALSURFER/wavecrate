use std::{
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use wavecrate::sample_sources::SourceDatabase;

use crate::native_app::audio::playback_history::LastPlayedPersistResult;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct LastPlayedPersistRequest {
    pub(super) file_id: String,
    pub(super) source_root: PathBuf,
    pub(super) relative_path: PathBuf,
    pub(super) played_at: i64,
}

pub(super) fn persist_last_played(request: LastPlayedPersistRequest) -> LastPlayedPersistResult {
    let result = persist_last_played_inner(&request);
    LastPlayedPersistResult {
        file_id: request.file_id,
        result,
    }
}

fn persist_last_played_inner(request: &LastPlayedPersistRequest) -> Result<(), String> {
    let (file_size, modified_ns) =
        file_metadata(&request.source_root.join(&request.relative_path))?;
    let db = SourceDatabase::open_for_user_metadata_write(&request.source_root)
        .map_err(|err| err.to_string())?;
    let mut batch = db.write_batch().map_err(|err| err.to_string())?;
    batch
        .upsert_file(&request.relative_path, file_size, modified_ns)
        .map_err(|err| err.to_string())?;
    batch
        .set_last_played_at(&request.relative_path, request.played_at)
        .map_err(|err| err.to_string())?;
    batch.commit().map_err(|err| err.to_string())
}

fn file_metadata(path: &Path) -> Result<(u64, i64), String> {
    let metadata = std::fs::metadata(path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let modified_ns = metadata
        .modified()
        .map_err(|err| format!("Missing modified time for {}: {err}", path.display()))?
        .duration_since(UNIX_EPOCH)
        .map_err(|_| String::from("File modified time is before epoch"))?
        .as_nanos() as i64;
    Ok((metadata.len(), modified_ns))
}
