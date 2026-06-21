use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use super::super::{DB_FILE_NAME, LEGACY_DB_FILE_NAME, SourceDbError};

pub(super) fn read_only_db_path(root: &Path) -> PathBuf {
    let db_path = root.join(DB_FILE_NAME);
    if db_path.is_file() {
        return db_path;
    }
    let legacy_path = root.join(LEGACY_DB_FILE_NAME);
    if legacy_path.is_file() {
        legacy_path
    } else {
        db_path
    }
}

pub(super) fn prepare_writable_db_path(root: &Path) -> Result<PathBuf, SourceDbError> {
    let db_path = root.join(DB_FILE_NAME);
    if db_path.exists() {
        return Ok(db_path);
    }
    let legacy_path = root.join(LEGACY_DB_FILE_NAME);
    if legacy_path.exists() {
        migrate_legacy_source_db(&legacy_path, &db_path)?;
    }
    Ok(db_path)
}

fn migrate_legacy_source_db(from: &Path, to: &Path) -> Result<(), SourceDbError> {
    fs::rename(from, to).map_err(|source| SourceDbError::RenameLegacyDatabase {
        from: from.to_path_buf(),
        to: to.to_path_buf(),
        source,
    })?;
    for suffix in ["-wal", "-shm"] {
        let legacy_sidecar = sqlite_sidecar_path(from, suffix);
        if legacy_sidecar.exists() {
            let current_sidecar = sqlite_sidecar_path(to, suffix);
            fs::rename(&legacy_sidecar, &current_sidecar).map_err(|source| {
                SourceDbError::RenameLegacyDatabase {
                    from: legacy_sidecar,
                    to: current_sidecar,
                    source,
                }
            })?;
        }
    }
    Ok(())
}

fn sqlite_sidecar_path(path: &Path, suffix: &str) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(suffix);
    PathBuf::from(name)
}
