use std::path::Path;

use super::super::SourceDatabase;

pub(super) trait RecoveryFilesystem {
    fn is_file(&self, path: &Path) -> bool;
    fn create_dir_all(&self, path: &Path) -> Result<(), std::io::Error>;
    fn rename(&self, from: &Path, to: &Path) -> Result<(), std::io::Error>;
    fn remove_file(&self, path: &Path) -> Result<(), std::io::Error>;
    fn metadata(&self, path: &Path) -> Result<std::fs::Metadata, std::io::Error>;
}

pub(super) struct SystemRecoveryFilesystem;

impl RecoveryFilesystem for SystemRecoveryFilesystem {
    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn create_dir_all(&self, path: &Path) -> Result<(), std::io::Error> {
        std::fs::create_dir_all(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> Result<(), std::io::Error> {
        std::fs::rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> Result<(), std::io::Error> {
        std::fs::remove_file(path)
    }

    fn metadata(&self, path: &Path) -> Result<std::fs::Metadata, std::io::Error> {
        std::fs::metadata(path)
    }
}

pub(super) trait RecoverySourceDatabases {
    fn open(&self, root: &Path) -> Result<SourceDatabase, String>;
}

pub(super) struct SourceDatabaseRecoveryAccess;

impl RecoverySourceDatabases for SourceDatabaseRecoveryAccess {
    fn open(&self, root: &Path) -> Result<SourceDatabase, String> {
        SourceDatabase::open_for_source_write(root)
            .map_err(|error| format!("Failed to open source DB for recovery: {error}"))
    }
}
