//! Shared WAL policy helpers for SQLite-backed Wavecrate databases.
//!
//! The source and library databases both run in WAL mode so readers can stay
//! responsive while scan, enqueue, and analysis writers churn in the
//! background. This module makes the policy explicit instead of relying on
//! SQLite defaults by:
//!
//! - setting a conservative auto-checkpoint threshold
//! - capping retained post-checkpoint journal size
//! - attempting throttled passive checkpoints only after the WAL has already
//!   grown beyond the normal steady-state target

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use rusqlite::{Connection, OpenFlags};

/// SQL fragment that applies the shared WAL-health settings for write-capable DBs.
pub(crate) const WORKLOAD_WAL_PRAGMAS_SQL: &str = "PRAGMA wal_autocheckpoint=4096;
                 PRAGMA journal_size_limit=67108864;";

const PASSIVE_CHECKPOINT_TRIGGER_BYTES: u64 = 32 * 1024 * 1024;
const PASSIVE_CHECKPOINT_MIN_INTERVAL: Duration = Duration::from_secs(15);
const CHECKPOINT_BUSY_TIMEOUT: Duration = Duration::from_millis(250);
const SLOW_CHECKPOINT_THRESHOLD: Duration = Duration::from_millis(50);

/// Apply the shared write-capable WAL policy to one SQLite connection.
pub(crate) fn apply_workload_wal_pragmas(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(WORKLOAD_WAL_PRAGMAS_SQL)?;
    connection.pragma_update(None, "wal_autocheckpoint", 4096_i64)?;
    connection.pragma_update(None, "journal_size_limit", 67_108_864_i64)
}

static LAST_PASSIVE_CHECKPOINT_AT: LazyLock<Mutex<HashMap<PathBuf, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, Copy)]
struct PassiveCheckpointStats {
    busy: bool,
    log_frames: i64,
    checkpointed_frames: i64,
}

/// Attempt one best-effort passive checkpoint when the WAL file is both large
/// enough and old enough to justify maintenance.
///
/// This is intentionally conservative:
/// - it never runs for small WAL files
/// - it is throttled per database file
/// - it uses `PASSIVE` so readers win if they are still holding snapshots
pub(crate) fn maybe_checkpoint_database_file(
    db_path: &Path,
    database_kind: &'static str,
    role: &'static str,
) {
    let wal_path = wal_path_for(db_path);
    let Some(wal_bytes_before) = wal_file_size(&wal_path) else {
        return;
    };
    if wal_bytes_before < PASSIVE_CHECKPOINT_TRIGGER_BYTES {
        return;
    }
    let now = Instant::now();
    if !claim_checkpoint_window(db_path, now) {
        return;
    }

    let started_at = Instant::now();
    let result = open_checkpoint_connection(db_path).and_then(run_passive_checkpoint);
    let elapsed = started_at.elapsed();
    let wal_bytes_after = wal_file_size(&wal_path).unwrap_or(0);
    match result {
        Ok(stats) => {
            tracing::info!(
                target: "perf::source_db",
                action = "wal_checkpoint",
                database_kind,
                role,
                mode = "passive",
                wal_bytes_before,
                wal_bytes_after,
                busy = stats.busy,
                log_frames = stats.log_frames,
                checkpointed_frames = stats.checkpointed_frames,
                elapsed_ms = elapsed.as_millis() as u64,
                "Evaluated WAL checkpoint policy"
            );
            if !stats.busy
                && wal_bytes_after >= PASSIVE_CHECKPOINT_TRIGGER_BYTES
                && elapsed >= SLOW_CHECKPOINT_THRESHOLD
            {
                tracing::info!(
                    target: "perf::source_db",
                    action = "wal_checkpoint_slow",
                    database_kind,
                    role,
                    wal_bytes_after,
                    elapsed_ms = elapsed.as_millis() as u64,
                    "Passive WAL checkpoint completed slowly and left a large WAL file"
                );
            }
        }
        Err(err) => {
            tracing::warn!(
                target: "perf::source_db",
                action = "wal_checkpoint",
                database_kind,
                role,
                wal_bytes_before,
                wal_bytes_after,
                elapsed_ms = elapsed.as_millis() as u64,
                error = %err,
                "WAL checkpoint policy evaluation failed"
            );
        }
    }
}

fn claim_checkpoint_window(db_path: &Path, now: Instant) -> bool {
    let mut checkpoints = match LAST_PASSIVE_CHECKPOINT_AT.lock() {
        Ok(checkpoints) => checkpoints,
        Err(poisoned) => poisoned.into_inner(),
    };
    if let Some(last_attempt) = checkpoints.get(db_path)
        && now.duration_since(*last_attempt) < PASSIVE_CHECKPOINT_MIN_INTERVAL
    {
        return false;
    }
    checkpoints.insert(db_path.to_path_buf(), now);
    true
}

fn open_checkpoint_connection(db_path: &Path) -> rusqlite::Result<Connection> {
    let connection = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    connection.busy_timeout(CHECKPOINT_BUSY_TIMEOUT)?;
    Ok(connection)
}

fn run_passive_checkpoint(connection: Connection) -> rusqlite::Result<PassiveCheckpointStats> {
    connection.query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |row| {
        Ok(PassiveCheckpointStats {
            busy: row.get::<_, i64>(0)? != 0,
            log_frames: row.get(1)?,
            checkpointed_frames: row.get(2)?,
        })
    })
}

fn wal_file_size(wal_path: &Path) -> Option<u64> {
    match std::fs::metadata(wal_path) {
        Ok(metadata) => Some(metadata.len()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => {
            tracing::debug!(
                target: "perf::source_db",
                wal_path = %wal_path.display(),
                error = %err,
                "Could not read WAL file metadata"
            );
            None
        }
    }
}

fn wal_path_for(db_path: &Path) -> PathBuf {
    let mut wal_name = OsString::from(db_path.as_os_str());
    wal_name.push("-wal");
    PathBuf::from(wal_name)
}
