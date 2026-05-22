use std::{
    ffi::OsString,
    fs,
    path::{Component, Path, PathBuf},
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

pub(super) fn is_user_library_root(root: &Path) -> bool {
    let Ok(home_root) = user_root_dir() else {
        return false;
    };
    let Ok(home_root) = home_root.canonicalize() else {
        return false;
    };
    let Ok(root_canonical) = root.canonicalize() else {
        return false;
    };
    let Ok(relative) = root_canonical.strip_prefix(&home_root) else {
        return false;
    };
    let mut components = relative.components();
    let Some(Component::Normal(first)) = components.next() else {
        return false;
    };
    is_user_library_root_name(first)
}

fn is_user_library_root_name(folder_name: &std::ffi::OsStr) -> bool {
    let name = folder_name.to_string_lossy().to_ascii_lowercase();
    matches!(
        name.as_str(),
        "music"
            | "documents"
            | "download"
            | "downloads"
            | "desktop"
            | "pictures"
            | "videos"
            | "video"
            | "movies"
            | "onedrive"
    )
}

fn user_root_dir() -> Result<PathBuf, &'static str> {
    if let Ok(home) = std::env::var("HOME") {
        return Ok(PathBuf::from(home));
    }
    if let (Ok(drive), Ok(path)) = (std::env::var("HOMEDRIVE"), std::env::var("HOMEPATH")) {
        return Ok(PathBuf::from(format!("{drive}{path}")));
    }
    if let Ok(user_profile) = std::env::var("USERPROFILE") {
        return Ok(PathBuf::from(user_profile));
    }
    Err("Missing HOME/USERPROFILE environment variable")
}
