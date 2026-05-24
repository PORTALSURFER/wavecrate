//! Global SQLite storage for sources that should not live in the config file.

use std::path::Path;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

use rusqlite::Connection;
use tracing::warn;

mod connection;
mod error;
mod migrations;
mod schema_checks;
mod schema_defs;
mod sources;
mod telemetry;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod telemetry_tests;

use super::{SampleSource, SourceId};
use connection::LibraryDatabase;
use error::map_sql_error;
use telemetry::record_library_db_event;

pub use error::LibraryError;

#[cfg(test)]
use crate::sample_sources::normalize_path;

/// Filename for the global library database stored under the user app directory.
pub const LIBRARY_DB_FILE_NAME: &str = "library.db";

/// Aggregate state loaded from or written to the library database.
#[derive(Debug, Clone, Default)]
pub struct LibraryState {
    /// All configured sample sources.
    pub sources: Vec<SampleSource>,
}

const KNOWN_SOURCES_KEY: &str = "known_sources_v1";

static LIBRARY_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

/// Load all sources from the global library database, creating it if missing.
pub fn load() -> Result<LibraryState, LibraryError> {
    let started_at = Instant::now();
    let _guard = lock_library();
    let db = LibraryDatabase::open()?;
    let result = db.load_state();
    record_library_db_event("library.load", started_at, result.as_ref().map(|_| ()));
    result
}

/// Persist sources to the global library database, replacing existing rows.
pub fn save(state: &LibraryState) -> Result<(), LibraryError> {
    let started_at = Instant::now();
    let _guard = lock_library();
    let mut db = LibraryDatabase::open()?;
    let result = db.replace_state(state);
    record_library_db_event("library.save", started_at, result.as_ref().map(|_| ()));
    result
}

/// Open a connection to the library DB with schema + migrations applied.
pub fn open_connection() -> Result<Connection, LibraryError> {
    let started_at = Instant::now();
    let _guard = lock_library();
    let db = LibraryDatabase::open()?;
    let result = Ok(db.into_connection());
    record_library_db_event(
        "library.open_connection",
        started_at,
        result.as_ref().map(|_| ()),
    );
    result
}

/// Attempt to reuse a historical source id for the given root folder.
///
/// This allows removing and re-adding a source without creating a new `source_id::...` namespace
/// (and therefore avoids re-analysis when files are unchanged).
pub fn lookup_source_id_for_root(root: &Path) -> Result<Option<SourceId>, LibraryError> {
    let started_at = Instant::now();
    let _guard = lock_library();
    let db = LibraryDatabase::open()?;
    let result = db.lookup_known_source_id(root);
    record_library_db_event(
        "library.lookup_source_id_for_root",
        started_at,
        result.as_ref().map(|_| ()),
    );
    result
}

fn lock_library() -> std::sync::MutexGuard<'static, ()> {
    match LIBRARY_LOCK.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            warn!("Library DB mutex poisoned; recovering to keep the app running.");
            poisoned.into_inner()
        }
    }
}
