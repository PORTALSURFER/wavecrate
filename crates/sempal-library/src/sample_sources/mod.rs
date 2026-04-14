use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod audio_support;
/// Per-source database helpers.
pub mod db;
/// Global library database helpers.
pub mod library;

pub use audio_support::{is_supported_audio, supported_audio_where_clause};
pub use db::normalize_relative_path;
pub use db::{DB_FILE_NAME, Rating, SourceDatabase, SourceDbError, WavEntry};
pub use library::{LIBRARY_DB_FILE_NAME, LibraryError, LibraryState};

/// Identifier for a configured sample source.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceId(String);

impl SourceId {
    /// Create a new unique source identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Rehydrate a source identifier from a stored string.
    pub fn from_string(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the identifier as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SourceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SourceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// User-selected folder that owns its own SQLite database of wav files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleSource {
    /// Stable identifier for the source.
    pub id: SourceId,
    /// Root folder path for the source.
    pub root: PathBuf,
}

impl SampleSource {
    /// Create a new sample source for the given directory.
    pub fn new(root: PathBuf) -> Self {
        Self {
            id: SourceId::new(),
            root,
        }
    }

    /// Create a sample source with an existing id.
    pub fn new_with_id(id: SourceId, root: PathBuf) -> Self {
        Self { id, root }
    }

    /// Location of the SQLite database for this source.
    pub fn db_path(&self) -> PathBuf {
        database_path_for(&self.root)
    }

    /// Open the SQLite database for this source, creating it if necessary.
    pub fn open_db(&self) -> Result<SourceDatabase, SourceDbError> {
        SourceDatabase::open(&self.root)
    }
}

/// Normalize a path for durable storage by preserving only its path components.
pub fn normalize_path(path: &Path) -> PathBuf {
    PathBuf::from_iter(path.components())
}

/// Name the per-source database using a hidden file inside the chosen folder.
pub fn database_path_for(root: &Path) -> PathBuf {
    root.join(DB_FILE_NAME)
}
